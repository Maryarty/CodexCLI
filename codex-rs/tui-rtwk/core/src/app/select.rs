//! The RPCs of the select screen (S14) the app sends and reads: `model/list`
//! for `/model`, and `configRequirements/read` followed by the pages of
//! `permissionProfile/list` for `/permissions`. The screen's model decides
//! what to ask (`Catalog::pending`, `Profiles::pending`: the request an
//! answer must belong to); the app gives each a ticket, sends it as a screen
//! command with no thread and turns the answer into the screen's events.

use std::rc::Rc;

use super::{AppCtx, AppFx, CodexApp, Command};
use crate::ids::ThreadId;
use crate::rpc::{Answer, RpcError};
use crate::screens::select::{self, permissions};
use crate::screens::{ScreenCommand, ScreenReply};

/// An intent of the select screen, and the asks it left open.
pub(super) fn intent(app: &mut CodexApp, intent: select::Intent, fx: &mut AppFx) {
    let screens = Rc::make_mut(&mut app.screens);
    let before = (
        screens.select.catalog.pending,
        screens.select.profiles.pending,
    );
    select::apply_intent(&mut screens.select, intent, fx);
    let after = (
        screens.select.catalog.pending,
        screens.select.profiles.pending,
    );
    if let Some(request) = after.0.filter(|_| after.0 != before.0) {
        ask(
            app,
            select::Command::Models { request },
            select::Ctx::Models { request },
            fx,
        );
    }
    if let Some(request) = after.1.filter(|_| after.1 != before.1) {
        ask(
            app,
            select::Command::Requirements { request },
            select::Ctx::Requirements { request },
            fx,
        );
    }
}

fn ask(app: &mut CodexApp, command: select::Command, ctx: select::Ctx, fx: &mut AppFx) {
    let ticket = app.mint.next();
    app.outgoing.insert(ticket, AppCtx::Select(ctx));
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Select(command),
    });
}

fn failure(error: RpcError) -> permissions::Failure {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) => permissions::Failure::Other(text),
        RpcError::Mismatch => permissions::Failure::Other(String::new()),
    }
}

fn text(error: RpcError) -> String {
    match failure(error) {
        permissions::Failure::Other(text) => text,
        _ => String::new(),
    }
}

/// The answer of one of those RPCs.
pub(super) fn answer(app: &mut CodexApp, ctx: select::Ctx, answer: Answer, fx: &mut AppFx) {
    let result = match answer {
        Answer::Screen(ScreenReply::Select(reply)) => Ok(reply),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    let event = |app: &mut CodexApp, ev: select::Ev, fx: &mut AppFx| {
        select::apply(&mut Rc::make_mut(&mut app.screens).select, ev, fx);
    };
    match (ctx, result) {
        (select::Ctx::Models { request }, result) => {
            let result = match result {
                Ok(select::Reply::Models(presets)) => Ok(presets),
                Ok(_) => Err(String::new()),
                Err(error) => Err(text(error)),
            };
            event(app, select::Ev::Fetched { request, result }, fx);
        }
        (select::Ctx::Requirements { request }, result) => match result {
            Ok(select::Reply::Requirements(requirements)) => {
                let ctx = select::Ctx::Profiles {
                    request,
                    requirements,
                    found: Vec::new(),
                    pages: 0,
                };
                ask_page(app, ctx, None, fx);
            }
            Ok(_) => discovered(
                app,
                request,
                Err(permissions::Failure::Other(String::new())),
                fx,
            ),
            Err(error) => discovered(app, request, Err(failure(error)), fx),
        },
        (
            select::Ctx::Profiles {
                request,
                requirements,
                mut found,
                pages,
            },
            result,
        ) => match result {
            Ok(select::Reply::Profiles { profiles, next }) => {
                found.extend(profiles);
                let pages = pages + 1;
                match next {
                    Some(_) if pages >= select::MAX_PAGES => {
                        discovered(app, request, Err(permissions::Failure::Pagination), fx);
                    }
                    Some(cursor) => {
                        let ctx = select::Ctx::Profiles {
                            request,
                            requirements,
                            found,
                            pages,
                        };
                        ask_page(app, ctx, Some(cursor), fx);
                    }
                    None => discovered(
                        app,
                        request,
                        permissions::Discovery::new(found, requirements),
                        fx,
                    ),
                }
            }
            Ok(_) => discovered(
                app,
                request,
                Err(permissions::Failure::Other(String::new())),
                fx,
            ),
            Err(error) => discovered(app, request, Err(failure(error)), fx),
        },
    }
}

fn ask_page(app: &mut CodexApp, ctx: select::Ctx, cursor: Option<String>, fx: &mut AppFx) {
    let select::Ctx::Profiles { request, .. } = ctx else {
        return;
    };
    let command = select::Command::Profiles {
        request,
        cursor,
        cwd: None,
    };
    ask(app, command, ctx, fx);
}

fn discovered(
    app: &mut CodexApp,
    request: u64,
    result: Result<permissions::Discovery, permissions::Failure>,
    fx: &mut AppFx,
) {
    select::apply(
        &mut Rc::make_mut(&mut app.screens).select,
        select::Ev::Discovered { request, result },
        fx,
    );
}
