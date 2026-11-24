//! The command center's worktree creation. The app drives each step; the
//! shell only creates or binds a checkout and sends the app's RPCs.

use std::rc::Rc;

use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::{Answer, RpcError};
use crate::screens::{ScreenCommand, ScreenReply, agents};
use crate::threads::{Origin, ThreadAsk};

use super::{
    AppCtx, AppFx, CodexApp, Command, apply_thread, attach_thread, fetch_history, session_note,
    show_held_thread,
};
use crate::screens::select;
use crate::screens::select::worktree::Refusal;
use crate::threads::{Item, Note, NoteKey, ThreadEv};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkout {
    pub root: String,
    pub cwd: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindFailure {
    Workspace(String),
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetainedReason {
    Start(RpcError),
    Bind(BindFailure),
}

/// What the chat's `/new`, `/fork` and `/worktree` want of the checkout (the command center's
/// creation has none): the conversation that goes on in it, as a fork of `source` or a fresh
/// thread, named `name` once it has started. Errors are lines of the transcript of `source`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Chat {
    source: ThreadId,
    fork: bool,
    name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Creating {
        chat: Option<Chat>,
    },
    Starting {
        checkout: Checkout,
        chat: Option<Chat>,
    },
    Binding {
        checkout: Checkout,
        thread: ThreadId,
        chat: Option<Chat>,
    },
    Unsubscribing {
        root: String,
        error: BindFailure,
        chat: Option<Chat>,
    },
}

impl Step {
    pub(super) fn create(app: &mut CodexApp, cwd: Option<String>, fx: &mut AppFx) {
        let backend = app.screens.agents.displayed;
        let ticket = Self::Creating { chat: None }.wait(app, backend);
        fx.commands.push(Command::CreateWorktree { ticket, cwd });
    }

    /// `StartManagedWorktree` of the chat (`start_managed_worktree`): the source-only checks,
    /// then a checkout of the session's repository the conversation goes on in. `fork`: a fork
    /// of the thread (`/fork`, `/worktree` continue), else a fresh thread (`/new`).
    pub(super) fn create_for_chat(
        app: &mut CodexApp,
        source: Option<ThreadId>,
        fork: bool,
        name: Option<String>,
        fx: &mut AppFx,
    ) {
        let refuse = |app: &mut CodexApp, why: Refusal, fx: &mut AppFx| {
            let note = Note::from(NoteKey::Select(select::Note::WorktreeRefused(why)));
            session_note(app, source.clone(), note, fx);
        };
        let Some(thread) = source.clone() else {
            return refuse(app, Refusal::NotIdle, fx);
        };
        let Some(model) = app.threads.get(&thread).map(Rc::clone) else {
            return refuse(app, Refusal::NotIdle, fx);
        };
        let part = &model.screen_slots().chat;
        let idle = !model.is_running()
            && !model.can_interrupt()
            && model.queue().is_empty()
            && model.pending_steers().is_empty();
        if !idle
            || app.screens.chat.flags.side_conversation_active
            || part.parent_owned
            || part.ephemeral
        {
            return refuse(app, Refusal::NotIdle, fx);
        }
        if app.outgoing.any(|ctx| {
            matches!(
                ctx,
                AppCtx::Worktree { .. } | AppCtx::NewConversation { .. }
            )
        }) {
            return refuse(app, Refusal::Pending, fx);
        }
        let has_history = model
            .transcript()
            .iter()
            .any(|(_, item)| !matches!(item, Item::Note(_)));
        if fork && has_history && part.path.is_none() {
            return refuse(app, Refusal::HistoryNotSaved, fx);
        }
        let backend = app.screens.agents.owner_of(thread.as_str());
        let cwd = Some(app.screens.select.worktree.cwd().to_owned());
        let chat = Some(Chat {
            source: thread,
            fork: fork && has_history,
            name,
        });
        let ticket = Self::Creating { chat }.wait(app, backend);
        fx.commands.push(Command::CreateWorktree { ticket, cwd });
    }

    fn wait(self, app: &mut CodexApp, backend: BackendId) -> Ticket {
        let ticket = app.mint.next();
        app.outgoing.insert(
            ticket,
            AppCtx::Worktree {
                backend,
                step: self,
            },
        );
        ticket
    }

    fn request(self, ask: ThreadAsk, app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
        let ticket = self.wait(app, backend);
        fx.commands.push(Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        });
    }

    pub(super) fn advance(
        self,
        app: &mut CodexApp,
        backend: BackendId,
        answer: Answer,
        fx: &mut AppFx,
    ) {
        match (self, answer) {
            (Self::Creating { chat }, Answer::WorktreeCreated(Ok(checkout))) => {
                let cwd = Some(checkout.cwd.clone());
                let ask = match &chat {
                    Some(Chat {
                        source, fork: true, ..
                    }) => ThreadAsk::ForkAt {
                        thread: source.to_string(),
                        cwd,
                    },
                    _ => ThreadAsk::Start { cwd },
                };
                Self::Starting { checkout, chat }.request(ask, app, backend, fx);
                return;
            }
            (Self::Creating { chat }, Answer::WorktreeCreated(Err(error))) => {
                return finish(app, chat, Said::Failed(error), fx);
            }
            (
                Self::Starting { checkout, chat },
                Answer::Screen(ScreenReply::Agents(
                    agents::Reply::Started(thread) | agents::Reply::Forked(thread),
                )),
            ) => {
                let id = ThreadId::new(&thread.id);
                let cwd = thread.cwd.display().to_string();
                let ticket = Self::Binding {
                    checkout: checkout.clone(),
                    thread: id.clone(),
                    chat,
                }
                .wait(app, backend);
                fx.commands.push(Command::BindWorktree {
                    ticket,
                    checkout,
                    thread: id,
                    cwd,
                });
                return;
            }
            (Self::Starting { checkout, chat }, answer) => {
                let error = match answer {
                    Answer::Failed(error) => error,
                    _ => RpcError::Mismatch,
                };
                let reason = RetainedReason::Start(error);
                let root = checkout.root;
                return finish(app, chat, Said::Retained { root, reason }, fx);
            }
            (Self::Binding { thread, chat, .. }, Answer::WorktreeBound(Ok(()))) => {
                let agents = &mut Rc::make_mut(&mut app.screens).agents;
                agents.observe(backend, thread.as_str());
                agents.display(backend);
                agents::finish_start(agents, thread.as_str());
                attach_thread(app, &thread, backend, Origin::Started, fx);
                if let Some(chat) = chat {
                    // `/new <name>`, `/fork <name>`: named once it has started.
                    if let Some(name) = chat.name {
                        let intent = crate::features::rename::ThreadIntent::Set(name).into();
                        apply_thread(app, &thread, ThreadEv::FeatureIntent(intent), fx);
                    }
                    show_held_thread(app, thread.clone(), fx);
                    if chat.fork {
                        fetch_history(app, &thread, fx);
                    }
                } else {
                    show_held_thread(app, thread, fx);
                }
                return;
            }
            (
                Self::Binding {
                    checkout,
                    thread,
                    chat,
                },
                Answer::WorktreeBound(Err(error)),
            ) => {
                Self::Unsubscribing {
                    root: checkout.root,
                    error,
                    chat,
                }
                .request(
                    ThreadAsk::Unsubscribe {
                        thread: thread.to_string(),
                    },
                    app,
                    backend,
                    fx,
                );
                return;
            }
            (Self::Unsubscribing { root, error, chat }, _) => {
                let reason = RetainedReason::Bind(error);
                return finish(app, chat, Said::Retained { root, reason }, fx);
            }
            _ => {}
        }
    }
}

/// How a creation ended badly: the host's own words, or a checkout that stays on disk.
enum Said {
    Failed(String),
    Retained {
        root: String,
        reason: RetainedReason,
    },
}

/// The line that says so: in the transcript of the thread the chat asked from, or the notice of
/// the command center.
fn finish(app: &mut CodexApp, chat: Option<Chat>, said: Said, fx: &mut AppFx) {
    let Some(chat) = chat else {
        let notice = match said {
            Said::Failed(error) => agents::Notice::Error(error),
            Said::Retained { root, reason } => agents::Notice::WorktreeRetained { root, reason },
        };
        Rc::make_mut(&mut app.screens).agents.notice = Some(notice);
        return;
    };
    let note = match said {
        Said::Failed(error) => select::Note::WorktreeFailed(error),
        Said::Retained { root, reason } => select::Note::WorktreeRetained { root, reason },
    };
    session_note(
        app,
        Some(chat.source),
        Note::from(NoteKey::Select(note)),
        fx,
    );
}
