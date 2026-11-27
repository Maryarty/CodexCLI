//! Enrich accepted local sends without putting a selection in the draft or queue.

use std::collections::BTreeMap;

use super::effects::{self, Ask, IdeFailure, IdePermit, IdePrompt};
use super::{AppFx, CodexApp, Command, Now, Wake, apply_thread, features};
use crate::features::{ide_opted_in, ide_permit, settle_ide_submit};
use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::Reply;
use crate::screens::{ScreenCommand, chat};
use crate::threads::ThreadEv;

// The oracle budgets five seconds for extension discovery and the actual read.
const READ_MILLIS: i64 = 5_000;

type Reading = Result<Option<IdePrompt>, IdeFailure>;

#[derive(Debug, Default)]
pub(super) struct State {
    waiting: BTreeMap<u64, Pending>,
}

#[derive(Debug)]
struct Pending {
    ticket: Ticket,
    command: Command,
    permit: IdePermit,
    deadline: Now,
    reading: Option<Reading>,
}

/// Called after the normal reducer registered the StartTurn or Steer. It sees
/// actual wire sends, including queue drain and steer retries.
pub(super) fn prepare(app: &mut CodexApp, fx: &mut AppFx, emitted_from: usize) {
    app.ide_submissions.waiting.retain(|&request, pending| {
        let Command::Screen {
            thread,
            command: ScreenCommand::Chat(chat::Command::Steer { client_id, .. }),
            ..
        } = &pending.command
        else {
            return true;
        };
        let keep = app
            .threads
            .retain_preparing_steer(thread, pending.ticket, client_id);
        if !keep {
            app.effects.take_ide_submit(request);
        }
        keep
    });

    if ide_opted_in(&app.features) && fx.commands[emitted_from..].iter().any(is_submission) {
        for command in fx.commands.split_off(emitted_from) {
            let Some((ticket, thread)) = submission(&command) else {
                fx.commands.push(command);
                continue;
            };
            let permit = ide_permit(&app.features, features::view(app), thread);
            let (Some(permit), Some(address)) = (permit, app.request_address(ticket)) else {
                fx.commands.push(command);
                continue;
            };
            let ask = Ask::IdePrompt {
                thread: permit.thread.clone(),
                cwd: permit.cwd.clone(),
            };
            let request = app
                .effects
                .ask_for_ide_submit(permit.thread.clone(), address);
            app.ide_submissions.waiting.insert(
                request,
                Pending {
                    ticket,
                    command,
                    permit,
                    deadline: app.now.plus_millis(READ_MILLIS),
                    reading: None,
                },
            );
            fx.commands.push(effects::command(request, ask));
        }
    }

    // A later read can return first. Release sends in the order they were accepted.
    while let Some((&request, pending)) = app.ide_submissions.waiting.first_key_value() {
        let permitted = still_permitted(app, &pending.permit);
        let expired = app.now.reached(pending.deadline);
        if pending.reading.is_none() && permitted && !expired {
            let wake = Wake::new(app.now, pending.deadline);
            if fx
                .wake
                .is_none_or(|current| wake.at().millis() < current.at().millis())
            {
                fx.wake = Some(wake);
            }
            break;
        }
        let Some(mut pending) = app.ide_submissions.waiting.remove(&request) else {
            break;
        };
        app.effects.take_ide_submit(request);
        if !app
            .request_address(pending.ticket)
            .is_some_and(|address| app.address(address.backend) == address)
        {
            restore(app, pending, fx);
            continue;
        }
        if permitted {
            // A result received while this send waited behind an earlier one
            // remains usable when the earlier read times out.
            let reading = pending.reading.take();
            features::run(app, fx, |states, cx| {
                settle_ide_submit(states, &pending.permit, reading.as_ref(), cx);
            });
            if let Some(Ok(Some(prompt))) = reading {
                attach(&mut pending.command, prompt);
            }
        }
        fx.commands.push(pending.command);
    }
}

/// The generic workspace receipt gate has checked the physical address and kind.
pub(super) fn answered(app: &mut CodexApp, request: u64, reading: Reading) {
    if app.effects.take_ide_submit(request).is_some()
        && let Some(pending) = app.ide_submissions.waiting.get_mut(&request)
    {
        pending.reading = Some(reading);
    }
}

/// These commands never crossed the wire. Settle them before ordinary connection
/// loss can classify them as admissions whose outcome is unknown.
pub(super) fn retire(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    retire_matching(app, fx, |app, pending| {
        app.request_address(pending.ticket)
            .is_some_and(|address| address.backend == backend)
    });
}

pub(super) fn shutdown(app: &mut CodexApp, fx: &mut AppFx) {
    retire_matching(app, fx, |_, _| true);
}

pub(super) fn retire_thread(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    retire_matching(app, fx, |_, pending| &pending.permit.thread == thread);
}

fn retire_matching(
    app: &mut CodexApp,
    fx: &mut AppFx,
    matches: impl Fn(&CodexApp, &Pending) -> bool,
) {
    let unsent: Vec<_> = app
        .ide_submissions
        .waiting
        .iter()
        .filter_map(|(&request, pending)| matches(app, pending).then_some(request))
        .collect();
    for request in unsent {
        app.effects.take_ide_submit(request);
        if let Some(pending) = app.ide_submissions.waiting.remove(&request) {
            restore(app, pending, fx);
        }
    }
}

fn still_permitted(app: &CodexApp, permit: &IdePermit) -> bool {
    ide_permit(&app.features, features::view(app), &permit.thread).as_ref() == Some(permit)
}

fn restore(app: &mut CodexApp, pending: Pending, fx: &mut AppFx) {
    apply_thread(
        app,
        &pending.permit.thread,
        ThreadEv::Reply {
            ticket: pending.ticket,
            reply: Reply::prepare_failed("The connection changed before the message was sent."),
        },
        fx,
    );
}

fn submission(command: &Command) -> Option<(Ticket, &ThreadId)> {
    match command {
        Command::StartTurn { ticket, thread, .. }
        | Command::Screen {
            ticket,
            thread,
            command: ScreenCommand::Chat(chat::Command::Steer { .. }),
        } => Some((*ticket, thread)),
        _ => None,
    }
}

fn is_submission(command: &Command) -> bool {
    submission(command).is_some()
}

fn attach(command: &mut Command, prompt: IdePrompt) {
    match command {
        Command::StartTurn { ide_prompt, .. }
        | Command::Screen {
            command: ScreenCommand::Chat(chat::Command::Steer { ide_prompt, .. }),
            ..
        } => *ide_prompt = Some(prompt),
        _ => {}
    }
}
