# Configuration

The launcher works without a config file. A TOML file can override any default;
see [`config.example.toml`](../config.example.toml) for a commented version.

## File locations

The first existing file wins:

1. `--config <FILE>` (must exist; an error is reported otherwise)
2. `$LAUNCHER_CONFIG`
3. `$XDG_CONFIG_HOME/launcher-rs/config.toml`
4. `~/.config/launcher-rs/config.toml`

Unknown keys are rejected (`deny_unknown_fields`), so typos produce a clear
error instead of being silently ignored.

Command-line flags override the file. Colors written explicitly in `[style]`
always win over the selected theme; `--theme` replaces the theme name from the
file and the same merge is applied.

## Example

```toml
[bar]
edge = "top"
height = 20
margin = 0
offset = 0
width = "100%"
align = "left"

[monitor]
target = "focused"

[appearance]
font = "Sans"
font_size = 12.0
font_weight = 400
prompt = "Run: "
icon_theme = ""
icon_size = 14
show_icons = true

[style]
theme = ""

[behavior]
match_mode = "fuzzy"
case_sensitive = false
sort = true
terminal = ""
print_selection = false
shell = "/bin/sh"
fallback_icon = "application-x-executable"
max_results = 0
```

## `[bar]`

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `edge` | `top` \| `bottom` | `top` | Monitor edge where the strip appears |
| `height` | pixels | `20` | Strip height |
| `offset` | pixels | `0` | Distance from the edge (use the status bar height to sit below it) |
| `margin` | pixels | `0` | Outer horizontal margin, applied on both sides |
| `width` | pixels \| `"80%"` | monitor width minus margins | Strip width |
| `align` | `left` \| `center` \| `right` | `left` | Alignment when the width is narrower than the monitor |

## `[monitor]`

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `target` | `focused` \| `primary` \| `pointer` \| index \| name | `focused` | Monitor used for positioning |

`focused` follows the monitor containing the active window and falls back to
the pointer, then to the primary monitor.

## `[appearance]`

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `font` | string | `"Sans"` | Primary font family; cosmic-text handles fallbacks |
| `font_size` | pixels | `12.0` | Font size |
| `font_weight` | `100`–`900` | `400` | CSS-style font weight (`700` is bold) |
| `prompt` | string | `"Run: "` | Prompt drawn before the input field |
| `placeholder` | string | unset | Hint drawn while the query is empty |
| `icon_theme` | string | `""` | Icon theme; empty auto-detects via GTK and falls back to Adwaita/hicolor |
| `icon_size` | pixels | `14` | Rendered icon size |
| `show_icons` | bool | `true` | Draw icons |

## `[style]`

Everything visual lives in `[style]`. Colors accept `#rgb`, `#rrggbb`,
`#rrggbbaa`, `0x…`, `black`, `white` and `transparent`.

### Colors

| Key | Default | Description |
| --- | --- | --- |
| `theme` | `""` | Built-in theme name; empty uses the defaults below |
| `bg` | `#dedede` | Strip background |
| `fg` | `#333333` | Item text |
| `prompt_fg` | `#333333` | Prompt text |
| `input_fg` | `#333333` | Typed text |
| `cursor` | `#333333` | Text cursor |
| `placeholder_fg` | `#333333` | Placeholder text |
| `selection_bg` | `#c5c5c5` | Selected item background |
| `selection_fg` | `#000000` | Selected item text |
| `scroll_fg` | `#333333` | `<` and `>` scroll indicators |
| `highlight_fg` | unset | Matched characters; unset disables highlighting |
| `panel_border` | `#c5c5c5` | Inner border around the whole strip |
| `selection_border` | `#ff8c00` | Outline of the selected item |
| `input_border` | `#c5c5c5` | Outline of the optional input box |

### Geometry and borders

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `padding` | px | `2.0` | Inner strip padding |
| `gap` | px | `0.0` | Gap between icon/text, prompt/input and list edges |
| `item_gap` | px | `0.0` | Gap between item cells |
| `item_padding` | px | `10.0` | Horizontal padding inside a cell |
| `search_width` | px \| percentage | `120` | Width of the search field |
| `cursor_width` | px | `1.5` | Text cursor width |
| `icon_radius` | px | `0.0` | Corner radius of the placeholder icon |
| `selection_radius` | px | `0.0` | Corner radius of the selection |
| `selection_border_width` | px | `1.0` | Selection border width |
| `input_bg` | color | unset | Input box background; unset disables the box |
| `input_radius` | px | `0.0` | Input box corner radius |
| `input_border_width` | px | `0.0` | Input box border width |
| `panel_border_width` | px | `0.0` | Strip border width |

## `[behavior]`

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `match_mode` | `prefix` \| `substring` \| `fuzzy` | `fuzzy` | Search strategy |
| `case_sensitive` | bool | `false` | Case-sensitive matching |
| `sort` | bool | `true` | Sort by score; `false` keeps scan/stdin order |
| `terminal` | string | `""` | Terminal for `Terminal=true` entries (e.g. `"alacritty -e"`); empty uses `$TERMINAL` and probes known terminals |
| `print_selection` | bool | `false` | Print instead of launching |
| `shell` | string | `"/bin/sh"` | Shell used by `--dmenu --exec` |
| `fallback_icon` | string | `"application-x-executable"` | Icon used when an entry has no resolvable icon |
| `max_results` | integer | `0` | Maximum number of results (`0` = unlimited) |

## Themes

The built-in themes are:

`catppuccin-mocha`, `catppuccin-macchiato`, `catppuccin-frappe`,
`catppuccin-latte`, `gruvbox-dark`, `gruvbox-light`, `nord`, `dracula`,
`tokyo-night`, `solarized-dark`, `solarized-light`, `one-dark`, `rose-pine`
and `adwaita-dark`.

```bash
launcher-rs --themes
launcher-rs --theme catppuccin-latte
```

The default (`theme = ""`) is the translation of `dmenu.rasi`: background
`#DEDEDE`, text `#333333`, selection `#C5C5C5` with `#000000` text and a 1px
`#FF8C00` border, square corners, `window padding 2px`,
`element padding 0px 10px`, height 20px, `entry 10em` and `font Sans 9`
(9pt ≈ 12px).

For a rounded look with an input box and a panel border:

```toml
[style]
theme = "tokyo-night"
selection_radius = 10.0
selection_border_width = 2.0
selection_border = "#9ece6a"
input_bg = "#00000066"
input_radius = 10.0
input_border_width = 1.5
input_border = "#7aa2f7"
panel_border_width = 2.0
panel_border = "#7aa2f7"
highlight_fg = "#9ece6a"      # enables match highlighting
```

## dmenu-compatible color flags

`-nb/--normal-bg`, `-nf/--normal-fg`, `-sb/--selected-bg` and
`-sf/--selected-fg` override the corresponding `[style]` colors.
