//! The footer of Find (oracle `transcript_view/search_footer.rs`): the query
//! row (`Find: ` and the editor) and the status row under it, which takes the
//! first of its spellings that fits.
//!
//! The state the status is read from is the finder's (`Status`: where the
//! search stands, whether the reader is on a match and reading it, whether the
//! query was cut); this file only turns it into words. Flutter: a `Row` of a
//! `Text` and an `Expanded` field, and a `LayoutBuilder` + `segmenter_of` that
//! picks the spelling (as `follow.rs`).

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::{EditingKeys, TextField};
use rtwk::focus::{Focus, FocusNode};
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::ThreadId;

use super::find::{self, Finder, Mode};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;

/// Where the search stands (oracle `search.rs` `Progress`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Nothing asked since the query last changed, or the answer is shown.
    Idle,
    /// The query changed: the scan starts again.
    Restart,
    /// Entries are being read.
    Scanning,
    /// The match lies in history that has not been loaded.
    AwaitingHistory,
    /// A match is shown.
    Found,
    /// No entry is left to read in the direction.
    Exhausted,
}

/// What the status row is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub progress: Progress,
    /// The reader has accepted the match and reads it (the footer says `Find · …`).
    pub reading: bool,
    /// A match is the current one.
    pub has_match: bool,
    /// The query editor is empty.
    pub query_empty: bool,
    /// The query was cut to 4 KiB.
    pub truncated: bool,
    /// Loading older history failed.
    pub history_failed: bool,
}

impl Status {
    /// A finder that has not been asked anything.
    pub fn idle() -> Status {
        Status {
            progress: Progress::Idle,
            reading: false,
            has_match: false,
            query_empty: true,
            truncated: false,
            history_failed: false,
        }
    }
}

/// The spellings of the status row, the widest first (`status_line`).
pub fn candidates(table: &Strings, status: &Status) -> Vec<String> {
    let join = table.hint_joiner;
    let key_pair = |older: &str, newer: &str| {
        format!(
            "{older} {}{join}{newer} {}",
            table.find_older, table.find_newer
        )
    };
    let retry = format!("{} {}", table.find_key_older, table.find_retry);
    let unavailable = format!("{}{join}{retry}", table.find_history_unavailable);
    let next = format!(
        "{}{join}{}",
        table.find_accept,
        key_pair(table.find_key_older, table.find_key_newer)
    );
    let exhausted = format!("{}{join}{next}", table.find_no_more);
    let plain_older = format!("{} {}", table.find_plain_older, table.find_older);
    let plain = key_pair(table.find_plain_older, table.find_plain_newer);
    let reading_exhausted = format!("{}{join}{plain}", table.find_no_more);
    let (text, compact): (&str, &str) = match status.progress {
        Progress::Idle | Progress::Found if status.reading => (&plain, &plain_older),
        Progress::Idle if !status.query_empty => (&next, table.find_accept),
        Progress::Idle => (table.find_type, table.find_type),
        Progress::Restart | Progress::Scanning => (table.find_searching, table.find_searching),
        Progress::AwaitingHistory if status.history_failed => (&unavailable, &retry),
        Progress::AwaitingHistory => (table.find_searching_history, table.find_loading),
        Progress::Found => (&next, table.find_accept),
        Progress::Exhausted if status.reading && status.has_match => {
            (&reading_exhausted, &plain_older)
        }
        Progress::Exhausted if status.has_match => (&exhausted, table.find_accept),
        Progress::Exhausted => (table.find_no_matches, table.find_no_matches),
    };
    if status.reading {
        let find = table.find_label;
        let latest = table.find_latest;
        return vec![
            format!("{find}{join}{text}{join}{latest}"),
            format!("{find}{join}{compact}{join}{latest}"),
            latest.to_owned(),
        ];
    }
    let limit = if status.truncated {
        format!("{join}{}", table.find_limited)
    } else {
        String::new()
    };
    let cancel = table.find_cancel;
    vec![
        format!(
            "{text}{join}{}{join}{cancel}{limit}",
            table.find_full_transcript
        ),
        format!("{text}{join}{cancel}{limit}"),
        format!("{compact}{join}{cancel}"),
        format!("{compact}{join}{}", table.key_esc),
        cancel.to_owned(),
    ]
}

/// The status line as spans (`navigation_line`): the key of a hint bold and its action dim, the
/// plain words dim, `esc` alone bold.
pub fn navigation_spans(table: &Strings, line: &str, key: &Style, dim: &Style) -> Vec<Span> {
    let actions = [
        table.find_older,
        table.find_newer,
        table.find_retry,
        table.find_accept.rsplit(' ').next().unwrap_or_default(),
        table.find_cancel.rsplit(' ').next().unwrap_or_default(),
        table.find_latest.rsplit(' ').next().unwrap_or_default(),
    ];
    let mut spans = Vec::new();
    for (index, hint) in line.split(table.hint_joiner).enumerate() {
        if index > 0 {
            spans.push(Span::styled(table.hint_joiner, dim.clone()));
        }
        let action = actions
            .iter()
            .filter(|action| !action.is_empty())
            .find(|action| {
                hint.len() > action.len() + 1
                    && hint.ends_with(**action)
                    && hint[..hint.len() - action.len()].ends_with(' ')
            });
        match action {
            Some(action) => {
                let split = hint.len() - action.len() - 1;
                spans.push(Span::styled(hint[..split].to_owned(), key.clone()));
                spans.push(Span::styled(hint[split..].to_owned(), dim.clone()));
            }
            None if hint == table.key_esc => spans.push(Span::styled(hint.to_owned(), key.clone())),
            None => spans.push(Span::styled(hint.to_owned(), dim.clone())),
        }
    }
    spans
}

