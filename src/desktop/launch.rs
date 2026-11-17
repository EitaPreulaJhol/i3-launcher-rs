//! Turning a [`DesktopEntry`] into a running process.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::config::Config;
use crate::desktop::parser::{expand_exec, tokenize_exec, DesktopEntry, ExecContext};
use crate::desktop::scan::is_executable_in_path;

/// A fully resolved command line, ready to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub argv: Vec<String>,
    pub cwd: Option<PathBuf>,
}

/// Resolve the `Exec` line of an entry (field codes, terminal wrapper).
pub fn build(entry: &DesktopEntry, cfg: &Config) -> Result<LaunchCommand> {
    let tokens = tokenize_exec(&entry.exec)
        .with_context(|| format!("Exec inválido em `{}`", entry.source.display()))?;
    let ctx = ExecContext {
        name: &entry.name,
        icon: entry.icon.as_deref(),
        desktop_file: &entry.source,
    };
    let mut argv = expand_exec(&tokens, &ctx);
    if argv.is_empty() {
        bail!("`{}` não produziu nenhum comando", entry.name);
    }

    if entry.terminal {
        let mut terminal = terminal_command(cfg)?;
        terminal.append(&mut argv);
        argv = terminal;
    }

    Ok(LaunchCommand {
        argv,
        cwd: entry.working_dir.clone(),
    })
}

/// Terminal command line used to wrap entries with `Terminal=true`.
///
/// Resolution order: `behavior.terminal` from the config, then `$TERMINAL`,
/// then a probe among common terminals.
pub fn terminal_command(cfg: &Config) -> Result<Vec<String>> {
    if !cfg.behavior.terminal.trim().is_empty() {
        return split_command(&cfg.behavior.terminal);
    }
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.trim().is_empty() {
            return split_command(&term);
        }
    }
    for candidate in [
        "x-terminal-emulator -e",
        "alacritty -e",
        "kitty",
        "foot",
        "wezterm start --",
        "urxvt -e",
        "st -e",
        "xterm -e",
    ] {
        let parts = split_command(candidate)?;
        if is_executable_in_path(&parts[0]) {
            return Ok(parts);
        }
    }
    bail!("nenhum terminal encontrado; defina `behavior.terminal` no config")
}

fn split_command(raw: &str) -> Result<Vec<String>> {
    let parts: Vec<String> = raw.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        bail!("comando vazio");
    }
    Ok(parts)
}

/// Spawn a command detached from the launcher.
///
/// When `setsid` is available the child gets its own session, so closing the
/// launcher (or its terminal) never kills the application.
pub fn spawn(command: &LaunchCommand) -> Result<()> {
    let mut cmd = if let Some(setsid) = find_setsid() {
        let mut cmd = Command::new(setsid);
        cmd.arg(&command.argv[0]);
        cmd
    } else {
        Command::new(&command.argv[0])
    };
    cmd.args(&command.argv[1..]);
    if let Some(dir) = &command.cwd {
        if dir.is_dir() {
            cmd.current_dir(dir);
        }
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("não foi possível executar `{}`", command.argv.join(" ")))?;
    Ok(())
}

/// Run a command line through the configured shell (`--dmenu --exec`).
pub fn spawn_shell(line: &str, shell: &str) -> Result<()> {
    Command::new(shell)
        .arg("-c")
        .arg(line)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("não foi possível executar `{line}`"))?;
    Ok(())
}

fn find_setsid() -> Option<PathBuf> {
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = dir.join("setsid");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::parser::parse_str;
    use std::path::Path;

    fn entry(exec: &str, terminal: bool) -> DesktopEntry {
        let raw = format!(
            "[Desktop Entry]\nType=Application\nName=Test\nExec={exec}\nTerminal={terminal}\nIcon=test\n"
        );
        parse_str(
            &raw,
            Path::new("/tmp/test.desktop"),
            "test",
            &Default::default(),
        )
        .unwrap()
    }

    #[test]
    fn expands_exec_without_files() {
        let cfg = Config::default();
        let cmd = build(&entry("app --open %U --name %c %i", false), &cfg).unwrap();
        assert_eq!(
            cmd.argv,
            vec!["app", "--open", "--name", "Test", "--icon", "test"]
        );
    }

    #[test]
    fn wraps_terminal_entries() {
        let mut cfg = Config::default();
        cfg.behavior.terminal = "myterm -e".into();
        let cmd = build(&entry("htop", true), &cfg).unwrap();
        assert_eq!(cmd.argv, vec!["myterm", "-e", "htop"]);
    }
}
