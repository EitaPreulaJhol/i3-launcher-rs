//! launcher-rs: dmenu-style application launcher strip with icons.

use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result};
use tiny_skia::Pixmap;

use launcher_rs::app;
use launcher_rs::cli;
use launcher_rs::config::Config;
use launcher_rs::desktop::launch;
use launcher_rs::ui::paint::{pixmap_to_x11, Painter};
use launcher_rs::ui::state::{App, Outcome};
use launcher_rs::x11::{InputEvent, X11};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("launcher-rs: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<i32> {
    let startup = Instant::now();
    let options = cli::parse(std::env::args().skip(1))?;
    if options.help {
        print!("{}", cli::help());
        return Ok(0);
    }
    if options.version {
        println!("launcher-rs {}", cli::VERSION);
        return Ok(0);
    }
    if options.list_themes {
        for name in launcher_rs::theme::names() {
            println!("{name}");
        }
        return Ok(0);
    }
    for warning in &options.warnings {
        eprintln!("launcher-rs: {warning}");
    }

    let mut cfg = Config::load(options.config.as_deref(), options.theme.as_deref())?;
    cli::apply(&options, &mut cfg)?;
    debug(|| {
        let theme = if cfg.style.theme.is_empty() {
            "default (dmenu.rasi)"
        } else {
            &cfg.style.theme
        };
        format!(
            "config: {} ms (theme: {theme})",
            startup.elapsed().as_millis()
        )
    });

    // Scanning is fast (~ms); reading stdin may block, so it happens before the
    // keyboard grab, exactly like dmenu's default behaviour.
    let scan_start = Instant::now();
    let entries = if options.dmenu {
        Vec::new()
    } else {
        app::load_entries()
    };
    let items = if options.dmenu {
        app::read_stdin(options.exec).context("could not read stdin")?
    } else {
        app::from_entries(&entries)
    };

    debug(|| {
        format!(
            "items={} in {} ms",
            items.len(),
            scan_start.elapsed().as_millis()
        )
    });

    let x11_start = Instant::now();
    // App mode is single-instance per display: pressing the keybinding again
    // while the strip is open must not stack extra windows (only the first
    // one can hold the keyboard grab). `--dmenu` pipes stay independent.
    let Some(x11) = X11::create(&cfg, !options.dmenu)? else {
        debug(|| "another instance is already running".to_string());
        return Ok(0);
    };
    debug(|| {
        format!(
            "monitor={} size={:?} (window in {} ms)",
            x11.monitor_name(),
            x11.size(),
            x11_start.elapsed().as_millis()
        )
    });

    let (width, height) = x11.size();
    let mut surface = Pixmap::new(width, height).context("could not allocate the framebuffer")?;
    // Fonts are loaded after the grab so no keystroke is lost.
    let font_start = Instant::now();
    let mut painter = Painter::new(&cfg);
    debug(|| {
        format!(
            "fonts/icon theme ({}) in {} ms",
            painter.icon_theme(),
            font_start.elapsed().as_millis()
        )
    });

    let mut application = if options.dmenu {
        App::new(items, &cfg.behavior)
    } else {
        App::with_path_loader(
            items,
            || {
                let start = Instant::now();
                let paths = app::path_items();
                debug(|| {
                    format!(
                        "PATH commands: {} in {} ms",
                        paths.len(),
                        start.elapsed().as_millis()
                    )
                });
                paths
            },
            &cfg.behavior,
        )
    };

    let first_frame = Instant::now();
    application.relayout(&mut painter, &cfg, width as f32, height as f32);
    painter.paint(&mut surface, &application, &cfg);
    x11.blit(&pixmap_to_x11(&surface))?;
    debug(|| {
        format!(
            "first frame: {} ms (total {} ms)",
            first_frame.elapsed().as_millis(),
            startup.elapsed().as_millis()
        )
    });

    let mut redraw = false;
    loop {
        if redraw {
            application.relayout(&mut painter, &cfg, width as f32, height as f32);
            painter.paint(&mut surface, &application, &cfg);
            x11.blit(&pixmap_to_x11(&surface))?;
            redraw = false;
        }

        let event = x11.wait_for_event()?;
        match x11.translate(event) {
            Some(InputEvent::Key { sym, mods }) => {
                redraw = application.handle_key(sym, mods, &cfg.behavior);
            }
            Some(InputEvent::Click { x, button, .. }) => {
                redraw = application.handle_click(x, button);
            }
            Some(InputEvent::Scroll { up }) => {
                redraw = application.handle_scroll(up);
            }
            Some(InputEvent::Expose) => redraw = true,
            Some(InputEvent::FocusIn) => {
                // A keybinding/window-manager focus transition can displace
                // the grab. Reassert it before waiting for the next keypress.
                x11.regrab();
                redraw = true;
            }
            Some(InputEvent::Quit) => return Ok(1),
            None => {}
        }

        if let Some(outcome) = application.outcome.take() {
            return handle_outcome(outcome, &entries, &cfg);
        }
    }
}

fn handle_outcome(
    outcome: Outcome,
    entries: &[launcher_rs::desktop::DesktopEntry],
    cfg: &Config,
) -> Result<i32> {
    match outcome {
        Outcome::Quit(code) => Ok(code),
        Outcome::Print(text) => {
            println!("{text}");
            Ok(0)
        }
        Outcome::Shell(line) => {
            launch::spawn_shell(&line, &cfg.behavior.shell)?;
            Ok(0)
        }
        Outcome::Command(name) => {
            if cfg.behavior.print_selection {
                println!("{name}");
                return Ok(0);
            }
            launch::spawn(&launch::LaunchCommand {
                argv: vec![name],
                cwd: None,
            })?;
            Ok(0)
        }
        Outcome::LaunchApp(index) => {
            let entry = entries
                .get(index)
                .context("the selected .desktop entry no longer exists")?;
            let command = launch::build(entry, cfg)?;
            if cfg.behavior.print_selection {
                println!("{}", entry.name);
                return Ok(0);
            }
            launch::spawn(&command)?;
            Ok(0)
        }
    }
}

fn debug(message: impl Fn() -> String) {
    if std::env::var_os("LAUNCHER_DEBUG").is_some() {
        eprintln!("launcher-rs: {}", message());
    }
}
