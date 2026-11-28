//! Command line parsing, deliberately close to dmenu's interface.

use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::config::{Align, Config, Dimension, Edge, MonitorTarget};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Default)]
pub struct Options {
    pub config: Option<PathBuf>,
    /// Built-in theme override (`--theme`).
    pub theme: Option<String>,
    /// List the available themes and exit.
    pub list_themes: bool,
    pub prompt: Option<String>,
    pub edge: Option<Edge>,
    pub monitor: Option<MonitorTarget>,
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub height: Option<u32>,
    pub margin: Option<u32>,
    pub offset: Option<u32>,
    pub width: Option<Dimension>,
    pub align: Option<Align>,
    pub icon_size: Option<u32>,
    pub no_icons: bool,
    pub term: Option<String>,
    /// Read items from stdin and print the selection (dmenu behaviour).
    pub dmenu: bool,
    /// With `--dmenu`, execute the selection through `/bin/sh` instead of printing.
    pub exec: bool,
    /// Print the selection instead of launching it (app mode).
    pub print: bool,
    pub case_sensitive: Option<bool>,
    pub match_mode: Option<crate::config::MatchMode>,
    pub help: bool,
    pub version: bool,
    /// dmenu flags whose effect is not implemented yet.
    pub warnings: Vec<String>,
    // dmenu color flags (validated/applied by `apply`)
    pub normal_bg: Option<String>,
    pub normal_fg: Option<String>,
    pub selected_bg: Option<String>,
    pub selected_fg: Option<String>,
}

const HELP: &str = "\
launcher-rs - dmenu-style launcher with icons for X11 window managers

USAGE:
    launcher-rs [OPTIONS]

