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
    match Query::new(query, opts) {
        None => everything(items.len(), opts),
        Some(query) => finish(collect(items, 0..items.len(), &query, opts), opts),
    }
}

/// Filter that reuses the previous result while the query only grows.
///
/// Every mode is monotonic: an item that matches `abc` also matches `ab`, so
/// typing one more character only has to re-check the items that matched
/// before. The result is identical to [`filter`]; the full (untruncated,
/// unsorted) match set is kept so `max_results` and `sort` stay exact.
#[derive(Debug, Default)]
pub struct IncrementalFilter {
    previous: Option<Previous>,
}

#[derive(Debug)]
struct Previous {
    /// Normalized (possibly lowercased) query that produced `indices`.
    query: String,
    mode: MatchMode,
    case_sensitive: bool,
    item_count: usize,
    /// Every matching item index, ascending.
    indices: Vec<usize>,
}

impl IncrementalFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget the previous result (call when the item list changes).
    pub fn reset(&mut self) {
        self.previous = None;
    }

    pub fn filter(&mut self, items: &[Item], query: &str, opts: &MatchOptions) -> Vec<Match> {
        let Some(query) = Query::new(query, opts) else {
            self.previous = None;
            return everything(items.len(), opts);
        };

        let reusable = self.previous.as_ref().filter(|previous| {
            previous.mode == opts.mode
                && previous.case_sensitive == opts.case_sensitive
                && previous.item_count == items.len()
                && query.text.starts_with(previous.query.as_str())
        });
        let matches = match reusable {
            Some(previous) => collect(items, previous.indices.iter().copied(), &query, opts),
            None => collect(items, 0..items.len(), &query, opts),
        };

        self.previous = Some(Previous {
            indices: matches.iter().map(|m| m.index).collect(),
            query: query.text,
            mode: opts.mode,
            case_sensitive: opts.case_sensitive,
            item_count: items.len(),
        });
        finish(matches, opts)
    }
}

/// A query prepared once per filtering pass instead of once per item.
struct Query {
    text: String,
    chars: Vec<char>,
}

impl Query {
    /// `None` for blank queries, which match everything.
    fn new(raw: &str, opts: &MatchOptions) -> Option<Self> {
        if raw.trim().is_empty() {
            return None;
        }
        let text = if opts.case_sensitive {
            raw.to_string()
        } else {
            raw.to_lowercase()
        };
        let chars = text.chars().collect();
        Some(Self { text, chars })
    }
}

fn max_results(opts: &MatchOptions) -> usize {
    if opts.max_results == 0 {
        usize::MAX
    } else {
        opts.max_results
    }
}

fn everything(len: usize, opts: &MatchOptions) -> Vec<Match> {
    (0..len.min(max_results(opts)))
        .map(|index| Match {
            index,
            score: 0,
            positions: Vec::new(),
        })
        .collect()
}

/// Score `indices` (ascending) against `query`, keeping item order.
fn collect(
    items: &[Item],
    indices: impl Iterator<Item = usize>,
    query: &Query,
    opts: &MatchOptions,
) -> Vec<Match> {
    let mut matches = Vec::new();
    for index in indices {
        let item = &items[index];
        let haystack = if opts.case_sensitive {
            item.label.as_str()
        } else {
            item.search.as_str()
        };
        if let Some((score, positions)) = score(haystack, query, opts.mode) {
            matches.push(Match {
                index,
                score,
                positions,
            });
        }
    }
    matches
}

fn finish(mut matches: Vec<Match>, opts: &MatchOptions) -> Vec<Match> {
    if opts.sort {
        // Stable: equal scores keep directory order.
        matches.sort_by_key(|m| std::cmp::Reverse(m.score));
    }
    matches.truncate(max_results(opts));
    matches
}

/// Score a single haystack; `None` when it does not match.
pub fn match_query(haystack: &str, query: &str, mode: MatchMode) -> Option<(i32, Vec<u32>)> {
    let query = Query {
        text: query.to_string(),
        chars: query.chars().collect(),
    };
    score(haystack, &query, mode)
}

fn score(haystack: &str, query: &Query, mode: MatchMode) -> Option<(i32, Vec<u32>)> {
    match mode {
        MatchMode::Prefix => score_prefix(haystack, query),
        MatchMode::Substring => score_substring(haystack, query),
        MatchMode::Fuzzy => score_fuzzy(haystack, &query.chars),
    }
}

fn score_prefix(haystack: &str, query: &Query) -> Option<(i32, Vec<u32>)> {
    if !haystack.starts_with(query.text.as_str()) {
        return None;
    }
    let query_len = query.chars.len() as i32;
    let len_diff = haystack.chars().count() as i32 - query_len;
    Some((2000 - len_diff, (0..query_len as u32).collect()))
}

