//! TOML configuration and the value parsers shared with the CLI.
#![no_main]

use launcher_rs::config::{Color, Config, Dimension, MonitorTarget};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|raw: &str| {
    let _ = Config::from_str(raw, None);
    if let Ok(color) = raw.parse::<Color>() {
        // Display must produce something the parser accepts again.
        let again: Color = color.to_string().parse().expect("Color round trip");
        assert_eq!(again, color);
    }
    if let Ok(dimension) = raw.parse::<Dimension>() {
        let _ = dimension.resolve(u32::MAX);
    }
    let _ = raw.parse::<MonitorTarget>();
});
