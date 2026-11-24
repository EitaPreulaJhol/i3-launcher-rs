# Usage

```
launcher-rs [OPTIONS]
```

## Options

| Flag | Description |
| --- | --- |
| `-c, --config <FILE>` | TOML config file |
| `--theme <NAME>` | Color theme, overriding the one from the config |
| `--themes` | List the available themes and exit |
| `-p, --prompt <TEXT>` | Prompt text (default `Run: `) |
| `-b, --bottom` | Place the strip at the bottom of the monitor |
| `-e, --edge <top\|bottom>` | Monitor edge |
| `-m, --monitor <TARGET>` | `focused`, `primary`, `pointer`, index or connector name |
| `--height <PX>` | Strip height |
| `--margin <PX>` | Outer horizontal margin |
| `--offset <PX>` | Distance from the edge |
| `--width <PX\|%>` | Strip width (pixels or percentage) |
| `--align <left\|center\|right>` | Horizontal alignment |
| `-fn, --font <FAMILY>` | Font family |
| `--font-size <PX>` | Font size in pixels |
| `--icon-size <PX>` | Icon size in pixels |
| `--no-icons` | Do not draw icons |
| `--terminal <CMD>` | Terminal used by entries with `Terminal=true` |
| `--case-sensitive` | Case-sensitive search |
| `-i, --ignore-case` | Case-insensitive search (default) |
| `--match <prefix\|substring\|fuzzy>` | Matching strategy |
| `-d, --dmenu` | Read items from stdin and print the selection |
| `--exec` | With `--dmenu`, run the selection through the shell |
| `--print` | Print the selection instead of running it |
| `-nb, --normal-bg <COLOR>` | Normal background (dmenu compatibility) |
| `-nf, --normal-fg <COLOR>` | Normal text color |
| `-sb, --selected-bg <COLOR>` | Selected background |
| `-sf, --selected-fg <COLOR>` | Selected text color |
| `-h, --help` | Show help and exit |
| `-v, --version` | Show version and exit |

`-l/--lines` (vertical list) and `-w/--embed` are accepted for dmenu
compatibility but are not implemented; the launcher prints a warning and uses a
single horizontal row.

Command-line flags override the configuration file. Search paths for the config
are documented in [Configuration](configuration.md).

## Keybindings

| Key | Action |
| --- | --- |
| `Enter` | run the selected item |
| `Tab` | switch apps (`.desktop`, with icons) ↔ `$PATH` commands (no icons) |
| `Esc` | close (exit code 1) |
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

In `--dmenu` mode `Tab` has no effect, because the list comes from stdin.

## Modes

### `.desktop` applications

The default mode lists applications found in the XDG data directories. Items
show their icons and matched characters can be highlighted. The search covers
`Name`, `GenericName`, `Keywords` and `Categories`; the matching strategy
controls how the query is compared.

### `$PATH` commands

Press `Tab` to switch to executables found in `$PATH`. This list has no icons
and the chosen binary is executed directly, without an intermediate shell. The
prompt stays `Run: `. `$PATH` is scanned on demand the first time you press
`Tab` and cached for the rest of the session.

### dmenu mode

Read lines from stdin and print the selected one to stdout:

```bash
printf 'one\ntwo\nthree\n' | launcher-rs --dmenu --prompt 'Choose: '
```

Run the selection through the configured shell (`/bin/sh` by default):

```bash
ls | launcher-rs --dmenu --exec
```

In dmenu mode the default prompt is empty; `-p` overrides it.

### `--print`

Print the selected application name instead of launching it. Useful for tests
and script integration:

```bash
launcher-rs --print
```

## Monitor selection

`--monitor` and `monitor.target` accept:

| Value | Monitor |
| --- | --- |
| `focused` | Monitor containing the active window (default; falls back to pointer, then primary) |
| `primary` | RandR primary monitor |
| `pointer` / `mouse` | Monitor under the mouse pointer |
| `0`, `1`, … | RandR monitor index |
| `HDMI-A-0`, `eDP-1`, … | RandR connector name |

## Examples

```bash
launcher-rs                                   # default strip on the focused monitor
launcher-rs -b                                # bottom edge
launcher-rs --width 60% --align center        # centered, 60% wide
launcher-rs --theme tokyo-night                # built-in theme
launcher-rs --match prefix --case-sensitive   # search options
launcher-rs --offset 24                       # just below a 24px status bar
```
