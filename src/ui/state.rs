//! Interactive state: query editing, filtering, selection and viewport scroll.

use xkeysym::Keysym;

use crate::app::{Action, Item};
use crate::config::{Behavior, Config};
use crate::matching::{self, Match, MatchOptions};
use crate::ui::layout::{Cell, Layout};
use crate::ui::paint::Painter;
use crate::x11::keys::Mods;

/// Active item source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Applications from `.desktop` files (with icons).
    Apps,
    /// Every executable in `$PATH` (no icons).
    Paths,
}

/// Prompt shown while the PATH mode is active.
const PATH_PROMPT: &str = "PATH: ";

/// What the launcher should do after handling an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    LaunchApp(usize),
    Shell(String),
    Command(String),
    Print(String),
    Quit(i32),
}

pub struct App {
    pub items: Vec<Item>,
    /// Items of the other mode, ready for `Tab`.
    alternate: Vec<Item>,
    /// Lazily fills `alternate` the first time Tab is pressed.
    alternate_loader: Option<Box<dyn Fn() -> Vec<Item>>>,
    pub mode: Mode,
    tabs_enabled: bool,
    pub filtered: Vec<Match>,
    pub query: String,
    /// Byte offset of the text cursor inside `query`.
    pub cursor: usize,
    /// Index into `filtered`.
    pub sel: usize,
    pub layout: Layout,
    pub outcome: Option<Outcome>,
    offset: usize,
}

impl App {
    /// Single-list launcher (dmenu mode or tests): Tab does nothing.
    pub fn new(items: Vec<Item>, behavior: &Behavior) -> Self {
        Self::build(items, Vec::new(), false, behavior)
    }

    /// Launcher with both modes and an eagerly built `$PATH` list.
    pub fn with_path_mode(apps: Vec<Item>, paths: Vec<Item>, behavior: &Behavior) -> Self {
        let tabs_enabled = !paths.is_empty();
        Self::build(apps, paths, tabs_enabled, behavior)
    }

    /// Like [`App::with_path_mode`], but `$PATH` is scanned on the first Tab.
    pub fn with_path_loader<F>(apps: Vec<Item>, loader: F, behavior: &Behavior) -> Self
    where
        F: Fn() -> Vec<Item> + 'static,
    {
        let mut app = Self::build(apps, Vec::new(), true, behavior);
        app.alternate_loader = Some(Box::new(loader));
        app
    }

    fn build(
        items: Vec<Item>,
        alternate: Vec<Item>,
        tabs_enabled: bool,
        behavior: &Behavior,
    ) -> Self {
        let filtered = matching::filter(&items, "", &match_options(behavior));
        Self {
            items,
            alternate,
            alternate_loader: None,
            mode: Mode::Apps,
            tabs_enabled,
            filtered,
            query: String::new(),
            cursor: 0,
            sel: 0,
            layout: Layout::default(),
            outcome: None,
            offset: 0,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Whether icons are drawn for the current mode (PATH mode has none).
    pub fn shows_icons(&self) -> bool {
        self.mode == Mode::Apps
    }

    /// Prompt for the current mode. PATH mode gets its own label.
    pub fn prompt<'a>(&self, configured: &'a str) -> &'a str {
        match self.mode {
            Mode::Apps => configured,
            Mode::Paths => PATH_PROMPT,
        }
    }

    pub fn selected_item(&self) -> Option<&Item> {
        self.filtered.get(self.sel).map(|m| &self.items[m.index])
    }

