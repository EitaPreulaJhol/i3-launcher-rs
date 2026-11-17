//! Freedesktop icon theme lookup (`index.theme`, inheritance, best-size fit).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Directory type from `index.theme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirType {
    Fixed,
    Scalable,
    Threshold,
}

#[derive(Debug, Clone)]
pub struct ThemeDir {
    /// Absolute directory that contains the icon files.
    pub path: PathBuf,
    pub size: u32,
    pub scale: u32,
    pub min_size: u32,
    pub max_size: u32,
    pub threshold: u32,
    pub kind: DirType,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct ThemeIndex {
    pub name: String,
    pub inherits: Vec<String>,
    pub dirs: Vec<ThemeDir>,
}

impl ThemeIndex {
    /// Parse an `index.theme` file located at `path`.
    pub fn parse(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let root = path.parent()?;
        Self::parse_str(&content, root)
    }

    pub fn parse_str(content: &str, root: &Path) -> Option<Self> {
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current = String::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current = line[1..line.len() - 1].to_string();
                sections.entry(current.clone()).or_default();
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                sections
                    .entry(current.clone())
                    .or_default()
                    .insert(k.trim().to_string(), v.trim().to_string());
            }
        }

        let meta = sections.get("Icon Theme")?;
        let name = meta.get("Name").cloned().unwrap_or_default();
        let inherits = meta
            .get("Inherits")
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let mut listed: Vec<String> = meta
            .get("Directories")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        if let Some(scaled) = meta.get("ScaledDirectories") {
            listed.extend(scaled.split(',').map(|s| s.trim().to_string()));
        }

        let mut dirs = Vec::new();
        for dir_name in listed {
            if dir_name.is_empty() {
                continue;
            }
            let Some(props) = sections.get(&dir_name) else {
                continue;
            };
            let kind = match props.get("Type").map(|s| s.as_str()) {
                Some("Scalable") => DirType::Scalable,
                Some("Threshold") => DirType::Threshold,
                _ => DirType::Fixed,
            };
            let size = props
                .get("Size")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0)
                .max(1);
            dirs.push(ThemeDir {
                path: root.join(&dir_name),
                size,
                scale: props
                    .get("Scale")
                    .and_then(|v| v.parse().ok())
                    .filter(|s| *s > 0)
                    .unwrap_or(1),
                min_size: props
                    .get("MinSize")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(size),
                max_size: props
                    .get("MaxSize")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(size),
                threshold: props
                    .get("Threshold")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2),
                kind,
                context: props.get("Context").cloned().unwrap_or_default(),
            });
        }

        Some(Self {
            name,
            inherits,
            dirs,
        })
    }

    /// Sort key for a directory given the requested size/scale/context.
    ///
    /// Lower is better: `(context_penalty, size_bucket, scale_penalty, distance)`.
    pub fn score(&self, dir: &ThemeDir, size: u32, scale: u32) -> (u8, u8, u8, u32) {
        let context_penalty =
            if dir.context.is_empty() || dir.context.eq_ignore_ascii_case("Applications") {
                0
            } else {
                1
            };
        let scale_penalty = if dir.scale == scale { 0 } else { 1 };

        match dir.kind {
            DirType::Scalable => (context_penalty, 0, scale_penalty, 0),
            DirType::Threshold => {
                let delta = dir.size.abs_diff(size);
                if delta <= dir.threshold {
                    (context_penalty, 0, scale_penalty, delta)
                } else {
                    (context_penalty, 2, scale_penalty, delta)
                }
            }
            DirType::Fixed => {
                let bucket = if dir.size == size {
                    0
                } else if dir.size > size {
                    1
                } else {
                    2
                };
                (
                    context_penalty,
                    bucket,
                    scale_penalty,
                    dir.size.abs_diff(size),
                )
            }
        }
    }
}

/// Roots searched for icon themes.
pub fn theme_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let data_dirs = data_dirs();
    for dir in &data_dirs {
        roots.push(dir.join("icons"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".icons"));
    }
    dedup(roots)
}

