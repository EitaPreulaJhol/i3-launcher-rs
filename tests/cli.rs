//! Smoke tests for the command line front-end (no display required).

use std::process::Command;

fn launcher() -> Command {
    Command::new(env!("CARGO_BIN_EXE_i3-launcher"))
}

#[test]
fn help_prints_usage() {
    let output = launcher().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("i3-launcher-rs"));
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
fn unknown_flag_fails() {
    let output = launcher().arg("--definitely-not-a-flag").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("opção desconhecida"));
}

#[test]
fn missing_value_fails() {
    let output = launcher().arg("--prompt").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exige um valor"));
}
