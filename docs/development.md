# Development

## Requirements

- Rust stable (1.89+) with `rustfmt` and `clippy`.
- An X11 session for running the launcher; unit tests do not need a display.

## Build and run

```bash
cargo build --release
cargo run --release
```

The release profile enables LTO, a single codegen unit and strips symbols;
`panic = "abort"` is set for the binary.

## Tests and lint

```bash
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all --check
```

The test suite covers the `.desktop` parser, XDG scanning, icon resolution,
matching, text editing, geometry and CLI behaviour (`tests/cli.rs`).
`src/config.rs` also has a test that keeps `config.example.toml` in sync with
the compiled defaults.

CI runs formatting, clippy and `cargo doc` in one job, tests on stable and the
MSRV (1.89.0) in another, and a release build afterwards.

## Debug tools

The examples open no window (except the last command):

```bash
cargo run --release --example measure       # widths measured by cosmic-text
cargo run --release --example layout_dump   # cells/viewport for 1366px
cargo run --release --example match_dump -- ala
LAUNCHER_DEBUG=1 cargo run --release       # monitor/item/theme log
```

`LAUNCHER_DEBUG=1` prints timing information (config load, scan, fonts, first
frame) and the selected monitor/theme.

## Code layout

See [Architecture](architecture.md) for the module map and data flow.
Conventions used in the repository:

- Keep the launcher synchronous; startup latency is a feature.
- Avoid `sh -c` for `.desktop` execution.
- Add or update tests next to the code you change.
- Update the matching section of `docs/configuration.md` and
  `config.example.toml` when adding config keys.
- Do not add documentation files to the repository root; use `docs/`.

## Contributing

Issues and pull requests are welcome. Before opening a PR, run:

```bash
cargo fmt
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Commit messages in the history follow a short `area: summary` style
(`ui:`, `x11:`, `docs:`, `build:`, …).

## Releases

1. Update the version in `Cargo.toml` (and `Cargo.lock`).
2. Commit the bump and create a signed/annotated tag:

   ```bash
   git tag -a v0.2.0 -m "v0.2.0"
   git push origin master --tags
   ```

3. The `Release` workflow builds the x86_64 Linux binary, packages it with
   `README.md`, `LICENSE` and `config.example.toml`, and publishes a GitHub
   release with the checksum.

The CI workflow runs on `master`/`main`, pull requests and manual dispatches.
