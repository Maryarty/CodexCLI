//! The /config panel (the oracle's `config_panel/`: `mod.rs`, `input.rs`, `render.rs`): a tab
//! strip of five groups over the rows of the catalogue, each a name, a value and a description,
//! with a search line that lists every group's matches. In Flutter it is a bottom sheet with a
//! `TabBar` over a `Column` of tappable rows. What the group, the cursor and the search text are
//! is the sheet's (`ConfigState`); what a row holds is the model's (`ConfigPanel`, ADR-013): a
//! switch asks `Edit::Toggled`, a choice `Edit::Chosen`, the test `Edit::TestNotification`.
//! The mouse (ADR-011, beyond the oracle): a tap puts the cursor on a row, a second tap does
//! what Enter does; a tap on a tab selects the group.

mod editors;

use std::rc::Rc;

use rtwk::Handler;
use rtwk::focus::{Focus, FocusNode, KeyCombo, KeyEventResult, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::select;
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::{
    ConfigPanel, Edit, Offer, PanelNotice, Section, SettingId, Value, Which,
};

use super::keys::{Keys, ListKey, is_interrupt, letter_of, list_key};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::keymap::{types_text, unmodified};
use crate::widgets::dots::dots_end;
use crate::widgets::session_list::{Move, target};
use crate::widgets::tab_strip::Tabs;

pub use editors::Editor;

/// The sheet of the configuration panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConfigSheet;

/// Widget state: the group, the cursor and the search text, and the editor on top.
pub struct ConfigState {
    keys: Keys,
    node: FocusNode,
    section: usize,
    cursor: usize,
    searching: bool,
    query: String,
    /// The editor of a row that is up in place of the list.
    editor: Option<Editor>,
    /// The row of the editor's list the cursor is on.
    pick: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Group(bool),
    Accept,
    Close,
    Search,
    Backspace,
    Type(char),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    /// A tap on a row (of the list or of the editor).
    Row(usize),
    Tab(usize),
}

fn act_of(keys: &codex_tui_core::keymap::Keymap, key: &KeyPress, searching: bool) -> Option<Act> {
    if is_interrupt(key) {
        return Some(Act::Close);
    }
    if KeyCombo::parse("backspace").is_ok_and(|combo| combo.accepts_press(key)) {
        return Some(Act::Backspace);
    }
    if let Some(ch) = letter_of(key)
        && types_text(key)
        && (searching || (ch == '/' && unmodified(key)))
    {
        return Some(if searching { Act::Type(ch) } else { Act::Search });
    }
    match list_key(keys, key, searching)? {
        ListKey::Move(mv) => Some(Act::Move(mv)),
        ListKey::Left => Some(Act::Group(false)),
        ListKey::Right => Some(Act::Group(true)),
        ListKey::Accept => Some(Act::Accept),
        ListKey::Cancel => Some(Act::Close),
    }
}

impl StatefulWidget for ConfigSheet {
    type State = ConfigState;

    fn create_state(&self) -> ConfigState {
        ConfigState {
            keys: Keys::new(),
            node: FocusNode::new(),
            section: 0,
            cursor: 0,
            searching: false,
            query: String::new(),
            editor: None,
            pick: 0,
        }
    }
}

impl_widget!(stateful eq ConfigSheet);

fn offered(cx: &mut Cx<'_, ConfigState>) -> Option<ConfigPanel> {
    select::<FeatureStates, Option<ConfigPanel>, _>(cx, |states| {
        match states.settings.offers.get(&Which::Config) {
            Some(Offer::Config(panel)) => Some(panel.clone()),
            _ => None,
        }
    })
}

impl ConfigState {
    fn visible(&self, panel: &ConfigPanel) -> Vec<usize> {
        panel.visible(Section::ALL[self.section], &self.query)
    }

    /// The row Enter acts on: the model's request, or the dialog it opens.
    fn accept(&mut self, panel: &ConfigPanel, cx: &mut Cx<'_, Self>) {
        let rows = self.visible(panel);
        let Some(row) = rows.get(self.cursor).map(|at| &panel.rows[*at]) else {
            return;
        };
        match (&row.value, row.id) {
            (Value::Toggle(on), id) => super::request(cx, Edit::Toggled(id, !on)),
            (Value::Test, _) => super::request(cx, Edit::TestNotification),
            (Value::Unavailable, _) => {}
            (_, SettingId::Effects) => self.edit(Editor::Effects),
            (_, SettingId::ModelProviders) => {
                close_dialog::<_, ()>(cx, None);
                crate::features::providers::open(cx);
            }
            (_, id) => match editors::dialog_of(id) {
                Some(which) => {
                    close_dialog::<_, ()>(cx, None);
                    super::open(cx, super::Args {
                        which,
                        inspect: false,
                    });
                }
                None if editors::is_choice(id) => self.edit(Editor::Choice(id)),
                None => {}
            },
        }
    }

    fn edit(&mut self, editor: Editor) {
        self.editor = Some(editor);
        self.pick = 0;
    }

    fn act(&mut self, act: Act, panel: &ConfigPanel, cx: &mut Cx<'_, Self>) {
        if let Some(editor) = self.editor {
            self.act_editor(editor, act, panel, cx);
            return;
        }
        let rows = self.visible(panel);
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(rows.len(), Some(self.cursor), &|_| true, mv, rows.len()) {
                    self.cursor = to;
                }
            }
            Act::Group(next) if self.query.is_empty() => {
                let count = Section::ALL.len();
                self.section = (self.section + if next { 1 } else { count - 1 }) % count;
                self.cursor = 0;
            }
            Act::Group(_) => {}
            Act::Accept => self.accept(panel, cx),
            // Esc clears the search first, then closes (`esc clear search/close`).
            Act::Close if self.searching || !self.query.is_empty() => {
                self.searching = false;
                self.query.clear();
                self.cursor = 0;
            }
            Act::Close => close_dialog::<_, ()>(cx, None),
            Act::Search => self.searching = true,
            Act::Backspace => {
                self.query.pop();
                self.cursor = 0;
            }
            Act::Type(ch) => {
                self.query.push(ch);
                self.cursor = 0;
            }
        }
    }
}

