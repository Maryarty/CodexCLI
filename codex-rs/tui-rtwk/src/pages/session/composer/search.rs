//! Reverse history search (Ctrl+R) as a mode of the composer: while it is
//! open every key is the search's (`handle_history_search_key`), the preview
//! stands in the draft's place (`ComposerDraft::search_*`) and the footer
//! row is the query. Oracle: `bottom_pane/chat_composer/history_search.rs`.

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::{EditingKeys, StoredStops, TextEditingController, TextField, editing_key};
use rtwk::focus::{FocusNode, KeyPress};
use rtwk::gestures::IgnorePointer;
use rtwk::layout::{
    BoxConstraints, ConstrainedBox, Expanded, Flexible, IntrinsicWidth, Row, SizedBox,
};
use rtwk::provider::select;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::presenter::CursorShape;
use rtwk_core::unicode::TextOverflow;

use codex_tui_core::data::history::{Lookup, Store};
use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{
    COMPOSER_HISTORY_SEARCH_NEXT, COMPOSER_HISTORY_SEARCH_PREVIOUS,
};
use codex_tui_core::kit::history::{SearchStatus, WalkDirection};
use codex_tui_core::kit::vim::VimDirection;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::composer::sanitize_paste;

use super::ComposerMsg;
use super::editing::{BeforeSearch, KeyData, before_search_key, note_keyboard_activity};
use super::strings;
use crate::app::theme;
use crate::keymap::{press_for, pressed, unmodified};
use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::thread_ui::ThreadUi;
use codex_tui_core::keymap::fixed::{
    DEBUG_KEY, HISTORY_NEXT, HISTORY_PREVIOUS, SEARCH_ACCEPT, SEARCH_CANCEL, SEARCH_CANCEL_BYTE,
    SEARCH_CLEAR, SEARCH_ERASE,
};

/// Whether `key` opens the search (`is_history_search_key`): only the
/// Composer context's `history_search_previous` does.
pub fn opens(data: &KeyData, key: &KeyPress) -> bool {
    pressed(&data.keymap, COMPOSER_HISTORY_SEARCH_PREVIOUS, key)
}

/// Ctrl+R: the search opens (`begin_history_search`).
pub fn begin(data: &KeyData, c: &TextEditingController) {
    data.draft.search_begin(c);
    (data.tell)(ComposerMsg::Searched);
}

/// What a key does while the search is open: the bound keys and Up, Down
/// step; Esc and Ctrl+C cancel; Enter accepts a match; Backspace (and
/// Ctrl+H) and Ctrl+U edit the query, a character extends it. Every other
/// key is consumed (`_ => (InputResult::None, true)`).
pub fn key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    let Some(asks) = take_key(&data.draft, c, &data.history, &data.keymap, key) else {
        return false;
    };
    tell(data, asks);
    true
}

fn take_key(
    draft: &ComposerDraft,
    c: &TextEditingController,
    store: &Store,
    keymap: &Keymap,
    key: &KeyPress,
) -> Option<Vec<Lookup>> {
    let mut asks = Vec::new();
    let plain = unmodified(key);
    if pressed(keymap, COMPOSER_HISTORY_SEARCH_PREVIOUS, key)
        || (pressed(keymap, HISTORY_PREVIOUS, key) && plain)
    {
        draft.search_step(c, store, WalkDirection::Older, &mut asks);
    } else if pressed(keymap, COMPOSER_HISTORY_SEARCH_NEXT, key)
        || (pressed(keymap, HISTORY_NEXT, key) && plain)
    {
        draft.search_step(c, store, WalkDirection::Newer, &mut asks);
    } else if pressed(keymap, SEARCH_CANCEL, key) || pressed(keymap, SEARCH_CANCEL_BYTE, key) {
        draft.search_cancel(c);
    } else if pressed(keymap, SEARCH_ACCEPT, key) && plain {
        draft.search_accept(c);
    } else if pressed(keymap, SEARCH_ERASE, key) {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, |q| {
                // the key the erase is, as the field's own editing reads it
                if let Some(erase) = press_for(SEARCH_ERASE) {
                    editing_key(q, erase, &|t| q.insert(t), &StoredStops, false, None);
                }
            });
        });
    } else if pressed(keymap, SEARCH_CLEAR, key) {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, TextEditingController::clear)
        });
    } else if let Some(text) = key.as_text() {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, |q| q.insert(text))
        });
    } else if pressed(keymap, DEBUG_KEY, key) {
        // the port's own hidden debug key is not the oracle's to swallow
        return None;
    }
    Some(asks)
}

