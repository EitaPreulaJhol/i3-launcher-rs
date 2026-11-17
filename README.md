# i3-launcher-rs

`dmenu`-style launcher for i3: a **horizontal strip** (at the top or bottom of
the monitor, configurable) with **application icons**, incremental search and
execution of `.desktop` entries — no GUI toolkit, just X11 + software
rendering.

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Run: firef▏   [🦊 Firefox]  [🌐 Chromium]  [📝 Gedit]  [⌨ Alacritty]      › │
└────────────────────────────────────────────────────────────────────────────┘
```

## Features

- `override-redirect` strip: i3 does not manage it, so there are no borders,
  gaps or stolen focus.
- Configurable position: top/bottom, offset, margins, width (px or %) and
  alignment.
- Multi-monitor via RandR 1.5 (Xinerama fallback): focused monitor, primary,
  under the pointer, by index or by connector name.
- Icons from the XDG Icon Theme spec, with theme inheritance, size selection,
  PNG resized with Lanczos and SVG rasterized at the final size.
- `prefix`, `substring` or `fuzzy` search with scoring; dmenu/readline-style
  line editing shortcuts.
- Correct execution of the Desktop Entry `Exec` (quoting, field codes,
  `Terminal=true` and `Path=`), with `setsid` when available.
- Two modes with `Tab`: `.desktop` applications (with icons) and `$PATH`
  commands (no icons, run directly). `PATH` is read on demand on the first Tab.
- dmenu mode (`--dmenu`) for pipes: reads lines from stdin and prints the
  selection.
- **TOML styling**: `[style]` section with colors, radii, borders, spacing,
  font weight and 14 ready-made themes (`--themes`).
- **Default theme faithful to `original_source/dmenu.rasi`**: light bar
  (`#DEDEDE`/`#333333`), selection `#C5C5C5` with black text and a 1px
  `#FF8C00` border, square corners, 20px height and `Sans 9` font.
- TOML config with validation (`deny_unknown_fields`), dmenu-compatible flags.
- Fast startup: ~25 ms to the first frame (release, 38 apps, fonts already in
  the OS cache).

## Installation

Build dependencies: stable Rust and the usual X11/XCB development libraries
(on Debian/Ubuntu: `libxcb1-dev libxcb-randr0-dev libxcb-xinerama0-dev`).
`cosmic-text` uses fontconfig.

```bash
git clone <this-repo> i3-launcher-rs
cd i3-launcher-rs

# option 1: install into ~/.local/bin (builds first)
./install.sh                              # or: make install

# option 2: cargo install (goes to ~/.cargo/bin, must be on PATH)
cargo install --path . --locked

# option 3: another prefix / packaging
PREFIX=/usr/local ./install.sh
DESTDIR=/tmp/pkg PREFIX=/usr ./install.sh
make install PREFIX=/usr/local

# build only:
cargo build --release                     # binary at target/release/i3-launcher
```

`./install.sh --no-build` installs the already-built binary. The `PREFIX`
(default `~/.local`), `BINDIR` (default `$PREFIX/bin`) and `DESTDIR` (default
empty) variables control the destination.

In `~/.config/i3/config`:

```
bindsym $mod+d exec --no-startup-id i3-launcher
# variants
bindsym $mod+Shift+d exec --no-startup-id i3-launcher -b      # at the bottom
bindsym $mod+x exec --no-startup-id i3-launcher --print       # print the selection
```

> If you use i3bar at the top, the strip covers the bar while it is open (like
> dmenu). To show it just below the bar, use `offset = 24` (or your i3bar
> height) in the `[bar]` section.

## Usage

```
i3-launcher [OPTIONS]

  -p, --prompt <TEXT>        prompt text
  -b, --bottom               anchor to the bottom of the monitor
  -e, --edge <top|bottom>
  -m, --monitor <TARGET>     focused | primary | pointer | INDEX | NAME
      --height <PX>  --margin <PX>  --offset <PX>  --width <PX|%>
      --align <left|center|right>
  -fn, --font <FAMILY>       font family
      --font-size <PX>       font size
      --icon-size <PX>       icon size
      --no-icons             do not draw icons
      --terminal <CMD>       terminal for Terminal=true entries
      --case-sensitive  --match <prefix|substring|fuzzy>
  -d, --dmenu                read items from stdin and print the selection
      --exec                 with --dmenu, run through the shell
      --print                print the selection instead of running it
  -nb/-nf/-sb/-sf <COLOR>    colors (dmenu compatibility)
      --theme <NAME>         override the config color theme
      --themes               list the available themes
  -c, --config <FILE>        TOML config
  -h, --help  -v, --version
```

Config: `$I3L_CONFIG`, `--config`, `$XDG_CONFIG_HOME/i3-launcher/config.toml`
or `~/.config/i3-launcher/config.toml`. See `config.example.toml` for all
keys.

## Style and themes

Everything visual lives in `[style]`. The default (without a config)
reproduces `original_source/dmenu.rasi`, the same scheme as the i3/polybar
bar:

