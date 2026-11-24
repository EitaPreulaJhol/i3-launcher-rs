# Installation

## Requirements

- Linux with X11 (Xorg). Any window manager works; Wayland is not supported
  yet.
- Rust stable, version **1.89** or newer.
- A working font stack through fontconfig; `cosmic-text` loads system fonts.

All Rust dependencies are pure Rust, so there is no need to install X11/XCB or
fontconfig development packages.

## From source

```bash
git clone https://github.com/EitaPreulaJhol/launcher-rs.git
cd launcher-rs
```

Pick one of the options below.

### Option 1: `install.sh` (recommended)

Builds the release binary and installs it into `~/.local/bin`:

```bash
./install.sh            # or: make install
```

Use `--no-build` to install an already-built binary:

```bash
./install.sh --no-build
```

The destination is controlled by environment variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `PREFIX` | `$HOME/.local` | Base installation directory |
| `BINDIR` | `$PREFIX/bin` | Directory that receives the binary |
| `DESTDIR` | empty | Packaging prefix, prepended to the paths |
| `CARGO` | `cargo` | Cargo command |
| `CARGO_TARGET_DIR` | `./target` | Build directory |

Examples:

```bash
PREFIX=/usr/local ./install.sh
DESTDIR=/tmp/pkg PREFIX=/usr ./install.sh
make install PREFIX=/usr/local
make uninstall
```

### Option 2: `cargo install`

```bash
cargo install --path . --locked
```

This installs into `~/.cargo/bin`, which must be on your `PATH`.

### Option 3: build only

```bash
cargo build --release
# binary at target/release/launcher-rs
```

### Prebuilt binaries

Prebuilt x86_64 Linux binaries are attached to
[releases](https://github.com/EitaPreulaJhol/launcher-rs/releases).

## Window manager setup

Bind `launcher-rs` to a key in your window manager's config. Examples:

**bspwm** (`~/.config/sxhkd/sxhkdrc`):

```
super + d
    launcher-rs
```

**awesome** (`~/.config/awesome/rc.lua`):

```lua
awful.key({ modkey }, "d", function() awful.spawn("launcher-rs") end)
```

**openbox** (`~/.config/openbox/rc.xml`):

```xml
<keybind key="W-d">
  <action name="Execute"><command>launcher-rs</command></action>
</keybind>
```

**dwm** (`config.h`):

```c
{ MODKEY, XK_d, spawn, SHCMD("launcher-rs") },
```

Useful variants of the command:

```
launcher-rs -b              # bottom edge
launcher-rs --print         # print the selection instead of launching it
```

> If you run a status bar at the top (polybar, xfce4-panel, …), the strip
> covers the bar while it is open (like dmenu). To show it just below the bar,
> set `offset = 24` (or your bar height) in the `[bar]` section.

Configuration is optional; without a config file the built-in defaults are
used. See [Configuration](configuration.md).

## Uninstall

```bash
make uninstall
# or remove the binary manually
rm ~/.local/bin/launcher-rs
```

The optional config lives at `~/.config/launcher-rs/config.toml`.
