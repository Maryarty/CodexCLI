#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::effects::{Ask, Done, ImageRead, ImageRevision, ImageSize, ImageSnapshot};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Ingress, Intent, Now, apply_app};
use codex_tui_core::features::{InterestStep, images};
use codex_tui_core::ids::{BackendId, ItemId, ThreadId, TurnId};
use codex_tui_core::rpc::{Address, Reply};
use codex_tui_core::threads::ItemKey;

fn step(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, event, &mut fx);
    fx
}

fn ready() -> (CodexApp, images::Interest) {
    let mut app = CodexApp::new();
    let fx = step(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|command| {
            if let Command::StartThread { ticket, .. } = command {
                Some(*ticket)
            } else {
                None
            }
        })
        .expect("thread start");
    let thread = ThreadId::new("thread-1");
    step(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(thread.clone())),
    );
    let interest = images::Interest {
        source: images::Source {
            thread,
            kind: images::SourceKind::Transcript(ItemKey {
                turn: TurnId::new("turn-1"),
                item: ItemId::new("image-1"),
            }),
            path: "/tmp/artifact.png".to_owned(),
        },
        preview: None,
    };
    (app, interest)
}

fn raise(app: &mut CodexApp, source: &images::Interest) -> u64 {
    let fx = step(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(source.clone().into()))),
    );
    snapshot_request(&fx, source)
}

fn snapshot_request(fx: &AppFx, source: &images::Interest) -> u64 {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                request,
                ask:
                    Ask::ImageSnapshot {
                        thread,
                        path,
                        preview,
                        ..
                    },
                ..
            } => {
                assert_eq!(thread, &source.source.thread);
                assert_eq!(path, &source.source.path);
                assert_eq!(*preview, source.preview);
                Some(*request)
            }
            _ => None,
        })
        .expect("the source's image snapshot request")
}

#[test]
fn an_image_read_is_reissued_for_the_same_row_after_physical_replacement() {
    let (mut app, source) = ready();
    let backend = BackendId::default();
    let epoch = app.epoch_of(backend);
    let next = epoch.next();
    let stale = raise(&mut app, &source);
    let effects = app
        .ingest(Ingress::transport_replaced(backend, epoch, next))
        .expect("the same connector installed a new handle");
    let current = snapshot_request(&effects, &source);
    let command = effects
        .commands()
        .iter()
        .find(|command| {
            matches!(
                command, Command::Effect { request, ask: Ask::ImageSnapshot { .. }, .. }
                    if *request == current
            )
        })
        .expect("image ask");
    assert_eq!(
        app.dispatch_route(command, None),
        Some((
            backend,
            Some(Address {
                backend,
                epoch: next
            })
        ))
    );

    for (source_backend, stamp, request) in [
        (backend, epoch, stale),
        (backend, epoch, current),
        (BackendId(1), epoch, current),
    ] {
        let response = image_answer(request, "file:///tmp/old.png");
        assert!(
            app.ingest(Ingress::transport_event(
                source_backend,
                epoch,
                stamp,
                response
            ))
            .is_none()
        );
    }
    assert!(app.features().images.snapshot(&source).is_none());

    app.ingest(Ingress::transport_event(
        backend,
        epoch,
        next,
        image_answer(current, "file:///tmp/fresh.png"),
    ))
    .expect("the response on the new handle");
    let snapshot = app
        .features()
        .images
        .snapshot(&source)
        .expect("current snapshot");
    assert_eq!(snapshot.link.as_deref(), Some("file:///tmp/fresh.png"));
}

fn close(app: &mut CodexApp, source: &images::Interest) {
    step(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(source.clone().into()))),
    );
}

fn image_answer(request: u64, url: &str) -> AppEv {
    AppEv::Effect {
        request,
        done: Done::ImageSnapshot(ImageRead::Snapshot(ImageSnapshot {
            link: Some(url.to_owned()),
            ..Default::default()
        })),
    }
}

#[test]
fn a_remounted_artifact_ignores_the_previous_reads_late_answer() {
    let (mut app, source) = ready();
    let stale = raise(&mut app, &source);
    close(&mut app, &source);
    let current = raise(&mut app, &source);
    step(&mut app, image_answer(current, "file:///tmp/new.png"));
    step(&mut app, image_answer(stale, "file:///tmp/old.png"));
    let image = app.features().images.snapshot(&source).expect("new read");
    assert_eq!(image.link.as_deref(), Some("file:///tmp/new.png"));
    close(&mut app, &source);
    assert!(app.features().images.snapshot(&source).is_none());
}

#[test]
fn the_open_action_requires_a_held_local_link_of_its_source_thread() {
    let (mut app, source) = ready();
    let current = raise(&mut app, &source);
    let link = "file:///tmp/artifact.png";
    step(&mut app, image_answer(current, link));
    let open = |thread: ThreadId, url: &str| {
        AppEv::Intent(Intent::Feature(
            images::Intent::Open(images::Open {
                thread,
                url: url.to_owned(),
            })
            .into(),
        ))
    };
    assert!(
        step(&mut app, open(ThreadId::new("other-thread"), link))
            .commands()
            .is_empty()
    );
    assert!(
        step(
            &mut app,
            open(source.source.thread.clone(), "file:///tmp/other.png")
        )
        .commands()
        .is_empty()
    );
    let fx = step(&mut app, open(source.source.thread.clone(), link));
    assert!(fx.commands().iter().any(|command| matches!(
        command, Command::Effect { ask: Ask::OpenUrl { url, .. }, .. } if url == link
    )));
    close(&mut app, &source);
    assert!(
        step(&mut app, open(source.source.thread, link))
            .commands()
            .is_empty()
    );
}

#[test]
fn mounted_attachment_refresh_is_conditional_and_cannot_outlive_its_interest() {
    let (mut app, mut source) = ready();
    source.source.kind = images::SourceKind::Attachment(7);
    source.preview = Some(ImageSize {
        width: 48,
        height: 48,
    });
    let answer = |request, result| AppEv::Effect {
        request,
        done: Done::ImageSnapshot(result),
    };
    let clock = |millis| AppEv::Clock(Now::at_millis(millis));
    let snapshot = |revision| {
        ImageRead::Snapshot(ImageSnapshot {
            revision: Some(ImageRevision(revision)),
            ..Default::default()
        })
    };

    let initial = raise(&mut app, &source);
    step(&mut app, answer(initial, snapshot(24)));
    assert!(step(&mut app, clock(999)).commands().is_empty());
    let fx = step(&mut app, clock(1000));
    let pending = snapshot_request(&fx, &source);
    assert!(fx.commands().iter().any(|command| matches!(
        command,
        Command::Effect {
            ask: Ask::ImageSnapshot {
                previous: Some(ImageRevision(24)),
                ..
            },
            ..
        }
    )));
    assert!(step(&mut app, clock(2000)).commands().is_empty());
    step(&mut app, answer(pending, ImageRead::Unchanged));
    assert_eq!(
        app.features().images.snapshot(&source).unwrap().revision,
        Some(ImageRevision(24))
    );

    let late = snapshot_request(&step(&mut app, clock(3000)), &source);
    close(&mut app, &source);
    assert!(step(&mut app, clock(4000)).commands().is_empty());
    let reopened = raise(&mut app, &source);
    step(&mut app, answer(reopened, snapshot(25)));
    step(&mut app, answer(late, snapshot(26)));
    assert_eq!(
        app.features().images.snapshot(&source).unwrap().revision,
        Some(ImageRevision(25))
    );
}
