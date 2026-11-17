//! Debug helper: print measured text widths (requires system fonts).
use i3_launcher::config::Config;
use i3_launcher::ui::paint::Painter;

fn main() {
    let cfg = Config::default();
    let mut painter = Painter::new(&cfg);
    println!(
        "tema: {} fonte: {}",
        painter.icon_theme(),
        cfg.appearance.font
    );
    for text in [
        "Run: ",
        "Firefox",
        "Files",
        "Terminal",
        "Alacritty",
        "Configurações",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        println!("{text:?} -> {:.2}px", painter.text_width(text));
    }
}
