//! What the transcript's status row says of the thread's history and of the news (oracle
//! `transcript_view/footer.rs`: `status_line_with_navigation`, the fallbacks of `footer`): while
//! an older page is on its way, that it is; after a page was refused, how to ask again; while
//! older messages can be loaded, that they can; then the key that goes on from here.
//!
//! The chat's footer (`follow::footer`) and the sheet's status row (`sheet::status`) choose among
//! the same spellings; each asks `candidates` for them, widest first, and takes the first that
//! fits its row.

use rtwk::Cx;
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};

use codex_tui_core::threads::{HistoryState, ThreadModel};

use crate::app::view::Hs;
use crate::widgets::thread_ui::Follow;

use super::strings::Strings;

/// Ctrl+Home, Alt+<, Home of the sheet (`jump_to_beginning`): the first row, or, with older pages
/// to come, the wait for it: the view holds while each page asks for the next.
pub fn to_beginning<S: 'static>(cx: &mut Cx<'_, S>, follow: &Follow) {
    let thread = read::<ThreadModel, _>(cx);
    match thread.history_state() {
        HistoryState::Complete => follow.jump_to_first(),
        HistoryState::Loading => follow.begin(),
        HistoryState::Partial | HistoryState::Failed => {
            follow.begin();
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.load_history, thread.id().clone());
        }
    }
}

/// The oracle's dim words of a status row: the DIM modifier on the terminal's own colours
/// (`Stylize::dim`), not the theme's grey.
pub fn plain_dim() -> Style {
    Style::new().attrs(Attrs::DIM)
}

/// The three styles of the row.
#[derive(Clone)]
pub struct Look {
    /// The arrow of a page on its way.
    pub accent: Style,
    pub dim: Style,
    /// The key labels.
    pub key: Style,
}

/// What the row has to say.
pub struct Say<'a> {
    pub history: HistoryState,
    /// New rows came in while the reader was away.
    pub news: bool,
    /// The hint that closes the row (the way back, the keys of a selection): styled spans, none
    /// for none.
    pub navigation: &'a [Span],
    /// Something is selected: the hint is the selection's, and a refused page names its retry
    /// after it.
    pub selected: bool,
}

/// An older page is on its way.
pub fn loading(history: HistoryState) -> bool {
    history == HistoryState::Loading
}

/// The history has something to say of itself before the news is a reason to show the row: a
/// page on its way or a refused one.
pub fn pending(history: HistoryState) -> bool {
    matches!(history, HistoryState::Loading | HistoryState::Failed)
}

fn dim(look: &Look, text: &'static str) -> Span {
    Span::styled(text, look.dim.clone())
}

fn key(look: &Look, text: &'static str) -> Span {
    Span::styled(text, look.key.clone())
}

/// The whole row (`status_line_with_navigation`).
pub fn whole(table: &Strings, look: &Look, say: &Say<'_>) -> Vec<Span> {
    let mut spans = Vec::new();
    if loading(say.history) {
        spans.push(Span::styled(table.loading_arrow, look.accent.clone()));
        spans.push(Span::new(table.history_loading));
        if say.news {
            spans.push(dim(look, table.loading_news));
        }
        if !say.navigation.is_empty() {
            spans.push(dim(look, table.footer_joiner));
            spans.extend(say.navigation.iter().cloned());
        }
        return spans;
    }
    if say.selected && say.history == HistoryState::Failed {
        spans.extend(say.navigation.iter().cloned());
        spans.push(dim(look, table.retry_selected));
        spans.push(key(look, table.begin_keys));
        return spans;
    }
    let history = match say.history {
        HistoryState::Failed => vec![
            dim(look, table.history_retry),
            key(look, table.begin_keys),
            dim(look, table.history_retry_end),
        ],
        HistoryState::Partial => vec![dim(look, table.history_partial)],
        HistoryState::Complete | HistoryState::Loading => Vec::new(),
    };
    if say.news {
        spans.push(dim(look, table.new_activity));
        if !history.is_empty() || !say.navigation.is_empty() {
            spans.push(dim(look, table.footer_joiner));
        }
    }
    spans.extend(history);
    spans.extend(say.navigation.iter().cloned());
    spans
}

/// The row when the whole one is too wide: the page on its way or the retry in short, else the
/// news in short, else the keys alone (`footer`: secondary text shortens before a shortcut is
/// clipped, and a refused page keeps its retry).
fn short(table: &Strings, look: &Look, say: &Say<'_>, fits: &dyn Fn(&[Span]) -> bool) -> Vec<Span> {
    let navigation = say.navigation;
    if loading(say.history) {
        let arrow = Span::styled(table.loading_arrow, look.accent.clone());
        if navigation.is_empty() {
            let text = [table.loading_short, table.loading_ellipsis]
                .into_iter()
                .find(|text| fits(&[arrow.clone(), Span::new(*text)]))
                .unwrap_or(table.loading_ellipsis);
            return vec![arrow, Span::new(text)];
        }
        let mut spans = vec![arrow.clone(), Span::new(table.loading_short_joined)];
        spans.extend(navigation.iter().cloned());
        if !fits(&spans) {
            spans = vec![arrow, Span::new(table.loading_ellipsis_joined)];
            spans.extend(navigation.iter().cloned());
        }
        return spans;
    }
    if say.history == HistoryState::Failed {
        return vec![dim(look, table.retry_short), key(look, table.begin_keys)];
    }
    if navigation.is_empty() {
        return vec![dim(look, table.new_activity)];
    }
    let mut spans = Vec::new();
    if say.news {
        spans.push(dim(look, table.new_short));
        spans.push(dim(look, table.footer_joiner));
    }
    spans.extend(navigation.iter().cloned());
    spans
}

/// The spellings of the row, widest first: the row (shortened when it does not fit), the keys
/// that ask a refused page again, the hint alone.
pub fn candidates(
    table: &Strings,
    look: &Look,
    say: &Say<'_>,
    fits: &dyn Fn(&[Span]) -> bool,
) -> Vec<Vec<Span>> {
    let mut row = whole(table, look, say);
    if !fits(&row) {
        row = short(table, look, say, fits);
    }
    let retry = if say.history == HistoryState::Failed {
        vec![key(look, table.retry_keys), dim(look, table.retry_action)]
    } else {
        say.navigation.to_vec()
    };
    vec![row, retry, say.navigation.to_vec()]
}
