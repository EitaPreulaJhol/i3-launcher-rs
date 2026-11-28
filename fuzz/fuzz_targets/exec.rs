//! `Exec=` quoting, escaping and field-code expansion.
#![no_main]

use std::path::Path;

use launcher_rs::desktop::parser::{expand_exec, tokenize_exec, unescape, ExecContext};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|exec: &str| {
    let _ = unescape(exec);
    let Ok(tokens) = tokenize_exec(exec) else {
        return;
    };
    let ctx = ExecContext {
        name: "Fuzz App",
        icon: Some("fuzz-icon"),
        desktop_file: Path::new("/fuzz/app.desktop"),
    };
    let _ = expand_exec(&tokens, &ctx);
});
