//! Software painting of the strip into a `tiny-skia` pixmap.

use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{
    FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use crate::config::{Appearance, Color, Config};
use crate::icons::IconCache;
use crate::ui::state::App;

pub struct Painter {
    font_system: FontSystem,
    swash: SwashCache,
    icons: IconCache,
    font_family: String,
    font_size: f32,
    line_height: f32,
    /// Measured label widths, keyed by text.
    text_widths: HashMap<String, f32>,
}

impl Painter {
    pub fn new(cfg: &Config) -> Self {
        let font_size = cfg.appearance.font_size.max(6.0);
        let line_height = (font_size * 1.35).ceil();
        Self {
            // Loading the font database is the slowest part of startup; this
            // happens after the window exists and has the keyboard grab.
            font_system: FontSystem::new(),
            swash: SwashCache::new(),
            icons: IconCache::new(&cfg.appearance, &cfg.behavior.fallback_icon),
            font_family: cfg.appearance.font.clone(),
            font_size,
            line_height,
            text_widths: HashMap::new(),
        }
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn icon_theme(&self) -> &str {
        self.icons.theme_name()
    }

    /// Width of `text` in pixels, cached.
    pub fn text_width(&mut self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        if let Some(width) = self.text_widths.get(text) {
            return *width;
        }
        let family = self.font_family.clone();
        let attrs = Attrs::new().family(Family::Name(&family));
        let metrics = Metrics::new(self.font_size, self.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(None, None);
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        // `layout_runs` does not shape on its own; `draw` does it internally.
        buffer.shape_until_scroll(&mut self.font_system, false);
        let width = buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0f32, f32::max);
        self.text_widths.insert(text.to_string(), width);
        width
    }

    /// Draw the whole UI for `app` into `target`.
    pub fn paint(&mut self, target: &mut Pixmap, app: &App, cfg: &Config) {
        let height = target.height() as f32;
        let appearance = &cfg.appearance;

        target.fill(appearance.bg.to_tiny_skia());

        let layout = &app.layout;
        if !appearance.prompt.is_empty() {
            self.draw_text(
                target,
                &appearance.prompt,
                layout.prompt_x,
                height,
                appearance.prompt_fg,
            );
        }

        let (visible_start, visible_end) =
            self.input_view(&app.query, app.cursor, layout.input_width - 6.0);
        let shown = &app.query[visible_start..visible_end];
        if shown.is_empty() && app.query.is_empty() {
            if let Some(placeholder) = &appearance.placeholder {
                self.draw_text(
                    target,
                    placeholder,
                    layout.input_x,
                    height,
                    appearance.placeholder_fg,
                );
            }
        } else if !shown.is_empty() {
            self.draw_text(target, shown, layout.input_x, height, appearance.input_fg);
        }

        // Text cursor.
        let cursor_offset = self
            .text_width(&app.query[visible_start..app.cursor])
            .min((layout.input_width - 2.0).max(0.0));
        let cursor_height = (self.font_size * 1.05).min(height);
        fill_rect(
            target,
            layout.input_x + cursor_offset,
            (height - cursor_height) / 2.0,
            1.5,
            cursor_height,
            appearance.cursor,
        );

        for cell in &layout.cells {
            let summary = &app.filtered[cell.index];
            let item = &app.items[summary.index];
            let selected = cell.index == app.sel;

            if selected {
                fill_rounded(
                    target,
                    cell.x,
                    appearance.padding,
                    cell.width,
                    layout.content_height,
                    appearance.sel_radius,
                    appearance.sel_bg,
                );
            }
            let foreground = if selected {
                appearance.sel_fg
            } else {
                appearance.fg
            };

            let mut label_x = cell.x + appearance.item_padding;
            if appearance.show_icons {
                let icon_size = appearance.icon_size as f32;
                let icon_y = (height - icon_size) / 2.0;
                match self.icons.get(item.icon.as_deref()) {
                    Some(icon) => {
                        let icon_width = icon.width() as f32;
                        let icon_height = icon.height() as f32;
                        let x = label_x + (icon_size - icon_width) / 2.0;
                        let y = icon_y + (icon_size - icon_height) / 2.0;
                        let paint = PixmapPaint {
                            quality: FilterQuality::Nearest,
                            ..Default::default()
                        };
                        target.draw_pixmap(
                            x.round() as i32,
                            y.round() as i32,
                            icon.as_ref().as_ref(),
                            &paint,
                            Transform::identity(),
                            None,
                        );
                    }
                    None => {
                        self.draw_icon_placeholder(
                            target,
                            label_x,
                            icon_y,
                            icon_size,
                            &item.label,
                            appearance,
                        );
                    }
                }
                label_x += icon_size + appearance.gap;
            }

            self.draw_text(target, &item.label, label_x, height, foreground);
        }

        if layout.prev_more {
            self.draw_scroll_hint(target, appearance, "<", true);
        }
        if layout.next_more {
            self.draw_scroll_hint(target, appearance, ">", false);
        }
    }

    fn draw_scroll_hint(
        &mut self,
        target: &mut Pixmap,
        appearance: &Appearance,
        glyph: &str,
        left: bool,
    ) {
        let width = target.width() as f32;
        let height = target.height() as f32;
        let text_width = self.text_width(glyph);
        let x = if left {
            appearance.padding + 2.0
        } else {
            width - appearance.padding - text_width - 2.0
        };
        // Mask whatever is underneath the hint.
        fill_rect(
            target,
            x - 3.0,
            0.0,
            text_width + 6.0,
            height,
            appearance.bg,
        );
        self.draw_text(target, glyph, x, height, appearance.scroll_fg);
    }

    fn draw_icon_placeholder(
        &mut self,
        target: &mut Pixmap,
        x: f32,
        y: f32,
        size: f32,
        label: &str,
        appearance: &Appearance,
    ) {
        fill_rounded(
            target,
            x,
            y,
            size,
            size,
            size * 0.22,
            appearance.placeholder_fg.with_alpha(48),
        );
        let Some(first) = label.chars().next() else {
            return;
        };
        let letter = first.to_uppercase().to_string();
        let letter_width = self.text_width(&letter);
        self.draw_text(
            target,
            &letter,
            x + (size - letter_width) / 2.0,
            size + y,
            appearance.placeholder_fg,
        );
    }

    /// Vertical centering: `box_height` is the full strip height.
    fn draw_text(
        &mut self,
        target: &mut Pixmap,
        text: &str,
        x: f32,
        box_height: f32,
        color: Color,
    ) {
        if text.is_empty() {
            return;
        }
        let family = self.font_family.clone();
        let attrs = Attrs::new().family(Family::Name(&family));
        let metrics = Metrics::new(self.font_size, self.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(None, None);
        buffer.set_text(text, &attrs, Shaping::Advanced, None);

        let y = ((box_height - self.line_height) / 2.0).round();
        self.blit_buffer(target, &mut buffer, x.round() as i32, y as i32, color);
    }

    fn blit_buffer(
        &mut self,
        target: &mut Pixmap,
        buffer: &mut Buffer,
        origin_x: i32,
        origin_y: i32,
        color: Color,
    ) {
        let width = target.width() as i32;
        let height = target.height() as i32;
        let data = target.data_mut();
        let Painter {
            font_system, swash, ..
        } = self;
        let base = cosmic_text::Color::rgba(color.r, color.g, color.b, color.a);
        buffer.draw(font_system, swash, base, |x, y, w, h, color| {
            blend_rect(
                data,
                width,
                height,
                origin_x + x,
                origin_y + y,
                w as i32,
                h as i32,
                color,
            );
        });
    }

    /// Slice of the query that fits the input field, keeping the cursor visible.
    fn input_view(&mut self, query: &str, cursor: usize, max_width: f32) -> (usize, usize) {
        let available = max_width.max(1.0);
        let mut start = 0;
        while start < cursor && self.text_width(&query[start..cursor]) > available {
            start = next_boundary(query, start);
        }
        let mut end = query.len();
        while end > start && self.text_width(&query[start..end]) > available {
            end = prev_boundary(query, end);
        }
        if end < cursor {
            end = cursor;
        }
        (start, end)
    }
}

/// Convert a premultiplied RGBA pixmap into X11 `ZPixmap` bytes (native u32
/// `0x00RRGGBB`, which on little-endian memory is B, G, R, X).
pub fn pixmap_to_x11(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        let value =
            ((color.red() as u32) << 16) | ((color.green() as u32) << 8) | color.blue() as u32;
        out.extend_from_slice(&value.to_ne_bytes());
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn blend_rect(
    data: &mut [u8],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: cosmic_text::Color,
) {
    let (r, g, b, a) = color.as_rgba_tuple();
    if a == 0 {
        return;
    }
    let inverse = 255 - a as u32;
    for py in y..(y + h) {
        if py < 0 || py >= height {
            continue;
        }
        for px in x..(x + w) {
            if px < 0 || px >= width {
                continue;
            }
            let index = ((py as usize * width as usize) + px as usize) * 4;
            let alpha = a as u32;
            data[index] = ((r as u32 * alpha + data[index] as u32 * inverse) / 255) as u8;
            data[index + 1] = ((g as u32 * alpha + data[index + 1] as u32 * inverse) / 255) as u8;
            data[index + 2] = ((b as u32 * alpha + data[index + 2] as u32 * inverse) / 255) as u8;
            data[index + 3] = 255;
        }
    }
}

fn fill_rect(target: &mut Pixmap, x: f32, y: f32, width: f32, height: f32, color: Color) {
    let Some(rect) = Rect::from_xywh(x, y, width, height) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia());
    paint.anti_alias = false;
    target.fill_rect(rect, &paint, Transform::identity(), None);
}

fn fill_rounded(
    target: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Color,
) {
    let Some(path) = rounded_rect(x, y, width, height, radius) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia());
    paint.anti_alias = true;
    target.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<tiny_skia::Path> {
    let rect = Rect::from_xywh(x, y, width, height)?;
    let radius = radius.max(0.0).min(width / 2.0).min(height / 2.0);
    if radius <= 0.5 {
        return Some(PathBuilder::from_rect(rect));
    }
    let mut builder = PathBuilder::new();
    builder.move_to(x + radius, y);
    builder.line_to(x + width - radius, y);
    builder.quad_to(x + width, y, x + width, y + radius);
    builder.line_to(x + width, y + height - radius);
    builder.quad_to(x + width, y + height, x + width - radius, y + height);
    builder.line_to(x + radius, y + height);
    builder.quad_to(x, y + height, x, y + height - radius);
    builder.line_to(x, y + radius);
    builder.quad_to(x, y, x + radius, y);
    builder.close();
    builder.finish()
}

fn prev_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| offset + index)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x11_pixels_are_bgrx() {
        let mut pixmap = Pixmap::new(2, 1).unwrap();
        pixmap.fill(Color::rgb(0x1e, 0x1e, 0x2e).to_tiny_skia());
        let bytes = pixmap_to_x11(&pixmap);
        // X11 depth-24 ZPixmap: native u32 0x00RRGGBB.
        assert_eq!(&bytes[..4], &[0x2e, 0x1e, 0x1e, 0x00]);
    }
}