/// The footer query after `edit` did its work to it as a one-line field's text (the caret at its
/// end: Backspace takes the last character, a typed one or a paste is added there): the query
/// is a field's text, edited by the field's keys, never by hand.
fn edited(query: &mut String, edit: impl FnOnce(&TextEditingController)) {
    let field = TextEditingController::new(query);
    edit(&field);
    *query = field.text().to_string();
}

fn tell(data: &KeyData, asks: Vec<Lookup>) {
    if !asks.is_empty() {
        (data.tell)(ComposerMsg::Lookups(asks));
    }
    (data.tell)(ComposerMsg::Searched);
}

/// The same paste behavior as the footer for a composer that cannot show its query.
pub fn paste(data: &KeyData, c: &TextEditingController, text: &str) {
    tell(data, paste_query(&data.draft, c, &data.history, text));
}

fn paste_query(
    draft: &ComposerDraft,
    c: &TextEditingController,
    store: &Store,
    text: &str,
) -> Vec<Lookup> {
    let pasted = sanitize_paste(text);
    let mut asks = Vec::new();
    if !pasted.is_empty() {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, |q| q.insert(&pasted));
        });
    }
    asks
}

/// The footer in the chat or the composer alone, with the standard two-cell inset.
pub fn footer(
    ui: &ThreadUi,
    back: FocusNode,
    on_changed: Handler<Rc<str>>,
    query: String,
    status: SearchStatus,
) -> AnyWidget {
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        .child(Expanded::new(1, line(ui, back, on_changed, query, status)))
        .into()
}

/// The same query field without the inset, for an inline question.
pub fn line(
    ui: &ThreadUi,
    back: FocusNode,
    on_changed: Handler<Rc<str>>,
    query: String,
    status: SearchStatus,
) -> AnyWidget {
    HistoryField {
        draft: ui.draft.clone(),
        preview: ui.composer.clone(),
        back,
        on_changed,
        query,
        status,
    }
    .into()
}

#[derive(Clone, PartialEq)]
struct HistoryField {
    draft: ComposerDraft,
    preview: TextEditingController,
    back: FocusNode,
    on_changed: Handler<Rc<str>>,
    query: String,
    status: SearchStatus,
}

struct HistoryFieldState {
    node: FocusNode,
    // The raw query belongs to HistorySearch. This controller displays its one-line projection,
    // with newline and tab marks, and gives the focused field its native caret.
    visible: TextEditingController,
}

enum Edit {
    Focus,
    BeforeSearch(BeforeSearch),
    Changed(Vec<Lookup>),
}

impl StatefulWidget for HistoryField {
    type State = HistoryFieldState;

    fn create_state(&self) -> HistoryFieldState {
        HistoryFieldState {
            node: FocusNode::new(),
            visible: TextEditingController::new(&self.query),
        }
    }
}

impl_widget!(stateful eq HistoryField);

impl State for HistoryFieldState {
    type Widget = HistoryField;
    type Msg = Edit;

