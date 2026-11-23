//! The generic list of the selection dialog (S14): the oracle's
//! `SelectionViewParams` and `SelectionItem` as plain data, and the pure rules
//! of the list (which rows a search leaves, where the cursor may stand, what
//! the number shortcuts pick). No rtwk: the view keeps the cursor, the scroll,
//! the query and the tab in rtwk controllers and asks these functions; the
//! scroll window is `ListView`'s own, not ours. Oracle:
//! `bottom_pane/list_selection_view.rs` (`apply_filter`, `build_rows`,
//! `move_up`, `page_down`, `actual_idx_for_enabled_number`).

use serde::Deserialize;

/// Items shown at once (`MAX_POPUP_ROWS`, bottom_pane/popup_consts.rs).
pub const MAX_ROWS: usize = 8;

/// A second key that accepts the highlighted row (`SelectionSecondaryAction`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Secondary {
    pub key: char,
    /// The footer while the row is highlighted (`footer_hint` of
    /// `SelectionSecondaryAction`); with `accept` it is the key's word.
    pub hint: String,
    /// The word of the accept key: the footer is then built from the keymap
    /// (`enter default · s session · esc back`), as the standard footer is.
    #[serde(default)]
    pub accept: Option<String>,
}

/// What a run of a row's text means. The view turns each into a style of its
/// theme (the oracle writes `"L".underlined()`, `"Ready".green().italic()`:
/// raw styles; here they are roles, ADR-010).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Plain,
    /// A mnemonic letter: underlined.
    Key,
    /// A state that is fine (`Ready`): the success colour, italic.
    Success,
    /// A state that needs a look: the warning colour, italic.
    Warning,
    /// Secondary text: dim.
    Dim,
    /// Emphasis: bold.
    Strong,
    /// A note that points at something: the info colour (cyan).
    Info,
    /// A warning about harm (`"..".red()`): the error colour.
    Danger,
}

/// A run of text and its role (`name_prefix_spans`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Span {
    pub text: String,
    pub role: Role,
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
    /// A multi-select row the ordering keys may move (`MultiSelectItem::orderable`).
    pub orderable: bool,
    /// A rule is drawn under the row, unless it is the last one shown.
    pub section_break_after: bool,
    /// Runs drawn after the row's number and before its name, each in its
    /// role (`name_prefix_spans`).
    pub prefix_spans: Vec<Span>,
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
    /// How the subtitle reads: dim unless this is `Info` (the advanced
    /// reasoning menu's cost note is cyan).
    pub subtitle_role: Role,
    /// The paragraphs of a header the oracle builds as a `Paragraph` (the full
    /// access confirmation): runs in their roles, bold title above, no dim
    /// subtitle; an empty paragraph is a blank row. Replaces `subtitle`.
    pub header: Vec<Vec<Span>>,
    pub footer_note: Option<String>,
    pub footer_hint: Option<String>,
    /// The footer says what the list's accept and cancel keys are, as the
    /// keymap names them (`picker_hint_line_for_keymap`: `enter select · esc
    /// back`), unless `footer_hint` is given. The bottom pane gives every list
    /// it shows this footer (`apply_standard_popup_hint`).
    pub standard_hint: bool,
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
    /// The description column hides when it would be narrower than this many
    /// cells (`SelectionDescriptionLayout::HideWhenNarrow`).
    pub hide_description_below: Option<u16>,
    pub side: Option<Side>,
    /// The item (in `items`) the cursor starts on.
    pub initial: Option<usize>,
    pub max_rows: Option<usize>,
    /// A multi-select: the rows with a toggle are the choices, Space flips
    /// the one under the cursor (`Intent::Toggle`) and Enter confirms them
    /// all at once (`Intent::Confirm`, answered as `Picked::Items`).
    pub multi: bool,
    /// The ordering keys (left and right) move orderable rows.
    pub ordering: bool,
    /// A line under the rows names the toggled-on ones in list order
    /// (`on_preview` of the multi-select picker).
    pub preview: bool,
    /// Drawn on the panel of the bottom-pane pickers (`PickerSurface::Panel`:
    /// a blank row on top, the panel colour) instead of the terminal's own.
    pub panel: bool,
    /// The most rows that show their ordinal: the model pickers number nine (a digit key
    /// reaches those), the rows after them keep the blank gutter of a disabled row.
    pub numbered_rows: Option<usize>,
}

