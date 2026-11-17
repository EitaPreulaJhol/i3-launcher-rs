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
    /// Height available for the selection pill / icons.
    pub content_height: f32,
    pub cells: Vec<Cell>,
    pub prev_more: bool,
    pub next_more: bool,
    /// Index into `filtered` of the first visible cell.
    pub offset: usize,
}

impl Layout {
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
