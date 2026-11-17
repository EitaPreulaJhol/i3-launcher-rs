//! Geometry of the strip: prompt, input field and the horizontal item viewport.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// Index into `App::filtered`.
    pub index: usize,
    pub x: f32,
    pub width: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub prompt_x: f32,
    pub prompt_width: f32,
    pub input_x: f32,
    pub input_width: f32,
    pub list_x: f32,
    pub list_width: f32,
    /// Vertical padding, clamped so the text line fits in short strips.
    pub padding_y: f32,
    /// Height available for the selection pill / icons.
    pub content_height: f32,
    pub cells: Vec<Cell>,
    pub prev_more: bool,
    pub next_more: bool,
    /// Index into `filtered` of the first visible cell.
    pub offset: usize,
}

impl Layout {
    /// Vertical padding clamped so at least one text line fits.
    pub fn vertical_padding(requested: f32, height: f32, line_height: f32) -> f32 {
        requested
            .max(0.0)
            .min(((height - line_height) / 2.0).max(0.0))
    }

    /// Icon size clamped to the available content height.
    pub fn icon_size(requested: f32, content_height: f32) -> f32 {
        requested.max(1.0).min(content_height.max(1.0))
    }

    /// Cell width for an item with the given label width.
    pub fn cell_width(
        label_width: f32,
        show_icons: bool,
        icon_size: f32,
        gap: f32,
        item_padding: f32,
    ) -> f32 {
        let icon = if show_icons { icon_size + gap } else { 0.0 };
        2.0 * item_padding + icon + label_width
    }

    /// Which cell (if any) contains the given x coordinate.
    pub fn cell_at(&self, x: f32) -> Option<usize> {
        self.cells
            .iter()
            .find(|cell| x >= cell.x && x < cell.x + cell.width)
            .map(|cell| cell.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_padding_adapts_to_short_strips() {
        // Tall strip: the requested padding is kept.
        assert_eq!(Layout::vertical_padding(8.0, 44.0, 19.0), 8.0);
        // 20px strip: only the leftover space around the line is used.
        let padding = Layout::vertical_padding(8.0, 20.0, 18.9);
        assert!((padding - 0.55).abs() < 0.01);
        // Strip shorter than the line: no padding.
        assert_eq!(Layout::vertical_padding(8.0, 10.0, 19.0), 0.0);
    }

    #[test]
    fn icon_size_is_clamped_to_content_height() {
        assert_eq!(Layout::icon_size(24.0, 30.0), 24.0);
        assert_eq!(Layout::icon_size(24.0, 18.9), 18.9);
        assert_eq!(Layout::icon_size(24.0, 0.0), 1.0);
    }

    #[test]
    fn cell_width_accounts_for_icon_and_padding() {
        assert_eq!(Layout::cell_width(50.0, true, 24.0, 8.0, 8.0), 98.0);
        assert_eq!(Layout::cell_width(50.0, false, 24.0, 8.0, 8.0), 66.0);
    }

    #[test]
    fn cell_at_respects_half_open_intervals() {
        let layout = Layout {
            cells: vec![
                Cell {
                    index: 3,
                    x: 10.0,
                    width: 20.0,
                },
                Cell {
                    index: 4,
                    x: 30.0,
                    width: 20.0,
                },
            ],
            ..Default::default()
        };
        assert_eq!(layout.cell_at(10.0), Some(3));
        assert_eq!(layout.cell_at(29.9), Some(3));
        assert_eq!(layout.cell_at(30.0), Some(4));
        assert_eq!(layout.cell_at(50.0), None);
    }
}
