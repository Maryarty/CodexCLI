//! The row while prompts are browsed (oracle `app_backtrack/prompt_navigation.rs`,
//! `prompt_navigation_footer`): the mode's name and its keys, the first spelling that fits the
//! row, whole to bare. The key of the details is the keymap's `open_transcript`, as the oracle's
//! `primary_hint` words it (a chord included), and none while that action is unbound.
//!
//! The chat's footer shows it in the place of the passive keys while prompts are browsed and no
//! selection, Find or copy mode asks for the row (`find_footer::slot` wraps the footer this
//! gives, so those come first as `has_active_interaction` has it), and not while a refused page
//! asks for its retry (`history == Failed`). The sheet's status row shows it above its hints.
//!
//! Flutter: a `ListenableBuilder` over the mode's controller choosing between two children, and
//! a `LayoutBuilder` + `segmenter_of` that picks the spelling (as `follow.rs`).

use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::provider::select;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_TRANSCRIPT;
use codex_tui_core::threads::{HistoryState, ThreadModel};

use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::widgets::thread_ui::Modes;

/// The label of the key that shows the details: the first binding the user wrote for
/// `open_transcript`, a chord or not.
pub fn details_label(keymap: &Keymap) -> Option<String> {
    crate::keymap::primary_label(keymap, GLOBAL_OPEN_TRANSCRIPT)
}

/// The row: the first spelling that fits the width (`first_fitting_line`).
pub fn row(
    table: &'static Strings,
    details: Option<String>,
    key: Style,
    accent: Style,
    dim: Style,
) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let [whole, short] = table.browsing_label;
        let line = |label: &'static str, hints: &[(String, &'static str)]| -> Vec<Span> {
            let mut spans = vec![Span::styled(label, accent.clone())];
            for (keys, action) in hints {
                spans.push(Span::styled(table.hint_joiner, dim.clone()));
                spans.push(Span::styled(keys.clone(), key.clone()));
                if !action.is_empty() {
                    spans.push(Span::styled(table.hint_gap, dim.clone()));
                    spans.push(Span::styled(*action, dim.clone()));
                }
            }
            spans
        };
        let pair = |keys: &'static str, action: &'static str| (keys.to_owned(), action);
        let mut full = vec![
            pair(table.browsing_scroll_keys, table.browsing_scroll),
            pair(table.browsing_prompt_keys, table.browsing_prompts),
        ];
        let mut compact = vec![pair(table.browsing_both_keys, "")];
        for hints in [&mut full, &mut compact] {
            if let Some(label) = &details {
                hints.push((label.clone(), table.browsing_details));
            }
            hints.push(pair(table.browsing_rewind_key, table.browsing_rewind));
            hints.push(pair(table.key_esc, table.browsing_back));
        }
        let rewind = pair(table.browsing_rewind_key, table.browsing_rewind);
        let back = pair(table.key_esc, table.browsing_back);
        let candidates = [
            line(whole, &full),
            line(short, &full),
            line(short, &compact),
            line(
                short,
                &[
                    pair(table.browsing_both_keys, ""),
                    rewind.clone(),
                    pair(table.key_esc, ""),
                ],
            ),
            line(short, &[rewind, back.clone()]),
            line(short, &[back]),
            line(short, &[pair(table.key_esc, "")]),
            line(short, &[]),
        ];
        let fits = |spans: &Vec<Span>| {
            let cells: usize = spans.iter().map(|span| segmenter.width(&span.text)).sum();
            i32::try_from(cells).unwrap_or(i32::MAX) <= constraints.max_width
        };
        let chosen = candidates
            .iter()
            .find(|spans| fits(spans))
            .cloned()
            .unwrap_or_default();
        Text::rich_line(chosen).overflow(TextOverflow::Clip).into()
    })
    .into()
}

/// The cells the chat's footer rows start in from the left (the composer's margin).
const MARGIN: i32 = 2;

/// The chat's footer: `passive` (the keys of the page) while no prompts are browsed or a
/// refused page asks for its retry, else this row, two cells in as the footer's own.
pub fn footer<S: 'static>(cx: &mut Cx<'_, S>, modes: Modes, passive: AnyWidget) -> AnyWidget {
    let table = strings::of(cx);
    let tokens = Tokens::of(cx);
    let (key, accent, dim) = (tokens.bold(), tokens.accent(), super::history::plain_dim());
    let details = details_label(&crate::keymap::keymap(cx));
    let state = select::<ThreadModel, _, _>(cx, ThreadModel::history_state);
    let listenable = modes.listenable();
    ListenableBuilder::new(listenable, move |_, _| {
        if !modes.browsing() || state == HistoryState::Failed {
            return passive.clone();
        }
        Row::new()
            .child(SizedBox::new(Some(MARGIN), None, None))
            .child(Expanded::new(
                1,
                row(
                    table,
                    details.clone(),
                    key.clone(),
                    accent.clone(),
                    dim.clone(),
                ),
            ))
            .into()
    })
    .into()
}
