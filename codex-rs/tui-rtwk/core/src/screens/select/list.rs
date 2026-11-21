//! The generic list of the selection dialog (S14): the oracle's
//! `SelectionViewParams` and `SelectionItem` as plain data, and the pure rules
//! of the list (which rows a search leaves, where the cursor may stand, what
//! the number shortcuts pick, which items a window shows). No rtwk: the view
//! keeps the cursor, the query and the tab in rtwk controllers and asks these
//! functions. Oracle: `bottom_pane/list_selection_view.rs` (`apply_filter`,
//! `build_rows`, `move_up`, `page_down`, `actual_idx_for_enabled_number`) and
//! `bottom_pane/scroll_state.rs`.

use serde::Deserialize;

/// Items shown at once (`MAX_POPUP_ROWS`, bottom_pane/popup_consts.rs).
pub const MAX_ROWS: usize = 8;

/// A second key that accepts the highlighted row (`SelectionSecondaryAction`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Secondary {
    pub key: char,
    pub hint: String,
}

/// One row (`SelectionItem`). A row is disabled when `disabled` is set or a
/// `disabled_reason` is given; it cannot be accepted and the cursor skips it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Item {
    /// What an accept returns; empty: the position (`List::id_of`).
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Shown instead of `description` while the row is highlighted.
    pub selected_description: Option<String>,
    pub current: bool,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
    /// Indented under the row above, lettered instead of numbered.
    pub child_label: Option<char>,
    /// A key that picks the row (`display_shortcut`).
    pub shortcut: Option<char>,
    /// What a search matches (a row without one is hidden by any query).
    pub search_value: Option<String>,
    /// A toggle row: its value (`[*]`, `[ ]`).
    pub toggle: Option<bool>,
    /// The toggle is drawn `[x]` and the accept key flips it.
    pub checkbox: bool,
    /// The footer while this row is highlighted.
    pub footer_hint: Option<String>,
    pub secondary: Option<Secondary>,
    /// An accept does not close the list.
    pub keep_open: bool,
}

impl Item {
    pub fn is_disabled(&self) -> bool {
        self.disabled || self.disabled_reason.is_some()
    }
}

/// A tab of a tabbed list: its own items.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub items: Vec<Item>,
}

/// How the description column is placed (`ColumnWidthMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Columns {
    /// From the rows in the window.
    #[default]
    AutoVisible,
    /// From all rows: scrolling does not move the column.
    AutoAllRows,
}

/// How wide the side preview is (`SideContentWidth`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideWidth {
    Fixed(u16),
    Half,
}

/// The preview beside (or, when it does not fit, below) the rows.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Side {
    pub lines: Vec<String>,
    /// Shown below when the side-by-side layout does not fit.
    pub stacked: Option<Vec<String>>,
    pub width: SideWidth,
    pub min_width: u16,
    /// A background colour name the preview keeps (`preserve_side_content_bg`).
    pub bg: Option<String>,
}

impl Default for Side {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            stacked: None,
            width: SideWidth::Half,
            min_width: 0,
            bg: None,
        }
    }
}

/// Cells between the rows and a side preview (`SIDE_CONTENT_GAP`).
pub const SIDE_GAP: u16 = 2;
/// The rows keep at least this many cells beside a preview
/// (`MIN_LIST_WIDTH_FOR_SIDE`).
pub const MIN_LIST_WIDTH_FOR_SIDE: u16 = 40;

impl Side {
    /// The width of the preview when it sits beside the rows in a popup whose
    /// content (inside its insets) is `content_width` cells wide; none when it
    /// does not fit and the stacked preview takes its place
    /// (`side_by_side_layout_widths`).
    pub fn beside(&self, content_width: u16) -> Option<u16> {
        let width = match self.width {
            SideWidth::Fixed(0) => return None,
            SideWidth::Fixed(width) => width,
            SideWidth::Half => content_width.saturating_sub(SIDE_GAP) / 2,
        };
        if width < self.min_width {
            return None;
        }
        let list = content_width.saturating_sub(SIDE_GAP + width);
        (list >= MIN_LIST_WIDTH_FOR_SIDE).then_some(width)
    }

    /// The lines shown below the rows when the preview does not fit beside.
    pub fn below(&self) -> &[String] {
        self.stacked.as_deref().unwrap_or(&self.lines)
    }
}

