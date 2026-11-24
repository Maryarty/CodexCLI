//! The security-setup reminder as a feature slice (ADR-012; oracle `security_setup.rs`,
//! `chatwidget/security_setup.rs`): the server's optional notice that an account should set up
//! hardware-key security, shown as an inline banner of the chat. The server owns eligibility,
//! rollout and the words; the host fetches the notice (a ChatGPT-signed-in, OpenAI-provider,
//! local session, the identity checked against the auth that is current after the fetch) and tells
//! the feature as a fact. Core keeps what IO and the surface need to agree on: the notice and the
//! identity it was verified for, and the *occurrence* it belongs to. Whether the banner is
//! dismissed is the banner's (UI state, ADR-013): the view keeps the occurrence it was dismissed
//! for, so a reconnect that announces the same account does not bring it back and a verified other
//! account does (`State::occurrence`). The banner's action is the verb `Intent::Open`.
//! Words and the banner: `src/features/security_setup/`.

use super::AppCx;
use crate::app::effects::{Ask, Done};
use crate::threads::Tone;

/// The marker of the feature (`feature_list!`).
pub struct SecuritySetup;

/// Who the notice was fetched for (oracle `Identity`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub account: String,
    pub user: String,
}

/// The server's notice: a title, a description and one action that opens a page (oracle `Notice`).
/// The host sends only a valid one (`valid`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub description: String,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub label: String,
    pub url: String,
}

impl Notice {
    /// The text rules of the oracle's `Notice::valid`, all but the address (the host, which parses
    /// URLs, checks that it is an `https://chatgpt.com` page without credentials or port).
    pub fn text_valid(&self) -> bool {
        let plain = |text: &str| !text.chars().any(char::is_control);
        !self.title.trim().is_empty()
            && self.title.len() <= 256
            && self.description.len() <= 2048
            && plain(&self.title)
            && plain(&self.description)
            && !self.action.label.trim().is_empty()
            && self.action.label.len() <= 128
            && plain(&self.action.label)
    }
}

/// What the host tells the feature unasked (and a script seeds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// A notice fetched and verified for `identity` (`show_security_setup`).
    Loaded { identity: Identity, notice: Notice },
    /// The account was announced again (`AccountUpdated`, a reconnect): what is held is stale and
    /// hides, results of requests made before are not wanted; the occurrence is kept until an
    /// identity is verified (`invalidate_security_setup`).
    Invalidated,
}

impl Fact {
    /// A notice the host fetched for the account (`account`, `user`) and checked the address of
    /// (the host parses URLs; the text rules are `Notice::text_valid`, checked on arrival).
    pub fn loaded(
        account: String,
        user: String,
        title: String,
        description: String,
        label: String,
        url: String,
    ) -> Fact {
        Fact::Loaded {
            identity: Identity { account, user },
            notice: Notice {
                title,
                description,
                action: Action { label, url },
            },
        }
    }
}

/// What the banner asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The banner's action: its page opens in the browser.
    Open,
}

/// What an ask of the host was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Page { url: String },
}

/// What the feature says in the transcript (an error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Failed to open browser for {url}: {error}`.
    PageFailed { url: String, error: String },
}

/// The notice that is up, for the identity it was verified for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    identity: Option<Identity>,
    notice: Option<Notice>,
    /// Counts the identities verified so far: a notice is the same occurrence while the identity
    /// is, and a dismissal belongs to its occurrence.
    occurrence: u64,
}

impl State {
    /// The notice to show now, if any.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// Which occurrence the notice belongs to (0: none was ever verified).
    pub fn occurrence(&self) -> u64 {
        self.occurrence
    }

    fn loaded(&mut self, identity: Identity, notice: Notice) {
        if !notice.text_valid() {
            return;
        }
        if self.identity.as_ref() != Some(&identity) {
            self.identity = Some(identity);
            self.occurrence += 1;
        }
        self.notice = Some(notice);
    }
}

feature! {
    impl Feature for SecuritySetup {
        const NAME: &'static str = "security_setup";
        type State = State;
        type Intent = Intent;
        type Ctx = Ctx;
        type Note = Note;
        type Fact = Fact;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                let Intent::Open = intent;
                if let Some(notice) = &state.notice {
                    let url = notice.action.url.clone();
                    cx.effect(Ask::OpenUrl { url: url.clone() }, Ctx::Page { url });
                }
            }

            fn effect_done(_state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
                let (Ctx::Page { url }, Done::Opened(Err(error))) = (ctx, done) else {
                    return;
                };
                cx.note(None, Note::PageFailed { url, error });
            }

            fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
                match fact {
                    Fact::Loaded { identity, notice } => state.loaded(identity, notice),
                    Fact::Invalidated => state.notice = None,
                }
            }

            fn note_tone(_note: &Note) -> Tone {
                Tone::Error
            }
        }
    }
}
