# Installation

## Requirements

- Linux with X11 (i3/Xorg). Wayland is not supported yet.
- Rust stable, version **1.89** or newer.
- A working font stack through fontconfig; `cosmic-text` loads system fonts.

All Rust dependencies are pure Rust, so there is no need to install X11/XCB or
fontconfig development packages.

## From source

```bash
git clone https://github.com/EitaPreulaJhol/i3-launcher-rs.git
cd i3-launcher-rs
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
# binary at target/release/i3-launcher
```

### Prebuilt binaries

Prebuilt x86_64 Linux binaries are attached to
[releases](https://github.com/EitaPreulaJhol/i3-launcher-rs/releases).

## i3 setup

Add a binding to `~/.config/i3/config`:

```
bindsym $mod+d exec --no-startup-id i3-launcher
```

Useful variants:

```
bindsym $mod+Shift+d exec --no-startup-id i3-launcher -b      # bottom edge
bindsym $mod+x exec --no-startup-id i3-launcher --print       # print selection
```

> If you use i3bar at the top, the strip covers the bar while it is open (like
> dmenu). To show it just below the bar, set `offset = 24` (or your i3bar
> height) in the `[bar]` section.

Configuration is optional; without a config file the built-in defaults are
used. See [Configuration](configuration.md).

## Uninstall

```bash
make uninstall
# or remove the binary manually
rm ~/.local/bin/i3-launcher
```

The optional config lives at `~/.config/i3-launcher/config.toml`.