impl State for ConfigState {
    type Widget = ConfigSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &ConfigSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let keys = self.keys.clone();
        let searching = Rc::new(std::cell::Cell::new(false));
        let seen = searching.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            match act_of(&keys.get(), &key, seen.get()) {
                Some(act) => {
                    if matches!(act, Act::Search) {
                        seen.set(true);
                    }
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            }
        })));
        // The callback keeps its own copy of "typing": Esc ends it in `update`.
        drop(searching);
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
    }

    fn did_change_dependencies(&mut self, _: &ConfigSheet, cx: &mut Cx<'_, Self>) {
        self.keys.refresh(cx);
    }

    fn update(&mut self, msg: Msg, _: &ConfigSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(panel) = offered(cx) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Act(act) => self.act(act, &panel, cx),
            Msg::Tab(index) if self.editor.is_none() && index < Section::ALL.len() => {
                self.section = index;
                self.cursor = 0;
            }
            Msg::Tab(_) => {}
            Msg::Row(row) => {
                let at = if self.editor.is_some() {
                    &mut self.pick
                } else {
                    &mut self.cursor
                };
                if *at == row {
                    self.act(Act::Accept, &panel, cx);
                } else {
                    *at = row;
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &ConfigSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let Some(panel) = offered(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let body = match self.editor {
            Some(editor) => self.editor_body(editor, &panel, s, &tokens, cx),
            None => self.list_body(&panel, s, &tokens, cx),
        };
        let surface = tokens.prompt_surface(cx, false);
        Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(body)))
            .focus_node(self.node.clone())
            .autofocus()
            .into()
    }
}

/// The words of the value column.
fn value_text(s: &Strings, value: &Value) -> String {
    match value {
        Value::Toggle(true) => s.value_on.to_string(),
        Value::Toggle(false) => s.value_off.to_string(),
        Value::Choice(label) => s.choice(label).to_string(),
        Value::Configure => s.value_configure.to_string(),
        Value::Test => s.value_test.to_string(),
        Value::Unavailable => s.value_unavailable.to_string(),
    }
}

fn notice_text(s: &Strings, notice: &PanelNotice) -> String {
    match notice {
        PanelNotice::CouldNotSave(error) => s.could_not_save(error).to_string(),
        PanelNotice::AppliesToNewComposers => s.notice_new_composers.to_string(),
        PanelNotice::TestScheduled => s.notice_test_scheduled.to_string(),
        PanelNotice::TestAlreadyScheduled => s.notice_test_already.to_string(),
    }
}