    /// Recompute the horizontal viewport for the current selection.
    pub fn relayout(&mut self, painter: &mut Painter, cfg: &Config, width: f32, height: f32) {
        let appearance = &cfg.appearance;
        let style = &cfg.style;
        let padding = style.padding.max(0.0);
        let gap = style.gap.max(0.0);
        let prompt = self.prompt(&appearance.prompt);
        let prompt_width = if prompt.is_empty() {
            0.0
        } else {
            painter.text_width(prompt)
        };
        let prompt_gap = if prompt_width > 0.0 { gap } else { 0.0 };

        let available = (width - 2.0 * padding).max(1.0);
        let max_input = (available - prompt_width - prompt_gap - gap - 48.0).max(60.0);
        let requested_input = style.search_width.resolve(width as u32) as f32;
        let input_width = requested_input.clamp(60.0, max_input);

        let mut list_x = padding + prompt_width + prompt_gap + input_width + gap;
        if list_x > width - 48.0 {
            list_x = (width - 48.0).max(padding);
        }
        let list_width = (width - padding - list_x).max(0.0);

        let line_height = painter.line_height();
        let padding_y = Layout::vertical_padding(style.padding, height, line_height);
        let content_height = (height - 2.0 * padding_y).max(0.0);
        let icon_size = Layout::icon_size(appearance.icon_size as f32, content_height);
        let show_icons = appearance.show_icons && self.shows_icons();
        let base = Layout::cell_width(0.0, show_icons, icon_size, gap, style.item_padding);
        let item_gap = style.item_gap.max(0.0);

        let len = self.filtered.len();
        let mut offset = if len == 0 {
            0
        } else {
            self.offset.min(len - 1)
        };
        if len > 0 && self.sel < offset {
            offset = self.sel;
        }

        // Reserve space for the `<`/`>` hints so they never cover an item.
        let gutter = if len > 0 {
            (painter.text_width("<") + 8.0).max(14.0)
        } else {
            0.0
        };

        let mut cells = Vec::new();
        let mut left_gutter = 0.0;
        let mut right_gutter = 0.0;
        if len > 0 && list_width > 1.0 && self.sel < len {
            // Two or three passes: one to know whether the `>` hint is needed,
            // and extra ones when the viewport offset changes.
            for _ in 0..3 {
                let wanted_left = if offset > 0 { gutter } else { 0.0 };
                left_gutter = wanted_left;
                let available = (list_width - left_gutter - right_gutter).max(0.0);
                let (new_offset, new_cells) = layout_viewport(
                    &self.items,
                    &self.filtered,
                    painter,
                    self.sel,
                    offset,
                    list_x + left_gutter,
                    available,
                    base,
                    item_gap,
                );
                offset = new_offset;
                cells = new_cells;

                let more = cells
                    .last()
                    .map(|cell| cell.index + 1 < len)
                    .unwrap_or(false);
                let wanted_right = if more { gutter } else { 0.0 };
                let stable = (if offset > 0 { gutter } else { 0.0 } - left_gutter).abs() < 0.5
                    && (wanted_right - right_gutter).abs() < 0.5;
                right_gutter = wanted_right;
                if stable {
                    break;
                }
            }
            left_gutter = if offset > 0 { gutter } else { 0.0 };
        }

        let next_more = cells
            .last()
            .map(|cell| cell.index + 1 < len)
            .unwrap_or(false);

        if std::env::var_os("I3L_DEBUG_LAYOUT").is_some() {
            let preview: Vec<String> = self
                .filtered
                .iter()
                .take(4)
                .map(|m| format!("{} ({})", self.items[m.index].label, m.score))
                .collect();
            eprintln!(
                "query={:?} len={} sel={} offset={} cells={} [{}]",
                self.query,
                len,
                self.sel,
                offset,
                cells.len(),
                preview.join(", ")
            );
        }
        self.offset = offset;
        self.layout = Layout {
            prompt_x: padding,
            prompt_width,
            input_x: padding + prompt_width + prompt_gap,
            input_width,
            list_x,
            list_width,
            left_gutter,
            right_gutter,
            padding_y,
            content_height,
            cells,
            prev_more: offset > 0,
            next_more,
            offset,
        };
    }

