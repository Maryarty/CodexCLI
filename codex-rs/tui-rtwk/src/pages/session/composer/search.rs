//! Reverse history search (Ctrl+R) as a mode of the composer: while it is
//! open every key is the search's (`handle_history_search_key`), the preview
//! stands in the draft's place (`ComposerDraft::search_*`) and the footer
//! row is the query. Oracle: `bottom_pane/chat_composer/history_search.rs`.

use rtwk::editing::TextEditingController;
use rtwk::focus::KeyPress;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::prelude::Mods;
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::keymap::actions::{
    COMPOSER_HISTORY_SEARCH_NEXT, COMPOSER_HISTORY_SEARCH_PREVIOUS,
};
use codex_tui_core::kit::history::Lookup;
use codex_tui_core::screens::composer::sanitize_paste;
use codex_tui_core::kit::history::{Direction, SearchStatus};

use super::ComposerMsg;
use super::editing::KeyData;
use super::strings;
use crate::keymap::fixed::{Fixed, FixedKey};
use crate::app::theme;
use crate::keymap::pressed;

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
    let plain = key.mods == Mods::NONE;
    let ctrl = |ch: char| *key == KeyPress::char(ch).with_mods(Mods::CTRL);
    if pressed(&data.keymap, COMPOSER_HISTORY_SEARCH_PREVIOUS, key)
        || (key.fixed() == Fixed::Up && plain)
    {
        draft.search_step(c, store, Direction::Older, &mut asks);
    } else if pressed(&data.keymap, COMPOSER_HISTORY_SEARCH_NEXT, key)
        || (key.fixed() == Fixed::Down && plain)
    {
        draft.search_step(c, store, Direction::Newer, &mut asks);
    } else if key.fixed() == Fixed::Escape
        || ctrl('c')
        || (key.mods.is_empty() && key.text() == Some("\u{3}"))
    {
        draft.search_cancel(c);
    } else if key.fixed() == Fixed::Enter && plain {
        draft.search_accept(c);
    } else if key.fixed() == Fixed::Backspace || ctrl('h') {
        draft.search_edit(c, store, &mut asks, |query| {
            query.pop();
        });
    } else if ctrl('u') {
        draft.search_edit(c, store, &mut asks, String::clear);
    } else if let Some(text) = key.as_text() {
        draft.search_edit(c, store, &mut asks, |query| query.push_str(text));
    } else if key.fixed() == Fixed::F12 {
        // the port's own hidden debug key is not the oracle's to swallow
        return false;
    }
    ask(data, asks);
    (data.tell)(ComposerMsg::Searched);
    true
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
            .search_edit(c, &data.history, &mut asks, |query| query.push_str(&pasted));
    }
    ask(data, asks);
    (data.tell)(ComposerMsg::Searched);
}

/// The footer row while the search is open (`history_search_footer_line`):
/// the label dim, the query in the info colour, then by where the search
/// stands: nothing, `searching`, the two actions of a match (keys bold,
/// words dim) or `no match` in red; two cells in, as every footer row.
pub fn footer<S>(cx: &mut Cx<'_, S>, query: String, status: SearchStatus) -> AnyWidget {
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
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        // one row, clipped at the edge (a long query is not wrapped)
        .child(Expanded::new(
            1,
            Text::rich_line(spans).overflow(rtwk_core::unicode::TextOverflow::Clip),
        ))
        .into()
}
