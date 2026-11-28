// cell: Item::VoiceTranscript { .. }
//! Realtime speech is a transcript row beside ordinary streaming output.

use codex_tui_core::threads::{Item, voice_transcript::Role};
use rtwk::AnyWidget;

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
    // a spoken prompt begins after its mark: the whitespace a speech transcript can start with is
    // not drawn (oracle `UserHistoryCell` Spoken)
    let text = match role {
        Role::User => text.trim_start(),
        Role::Assistant => text,
    };
    Texts::Exact(vec![
        Part::gutter(mark(role, cells, chat)),
        Part::text(text),
    ])
}

pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::VoiceTranscript { role, text } = item else {
        return AnyWidget::default();
    };
    if *role == Role::User {
        return super::user::spoken(text, env, place);
    }
    below(
        place.separator,
        super::spoken::message(
            text,
            env.cwd,
            &super::agent::Look {
                bullet: env.bullet,
                bullet_style: env.bullet_style.clone(),
                markdown: env.markdown,
                cells: env.cells,
            },
        ),
    )
}