    /// Handle a key press; returns `true` when a redraw is needed.
    pub fn handle_key(&mut self, sym: Keysym, mods: Mods, behavior: &Behavior) -> bool {
        if sym.is_modifier_key() {
            return false;
        }

        if mods.ctrl {
            // Lateral arrows navigate the list; Ctrl+arrows move the text
            // cursor by word (plain arrows are reserved for item navigation).
            match sym {
                Keysym::Left | Keysym::KP_Left => {
                    self.move_cursor_word(-1);
                    return true;
                }
                Keysym::Right | Keysym::KP_Right => {
                    self.move_cursor_word(1);
                    return true;
                }
                _ => {}
            }
            if let Some(c) = sym.key_char().map(|c| c.to_ascii_lowercase()) {
                match c {
                    'a' => {
                        self.cursor = 0;
                        return true;
                    }
                    'e' => {
                        self.cursor = self.query.len();
                        return true;
                    }
                    'b' => {
                        self.move_cursor(-1);
                        return true;
                    }
                    'f' => {
                        self.move_cursor(1);
                        return true;
                    }
                    'h' => return self.backspace(behavior),
                    'd' => return self.delete_forward(behavior),
                    'u' => {
                        let cursor = self.cursor;
                        return self.delete_range(0, cursor, behavior);
                    }
                    'k' => {
                        let cursor = self.cursor;
                        let end = self.query.len();
                        return self.delete_range(cursor, end, behavior);
                    }
                    'w' => return self.delete_word(behavior),
                    'n' | 'j' => return self.move_selection(1),
                    'p' => return self.move_selection(-1),
                    _ => {}
                }
            }
        }

        if mods.alt {
            if let Some(c) = sym.key_char().map(|c| c.to_ascii_lowercase()) {
                match c {
                    'b' => {
                        self.move_cursor_word(-1);
                        return true;
                    }
                    'f' => {
                        self.move_cursor_word(1);
                        return true;
                    }
                    _ => {}
                }
            }
        }

        match sym {
            Keysym::Escape => {
                self.outcome = Some(Outcome::Quit(1));
                true
            }
            Keysym::Return | Keysym::KP_Enter => {
                self.activate();
                true
            }
            Keysym::BackSpace => self.backspace(behavior),
            Keysym::Delete | Keysym::KP_Delete => self.delete_forward(behavior),
            Keysym::Left | Keysym::KP_Left => self.move_selection(-1),
            Keysym::Right | Keysym::KP_Right => self.move_selection(1),
            Keysym::Home | Keysym::KP_Home => {
                // Like dmenu: once the cursor is at the start, jump to the
                // first item.
                if self.cursor == 0 && self.sel != 0 {
                    self.sel = 0;
                } else {
                    self.cursor = 0;
                }
                true
            }
            Keysym::End | Keysym::KP_End => {
                // Like dmenu: once the cursor is at the end, jump to the
                // last item.
                if self.cursor == self.query.len() && !self.filtered.is_empty() {
                    self.sel = self.filtered.len() - 1;
                } else {
                    self.cursor = self.query.len();
                }
                true
            }
            Keysym::Up | Keysym::KP_Up => self.move_selection(-1),
            Keysym::Down | Keysym::KP_Down => self.move_selection(1),
            Keysym::Prior | Keysym::KP_Prior => self.move_selection(-10),
            Keysym::Next | Keysym::KP_Next => self.move_selection(10),
            Keysym::Tab | Keysym::KP_Tab => self.toggle_mode(behavior),
            _ => {
                if !mods.ctrl && !mods.alt && !mods.logo {
                    if let Some(c) = sym.key_char() {
                        if !c.is_control() {
                            return self.insert_char(c, behavior);
                        }
                    }
                }
                false
            }
        }
    }

    pub fn handle_click(&mut self, x: f32, button: u8) -> bool {
        if button != 1 {
            return false;
        }
        let Some(index) = self.layout.cell_at(x) else {
            return false;
        };
        if index == self.sel {
            self.activate();
        } else {
            self.sel = index;
        }
        true
    }

