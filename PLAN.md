# i3-launcher-rs — Plan

`dmenu`-style launcher: a **horizontal strip** (top of the monitor by default,
configurable position), `override-redirect`, with **application icons**,
incremental search and execution of `.desktop` entries.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Run: firef▏   [🦊 Firefox]  [🌐 Chromium]  [📝 Gedit]  [⌨ Alacritty]      │
└──────────────────────────────────────────────────────────────────────────┘
```

## 1. Goal and scope

| In scope (v1)                                        | Out of scope (roadmap)                  |
| ---------------------------------------------------- | --------------------------------------- |
| Top/bottom strip, monitor width, margins, offset     | Grid layout (multiple rows)             |
| Icons via the XDG Icon Theme spec                    | Wayland / layer-shell                   |
| Substring + fuzzy search, score ordering             | IME / complex character composition     |
| Keyboard + mouse navigation (click/scroll)           | History/usage frequency, `frecency`     |
| `.desktop` execution with `Exec` and `Terminal`      | Plugins, multiple providers             |
| dmenu mode (stdin) for compatibility                 | Themes loaded from external files       |
| TOML config + dmenu-style flags                      | Preview pane / per-item actions         |

Target: **X11 + i3**. No async runtime, no heavy toolkit: own window via
`x11rb`, software drawing, startup < 50 ms with ~50 apps.

## 2. Technical decisions

| Area      | Choice                   | Why |
| --------- | ------------------------ | --- |
| Window    | `x11rb` (pure protocol)  | Exact control over position/size, `override-redirect` (i3 does not manage it, no border/gap), keyboard grab, multi-monitor via RandR 1.5 |
| Drawing   | `tiny-skia` (CPU)        | Antialiased rasterization, `Pixmap` compatible with `resvg`, no GPU/compositor |
| Text      | `cosmic-text`            | Shaping, font fallback, glyph rasterization (`SwashCache`) |
| Icons     | `resvg` + `image` (png)  | SVG rendered at the final size (sharp), PNG resized with Lanczos |
| Config    | `serde` + `toml`         | Typed schema, `deny_unknown_fields` to catch typos |
| Execution | `std::process::Command`  | No `sh -c` for `.desktop` (respects the spec quoting) |

Why not GTK/egui/iced: a window managed by the WM gets a border, gaps and
loses focus when clicking elsewhere; the launcher needs to behave like dmenu
(ephemeral overlay, exclusive keyboard grab, no decorations).

## 3. Architecture

```text
src/
  main.rs            arg parsing → config → X11 → items → event loop
  lib.rs
  cli.rs             dmenu-compatible flags (-b -p -fn -nb -nf -sb -sf -m)
  config.rs          TOML (bar/appearance/style/behavior), defaults, CLI, dimensions
  theme.rs           ready-made color palettes (catppuccin, gruvbox, nord, ...)
  app.rs             Item/Action: unifies .desktop entries and stdin lines
  matching.rs        substring/fuzzy + score + ordered filter
  desktop/
    parser.rs        Desktop Entry Spec parser (groups, i18n, escapes)
    scan.rs          XDG_DATA_DIRS scan + dedup + filters (NoDisplay, OnlyShowIn...)
    launch.rs        Exec → argv (field codes), Terminal, detach
  icons/
    theme.rs         index.theme, inheritance, best-fit lookup by size/context
    mod.rs           IconCache: name → already scaled Pixmap (PNG/SVG)
  x11/
    mod.rs           connection, atoms, window, grab, PutImage
    monitor.rs       RandR 1.5 → Xinerama → screen; focused monitor via _NET_ACTIVE_WINDOW
    keys.rs          keycode/keysyms → char + modifiers
  ui/
    state.rs         query, cursor, selection, scroll, line editing
    layout.rs        prompt, input, viewport and cell geometry
    paint.rs         drawing into the Pixmap (bar, text, icons, cursor, chevrons,
                     radii, borders and match highlighting)
```

Data flow:

```text
.desktop ──scan──▶ Vec<Item> ──filter(query)──▶ Vec<Match> ──layout──▶ Paint
     stdin ────────┘                                 ▲                  │
                                                    └── keyboard/mouse ◀┘
Enter ──▶ Action::Launch(argv) | Action::Shell(cmd) | Action::Print(text)
```

## 4. Positioning and geometry

Everything derives from the chosen monitor (`monitor.target`):

```text
monitor = RandR.monitors[i]                    (fallback: Xinerama, then screen)
w  = config.bar.width  ?? monitor.width - 2*margin
x  = monitor.x + margin + align_offset         (left | center | right)
h  = config.bar.height
y  = monitor.y + offset                        (edge = top)
y  = monitor.y + monitor.height - h - offset    (edge = bottom)
```

Internal layout (one row, like dmenu):

```text
[ padding ][ prompt ][ input: ~33% of the width ][ item cells ......... ]
                                                          └ horizontally scrolling viewport
