//! Incremental search: prefix, substring and fuzzy subsequence matching.

use crate::app::Item;
use crate::config::MatchMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// Index into the item list.
    pub index: usize,
    pub score: i32,
    /// Matched character positions (for highlighting, v2).
    pub positions: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct MatchOptions {
    pub mode: MatchMode,
    pub case_sensitive: bool,
    pub sort: bool,
    pub max_results: usize,
}

/// Filter `items` by `query`, keeping the original order unless `sort` is set.
pub fn filter(items: &[Item], query: &str, opts: &MatchOptions) -> Vec<Match> {
    let max = if opts.max_results == 0 {
        usize::MAX
    } else {
        opts.max_results
    };

    if query.trim().is_empty() {
        return (0..items.len().min(max))
            .map(|index| Match {
                index,
                score: 0,
                positions: Vec::new(),
            })
            .collect();
    }

    let query = if opts.case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    let mut matches = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let haystack = if opts.case_sensitive {
            item.label.as_str()
        } else {
            item.search.as_str()
        };
        if let Some((score, positions)) = match_query(haystack, &query, opts.mode) {
            matches.push(Match {
                index,
                score,
                positions,
            });
        }
    }

    if opts.sort {
        // Stable: equal scores keep directory order.
        matches.sort_by_key(|m| std::cmp::Reverse(m.score));
    }
    matches.truncate(max);
    matches
}

/// Score a single haystack; `None` when it does not match.
pub fn match_query(haystack: &str, query: &str, mode: MatchMode) -> Option<(i32, Vec<u32>)> {
    match mode {
        MatchMode::Prefix => score_prefix(haystack, query),
        MatchMode::Substring => score_substring(haystack, query),
        MatchMode::Fuzzy => score_fuzzy(haystack, query),
    }
}

fn score_prefix(haystack: &str, query: &str) -> Option<(i32, Vec<u32>)> {
    if !haystack.starts_with(query) {
        return None;
    }
    let len_diff = haystack.chars().count() as i32 - query.chars().count() as i32;
    Some((2000 - len_diff, (0..query.chars().count() as u32).collect()))
}

fn score_substring(haystack: &str, query: &str) -> Option<(i32, Vec<u32>)> {
    let byte_pos = haystack.find(query)?;
    let char_pos = haystack[..byte_pos].chars().count();
    let hay_len = haystack.chars().count() as i32;
    let query_len = query.chars().count() as i32;

    let mut score = 1000 - (char_pos as i32) * 3 - (hay_len - query_len);
    if char_pos == 0 {
        score += 500;
    } else if is_boundary(haystack, byte_pos) {
        score += 250;
    }

    Some((
        score,
        (char_pos as u32..(char_pos as u32 + query_len as u32)).collect(),
    ))
}

fn score_fuzzy(haystack: &str, query: &str) -> Option<(i32, Vec<u32>)> {
    let hay: Vec<char> = haystack.chars().collect();
    let needle: Vec<char> = query.chars().collect();
    if needle.is_empty() {
        return Some((0, Vec::new()));
    }

    let mut positions = Vec::with_capacity(needle.len());
    let mut cursor = 0usize;
    let mut score = 0i32;
    let mut previous: Option<usize> = None;

    for (needle_idx, wanted) in needle.iter().enumerate() {
        let found = loop {
            let char = *hay.get(cursor)?;
            cursor += 1;
            if char == *wanted {
                break cursor - 1;
            }
        };

        positions.push(found as u32);
        score += 20;
        if found == 0 || is_word_char_boundary(hay.get(found.wrapping_sub(1)).copied()) {
            score += 30;
        }
        match previous {
            Some(prev) if found == prev + 1 => score += 25,
            Some(prev) => score -= ((found - prev - 1) as i32).min(20),
            None => {}
        }
        if needle_idx == 0 && found == 0 {
            score += 40;
        }
        previous = Some(found);
    }

    let excess = hay.len().saturating_sub(needle.len()) as i32;
    score -= excess / 2;
    Some((score, positions))
}

fn is_boundary(haystack: &str, byte_pos: usize) -> bool {
    haystack[..byte_pos]
        .chars()
        .next_back()
        .map(|c| is_word_char_boundary(Some(c)))
        .unwrap_or(true)
}

fn is_word_char_boundary(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(c) => {
            c.is_whitespace() || matches!(c, '-' | '_' | '.' | '/' | ':' | '(' | '[' | '+' | '@')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Action, Item};

    fn item(label: &str) -> Item {
        Item {
            label: label.to_string(),
            icon: None,
            search: label.to_lowercase(),
            action: Action::Print(label.to_string()),
        }
    }

    fn opts(mode: MatchMode, sort: bool) -> MatchOptions {
        MatchOptions {
            mode,
            case_sensitive: false,
            sort,
            max_results: 0,
        }
    }

    #[test]
    fn substring_prefers_prefix() {
        let items = vec![item("xterm"), item("Alacritty"), item("kitty")];
        let found = filter(&items, "term", &opts(MatchMode::Substring, true));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].index, 0);

        let items = vec![item("Mozilla Firefox"), item("Firefox Developer")];
        let found = filter(&items, "firefox", &opts(MatchMode::Substring, true));
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].index, 1);
    }

    #[test]
    fn prefix_mode_is_strict() {
        let items = vec![item("Mozilla Firefox")];
        assert!(filter(&items, "firefox", &opts(MatchMode::Prefix, true)).is_empty());
        assert_eq!(
            filter(&items, "mozilla", &opts(MatchMode::Prefix, true)).len(),
            1
        );
    }

    #[test]
    fn fuzzy_matches_subsequence_and_orders_by_score() {
        let items = vec![
            item("Xfce Terminal"),
            item("Firefox"),
            item("File Manager"),
            item("Neovim"),
        ];
        let found = filter(&items, "fm", &opts(MatchMode::Fuzzy, true));
        let labels: Vec<&str> = found
            .iter()
            .map(|m| items[m.index].label.as_str())
            .collect();
        assert_eq!(labels[0], "File Manager");
        assert!(labels.contains(&"Xfce Terminal"));

        let found = filter(&items, "nvim", &opts(MatchMode::Fuzzy, true));
        assert_eq!(items[found[0].index].label, "Neovim");
    }

    #[test]
    fn empty_query_returns_everything_and_respects_max() {
        let items = vec![item("a"), item("b"), item("c")];
        let mut options = opts(MatchMode::Fuzzy, true);
        options.max_results = 2;
        let found = filter(&items, "", &options);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].index, 1);
    }

    #[test]
    fn case_sensitive_matching() {
        let items = vec![item("Firefox")];
        let mut options = opts(MatchMode::Substring, true);
        options.case_sensitive = true;
        assert!(filter(&items, "fire", &options).is_empty());
        assert_eq!(filter(&items, "Fire", &options).len(), 1);
    }
}
