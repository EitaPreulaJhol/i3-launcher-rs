# i3-launcher-rs

[![CI](https://github.com/EitaPreulaJhol/i3-launcher-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/EitaPreulaJhol/i3-launcher-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`dmenu`-style launcher for i3: a **horizontal strip** with **application icons**,
incremental search and execution of `.desktop` entries. It talks to X11
directly and renders in software — no GTK, Qt or other GUI toolkit.

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Run: firef▏   [🦊 Firefox]  [🌐 Chromium]  [📝 Gedit]  [⌨ Alacritty]    › │
└────────────────────────────────────────────────────────────────────────────┘
```

## Features

- `override-redirect` strip: no borders, gaps or stolen focus.
- Configurable position, width, margins, height and alignment; multi-monitor
  via RandR 1.5 (Xinerama fallback).
- Icons from the XDG Icon Theme spec (PNG and SVG, theme inheritance).
- `prefix`, `substring` or `fuzzy` search, with dmenu/readline editing keys.
- Correct `.desktop` `Exec` handling (quoting, field codes, `Terminal`,
  `Path`), with `setsid` when available.
- `Tab` switches between `.desktop` apps and `$PATH` commands; `--dmenu` mode
  for pipes.
- TOML styling with 14 ready-made themes, radii and borders.
- Single instance per display; ~25 ms to the first frame.

## Installation

Requires Rust stable (1.89+) and an X11 session. All dependencies are pure Rust,
so no X11/XCB or fontconfig development packages are needed.

```bash
git clone https://github.com/EitaPreulaJhol/i3-launcher-rs.git
cd i3-launcher-rs
./install.sh                 # builds and installs into ~/.local/bin
```

Then bind it in `~/.config/i3/config`:

```
bindsym $mod+d exec --no-startup-id i3-launcher
```

Other install methods, prebuilt binaries and the i3bar note are in
[docs/installation.md](docs/installation.md).

## Documentation

| Document | Contents |
| --- | --- |
| [Installation](docs/installation.md) | Requirements, `install.sh`, `cargo install`, i3 setup |
| [Usage](docs/usage.md) | CLI flags, keybindings, `--dmenu`, `--print`, monitor targets |
| [Configuration](docs/configuration.md) | TOML reference, themes, colors and style options |
| [Architecture](docs/architecture.md) | Modules, data flow, rendering, geometry and design decisions |
| [Development](docs/development.md) | Build, tests, debug tools, contributing and releases |
| [Roadmap](docs/roadmap.md) | Scope, milestones, known limitations and next steps |

The commented example config is [`config.example.toml`](config.example.toml).

## License

MIT.
