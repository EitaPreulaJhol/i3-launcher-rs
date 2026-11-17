//! Configuration: defaults, TOML loading and CLI overrides.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

/// Directory name used under `$XDG_CONFIG_HOME`.
pub const CONFIG_DIR: &str = "i3-launcher";
/// File name searched inside [`CONFIG_DIR`].
pub const CONFIG_FILE: &str = "config.toml";

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// Straight (non-premultiplied) sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    /// Premultiplied RGBA, the format used internally by `tiny-skia`.
    pub fn premultiplied(&self) -> [u8; 4] {
        let a = self.a as u32;
        [
            ((self.r as u32 * a) / 255) as u8,
            ((self.g as u32 * a) / 255) as u8,
            ((self.b as u32 * a) / 255) as u8,
            self.a,
        ]
    }

    pub fn with_alpha(&self, a: u8) -> Self {
        Self { a, ..*self }
    }

    pub fn to_tiny_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        match s.to_ascii_lowercase().as_str() {
            "black" => return Ok(Self::rgb(0, 0, 0)),
            "white" => return Ok(Self::rgb(255, 255, 255)),
            "transparent" => return Ok(Self::rgba(0, 0, 0, 0)),
            _ => {}
        }

        let hex = s
            .strip_prefix('#')
            .or_else(|| s.strip_prefix("0x"))
            .ok_or_else(|| format!("cor inválida `{s}`: use #rrggbb, #rrggbbaa ou #rgb"))?;

        let digit = |c: char| {
            c.to_digit(16)
                .map(|d| d as u8)
                .ok_or_else(|| format!("cor inválida `{s}`: caractere não hexadecimal"))
        };

        let mut chars = hex.chars();
        let bytes: Vec<u8> = match hex.len() {
            3 | 4 => {
                let mut out = Vec::with_capacity(4);
                for _ in 0..hex.len() {
                    let c = chars.next().unwrap();
                    let d = digit(c)?;
                    out.push(d * 17);
                }
                out
            }
            6 | 8 => {
                let mut out = Vec::with_capacity(4);
                while let (Some(hi), Some(lo)) = (chars.next(), chars.next()) {
                    out.push(digit(hi)? * 16 + digit(lo)?);
                }
                out
            }
            n => return Err(format!("cor inválida `{s}`: {n} dígitos")),
        };

        Ok(Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
            a: *bytes.get(3).unwrap_or(&255),
        })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Dimension
// ---------------------------------------------------------------------------

/// A length that is either absolute pixels or a percentage of the monitor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Px(u32),
    Percent(f32),
}

impl Dimension {
    pub fn resolve(self, total: u32) -> u32 {
        match self {
            Dimension::Px(px) => px,
            Dimension::Percent(pct) => ((total as f32) * (pct / 100.0)).round().max(0.0) as u32,
        }
    }
}

impl FromStr for Dimension {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(pct) = s.strip_suffix('%') {
            let v: f32 = pct
                .trim()
                .parse()
                .map_err(|_| format!("porcentagem inválida `{s}`"))?;
            return Ok(Dimension::Percent(v));
        }
        let px: u32 = s
            .parse()
            .map_err(|_| format!("dimensão inválida `{s}` (use px ou %)"))?;
        Ok(Dimension::Px(px))
    }
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Dimension;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("um número de pixels ou uma string percentual (ex.: \"80%\")")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Dimension::Px(v.min(u32::MAX as u64) as u32))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Dimension::Px(v.max(0).min(u32::MAX as i64) as u32))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Dimension::Px(v.max(0.0).round() as u32))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

macro_rules! impl_enum_serde {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim().to_ascii_lowercase().as_str() {
                    $($text => Ok($name::$variant),)+
                    other => Err(format!(
                        "valor inválido `{other}`; esperado um de: {}",
                        [$($text),+].join(", ")
                    )),
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                String::deserialize(deserializer)?
                    .parse()
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}

/// Edge of the monitor where the strip is anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edge {
    #[default]
    Top,
    Bottom,
}

impl_enum_serde!(Edge {
    Top => "top",
    Bottom => "bottom",
});

/// Horizontal alignment when the strip is narrower than the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

impl_enum_serde!(Align {
    Left => "left",
    Center => "center",
    Right => "right",
});

/// Matching strategy for the search engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchMode {
    Prefix,
    Substring,
    #[default]
    Fuzzy,
}

impl_enum_serde!(MatchMode {
    Prefix => "prefix",
    Substring => "substring",
    Fuzzy => "fuzzy",
});

