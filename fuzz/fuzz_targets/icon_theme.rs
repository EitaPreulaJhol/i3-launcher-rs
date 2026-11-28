//! `index.theme` parsing and directory scoring.
#![no_main]

use std::path::Path;

use launcher_rs::icons::theme::ThemeIndex;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|content: &str| {
    let Some(theme) = ThemeIndex::parse_str(content, Path::new("/fuzz/icons/Theme")) else {
        return;
    };
    for dir in &theme.dirs {
        for size in [0, 1, 16, 24, 48, 256, u32::MAX] {
            let _ = theme.score(dir, size, 1);
        }
    }
});
