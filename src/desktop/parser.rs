//! Parser for the freedesktop.org Desktop Entry specification.
//!
//! Only the `[Desktop Entry]` group is considered, as that is the only group
//! that describes applications.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

/// A parsed `.desktop` file. Fields are kept close to the spec; filtering of
/// entries (Type, `NoDisplay`, `TryExec`, ...) happens in [`crate::desktop::scan`].
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    /// Desktop file ID: path relative to the applications dir, `/` replaced by `-`.
    pub id: String,
    pub source: PathBuf,
    pub entry_type: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    /// Raw `Exec` value (field codes not yet expanded).
    pub exec: String,
    pub try_exec: Option<String>,
    pub icon: Option<String>,
    pub terminal: bool,
    pub no_display: bool,
    pub hidden: bool,
    pub only_show_in: Vec<String>,
    pub not_show_in: Vec<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    /// `Path=` key: working directory for the command.
    pub working_dir: Option<PathBuf>,
    pub startup_wm_class: Option<String>,
}

impl DesktopEntry {
    /// Whether the entry is meant to appear in a launcher.
    pub fn is_visible_application(&self) -> bool {
        self.entry_type == "Application" && !self.no_display && !self.hidden
    }

    /// Lowercased blob used by the search engine.
    pub fn search_text(&self) -> String {
        let mut out = String::with_capacity(self.name.len() + 64);
        out.push_str(&self.name);
        for extra in self
            .generic_name
            .iter()
            .chain(self.keywords.iter())
            .chain(self.categories.iter())
        {
            out.push(' ');
            out.push_str(extra);
        }
        out.to_lowercase()
    }
}

/// Language fallback chain, e.g. `["pt_BR", "pt"]`.
#[derive(Debug, Clone, Default)]
pub struct Locale {
    keys: Vec<String>,
}

impl Locale {
    pub fn from_env() -> Self {
        Self::from_vars(
            std::env::var("LANGUAGE").ok().as_deref(),
            std::env::var("LC_ALL").ok().as_deref(),
            std::env::var("LC_MESSAGES").ok().as_deref(),
            std::env::var("LANG").ok().as_deref(),
        )
    }

    pub fn from_vars(
        language: Option<&str>,
        lc_all: Option<&str>,
        lc_messages: Option<&str>,
        lang: Option<&str>,
    ) -> Self {
        let mut keys: Vec<String> = Vec::new();
        let mut push = |raw: &str| {
            let raw = raw.trim();
            if raw.is_empty() || raw == "C" || raw == "POSIX" {
                return;
            }
            // Drop the codeset/charset part: pt_BR.UTF-8 -> pt_BR
            let raw = raw.split('.').next().unwrap_or(raw);
            let raw = raw.split('@').next().unwrap_or(raw);
            if raw.is_empty() {
                return;
            }
            if !keys.iter().any(|k| k == raw) {
                keys.push(raw.to_string());
            }
            // Base language, used when there is no exact translation.
            if let Some((base, _)) = raw.split_once('_') {
                if !keys.iter().any(|k| k == base) {
                    keys.push(base.to_string());
                }
            }
        };

        if let Some(list) = language {
            for item in list.split(':') {
                push(item);
            }
        }
        for var in [lc_all, lc_messages, lang].into_iter().flatten() {
            push(var);
        }

        Self { keys }
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Parse a `.desktop` file from disk.
pub fn parse_file(path: &Path, id: &str, locale: &Locale) -> Result<DesktopEntry> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("could not read `{}`: {e}", path.display()))?;
    parse_str(&content, path, id, locale)
}