/// Which monitor the strip is placed on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MonitorTarget {
    /// Monitor containing the active window (falls back to pointer, then primary).
    #[default]
    Focused,
    Primary,
    /// Monitor under the mouse pointer.
    Pointer,
    /// RandR monitor index.
    Index(usize),
    /// RandR monitor connector name (e.g. `HDMI-A-0`).
    Name(String),
}

impl FromStr for MonitorTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        match s.to_ascii_lowercase().as_str() {
            "focused" => Ok(MonitorTarget::Focused),
            "primary" => Ok(MonitorTarget::Primary),
            "pointer" | "mouse" => Ok(MonitorTarget::Pointer),
            _ => {
                if let Ok(idx) = s.parse::<usize>() {
                    Ok(MonitorTarget::Index(idx))
                } else if s.is_empty() {
                    Err("monitor vazio".into())
                } else {
                    Ok(MonitorTarget::Name(s.to_string()))
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for MonitorTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarConfig {
    /// `top` or `bottom`.
    pub edge: Edge,
    /// Distance in pixels from the monitor edge.
    pub offset: u32,
    /// Strip height in pixels.
    pub height: u32,
    /// Outer horizontal margin in pixels (on each side).
    pub margin: u32,
    /// Strip width; `None` means the monitor width minus margins.
    pub width: Option<Dimension>,
    /// Horizontal alignment when `width` is narrower than the monitor.
    pub align: Align,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            edge: Edge::Top,
            offset: 0,
            height: 20,
            margin: 0,
            width: None,
            align: Align::Left,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MonitorConfig {
    pub target: MonitorTarget,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    /// Primary font family. Fallbacks are handled by cosmic-text.
    pub font: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// CSS-style font weight (100 = thin, 400 = normal, 700 = bold).
    pub font_weight: u16,
    /// Prompt shown before the input field.
    pub prompt: String,
    /// Placeholder drawn in the input field while the query is empty (optional).
    pub placeholder: Option<String>,
    /// Icon theme name; empty means auto-detect.
    pub icon_theme: String,
    /// Rendered icon size in pixels.
    pub icon_size: u32,
    /// Draw application icons.
    pub show_icons: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font: "DejaVu Sans".to_string(),
            font_size: 14.0,
            font_weight: 400,
            prompt: "Run: ".to_string(),
            placeholder: None,
            icon_theme: String::new(),
            icon_size: 24,
            show_icons: true,
        }
    }
}

/// Everything visual: colors, spacing, radii and borders.
///
/// `theme` picks one of the built-in palettes; colors set explicitly in this
/// section always win over the palette.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Style {
    /// Built-in theme name (see `--themes`); empty means "use only the keys".
    pub theme: String,

    pub bg: Color,
    pub fg: Color,
    pub prompt_fg: Color,
    pub input_fg: Color,
    pub cursor: Color,
    pub placeholder_fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub scroll_fg: Color,
    /// Color of the characters matched by the query.
    pub highlight_fg: Color,

    /// Inner border around the whole strip.
    pub panel_border: Color,
    pub panel_border_width: f32,

    /// Outline of the selected item.
    pub selection_border: Color,
    pub selection_border_width: f32,
    pub selection_radius: f32,

    /// Optional rounded box behind the input field. Omit to disable it.
    pub input_bg: Option<Color>,
    pub input_border: Color,
    pub input_border_width: f32,
    pub input_radius: f32,

    /// Corner radius used by the placeholder icon of entries without an icon.
    pub icon_radius: f32,
    /// Width of the text cursor.
    pub cursor_width: f32,

    /// Inner padding of the strip.
    pub padding: f32,
    /// Gap between icon and label, prompt and input, and list edges.
    pub gap: f32,
    /// Gap between items.
    pub item_gap: f32,
    /// Horizontal padding inside an item cell.
    pub item_padding: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            theme: String::new(),
            bg: Color::rgb(0x1e, 0x1e, 0x2e),
            fg: Color::rgb(0xcd, 0xd6, 0xf4),
            prompt_fg: Color::rgb(0xf9, 0xe2, 0xaf),
            input_fg: Color::rgb(0xcd, 0xd6, 0xf4),
            cursor: Color::rgb(0xf5, 0xe0, 0xdc),
            placeholder_fg: Color::rgb(0x6c, 0x70, 0x86),
            selection_bg: Color::rgb(0x89, 0xb4, 0xfa),
            selection_fg: Color::rgb(0x1e, 0x1e, 0x2e),
            scroll_fg: Color::rgb(0x6c, 0x70, 0x86),
            highlight_fg: Color::rgb(0xfa, 0xb3, 0x87),
            panel_border: Color::rgb(0x11, 0x11, 0x1b),
            panel_border_width: 0.0,
            selection_border: Color::rgb(0x45, 0x47, 0x5a),
            selection_border_width: 0.0,
            selection_radius: 0.0,
            input_bg: None,
            input_border: Color::rgb(0x89, 0xb4, 0xfa),
            input_border_width: 0.0,
            input_radius: 0.0,
            icon_radius: 0.0,
            cursor_width: 1.5,
            padding: 8.0,
            gap: 8.0,
            item_gap: 4.0,
            item_padding: 8.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Behavior {
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    /// Sort results by match score (dmenu keeps input order).
    pub sort: bool,
    /// Terminal command used for entries with `Terminal=true`
    /// (e.g. `"alacritty -e"`). Empty means `$TERMINAL`.
    pub terminal: String,
    /// Print the selection to stdout instead of launching it.
    pub print_selection: bool,
    /// Command run in a shell for `--dmenu --exec`.
    pub shell: String,
    /// Icon name used when an entry's icon cannot be resolved.
    pub fallback_icon: String,
    /// Maximum number of results kept (0 = unlimited).
    pub max_results: usize,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            match_mode: MatchMode::Fuzzy,
            case_sensitive: false,
            sort: true,
            terminal: String::new(),
            print_selection: false,
            shell: "/bin/sh".to_string(),
            fallback_icon: "application-x-executable".to_string(),
            max_results: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub bar: BarConfig,
    pub monitor: MonitorConfig,
    pub appearance: Appearance,
    pub style: Style,
    pub behavior: Behavior,
}

impl Config {
    /// Load the configuration.
    ///
    /// An explicitly given path (from `--config`) must exist. Otherwise the
    /// search path is `$I3L_CONFIG`, the XDG config dir and finally the HOME
    /// fallback; missing files yield the built-in defaults.
    ///
    /// `theme` overrides the theme written in the file (from `--theme`).
    pub fn load(explicit: Option<&Path>, theme: Option<&str>) -> Result<Self> {
        if let Some(path) = explicit {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("não foi possível ler `{}`", path.display()))?;
            return Self::from_str(&raw, theme)
                .with_context(|| format!("config `{}` inválido", path.display()));
        }

        for path in Self::search_paths() {
            if !path.is_file() {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("não foi possível ler `{}`", path.display()))?;
            return Self::from_str(&raw, theme)
                .with_context(|| format!("config `{}` inválido", path.display()));
        }

        Self::from_str("", theme)
    }

    /// Parse a TOML document, resolving the `[style]` theme first.
    pub fn from_str(raw: &str, theme: Option<&str>) -> Result<Self> {
        let mut value: toml::Value = if raw.trim().is_empty() {
            toml::Value::Table(Default::default())
        } else {
            toml::from_str(raw)?
        };

        if let Some(theme) = theme {
            set_theme(&mut value, theme)?;
        }
        apply_theme(&mut value)?;

        let config: Config = value.try_into()?;
        Ok(config)
    }

    /// Convenience wrapper used by tests and tools.
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_str(&raw, None)
    }

