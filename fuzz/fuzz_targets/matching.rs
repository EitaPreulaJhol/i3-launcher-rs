//! Search scoring: no panics, sane highlight positions, and the incremental
//! filter agreeing with a full filter while the query grows.
#![no_main]

use launcher_rs::app::{Action, Item};
use launcher_rs::config::MatchMode;
use launcher_rs::matching::{filter, match_query, IncrementalFilter, MatchOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (Vec<String>, String, u8)| {
    let (labels, query, flags) = input;
    let hay_chars = |label: &str| label.chars().count() as u32;

    for mode in [MatchMode::Prefix, MatchMode::Substring, MatchMode::Fuzzy] {
        for label in &labels {
            if let Some((_, positions)) = match_query(label, &query, mode) {
                assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(positions.iter().all(|&p| p < hay_chars(label).max(1)));
            }
        }

        let items: Vec<Item> = labels
            .iter()
            .map(|label| Item::new(label.clone(), None, Action::Print(label.clone())))
            .collect();
        let options = MatchOptions {
            mode,
            case_sensitive: flags & 1 != 0,
            sort: flags & 2 != 0,
            max_results: usize::from(flags >> 4),
        };
        let mut incremental = IncrementalFilter::new();
        let mut typed = String::new();
        for c in query.chars().take(16) {
            typed.push(c);
            assert_eq!(
                incremental.filter(&items, &typed, &options),
                filter(&items, &typed, &options),
                "query {typed:?}"
            );
        }
    }
});
