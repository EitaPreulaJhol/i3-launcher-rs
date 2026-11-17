//! Monitor discovery (RandR 1.5 → Xinerama → screen) and bar geometry.

use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as RandrExt;
use x11rb::protocol::xinerama::ConnectionExt as XineramaExt;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoExt, Window};

use crate::config::{Align, Config, Edge, MonitorTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub rect: Rect,
    pub name: String,
    pub primary: bool,
}

/// Enumerate monitors, falling back gracefully when extensions are missing.
pub fn list<C: Connection>(
    conn: &C,
    root: Window,
    screen_width: u16,
    screen_height: u16,
) -> Vec<Monitor> {
    if let Ok(monitors) = randr_monitors(conn, root) {
        if !monitors.is_empty() {
            return monitors;
        }
    }
    if let Ok(monitors) = xinerama_monitors(conn) {
        if !monitors.is_empty() {
            return monitors;
        }
    }
    vec![Monitor {
        rect: Rect {
            x: 0,
            y: 0,
            width: screen_width,
            height: screen_height,
        },
        name: "screen".to_string(),
        primary: true,
    }]
}

fn randr_monitors<C: Connection>(conn: &C, root: Window) -> Result<Vec<Monitor>> {
    let reply = conn.randr_get_monitors(root, true)?.reply()?;
    let mut monitors = Vec::with_capacity(reply.monitors.len());
    for monitor in reply.monitors {
        let name = conn
            .get_atom_name(monitor.name)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| String::from_utf8_lossy(&reply.name).into_owned())
            .unwrap_or_else(|| format!("#{:?}", monitor.name));
        monitors.push(Monitor {
            rect: Rect {
                x: monitor.x,
                y: monitor.y,
                width: monitor.width,
                height: monitor.height,
            },
            name,
            primary: monitor.primary,
        });
    }
    Ok(monitors)
}

fn xinerama_monitors<C: Connection>(conn: &C) -> Result<Vec<Monitor>> {
    let reply = conn.xinerama_query_screens()?.reply()?;
    Ok(reply
        .screen_info
        .iter()
        .enumerate()
        .map(|(i, screen)| Monitor {
            rect: Rect {
                x: screen.x_org,
                y: screen.y_org,
                width: screen.width,
                height: screen.height,
            },
            name: format!("xinerama-{i}"),
            primary: i == 0,
        })
        .collect())
}

/// Resolve `MonitorTarget` to an index in `monitors`.
pub fn select<C: Connection>(
    conn: &C,
    root: Window,
    monitors: &[Monitor],
    target: &MonitorTarget,
) -> usize {
    if monitors.is_empty() {
        return 0;
    }
    match target {
        MonitorTarget::Index(index) => (*index).min(monitors.len() - 1),
        MonitorTarget::Name(name) => monitors
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| primary_index(conn, root, monitors)),
        MonitorTarget::Primary => primary_index(conn, root, monitors),
        MonitorTarget::Pointer => pointer_monitor(conn, root, monitors)
            .unwrap_or_else(|| primary_index(conn, root, monitors)),
        MonitorTarget::Focused => focused_monitor(conn, root, monitors)
            .or_else(|| pointer_monitor(conn, root, monitors))
            .unwrap_or_else(|| primary_index(conn, root, monitors)),
    }
}

fn primary_index<C: Connection>(conn: &C, root: Window, monitors: &[Monitor]) -> usize {
    if let Some(index) = monitors.iter().position(|m| m.primary) {
        return index;
    }
    // RandR's `primary` flag may be unset; ask the server directly.
    let output = conn
        .randr_get_output_primary(root)
        .ok()
        .and_then(|cookie| cookie.reply().ok());
    if let Some(output) = output {
        let reply = conn
            .randr_get_monitors(root, true)
            .ok()
            .and_then(|cookie| cookie.reply().ok());
        if let Some(reply) = reply {
            for (i, monitor) in reply.monitors.iter().enumerate() {
                if monitor.outputs.contains(&output.output) {
                    return i.min(monitors.len() - 1);
                }
            }
        }
    }
    0
}

fn pointer_monitor<C: Connection>(conn: &C, root: Window, monitors: &[Monitor]) -> Option<usize> {
    let reply = conn.query_pointer(root).ok()?.reply().ok()?;
    let (x, y) = (reply.root_x as i32, reply.root_y as i32);
    monitors
        .iter()
        .position(|monitor| contains(monitor.rect, x, y))
        .or_else(|| {
            // Pointer outside all monitors: choose the nearest one.
            monitors
                .iter()
                .enumerate()
                .max_by_key(|(_, monitor)| clamp_area(monitor.rect, x, y, 1, 1))
                .map(|(index, _)| index)
        })
}

