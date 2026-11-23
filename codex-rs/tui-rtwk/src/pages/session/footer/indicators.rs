//! The indicators of the footer row: Plan mode and its cycle hint, the goal,
//! the side conversation's label, the viewed agent's name and the Vim mode
//! (oracle `bottom_pane/footer.rs` `mode_indicator_line`,
//! `goal_status_indicator_line`, `side_conversation_context_line`,
//! `textarea.rs` `vim_mode_indicator_span`).
//!
//! Each is a `Text.rich` of spans in the theme's roles, one or two spellings
//! widest first, which `hints::fit` picks from by the measured width. What an
//! indicator says is read from the facts (`chat::facts::Footer`, the thread's
//! goal); the words are `strings.rs`.

use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;

use codex_tui_core::screens::composer::vim::VimMode;

use super::{Form, Spec};

/// One spelling of spans: the cells they take and the widget that draws them.
fn form(spans: Vec<Span>, width: &dyn Fn(&str) -> usize) -> Form {
    let cells = spans.iter().map(|span| width(&span.text)).sum();
    Form {
        cells,
        widget: Text::rich_line(spans).into(),
    }
}

/// `Plan mode (⇧tab to cycle)` while the key works, then `Plan mode`.
pub(super) fn plan(spec: &Spec, width: &dyn Fn(&str) -> usize) -> Vec<Form> {
    let own = spec.own;
    let tokens = &spec.tokens;
    let bare = || vec![Span::styled(own.plan_mode, tokens.accent())];
    let with_cycle = || {
        let mut spans = bare();
        spans.extend([
            Span::styled(own.plan_cycle_open, tokens.dim()),
            Span::styled(own.plan_cycle_key, tokens.bold()),
            Span::styled(own.plan_cycle_close, tokens.dim()),
        ]);
        spans
    };
    let first = if spec.show_cycle {
        with_cycle()
    } else {
        bare()
    };
    vec![form(first, width), form(bare(), width)]
}

/// The goal's line, in the accent.
pub(super) fn goal(spec: &Spec, width: &dyn Fn(&str) -> usize) -> Option<Vec<Form>> {
    let indicator = spec.goal.as_ref()?;
    let text = spec.own.goal(indicator);
    Some(vec![form(
        vec![Span::styled(text, spec.tokens.accent())],
        width,
    )])
}

/// `Vim: Normal` (accent), `Vim: Insert` (success), `Vim: Replace` (info).
pub(super) fn vim(spec: &Spec, width: &dyn Fn(&str) -> usize) -> Option<Vec<Form>> {
    let mode = spec.vim?;
    let (text, style) = match mode {
        VimMode::Normal => (spec.own.vim_normal, spec.tokens.accent()),
        VimMode::Insert => (spec.own.vim_insert, spec.tokens.success()),
        VimMode::Replace => (spec.own.vim_replace, spec.tokens.info()),
    };
    Some(vec![form(vec![Span::styled(text, style)], width)])
}

/// The viewed agent's label, secondary.
pub(super) fn agent_name(spec: &Spec, width: &dyn Fn(&str) -> usize) -> Option<Vec<Form>> {
    let label = spec.agent_label.as_ref()?;
    Some(vec![form(
        vec![Span::styled(label.clone(), spec.tokens.dim())],
        width,
    )])
}

/// The side conversation's label (`side_conversation_context_line`): the
/// opening word bold in the accent, the parts that end in a key hint with the
/// key bold and the action secondary, the rest in the accent.
pub(super) fn side(spec: &Spec, label: &str, width: &dyn Fn(&str) -> usize) -> Vec<Form> {
    let own = spec.own;
    let tokens = &spec.tokens;
    let mut spans = Vec::new();
    let opening = format!("{} ", own.side_word);
    let rest = match label.strip_prefix(&opening) {
        Some(rest) => {
            spans.push(Span::styled(
                own.side_word,
                tokens.accent().attrs(Attrs::BOLD),
            ));
            spans.push(Span::new(opening[own.side_word.len()..].to_owned()));
            rest
        }
        None => label,
    };
    for (index, part) in rest.split(own.side_join).enumerate() {
        if index > 0 {
            spans.push(Span::styled(own.side_join, tokens.dim()));
        }
        let hint = own
            .side_actions
            .iter()
            .find_map(|action| part.strip_suffix(action).map(|keys| (keys, *action)));
        match hint {
            Some((keys, action)) => {
                spans.push(Span::styled(keys.to_owned(), tokens.bold()));
                spans.push(Span::styled(action, tokens.dim()));
            }
            None => spans.push(Span::styled(part.to_owned(), tokens.accent())),
        }
    }
    vec![form(spans, width)]
}