/// Parse `.desktop` content.
pub fn parse_str(content: &str, source: &Path, id: &str, locale: &Locale) -> Result<DesktopEntry> {
    let mut values: HashMap<String, String> = HashMap::new();
    let mut in_desktop_entry = false;
    let mut seen_group = false;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let group = &trimmed[1..trimmed.len() - 1];
            in_desktop_entry = group == "Desktop Entry";
            seen_group |= in_desktop_entry;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            // Malformed line: the spec says to ignore it.
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        // The spec says later duplicates must be ignored.
        values
            .entry(key.to_string())
            .or_insert_with(|| value.trim_start().to_string());
    }

    if !seen_group {
        bail!("{}: missing [Desktop Entry] group", source.display());
    }

    let entry_type = values
        .get("Type")
        .cloned()
        .unwrap_or_else(|| "Application".to_string());
    let name = localized(&values, "Name", locale)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("{}: missing `Name` key", source.display()))?;
    let exec = values
        .get("Exec")
        .cloned()
        .ok_or_else(|| anyhow!("{}: missing `Exec` key", source.display()))?;
    // Reject unparsable Exec early instead of at launch time.
    tokenize_exec(&exec).map_err(|e| anyhow!("{}: invalid Exec: {e}", source.display()))?;

    Ok(DesktopEntry {
        id: id.to_string(),
        source: source.to_path_buf(),
        entry_type,
        name,
        generic_name: localized(&values, "GenericName", locale).filter(|s| !s.is_empty()),
        comment: localized(&values, "Comment", locale).filter(|s| !s.is_empty()),
        exec,
        try_exec: values.get("TryExec").cloned().filter(|s| !s.is_empty()),
        icon: values.get("Icon").cloned().filter(|s| !s.is_empty()),
        terminal: parse_bool(values.get("Terminal")),
        no_display: parse_bool(values.get("NoDisplay")),
        hidden: parse_bool(values.get("Hidden")),
        only_show_in: parse_list(values.get("OnlyShowIn")),
        not_show_in: parse_list(values.get("NotShowIn")),
        categories: parse_list(values.get("Categories")),
        keywords: localized(&values, "Keywords", locale)
            .map(|s| parse_list(Some(&s)))
            .unwrap_or_default(),
        working_dir: values
            .get("Path")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        startup_wm_class: values
            .get("StartupWMClass")
            .cloned()
            .filter(|s| !s.is_empty()),
    })
}

fn localized(values: &HashMap<String, String>, base: &str, locale: &Locale) -> Option<String> {
    for key in locale.keys() {
        if let Some(value) = values.get(&format!("{base}[{key}]")) {
            if !value.is_empty() {
                return Some(unescape(value));
            }
        }
    }
    values.get(base).map(|v| unescape(v))
}

fn parse_bool(value: Option<&String>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()),
        Some(v) if v == "true" || v == "1"
    )
}

fn parse_list(value: Option<&String>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    value
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(unescape)
        .collect()
}

/// Unescape the `\s`, `\n`, `\t`, `\r` and `\\` sequences of string values.
pub fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Tokenize an `Exec` value according to the Desktop Entry spec.
///
/// Arguments may be double quoted; inside quotes `\"`, `` \` ``, `\$` and
/// `\\` are the only valid escapes.
pub fn tokenize_exec(exec: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            '\\' if in_quotes => match chars.next() {
                Some(e @ ('"' | '`' | '$' | '\\')) => {
                    current.push(e);
                    has_token = true;
                }
                Some(other) => {
                    current.push('\\');
                    current.push(other);
                    has_token = true;
                }
                None => bail!("backslash at the end of the string"),
            },
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    has_token = true;
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }

    if in_quotes {
        bail!("unterminated double quote");
    }
    if has_token {
        args.push(current);
    }
    if args.is_empty() {
        bail!("empty command");
    }
    Ok(args)
}

/// Context needed to expand field codes in `Exec`.
#[derive(Debug, Clone, Copy)]
pub struct ExecContext<'a> {
    pub name: &'a str,
    pub icon: Option<&'a str>,
    pub desktop_file: &'a Path,
}

