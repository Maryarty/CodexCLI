//! The model provider list and its editor as an ADR-013 feature (oracle `app/provider.rs`,
//! `provider/{editor,fields,write,rpc}.rs`): the settings panel's `Model providers` row opens a
//! sheet with the providers the server knows, their details and a form for a custom gateway or a
//! cloud provider's settings. Core keeps what IO needs: the catalog the server answered with (a
//! `Load`), the write under way (`Saving`), what the last write ended with (an `Outcome`, shown
//! under the list once, or in the form when it failed) and what the client's launch settings
//! select. The sheet, its pages, the form and the words are `src/features/providers/`; the fields
//! of a provider and the patch a form asks for are `form.rs`; the RPCs are `rpcs.rs`.
//!
//! The sheet says it is interested in the list (`Interest::List`) while it is open: the feature
//! reads it then, and a late answer of an older read is dropped. A page that waits for a write
//! says so too (`Interest::Saving`): a write that ends while one waits is *present* and the
//! sheet shows its end; one that ends after the user left the page is said in the transcript
//! (oracle `Completion::finish`, `dismissed`).

pub mod form;
mod rpcs;

use codex_app_server_protocol::{ClientRequest, ModelProviderListResponse, RequestId};

use super::{AppCx, Connection};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::threads::Tone;

pub use codex_app_server_protocol::{ModelProviderSummary, ProviderSettings};
pub use rpcs::{Chosen, Command, Reply, refusal};

/// The marker of the feature (`feature_list!`).
pub struct Providers;

/// What the sheet and its pages ask of the feature: verbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Save what a form asks for (`form::Draft::patch`).
    Save(codex_app_server_protocol::ModelProviderConfigWriteParams),
    /// `model_provider` of the user's config becomes `id`: the default for new chats.
    SetDefault(String),
    /// Read the list again (the retry of a failed one).
    Reload,
}

/// What a surface can be interested in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interest {
    /// The list is on screen: it is read, and kept for as long as the sheet is open.
    List,
    /// A page waits for a write.
    Saving,
}

/// What the host tells the feature unasked (and a script seeds: `{"kind":"override","id":"x"}`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fact {
    /// The provider this client's launch settings select for new chats (none: no selection).
    Override { id: Option<String> },
}

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// A read of the list, numbered.
    List { request: u64 },
    /// The write of a provider's settings (`create`: of a new gateway).
    Write { create: bool },
    /// The write of the default for new chats.
    Default { id: String },
}

/// What a write ended with, for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Saved(Saved),
    Trouble(Trouble),
}

/// How a write went through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    /// A provider's settings (`Saved provider configuration.`).
    Settings,
    /// `/new` uses `id` now.
    Default { id: String },
    /// `id` was written but a higher-priority setting overrides it; the server's reason, if any.
    Overridden { id: String, reason: Option<String> },
}

/// Why a write did not go through (the words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trouble {
    /// The connection to the server is down.
    Offline,
    /// The server's refusal of a provider's settings, as it said it.
    Message(String),
    /// The server's refusal of the default.
    Rejected(String),
    /// The reply of the write of the default did not come: it may have been committed.
    Lost { id: String, error: String },
    /// The reply of the write of a new gateway did not come: it may have been committed.
    UnconfirmedCreate,
}

/// What the feature says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// A write was asked while one was under way.
    Busy,
    /// A write ended where the user did not see it, or ended in a way to keep.
    Outcome(Outcome),
}

/// The list as the server answered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Load {
    /// Nobody asked.
    #[default]
    Idle,
    Loading,
    Ready(Page),
    Failed(Failure),
}

/// The catalog, and what the write that was followed by this read ended with (shown in the
/// list's place of the note, once).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub list: ModelProviderListResponse,
    pub notice: Option<Outcome>,
}

/// Why the list could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Offline,
    /// The server does not know the list.
    Unsupported,
    /// The server's or the decoder's text.
    Server(String),
}

/// The write under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saving {
    Default(String),
    Settings,
}