fn score_substring(haystack: &str, query: &Query) -> Option<(i32, Vec<u32>)> {
    let byte_pos = haystack.find(query.text.as_str())?;
    let char_pos = haystack[..byte_pos].chars().count();
    let hay_len = haystack.chars().count() as i32;
    let query_len = query.chars.len() as i32;

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

/// Greedy subsequence match, walking the haystack once without collecting it.
fn score_fuzzy(haystack: &str, needle: &[char]) -> Option<(i32, Vec<u32>)> {
    if needle.is_empty() {
        return Some((0, Vec::new()));
    }

    let mut positions = Vec::with_capacity(needle.len());
    let mut chars = haystack.chars().enumerate();
    // Character right before the cursor, for word-boundary bonuses.
    let mut before: Option<char> = None;
    let mut score = 0i32;
    let mut previous: Option<usize> = None;

    for (needle_idx, wanted) in needle.iter().enumerate() {
        let (found, preceding) = loop {
            let (index, char) = chars.next()?;
            let preceding = before;
            before = Some(char);
            if char == *wanted {
                break (index, preceding);
            }
        };

        positions.push(found as u32);
        score += 20;
        if found == 0 || is_word_char_boundary(preceding) {
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

    let consumed = previous.map_or(0, |last| last + 1);
    let hay_len = consumed + chars.count();
    let excess = hay_len.saturating_sub(needle.len()) as i32;
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

    /// The original fuzzy scorer (collects both strings), kept as a reference
    /// for the single-pass implementation.
    fn reference_fuzzy(haystack: &str, query: &str) -> Option<(i32, Vec<u32>)> {
        let hay: Vec<char> = haystack.chars().collect();
        let needle: Vec<char> = query.chars().collect();
        if needle.is_empty() {
            return Some((0, Vec::new()));
        }
        let mut positions = Vec::new();
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

    const LABELS: &[&str] = &[
        "Firefox",
        "Mozilla Firefox",
        "Firefox Developer Edition",
        "File Manager",
        "Xfce Terminal",
        "GNU Image Manipulation Program",
        "fire-starter",
        "Ελληνικά ΣΟΦΟΣ",
        "Ünïcödé Ēditor",
        "a",
        "",
        "  spaced  out  ",
        "LibreOffice Calc",
        "libreoffice-writer",
        "Neovim",
        "nvim-qt",
    ];

    #[test]
    fn single_pass_fuzzy_matches_reference() {
        let queries = [
            "", "f", "fi", "fir", "fm", "ff", "xyz", "nvim", "e", "ee", "σ", "ος", "ü", " ", "o o",
            "calc", "lo", "a", "aa",
        ];
        for label in LABELS {
            let hay = label.to_lowercase();
            for query in queries {
                assert_eq!(
                    match_query(&hay, query, MatchMode::Fuzzy),
                    reference_fuzzy(&hay, query),
                    "{hay:?} / {query:?}"
                );
            }
        }
    }

    #[test]
    fn incremental_filter_equals_full_filter() {
        let items: Vec<Item> = LABELS.iter().map(|label| item(label)).collect();
        // Typing sessions with growth, deletion, replacement and blanks.
        let sessions: &[&[&str]] = &[
            &["f", "fi", "fir", "fire", "fir", "fi", "fil", "file"],
            &[
                "l", "li", "lib", "libr", "x", "xf", "", "n", "nv", "nvi", "nvim",
            ],
            &["Σ", "ΣΟ", "ΣΟΦ", "ΣΟΦΟ", "ΣΟΦΟΣ"],
            &[" ", "  ", " s", " sp", "e", "e ", "e d"],
            &["F", "Fi", "FI", "FIR", "fir"],
            &["m", "ma", "man", "mana", "manag", "zzz", "zzzz"],
        ];
        for mode in [MatchMode::Prefix, MatchMode::Substring, MatchMode::Fuzzy] {
            for case_sensitive in [false, true] {
                for sort in [false, true] {
                    for max_results in [0, 1, 3] {
                        let options = MatchOptions {
                            mode,
                            case_sensitive,
                            sort,
                            max_results,
                        };
                        for session in sessions {
                            let mut incremental = IncrementalFilter::new();
                            for query in *session {
                                assert_eq!(
                                    incremental.filter(&items, query, &options),
                                    filter(&items, query, &options),
                                    "{options:?} query={query:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn incremental_filter_resets_when_items_change() {
        let options = opts(MatchMode::Substring, false);
        let first = vec![item("alpha"), item("beta")];
        let second = vec![item("gamma"), item("alphabet"), item("delta")];
        let mut incremental = IncrementalFilter::new();
        assert_eq!(incremental.filter(&first, "al", &options).len(), 1);
        // A different list of the same query prefix must not reuse indices
        // (the item count differs, so it is detected even without `reset`).
        assert_eq!(
            incremental.filter(&second, "alp", &options),
            filter(&second, "alp", &options)
        );
        incremental.reset();
        assert_eq!(
            incremental.filter(&second, "alph", &options),
            filter(&second, "alph", &options)
        );
    }
}
