# Roadmap

## Scope

| In scope (v1) | Out of scope (future) |
| --- | --- |
| Top/bottom strip, monitor width, margins, offset | Grid layout (multiple rows) |
| Icons via the XDG Icon Theme spec | Wayland / layer-shell |
| Prefix, substring and fuzzy search, score ordering | IME / complex character composition |
| Keyboard and mouse navigation | History/usage frequency (`frecency`) |
| `.desktop` execution with `Exec` and `Terminal` | Plugins, multiple providers |
| `Tab` mode for `$PATH` commands | Preview pane / per-item actions |
| dmenu mode (stdin) and `--dmenu --exec` | Themes loaded from external files |
| TOML config, dmenu-style flags, built-in themes | HiDPI scale factor |
| Single instance per display | |

Target: **X11 + i3**, startup under 50 ms with ~50 applications.

## Milestones

- [x] **M0 — Skeleton**: Cargo, dependencies, config, CLI, docs, module layout.
- [x] **M1 — Data**: `.desktop` parser (i18n/escapes/field codes), XDG scan,
      icon resolution (PNG/SVG, inheritance), matching, unit tests.
- [x] **M2 — Window**: X11 connection, per-monitor geometry (RandR/Xinerama),
      `override-redirect` window, keyboard grab, Esc/Enter.
- [x] **M3 — UI**: strip painting, text editing, navigation, horizontal scroll,
      application launching.
- [x] **M4 — Polish**: mouse, scroll indicators, GTK theme detection, text
      measurement cache, `--dmenu`, `Exec` validation, benchmark.
- [x] **M4.5 — Style**: `[style]` section, 14 ready-made themes, selection and
      input radii/borders, panel border, match highlighting, font weight.
- [x] **M4.6 — Modes**: `Tab` switches between `.desktop` apps and `$PATH`
      executables (no icons, direct execution, on-demand scan).
- [x] **M5 — Delivery**: installation docs, i3 binding, `make install`,
      clean `cargo clippy -D warnings`, integration tests, release build.
- [x] **M5.1 — Instance**: single instance per display with atomic claim and
      reactivation of the running strip.

## Measured startup

i3 4.22, Xorg, 1366x768, 38 applications:

```text
config:           0 ms
scan .desktop:    1-2 ms
X11 window:       0 ms
fonts + theme:    8-9 ms
first frame:     12-13 ms
TOTAL:           23-26 ms until the strip is visible
```

The first `Tab` adds 6-15 ms to scan `$PATH`; later presses use the cache.

## Deviations from the original plan

- `bar.corner_radius` became `style.selection_radius`: rounding the whole
  window needs an ARGB visual plus a compositor, while the selection highlight
  already looks good at depth 24.
- `--print` was added for tests and scripts.
- `examples/` gained `measure`, `layout_dump` and `match_dump`.
- `Tab` mode for `$PATH` commands and single-instance handling were added after
  the initial milestone list.

## Next steps (v2)

1. Grid layout (`layout = "grid"`, `rows = N`) for many applications.
2. `xkbcommon` for IME, dead keys and non-Latin layouts.
3. History/frequency (`frecency`) and pinned applications.
4. Wayland support (`wlr-layer-shell`) while keeping the X11 backend.
5. Whole-strip rounded corners and transparency: requires an ARGB visual
   (depth 32) and a compositor.
6. HiDPI scale factor.
7. Preview/extra actions (open with terminal, copy `Exec`, …).

## Known limitations

- **X11 only** (i3/Xorg).
- No IME or dead keys: the keyboard map uses `GetKeyboardMapping` (levels 0/1
  with Shift). Complex keyboards need `xkbcommon`.
- No grid layout: the list is a single row with horizontal scrolling, like
  dmenu.
- Rounded corners/transparency of the whole strip require an ARGB visual
  (depth 32) and a compositor; without them the outer shape stays square.
  Inner elements (selection, input box, icon placeholder) support a radius.
- SVG icons with `<text>` do not render text (`resvg` is built without the
  `text` feature).
- XPM icons are not supported.
- HiDPI is assumed to be 1.0; `font_size` and `icon_size` are in pixels.
