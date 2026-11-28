//! Per-keystroke cost: filter, relayout, paint and X11 conversion, rendered
//! headlessly with the system fonts and the installed `.desktop` entries.
//!
//! `cargo bench --bench frame`

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use launcher_rs::app;
use launcher_rs::config::Config;
use launcher_rs::ui::paint::{pixmap_to_x11_into, Painter};
use launcher_rs::ui::state::App;
use launcher_rs::x11::keys::Mods;
use tiny_skia::Pixmap;
use xkeysym::Keysym;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 30;

fn bench_frames(c: &mut Criterion) {
    let Ok(cfg) = Config::from_str("", Some("catppuccin-mocha")) else {
        panic!("the built-in theme must load");
    };
    let entries = app::load_entries();
    let mut painter = Painter::new(&cfg);
    let Some(mut surface) = Pixmap::new(WIDTH, HEIGHT) else {
        panic!("could not allocate the framebuffer");
    };
    let mut frame = Vec::new();

    let mut render = |state: &mut App, painter: &mut Painter| {
        state.relayout(painter, &cfg, WIDTH as f32, HEIGHT as f32);
        painter.paint(&mut surface, state, &cfg);
        pixmap_to_x11_into(&surface, &mut frame);
        black_box(&frame);
    };

    let mut group = c.benchmark_group("frame");
    group.bench_function("type_and_erase", |b| {
        b.iter(|| {
            let mut state = App::new(app::from_entries(&entries), &cfg.behavior);
            render(&mut state, &mut painter);
            for c in "term".chars() {
                state.handle_key(Keysym::from_char(c), Mods::default(), &cfg.behavior);
                render(&mut state, &mut painter);
            }
            for _ in 0..4 {
                state.handle_key(Keysym::BackSpace, Mods::default(), &cfg.behavior);
                render(&mut state, &mut painter);
            }
        });
    });
    group.bench_function("move_selection", |b| {
        let mut state = App::new(app::from_entries(&entries), &cfg.behavior);
        b.iter(|| {
            for key in [Keysym::Right, Keysym::Right, Keysym::Left, Keysym::Left] {
                state.handle_key(key, Mods::default(), &cfg.behavior);
                render(&mut state, &mut painter);
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_frames);
criterion_main!(benches);