/// The state of the feature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    load: Load,
    /// Numbers the reads: a late answer of an older one is dropped.
    request: u64,
    /// The sheet is open.
    watching: bool,
    /// A page waits for the write.
    present: bool,
    offline: bool,
    saving: Option<Saving>,
    /// Numbers the writes of settings; a failure belongs to the write that made it.
    writes: u64,
    failure: Option<(u64, Trouble)>,
    /// What the write that made the next read ended with.
    notice: Option<Outcome>,
    client_override: Option<String>,
}

impl State {
    pub fn load(&self) -> &Load {
        &self.load
    }

    pub fn saving(&self) -> Option<&Saving> {
        self.saving.as_ref()
    }

    /// How many writes of settings were asked: a form started after the n-th shows only a failure
    /// of a later one.
    pub fn writes(&self) -> u64 {
        self.writes
    }

    /// What the last write of settings failed with, when it was made after `since` writes.
    pub fn failure_since(&self, since: u64) -> Option<&Trouble> {
        self.failure
            .as_ref()
            .filter(|(write, _)| *write > since)
            .map(|(_, trouble)| trouble)
    }

    /// The provider the client's launch settings select for new chats.
    pub fn client_override(&self) -> Option<&str> {
        self.client_override.as_deref()
    }

    fn read(&mut self, notice: Option<Outcome>, cx: &mut AppCx<'_>) {
        self.request += 1;
        self.notice = notice;
        if self.offline {
            self.notice = None;
            self.load = Load::Failed(Failure::Offline);
            return;
        }
        self.load = Load::Loading;
        cx.ask(
            None,
            Command::List,
            Ctx::List {
                request: self.request,
            },
        );
    }

    fn save(&mut self, saving: Saving, command: Command, ctx: Ctx, cx: &mut AppCx<'_>) {
        if self.saving.is_some() {
            cx.note(None, Note::Busy);
            return;
        }
        let settings = matches!(saving, Saving::Settings);
        if settings {
            self.writes += 1;
        }
        if self.offline {
            // Nothing is sent; the page that asked is still the one the user is on.
            self.complete(saving, Err(Trouble::Offline), true, cx);
            return;
        }
        self.saving = Some(saving);
        cx.ask(None, command, ctx);
    }

    /// A write ended: what the user sees, and what the transcript says (oracle
    /// `Completion::finish`).
    fn complete(
        &mut self,
        saving: Saving,
        result: Result<Saved, Trouble>,
        present: bool,
        cx: &mut AppCx<'_>,
    ) {
        self.saving = None;
        let settings = matches!(saving, Saving::Settings);
        let outcome = match result {
            Ok(saved) => Outcome::Saved(saved),
            Err(trouble) => Outcome::Trouble(trouble),
        };
        let reload = |state: &mut State, notice, cx: &mut AppCx<'_>| {
            if state.watching {
                state.read(notice, cx);
            }
        };
        match (&outcome, present) {
            (Outcome::Saved(_), false) => {
                cx.note(None, Note::Outcome(outcome));
                reload(self, None, cx);
            }
            (Outcome::Saved(_), true) => reload(self, Some(outcome), cx),
            // The user is told that the save is unknown, wherever they are.
            (Outcome::Trouble(Trouble::UnconfirmedCreate), present) => {
                cx.note(None, Note::Outcome(outcome.clone()));
                reload(self, present.then_some(outcome), cx);
            }
            (Outcome::Trouble(trouble), true) if settings => {
                self.failure = Some((self.writes, trouble.clone()));
            }
            (Outcome::Trouble(_), true) => {
                cx.note(None, Note::Outcome(outcome.clone()));
                reload(self, Some(outcome), cx);
            }
            (Outcome::Trouble(_), false) => cx.note(None, Note::Outcome(outcome)),
        }
    }

    fn loaded(&mut self, request: u64, result: Result<Reply, RpcError>) {
        if request != self.request || !self.watching {
            return;
        }
        let notice = self.notice.take();
        self.load = match result {
            Ok(Reply::List(list)) => Load::Ready(Page { list, notice }),
            Ok(Reply::Unsupported) => Load::Failed(Failure::Unsupported),
            Ok(_) | Err(RpcError::Mismatch) => Load::Failed(Failure::Server(String::new())),
            Err(error) => Load::Failed(Failure::Server(text(error))),
        };
    }

    fn written(&mut self, create: bool, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let present = self.present;
        let result = match result {
            Ok(Reply::Saved(_)) => Ok(Saved::Settings),
            Ok(Reply::Unconfirmed(_)) => Err(Trouble::UnconfirmedCreate),
            Err(RpcError::Malformed(_)) if create => Err(Trouble::UnconfirmedCreate),
            Ok(_) => Err(Trouble::Message(String::new())),
            Err(error) => Err(Trouble::Message(text(error))),
        };
        self.complete(Saving::Settings, result, present, cx);
    }

    fn chosen(&mut self, id: String, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let present = self.present;
        let result = match result {
            Ok(Reply::Chosen(Chosen::Written)) => Ok(Saved::Default { id: id.clone() }),
            Ok(Reply::Chosen(Chosen::Overridden(reason))) => Ok(Saved::Overridden {
                id: id.clone(),
                reason,
            }),
            // The server may have committed the write before its reply was lost.
            Ok(Reply::Unconfirmed(error)) | Err(RpcError::Malformed(error)) => Err(Trouble::Lost {
                id: id.clone(),
                error,
            }),
            Ok(_) => Err(Trouble::Rejected(String::new())),
            Err(error) => Err(Trouble::Rejected(text(error))),
        };
        self.complete(Saving::Default(id), result, present, cx);
    }
}