/// The whole list (`SelectionViewParams`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct List {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub footer_note: Option<String>,
    pub footer_hint: Option<String>,
    pub items: Vec<Item>,
    pub tabs: Vec<Tab>,
    pub initial_tab: Option<String>,
    pub searchable: bool,
    pub search_placeholder: Option<String>,
    /// Rows carry their ordinal as a shortcut (`auto_number_shortcuts`).
    pub numbered: bool,
    pub columns: Columns,
    /// One line per row, cut with an ellipsis (`SelectionRowDisplay::SingleLine`).
    pub single_line: bool,
    pub name_column_width: Option<usize>,
    pub side: Option<Side>,
    /// The item (in `items`) the cursor starts on.
    pub initial: Option<usize>,
    pub max_rows: Option<usize>,
}

impl Default for List {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            footer_note: None,
            footer_hint: None,
            items: Vec::new(),
            tabs: Vec::new(),
            initial_tab: None,
            searchable: false,
            search_placeholder: None,
            numbered: true,
            columns: Columns::AutoVisible,
            single_line: false,
            name_column_width: None,
            side: None,
            initial: None,
            max_rows: None,
        }
    }
}

/// A row ready to draw: the gutter, the name, the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Marker, number or letter, toggle box: everything before the name.
    pub prefix: String,
    pub name: String,
    pub description: Option<String>,
    pub disabled: bool,
    /// A disabled row's name is followed by ` (disabled)`.
    pub disabled_mark: bool,
}

impl List {
    pub fn max_rows(&self) -> usize {
        self.max_rows.unwrap_or(MAX_ROWS).max(1)
    }

    /// The tab the list opens on, none when it has no tabs.
    pub fn first_tab(&self) -> Option<usize> {
        if self.tabs.is_empty() {
            return None;
        }
        Some(
            self.initial_tab
                .as_ref()
                .and_then(|id| self.tabs.iter().position(|tab| &tab.id == id))
                .unwrap_or(0),
        )
    }

    /// The items of `tab` (the list's own when it has no tabs).
    pub fn items_of(&self, tab: Option<usize>) -> &[Item] {
        match tab.and_then(|tab| self.tabs.get(tab)) {
            Some(tab) => &tab.items,
            None => &self.items,
        }
    }

    /// The id an accept of item `index` of `tab` returns.
    pub fn id_of(&self, tab: Option<usize>, index: usize) -> String {
        match self.items_of(tab).get(index) {
            Some(item) if !item.id.is_empty() => item.id.clone(),
            _ => match tab {
                Some(tab) => format!("{tab}:{index}"),
                None => index.to_string(),
            },
        }
    }

    fn each(&self) -> impl Iterator<Item = (String, &Item)> {
        let tabs = self.tabs.iter().enumerate().flat_map(|(t, tab)| {
            tab.items.iter().enumerate().map(move |(i, item)| (Some(t), i, item))
        });
        let flat = self.items.iter().enumerate().map(|(i, item)| (None, i, item));
        flat.chain(tabs)
            .map(|(t, i, item)| (self.id_of(t, i), item))
    }

    /// The item with this id.
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.each().find(|(found, _)| found == id).map(|(_, item)| item)
    }

    /// Flips the toggle of the item with this id.
    pub fn flip(&mut self, id: &str) {
        let target = self.each().position(|(found, _)| found == id);
        let Some(mut target) = target else { return };
        let flat = self.items.len();
        let item = if target < flat {
            self.items.get_mut(target)
        } else {
            target -= flat;
            self.tabs.iter_mut().flat_map(|tab| tab.items.iter_mut()).nth(target)
        };
        if let Some(Item { toggle: Some(on), .. }) = item {
            *on = !*on;
        }
    }
}

/// The list as one tab, a query and a cursor see it.
#[derive(Clone, Copy)]
pub struct Browse<'a> {
    pub list: &'a List,
    pub tab: Option<usize>,
    pub query: &'a str,
}

