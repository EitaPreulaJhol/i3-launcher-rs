//! Smoke tests for the command line front-end (no display required).

use std::process::Command;

fn launcher() -> Command {
    Command::new(env!("CARGO_BIN_EXE_launcher-rs"))
}

#[test]
fn help_prints_usage() {
    let output = launcher().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("launcher-rs"));
    assert!(stdout.contains("--dmenu"));
    assert!(stdout.contains("--monitor"));
}

#[test]
fn version_prints_version() {
    let output = launcher().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn lists_available_themes() {
    let output = launcher().arg("--themes").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("catppuccin-mocha"));
    assert!(stdout.contains("gruvbox-dark"));
    assert!(stdout.lines().count() >= 14);
}

#[test]
fn unknown_flag_fails() {
    let output = launcher().arg("--definitely-not-a-flag").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown option"));
}

#[test]
fn missing_value_fails() {
    let output = launcher().arg("--prompt").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires a value"));
}

/// Every long flag in `--help` must be documented in the man page, and the
/// man page must carry the current version.
#[test]
fn man_page_matches_help() {
    let man_page =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/launcher-rs.1"))
            .unwrap();
    // roff escapes `-` as `\-`.
    let man_page = man_page.replace("\\-", "-");
    assert!(
        man_page.contains(&format!("\"launcher-rs {}\"", env!("CARGO_PKG_VERSION"))),
        "update the version in the .TH line of docs/launcher-rs.1"
    );

    let output = launcher().arg("--help").output().unwrap();
    let help = String::from_utf8_lossy(&output.stdout);
    let flags: Vec<&str> = help
        .split(|c: char| c.is_whitespace() || c == ',' || c == '=')
        .filter(|word| word.starts_with("--") && word.len() > 2)
        .collect();
    assert!(flags.len() > 20, "could not find the flags in --help");
    for flag in flags {
        assert!(
            man_page.contains(flag),
            "docs/launcher-rs.1 is missing {flag}"
        );
    }
}
