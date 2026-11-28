//! Whole `.desktop` files: parsing, search text and command construction.
#![no_main]

use std::path::Path;

use launcher_rs::desktop::parser::{expand_exec, parse_str, tokenize_exec, ExecContext, Locale};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|content: &str| {
    let locale = Locale::from_vars(Some("pt_BR"), None, None, Some("pt_BR.UTF-8"));
    let Ok(entry) = parse_str(content, Path::new("/fuzz/app.desktop"), "app", &locale) else {
        return;
    };
    let _ = entry.is_visible_application();
    let _ = entry.search_text();
    if let Ok(tokens) = tokenize_exec(&entry.exec) {
        let ctx = ExecContext {
            name: &entry.name,
            icon: entry.icon.as_deref(),
            desktop_file: &entry.source,
        };
        let _ = expand_exec(&tokens, &ctx);
    }
});