    pub fn handle_scroll(&mut self, up: bool) -> bool {
        self.move_selection(if up { -1 } else { 1 })
    }

    /// Resolve the current selection into an [`Outcome`].
    pub fn activate(&mut self) {
        let action = self
            .selected_item()
            .map(|item| item.action.clone())
            .or_else(|| {
                let text = self.query.trim();
                (!text.is_empty()).then(|| Action::Print(self.query.clone()))
            });
        self.outcome = action.map(|action| match action {
            Action::LaunchApp(index) => Outcome::LaunchApp(index),
            Action::Shell(line) => Outcome::Shell(line),
            Action::Command(name) => Outcome::Command(name),
            Action::Print(text) => Outcome::Print(text),
        });
    }

    fn toggle_mode(&mut self, behavior: &Behavior) -> bool {
        if !self.tabs_enabled {
            return false;
        }
        if self.alternate.is_empty() {
            if let Some(loader) = self.alternate_loader.take() {
                self.alternate = loader();
            }
            if self.alternate.is_empty() {
                // No executables in PATH: stop offering the switch.
                self.tabs_enabled = false;
                return false;
            }
        }
        std::mem::swap(&mut self.items, &mut self.alternate);
        self.mode = match self.mode {
            Mode::Apps => Mode::Paths,
            Mode::Paths => Mode::Apps,
        };
        self.sel = 0;
        self.offset = 0;
        self.refilter(behavior);
        true
    }

    fn refilter(&mut self, behavior: &Behavior) {
        self.filtered = matching::filter(&self.items, &self.query, &match_options(behavior));
        if self.filtered.is_empty() {
            self.sel = 0;
            self.offset = 0;
        } else {
            self.sel = self.sel.min(self.filtered.len() - 1);
            self.offset = self.offset.min(self.sel);
        }
    }

    fn insert_char(&mut self, c: char, behavior: &Behavior) -> bool {
        self.query.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.sel = 0;
        self.offset = 0;
        self.refilter(behavior);
        true
    }

    fn backspace(&mut self, behavior: &Behavior) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = prev_boundary(&self.query, self.cursor);
        self.delete_range(start, self.cursor, behavior)
    }

    fn delete_forward(&mut self, behavior: &Behavior) -> bool {
        let end = next_boundary(&self.query, self.cursor);
        if end == self.cursor {
            return false;
        }
        self.delete_range(self.cursor, end, behavior)
    }

    fn delete_range(&mut self, start: usize, end: usize, behavior: &Behavior) -> bool {
        if start >= end || end > self.query.len() {
            return false;
        }
        self.query.replace_range(start..end, "");
        if self.cursor >= end {
            self.cursor -= end - start;
        } else if self.cursor > start {
            self.cursor = start;
        }
        self.sel = 0;
        self.offset = 0;
        self.refilter(behavior);
        true
    }

    fn delete_word(&mut self, behavior: &Behavior) -> bool {
        let end = self.cursor;
        self.move_cursor_word(-1);
        let start = self.cursor;
        self.delete_range(start, end, behavior)
    }

    fn move_cursor(&mut self, direction: i32) {
        self.cursor = if direction < 0 {
            prev_boundary(&self.query, self.cursor)
        } else {
            next_boundary(&self.query, self.cursor)
        };
    }

    fn move_cursor_word(&mut self, direction: i32) {
        let chars: Vec<(usize, char)> = self.query.char_indices().collect();
        let mut index = chars
            .iter()
            .position(|(byte, _)| *byte >= self.cursor)
            .unwrap_or(chars.len());

        if direction < 0 {
            while index > 0 && !is_word(chars[index - 1].1) {
                index -= 1;
            }
            while index > 0 && is_word(chars[index - 1].1) {
                index -= 1;
            }
        } else {
            while index < chars.len() && is_word(chars[index].1) {
                index += 1;
            }
            while index < chars.len() && !is_word(chars[index].1) {
                index += 1;
            }
        }

        self.cursor = chars
            .get(index)
            .map(|(byte, _)| *byte)
            .unwrap_or(self.query.len());
    }

    fn move_selection(&mut self, delta: isize) -> bool {
        if self.filtered.is_empty() {
            return false;
        }
        let last = self.filtered.len() as isize - 1;
        let next = (self.sel as isize + delta).clamp(0, last) as usize;
        if next == self.sel {
            return false;
        }
        self.sel = next;
        true
    }
}

