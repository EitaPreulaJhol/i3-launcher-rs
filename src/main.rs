//! i3-launcher: dmenu-style application launcher strip with icons.

use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result};
use tiny_skia::Pixmap;

use i3_launcher::app;
use i3_launcher::cli;
use i3_launcher::config::Config;
use i3_launcher::desktop::launch;
use i3_launcher::ui::paint::{pixmap_to_x11, Painter};
use i3_launcher::ui::state::{App, Outcome};
use i3_launcher::x11::{InputEvent, X11};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("i3-launcher: {error:#}");
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
        println!("i3-launcher {}", cli::VERSION);
        return Ok(0);
    }
    if options.list_themes {
        for name in i3_launcher::theme::names() {
            println!("{name}");
        }
        return Ok(0);
    }
    for warning in &options.warnings {
        eprintln!("i3-launcher: {warning}");
    }

    let mut cfg = Config::load(options.config.as_deref(), options.theme.as_deref())?;
    cli::apply(&options, &mut cfg)?;
    debug(|| {
        let theme = if cfg.style.theme.is_empty() {
            "padrão (dmenu.rasi)"
        } else {
            &cfg.style.theme
        };
        format!(
            "config: {} ms (tema: {theme})",
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
        app::read_stdin(options.exec).context("não foi possível ler o stdin")?
    } else {
        app::from_entries(&entries)
    };

    debug(|| {
        format!(
            "itens={} em {} ms",
            items.len(),
            scan_start.elapsed().as_millis()
        )
    });

    let x11_start = Instant::now();
    let x11 = X11::create(&cfg)?;
    debug(|| {
        format!(
            "monitor={} tamanho={:?} (janela em {} ms)",
            x11.monitor_name(),
            x11.size(),
            x11_start.elapsed().as_millis()
        )
    });

    let (width, height) = x11.size();
    let mut surface =
        Pixmap::new(width, height).context("não foi possível alocar o framebuffer")?;
    // Fonts are loaded after the grab so no keystroke is lost.
    let font_start = Instant::now();
    let mut painter = Painter::new(&cfg);
    debug(|| {
        format!(
            "fontes/tema de ícones ({}) em {} ms",
            painter.icon_theme(),
            font_start.elapsed().as_millis()
        )
    });

    let mut application = App::new(items, &cfg.behavior);
    let first_frame = Instant::now();
    application.relayout(&mut painter, &cfg, width as f32, height as f32);
    painter.paint(&mut surface, &application, &cfg);
    x11.blit(&pixmap_to_x11(&surface))?;
    debug(|| {
        format!(
            "primeiro frame: {} ms (total {} ms)",
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
            Some(InputEvent::FocusIn) => redraw = true,
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
    entries: &[i3_launcher::desktop::DesktopEntry],
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
        Outcome::LaunchApp(index) => {
            let entry = entries
                .get(index)
                .context("entrada .desktop selecionada não existe mais")?;
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
    if std::env::var_os("I3L_DEBUG").is_some() {
        eprintln!("i3-launcher: {}", message());
    }
}