/// The status row: the first spelling that fits the width, clipped, empty when none does.
pub fn status_row(table: &'static Strings, status: Status, key: Style, dim: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let line = candidates(table, &status)
            .into_iter()
            .find(|line| {
                i32::try_from(segmenter.width(line)).unwrap_or(i32::MAX) <= constraints.max_width
            })
            .unwrap_or_default();
        Text::rich_line(navigation_spans(table, &line, &key, &dim))
            .overflow(TextOverflow::Clip)
            .into()
    })
    .into()
}

/// The query row: the dim label and the editor after it, which takes the rest.
pub fn query_row(table: &'static Strings, label: Style, editor: AnyWidget) -> AnyWidget {
    Row::new()
        .child(Text::new(table.find_prefix).style(label))
        .child(Expanded::new(1, editor))
        .into()
}

/// The footer under the composer: while Find is open its query row (editing only) and its status
/// row, two cells in, in the place of `normal`, the footer the page would draw (Flutter: a
/// `ListenableBuilder` on the finder). One call from the page's footer slot.
pub fn slot<S>(cx: &mut Cx<'_, S>, thread: &ThreadId, normal: AnyWidget) -> AnyWidget {
    let table = strings::of(cx);
    let tokens = Tokens::of(cx);
    let (dim, bold) = (tokens.dim(), tokens.bold());
    let finder = find::finder(thread);
    ListenableBuilder::new(finder.listenable(), move |_, _| {
        let mode = finder.mode();
        if mode == Mode::Closed {
            return normal.clone();
        }
        let mut rows = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if mode == Mode::Editing {
            rows = rows.child(row(QueryField::new(&finder, table, dim.clone()).into()));
        }
        rows.child(row(status_row(
            table,
            finder.status(),
            bold.clone(),
            dim.clone(),
        )))
        .into()
    })
    .into()
}

/// A footer row two cells in.
fn row(child: AnyWidget) -> AnyWidget {
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        .child(Expanded::new(1, child))
        .into()
}

/// The query row: `Find: ` and the field that edits the finder's query. The field takes the
/// focus as it appears and gives it back, to whoever had it, as it goes (the composer: Flutter's
/// `FocusScope` of a route returning focus to what held it).
#[derive(Clone)]
struct QueryField {
    finder: Finder,
    table: &'static Strings,
    label: Style,
}

impl QueryField {
    fn new(finder: &Finder, table: &'static Strings, label: Style) -> QueryField {
        QueryField {
            finder: finder.clone(),
            table,
            label,
        }
    }
}

impl PartialEq for QueryField {
    fn eq(&self, other: &QueryField) -> bool {
        self.finder == other.finder && self.label == other.label
    }
}

/// What the field's state is told: its text changed, it is to take the focus.
enum Edit {
    Text,
    Focus,
}

struct QueryState {
    back: Option<FocusNode>,
}

impl StatefulWidget for QueryField {
    type State = QueryState;

    fn create_state(&self) -> QueryState {
        QueryState { back: None }
    }
}

impl_widget!(stateful eq QueryField);

impl State for QueryState {
    type Widget = QueryField;
    type Msg = Edit;

    fn init_state(&mut self, _: &QueryField, cx: &mut Cx<'_, Self>) {
        self.back = Focus::maybe_of(cx, false)
            .and_then(|node| node.manager())
            .and_then(|manager| manager.primary_focus());
        // a field that mounts takes the focus only where its scope holds none, and the composer
        // holds it: asked once the frame that built the field is done (Flutter:
        // `addPostFrameCallback(requestFocus)`)
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Edit::Focus));
        }
    }

    fn dispose(&mut self) {
        FocusNode::give_back(self.back.as_ref(), None);
    }

    fn update(&mut self, msg: Edit, w: &QueryField, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Edit::Text => w.finder.changed(),
            Edit::Focus => w.finder.node().request_focus(),
        }
        Rebuild::No
    }

    fn build(&self, w: &QueryField, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let finder = w.finder.clone();
        let field = TextField::new()
            .controller(finder.query())
            .focus_node(finder.node())
            .autofocus(true)
            .key_hints(false)
            .on_changed(cx.map(|_: Rc<str>| Edit::Text))
            .on_paste({
                let finder = finder.clone();
                move |text| {
                    finder.paste(text);
                    true
                }
            });
        let keys = finder.clone();
        let field = EditingKeys::new(
            move |_, key, _| find::key(keys.mode(), key).is_some_and(|taken| keys.take(taken)),
            field,
        )
        .controller(finder.query());
        query_row(w.table, w.label.clone(), field.into())
    }
}
