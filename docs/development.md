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
matching (including the incremental filter against a full filter), text
editing, geometry, keyboard mapping, CLI parsing (`src/cli.rs`) and CLI
behaviour (`tests/cli.rs`). `src/config.rs` also has a test that keeps
`config.example.toml` in sync with the compiled defaults, and `tests/cli.rs`
checks that every long flag in `--help` is documented in the manual page
`docs/launcher-rs.1` and that its version matches `Cargo.toml`.

Coverage (the CI publishes a summary and an lcov report):

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --open
```

## Lints

Extra rustc/clippy lints are configured in `[lints]` in `Cargo.toml`:
`unsafe_code` is forbidden, `unwrap`/`expect` are not allowed outside tests
(see `clippy.toml`), and a few lints catch needless clones and allocations.
Every `#[allow]` needs a `reason = "..."`.

## Benchmarks

```bash
cargo bench                    # or: make bench
cargo bench --bench matching   # search over 4000 synthetic items
cargo bench --bench frame      # filter + layout + paint per keystroke
cargo bench --bench startup    # .desktop scan, $PATH, icon resolution
```

`frame` and `startup` use the fonts, icon themes and applications installed on
the machine, so compare their numbers only on the same system. Criterion keeps
the previous run and reports the change; HTML reports are written to
`target/criterion/report`.

## Fuzzing

`fuzz/` holds [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets for
the `.desktop` parser, `Exec` tokenization/expansion, `index.theme`, the TOML
config and color parsers, and the matcher (which also checks that incremental
filtering equals a full filter). It needs a nightly toolchain:

```bash
cargo install cargo-fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run desktop_entry -- -max_total_time=60
```

The `Fuzz` workflow runs every target weekly and on demand, and uploads any
crashing input as an artifact.

CI runs formatting, clippy and `cargo doc` in one job, tests on stable and the
MSRV (1.89.0) in another, coverage in a third, and release builds for glibc and
static musl afterwards.

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

- Keep the event loop synchronous; startup latency is a feature. The only
  helper thread loads fonts and the icon theme during startup.
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

1. Update the version in `Cargo.toml` (and `Cargo.lock`) and in the `.TH`
   line of `docs/launcher-rs.1` (a test fails otherwise).
2. Commit the bump and create a signed/annotated tag:

   ```bash
   git tag -a v0.2.0 -m "v0.2.0"
   git push origin master --tags
   ```

3. The `Release` workflow builds x86_64 Linux binaries for glibc and static
   musl, packages each with `README.md`, `LICENSE`, `config.example.toml` and
   `docs/` (including the manual page), and publishes a GitHub release with the
   checksums.

The CI workflow runs on `master`/`main`, pull requests and manual dispatches.