    fn init_state(&mut self, _: &HistoryField, cx: &mut Cx<'_, Self>) {
        // Autofocus only fills an empty scope. The preview still owns focus as the footer mounts.
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Edit::Focus));
        }
    }

    fn did_update_widget(&mut self, old: &HistoryField, w: &HistoryField, _: &mut Cx<'_, Self>) {
        if old.query != w.query {
            self.visible.set_text(&w.query);
        }
    }

    fn update(&mut self, msg: Edit, w: &HistoryField, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Edit::Focus if w.draft.searching() && w.back.has_primary_focus() => {
                self.node.request_focus();
            }
            Edit::Focus => {}
            Edit::BeforeSearch(action) => {
                if matches!(action, BeforeSearch::VimToggled(_)) {
                    cx.emit(&w.on_changed, w.preview.text());
                }
                return super::apply_before_search(cx, action);
            }
            Edit::Changed(lookups) => {
                super::ask(cx, lookups);
                if !w.draft.searching() && self.node.has_primary_focus() {
                    w.back.request_focus();
                }
                cx.emit(&w.on_changed, w.preview.text());
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &HistoryField, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (history, clipboard_answers) = select::<ScreenStates, _, _>(cx, |states| {
            (
                states.popovers.history.clone(),
                states.popovers.pasted.answers(),
            )
        });
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let keymap = crate::keymap::keymap(cx);
        let send = cx.local_sender();
        let draft = w.draft.clone();
        let preview = w.preview.clone();
        let tokens = theme::Tokens::of(cx);
        let field = TextField::new()
            .controller(self.visible.clone())
            .focus_node(self.node.clone())
            .autofocus(true)
            .key_hints(false)
            .cursor_shape(CursorShape::TerminalDefault)
            .style(tokens.info())
            .focused_style(tokens.info())
            .on_paste({
                let draft = draft.clone();
                let preview = preview.clone();
                let history = history.clone();
                let send = send.clone();
                move |text| {
                    send(Edit::Changed(paste_query(&draft, &preview, &history, text)));
                    true
                }
            });
        let keys = EditingKeys::new(
            move |_, key, _| {
                let now = || clock.as_ref().map(|clock| clock.now());
                note_keyboard_activity(&draft, key, now);
                if before_search_key(
                    &draft,
                    &keymap,
                    key,
                    || now().map(|time| (time, clipboard_answers)),
                    |action| send(Edit::BeforeSearch(action)),
                ) {
                    return true;
                }
                let Some(lookups) = take_key(&draft, &preview, &history, &keymap, key) else {
                    return false;
                };
                send(Edit::Changed(lookups));
                true
            },
            field,
        )
        .controller(self.visible.clone());
        let field: AnyWidget = IgnorePointer::new().child(keys).into();
        let table = strings::of(cx);
        let dim = tokens.dim().attrs(Attrs::DIM);
        let suffix = status_line(table, w.status, &tokens, &dim);
        LayoutBuilder::new(move |cx, constraints| {
            let label = table.search_label;
            let width = i32::try_from(segmenter_of(cx).width(label)).unwrap_or(i32::MAX);
            let label_width = width.min(constraints.max_width.saturating_sub(1).max(0));
            Row::new()
                .child(ConstrainedBox::new(
                    BoxConstraints::new(0, label_width, 1, 1),
                    Text::line(label)
                        .style(dim.clone())
                        .overflow(TextOverflow::Clip),
                ))
                .child(ConstrainedBox::new(
                    BoxConstraints::new(0, constraints.max_width - label_width, 1, 1),
                    IntrinsicWidth::new(field.clone()),
                ))
                .child(Flexible::new(1, suffix.clone()))
                .into()
        })
        .into()
    }
}

fn status_line(
    strings: &strings::Strings,
    status: SearchStatus,
    tokens: &theme::Tokens,
    dim: &Style,
) -> AnyWidget {
    // The input field already measures a cell for its caret before the status.
    let after_caret = |s: &'static str| s.strip_prefix(' ').unwrap_or(s);
    let spans = match status {
        SearchStatus::Idle => Vec::new(),
        SearchStatus::Searching => vec![Span::styled(
            after_caret(strings.search_searching),
            dim.clone(),
        )],
        SearchStatus::Match => vec![
            Span::styled(after_caret(strings.search_gap), dim.clone()),
            Span::styled(strings.search_accept_key, tokens.bold()),
            Span::styled(strings.search_accept, dim.clone()),
            Span::styled(strings.search_between, dim.clone()),
            Span::styled(strings.search_cancel_key, tokens.bold()),
            Span::styled(strings.search_cancel, dim.clone()),
        ],
        SearchStatus::NoMatch => vec![Span::styled(
            after_caret(strings.search_no_match),
            tokens.error(),
        )],
    };
    Text::rich_line(spans).overflow(TextOverflow::Clip).into()
}

/// The row of the `/` or `?` query while Normal mode types one (`SearchInput::render`): the
/// direction's mark in cyan, then the typed text, in the footer's place; `None` with no query
/// open.
pub fn query_footer<S>(cx: &mut Cx<'_, S>, draft: &ComposerDraft) -> Option<AnyWidget> {
    let line = query_line(cx, draft)?;
    Some(
        Row::new()
            .child(SizedBox::new(Some(2), None, None))
            .child(Expanded::new(1, line))
            .into(),
    )
}

/// The row of [`query_footer`] without its two cells of indent.
pub fn query_line<S>(cx: &mut Cx<'_, S>, draft: &ComposerDraft) -> Option<AnyWidget> {
    let direction = draft.with_vim(|v| v.searching())?;
    let tokens = theme::Tokens::of(cx);
    let mark = match direction {
        VimDirection::Forward => "/",
        VimDirection::Backward => "?",
    };
    let spans = vec![
        Span::styled(mark, tokens.info()),
        Span::styled(draft.vim_query().text().to_string(), Style::default()),
    ];
    Some(
        Text::rich_line(spans)
            .overflow(rtwk_core::unicode::TextOverflow::Clip)
            .into(),
    )
}
