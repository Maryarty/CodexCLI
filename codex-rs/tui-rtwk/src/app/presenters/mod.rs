//! The surface presenters (ADR-013 Ж): where the one-shots the features leave (`SurfaceWish`)
//! are shown. The presenter is the only place that says a wish was shown (`Intent::WishShown`,
//! rules test `adr013_wishes_shown_by_the_presenter`), so a wish is shown once, and the feature
//! never learns how. How a wish looks is the feature's view (`FeatureView::shown`): a sheet that
//! is open for the wish's scope shows it from its own slice of the feature's state, else a sheet
//! is opened that does; this file only does what is the same for every feature. `present` runs
//! after each ingested event.
//!
//! Whether a sheet is open for the scope is read where core keeps it: the counter of interests
//! (`Shown::held_by`), which a sheet raises while it is mounted. No registry of open sheets is
//! kept here, so a sheet open for another scope is never taken for this one and a sheet that
//! closes leaves nothing behind. (Two wishes for a scope made in one flush, before its sheet is
//! mounted, open two sheets; the sheets then show the same page twice, which is harmless.)

use rtwk::AnyWidget;
use rtwk::app::{AppCx, AppDialog, ViewRoot};
use rtwk::overlay::DialogOptions;

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::Scope;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::select::{
    Intent as SelectIntent, ModelCompletion, ModelResult, QueuedModelShow, apps, permissions,
};
use codex_tui_core::screens::transcript::Intent as TranscriptIntent;
use codex_tui_core::screens::{ScreenIntent, ScreenThreadIntent, chat};
use codex_tui_core::threads::queue::{
    CopyChoice, ModalOperation, ModalReceipt, QueueEvent, QueueOperation, StatusLineChoice,
};

use crate::app::UiApp;
use crate::features::InPlace;

/// What the presenter keeps: plain data, UI-only (`UiApp.ui`).
#[derive(Default)]
pub struct Presenters {
    /// The number of the last wish taken: a wish still queued while its `WishShown` is on its
    /// way is not shown again.
    taken: u64,
    /// The same for the select screen's shows (`select::Show`).
    taken_select: u64,
    /// The same for the static pager's documents (`transcript::Directive`).
    taken_pager: u64,
    queue_dialogs: Vec<(ModalOperation, QueueDialog)>,
}

/// The result type of each native dialog is the one its issued token accepts.
enum QueueDialog {
    Copy(AppDialog<CopyChoice>),
    StatusLine(AppDialog<StatusLineChoice>),
    Permissions(AppDialog<permissions::Selection>),
    Apps(AppDialog<apps::ModalChoice>),
}

impl QueueDialog {
    fn retain(&self, active: bool) -> bool {
        fn open<T: 'static>(dialog: &AppDialog<T>, active: bool) -> bool {
            if !active {
                dialog.close(None);
            }
            !dialog.is_closed()
        }

        match self {
            Self::Copy(dialog) => open(dialog, active),
            Self::StatusLine(dialog) => open(dialog, active),
            Self::Permissions(dialog) => open(dialog, active),
            Self::Apps(dialog) => open(dialog, active),
        }
    }
}

/// Shows the wishes of the features and the sheets the select screen asked for that are waiting
/// and can be shown now.
pub fn present(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let core = &app.core;
    app.ui
        .presenters
        .queue_dialogs
        .retain(|(operation, dialog)| dialog.retain(core.queue_modal_visible(operation)));
    present_wishes(app, cx);
    present_shows(app, cx);
    present_pagers(app, cx);
}

/// The static pager's documents: a pager the size of the screen opens over the chat for each
/// (`Overlay::new_static_with_lines`), and the directive is said shown.
fn present_pagers(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let pending: Vec<_> = app
        .core
        .screens()
        .transcript
        .pending()
        .filter(|directive| directive.seq > app.ui.presenters.taken_pager)
        .cloned()
        .collect();
    for directive in pending {
        let Some(place) = app.ui.presenter.place().get() else {
            return;
        };
        let doc = directive.value.clone();
        let dialog = cx.show_dialog_from::<()>(
            place,
            move |_| crate::pages::session::transcript::pager::covering(doc.clone()),
            crate::pages::session::transcript::pager::cover_options(),
            |_app, _answer, _cx| {},
        );
        if dialog.is_closed() {
            return;
        }
        app.ui.presenters.taken_pager = directive.seq;
        app.send(AppEv::Intent(Intent::Screen(ScreenIntent::Transcript(
            TranscriptIntent::Shown(directive.seq),
        ))));
    }
}

