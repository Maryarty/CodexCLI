//! `/review` in the selection sheet (oracle `chatwidget/review_popups.rs`): the presets, the base
//! branches and the commits are lists of the shared picker (the rows are core's
//! `review::Menu::rows`, the words are `review/strings.rs`), the custom instructions are a text
//! prompt of their own (`CustomPromptView` on the shared-picker layout). In Flutter the lists are
//! the `showModalBottomSheet` builders over a `ListView`, the prompt a `TextField` over a
//! `TextEditingController` in the same sheet.

use rtwk::editing::{TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::list::{Columns, Item, List};
use codex_tui_core::screens::select::review::{self, Menu};

use crate::app::theme::Tokens;

pub mod strings;

use self::strings::Strings;

/// The list a review menu is shown as; none for the instructions (a prompt, `Instructions`).
/// `cursor` is the row of the menu that opened the one above it, shown again by Esc.
pub fn list(menu: &Menu, cursor: Option<&str>, s: &Strings) -> Option<List> {
    let rows = menu.rows();
    let initial = cursor.and_then(|id| rows.iter().position(|row| row.id == id));
    Some(match menu {
        Menu::Presets => List {
            title: Some(s.presets_title.to_string()),
            standard_hint: true,
            items: rows.iter().map(|row| preset(&row.id, s)).collect(),
            initial,
            panel: true,
            columns: Columns::AutoAllRows,
            hide_description_below: Some(24),
            ..List::default()
        },
        Menu::Branches { current, .. } => List {
            title: Some(s.branches_title.to_string()),
            subtitle: Some(s.current_branch(current.as_deref())),
            standard_hint: true,
            items: rows.iter().map(named).collect(),
            initial,
            searchable: true,
            search_placeholder: Some(s.branches_search.to_string()),
            panel: true,
            ..List::default()
        },
        Menu::Commits(_) => List {
            title: Some(s.commits_title.to_string()),
            standard_hint: true,
            items: rows.iter().map(named).collect(),
            initial,
            searchable: true,
            search_placeholder: Some(s.commits_search.to_string()),
            panel: true,
            ..List::default()
        },
        Menu::Instructions => return None,
    })
}

/// A row of the presets: its name by id, `(PR Style)` after the first.
fn preset(id: &str, s: &Strings) -> Item {
    let (name, description) = match id {
        review::BASE => (s.base_name, Some(s.base_description.to_string())),
        review::UNCOMMITTED => (s.uncommitted_name, None),
        review::COMMIT => (s.commit_name, None),
        _ => (s.custom_name, None),
    };
    Item {
        id: id.to_string(),
        name: name.to_string(),
        description,
        ..Item::default()
    }
}

/// A row of the branches or of the commits: the model's own words, searched by its value.
fn named(row: &review::Row) -> Item {
    Item {
        id: row.id.clone(),
        name: row.name.clone().unwrap_or_default(),
        search_value: row.search.clone(),
        ..Item::default()
    }
}

/// What the instructions prompt answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Enter on a text that is not blank (the model trims it).
    Submit(String),
    /// Esc: back to the presets.
    Back,
}

/// The text prompt of `Custom review instructions`: a title, the input under it behind the `›`
/// mark, the hint under both (the oracle's panel layout of `CustomPromptView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Instructions {
    pub on_answer: Handler<Answer>,
}

pub struct InstructionsState {
    text: TextEditingController,
    node: FocusNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Submit(std::rc::Rc<str>),
    Back,
}

impl StatefulWidget for Instructions {
    type State = InstructionsState;

    fn create_state(&self) -> InstructionsState {
        InstructionsState {
            text: TextEditingController::new(""),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Instructions);

impl State for InstructionsState {
    type Widget = Instructions;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.text.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Instructions, cx: &mut Cx<'_, Self>) -> Rebuild {
        let answer = match msg {
            Msg::Back => Answer::Back,
            // A blank text is not an answer: the prompt stays open.
            Msg::Submit(text) if text.trim().is_empty() => return Rebuild::No,
            Msg::Submit(text) => Answer::Submit(text.trim().to_string()),
        };
        cx.emit(&w.on_answer, answer);
        Rebuild::No
    }

    fn build(&self, _: &Instructions, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let gap = || SizedBox::new(None, Some(1), None);
        let input = TextArea::new()
            .controller(self.text.clone())
            .placeholder(s.instructions_placeholder)
            .placeholder_style(tokens.dim())
            .min_lines(1)
            .max_lines(8)
            .focus_node(self.node.clone())
            .autofocus(true)
            .on_submitted(cx.map(Msg::Submit));
        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::only(2, 0, 0, 0), child).into()
        };
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(gap())
            .child(Padding::new(
                EdgeInsets::symmetric(2, 0),
                Text::new(s.instructions_title)
                    .style(tokens.bold())
                    .wrap(TextWrap::Word),
            ))
            .child(gap())
            .child(
                Row::new()
                    .child(Text::line(s.prompt_mark).style(tokens.accent()))
                    .child(SizedBox::new(Some(1), None, None))
                    .child(Expanded::new(1, input))
                    .child(SizedBox::new(Some(2), None, None)),
            )
            .child(gap())
            .child(inset(
                Text::line(s.instructions_hint).style(tokens.dim()).into(),
            ));
        // The text area leaves Esc to the nodes above it.
        Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Back))
        .into()
    }
}
