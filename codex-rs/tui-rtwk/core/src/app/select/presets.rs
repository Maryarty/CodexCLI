//! A confirmed preset write keeps the native dialog's queue hold until the config answers.

use std::rc::Rc;

use crate::app::{AppFx, CodexApp, features};
use crate::rpc::RpcError;
use crate::screens::select::quick_dials::Write;
use crate::screens::select::{self, Opening};
use crate::threads::queue::{ModalKind, ModalOperation, ModalToken};

pub(in crate::app) fn apply(
    app: &mut CodexApp,
    token: ModalToken<Write>,
    write: Write,
    fx: &mut AppFx,
) {
    if let Err(note) = write.check_against(&app.screens.select.quick) {
        let operation = ModalOperation::Preset(token);
        super::write_notes_on(app, operation.thread(), [note], fx);
        restore_editor(app, &operation, write, fx);
        features::acknowledge_queue_modal(app, operation, fx);
        return;
    }
    super::ask(
        app,
        select::Command::Preset {
            slot: write.slot,
            dial: write.dial.clone(),
        },
        select::Ctx::OwnedPreset { token, write },
        fx,
    );
}

pub(super) fn answered(
    app: &mut CodexApp,
    token: ModalToken<Write>,
    write: Write,
    result: Result<select::Reply, RpcError>,
    fx: &mut AppFx,
) {
    let operation = ModalOperation::Preset(token);
    let result = match result {
        Ok(select::Reply::Written { .. }) => Ok(()),
        Ok(_) => Err(super::text(RpcError::Mismatch)),
        Err(error) => Err(super::text(error)),
    };
    let failed = result.is_err();
    let note = Rc::make_mut(&mut app.screens)
        .select
        .quick
        .write_result(&write, result);
    super::write_notes_on(app, operation.thread(), [note], fx);
    if failed {
        restore_editor(app, &operation, write, fx);
    }
    features::acknowledge_queue_modal(app, operation, fx);
}

fn restore_editor(app: &mut CodexApp, operation: &ModalOperation, write: Write, fx: &mut AppFx) {
    if let Some(menu) = write.retry_editor()
        && features::queue_modal_can_reopen(app, operation)
        && let Some(next) =
            features::register_queue_modal(app, operation.thread(), None, ModalKind::Preset, fx)
    {
        // Register the replacement before returning the old hold. The queue stays blocked
        // and a reply for the previous operation cannot close this editor.
        Rc::make_mut(&mut app.screens)
            .select
            .shows
            .push_modal(Opening::Quick(Box::new(menu)), Some(next));
    }
}
