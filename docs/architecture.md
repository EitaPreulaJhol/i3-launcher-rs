# Architecture

## Overview

`i3-launcher-rs` is a small X11 client: it scans `.desktop` files, filters and
sorts them as you type, paints a single horizontal row into a pixmap and copies
that pixmap to an `override-redirect` window. There is no async runtime, no
toolkit and no compositor dependency.

```text
.desktop ──scan──▶ items ──filter(query)──▶ score ──layout──▶ tiny-skia ──PutImage──▶ X11
     stdin ────────┘                              ▲                                   │
                                                  └────────── keyboard/mouse ──────────┘
```

The process exits after launching an item, printing a selection or being
closed with `Esc`. A second launch while the strip is open raises and refocuses
the existing instance instead of creating another window.

## Technical decisions

| Area | Choice | Why |
| --- | --- | --- |
| Window | `x11rb` (pure protocol) | Exact control over position/size, `override-redirect` (i3 does not manage it), keyboard grab, multi-monitor via RandR 1.5 |
| Drawing | `tiny-skia` (CPU) | Antialiased rasterization; `Pixmap` compatible with `resvg`; no GPU or compositor |
| Text | `cosmic-text` | Shaping, font fallback and glyph rasterization (`SwashCache`) |
| Icons | `resvg` + `image` | SVG rendered at the final size; PNG resized with Lanczos |
| Config | `serde` + `toml` | Typed schema; `deny_unknown_fields` catches typos |
| Execution | `std::process::Command` | No `sh -c` for `.desktop` entries, so the spec quoting is respected |

A window managed by the WM would get borders, gaps and lose focus when clicked
elsewhere. The launcher needs dmenu-like behaviour: an ephemeral overlay with
an exclusive keyboard grab and no decorations.

## Modules

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
    mod.rs           connection, atoms, window, grab, PutImage, single instance
    monitor.rs       RandR 1.5 → Xinerama → screen; focused monitor via _NET_ACTIVE_WINDOW
    keys.rs          keycode/keysyms → char + modifiers
  ui/
    state.rs         query, cursor, selection, scroll, line editing
    layout.rs        prompt, input, viewport and cell geometry
    paint.rs         drawing into the Pixmap (bar, text, icons, cursor, chevrons,
                     radii, borders and match highlighting)
```

Tests live next to the modules and in `tests/cli.rs`; small debug binaries live
in `examples/`.

## Data flow

```text
.desktop ──scan──▶ Vec<Item> ──filter(query)──▶ Vec<Match> ──layout──▶ Paint
     stdin ────────┘                                 ▲                  │
                                                    └── keyboard/mouse ◀┘
Enter ──▶ Action::Launch(argv) | Action::Shell(cmd) | Action::Print(text)
```

`Item` unifies `.desktop` entries and stdin lines. `Tab` swaps the active list
for the `$PATH` list (scanned lazily on first use) and back.

## Positioning and geometry

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
[ padding ][ prompt ][ input: configured width ][ item cells ......... ]
                                                          └ horizontally scrolling viewport
cell = pad + icon(icon_size) + gap + text + pad     (height = h - 2*padding)
```

- The selection is kept visible: the viewport offset is adjusted after every
  move.
- `<` / `>` indicators appear when items are outside the viewport.
- Text is vertically centered using the `cosmic-text` baseline; icons align to
  the cell center.

## Icons (XDG Icon Theme Spec)

Resolution order for `Icon=<name>`:

1. Absolute path → used directly.
2. Configured theme (`appearance.icon_theme`), with recursive inheritance
   (`Inherits`).
3. `hicolor` (the spec's default fallback).
4. `$XDG_DATA_DIRS/pixmaps`, `/usr/share/pixmaps` and `~/.icons`.
5. Generic icon (`application-x-executable`) or a drawn placeholder.

File selection per theme directory (`index.theme`: `Size`, `Scale`, `Type`,
`Context`): `Context=Applications` first, exact size, then larger, then
smaller; `Type=Scalable` (SVG) is rendered on demand at the target size.
Extensions: `svg` → `png` (XPM is not supported).

The cache is a `HashMap<(name, size), Option<Pixmap>>`. Resolution is lazy,
only for visible items, and the result already comes out at the final size.

## Search and ordering

- Case folding is applied unless `behavior.case_sensitive` is true.
- `behavior.match_mode`:
  - `prefix`: the query must start the candidate.
  - `substring`: match position, bonus at word start, length penalty.
  - `fuzzy`: greedy subsequence, bonus for consecutive characters and word
    starts.
- Score descending, stable tie (original order). `sort = false` keeps the
  scan/stdin order.
- Searchable fields: `Name`, `GenericName`, `Keywords`, `Categories`.
- Matched characters can be highlighted with `style.highlight_fg` (rendered via
  cosmic-text's rich text with the positions returned by the matcher).

## Window, focus and single instance

- The window is `override-redirect`, so the WM ignores it: no borders or gaps.
- The keyboard is grabbed exclusively; mouse events stay available.
- Before opening, the current input focus is saved. On close, focus is returned
  to that window only if the launcher still held it.
- A `_I3_LAUNCHER_RUNNING` selection is claimed atomically. If another instance
  already owns it, the new process raises and focuses the running strip and
  exits, so pressing the i3 binding repeatedly never stacks windows.

## Rendering path

1. Compute geometry and layout.
2. Paint into a `tiny_skia::Pixmap` (background, panel border, prompt, input,
   cursor, item cells, icons, selection, scroll indicators).
3. Convert the pixmap to the X11 image format and `PutImage` it into a
   double-buffered pixmap, then copy to the window.

Icons are loaded only for visible cells. Text measurement results are cached.

## Startup cost

Measured with i3 4.22/Xorg at 1366x768 and 38 applications:

```text
config:           0 ms
scan .desktop:    1-2 ms
X11 window:       0 ms (keyboard grab already active)
fonts + theme:    8-9 ms (fontconfig/Adwaita)
first frame:     12-13 ms
TOTAL:           23-26 ms until the strip is visible
(first Tab:      +6-15 ms to scan $PATH; cached afterwards)
```

## Design risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Keyboard grab does not receive events | Grab on `root` with `owner_events=true`, `SetInputFocus`, re-grab on `FocusIn` (like dmenu) |
| Previous window loses focus on close | Save `GetInputFocus`, use `RevertToPointerRoot`, restore after destroying the window |
| Slow icons on the first frame | Cache by `(name, size)`, load only the viewport |
| Missing SVG fonts (`resvg` without `text`) | Icons with `<text>` are rare; enable the `text`/`system-fonts` features if needed |
| Theme has no PNG/SVG at the requested size | Lanczos scaling (PNG), vector rendering (SVG), `hicolor`/pixmaps fallback |
| HiDPI | Scale factor is not implemented yet; sizes are in pixels |
| Dead keys / non-Latin layouts | Current limitation; path is `xkbcommon` |
| Key stuck in another client | Exclusive grab plus `XUngrabKeyboard` on cleanup, including the panic hook |

See [Roadmap](roadmap.md) for limitations and future work.