/// The select screen's shows: a sheet opens for each, built from what the model asked for.
fn present_shows(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let shows: Vec<_> = app
        .core
        .screens()
        .select
        .shows
        .pending()
        .filter(|show| show.seq > app.ui.presenters.taken_select)
        .cloned()
        .collect();
    for show in shows {
        if let Some(operation) = show.queue_modal {
            if !app.core.queue_modal_visible(&operation) {
                finish_queue_modal(app, operation.cancelled());
            } else if let Some((sheet, options)) =
                crate::sheets::select::owned(&show.opening, &operation)
            {
                let Some(place) = app.ui.presenter.place().get() else {
                    return;
                };
                show_queue_modal(app, cx, place, operation, move || sheet.clone(), options);
            } else {
                finish_queue_modal(app, operation.cancelled());
            }
            app.ui.presenters.taken_select = show.seq;
            app.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(
                SelectIntent::Shown(show.seq),
            ))));
            continue;
        }
        let operation = show
            .queued_model
            .as_ref()
            .map(|queued| queued.operation().clone());
        let admissible = match &show.queued_model {
            Some(QueuedModelShow::Requested(operation)) => {
                app.core.session().active.as_ref() == Some(operation.thread())
            }
            Some(QueuedModelShow::Withdrawn(_)) => false,
            None => true,
        };
        let opened = if let Some(place) = app.ui.presenter.place().get()
            && admissible
        {
            if let Some(operation) = operation.clone() {
                if let Some((sheet, options)) =
                    crate::sheets::select::queued_model(&show.opening, &operation)
                {
                    let dialog = cx.show_dialog_from::<ModelResult>(
                        place,
                        move |_| sheet.clone(),
                        options,
                        move |app, answer, _cx| match answer {
                            Some(result) => {
                                app.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(
                                    SelectIntent::Settle(ModelCompletion::Queued {
                                        operation,
                                        result,
                                    }),
                                ))));
                            }
                            None => queue_receipt(app, operation, QueueEvent::PickerClosed),
                        },
                    );
                    !dialog.is_closed()
                } else {
                    false
                }
            } else {
                let opening = show.opening;
                let options = crate::sheets::select::shown(&opening).1;
                let dialog = cx.show_dialog_from::<()>(
                    place,
                    move |_| crate::sheets::select::shown(&opening).0,
                    options,
                    |_app, _answer, _cx| {},
                );
                !dialog.is_closed()
            }
        } else {
            false
        };
        if !opened && operation.is_none() {
            return;
        }
        app.ui.presenters.taken_select = show.seq;
        app.send(AppEv::Intent(Intent::Screen(ScreenIntent::Select(
            SelectIntent::Shown(show.seq),
        ))));
        if let Some(operation) = operation {
            if opened {
                queue_receipt(app, operation, QueueEvent::PickerOpened);
            } else {
                queue_receipt(app, operation, QueueEvent::PickerClosed);
            }
        }
    }
}

fn queue_receipt(
    app: &mut UiApp,
    operation: QueueOperation,
    receipt: impl FnOnce(QueueOperation) -> QueueEvent,
) {
    let thread = operation.thread().clone();
    thread_queue_event(app, thread, receipt(operation));
}

fn thread_queue_event(app: &mut UiApp, thread: ThreadId, event: QueueEvent) {
    app.send(AppEv::Intent(Intent::ThreadScreen {
        thread,
        intent: ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(event)),
    }));
}