impl Browse<'_> {
    pub fn items(&self) -> &[Item] {
        self.list.items_of(self.tab)
    }

    /// Indices into `items()` the search leaves, in order.
    pub fn visible(&self) -> Vec<usize> {
        let items = self.items();
        if self.list.searchable && !self.query.is_empty() {
            let query = self.query.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.search_value
                        .as_ref()
                        .is_some_and(|value| value.to_lowercase().contains(&query))
                })
                .map(|(index, _)| index)
                .collect()
        } else {
            (0..items.len()).collect()
        }
    }

    fn enabled(&self, visible: &[usize], pos: usize) -> bool {
        visible
            .get(pos)
            .and_then(|index| self.items().get(*index))
            .is_some_and(|item| !item.is_disabled())
    }

    /// The cursor (a position in `visible`) after the rows changed: the item
    /// that was highlighted if it is still there and enabled, else the
    /// initial one, else the current row (of a list with no search), else the
    /// first enabled row.
    pub fn settle(&self, visible: &[usize], previous: Option<usize>, initial: Option<usize>) -> Option<usize> {
        let items = self.items();
        let enabled_at = |index: usize| items.get(index).is_some_and(|item| !item.is_disabled());
        let wanted = previous
            .filter(|index| enabled_at(*index))
            .or_else(|| initial.filter(|index| enabled_at(*index)))
            .or_else(|| {
                (!self.list.searchable)
                    .then(|| items.iter().position(|item| item.current && !item.is_disabled()))
                    .flatten()
            });
        wanted
            .and_then(|index| visible.iter().position(|v| *v == index))
            .or_else(|| (0..visible.len()).find(|pos| self.enabled(visible, *pos)))
            .or_else(|| (!visible.is_empty()).then_some(0))
    }

    /// One row on (`down`) or back, round the ends, over disabled rows.
    pub fn step(&self, visible: &[usize], from: Option<usize>, down: bool) -> Option<usize> {
        let len = visible.len();
        if len == 0 {
            return None;
        }
        let mut pos = from;
        let next = |pos: Option<usize>| match (pos, down) {
            (Some(p), true) if p + 1 < len => p + 1,
            (_, true) => 0,
            (Some(p), false) if p > 0 => p - 1,
            (Some(_), false) => len - 1,
            (None, false) => 0,
        };
        pos = Some(next(pos));
        for _ in 0..len {
            if pos.is_some_and(|p| self.enabled(visible, p)) {
                break;
            }
            pos = Some(next(pos));
        }
        pos
    }

    /// A page on or back, stopping at the ends, then off a disabled row.
    pub fn page(&self, visible: &[usize], from: Option<usize>, down: bool, window: usize) -> Option<usize> {
        let len = visible.len();
        if len == 0 {
            return None;
        }
        let step = window.max(1);
        let current = from.unwrap_or(0).min(len - 1);
        let to = if down {
            current.saturating_add(step).min(len - 1)
        } else {
            current.saturating_sub(step)
        };
        Some(self.off_disabled(visible, to, down))
    }

    /// The first (`top`) or last row, then off a disabled row.
    pub fn jump(&self, visible: &[usize], top: bool) -> Option<usize> {
        let len = visible.len();
        if len == 0 {
            return None;
        }
        let to = if top { 0 } else { len - 1 };
        Some(self.off_disabled(visible, to, top))
    }

    /// `pos`, or the nearest enabled row in direction `down` first, then the
    /// other (`skip_disabled_*_clamped`).
    fn off_disabled(&self, visible: &[usize], pos: usize, down: bool) -> usize {
        if self.enabled(visible, pos) {
            return pos;
        }
        let len = visible.len();
        let after = ((pos + 1)..len).find(|p| self.enabled(visible, *p));
        let before = (0..pos).rev().find(|p| self.enabled(visible, *p));
        if down { after.or(before) } else { before.or(after) }.unwrap_or(pos)
    }

    /// The item a typed key picks (index into `items()`): a row's own key or
    /// letter, else a number among the enabled, unlettered rows.
    pub fn shortcut(&self, key: char) -> Option<usize> {
        if self.list.searchable {
            return None;
        }
        let items = self.items();
        let own = items.iter().position(|item| {
            (item.shortcut == Some(key) || item.child_label == Some(key)) && !item.is_disabled()
        });
        if own.is_some() {
            return own;
        }
        if !self.list.numbered {
            return None;
        }
        let number = usize::try_from(key.to_digit(10)?).ok()?;
        if number == 0 {
            return None;
        }
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_disabled() && item.child_label.is_none())
            .nth(number - 1)
            .map(|(index, _)| index)
    }

    /// The rows to draw, one per visible item, `cursor` a position in
    /// `visible`.
    pub fn rows(&self, visible: &[usize], cursor: Option<usize>) -> Vec<Row> {
        let items = self.items();
        let number_width = visible
            .iter()
            .filter(|index| {
                items
                    .get(**index)
                    .is_some_and(|item| !item.is_disabled() && item.child_label.is_none())
            })
            .count()
            .max(1)
            .to_string()
            .len();
        let mut number = 0usize;
        visible
            .iter()
            .enumerate()
            .filter_map(|(pos, index)| items.get(*index).map(|item| (pos, item)))
            .map(|(pos, item)| {
                let selected = cursor == Some(pos);
                let mark = if selected { '\u{203a}' } else { ' ' };
                let disabled = item.is_disabled();
                let gutter = if self.list.searchable {
                    format!("{mark} ")
                } else if !self.list.numbered {
                    item.shortcut
                        .map_or_else(|| format!("{mark} "), |key| format!("{mark} {key}. "))
                } else if disabled {
                    format!("{mark} {}", " ".repeat(number_width + 2))
                } else if let Some(label) = item.child_label {
                    format!("{mark}    {label}. ")
                } else {
                    number += 1;
                    format!("{mark} {number}. ")
                };
                let toggle = match item.toggle {
                    Some(true) if item.checkbox => "[x] ",
                    Some(true) => "[*] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                let description = if selected {
                    item.selected_description.clone().or_else(|| item.description.clone())
                } else {
                    item.description.clone()
                };
                let description = match (&description, &item.disabled_reason) {
                    (Some(text), Some(reason)) => Some(format!("{text} (disabled: {reason})")),
                    (None, Some(reason)) => Some(format!("disabled: {reason}")),
                    _ => description,
                };
                let mark_current = if item.current { " (current)" } else { "" };
                Row {
                    prefix: format!("{gutter}{toggle}"),
                    name: format!("{}{mark_current}", item.name),
                    description,
                    disabled,
                    disabled_mark: item.disabled_reason.is_some(),
                }
            })
            .collect()
    }
}