fn match_options(behavior: &Behavior) -> MatchOptions {
    MatchOptions {
        mode: behavior.match_mode,
        case_sensitive: behavior.case_sensitive,
        sort: behavior.sort,
        max_results: behavior.max_results,
    }
}

/// Lay out the visible cells and make sure the selection is inside the view.
#[allow(clippy::too_many_arguments)]
fn layout_viewport(
    items: &[Item],
    filtered: &[Match],
    painter: &mut Painter,
    selected: usize,
    mut offset: usize,
    list_x: f32,
    list_width: f32,
    base: f32,
    item_gap: f32,
) -> (usize, Vec<Cell>) {
    if filtered.is_empty() || list_width <= 1.0 || selected >= filtered.len() {
        return (offset.min(filtered.len().saturating_sub(1)), Vec::new());
    }

    let mut cells = build_cells(
        items, filtered, painter, offset, list_x, list_width, base, item_gap,
    );

    if selected >= offset + cells.len() {
        // Selection fell after the viewport: start the viewport at the
        // selection and fill it backwards with the items that still fit.
        let mut first = selected;
        let mut used = base + painter.text_width(&items[filtered[first].index].label);
        while first > 0 {
            let width =
                base + painter.text_width(&items[filtered[first - 1].index].label) + item_gap;
            if used + width > list_width {
                break;
            }
            used += width;
            first -= 1;
        }
        offset = first;
        cells = build_cells(
            items, filtered, painter, offset, list_x, list_width, base, item_gap,
        );
    }

    (offset, cells)
}

