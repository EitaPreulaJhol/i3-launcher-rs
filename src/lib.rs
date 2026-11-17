//! i3-launcher-rs: a dmenu-style application launcher strip for i3.
//!
//! The crate is split so that everything except the X11 layer is testable
//! without a display: `.desktop` parsing/scanning, icon theme resolution,
//! search, line editing and layout geometry.

pub mod app;
pub mod cli;
pub mod config;
pub mod desktop;
pub mod icons;
pub mod matching;
pub mod theme;
pub mod ui;
pub mod x11;