    /// Candidate config locations, highest priority first.
    pub fn search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(p) = std::env::var_os("I3L_CONFIG") {
            if !p.is_empty() {
                paths.push(PathBuf::from(p));
            }
        }
        if let Some(dir) = dirs::config_dir() {
            paths.push(dir.join(CONFIG_DIR).join(CONFIG_FILE));
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config").join(CONFIG_DIR).join(CONFIG_FILE));
        }
        paths
    }
}

fn style_table(value: &mut toml::Value) -> Result<&mut toml::value::Table> {
    let root = value
        .as_table_mut()
        .context("o config TOML deve ser uma tabela")?;
    let style = root
        .entry("style".to_string())
        .or_insert_with(|| toml::Value::Table(Default::default()));
    style
        .as_table_mut()
        .context("`[style]` deve ser uma tabela")
}

fn set_theme(value: &mut toml::Value, theme: &str) -> Result<()> {
    style_table(value)?.insert(
        "theme".to_string(),
        toml::Value::String(theme.trim().to_string()),
    );
    Ok(())
}

/// Fill the style colors from the selected theme, without overriding keys the
/// user wrote explicitly.
fn apply_theme(value: &mut toml::Value) -> Result<()> {
    let name = value
        .get("style")
        .and_then(|style| style.get("theme"))
        .and_then(|theme| theme.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    if name.is_empty() {
        return Ok(());
    }

    let palette = crate::theme::find(&name).ok_or_else(|| {
        anyhow::anyhow!(
            "tema desconhecido `{name}`; disponíveis: {}",
            crate::theme::names().collect::<Vec<_>>().join(", ")
        )
    })?;

    let table = style_table(value)?;
    for (key, color) in palette.colors() {
        table
            .entry(key.to_string())
            .or_insert_with(|| toml::Value::String(color.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_is_valid_and_matches_defaults() {
        let raw = include_str!("../config.example.toml");
        let parsed = Config::from_str(raw, None).expect("config.example.toml inválido");
        let defaults = Config::default();
        // Keep the documented example in sync with the compiled defaults.
        assert_eq!(parsed.bar.edge, defaults.bar.edge);
        assert_eq!(parsed.bar.height, defaults.bar.height);
        assert_eq!(parsed.behavior.match_mode, defaults.behavior.match_mode);
        assert_eq!(parsed.appearance.font, defaults.appearance.font);
        assert_eq!(parsed.style.bg, defaults.style.bg);
        assert_eq!(
            parsed.style.selection_radius,
            defaults.style.selection_radius
        );
    }

    #[test]
    fn every_theme_deserializes() {
        for name in crate::theme::names() {
            let raw = format!("[style]\ntheme = \"{name}\"\n");
            Config::from_str(&raw, None)
                .unwrap_or_else(|err| panic!("tema `{name}` falhou: {err:#}"));
        }
    }

    #[test]
    fn theme_fills_colors_and_explicit_keys_win() {
        let raw = "[style]\ntheme = \"gruvbox-dark\"\nbg = \"#000000\"\n";
        let cfg = Config::from_str(raw, None).unwrap();
        assert_eq!(cfg.style.theme, "gruvbox-dark");
        assert_eq!(cfg.style.bg, Color::rgb(0, 0, 0));
        assert_eq!(cfg.style.fg, "#ebdbb2".parse().unwrap());
        assert_eq!(cfg.style.selection_bg, "#458588".parse().unwrap());
    }

    #[test]
    fn cli_theme_override_replaces_the_file_theme() {
        let raw = "[style]\ntheme = \"nord\"\n";
        let cfg = Config::from_str(raw, Some("dracula")).unwrap();
        assert_eq!(cfg.style.theme, "dracula");
        assert_eq!(cfg.style.bg, "#282a36".parse().unwrap());
    }

    #[test]
    fn explicit_colors_win_over_cli_theme() {
        let cfg = Config::from_str("[style]\nbg = \"#010101\"\n", Some("dracula")).unwrap();
        assert_eq!(cfg.style.bg, Color::rgb(1, 1, 1));
        assert_eq!(cfg.style.fg, "#f8f8f2".parse().unwrap());
    }

    #[test]
    fn unknown_theme_lists_available_names() {
        let err = Config::from_str("[style]\ntheme = \"nao-existe\"\n", None).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("nao-existe"));
        assert!(message.contains("catppuccin-mocha"));
    }

    #[test]
    fn style_defaults_are_square() {
        let style = Style::default();
        assert_eq!(style.selection_radius, 0.0);
        assert_eq!(style.input_radius, 0.0);
        assert_eq!(style.icon_radius, 0.0);
    }

    #[test]
    fn style_geometry_is_loaded() {
        let raw = "[style]\nselection_radius = 10.0\ninput_bg = \"#00000055\"\npanel_border_width = 2.0\n";
        let cfg = Config::from_str(raw, None).unwrap();
        assert_eq!(cfg.style.selection_radius, 10.0);
        assert_eq!(cfg.style.input_bg, Some(Color::rgba(0, 0, 0, 0x55)));
        assert_eq!(cfg.style.panel_border_width, 2.0);
    }

    #[test]
    fn rejects_unknown_keys() {
        let err = toml::from_str::<Config>("[bar]\nheigth = 42\n").unwrap_err();
        assert!(err.to_string().contains("heigth"));
    }

    #[test]
    fn parses_colors_and_dimensions() {
        assert_eq!(
            "#1e1e2e".parse::<Color>().unwrap(),
            Color::rgb(0x1e, 0x1e, 0x2e)
        );
        assert_eq!(
            "#abc".parse::<Color>().unwrap(),
            Color::rgb(0xaa, 0xbb, 0xcc)
        );
        assert_eq!(
            "#11223344".parse::<Color>().unwrap(),
            Color::rgba(0x11, 0x22, 0x33, 0x44)
        );
        assert!("nope".parse::<Color>().is_err());
        assert_eq!("50%".parse::<Dimension>().unwrap().resolve(800), 400);
        assert_eq!("320".parse::<Dimension>().unwrap().resolve(800), 320);
    }
}
