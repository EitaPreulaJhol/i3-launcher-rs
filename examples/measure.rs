//! Debug helper: print measured text widths (requires system fonts).
use launcher_rs::config::Config;
use launcher_rs::ui::paint::Painter;

fn main() {
    let cfg = Config::default();
    let mut painter = Painter::new(&cfg);
    println!(
        "theme: {} font: {}",
        painter.icon_theme(),
        cfg.appearance.font
    );
    for text in [
        "Run: ",
        "Firefox",
        "Files",
        "Terminal",
        "Alacritty",
        "Settings",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        println!("{text:?} -> {:.2}px", painter.text_width(text));
    }
}
