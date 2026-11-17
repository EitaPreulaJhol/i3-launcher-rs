//! Debug helper: dump match scores for a query.
use i3_launcher::app;
use i3_launcher::config::Config;
use i3_launcher::matching::{self, MatchOptions};

fn main() {
    let query = std::env::args().nth(1).unwrap_or_else(|| "ala".into());
    let cfg = Config::default();
    let entries = app::load_entries();
    let items = app::from_entries(&entries);
    let opts = MatchOptions {
        mode: cfg.behavior.match_mode,
        case_sensitive: cfg.behavior.case_sensitive,
        sort: cfg.behavior.sort,
        max_results: 0,
    };
    let matches = matching::filter(&items, &query, &opts);
    for m in matches.iter().take(15) {
        let item = &items[m.index];
        println!(
            "score={:5} label={:?} search={:?}",
            m.score, item.label, item.search
        );
    }
}
