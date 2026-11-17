//! Built-in color themes.
//!
//! A theme only provides colors; geometry (radii, borders, paddings) stays in
//! the `[style]` section of the config. Explicit color keys in the config
//! always win over the theme.

/// A named set of colors, stored as hex strings and injected into the TOML
/// document before it is deserialized.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub name: &'static str,
    pub bg: &'static str,
    pub fg: &'static str,
    pub prompt_fg: &'static str,
    pub input_fg: &'static str,
    pub cursor: &'static str,
    pub placeholder_fg: &'static str,
    pub selection_bg: &'static str,
    pub selection_fg: &'static str,
    pub scroll_fg: &'static str,
    pub highlight_fg: &'static str,
    pub panel_border: &'static str,
    pub selection_border: &'static str,
    pub input_border: &'static str,
}

impl Palette {
    /// Color keys exactly as they appear in `[style]`.
    pub fn colors(&self) -> [(&'static str, &'static str); 13] {
        [
            ("bg", self.bg),
            ("fg", self.fg),
            ("prompt_fg", self.prompt_fg),
            ("input_fg", self.input_fg),
            ("cursor", self.cursor),
            ("placeholder_fg", self.placeholder_fg),
            ("selection_bg", self.selection_bg),
            ("selection_fg", self.selection_fg),
            ("scroll_fg", self.scroll_fg),
            ("highlight_fg", self.highlight_fg),
            ("panel_border", self.panel_border),
            ("selection_border", self.selection_border),
            ("input_border", self.input_border),
        ]
    }
}

macro_rules! palette {
    (
        $name:literal,
        bg = $bg:literal, fg = $fg:literal, prompt = $prompt:literal,
        input = $input:literal, cursor = $cursor:literal, placeholder = $placeholder:literal,
        sel_bg = $sel_bg:literal, sel_fg = $sel_fg:literal, scroll = $scroll:literal,
        highlight = $highlight:literal, border = $border:literal,
        sel_border = $sel_border:literal, input_border = $input_border:literal $(,)?
    ) => {
        Palette {
            name: $name,
            bg: $bg,
            fg: $fg,
            prompt_fg: $prompt,
            input_fg: $input,
            cursor: $cursor,
            placeholder_fg: $placeholder,
            selection_bg: $sel_bg,
            selection_fg: $sel_fg,
            scroll_fg: $scroll,
            highlight_fg: $highlight,
            panel_border: $border,
            selection_border: $sel_border,
            input_border: $input_border,
        }
    };
}

