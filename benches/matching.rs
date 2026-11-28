//! Search benchmarks over a synthetic, deterministic item list.
//!
//! `cargo bench --bench matching`

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use launcher_rs::app::{Action, Item};
use launcher_rs::config::MatchMode;
use launcher_rs::matching::{filter, IncrementalFilter, MatchOptions};

/// About as many entries as a large `$PATH` (the bigger of the two modes).
fn items(count: usize) -> Vec<Item> {
    const WORDS: &[&str] = &[
        "fire", "fox", "term", "edit", "office", "calc", "image", "view", "media", "player",
        "manager", "settings", "gnome", "kde", "x", "qt", "tool", "config", "net", "work",
    ];
    (0..count)
        .map(|index| {
            let a = WORDS[index % WORDS.len()];
            let b = WORDS[(index / WORDS.len()) % WORDS.len()];
            let label = format!("{a}-{b}{index}");
            Item::new(label.clone(), None, Action::Command(label))
        })
        .collect()
}

fn options(mode: MatchMode) -> MatchOptions {
    MatchOptions {
        mode,
        case_sensitive: false,
        sort: true,
        max_results: 0,
    }
}

fn bench_filter(c: &mut Criterion) {
    let items = items(4000);
    let mut group = c.benchmark_group("filter");
    for (name, mode) in [
        ("prefix", MatchMode::Prefix),
        ("substring", MatchMode::Substring),
        ("fuzzy", MatchMode::Fuzzy),
    ] {
        let opts = options(mode);
        group.bench_function(name, |b| {
            b.iter(|| filter(black_box(&items), black_box("fo"), &opts));
        });
    }
    group.finish();
}

/// One keystroke at a time, as the UI does it.
fn bench_typing(c: &mut Criterion) {
    let items = items(4000);
    let opts = options(MatchMode::Fuzzy);
    let keystrokes = ["f", "fi", "fir", "fire", "fire-", "fire-t"];
    let mut group = c.benchmark_group("typing");
    group.bench_function("full", |b| {
        b.iter(|| {
            for query in keystrokes {
                black_box(filter(&items, query, &opts));
            }
        });
    });
    group.bench_function("incremental", |b| {
        b.iter(|| {
            let mut incremental = IncrementalFilter::new();
            for query in keystrokes {
                black_box(incremental.filter(&items, query, &opts));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_filter, bench_typing);
criterion_main!(benches);
