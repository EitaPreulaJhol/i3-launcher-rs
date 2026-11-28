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

The script installs the binary and the manual page (`man launcher-rs`; make
sure `$PREFIX/share/man` is in your `MANPATH` for a `~/.local` install).

Use `--no-build` to install an already-built binary:

```bash
./install.sh --no-build
```

The destination is controlled by environment variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `PREFIX` | `$HOME/.local` | Base installation directory |
| `BINDIR` | `$PREFIX/bin` | Directory that receives the binary |
| `MANDIR` | `$PREFIX/share/man` | Directory that receives `man1/launcher-rs.1` |
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
[releases](https://github.com/EitaPreulaJhol/launcher-rs/releases), each archive
with its `.sha256` checksum and the manual page in `docs/launcher-rs.1`:

| Archive | Use it when |
| --- | --- |
| `launcher-rs-VERSION-x86_64-unknown-linux-gnu.tar.gz` | glibc 2.28 or newer: Debian 10+, Ubuntu 18.10+, Fedora 29+, RHEL 8+ and others (recommended) |
| `launcher-rs-VERSION-x86_64-unknown-linux-musl.tar.gz` | Any distribution, including musl-based ones (Alpine, Void musl); fully static |

The static musl build starts a few milliseconds slower (about 12 ms instead of
9 ms to load fonts) because of musl's memory allocator; typing speed is the
same.

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
make uninstall                 # binary and manual page
make uninstall PREFIX=/usr/local
# or remove them manually
rm ~/.local/bin/launcher-rs ~/.local/share/man/man1/launcher-rs.1
```

The optional config lives at `~/.config/launcher-rs/config.toml`.