/// The first item of the window after the cursor moved to `cursor`: the window
/// stays where it is while the cursor is inside it (`ScrollState::ensure_visible`).
pub fn window_top(top: usize, cursor: Option<usize>, len: usize, rows: usize) -> usize {
    let mut top = top.min(len.saturating_sub(1));
    if let Some(cursor) = cursor {
        if cursor < top {
            top = cursor;
        } else if cursor >= top + rows {
            top = cursor + 1 - rows;
        }
    }
    top
}

/// A pasted search query: its words joined by one blank; none: nothing to
/// paste (`normalize_pasted_search_query`).
pub fn normalize_paste(pasted: &str) -> Option<String> {
    let words = pasted.split_whitespace().collect::<Vec<_>>().join(" ");
    (!words.is_empty()).then_some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> Item {
        Item {
            name: name.to_string(),
            search_value: Some(name.to_lowercase()),
            ..Item::default()
        }
    }

    fn list(items: Vec<Item>) -> List {
        List { items, ..List::default() }
    }

    fn browse<'a>(list: &'a List, query: &'a str) -> Browse<'a> {
        Browse { list, tab: None, query }
    }

    #[test]
    fn search_keeps_the_rows_that_match_and_hides_rows_without_a_value() {
        let mut hidden = item("Zed");
        hidden.search_value = None;
        let mut list = list(vec![item("Alpha"), item("Beta"), hidden]);
        list.searchable = true;
        assert_eq!(browse(&list, "").visible(), vec![0, 1, 2]);
        assert_eq!(browse(&list, "ET").visible(), vec![1]);
        assert!(browse(&list, "zed").visible().is_empty());
    }

    #[test]
    fn settle_keeps_the_highlight_then_the_current_row_then_the_first_enabled() {
        let mut off = item("Off");
        off.disabled = true;
        let mut now = item("Now");
        now.current = true;
        let list = list(vec![off, item("A"), now]);
        let view = browse(&list, "");
        let visible = view.visible();
        assert_eq!(view.settle(&visible, Some(1), None), Some(1));
        assert_eq!(view.settle(&visible, None, None), Some(2));
        assert_eq!(view.settle(&visible, Some(0), None), Some(2));
        let mut plain = List { items: vec![item("Off"), item("A")], ..List::default() };
        plain.items[0].disabled = true;
        let view = browse(&plain, "");
        assert_eq!(view.settle(&view.visible(), None, None), Some(1));
    }

    #[test]
    fn step_wraps_round_the_ends_and_skips_disabled_rows() {
        let mut list = list(vec![item("a"), item("b"), item("c")]);
        list.items[1].disabled_reason = Some("no".to_string());
        let view = browse(&list, "");
        let visible = view.visible();
        assert_eq!(view.step(&visible, Some(0), true), Some(2));
        assert_eq!(view.step(&visible, Some(2), true), Some(0));
        assert_eq!(view.step(&visible, Some(0), false), Some(2));
        assert_eq!(view.step(&visible, None, false), Some(0));
        assert_eq!(view.step(&[], None, true), None);
    }

    #[test]
    fn page_and_jump_stop_at_the_ends_and_leave_disabled_rows() {
        let mut list = list((0..10).map(|n| item(&n.to_string())).collect());
        list.items[9].disabled = true;
        let view = browse(&list, "");
        let visible = view.visible();
        assert_eq!(view.page(&visible, Some(1), true, 4), Some(5));
        assert_eq!(view.page(&visible, Some(7), true, 4), Some(8));
        assert_eq!(view.page(&visible, Some(2), false, 4), Some(0));
        assert_eq!(view.jump(&visible, true), Some(0));
        assert_eq!(view.jump(&visible, false), Some(8));
    }

    #[test]
    fn a_number_picks_among_the_enabled_unlettered_rows_a_search_takes_no_keys() {
        let mut off = item("off");
        off.disabled = true;
        let mut child = item("child");
        child.child_label = Some('a');
        let mut list = list(vec![off, item("one"), child, item("two")]);
        assert_eq!(browse(&list, "").shortcut('1'), Some(1));
        assert_eq!(browse(&list, "").shortcut('2'), Some(3));
        assert_eq!(browse(&list, "").shortcut('a'), Some(2));
        assert_eq!(browse(&list, "").shortcut('0'), None);
        list.searchable = true;
        assert_eq!(browse(&list, "").shortcut('1'), None);
    }

    #[test]
    fn rows_number_the_enabled_rows_and_mark_the_cursor_and_current() {
        let mut off = item("Off");
        off.disabled_reason = Some("why".to_string());
        off.description = Some("text".to_string());
        let mut now = item("Now");
        now.current = true;
        let list = list(vec![item("A"), off, now]);
        let view = browse(&list, "");
        let rows = view.rows(&view.visible(), Some(0));
        assert_eq!(rows[0].prefix, "\u{203a} 1. ");
        assert_eq!(rows[1].prefix, "     ");
        assert!(rows[1].disabled && rows[1].disabled_mark);
        assert_eq!(rows[1].description.as_deref(), Some("text (disabled: why)"));
        assert_eq!(rows[2].prefix, "  2. ");
        assert_eq!(rows[2].name, "Now (current)");
    }

    #[test]
    fn the_window_follows_the_cursor_only_when_it_leaves() {
        assert_eq!(window_top(0, Some(7), 20, 8), 0);
        assert_eq!(window_top(0, Some(8), 20, 8), 1);
        assert_eq!(window_top(5, Some(2), 20, 8), 2);
        assert_eq!(window_top(30, None, 20, 8), 19);
    }

    #[test]
    fn a_preview_sits_beside_the_rows_only_when_both_fit() {
        let side = |width, min_width| Side { width, min_width, ..Side::default() };
        // `side_layout_width_half_uses_exact_split`: half of what the gap leaves
        assert_eq!(side(SideWidth::Half, 10).beside(120), Some(59));
        // `side_layout_width_half_falls_back_when_list_would_be_too_narrow`
        assert_eq!(side(SideWidth::Half, 50).beside(80), None);
        assert_eq!(side(SideWidth::Fixed(70), 0).beside(100), None);
        assert_eq!(side(SideWidth::Fixed(0), 0).beside(100), None);
        assert_eq!(side(SideWidth::Fixed(30), 0).beside(100), Some(30));
        // the stacked fixture: a 70 wide popup, half is 33 < min 60
        assert_eq!(side(SideWidth::Half, 60).beside(66), None);
    }

    #[test]
    fn a_paste_is_one_line_of_words_or_nothing() {
        assert_eq!(normalize_paste(" a \n b\t c "), Some("a b c".to_string()));
        assert_eq!(normalize_paste(" \n "), None);
    }

    #[test]
    fn flip_changes_the_toggle_of_the_item_with_the_id() {
        let mut one = item("one");
        one.id = "x".to_string();
        one.toggle = Some(false);
        let mut list = list(vec![one]);
        list.flip("x");
        assert_eq!(list.find("x").and_then(|item| item.toggle), Some(true));
    }
}
