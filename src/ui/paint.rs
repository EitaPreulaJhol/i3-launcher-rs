//! Software painting of the strip into a `tiny-skia` pixmap.

use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};
use tiny_skia::{
    FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform,
};

use crate::config::{Color, Config, Style};
use crate::icons::IconCache;
use crate::ui::layout::Layout;
use crate::ui::state::App;

pub struct Painter {
    font_system: FontSystem,
    swash: SwashCache,
    icons: IconCache,
    font_family: String,
    font_size: f32,
    font_weight: u16,
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
            font_weight: cfg.appearance.font_weight.clamp(100, 1000),
            line_height,
            text_widths: HashMap::new(),
        }
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
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
        let attrs = self.attrs(&family);
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

    fn attrs<'a>(&self, family: &'a str) -> Attrs<'a> {
        Attrs::new()
            .family(parse_family(family))
            .weight(Weight(self.font_weight))
    }

    /// Draw the whole UI for `app` into `target`.
    pub fn paint(&mut self, target: &mut Pixmap, app: &App, cfg: &Config) {
        let width = target.width() as f32;
        let height = target.height() as f32;
        let appearance = &cfg.appearance;
        let style = &cfg.style;
        let layout = &app.layout;

        target.fill(style.bg.to_tiny_skia());

        let prompt = app.prompt(&appearance.prompt);
        if !prompt.is_empty() {
            self.draw_text(target, prompt, layout.prompt_x, height, style.prompt_fg);
        }

        // Optional rounded box behind the input field.
        if style.input_bg.is_some() || style.input_border_width > 0.0 {
            let padding = style.padding.max(0.0);
            let x = (layout.input_x - style.gap * 0.5).max(padding);
            let y = layout.padding_y;
            let box_width = (layout.list_x - style.gap * 0.5 - x).max(0.0);
            let box_height = layout.content_height;
            if let Some(background) = style.input_bg {
                fill_rounded(
                    target,
                    x,
                    y,
                    box_width,
                    box_height,
                    style.input_radius,
                    background,
                );
            }
            if style.input_border_width > 0.0 {
                stroke_rounded(
                    target,
                    x,
                    y,
                    box_width,
                    box_height,
                    style.input_radius,
                    style.input_border_width,
                    style.input_border,
                );
            }
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
                    style.placeholder_fg,
                );
            }
        } else if !shown.is_empty() {
            self.draw_text(target, shown, layout.input_x, height, style.input_fg);
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
            style.cursor_width.max(0.5),
            cursor_height,
            style.cursor,
        );

        for cell in &layout.cells {
            let summary = &app.filtered[cell.index];
            let item = &app.items[summary.index];
            let selected = cell.index == app.sel;

            if selected {
                fill_rounded(
                    target,
                    cell.x,
                    layout.padding_y,
                    cell.width,
                    layout.content_height,
                    style.selection_radius,
                    style.selection_bg,
                );
                if style.selection_border_width > 0.0 {
                    stroke_rounded(
                        target,
                        cell.x,
                        layout.padding_y,
                        cell.width,
                        layout.content_height,
                        style.selection_radius,
                        style.selection_border_width,
                        style.selection_border,
                    );
                }
            }
            let foreground = if selected {
                style.selection_fg
            } else {
                style.fg
            };

            let mut label_x = cell.x + style.item_padding;
            if appearance.show_icons && app.shows_icons() {
                let icon_size =
                    Layout::icon_size(appearance.icon_size as f32, layout.content_height);
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
                            style,
                        );
                    }
                }
                label_x += icon_size + style.gap;
            }

            let highlight_color = style.highlight_fg.unwrap_or(foreground);
            self.draw_text_highlighted(
                target,
                &item.label,
                &summary.positions,
                label_x,
                height,
                foreground,
                highlight_color,
            );
        }

        if layout.prev_more {
            self.draw_scroll_hint(target, style, layout.list_x, layout.left_gutter, "<");
        }
        if layout.next_more {
            self.draw_scroll_hint(
                target,
                style,
                layout.cells_right(),
                layout.right_gutter,
                ">",
            );
        }

        if style.panel_border_width > 0.0 {
            stroke_rounded(
                target,
                0.0,
                0.0,
                width,
                height,
                0.0,
                style.panel_border_width,
                style.panel_border,
            );
        }
    }

    fn draw_scroll_hint(
        &mut self,
        target: &mut Pixmap,
        style: &Style,
        gutter_x: f32,
        gutter_width: f32,
        glyph: &str,
    ) {
        let height = target.height() as f32;
        let text_width = self.text_width(glyph);
        let x = gutter_x + (gutter_width - text_width) / 2.0;
        self.draw_text(target, glyph, x, height, style.scroll_fg);
    }

    fn draw_icon_placeholder(
        &mut self,
        target: &mut Pixmap,
        x: f32,
        y: f32,
        size: f32,
        label: &str,
        style: &Style,
    ) {
        fill_rounded(
            target,
            x,
            y,
            size,
            size,
            style.icon_radius,
            style.placeholder_fg.with_alpha(48),
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
            style.placeholder_fg,
        );
    }

    fn draw_text(
        &mut self,
        target: &mut Pixmap,
        text: &str,
        x: f32,
        box_height: f32,
        color: Color,
    ) {
        self.draw_text_highlighted(target, text, &[], x, box_height, color, color);
    }

    /// Draw text, coloring the characters whose indices appear in `positions`.
    #[allow(clippy::too_many_arguments)]
    fn draw_text_highlighted(
        &mut self,
        target: &mut Pixmap,
        text: &str,
        positions: &[u32],
        x: f32,
        box_height: f32,
        normal_color: Color,
        highlight_color: Color,
    ) {
        if text.is_empty() {
            return;
        }
        let family = self.font_family.clone();
        let base = self.attrs(&family);
        let normal = base.clone().color(to_cosmic(normal_color));
        let highlight = base.clone().color(to_cosmic(highlight_color));
        let spans = build_spans(text, positions, &normal, &highlight);

        let metrics = Metrics::new(self.font_size, self.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(None, None);
        buffer.set_rich_text(spans, &base, Shaping::Advanced, None);

        let y = ((box_height - self.line_height) / 2.0).round();
        self.blit_buffer(
            target,
            &mut buffer,
            x.round() as i32,
            y as i32,
            normal_color,
        );
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
        let base = to_cosmic(color);
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

/// Split `text` into contiguous spans, alternating normal/highlight attributes
/// at the given character indices.
fn build_spans<'a>(
    text: &'a str,
    positions: &[u32],
    normal: &Attrs<'a>,
    highlight: &Attrs<'a>,
) -> Vec<(&'a str, Attrs<'a>)> {
    if text.is_empty() {
        return Vec::new();
    }
    if positions.is_empty() {
        return vec![(text, normal.clone())];
    }

    let mut spans = Vec::new();
    let mut span_start = 0usize;
    let mut current = false;

    for (char_index, (byte_index, _)) in text.char_indices().enumerate() {
        let is_match = positions.binary_search(&(char_index as u32)).is_ok();
        if is_match != current {
            if byte_index > span_start {
                spans.push((
                    &text[span_start..byte_index],
                    if current {
                        highlight.clone()
                    } else {
                        normal.clone()
                    },
                ));
            }
            span_start = byte_index;
            current = is_match;
        }
    }

    if span_start < text.len() {
        spans.push((
            &text[span_start..],
            if current {
                highlight.clone()
            } else {
                normal.clone()
            },
        ));
    }
    if spans.is_empty() {
        spans.push((text, normal.clone()));
    }
    spans
}

fn to_cosmic(color: Color) -> cosmic_text::Color {
    cosmic_text::Color::rgba(color.r, color.g, color.b, color.a)
}

/// Map CSS-ish generic families (as used by rofi/Pango, e.g. "Sans 9") to
/// cosmic-text's generic families.
fn parse_family(name: &str) -> Family<'_> {
    match name.trim().to_ascii_lowercase().as_str() {
        "sans" | "sans-serif" | "sans serif" => Family::SansSerif,
        "serif" => Family::Serif,
        "monospace" | "mono" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        _ => Family::Name(name),
    }
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

#[allow(clippy::too_many_arguments)]
fn stroke_rounded(
    target: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    stroke_width: f32,
    color: Color,
) {
    let stroke_width = stroke_width.max(0.0);
    let inset = stroke_width * 0.5;
    if width <= stroke_width || height <= stroke_width {
        return;
    }
    let Some(path) = rounded_rect(
        x + inset,
        y + inset,
        width - stroke_width,
        height - stroke_width,
        (radius - inset).max(0.0),
    ) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia());
    paint.anti_alias = true;
    let stroke = Stroke {
        width: stroke_width,
        ..Default::default()
    };
    target.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
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

    #[test]
    fn highlight_spans_toggle_at_positions() {
        let normal = Attrs::new();
        let highlight = Attrs::new().color(cosmic_text::Color::rgb(0xff, 0x00, 0x00));

        let spans = build_spans("Firefox", &[0, 1, 2], &normal, &highlight);
        let joined: String = spans.iter().map(|(text, _)| *text).collect();
        assert_eq!(joined, "Firefox");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "Fir");
        assert_eq!(spans[1].0, "efox");
        assert_eq!(
            spans[0].1.color_opt,
            Some(cosmic_text::Color::rgb(0xff, 0x00, 0x00))
        );
        assert_eq!(spans[1].1.color_opt, None);
    }

    #[test]
    fn highlight_spans_ignore_out_of_range_positions() {
        let normal = Attrs::new();
        let highlight = Attrs::new();
        let spans = build_spans("ab", &[1, 5], &normal, &highlight);
        assert_eq!(spans.len(), 2);
        let joined: String = spans.iter().map(|(text, _)| *text).collect();
        assert_eq!(joined, "ab");
    }

    #[test]
    fn generic_families_are_mapped() {
        assert!(matches!(parse_family("Sans"), Family::SansSerif));
        assert!(matches!(parse_family("sans-serif"), Family::SansSerif));
        assert!(matches!(parse_family("Monospace"), Family::Monospace));
        assert!(matches!(
            parse_family("DejaVu Sans"),
            Family::Name(name) if name == "DejaVu Sans"
        ));
    }

    #[test]
    fn empty_text_has_no_spans() {
        assert!(build_spans("", &[0], &Attrs::new(), &Attrs::new()).is_empty());
    }
}