#[allow(clippy::too_many_arguments)]
fn build_cells(
    items: &[Item],
    filtered: &[Match],
    painter: &mut Painter,
    offset: usize,
    list_x: f32,
    list_width: f32,
    base: f32,
    item_gap: f32,
) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut x = list_x;
    for position in offset..filtered.len() {
        let width = base + painter.text_width(&items[filtered[position].index].label);
        if !cells.is_empty() && x + width > list_x + list_width {
            break;
        }
        cells.push(Cell {
            index: position,
            x,
            width,
        });
        x += width + item_gap;
        if x >= list_x + list_width {
            break;
        }
    }
    cells
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

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Item> {
        vec![
            Item::new("Firefox".into(), None, Action::Print("firefox".into())),
            Item::new("Files".into(), None, Action::Print("files".into())),
            Item::new("Terminal".into(), None, Action::Print("terminal".into())),
        ]
    }

    fn app() -> App {
        App::new(items(), &Behavior::default())
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_key(Keysym::from_char(c), Mods::default(), &Behavior::default());
        }
    }

    #[test]
    fn typing_filters_and_tracks_cursor() {
        let mut app = app();
        type_text(&mut app, "fir");
        assert_eq!(app.query, "fir");
        assert_eq!(app.cursor, 3);
        assert_eq!(app.filtered.len(), 1); // fuzzy: f-i-r is a subsequence

        let mut app = self::app();
        type_text(&mut app, "fire");
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.selected_item().unwrap().label, "Firefox");
    }

    #[test]
    fn backspace_and_delete_work_on_utf8() {
        let mut app = self::app();
        type_text(&mut app, "aé");
        assert_eq!(app.query, "aé");
        assert!(app.backspace(&Behavior::default()));
        assert_eq!(app.query, "a");
        assert!(!app.delete_forward(&Behavior::default()));
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut app = self::app();
        assert!(app.move_selection(1));
        assert_eq!(app.sel, 1);
        assert!(app.move_selection(10));
        assert_eq!(app.sel, 2);
        assert!(!app.move_selection(1));
        assert!(app.move_selection(-10));
        assert_eq!(app.sel, 0);
    }

    #[test]
    fn scroll_hints_get_their_own_gutter() {
        let cfg = Config::default();
        let mut painter = Painter::new(&cfg);
        let items: Vec<Item> = (0..30)
            .map(|i| {
                Item::new(
                    format!("Application {i:02}"),
                    None,
                    Action::Print(format!("app{i}")),
                )
            })
            .collect();
        let mut app = App::new(items, &cfg.behavior);

        app.relayout(&mut painter, &cfg, 640.0, 20.0);
        assert!(app.layout.next_more);
        assert_eq!(app.layout.left_gutter, 0.0);
        assert!(app.layout.right_gutter > 0.0);
        assert_cells_inside_viewport(&app);

        // Scrolling right also reserves space for the left hint.
        app.sel = 20;
        app.relayout(&mut painter, &cfg, 640.0, 20.0);
        assert!(app.layout.offset > 0);
        assert!(app.layout.left_gutter > 0.0);
        assert_cells_inside_viewport(&app);
    }

    fn assert_cells_inside_viewport(app: &App) {
        assert!(!app.layout.cells.is_empty());
        let left = app.layout.cells_x();
        let right = app.layout.cells_right();
        for cell in &app.layout.cells {
            assert!(cell.x + 0.1 >= left, "célula antes do viewport: {cell:?}");
            assert!(
                cell.x + cell.width <= right + 0.1,
                "célula depois do viewport: {cell:?}"
            );
        }
    }

    #[test]
    fn tab_switches_to_path_mode_and_back() {
        let apps = vec![Item::new(
            "Firefox".into(),
            Some("firefox".into()),
            Action::Print("firefox".into()),
        )];
        let paths = vec![
            Item::new("firefox".into(), None, Action::Command("firefox".into())),
            Item::new("git".into(), None, Action::Command("git".into())),
        ];
        let mut app = App::with_path_mode(apps, paths, &Behavior::default());

        assert_eq!(app.mode(), Mode::Apps);
        assert!(app.shows_icons());
        assert_eq!(app.prompt("Run: "), "Run: ");

        type_text(&mut app, "gi");
        assert_eq!(app.query, "gi");

        assert!(app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default()));
        assert_eq!(app.mode(), Mode::Paths);
        assert!(!app.shows_icons());
        assert_eq!(app.prompt("Run: "), "PATH: ");
        assert_eq!(app.query, "gi");
        assert_eq!(app.selected_item().unwrap().label, "git");

        assert!(app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default()));
        assert_eq!(app.mode(), Mode::Apps);
        assert!(app.shows_icons());
        assert_eq!(app.query, "gi");
    }

    #[test]
    fn path_items_are_loaded_once_on_first_tab() {
        use std::cell::Cell;
        use std::rc::Rc;

        let calls = Rc::new(Cell::new(0));
        let calls_loader = calls.clone();
        let apps = vec![Item::new(
            "Firefox".into(),
            None,
            Action::Print("firefox".into()),
        )];
        let mut app = App::with_path_loader(
            apps,
            move || {
                calls_loader.set(calls_loader.get() + 1);
                vec![Item::new(
                    "htop".into(),
                    None,
                    Action::Command("htop".into()),
                )]
            },
            &Behavior::default(),
        );

        assert_eq!(calls.get(), 0);
        app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default());
        assert_eq!(calls.get(), 1);
        assert_eq!(app.mode(), Mode::Paths);
        app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default());
        app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default());
        assert_eq!(calls.get(), 1, "PATH must be scanned only once");
        assert_eq!(app.mode(), Mode::Paths);
    }

    #[test]
    fn tab_does_not_overwrite_the_query_without_modes() {
        let mut app = self::app();
        type_text(&mut app, "term");
        app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default());
        assert_eq!(app.query, "term");
        assert_eq!(app.cursor, 4);
        assert_eq!(app.mode(), Mode::Apps);
    }

    #[test]
    fn path_mode_activates_commands() {
        let paths = vec![Item::new(
            "htop".into(),
            None,
            Action::Command("htop".into()),
        )];
        let mut app = App::with_path_mode(Vec::new(), paths, &Behavior::default());
        app.handle_key(Keysym::Tab, Mods::default(), &Behavior::default());
        app.handle_key(Keysym::Return, Mods::default(), &Behavior::default());
        assert_eq!(app.outcome, Some(Outcome::Command("htop".into())));
    }

    #[test]
    fn enter_activates_selection() {
        let mut app = self::app();
        app.handle_key(Keysym::Return, Mods::default(), &Behavior::default());
        assert_eq!(app.outcome, Some(Outcome::Print("firefox".into())));
    }

    #[test]
    fn enter_without_selection_prints_query() {
        let mut app = self::app();
        type_text(&mut app, "zzz");
        assert!(app.filtered.is_empty());
        app.handle_key(Keysym::Return, Mods::default(), &Behavior::default());
        assert_eq!(app.outcome, Some(Outcome::Print("zzz".into())));
    }

    #[test]
    fn escape_requests_quit() {
        let mut app = self::app();
        app.handle_key(Keysym::Escape, Mods::default(), &Behavior::default());
        assert_eq!(app.outcome, Some(Outcome::Quit(1)));
    }

    #[test]
    fn lateral_arrows_move_selection_and_ctrl_moves_cursor() {
        let mut app = self::app();

        // Empty query: arrows navigate the list.
        app.handle_key(Keysym::Right, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 1);
        app.handle_key(Keysym::Left, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 0);

        // With text, arrows still navigate items; the cursor does not move.
        type_text(&mut app, "fi");
        assert_eq!(app.cursor, 2);
        app.handle_key(Keysym::Right, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 1);
        assert_eq!(app.cursor, 2);

        // Ctrl+Left moves the text cursor by word.
        let ctrl = Mods {
            ctrl: true,
            ..Default::default()
        };
        app.handle_key(Keysym::Left, ctrl, &Behavior::default());
        assert_eq!(app.cursor, 0);
        app.handle_key(Keysym::Right, ctrl, &Behavior::default());
        assert_eq!(app.cursor, 2);
    }

    #[test]
    fn home_and_end_jump_to_list_edges() {
        let mut app = self::app();
        app.handle_key(Keysym::End, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 2);
        assert_eq!(app.query, "");
        app.handle_key(Keysym::Home, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 0);

        // With text, the first press only moves the cursor.
        type_text(&mut app, "fi");
        app.handle_key(Keysym::Home, Mods::default(), &Behavior::default());
        assert_eq!(app.cursor, 0);
        app.handle_key(Keysym::Home, Mods::default(), &Behavior::default());
        assert_eq!(app.sel, 0);
    }

    #[test]
    fn ctrl_w_removes_word() {
        let mut app = self::app();
        type_text(&mut app, "foo bar");
        let mods = Mods {
            ctrl: true,
            ..Default::default()
        };
        app.handle_key(Keysym::from_char('w'), mods, &Behavior::default());
        assert_eq!(app.query, "foo ");
    }

    #[test]
    fn word_movement_respects_boundaries() {
        let mut app = self::app();
        type_text(&mut app, "foo bar baz");
        app.move_cursor_word(-1);
        assert_eq!(app.cursor, 8);
        app.move_cursor_word(-1);
        assert_eq!(app.cursor, 4);
        app.move_cursor_word(1);
        assert_eq!(app.cursor, 8);
    }
}