OPTIONS:
    -c, --config <FILE>        Path to the TOML config
        --theme <NAME>         Color theme (overrides the one from the config)
        --themes               List the available themes and exit
    -p, --prompt <TEXT>        Prompt text (default: \"Run: \")
    -b, --bottom               Place the strip at the bottom of the monitor
    -e, --edge <top|bottom>    Monitor edge
    -m, --monitor <TARGET>     focused | primary | pointer | INDEX | NAME
        --height <PX>          Strip height
        --margin <PX>          Outer horizontal margin
        --offset <PX>          Distance from the edge
        --width <PX|%>         Strip width
        --align <left|center|right>
    -fn, --font <FAMILY>       Font family
        --font-size <PX>       Font size
        --icon-size <PX>       Icon size (default: 24)
        --no-icons             Do not draw icons
        --terminal <CMD>       Terminal for entries with Terminal=true
        --case-sensitive       Case-sensitive search
        --match <prefix|substring|fuzzy>
    -d, --dmenu                Read items from stdin and print the selection
        --exec                 With --dmenu, run the selection through the shell
        --print                Print the selection instead of running it
    -i                         Alias for --ignore-case (default)
    -nb, --normal-bg <COLOR>   Normal background color
    -nf, --normal-fg <COLOR>   Normal text color
    -sb, --selected-bg <COLOR> Selected background color
    -sf, --selected-fg <COLOR> Selected text color
    -h, --help                 Show this help
    -v, --version              Show the version

KEYS:
    Tab                        Switch between apps (.desktop) and $PATH commands
    Enter                      Run the selected item
    Esc                        Close

COLORS: #rgb, #rrggbb or #rrggbbaa.

CONFIG: $LAUNCHER_CONFIG, $XDG_CONFIG_HOME/launcher-rs/config.toml,
        ~/.config/launcher-rs/config.toml
";

/// Parse CLI arguments (without the program name).
pub fn parse<I>(args: I) -> Result<Options>
where
    I: IntoIterator<Item = String>,
{
    let mut opts = Options::default();
    let mut args = args.into_iter().peekable();

    while let Some(arg) = args.next() {
        // Split `--flag=value`.
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with('-') => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut next_value = |flag: &str| -> Result<String> {
            if let Some(v) = inline.clone() {
                Ok(v)
            } else {
                match args.next() {
                    Some(v) => Ok(v),
                    None => bail!("option `{flag}` requires a value"),
                }
            }
        };

        match flag.as_str() {
            "-h" | "--help" => opts.help = true,
            "-v" | "--version" => opts.version = true,
            "-c" | "--config" => opts.config = Some(PathBuf::from(next_value(&flag)?)),
            "--theme" => opts.theme = Some(next_value(&flag)?),
            "--themes" | "--list-themes" => opts.list_themes = true,
            "-p" | "--prompt" => opts.prompt = Some(next_value(&flag)?),
            "-b" | "--bottom" => opts.edge = Some(Edge::Bottom),
            "-e" | "--edge" => opts.edge = Some(parse_value(&flag, &next_value(&flag)?)?),
            "-m" | "--monitor" => opts.monitor = Some(parse_value(&flag, &next_value(&flag)?)?),
            "-fn" | "--font" => opts.font = Some(next_value(&flag)?),
            "--font-size" => opts.font_size = Some(parse_number(&flag, &next_value(&flag)?)?),
            "--height" => opts.height = Some(parse_number(&flag, &next_value(&flag)?)?),
            "--margin" => opts.margin = Some(parse_number(&flag, &next_value(&flag)?)?),
            "--offset" => opts.offset = Some(parse_number(&flag, &next_value(&flag)?)?),
            "--width" => opts.width = Some(parse_value(&flag, &next_value(&flag)?)?),
            "--align" => opts.align = Some(parse_value(&flag, &next_value(&flag)?)?),
            "--icon-size" => opts.icon_size = Some(parse_number(&flag, &next_value(&flag)?)?),
            "--no-icons" => opts.no_icons = true,
            "--terminal" | "--term" => opts.term = Some(next_value(&flag)?),
            "-d" | "--dmenu" => opts.dmenu = true,
            "--exec" => opts.exec = true,
            "--print" => opts.print = true,
            "--case-sensitive" => opts.case_sensitive = Some(true),
            "-i" | "--ignore-case" => opts.case_sensitive = Some(false),
            "--match" => opts.match_mode = Some(parse_value(&flag, &next_value(&flag)?)?),
            "-nb" | "--normal-bg" => opts.normal_bg = Some(next_value(&flag)?),
            "-nf" | "--normal-fg" => opts.normal_fg = Some(next_value(&flag)?),
            "-sb" | "--selected-bg" => opts.selected_bg = Some(next_value(&flag)?),
            "-sf" | "--selected-fg" => opts.selected_fg = Some(next_value(&flag)?),
            "-l" | "--lines" => {
                let _ = next_value(&flag)?;
                opts.warnings
                    .push("-l (vertical list) is not supported yet; using a single line".into());
            }
            "-f" => {
                // dmenu's "grab keyboard before reading stdin": we always grab early.
            }
            "-w" | "--embed" => {
                let _ = next_value(&flag)?;
                opts.warnings
                    .push("-w (embedding) is not supported yet".into());
            }
            other => bail!("unknown option `{other}` (use --help)"),
        }
    }

    Ok(opts)
}

fn parse_number<T: std::str::FromStr>(flag: &str, raw: &str) -> Result<T> {
    raw.trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid value for `{flag}`: `{raw}`"))
}

/// Parse a value whose `FromStr::Err` is a human readable message.
fn parse_value<T>(flag: &str, raw: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    raw.trim()
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid value for `{flag}`: {error}"))
}

/// Apply CLI overrides on top of the loaded configuration.
pub fn apply(opts: &Options, cfg: &mut Config) -> Result<()> {
    if let Some(prompt) = &opts.prompt {
        cfg.appearance.prompt = prompt.clone();
    }
    if let Some(edge) = opts.edge {
        cfg.bar.edge = edge;
    }
    if let Some(monitor) = &opts.monitor {
        cfg.monitor.target = monitor.clone();
    }
    if let Some(font) = &opts.font {
        cfg.appearance.font = font.clone();
    }
    if let Some(size) = opts.font_size {
        cfg.appearance.font_size = size;
    }
    if let Some(height) = opts.height {
        cfg.bar.height = height;
    }
    if let Some(margin) = opts.margin {
        cfg.bar.margin = margin;
    }
    if let Some(offset) = opts.offset {
        cfg.bar.offset = offset;
    }
    if let Some(width) = opts.width {
        cfg.bar.width = Some(width);
    }
    if let Some(align) = opts.align {
        cfg.bar.align = align;
    }
    if let Some(size) = opts.icon_size {
        cfg.appearance.icon_size = size;
    }
    if opts.no_icons {
        cfg.appearance.show_icons = false;
    }
    if let Some(term) = &opts.term {
        cfg.behavior.terminal = term.clone();
    }
    if opts.print {
        cfg.behavior.print_selection = true;
    }
    if opts.dmenu && opts.prompt.is_none() {
        // dmenu's default prompt is empty; `-p` still wins.
        cfg.appearance.prompt.clear();
    }
    if let Some(cs) = opts.case_sensitive {
        cfg.behavior.case_sensitive = cs;
    }
    if let Some(mode) = opts.match_mode {
        cfg.behavior.match_mode = mode;
    }

    for (raw, slot) in [
        (&opts.normal_bg, &mut cfg.style.bg),
        (&opts.normal_fg, &mut cfg.style.fg),
        (&opts.selected_bg, &mut cfg.style.selection_bg),
        (&opts.selected_fg, &mut cfg.style.selection_fg),
    ] {
        if let Some(raw) = raw {
            *slot = raw
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid color `{raw}`: {e}"))?;
        }
    }

    Ok(())
}

pub fn help() -> &'static str {
    HELP
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Color, MatchMode};

    fn parse_args(args: &[&str]) -> Result<Options> {
        parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn empty_arguments_give_defaults() {
        let opts = parse_args(&[]).unwrap();
        assert!(!opts.help && !opts.version && !opts.dmenu && !opts.print);
        assert!(opts.prompt.is_none() && opts.edge.is_none() && opts.warnings.is_empty());
    }

    #[test]
    fn flags_accept_separate_and_inline_values() {
        let opts = parse_args(&["-p", "Go: ", "--font=Iosevka", "--height", "24"]).unwrap();
        assert_eq!(opts.prompt.as_deref(), Some("Go: "));
        assert_eq!(opts.font.as_deref(), Some("Iosevka"));
        assert_eq!(opts.height, Some(24));

        // Only the first `=` splits; the rest belongs to the value.
        let opts = parse_args(&["--prompt=a=b"]).unwrap();
        assert_eq!(opts.prompt.as_deref(), Some("a=b"));
    }

    #[test]
    fn values_that_look_like_flags_are_taken_verbatim() {
        let opts = parse_args(&["-p", "--dmenu"]).unwrap();
        assert_eq!(opts.prompt.as_deref(), Some("--dmenu"));
        assert!(!opts.dmenu);
    }

    #[test]
    fn positional_argument_with_equals_is_rejected() {
        let error = parse_args(&["key=value"]).unwrap_err().to_string();
        assert!(error.contains("unknown option `key=value`"), "{error}");
    }

    #[test]
    fn missing_value_is_an_error() {
        let error = parse_args(&["--theme"]).unwrap_err().to_string();
        assert_eq!(error, "option `--theme` requires a value");
    }

    #[test]
    fn short_and_long_aliases_match() {
        let pairs: &[(&[&str], &[&str])] = &[
            (&["-b"], &["--edge", "bottom"]),
            (&["-d"], &["--dmenu"]),
            (&["-fn", "Mono"], &["--font", "Mono"]),
            (&["--term", "st"], &["--terminal", "st"]),
            (&["--themes"], &["--list-themes"]),
            (&["-m", "1"], &["--monitor=1"]),
        ];
        for (short, long) in pairs {
            let a = format!("{:?}", parse_args(short).unwrap());
            let b = format!("{:?}", parse_args(long).unwrap());
            assert_eq!(a, b, "{short:?} vs {long:?}");
        }
    }

    #[test]
    fn typed_values_are_parsed() {
        let opts = parse_args(&[
            "--edge",
            "Bottom",
            "--monitor",
            "HDMI-A-0",
            "--width",
            "50%",
            "--align",
            "center",
            "--match",
            "prefix",
            "--font-size",
            " 11.5 ",
        ])
        .unwrap();
        assert_eq!(opts.edge, Some(Edge::Bottom));
        assert_eq!(opts.monitor, Some(MonitorTarget::Name("HDMI-A-0".into())));
        assert_eq!(opts.width, Some(Dimension::Percent(50.0)));
        assert_eq!(opts.align, Some(Align::Center));
        assert_eq!(opts.match_mode, Some(MatchMode::Prefix));
        assert_eq!(opts.font_size, Some(11.5));
    }

    #[test]
    fn invalid_values_name_the_flag() {
        for (args, needle) in [
            (
                &["--height", "tall"][..],
                "invalid value for `--height`: `tall`",
            ),
            (
                &["--icon-size", "-3"][..],
                "invalid value for `--icon-size`",
            ),
            (&["--align", "middle"][..], "invalid value for `--align`"),
            (&["--edge=left"][..], "invalid value for `--edge`"),
            (&["--match", "regex"][..], "invalid value for `--match`"),
        ] {
            let error = parse_args(args).unwrap_err().to_string();
            assert!(error.contains(needle), "{args:?}: {error}");
        }
    }

    #[test]
    fn case_flags_last_one_wins() {
        let opts = parse_args(&["--case-sensitive", "-i"]).unwrap();
        assert_eq!(opts.case_sensitive, Some(false));
        let opts = parse_args(&["-i", "--case-sensitive"]).unwrap();
        assert_eq!(opts.case_sensitive, Some(true));
    }

    #[test]
    fn unsupported_dmenu_flags_warn_and_consume_their_value() {
        let opts = parse_args(&["-l", "10", "-w", "0x123", "-f", "--print"]).unwrap();
        assert_eq!(opts.warnings.len(), 2);
        assert!(opts.print);

        assert!(parse_args(&["-l"]).is_err());
    }

    #[test]
    fn apply_overrides_config() {
        let opts = parse_args(&[
            "-p",
            "> ",
            "-b",
            "--height=30",
            "--margin=4",
            "--offset=2",
            "--width=800",
            "--align=right",
            "--icon-size=16",
            "--no-icons",
            "--terminal=foot",
            "--print",
            "--case-sensitive",
            "--match=substring",
            "-fn",
            "Serif",
            "--font-size=10",
            "-m",
            "primary",
        ])
        .unwrap();
        let mut cfg = Config::default();
        apply(&opts, &mut cfg).unwrap();
        assert_eq!(cfg.appearance.prompt, "> ");
        assert_eq!(cfg.bar.edge, Edge::Bottom);
        assert_eq!(cfg.bar.height, 30);
        assert_eq!(cfg.bar.margin, 4);
        assert_eq!(cfg.bar.offset, 2);
        assert_eq!(cfg.bar.width, Some(Dimension::Px(800)));
        assert_eq!(cfg.bar.align, Align::Right);
        assert_eq!(cfg.appearance.icon_size, 16);
        assert!(!cfg.appearance.show_icons);
        assert_eq!(cfg.behavior.terminal, "foot");
        assert!(cfg.behavior.print_selection);
        assert!(cfg.behavior.case_sensitive);
        assert_eq!(cfg.behavior.match_mode, MatchMode::Substring);
        assert_eq!(cfg.appearance.font, "Serif");
        assert_eq!(cfg.appearance.font_size, 10.0);
        assert_eq!(cfg.monitor.target, MonitorTarget::Primary);
    }

    #[test]
    fn apply_without_flags_keeps_config() {
        let mut cfg = Config::default();
        let before = format!("{cfg:?}");
        apply(&Options::default(), &mut cfg).unwrap();
        assert_eq!(format!("{cfg:?}"), before);
    }

    #[test]
    fn dmenu_clears_prompt_unless_given() {
        let mut cfg = Config::default();
        apply(&parse_args(&["--dmenu"]).unwrap(), &mut cfg).unwrap();
        assert!(cfg.appearance.prompt.is_empty());

        let mut cfg = Config::default();
        apply(&parse_args(&["--dmenu", "-p", "pick"]).unwrap(), &mut cfg).unwrap();
        assert_eq!(cfg.appearance.prompt, "pick");
    }

    #[test]
    fn apply_parses_dmenu_colors() {
        let opts = parse_args(&[
            "-nb", "#000", "-nf", "#fff", "-sb", "#112233", "-sf", "white",
        ])
        .unwrap();
        let mut cfg = Config::default();
        apply(&opts, &mut cfg).unwrap();
        assert_eq!(cfg.style.bg, Color::rgb(0, 0, 0));
        assert_eq!(cfg.style.fg, Color::rgb(255, 255, 255));
        assert_eq!(cfg.style.selection_bg, Color::rgb(0x11, 0x22, 0x33));
        assert_eq!(cfg.style.selection_fg, Color::rgb(255, 255, 255));
    }

    #[test]
    fn apply_rejects_bad_colors() {
        let opts = parse_args(&["-sb", "#12"]).unwrap();
        let error = apply(&opts, &mut Config::default())
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("invalid color `#12`"), "{error}");
    }

    #[test]
    fn help_mentions_every_long_flag() {
        for flag in [
            "--config",
            "--theme",
            "--themes",
            "--prompt",
            "--bottom",
            "--edge",
            "--monitor",
            "--height",
            "--margin",
            "--offset",
            "--width",
            "--align",
            "--font",
            "--font-size",
            "--icon-size",
            "--no-icons",
            "--terminal",
            "--case-sensitive",
            "--match",
            "--dmenu",
            "--exec",
            "--print",
            "--normal-bg",
            "--normal-fg",
            "--selected-bg",
            "--selected-fg",
            "--help",
            "--version",
        ] {
            assert!(help().contains(flag), "help is missing {flag}");
        }
    }
}