cell = pad + icon(icon_size) + gap + text + pad     (height = h - 2*padding)
```

- Selection always visible: `offset` is adjusted in `state.rs` after every
  move.
- `<` / `>` indicators when there are items outside the viewport.
- Text vertically centered by the `cosmic-text` baseline; icons aligned to the
  cell center.

## 5. Icons (XDG Icon Theme Spec)

Resolution order for `Icon=<name>`:

1. Absolute path → used directly.
2. Configured theme (`appearance.icon_theme`), with recursive inheritance
   (`Inherits`).
3. `hicolor` (the spec's default fallback).
4. `$XDG_DATA_DIRS/pixmaps`, `/usr/share/pixmaps` and `~/.icons`.
5. Generic icon (`application-x-executable`) or a drawn placeholder.

File selection per theme directory (`index.theme`: `Size`, `Scale`, `Type`,
`Context`): `Context=Applications` first, exact size, then larger, then
smaller; `Type=Scalable` (SVG) rendered on demand at the target size.
Extensions: `svg` → `png` (XPM is left out in v1).

Cache: `HashMap<(name, size), Option<Pixmap>>` — resolution is lazy, only for
visible items, and the result already comes out at the final size.

## 6. Search and ordering

- `case folding` normalization (config `case_sensitive = false`).
- `match_mode`:
  - `substring`: match position, bonus at word start, length penalty.
  - `fuzzy`: greedy subsequence, bonus for consecutive characters and word
    start.
- Score desc, stable tie (original order). `sort = false` keeps the scan/stdin
  order.
- Searchable fields: `Name`, `GenericName`, `Keywords`, `Categories`.
- Highlight of matched characters in `style.highlight_fg` (via cosmic-text's
  `set_rich_text`, using the positions returned by the matcher).

## 7. Keyboard and editing (dmenu compat)

| Key                     | Action |
| ----------------------- | ------ |
| `Esc`                   | exit (code 1) |
| `Enter`                 | run the selected item |
| `Tab`                   | switch apps (`.desktop`) ↔ `$PATH` commands |
| `↑` / `↓`, `Ctrl+p/n`   | move selection |
| `←` / `→`               | move the active item |
| `Ctrl+←` / `Ctrl+→`     | move the text cursor by word |
| `Home` / `End`          | start/end of text; repeat goes to the first/last item |
| `Backspace` / `Delete`  | delete before/after the cursor |
| `Ctrl+W` / `Ctrl+U`     | delete word / line |
| `PgUp` / `PgDn`         | jump 10 items |
| Click / scroll          | select / navigate |

Modifiers read from the X11 event `state`; text via `GetKeyboardMapping`
(levels 0/1 + Shift). No IME — limitation documented in v1.

## 8. Configuration (TOML)

Search: `$I3L_CONFIG`, `--config`, `$XDG_CONFIG_HOME/i3-launcher/config.toml`,
`~/.config/i3-launcher/config.toml`. See `config.example.toml`. Example:

```toml
[bar]
edge = "top"        # top | bottom
height = 20         # thin strip; icons are limited to the height
margin = 0
offset = 0
width = "100%"      # px or "%"
align = "left"      # left | center | right

[monitor]
target = "focused"  # focused | primary | pointer | 0 | "HDMI-A-0"

[appearance]
font = "Sans"       # dmenu.rasi: "Sans 9" (9pt ≈ 12px)
font_size = 12.0
font_weight = 400   # 100..900
prompt = "Run: "
icon_theme = ""     # empty = auto-detect (gtk settings / hicolor)
icon_size = 14      # ~1.2em, like rofi's element-icon
show_icons = true

[style]
# Default = original_source/dmenu.rasi (light i3/polybar bar).
# theme = "" uses the colors below; --themes lists the 14 alternative themes.
theme = ""
bg = "#dedede"
fg = "#333333"
selection_bg = "#c5c5c5"
selection_fg = "#000000"
selection_border = "#ff8c00"
highlight_fg = ""          # optional; the rasi does not highlight matches
padding = 2.0              # window { padding: 2px; }
gap = 0.0
item_gap = 0.0             # listview { spacing: 0px; }
item_padding = 10.0        # element { padding: 0px 10px; }
search_width = 120         # entry { width: 10em; }
selection_radius = 0.0
selection_border_width = 1.0
input_bg = ""              # no box (the entry inherits the background)
input_radius = 0.0
panel_border_width = 0.0

