//! The command center over more than one backend (the local server and the
//! cloud): rows keep their owner, pins are kept per backend, a refresh of one
//! backend never reorders the other's. Ported from the oracle
//! (`app/agents_overview_tests.rs`: `refreshing_one_backends_pins_preserves_cross_backend_rank`
//! at 181, `overview_keeps_rows_and_routes_actions_to_the_owner` at 223,
//! codex-tui 0.170.0). The oracle's refresh result carries the backend that
//! was asked; here the page does (`Ev::ListedBy`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::Thread;
use codex_tui_core::app::AppFx;
use codex_tui_core::ids::BackendId;
use codex_tui_core::screens::agents::{Ev, Model, Page, Pins, apply};
use serde_json::json;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread`: an idle root named `name`.
fn thread(id: &str, name: &str) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn page(threads: Vec<Thread>, pins: Option<Vec<String>>) -> Page {
    Page {
        threads,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: Some(Pins {
            ids: pins,
            threads: Vec::new(),
        }),
    }
}

fn listed(model: &mut Model, owner: BackendId, page: Page) {
    apply(model, Ev::ListedBy(owner, page), &mut AppFx::default());
}

/// `refreshing_one_backends_pins_preserves_cross_backend_rank`: the pins of
/// the local backend and the cloud keep one order after each refreshes in turn.
#[test]
fn refreshing_one_backends_pins_preserves_cross_backend_rank() {
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    let (local_pin, cloud_pin) = (id(1), id(2));
    let mut previous = None;
    for (owner, pin) in [
        (local, &local_pin),
        (cloud, &cloud_pin),
        (local, &local_pin),
    ] {
        listed(&mut model, owner, page(Vec::new(), Some(vec![pin.clone()])));
        let ranks = model.pinned.clone().expect("pins");
        if owner == cloud {
            assert_eq!(ranks.len(), 2);
            previous = Some(ranks);
        } else if let Some(expected) = &previous {
            assert_eq!(&ranks, expected);
        }
    }
}

/// `overview_keeps_rows_and_routes_actions_to_the_owner`, the rows' part: a
/// thread belongs to the first backend that listed it; the displayed
/// backend's pins stay `None` when it has none, the other's are kept; a row of
/// an id another backend owns is dropped.
#[test]
fn overview_keeps_rows_and_owners_per_backend() {
    let mut model = Model::default();
    let (local, remote) = (BackendId(1), BackendId(2));
    model.displayed = local;
    let (local_id, remote_id, colliding_id) = (id(1), id(2), id(3));
    listed(
        &mut model,
        local,
        page(
            vec![
                thread(&local_id, "Local task"),
                thread(&colliding_id, "Name from the displayed backend"),
            ],
            None,
        ),
    );
    assert_eq!(model.owners.get(&remote_id), None);
    assert_eq!(model.pins_by_backend.get(&local), Some(&None));
    listed(
        &mut model,
        remote,
        page(
            vec![
                thread(&remote_id, "Remote task"),
                thread(&colliding_id, "Colliding task"),
            ],
            Some(Vec::new()),
        ),
    );
    assert_eq!(model.pins_by_backend.get(&remote), Some(&Some(Vec::new())));
    assert_eq!(model.owners.get(&remote_id), Some(&remote));
    assert_eq!(model.owners.get(&colliding_id), Some(&local));
    // The oracle's rows are a map; the order is not asserted.
    let mut names: Vec<_> = model
        .threads
        .iter()
        .map(|held| held.name.clone().unwrap_or_default())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "Local task",
            "Name from the displayed backend",
            "Remote task"
        ]
    );
}
