//! The command center's worktree creation. The app drives each step; the
//! shell only creates or binds a checkout and sends the app's RPCs.

use std::rc::Rc;

use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::{Answer, RpcError};
use crate::screens::{ScreenCommand, ScreenReply, agents};
use crate::threads::{Origin, ThreadAsk};

use super::{AppCtx, AppFx, CodexApp, Command, attach_thread, show_held_thread};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Creating,
    Starting(Checkout),
    Binding {
        checkout: Checkout,
        thread: ThreadId,
    },
    Unsubscribing {
        root: String,
        error: BindFailure,
    },
}

impl Step {
    pub(super) fn create(app: &mut CodexApp, cwd: Option<String>, fx: &mut AppFx) {
        let backend = app.screens.agents.displayed;
        let ticket = Self::Creating.wait(app, backend);
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
        let notice = match (self, answer) {
            (Self::Creating, Answer::WorktreeCreated(Ok(checkout))) => {
                let cwd = Some(checkout.cwd.clone());
                Self::Starting(checkout).request(ThreadAsk::Start { cwd }, app, backend, fx);
                return;
            }
            (Self::Creating, Answer::WorktreeCreated(Err(error))) => agents::Notice::Error(error),
            (
                Self::Starting(checkout),
                Answer::Screen(ScreenReply::Agents(agents::Reply::Started(thread))),
            ) => {
                let id = ThreadId::new(&thread.id);
                let cwd = thread.cwd.display().to_string();
                let ticket = Self::Binding {
                    checkout: checkout.clone(),
                    thread: id.clone(),
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
            (Self::Starting(checkout), answer) => {
                let error = match answer {
                    Answer::Failed(error) => error,
                    _ => RpcError::Mismatch,
                };
                agents::Notice::WorktreeRetained {
                    root: checkout.root,
                    reason: RetainedReason::Start(error),
                }
            }
            (Self::Binding { thread, .. }, Answer::WorktreeBound(Ok(()))) => {
                let agents = &mut Rc::make_mut(&mut app.screens).agents;
                agents.observe(backend, thread.as_str());
                agents.display(backend);
                agents::finish_start(agents, thread.as_str());
                attach_thread(app, &thread, backend, Origin::Started, fx);
                show_held_thread(app, thread, fx);
                return;
            }
            (Self::Binding { checkout, thread }, Answer::WorktreeBound(Err(error))) => {
                Self::Unsubscribing {
                    root: checkout.root,
                    error,
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
            (Self::Unsubscribing { root, error }, _) => agents::Notice::WorktreeRetained {
                root,
                reason: RetainedReason::Bind(error),
            },
            _ => return,
        };
        Rc::make_mut(&mut app.screens).agents.notice = Some(notice);
    }
}
