// cell: Item::VoiceTranscript { .. }
//! Realtime speech is a transcript row beside ordinary streaming output.

use codex_tui_core::threads::{Item, voice_transcript::Role};
use rtwk::AnyWidget;
use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::widgets::Text;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use super::text::{Part, Texts};
use crate::pages::session::strings::Strings as Chat;

fn mark(role: Role, cells: &Strings, chat: &Chat) -> &'static str {
    match role {
        Role::User => chat.prompt.trim_end(),
        Role::Assistant => cells.bullet.trim_end(),
    }
}

pub(super) fn texts(role: Role, text: &str, cells: &Strings, chat: &Chat) -> Texts {
    Texts::Exact(vec![
        Part::gutter(mark(role, cells, chat)),
        Part::text(text),
    ])
}

pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::VoiceTranscript { role, text } = item else {
        return AnyWidget::default();
    };
    let style = match role {
        Role::User => env.tokens.accent_on(env.prompt_surface),
        Role::Assistant => env.bullet_style.clone(),
    };
    below(
        place.separator,
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(ExcludeSelection::new(SizedBox::new(
                Some(2),
                None,
                Some(
                    Text::new(mark(*role, env.cells, env.chat))
                        .style(style)
                        .into(),
                ),
            )))
            .child(Expanded::new(1, Text::new(text.clone())))
            .into(),
    )
}