/// Expand the field codes of an already tokenized `Exec`.
///
/// `%f/%F/%u/%U` expand to nothing because we never launch with files or URLs;
/// bare arguments that become empty are dropped, embedded ones are kept.
pub fn expand_exec(args: &[String], ctx: &ExecContext<'_>) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len() + 2);
    for arg in args {
        if arg == "%i" {
            if let Some(icon) = ctx.icon {
                out.push("--icon".to_string());
                out.push(icon.to_string());
            }
            continue;
        }

        let mut expanded = String::with_capacity(arg.len());
        let mut skip_arg = false;
        let mut chars = arg.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                expanded.push(c);
                continue;
            }
            let Some(code) = chars.next() else {
                expanded.push('%');
                break;
            };
            match code {
                '%' => expanded.push('%'),
                'f' | 'F' | 'u' | 'U' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm' | 'x' | 'X' => {
                    // No files/URLs/deprecated codes to substitute.
                    if expanded.is_empty() {
                        skip_arg = true;
                    }
                }
                'c' => expanded.push_str(ctx.name),
                'k' => expanded.push_str(&ctx.desktop_file.display().to_string()),
                'i' => {
                    if let Some(icon) = ctx.icon {
                        out.push("--icon".to_string());
                        out.push(icon.to_string());
                    }
                }
                other => {
                    // Unknown field code: remove it, per the spec.
                    let _ = other;
                }
            }
        }

        if !skip_arg {
            out.push(expanded);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locale() -> Locale {
        Locale::from_vars(None, None, None, Some("pt_BR.UTF-8"))
    }

    const SAMPLE: &str = r#"
[Desktop Entry]
Type=Application
Name=Firefox
Name[pt_BR]=Navegador Firefox
GenericName=Web Browser
Comment=Explore the Web
Exec=firefox %u
Icon=firefox
Terminal=false
Categories=Network;WebBrowser;
Keywords=web;browser;
"#;

    #[test]
    fn parses_localized_fields() {
        let e = parse_str(
            SAMPLE,
            Path::new("/usr/share/applications/firefox.desktop"),
            "firefox",
            &locale(),
        )
        .unwrap();
        assert_eq!(e.name, "Navegador Firefox");
        assert_eq!(e.generic_name.as_deref(), Some("Web Browser"));
        assert_eq!(e.categories, vec!["Network", "WebBrowser"]);
        assert_eq!(e.keywords, vec!["web", "browser"]);
        assert!(e.is_visible_application());
    }

    #[test]
    fn falls_back_to_base_name() {
        let e = parse_str(
            SAMPLE,
            Path::new("x.desktop"),
            "firefox",
            &Locale::from_vars(None, None, None, Some("fr_FR")),
        )
        .unwrap();
        assert_eq!(e.name, "Firefox");
    }

    #[test]
    fn duplicate_keys_keep_the_first_value() {
        let raw = "[Desktop Entry]\nType=Application\nName=First\nName=Second\nExec=x\n";
        let entry = parse_str(raw, Path::new("x.desktop"), "x", &Locale::default()).unwrap();
        assert_eq!(entry.name, "First");
    }

    #[test]
    fn unescapes_strings() {
        assert_eq!(unescape(r"a\sb\nc\\d"), "a b\nc\\d");
    }

    #[test]
    fn tokenizes_quotes_and_escapes() {
        let args = tokenize_exec(r#"prog --flag="a b" "x\"y" plain"#).unwrap();
        assert_eq!(args, vec!["prog", "--flag=a b", "x\"y", "plain"]);
        let empty = tokenize_exec(r#"prog "" x"#).unwrap();
        assert_eq!(empty, vec!["prog", "", "x"]);
        assert!(tokenize_exec("prog \"unterminated").is_err());
    }

    #[test]
    fn expands_field_codes() {
        let args = tokenize_exec(r#"app --file=%f %U --name=%c %i %% done"#).unwrap();
        let ctx = ExecContext {
            name: "My App",
            icon: Some("my-icon"),
            desktop_file: Path::new("/usr/share/applications/my.desktop"),
        };
        let expanded = expand_exec(&args, &ctx);
        assert_eq!(
            expanded,
            vec![
                "app",
                "--file=",
                "--name=My App",
                "--icon",
                "my-icon",
                "%",
                "done"
            ]
        );
    }

    #[test]
    fn rejects_missing_exec() {
        let err = parse_str(
            "[Desktop Entry]\nType=Application\nName=X\n",
            Path::new("x.desktop"),
            "x",
            &Locale::default(),
        );
        assert!(err.is_err());
    }
}
