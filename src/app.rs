//! Items shown by the launcher and the action attached to each one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::desktop::{parser::DesktopEntry, scan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Launch the `.desktop` entry at this index.
    LaunchApp(usize),
    /// Run a command line through the shell.
    Shell(String),
    /// Execute a program found in `$PATH` directly (no shell).
    Command(String),
    /// Write the label to stdout.
    Print(String),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    pub icon: Option<String>,
    /// Lowercased blob used for case-insensitive search.
    pub search: String,
    pub action: Action,
}

impl Item {
    pub fn new(label: String, icon: Option<String>, action: Action) -> Self {
        let search = label.to_lowercase();
        Self {
            label,
            icon,
            search,
            action,
        }
    }

    fn from_entry(index: usize, entry: &DesktopEntry) -> Self {
        Self {
            label: entry.name.clone(),
            icon: entry.icon.clone(),
            search: entry.search_text(),
            action: Action::LaunchApp(index),
        }
    }
}

/// Build items from the discovered `.desktop` entries.
///
/// The entries themselves stay in the returned vector so `LaunchApp(index)`
/// can resolve the command lazily, only for the selected item.
pub fn from_entries(entries: &[DesktopEntry]) -> Vec<Item> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| Item::from_entry(index, entry))
        .collect()
}

/// Load entries from the default XDG directories.
pub fn load_entries() -> Vec<DesktopEntry> {
    scan::scan()
}

/// Build items for every executable found in `$PATH`, sorted by name.
pub fn path_items() -> Vec<Item> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    path_items_in(&std::env::split_paths(&path).collect::<Vec<_>>())
}

/// Same as [`path_items`], but over an explicit list of directories.
pub fn path_items_in(dirs: &[PathBuf]) -> Vec<Item> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            if !is_executable_file(&path) {
                continue;
            }
            // PATH precedence: the first directory with an executable wins.
            if !seen.insert(name.clone()) {
                continue;
            }
            items.push(Item::new(name.clone(), None, Action::Command(name)));
        }
    }

    items.sort_by(|a, b| {
        a.label
            .to_lowercase()
            .cmp(&b.label.to_lowercase())
            .then_with(|| a.label.cmp(&b.label))
    });
    items
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Build items from stdin lines (dmenu mode).
pub fn from_lines(lines: impl IntoIterator<Item = String>, execute: bool) -> Vec<Item> {
    lines
        .into_iter()
        .map(|line| line.trim_end_matches(['\n', '\r']).to_string())
        .filter(|line| !line.is_empty())
        .map(|line| {
            let action = if execute {
                Action::Shell(line.clone())
            } else {
                Action::Print(line.clone())
            };
            Item::new(line, None, action)
        })
        .collect()
}

/// Read all of stdin into items.
pub fn read_stdin(execute: bool) -> std::io::Result<Vec<Item>> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let lines = stdin.lock().lines().collect::<Result<Vec<_>, _>>()?;
    Ok(from_lines(lines, execute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::parser::parse_str;
    use std::path::Path;

    #[test]
    fn items_keep_entry_index_and_search_text() {
        let raw = "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox\nIcon=firefox\nKeywords=web;browser;\n";
        let entry = parse_str(raw, Path::new("/tmp/f.desktop"), "f", &Default::default()).unwrap();
        let items = from_entries(&[entry]);
        assert_eq!(items[0].label, "Firefox");
        assert_eq!(items[0].icon.as_deref(), Some("firefox"));
        assert!(items[0].search.contains("browser"));
        assert_eq!(items[0].action, Action::LaunchApp(0));
    }

    #[test]
    fn path_items_are_unique_executables() {
        let base = std::env::temp_dir().join(format!("i3l-path-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let first = base.join("a");
        let second = base.join("b");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        for (dir, name) in [
            (&first, "alpha"),
            (&first, "beta"),
            (&second, "alpha"), // shadowed by the first PATH entry
            (&second, "gamma"),
            (&second, ".hidden"),
            (&second, "not-exec"),
            (&first, "delta-shadow"), // non-executable here, executable in `second`
            (&second, "delta-shadow"),
        ] {
            std::fs::write(dir.join(name), b"#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = if name == "not-exec" || (name == "delta-shadow" && dir == &first) {
                    0o644
                } else {
                    0o755
                };
                std::fs::set_permissions(dir.join(name), std::fs::Permissions::from_mode(mode))
                    .unwrap();
            }
        }
        // Directories must be ignored even if they are executable.
        std::fs::create_dir_all(second.join("a-directory")).unwrap();

        let labels: Vec<String> = path_items_in(&[first.clone(), second.clone()])
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert_eq!(labels, vec!["alpha", "beta", "delta-shadow", "gamma"]);
        assert!(matches!(
            path_items_in(&[first]).first().map(|item| &item.action),
            Some(Action::Command(name)) if name == "alpha"
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn stdin_items_can_execute() {
        let items = from_lines(vec!["a".into(), "".into(), "b".into()], true);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].action, Action::Shell("b".into()));
        let items = from_lines(["x".to_string()], false);
        assert_eq!(items[0].action, Action::Print("x".into()));
    }
}
