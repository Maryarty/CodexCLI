//! Opens a verified web destination from the transcript through the host.

use super::AppCx;
use crate::app::effects::{Ask, Done};
use crate::data::links::web_destination;
use crate::ids::ThreadId;
use crate::threads::Tone;

pub struct TranscriptLinks;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub thread: ThreadId,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    thread: ThreadId,
    url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub url: String,
    pub error: String,
}

feature! {
    impl Feature for TranscriptLinks {
        const NAME: &'static str = "transcript_links";
        type Intent = Intent;
        type Ctx = Ctx;
        type Note = Note;
        {
            fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
                if cx.view().thread(&intent.thread).is_some()
                    && let Some(url) = web_destination(&intent.url)
                {
                    cx.effect(Ask::OpenUrl { url: url.clone() }, Ctx { thread: intent.thread, url });
                }
            }

            fn effect_done(_state: &mut (), ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
                if let Done::Opened(Err(error)) = done
                    && cx.view().thread(&ctx.thread).is_some()
                {
                    cx.note(Some(ctx.thread), Note { url: ctx.url, error });
                }
            }

            fn note_tone(_note: &Note) -> Tone {
                Tone::Error
            }
        }
    }
}
