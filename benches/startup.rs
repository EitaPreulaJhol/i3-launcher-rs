//! Startup work measured on the machine running the benchmark: `.desktop`
//! scanning, `$PATH` listing and icon theme resolution use the real XDG
//! directories, so numbers are only comparable on the same system.
//!
//! `cargo bench --bench startup`

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use launcher_rs::app;
use launcher_rs::icons::IconResolver;

fn bench_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan");
    group.sample_size(20);
    group.bench_function("desktop_entries", |b| b.iter(app::load_entries));
    group.bench_function("path_commands", |b| b.iter(app::path_items));
    group.finish();
}

/// Resolve every icon named by the installed `.desktop` entries, plus a few
/// that do not exist (the expensive case: every directory is probed).
fn bench_icons(c: &mut Criterion) {
    let entries = app::load_entries();
    let mut names: Vec<String> = entries.iter().filter_map(|e| e.icon.clone()).collect();
    names.extend((0..10).map(|index| format!("launcher-rs-missing-icon-{index}")));

    let mut group = c.benchmark_group("icons");
    group.sample_size(20);
    group.bench_function("resolve_all", |b| {
        b.iter(|| {
            let mut resolver = IconResolver::new("");
            for name in &names {
                black_box(resolver.find(name, 24));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_scan, bench_icons);
criterion_main!(benches);
