//! Debug helper: dump layout geometry for the default config.
use i3_launcher::app;
use i3_launcher::config::Config;
use i3_launcher::ui::paint::Painter;
use i3_launcher::ui::state::App;

fn main() {
    let cfg = Config::default();
    let entries = app::load_entries();
    println!("entries: {}", entries.len());
    let items = app::from_entries(&entries);
    let mut painter = Painter::new(&cfg);
    let mut app_state = App::new(items, &cfg.behavior);
    app_state.relayout(&mut painter, &cfg, 1366.0, 44.0);
    let layout = &app_state.layout;
    println!(
        "cells={} list_x={:.1} list_w={:.1} input_x={:.1} input_w={:.1} prompt_w={:.1}",
        layout.cells.len(),
        layout.list_x,
        layout.list_width,
        layout.input_x,
        layout.input_width,
        layout.prompt_width
    );
    for cell in &layout.cells {
        let label = &app_state.items[app_state.filtered[cell.index].index].label;
        println!(
            "#{} x={:.1} w={:.1} label={:?} text_w={:.1}",
            cell.index,
            cell.x,
            cell.width,
            label,
            painter.text_width(label)
        );
    }
}