impl ConfigState {
    fn list_body(
        &self,
        panel: &ConfigPanel,
        s: &'static Strings,
        tokens: &Tokens,
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let seg = segmenter_of(cx);
        let marks = crate::widgets::picker::strings::of(cx);
        let gutter = EdgeInsets::symmetric(2, 0);
        let blank = || Text::new(String::new());
        let rows = self.visible(panel);
        let searching = self.searching || !self.query.is_empty();
        // The columns are as wide as the widest of every group's rows.
        let name_wide = panel
            .rows
            .iter()
            .map(|row| seg.width(&row.label))
            .max()
            .unwrap_or(0);
        let value_wide = panel
            .rows
            .iter()
            .map(|row| seg.width(&value_text(s, &row.value)))
            .max()
            .unwrap_or(0)
            .max(
                editors::choice_labels()
                    .map(|label| seg.width(&s.choice(&label).to_string()))
                    .max()
                    .unwrap_or(0),
            );
        let mut list = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, at) in rows.iter().enumerate() {
            let row = &panel.rows[*at];
            let on = index == self.cursor;
            let (name_style, value_style, description_style) = if on {
                (tokens.selected(), tokens.selected(), tokens.selected())
            } else if matches!(row.value, Value::Unavailable) {
                (tokens.dim(), tokens.dim(), tokens.dim())
            } else {
                (Style::new(), Style::new(), tokens.dim())
            };
            let marker = if on { s.cursor_marker } else { s.indent };
            let description = if searching {
                s.search_desc(
                    s.config_groups[Section::ALL
                        .iter()
                        .position(|section| *section == row.section)
                        .unwrap_or(0)],
                    row.description.as_str(),
                )
                .to_string()
            } else {
                row.description.clone()
            };
            let tap: Handler<PointerEvent> = cx.map(move |_: PointerEvent| Msg::Row(index));
            let line = Row::new()
                .child(SizedBox::new(
                    Some(i32::try_from(name_wide + 4).unwrap_or(0)),
                    Some(1),
                    Some(dots_end(format!("{marker}{}", row.label), name_style)),
                ))
                .child(SizedBox::new(
                    Some(i32::try_from(value_wide + 2).unwrap_or(0)),
                    Some(1),
                    Some(dots_end(value_text(s, &row.value), value_style)),
                ))
                .child(Expanded::new(1, dots_end(description, description_style)));
            list = list.child(GestureDetector::new().on_tap(tap).child(line));
        }
        if rows.is_empty() {
            list = list.child(Text::line(format!("{}{}", s.indent, s.no_matches)).style(tokens.dim()));
        }

        let labels: Rc<[String]> = s.config_groups.iter().map(|g| (*g).to_string()).collect();
        let taps = (0..labels.len())
            .map(|index| cx.map(move |_: PointerEvent| Msg::Tab(index)))
            .collect();
        let tabs = Tabs {
            labels,
            active: if searching { usize::MAX } else { self.section },
            marks: (marks.tab_left, marks.tab_right),
            styles: (tokens.selected(), Style::new()),
            taps,
        };
        let hint = hint_of(s, panel, rows.get(self.cursor).map(|at| &panel.rows[*at]), searching);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(Text::line(format!("{}{}", s.indent, s.config_title)).style(tokens.bold()))
            .child(Padding::new(
                gutter,
                Text::new(
                    panel
                        .notice
                        .as_ref()
                        .map_or_else(|| s.config_subtitle.to_string(), |n| notice_text(s, n)),
                )
                .style(tokens.dim())
                .wrap(TextWrap::Word),
            ))
            .child(blank())
            .child(Padding::new(EdgeInsets::symmetric(2, 0), tabs.widget()))
            .child(blank());
        if searching {
            column = column
                .child(Text::line(format!("{}{}", s.indent_search(), self.query)))
                .child(blank());
        }
        column
            .child(Padding::new(EdgeInsets::symmetric(0, 0), list))
            .child(blank())
            .child(Padding::new(
                gutter,
                Text::new(hint.to_string()).style(tokens.dim()).wrap(TextWrap::Word),
            ))
            .into()
    }
}

/// The footer for the row the cursor is on (`render.rs`: the hint names what Enter does).
fn hint_of(
    s: &Strings,
    _panel: &ConfigPanel,
    row: Option<&codex_tui_core::features::settings::Row>,
    searching: bool,
) -> &'static str {
    let kind = row.map(|row| &row.value);
    match (searching, kind) {
        (false, Some(Value::Toggle(_))) => s.config_hint_toggle,
        (false, Some(Value::Test)) => s.config_hint_test,
        (false, Some(Value::Choice(_) | Value::Configure)) => s.config_hint_open,
        (false, _) => s.config_hint_plain,
        (true, Some(Value::Toggle(_))) => s.config_search_hint_toggle,
        (true, Some(Value::Test)) => s.config_search_hint_test,
        (true, Some(Value::Choice(_) | Value::Configure)) => s.config_search_hint_open,
        (true, _) => s.config_search_hint_plain,
    }
}

impl Strings {
    /// The search line's lead: two blanks and the prompt.
    fn indent_search(&self) -> String {
        format!("{}{}", self.indent, self.config_search_line)
    }
}
