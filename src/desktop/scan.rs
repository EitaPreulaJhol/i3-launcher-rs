//! Discovery of `.desktop` entries across the XDG data directories.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::desktop::parser::{self, DesktopEntry, Locale};

/// Directories that may contain applications, in priority order.
pub fn applications_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::data_dir);
    if let Some(home) = data_home {
        dirs.push(home.join("applications"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|v| !v.is_empty())
        .map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    for dir in data_dirs {
        dirs.push(dir.join("applications"));
    }

    // Flatpak exports are commonly missing from XDG_DATA_DIRS in minimal
    // window managers, so add them explicitly (lowest priority).
    if let Some(home) = dirs::home_dir() {
        dirs.push(
            home.join(".local/share/flatpak/exports/share/applications")
                .to_path_buf(),
        );
        dirs.push(home.join(".local/share/applications"));
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    // Deduplicate while preserving priority.
    let mut seen = HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

/// Scan the default application directories.
pub fn scan() -> Vec<DesktopEntry> {
    scan_dirs(&applications_dirs(), &Locale::from_env())
}

/// Scan the given directories, filtering and deduplicating entries.
///
/// Directories earlier in the list win when two files share the same desktop
/// file ID, matching the XDG "first match wins" rule.
pub fn scan_dirs(dirs: &[PathBuf], locale: &Locale) -> Vec<DesktopEntry> {
    let desktops = current_desktops();
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        collect_desktop_files(dir, dir, 0, &mut files);
        files.sort();

        for (path, id) in files {
            if !seen.insert(id.clone()) {
                continue;
            }
            let entry = match parser::parse_file(&path, &id, locale) {
                Ok(entry) => entry,
                Err(err) => {
                    debug(&format!("ignorando entrada: {err}"));
                    continue;
                }
            };
            if !entry.is_visible_application() {
                continue;
            }
            if let Some(try_exec) = &entry.try_exec {
                if !is_executable_in_path(try_exec) {
                    continue;
                }
            }
            if !allowed_in_current_desktop(&entry, desktops.as_deref()) {
                continue;
            }
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    entries
}

fn collect_desktop_files(root: &Path, dir: &Path, depth: usize, out: &mut Vec<(PathBuf, String)>) {
    if depth > 4 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        // Follow symlinks: many packages install `.desktop` files as links
        // into their own data directory (e.g. Discord).
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            collect_desktop_files(root, &path, depth + 1, out);
        } else if metadata.is_file() && name.ends_with(".desktop") {
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let id = relative
                .to_string_lossy()
                .trim_end_matches(".desktop")
                .replace('/', "-");
            out.push((path, id));
        }
    }
}

fn current_desktops() -> Option<Vec<String>> {
    let raw = std::env::var("XDG_CURRENT_DESKTOP").ok()?;
    let list: Vec<String> = raw
        .split(':')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
}

fn allowed_in_current_desktop(entry: &DesktopEntry, desktops: Option<&[String]>) -> bool {
    let Some(desktops) = desktops else {
        // Without XDG_CURRENT_DESKTOP we cannot tell; keep the entry.
        return true;
    };
    let matches = |list: &[String]| {
        list.iter()
            .any(|item| desktops.iter().any(|d| d.eq_ignore_ascii_case(item)))
    };
    if !entry.only_show_in.is_empty() && !matches(&entry.only_show_in) {
        return false;
    }
    if !entry.not_show_in.is_empty() && matches(&entry.not_show_in) {
        return false;
    }
    true
}

/// Check `TryExec`: absolute paths are checked directly, bare names are
/// searched in `$PATH`.
pub fn is_executable_in_path(program: &str) -> bool {
    if program.contains('/') {
        return is_executable(Path::new(program));
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        if is_executable(&dir.join(program)) {
            return true;
        }
    }
    false
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn debug(msg: &str) {
    if std::env::var_os("I3L_DEBUG").is_some() {
        eprintln!("i3-launcher: {msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_recursively_and_dedups() {
        let base = std::env::temp_dir().join(format!("i3l-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let high = base.join("high/applications");
        let low = base.join("low/applications");
        fs::create_dir_all(&high).unwrap();
        fs::create_dir_all(&low).unwrap();
        fs::write(
            high.join("foo.desktop"),
            "[Desktop Entry]\nType=Application\nName=High\nExec=foo\n",
        )
        .unwrap();
        fs::write(
            low.join("foo.desktop"),
            "[Desktop Entry]\nType=Application\nName=Low\nExec=bar\n",
        )
        .unwrap();
        fs::write(
            low.join("hidden.desktop"),
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=x\nNoDisplay=true\n",
        )
        .unwrap();
        fs::create_dir_all(low.join("sub")).unwrap();
        fs::write(
            low.join("sub/nested.desktop"),
            "[Desktop Entry]\nType=Application\nName=Nested\nExec=nested\n",
        )
        .unwrap();

        let entries = scan_dirs(&[high, low], &Locale::default());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["High", "Nested"]);
        assert_eq!(entries[1].id, "sub-nested");

        let _ = fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn follows_symlinked_desktop_files() {
        let base = std::env::temp_dir().join(format!("i3l-symlink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let apps = base.join("applications");
        let data = base.join("data");
        fs::create_dir_all(&apps).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("linked.desktop"),
            "[Desktop Entry]\nType=Application\nName=Linked\nExec=linked\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(data.join("linked.desktop"), apps.join("linked.desktop"))
            .unwrap();
        // Broken symlinks must be skipped, not fail the scan.
        std::os::unix::fs::symlink(
            data.join("missing.desktop"),
            apps.join("missing.desktop"),
        )
        .unwrap();

        let entries = scan_dirs(&[apps], &Locale::default());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Linked"]);

        let _ = fs::remove_dir_all(&base);
    }
}