impl Default for List {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            subtitle_role: Role::Plain,
            header: Vec::new(),
            footer_note: None,
            footer_hint: None,
            standard_hint: false,
            items: Vec::new(),
            tabs: Vec::new(),
            initial_tab: None,
            searchable: false,
            search_placeholder: None,
            numbered: true,
            columns: Columns::AutoVisible,
            single_line: false,
            name_column_width: None,
            hide_description_below: None,
            side: None,
            initial: None,
            max_rows: None,
            multi: false,
            ordering: false,
            preview: false,
            panel: false,
            numbered_rows: None,
        }
    }
}

/// A row ready to draw: the gutter, the name, the description. Nothing in it
/// depends on where the cursor is: the view puts the cursor's mark before the
/// prefix and picks the description the cursor row shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Number or letter, toggle box: everything between the cursor's mark (one
    /// cell, the view's) and the name.
    pub prefix: String,
    pub name: String,
    pub description: Option<String>,
    /// Shown instead of `description` while the cursor is on the row.
    pub selected_description: Option<String>,
    pub disabled: bool,
    /// The row is the one in use: its name is followed by the view's mark.
    pub current: bool,
    /// Why a disabled row is: shown after its name and in its description.
    pub reason: Option<String>,
    /// A rule row follows this one (`Item::section_break_after`).
    pub section_break: bool,
    /// The runs between the prefix and the name (`Item::prefix_spans`).
    pub prefix_spans: Vec<Span>,
}