/// Roots searched for legacy flat icons (`/usr/share/pixmaps`).
pub fn pixmap_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for dir in data_dirs() {
        dirs.push(dir.join("pixmaps"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".icons"));
    }
    dedup(dirs)
}

fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::data_dir);
    if let Some(home) = home {
        dirs.push(home.clone());
        dirs.push(home.join("flatpak/exports/share"));
    }
    let system: Vec<PathBuf> = std::env::var_os("XDG_DATA_DIRS")
        .filter(|v| !v.is_empty())
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    dirs.extend(system);
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/flatpak/exports/share"));
    }
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    dedup(dirs)
}

fn dedup(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths
}

/// Resolves icon names to files, honouring theme inheritance.
pub struct IconResolver {
    roots: Vec<PathBuf>,
    pixmaps: Vec<PathBuf>,
    themes: HashMap<String, Option<Arc<ThemeIndex>>>,
    preferred: String,
}

impl IconResolver {
    /// `requested` empty means auto-detect (GTK setting, then Adwaita/hicolor).
    pub fn new(requested: &str) -> Self {
        let preferred = if requested.trim().is_empty() {
            detect_theme().unwrap_or_else(|| "hicolor".to_string())
        } else {
            requested.trim().to_string()
        };
        Self {
            roots: theme_roots(),
            pixmaps: pixmap_dirs(),
            themes: HashMap::new(),
            preferred,
        }
    }

    pub fn theme_name(&self) -> &str {
        &self.preferred
    }

    /// Find the file for `name` at (approximately) `size` pixels.
    pub fn find(&mut self, name: &str, size: u32) -> Option<PathBuf> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let direct = Path::new(name);
        if direct.is_absolute() {
            return direct.is_file().then(|| direct.to_path_buf());
        }

        let has_extension = ["png", "svg", "svgz", "xpm"]
            .iter()
            .any(|ext| name.to_ascii_lowercase().ends_with(&format!(".{ext}")));

        let start = self.preferred.clone();
        for theme in self.theme_chain(&start) {
            if let Some(found) = self.search_theme(&theme, name, size, has_extension) {
                return Some(found);
            }
        }