fn present_wishes(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let wishes: Vec<_> = app
        .core
        .wishes()
        .pending()
        .filter(|wish| wish.seq > app.ui.presenters.taken)
        .map(|wish| {
            (
                wish.seq,
                wish.wish.clone(),
                wish.scope.clone(),
                wish.queue_operation.clone(),
                crate::features::shown(&wish.wish, &wish.scope),
            )
        })
        .collect();
    for (seq, wish, scope, operation, shown) in wishes {
        if let Some(operation) = &operation
            && !app.core.queue_modal_visible(operation)
        {
            finish_queue_modal(app, operation.clone().cancelled());
            app.ui.presenters.taken = seq;
            app.send(AppEv::Intent(Intent::WishShown(seq)));
            continue;
        }
        // `/copy` on the chat's own transcript is copy mode, not a sheet (`show_copy_picker`):
        // the picker is for a transcript the chat does not own, or where Find or another mode
        // holds the keys (`FeatureView::in_place`)
        if operation.is_none()
            && app.core.tui().transcript_owned
            && let Scope::Thread(thread) = &scope
        {
            match crate::features::in_place(&wish, &app.ui.thread_ui.get(Some(thread))) {
                InPlace::Not => {}
                taken => {
                    app.ui.presenters.taken = seq;
                    app.send(AppEv::Intent(Intent::WishShown(seq)));
                    if let InPlace::Declined(intent) = taken {
                        app.send(AppEv::Intent(Intent::Feature(intent)));
                    }
                    continue;
                }
            }
        }
        if let Some(shown) = shown
            && shown
                .held_by
                .as_ref()
                .is_none_or(|held_by| app.core.interests().count(held_by) == 0)
        {
            let Some(place) = app.ui.presenter.place().get() else {
                // no home to show from yet: the wish stays queued for the next event
                return;
            };
            let open = shown.open.clone();
            if let Some(operation) = operation {
                show_queue_modal(app, cx, place, operation, move || open(), shown.options);
            } else {
                let dialog = cx.show_dialog_from::<()>(
                    place,
                    move |_| open(),
                    shown.options,
                    |_app, _answer, _cx| {},
                );
                if dialog.is_closed() {
                    return;
                }
            }
        } else if let Some(operation) = operation {
            finish_queue_modal(app, operation.cancelled());
        }
        app.ui.presenters.taken = seq;
        app.send(AppEv::Intent(Intent::WishShown(seq)));
    }
}

fn show_queue_modal(
    app: &mut UiApp,
    cx: &mut AppCx<'_, '_, UiApp>,
    place: ViewRoot<UiApp>,
    operation: ModalOperation,
    open: impl Fn() -> AnyWidget + 'static,
    options: DialogOptions,
) {
    let owner = operation.thread().clone();
    let dialog = match operation.clone() {
        ModalOperation::Copy(token) => QueueDialog::Copy(cx.show_dialog_from(
            place,
            move |_| open(),
            options,
            move |app, answer, _| finish_queue_modal(app, ModalReceipt::Copy(token, answer)),
        )),
        ModalOperation::StatusLine(token) => QueueDialog::StatusLine(cx.show_dialog_from(
            place,
            move |_| open(),
            options,
            move |app, answer, _| finish_queue_modal(app, ModalReceipt::StatusLine(token, answer)),
        )),
        ModalOperation::Permissions(token) => QueueDialog::Permissions(cx.show_dialog_from(
            place,
            move |_| open(),
            options,
            move |app, answer, _| finish_queue_modal(app, ModalReceipt::Permissions(token, answer)),
        )),
        ModalOperation::Apps(token) => QueueDialog::Apps(cx.show_dialog_from(
            place,
            move |_| open(),
            options,
            move |app, answer, _| finish_queue_modal(app, ModalReceipt::Apps(token, answer)),
        )),
    };
    if dialog.retain(true) {
        app.ui
            .presenters
            .queue_dialogs
            .push((operation.clone(), dialog));
        thread_queue_event(app, owner, QueueEvent::ModalOpened(operation));
    }
}

fn finish_queue_modal(app: &mut UiApp, receipt: ModalReceipt) {
    let thread = receipt.thread().clone();
    thread_queue_event(app, thread, QueueEvent::ModalFinished(receipt));
}