/// All available themes. The built-in default style is separate and follows
/// `original_source/dmenu.rasi` (light polybar/i3 palette).
pub const PALETTES: &[Palette] = &[
    palette!(
        "catppuccin-mocha",
        bg = "#1e1e2e",
        fg = "#cdd6f4",
        prompt = "#f9e2af",
        input = "#cdd6f4",
        cursor = "#f5e0dc",
        placeholder = "#6c7086",
        sel_bg = "#89b4fa",
        sel_fg = "#1e1e2e",
        scroll = "#6c7086",
        highlight = "#fab387",
        border = "#11111b",
        sel_border = "#45475a",
        input_border = "#45475a",
    ),
    palette!(
        "catppuccin-macchiato",
        bg = "#24273a",
        fg = "#cad3f5",
        prompt = "#eed49f",
        input = "#cad3f5",
        cursor = "#f4dbd6",
        placeholder = "#6e738d",
        sel_bg = "#8aadf4",
        sel_fg = "#24273a",
        scroll = "#6e738d",
        highlight = "#f5a97f",
        border = "#181926",
        sel_border = "#494d64",
        input_border = "#494d64",
    ),
    palette!(
        "catppuccin-frappe",
        bg = "#303446",
        fg = "#c6d0f5",
        prompt = "#e5c890",
        input = "#c6d0f5",
        cursor = "#f2d5cf",
        placeholder = "#737994",
        sel_bg = "#8caaee",
        sel_fg = "#303446",
        scroll = "#737994",
        highlight = "#ef9f76",
        border = "#232634",
        sel_border = "#51576d",
        input_border = "#51576d",
    ),
    palette!(
        "catppuccin-latte",
        bg = "#eff1f5",
        fg = "#4c4f69",
        prompt = "#df8e1d",
        input = "#4c4f69",
        cursor = "#dc8a78",
        placeholder = "#9ca0b0",
        sel_bg = "#1e66f5",
        sel_fg = "#eff1f5",
        scroll = "#9ca0b0",
        highlight = "#fe640b",
        border = "#dce0e8",
        sel_border = "#bcc0cc",
        input_border = "#bcc0cc",
    ),
    palette!(
        "gruvbox-dark",
        bg = "#282828",
        fg = "#ebdbb2",
        prompt = "#fabd2f",
        input = "#ebdbb2",
        cursor = "#fbf1c7",
        placeholder = "#928374",
        sel_bg = "#458588",
        sel_fg = "#fbf1c7",
        scroll = "#928374",
        highlight = "#fe8019",
        border = "#1d2021",
        sel_border = "#504945",
        input_border = "#504945",
    ),
    palette!(
        "gruvbox-light",
        bg = "#fbf1c7",
        fg = "#3c3836",
        prompt = "#b57614",
        input = "#3c3836",
        cursor = "#af3a03",
        placeholder = "#928374",
        sel_bg = "#458588",
        sel_fg = "#fbf1c7",
        scroll = "#928374",
        highlight = "#af3a03",
        border = "#d5c4a1",
        sel_border = "#bdae93",
        input_border = "#bdae93",
    ),
    palette!(
        "nord",
        bg = "#2e3440",
        fg = "#d8dee9",
        prompt = "#ebcb8b",
        input = "#eceff4",
        cursor = "#88c0d0",
        placeholder = "#616e88",
        sel_bg = "#88c0d0",
        sel_fg = "#2e3440",
        scroll = "#4c566a",
        highlight = "#a3be8c",
        border = "#242933",
        sel_border = "#434c5e",
        input_border = "#434c5e",
    ),
    palette!(
        "dracula",
        bg = "#282a36",
        fg = "#f8f8f2",
        prompt = "#ffb86c",
        input = "#f8f8f2",
        cursor = "#f8f8f2",
        placeholder = "#6272a4",
        sel_bg = "#bd93f9",
        sel_fg = "#282a36",
        scroll = "#6272a4",
        highlight = "#50fa7b",
        border = "#191a21",
        sel_border = "#44475a",
        input_border = "#44475a",
    ),
    palette!(
        "tokyo-night",
        bg = "#1a1b26",
        fg = "#c0caf5",
        prompt = "#e0af68",
        input = "#c0caf5",
        cursor = "#c0caf5",
        placeholder = "#565f89",
        sel_bg = "#7aa2f7",
        sel_fg = "#1a1b26",
        scroll = "#565f89",
        highlight = "#9ece6a",
        border = "#16161e",
        sel_border = "#292e42",
        input_border = "#292e42",
    ),
    palette!(
        "solarized-dark",
        bg = "#002b36",
        fg = "#93a1a1",
        prompt = "#b58900",
        input = "#eee8d5",
        cursor = "#93a1a1",
        placeholder = "#586e75",
        sel_bg = "#268bd2",
        sel_fg = "#fdf6e3",
        scroll = "#586e75",
        highlight = "#2aa198",
        border = "#001f27",
        sel_border = "#073642",
        input_border = "#073642",
    ),
    palette!(
        "solarized-light",
        bg = "#fdf6e3",
        fg = "#586e75",
        prompt = "#b58900",
        input = "#073642",
        cursor = "#586e75",
        placeholder = "#93a1a1",
        sel_bg = "#268bd2",
        sel_fg = "#fdf6e3",
        scroll = "#93a1a1",
        highlight = "#cb4b16",
        border = "#eee8d5",
        sel_border = "#eee8d5",
        input_border = "#eee8d5",
    ),
    palette!(
        "one-dark",
        bg = "#282c34",
        fg = "#abb2bf",
        prompt = "#e5c07b",
        input = "#abb2bf",
        cursor = "#abb2bf",
        placeholder = "#5c6370",
        sel_bg = "#61afef",
        sel_fg = "#282c34",
        scroll = "#5c6370",
        highlight = "#98c379",
        border = "#21252b",
        sel_border = "#3e4451",
        input_border = "#3e4451",
    ),
    palette!(
        "rose-pine",
        bg = "#191724",
        fg = "#e0def4",
        prompt = "#f6c177",
        input = "#e0def4",
        cursor = "#e0def4",
        placeholder = "#6e6a86",
        sel_bg = "#c4a7e7",
        sel_fg = "#191724",
        scroll = "#6e6a86",
        highlight = "#9ccfd8",
        border = "#111019",
        sel_border = "#26233a",
        input_border = "#26233a",
    ),
    palette!(
        "adwaita-dark",
        bg = "#242424",
        fg = "#ffffff",
        prompt = "#78aeed",
        input = "#ffffff",
        cursor = "#ffffff",
        placeholder = "#9a9a9a",
        sel_bg = "#3584e4",
        sel_fg = "#ffffff",
        scroll = "#9a9a9a",
        highlight = "#99c1f1",
        border = "#1b1b1b",
        sel_border = "#5e5e5e",
        input_border = "#5e5e5e",
    ),
];

/// Look up a theme by name (case-insensitive).
pub fn find(name: &str) -> Option<&'static Palette> {
    let name = name.trim();
    PALETTES.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Theme names, in the order they are listed in the docs.
pub fn names() -> impl Iterator<Item = &'static str> {
    PALETTES.iter().map(|p| p.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Color;
    use std::collections::HashSet;

    #[test]
    fn every_palette_has_valid_colors() {
        for palette in PALETTES {
            for (key, value) in palette.colors() {
                value.parse::<Color>().unwrap_or_else(|err| {
                    panic!("theme {}: invalid color in {key}: {err}", palette.name)
                });
            }
        }
    }

    #[test]
    fn names_are_unique() {
        let names: HashSet<_> = names().collect();
        assert_eq!(names.len(), PALETTES.len());
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(find("GRUVBOX-DARK").unwrap().name, "gruvbox-dark");
        assert!(find("does-not-exist").is_none());
    }

    #[test]
    fn themes_keep_match_highlighting() {
        // The default style disables highlighting, but themes provide a color.
        for palette in PALETTES {
            palette
                .highlight_fg
                .parse::<Color>()
                .unwrap_or_else(|err| panic!("theme {}: {err}", palette.name));
        }
    }
}