        // Legacy flat icon directories.
        let extensions: &[&str] = if has_extension {
            &[""]
        } else {
            &["png", "svg", "svgz"]
        };
        for dir in &self.pixmaps {
            for ext in extensions {
                let candidate = if ext.is_empty() {
                    dir.join(name)
                } else {
                    dir.join(format!("{name}.{ext}"))
                };
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    fn search_theme(
        &self,
        theme: &ThemeIndex,
        name: &str,
        size: u32,
        has_extension: bool,
    ) -> Option<PathBuf> {
        let mut dirs: Vec<&ThemeDir> = theme.dirs.iter().collect();
        dirs.sort_by_key(|dir| theme.score(dir, size, 1));
        for dir in dirs {
            if !dir.path.is_dir() {
                continue;
            }
            if has_extension {
                let candidate = dir.path.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
                continue;
            }
            // Prefer vector icons for scalable dirs, raster otherwise.
            let extensions: &[&str] = match dir.kind {
                DirType::Scalable => &["svg", "svgz", "png"],
                _ => &["png", "svg", "svgz"],
            };
            for ext in extensions {
                let candidate = dir.path.join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn theme_chain(&mut self, start: &str) -> Vec<Arc<ThemeIndex>> {
        use std::collections::{HashSet, VecDeque};

        let mut chain: Vec<Arc<ThemeIndex>> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start.to_string());

        while let Some(name) = queue.pop_front() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if let Some(theme) = self.load_theme(&name) {
                for inherited in &theme.inherits {
                    if !seen.contains(inherited) {
                        queue.push_back(inherited.clone());
                    }
                }
                chain.push(theme);
            }
        }

        // `hicolor` is the mandatory last-resort theme of the spec.
        if !seen.contains("hicolor") {
            if let Some(theme) = self.load_theme("hicolor") {
                chain.push(theme);
            }
        }
        chain
    }

    fn load_theme(&mut self, name: &str) -> Option<Arc<ThemeIndex>> {
        if let Some(cached) = self.themes.get(name) {
            return cached.clone();
        }
        let mut loaded = None;
        for root in &self.roots {
            let candidate = root.join(name);
            let index = candidate.join("index.theme");
            if index.is_file() {
                if let Some(theme) = ThemeIndex::parse(&index) {
                    loaded = Some(Arc::new(theme));
                    break;
                }
            }
        }
        self.themes.insert(name.to_string(), loaded.clone());
        loaded
    }
}

/// Try to read the icon theme configured in GTK settings.
pub fn detect_theme() -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(config) = dirs::config_dir() {
        candidates.push(config.join("gtk-4.0/settings.ini"));
        candidates.push(config.join("gtk-3.0/settings.ini"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config/gtk-4.0/settings.ini"));
        candidates.push(home.join(".config/gtk-3.0/settings.ini"));
    }
    for path in candidates {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            if let Some(value) = line.trim().strip_prefix("gtk-icon-theme-name") {
                let value = value.trim_start().trim_start_matches('=').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    // Sensible fallback on systems without GTK config.
    for fallback in ["Adwaita", "hicolor"] {
        let roots = theme_roots();
        if roots
            .iter()
            .any(|r| r.join(fallback).join("index.theme").is_file())
        {
            return Some(fallback.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "i3l-icons-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&base);
        let icons = base.join("icons");
        let theme = icons.join("TestTheme");
        fs::create_dir_all(theme.join("16x16/apps")).unwrap();
        fs::create_dir_all(theme.join("48x48/apps")).unwrap();
        fs::create_dir_all(theme.join("scalable/apps")).unwrap();
        fs::write(
            theme.join("index.theme"),
            "[Icon Theme]\nName=Test\nInherits=Fallback\nDirectories=16x16/apps,48x48/apps,scalable/apps\n\
             [16x16/apps]\nSize=16\nContext=Applications\nType=Fixed\n\
             [48x48/apps]\nSize=48\nContext=Applications\nType=Fixed\n\
             [scalable/apps]\nSize=48\nMinSize=8\nMaxSize=512\nContext=Applications\nType=Scalable\n",
        )
        .unwrap();
        fs::write(theme.join("16x16/apps/exact.png"), b"x").unwrap();
        fs::write(theme.join("48x48/apps/large.png"), b"x").unwrap();
        fs::write(theme.join("scalable/apps/vector.svg"), b"x").unwrap();

        let fallback = icons.join("Fallback");
        fs::create_dir_all(fallback.join("32x32/apps")).unwrap();
        fs::write(
            fallback.join("index.theme"),
            "[Icon Theme]\nName=Fallback\nInherits=hicolor\nDirectories=32x32/apps\n\
             [32x32/apps]\nSize=32\nContext=Applications\nType=Fixed\n",
        )
        .unwrap();
        fs::write(fallback.join("32x32/apps/inherited.png"), b"x").unwrap();
        (base, icons)
    }

    #[test]
    fn finds_exact_size_first() {
        let (base, icons) = fixture();
        let mut resolver = IconResolver {
            roots: vec![icons],
            pixmaps: vec![],
            themes: HashMap::new(),
            preferred: "TestTheme".into(),
        };
        let path = resolver.find("exact", 16).unwrap();
        assert_eq!(path, base.join("icons/TestTheme/16x16/apps/exact.png"));
        // 48 is closer to the large fixed dir than to 16.
        let path = resolver.find("large", 48).unwrap();
        assert_eq!(path, base.join("icons/TestTheme/48x48/apps/large.png"));
        let path = resolver.find("vector", 96).unwrap();
        assert_eq!(path, base.join("icons/TestTheme/scalable/apps/vector.svg"));
        let path = resolver.find("inherited", 32).unwrap();
        assert_eq!(path, base.join("icons/Fallback/32x32/apps/inherited.png"));
        assert!(resolver.find("missing", 32).is_none());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_unknown_values() {
        assert!(ThemeIndex::parse_str("lixo", Path::new("/tmp")).is_none());
    }
}