[behavior]
match_mode = "fuzzy"
case_sensitive = false
sort = true
terminal = ""       # e.g. "alacritty -e"; empty = $TERMINAL
print_selection = false
```

CLI flags override the config: `-p/--prompt`, `-b/--bottom`, `-e/--edge`,
`-m/--monitor`, `-fn/--font`, `-nb/-nf/-sb/-sf`, `--icon-size`, `--no-icons`,
`--height`, `--theme/--themes`, `--dmenu`, `--print`, `-c/--config`.

Themes are applied by filling only the color keys missing from the TOML, so a
color written by the user always wins over the palette; `--theme` replaces the
theme from the file and then the same merge happens.

The default style (without a config) is the translation of
`original_source/dmenu.rasi`: `background #DEDEDE`, `text #333333`, selection
`#C5C5C5` with `#000000` text and a 1px `#FF8C00` border, square corners,
`window padding 2px`, `element padding 0px 10px`, `height 20px`, `entry 10em`
and `font Sans 9`.

## 9. Milestones

- [x] **M0 — Skeleton**: Cargo, deps, config, CLI, docs, module structure.
- [x] **M1 — Data**: `.desktop` parser (i18n/escapes/field codes), XDG scan,
      icon resolution (PNG/SVG, inheritance), matching, unit tests.
- [x] **M2 — Window**: X11 connection, per-monitor geometry (RandR/Xinerama),
      `override-redirect` window, keyboard grab, Esc/Enter.
- [x] **M3 — UI**: strip painting (background, prompt, input, cursor, icons),
      text editing, navigation, horizontal scroll, app launching.
- [x] **M4 — Polish**: mouse, scroll indicators, GTK theme detection, text
      measurement cache, `--dmenu`, `Exec` validation, benchmark.
- [x] **M4.5 — Style**: `[style]` section, 14 ready-made themes, selection and
      input radii/borders, panel border, match highlighting, font weight.
- [x] **M4.6 — Modes**: `Tab` switches between `.desktop` apps and `$PATH`
      executables (no icons, direct execution, on-demand scan).
- [x] **M5 — Delivery**: README with installation/i3 bind, `make install`,
      clean `cargo clippy -D warnings`, integration tests, release build.

### Current state (measured on this machine: i3 4.22, Xorg, 1366x768, 38 apps)

```text
config:           0 ms
scan .desktop:    1-2 ms
X11 window:       0 ms (keyboard grab already active)
fonts + theme:    8-9 ms (fontconfig/Adwaita)
first frame:     12-13 ms
TOTAL:           23-26 ms until the strip is visible
(first Tab: +6-15 ms to scan $PATH; cached afterwards)
```

Deviations from the original plan:

- `bar.corner_radius` was replaced by `appearance.sel_radius`: rounding the
  window would require an ARGB visual + compositor, while the selection
  highlight already looks good at depth 24.
- `--print` was added for tests/scripts (prints instead of running).
- Debug tools in `examples/` (`measure`, `layout_dump`, `match_dump`).

### Next steps (v2)

1. Grid layout (`layout = "grid"`, `rows = N`) for many apps.
2. `xkbcommon` for IME, dead keys and non-Latin layouts.
3. History/frequency (`frecency`) and pinned apps.
4. Wayland support (`wlr-layer-shell` layer) while keeping the X11 backend.
5. Rounded corners/panel transparency: requires an ARGB visual (depth 32) and
   a compositor; this server does not offer a 32-bit visual, so only the inner
   elements (selection/input/icons) have a configurable radius today.
6. Preview/extra actions (open with terminal, copy Exec, etc.).

## 10. Risks and mitigations

| Risk | Mitigation |
| ---- | ---------- |
| Keyboard grab does not receive events | Grab on `root` with `owner_events=true` + `SetInputFocus`; re-grab on `FocusIn` (like dmenu) |
| Previous window loses focus on close | Save `GetInputFocus` before opening, `RevertToPointerRoot` and restore after destroying the window (only if the launcher still held focus) |
| Slow icons on the first frame | Cache by `(name, size)`; load only the viewport; pre-warm in a thread if necessary |
| Missing SVG fonts (resvg without `text`) | Icons with `<text>` are rare; enable the `text`/`system-fonts` feature if they show up |
| Theme without a PNG/SVG at the size | Lanczos scaling (PNG) / vector rendering (SVG) + `hicolor`/pixmaps fallback |
| HiDPI | `scale` in the config (v1.1); today assumes 1.0 and uses `icon_size`/`font_size` in px |
| Dead/non-Latin keys | v1 limitation; future path: `xkbcommon` for a full keymap |
| Key stuck in another client | Exclusive grab at startup; `XUngrabKeyboard` on cleanup (including the panic hook) |

## 11. How to test

```bash
cargo test                 # parser, matching, icon resolution
cargo run --release        # opens the strip on the focused monitor
cargo run --release -- --dmenu < /usr/share/applications/list.txt
```

Suggested i3 bind (`~/.config/i3/config`):

```
bindsym $mod+d exec --no-startup-id i3-launcher
```