fn focused_monitor<C: Connection>(conn: &C, root: Window, monitors: &[Monitor]) -> Option<usize> {
    let active = active_window(conn, root)?;
    let (x, y, width, height) = window_frame_geometry(conn, root, active)?;
    let mut best = None;
    let mut best_area = i64::MIN;
    for (index, monitor) in monitors.iter().enumerate() {
        let area = clamp_area(monitor.rect, x, y, width, height);
        if area > best_area {
            best_area = area;
            best = Some(index);
        }
    }
    best.filter(|_| best_area > 0)
}

fn active_window<C: Connection>(conn: &C, root: Window) -> Option<Window> {
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let mut values = reply.value32()?;
    values.next().filter(|window| *window != 0)
}

/// Absolute geometry of the top-level window containing `window`.
fn window_frame_geometry<C: Connection>(
    conn: &C,
    root: Window,
    window: Window,
) -> Option<(i32, i32, i32, i32)> {
    let mut top = window;
    for _ in 0..16 {
        let tree = conn.query_tree(top).ok()?.reply().ok()?;
        if tree.parent == root || tree.parent == 0 {
            break;
        }
        top = tree.parent;
    }
    let geometry = conn.get_geometry(top).ok()?.reply().ok()?;
    let translated = conn
        .translate_coordinates(top, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    Some((
        translated.dst_x as i32,
        translated.dst_y as i32,
        geometry.width as i32,
        geometry.height as i32,
    ))
}

fn contains(rect: Rect, x: i32, y: i32) -> bool {
    x >= rect.x as i32
        && y >= rect.y as i32
        && x < rect.x as i32 + rect.width as i32
        && y < rect.y as i32 + rect.height as i32
}

fn clamp_area(rect: Rect, x: i32, y: i32, width: i32, height: i32) -> i64 {
    let left = (rect.x as i32).max(x);
    let top = (rect.y as i32).max(y);
    let right = (rect.x as i32 + rect.width as i32).min(x + width);
    let bottom = (rect.y as i32 + rect.height as i32).min(y + height);
    let w = (right - left).max(0) as i64;
    let h = (bottom - top).max(0) as i64;
    w * h
}

/// Geometry of the launcher strip inside `monitor`.
pub fn bar_rect(monitor: Rect, cfg: &Config) -> Rect {
    let margin = cfg.bar.margin as i32;
    let mon_width = monitor.width as i32;
    let height = (cfg.bar.height as i32).clamp(1, monitor.height as i32);
    let usable_width = (mon_width - 2 * margin).max(1);
    let width = cfg
        .bar
        .width
        .map(|dimension| dimension.resolve(monitor.width as u32) as i32)
        .unwrap_or(usable_width)
        .clamp(1, usable_width);

    let align_offset = match cfg.bar.align {
        Align::Left => margin,
        Align::Center => (mon_width - width) / 2,
        Align::Right => mon_width - width - margin,
    };

    let x = monitor.x as i32 + align_offset;
    let y = match cfg.bar.edge {
        Edge::Top => monitor.y as i32 + cfg.bar.offset as i32,
        Edge::Bottom => monitor.y as i32 + monitor.height as i32 - height - cfg.bar.offset as i32,
    };

    Rect {
        x: x as i16,
        width: width as u16,
        y: y as i16,
        height: height as u16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Dimension;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn bar_at_top_full_width() {
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let rect = bar_rect(monitor, &config());
        assert_eq!(
            rect,
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 20
            }
        );
    }

    #[test]
    fn bar_at_bottom_with_offset_and_margin() {
        let mut cfg = config();
        cfg.bar.edge = Edge::Bottom;
        cfg.bar.offset = 4;
        cfg.bar.margin = 10;
        cfg.bar.height = 40;
        let monitor = Rect {
            x: 100,
            y: 50,
            width: 1000,
            height: 800,
        };
        let rect = bar_rect(monitor, &cfg);
        assert_eq!(
            rect,
            Rect {
                x: 110,
                y: 50 + 800 - 40 - 4,
                width: 980,
                height: 40
            }
        );
    }

    #[test]
    fn explicit_width_respects_margins() {
        let mut cfg = config();
        cfg.bar.margin = 10;
        cfg.bar.width = Some(Dimension::Percent(100.0));
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1000,
            height: 600,
        };
        let rect = bar_rect(monitor, &cfg);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.width, 980);
    }

    #[test]
    fn respects_percentage_width_and_alignment() {
        let mut cfg = config();
        cfg.bar.width = Some(Dimension::Percent(50.0));
        cfg.bar.align = Align::Center;
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1000,
            height: 600,
        };
        let rect = bar_rect(monitor, &cfg);
        assert_eq!(rect.x, 250);
        assert_eq!(rect.width, 500);
    }

    #[test]
    fn intersection_area() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        assert_eq!(clamp_area(rect, 50, 50, 100, 100), 2500);
        assert_eq!(clamp_area(rect, 200, 200, 10, 10), 0);
    }
}
