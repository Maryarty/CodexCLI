//! Reverse history search (Ctrl+R) as a mode of the composer: while it is
//! open every key is the search's (`handle_history_search_key`), the preview
//! stands in the draft's place (`ComposerDraft::search_*`) and the footer
//! row is the query. Oracle: `bottom_pane/chat_composer/history_search.rs`.

use rtwk::editing::{StoredStops, TextEditingController, editing_key};
use rtwk::focus::KeyPress;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::data::history::Lookup;
use codex_tui_core::keymap::actions::{
    COMPOSER_HISTORY_SEARCH_NEXT, COMPOSER_HISTORY_SEARCH_PREVIOUS,
};
use codex_tui_core::kit::history::{SearchStatus, WalkDirection};
use codex_tui_core::kit::vim::VimDirection;
use codex_tui_core::screens::composer::sanitize_paste;

use super::ComposerMsg;
use super::editing::KeyData;
use super::strings;
use crate::app::theme;
use crate::keymap::{press_for, pressed, unmodified};
use crate::widgets::composer_draft::ComposerDraft;
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
    let draft = &data.draft;
    let store = &data.history;
    let mut asks = Vec::new();
    let plain = unmodified(key);
    if pressed(&data.keymap, COMPOSER_HISTORY_SEARCH_PREVIOUS, key)
        || (pressed(&data.keymap, HISTORY_PREVIOUS, key) && plain)
    {
        draft.search_step(c, store, WalkDirection::Older, &mut asks);
    } else if pressed(&data.keymap, COMPOSER_HISTORY_SEARCH_NEXT, key)
        || (pressed(&data.keymap, HISTORY_NEXT, key) && plain)
    {
        draft.search_step(c, store, WalkDirection::Newer, &mut asks);
    } else if pressed(&data.keymap, SEARCH_CANCEL, key)
        || pressed(&data.keymap, SEARCH_CANCEL_BYTE, key)
    {
        draft.search_cancel(c);
    } else if pressed(&data.keymap, SEARCH_ACCEPT, key) && plain {
        draft.search_accept(c);
    } else if pressed(&data.keymap, SEARCH_ERASE, key) {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, |q| {
                // the key the erase is, as the field's own editing reads it
                if let Some(erase) = press_for(SEARCH_ERASE) {
                    editing_key(q, erase, &|t| q.insert(t), &StoredStops, false, None);
                }
            });
        });
    } else if pressed(&data.keymap, SEARCH_CLEAR, key) {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, TextEditingController::clear)
        });
    } else if let Some(text) = key.as_text() {
        draft.search_edit(c, store, &mut asks, |query| {
            edited(query, |q| q.insert(text))
        });
    } else if pressed(&data.keymap, DEBUG_KEY, key) {
        // the port's own hidden debug key is not the oracle's to swallow
        return false;
    }
    ask(data, asks);
    (data.tell)(ComposerMsg::Searched);
    true
}

/// The footer query after `edit` did its work to it as a one-line field's text (the caret at its
/// end: Backspace takes the last character, a typed one or a paste is added there): the query
/// is a field's text, edited by the field's keys, never by hand.
fn edited(query: &mut String, edit: impl FnOnce(&TextEditingController)) {
    let field = TextEditingController::new(query);
    edit(&field);
    *query = field.text().to_string();
}

/// The reads the search needs go to the app.
fn ask(data: &KeyData, asks: Vec<Lookup>) {
    if !asks.is_empty() {
        (data.tell)(ComposerMsg::Lookups(asks));
    }
}

/// A paste while the search is open goes into the query, in full and
/// sanitized; an empty one changes nothing (`apply_paste`).
pub fn paste(data: &KeyData, c: &TextEditingController, text: &str) {
    let pasted = sanitize_paste(text);
    let mut asks = Vec::new();
    if !pasted.is_empty() {
        data.draft
            .search_edit(c, &data.history, &mut asks, |query| {
                edited(query, |q| q.insert(&pasted));
            });
    }
    ask(data, asks);
    (data.tell)(ComposerMsg::Searched);
}

/// The footer row while the search is open (`history_search_footer_line`):
/// the label dim, the query in the info colour, then by where the search
/// stands: nothing, `searching`, the two actions of a match (keys bold,
/// words dim) or `no match` in red; two cells in, as every footer row.
pub fn footer<S>(cx: &mut Cx<'_, S>, query: String, status: SearchStatus) -> AnyWidget {
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        // one row, clipped at the edge (a long query is not wrapped)
        .child(Expanded::new(1, line(cx, query, status)))
        .into()
}

/// The row of [`footer`] without its two cells of indent: where the owner of the field draws it
/// under the field's own text (the inline editor of the async questions).
pub fn line<S>(cx: &mut Cx<'_, S>, query: String, status: SearchStatus) -> AnyWidget {
    let tokens = theme::Tokens::of(cx);
    let strings = strings::of(cx);
    let dim = tokens.dim().attrs(Attrs::DIM);
    let mut spans = vec![
        Span::styled(strings.search_label, dim.clone()),
        Span::styled(query, tokens.info()),
    ];
    match status {
        SearchStatus::Idle => {}
        SearchStatus::Searching => spans.push(Span::styled(strings.search_searching, dim)),
        SearchStatus::Match => spans.extend([
            Span::styled(strings.search_gap, dim.clone()),
            Span::styled(strings.search_accept_key, tokens.bold()),
            Span::styled(strings.search_accept, dim.clone()),
            Span::styled(strings.search_between, dim.clone()),
            Span::styled(strings.search_cancel_key, tokens.bold()),
            Span::styled(strings.search_cancel, dim),
        ]),
        SearchStatus::NoMatch => spans.push(Span::styled(strings.search_no_match, tokens.error())),
    }
    Text::rich_line(spans)
        .overflow(rtwk_core::unicode::TextOverflow::Clip)
        .into()
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