```toml
[appearance]
font = "Sans"                 # "Sans 9" in the rasi (9pt ≈ 12px at 96dpi)
font_size = 12.0
icon_size = 14                # ~1.2em

[style]
theme = ""                    # empty = dmenu.rasi default
padding = 2.0                 # window { padding: 2px; }
gap = 0.0
item_gap = 0.0                # listview { spacing: 0px; }
item_padding = 10.0           # element { padding: 0px 10px; }
search_width = 120            # entry { width: 10em; }
selection_radius = 0.0        # square corners
selection_border_width = 1.0
selection_border = "#ff8c00"  # element selected
```

The default colors are `bg = "#dedede"`, `fg = "#333333"`,
`selection_bg = "#c5c5c5"` and `selection_fg = "#000000"`; the rasi does not
highlight matched characters, so `highlight_fg` is optional and off by
default.

Any color written in `[style]` overrides the theme. For a rounded look, an
input box and a panel border:

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

Included themes: `catppuccin-mocha`, `catppuccin-macchiato`,
`catppuccin-frappe`, `catppuccin-latte`, `gruvbox-dark`, `gruvbox-light`,
`nord`, `dracula`, `tokyo-night`, `solarized-dark`, `solarized-light`,
`one-dark`, `rose-pine` and `adwaita-dark`. The rasi default is not a named
theme (`theme = ""`).

```bash
i3-launcher --themes
i3-launcher --theme catppuccin-latte
```

### Keys

| Key | Action |
| --- | --- |
| `Enter` | run the selected item |
| `Tab` | switch apps (`.desktop`, with icons) ↔ `$PATH` commands (no icons) |
| `Esc` | close (code 1) |
| `↑` `↓` / `Ctrl+P` `Ctrl+N` | move the selection |
| `PgUp` `PgDn` | jump 10 items |
| `←` `→` | move the active item |
| `Ctrl+←` `Ctrl+→` | move the text cursor by word |
| `Ctrl+B` `Ctrl+F` | move the text cursor by character |
| `Alt+B` `Alt+F` | move the text cursor by word |
| `Home` `End` / `Ctrl+A` `Ctrl+E` | start/end of text (repeat: first/last item) |
| `Backspace` `Delete` / `Ctrl+H` `Ctrl+D` | delete character |
| `Ctrl+W` `Ctrl+U` `Ctrl+K` | delete word / to the start / to the end |
| click / scroll | select / navigate |

In `--dmenu` mode `Tab` has no effect (the list comes from stdin). In PATH
mode the prompt stays the same (`Run: `) and the chosen binary is executed
directly, without an intermediate shell.

### dmenu mode

```bash
printf 'one\ntwo\nthree\n' | i3-launcher --dmenu --prompt 'Choose: '
# prints the chosen line to stdout

ls | i3-launcher --dmenu --exec   # runs through /bin/sh
```

### `--print`

Prints the selected application name instead of launching it. Useful for tests
and script integration.

## How it works

```text
.desktop ──scan──▶ items ──filter(query)──▶ score ──layout──▶ tiny-skia ──PutImage──▶ X11
     stdin ────────┘                              ▲                                   │
                                                  └────────── keyboard/mouse ──────────┘
```

- **Window**: `x11rb`, `override-redirect`, `PutImage` into a double-buffered
  pixmap; exclusive keyboard grab (the mouse stays free) and focus returned to
  the window that was active when it closed.
- **Text**: `cosmic-text` (shaping, font fallback) with glyph rasterization
  and manual blending into the pixmap.
- **Icons**: XDG resolution + cache by `(name, size)`; PNG with Lanczos, SVG
  with `resvg` at the target size.
- **Desktop Entry**: own parser (groups, localized keys, escapes, `Exec` with
  field codes) and filters (`Type`, `NoDisplay`, `Hidden`, `TryExec`,
  `OnlyShowIn`/`NotShowIn`).

Architecture details and roadmap: [`PLAN.md`](PLAN.md).

## Development

```bash
cargo test          # parser, scan, icons, matching, editing, geometry
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run --release
```

Debug tools (without opening a window, except the last one):

```bash
cargo run --release --example measure       # widths measured by cosmic-text
cargo run --release --example layout_dump   # cells/viewport for 1366px
cargo run --release --example match_dump -- ala
I3L_DEBUG=1 cargo run --release             # monitor/item/theme log
```

## Known limitations

- **X11 only** (i3/Xorg). Wayland/layer-shell is on the roadmap.
- No IME or dead keys: the keyboard map uses `GetKeyboardMapping` (levels 0/1
  with Shift). For complex keyboards, the path is `xkbcommon`.
- No grid layout: the list is a single row with horizontal scrolling, like
  dmenu.
- Rounded corners/transparency of the whole strip require an ARGB visual
  (depth 32) + compositor; on X11 without ARGB they stay square. Inner
  elements (selection, input box, icon placeholder) have a configurable
  radius.
- SVG icons with `<text>` do not render text (resvg without the `text`
  feature).

## License

MIT.