/// The text of a failed ask.
fn text(error: RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => text,
        RpcError::Mismatch | RpcError::Provider(_) => String::new(),
    }
}

feature! {
    impl Feature for Providers {
        const NAME: &'static str = "providers";
        const SEED: Option<fn(serde_json::Value) -> Result<Fact, SeedError>> = Some(seed);
        const REFUSAL: Option<super::RefusalFn<Command, Reply>> = Some(refusal);
        const SETTING: Option<super::SettingId> = Some(super::SettingId::ModelProviders);
        type State = State;
        type Intent = Intent;
        type Command = Command;
        type Reply = Reply;
        type Ctx = Ctx;
        type Note = Note;
        type Fact = Fact;
        type Interest = Interest;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Save(params) => {
                        let create = form::creates(&params);
                        state.save(Saving::Settings, Command::Write(params), Ctx::Write { create }, cx);
                    }
                    Intent::SetDefault(id) => {
                        state.save(
                            Saving::Default(id.clone()),
                            Command::SetDefault(id.clone()),
                            Ctx::Default { id },
                            cx,
                        );
                    }
                    Intent::Reload => {
                        if state.watching {
                            state.read(None, cx);
                        }
                    }
                }
            }

            fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
                match ctx {
                    Ctx::List { request } => state.loaded(request, result),
                    Ctx::Write { create } => state.written(create, result, cx),
                    Ctx::Default { id } => state.chosen(id, result, cx),
                }
            }

            fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
                let Fact::Override { id } = fact;
                state.client_override = id;
            }

            fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
                match interest {
                    Interest::List if open => {
                        state.watching = true;
                        state.read(None, cx);
                    }
                    Interest::List => {
                        state.watching = false;
                        // A late answer is not wanted by whoever opens the sheet next.
                        state.request += 1;
                        state.notice = None;
                        state.load = Load::Idle;
                    }
                    Interest::Saving => state.present = open,
                }
            }

            fn on_connection(state: &mut State, connection: Connection, _cx: &mut AppCx<'_>) {
                match connection {
                    Connection::Up => state.offline = false,
                    Connection::Lost => state.offline = true,
                    Connection::ShuttingDown => {}
                }
            }

            fn note_tone(note: &Note) -> Tone {
                match note {
                    Note::Outcome(Outcome::Saved(_)) => Tone::Info,
                    Note::Busy | Note::Outcome(Outcome::Trouble(_)) => Tone::Error,
                }
            }

            fn wire(command: &Command, _thread: &ThreadId, id: RequestId) -> ClientRequest {
                command.request(id)
            }

            fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
                command.reply(result)
            }
        }
    }
}

fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))
}