impl List {
    /// What the list is drawn on.
    pub fn surface(&self) -> super::layout::Surface {
        use super::layout::Surface;
        if self.multi {
            Surface::Multi
        } else if self.panel {
            Surface::Panel
        } else {
            Surface::Terminal
        }
    }

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
            tab.items
                .iter()
                .enumerate()
                .map(move |(i, item)| (Some(t), i, item))
        });
        let flat = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| (None, i, item));
        flat.chain(tabs)
            .map(|(t, i, item)| (self.id_of(t, i), item))
    }

    /// The item with this id.
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.each()
            .find(|(found, _)| found == id)
            .map(|(_, item)| item)
    }

    /// The ids of the items whose toggle is on, in the order of the list
    /// (the answer of a multi-select).
    pub fn checked(&self) -> Vec<String> {
        self.each()
            .filter(|(_, item)| item.toggle == Some(true))
            .map(|(id, _)| id)
            .collect()
    }

    /// Moves the item with this id one place up or down among the list's own
    /// items, when it and the item it swaps with are both orderable
    /// (`MultiSelectPicker::move_selected_item`).
    pub fn reorder(&mut self, id: &str, down: bool) {
        if let Some((at, to)) = self.reorder_of(id, down) {
            self.items.swap(at, to);
        }
    }

    /// Whether `reorder` would move the item.
    pub fn can_reorder(&self, id: &str, down: bool) -> bool {
        self.reorder_of(id, down).is_some()
    }

    /// The two places a move swaps.
    fn reorder_of(&self, id: &str, down: bool) -> Option<(usize, usize)> {
        if !self.ordering {
            return None;
        }
        let at = (0..self.items.len()).find(|index| self.id_of(None, *index) == id)?;
        let to = if down { at + 1 } else { at.checked_sub(1)? };
        let orderable = |index: usize| self.items.get(index).is_some_and(|item| item.orderable);
        (orderable(at) && orderable(to)).then_some((at, to))
    }

    /// The names of the toggled-on items in list order, for the preview line.
    pub fn preview_names(&self) -> Option<Vec<&str>> {
        self.preview.then(|| {
            self.items
                .iter()
                .filter(|item| item.toggle == Some(true))
                .map(|item| item.name.as_str())
                .collect()
        })
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
            self.tabs
                .iter_mut()
                .flat_map(|tab| tab.items.iter_mut())
                .nth(target)
        };
        if let Some(Item {
            toggle: Some(on), ..
        }) = item
        {
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
    pub fn settle(
        &self,
        visible: &[usize],
        previous: Option<usize>,
        initial: Option<usize>,
    ) -> Option<usize> {
        let items = self.items();
        let enabled_at = |index: usize| items.get(index).is_some_and(|item| !item.is_disabled());
        let wanted = previous
            .filter(|index| enabled_at(*index))
            .or_else(|| initial.filter(|index| enabled_at(*index)))
            .or_else(|| {
                (!self.list.searchable)
                    .then(|| {
                        items
                            .iter()
                            .position(|item| item.current && !item.is_disabled())
                    })
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
    pub fn page(
        &self,
        visible: &[usize],
        from: Option<usize>,
        down: bool,
        window: usize,
    ) -> Option<usize> {
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
        if down {
            after.or(before)
        } else {
            before.or(after)
        }
        .unwrap_or(pos)
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

    /// Where the row of the visible item at `pos` stands among the entries
    /// of the drawn list: a rule under a row (`Row::section_break`) is an
    /// entry of its own.
    pub fn entry_of(&self, visible: &[usize], pos: usize) -> usize {
        let items = self.items();
        let rules = visible
            .iter()
            .take(pos)
            .filter(|index| {
                items
                    .get(**index)
                    .is_some_and(|item| item.section_break_after)
            })
            .count();
        pos + rules
    }

    /// The rows to draw, one per visible item (`visible`, from `visible()`).
    pub fn rows(&self, visible: &[usize]) -> Vec<Row> {
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
                let disabled = item.is_disabled();
                let gutter = if self.list.searchable {
                    " ".to_string()
                } else if !self.list.numbered {
                    item.shortcut
                        .map_or_else(|| " ".to_string(), |key| format!(" {key}. "))
                } else if disabled {
                    format!("{:w$}", "", w = number_width + 3)
                } else if let Some(label) = item.child_label {
                    format!("    {label}. ")
                } else {
                    number += 1;
                    if self.list.numbered_rows.is_some_and(|rows| number > rows) {
                        format!("{:w$}", "", w = number_width + 3)
                    } else {
                        format!(" {number}. ")
                    }
                };
                let toggle = match item.toggle {
                    Some(true) if item.checkbox => "[x] ",
                    Some(true) => "[*] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                Row {
                    prefix: format!("{gutter}{toggle}"),
                    name: item.name.clone(),
                    description: item.description.clone(),
                    selected_description: item.selected_description.clone(),
                    disabled,
                    current: item.current,
                    reason: item.disabled_reason.clone(),
                    section_break: item.section_break_after && pos + 1 < visible.len(),
                    prefix_spans: item.prefix_spans.clone(),
                }
            })
            .collect()
    }
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
        List {
            items,
            ..List::default()
        }
    }

    fn browse<'a>(list: &'a List, query: &'a str) -> Browse<'a> {
        Browse {
            list,
            tab: None,
            query,
        }
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
        let mut plain = List {
            items: vec![item("Off"), item("A")],
            ..List::default()
        };
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
    fn rows_number_the_enabled_rows_and_mark_the_current() {
        let mut off = item("Off");
        off.disabled_reason = Some("why".to_string());
        off.description = Some("text".to_string());
        let mut now = item("Now");
        now.current = true;
        let list = list(vec![item("A"), off, now]);
        let view = browse(&list, "");
        let rows = view.rows(&view.visible());
        assert_eq!(rows[0].prefix, " 1. ");
        assert_eq!(rows[1].prefix, "    ");
        assert!(rows[1].disabled);
        assert_eq!(rows[1].reason.as_deref(), Some("why"));
        assert_eq!(rows[1].description.as_deref(), Some("text"));
        assert_eq!(rows[2].prefix, " 2. ");
        assert!(rows[2].current);
    }

    #[test]
    fn a_preview_sits_beside_the_rows_only_when_both_fit() {
        let side = |width, min_width| Side {
            width,
            min_width,
            ..Side::default()
        };
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
