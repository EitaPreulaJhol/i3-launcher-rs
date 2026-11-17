//! Items shown by the launcher and the action attached to each one.

use crate::desktop::{parser::DesktopEntry, scan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Launch the `.desktop` entry at this index.
    LaunchApp(usize),
    /// Run a command line through the shell.
    Shell(String),
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
    fn stdin_items_can_execute() {
        let items = from_lines(vec!["a".into(), "".into(), "b".into()], true);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].action, Action::Shell("b".into()));
        let items = from_lines(["x".to_string()], false);
        assert_eq!(items[0].action, Action::Print("x".into()));
    }
}
