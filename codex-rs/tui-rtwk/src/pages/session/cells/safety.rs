//! A request the server refused for safety (oracle history_cell/notices.rs
//! `SafetyAccessBlockCell`, `new_safety_access_block_event`,
//! `new_cyber_policy_error_event`): `ⓘ ` and a bold title, the explanation and
//! body dim two columns in (two free at the right), then one row per action
//! (`label: url`, the url in the accent, underlined, a hyperlink).
//!
//! Flutter: a `Column` of a `Text.rich` and `Padding(horizontal: 2)` around
//! the wrapped `Text`s.

use codex_tui_core::screens::cells::Daybreak;
use codex_tui_core::threads::Notice;
use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;

/// What a refusal says: its title, the explanation before the body (the
/// Apply and biology forms have one), the body and the actions.
pub struct Words {
    pub title: &'static str,
    pub explanation: Option<&'static str>,
    pub body: &'static str,
    pub actions: Vec<(&'static str, &'static str)>,
}

/// The words of `notice`; `None` for a notice that is no refusal.
pub fn words(notice: &Notice, strings: &Strings) -> Option<Words> {
    let learn = (strings.learn_more, strings.learn_more_url);
    Some(match notice {
        Notice::SafetyAccessBlock => Words {
            title: strings.safety_title,
            explanation: Some(strings.safety_explanation),
            body: strings.safety_body,
            actions: vec![(strings.trusted_access, strings.trusted_access_url), learn],
        },
        Notice::CyberPolicy(kind) => {
            let (body, actions) = match kind {
                Daybreak::Apply => (
                    strings.cyber_apply_body,
                    vec![learn, (strings.cyber_apply_label, strings.cyber_apply_url)],
                ),
                Daybreak::Disabled => (strings.cyber_disabled_body, vec![learn]),
                Daybreak::Astra => (strings.cyber_astra_body, vec![learn]),
                Daybreak::Enabled => (strings.cyber_enabled_body, vec![learn]),
            };
            Words {
                title: strings.cyber_title,
                explanation: (*kind == Daybreak::Apply).then_some(strings.cyber_explanation),
                body,
                actions,
            }
        }
        _ => return None,
    })
}

impl Words {
    /// The explanation and the body, one text.
    fn message(&self) -> String {
        match self.explanation {
            Some(explanation) => [explanation, " ", self.body].concat(),
            None => self.body.to_owned(),
        }
    }
}

/// The texts of the cell: the title row (its mark is the row's first span),
/// the message, one row per action.
pub fn texts(words: &Words, strings: &Strings) -> Vec<Part> {
    let mut parts = vec![
        Part::text([strings.safety_mark, words.title].concat()),
        Part::text(words.message()),
    ];
    parts.extend(
        words
            .actions
            .iter()
            .map(|(label, url)| Part::text([label, strings.action_separator, url].concat())),
    );
    parts
}

/// The cell.
pub fn cell(words: &Words, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let inset = |row: Text| -> AnyWidget { Padding::new(EdgeInsets::only(2, 0, 2, 0), row).into() };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(vec![
            Span::styled(strings.safety_mark, tokens.accent()),
            Span::styled(words.title, tokens.bold()),
        ]))
        .child(inset(Text::new(words.message()).style(dim.clone())));
    for (label, url) in &words.actions {
        column = column.child(inset(Text::rich(vec![
            Span::styled([label, strings.action_separator].concat(), dim.clone()),
            Span::styled(*url, tokens.accent().attrs(Attrs::UNDERLINE).link(*url)),
        ])));
    }
    column.into()
}
