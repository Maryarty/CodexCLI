#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the facts the footer and the status line read besides the session's own
//! (task chunk-status-facts): the collaboration mode, the side and agent labels, the pending
//! token usage of a resumed thread, the git summary probed per cwd, the workspace headline
//! throttled by its due time. Expected values are the oracle's
//! (`chatwidget/status_surfaces.rs`, `branch_summary.rs`, `workspace_messages.rs`,
//! `app/agent_navigation.rs`).

use codex_tui_core::app::{AppFx, CodexApp, Now, apply_thread};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::chat::facts::{
    self, BranchChanges, GitSummary, HeadlineAnswer, PullRequest,
};
use codex_tui_core::screens::chat::{Probe, ThreadEv as ChatThreadEv};
use codex_tui_core::screens::{ScreenThreadEv, chat};
use codex_tui_core::threads::ThreadEv;
use serde_json::json;

fn screen(ev: serde_json::Value) -> String {
    format!(r#"{{"step":"screen","dir":"chat","ev":{ev}}}"#)
}

fn play(lines: &[String]) -> Sandbox {
    Sandbox::play(&lines.join("\n")).expect("plays")
}

fn facts_of(sandbox: &Sandbox) -> chat::Facts {
    sandbox.app().screens().chat.facts.clone()
}

fn items(ids: &[&str]) -> String {
    screen(json!({"status": {"items": ids}}))
}

fn cwd(path: &str) -> String {
    screen(json!({"facts": {"cwd": path}}))
}

fn clock(now: u64) -> String {
    format!(r#"{{"step":"clock","secs":{now}}}"#)
}

mod git_summary {
    use super::*;
    use pretty_assertions::assert_eq;

    fn summary() -> GitSummary {
        GitSummary {
            pull_request: Some(PullRequest {
                number: 42,
                url: "https://example.com/pr/42".into(),
            }),
            changes: Some(BranchChanges {
                additions: 3,
                deletions: 1,
            }),
        }
    }

    fn answer(request: u64, summary: &GitSummary) -> String {
        screen(
            json!({"facts": {"git_summary": {"request": request, "summary": {
                "pull_request": {"number": summary.pull_request.as_ref().unwrap().number,
                                  "url": summary.pull_request.as_ref().unwrap().url},
                "changes": {"additions": 3, "deletions": 1},
            }}}}),
        )
    }

    #[test]
    fn nothing_is_probed_while_no_item_needs_it() {
        let sandbox = play(&[cwd("/a"), items(&["git-branch", "model"])]);
        assert_eq!(facts_of(&sandbox).probes(), vec![]);
        // the default list has none of them either
        let sandbox = play(&[cwd("/a")]);
        assert_eq!(facts_of(&sandbox).probes(), vec![]);
    }

    #[test]
    fn the_summary_of_the_cwd_is_asked_once_on_open() {
        let sandbox = play(&[items(&["pull-request-number"]), cwd("/a")]);
        assert_eq!(
            facts_of(&sandbox).probes(),
            vec![Probe::GitSummary {
                request: 1,
                cwd: "/a".into()
            }]
        );
        // the same cwd again asks nothing new
        let sandbox = play(&[items(&["branch-changes"]), cwd("/a"), cwd("/a")]);
        assert_eq!(facts_of(&sandbox).probes().len(), 1);
    }

    #[test]
    fn the_answer_fills_the_items_texts_and_ends_the_probe() {
        let sandbox = play(&[
            items(&["pull-request-number", "branch-changes"]),
            cwd("/a"),
            answer(1, &summary()),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.probes(), vec![]);
        let got = facts.git_summary().expect("a summary");
        assert_eq!(got.pull_request_text().as_deref(), Some("PR #42"));
        assert_eq!(got.changes_text().as_deref(), Some("+3 -1"));
    }

    #[test]
    fn no_changes_reads_no_changes() {
        let none = GitSummary {
            pull_request: None,
            changes: Some(BranchChanges {
                additions: 0,
                deletions: 0,
            }),
        };
        assert_eq!(none.changes_text().as_deref(), Some("No changes"));
        assert_eq!(none.pull_request_text(), None);
    }

    #[test]
    fn another_cwd_drops_the_summary_and_asks_again() {
        let sandbox = play(&[
            items(&["pull-request-number"]),
            cwd("/a"),
            answer(1, &summary()),
            cwd("/b"),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.git_summary(), None);
        assert_eq!(
            facts.probes(),
            vec![Probe::GitSummary {
                request: 2,
                cwd: "/b".into()
            }]
        );
    }

    #[test]
    fn an_answer_of_the_earlier_cwd_is_stale() {
        let sandbox = play(&[
            items(&["pull-request-number"]),
            cwd("/a"),
            cwd("/b"),
            answer(1, &summary()),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.git_summary(), None);
        assert_eq!(
            facts.probes(),
            vec![Probe::GitSummary {
                request: 2,
                cwd: "/b".into()
            }]
        );
    }

    #[test]
    fn leaving_the_items_forgets_the_summary() {
        let sandbox = play(&[
            items(&["pull-request-number"]),
            cwd("/a"),
            answer(1, &summary()),
            items(&["model"]),
        ]);
        assert_eq!(facts_of(&sandbox).git_summary(), None);
        // and putting it back reads it again
        let sandbox = play(&[
            items(&["pull-request-number"]),
            cwd("/a"),
            answer(1, &summary()),
            items(&["model"]),
            items(&["branch-changes"]),
        ]);
        assert_eq!(
            facts_of(&sandbox).probes(),
            vec![Probe::GitSummary {
                request: 2,
                cwd: "/a".into()
            }]
        );
    }
}

mod workspace_headline {
    use super::*;
    use pretty_assertions::assert_eq;

    fn answer(request: u64, body: serde_json::Value) -> String {
        screen(json!({"facts": {"headline": {"request": request, "answer": body}}}))
    }

    const ITEM: &str = "workspace-headline";

    #[test]
    fn it_is_read_once_when_the_item_is_on_and_the_account_is_signed_in() {
        let sandbox = play(&[clock(1_000), items(&[ITEM])]);
        assert_eq!(
            facts_of(&sandbox).probes(),
            vec![Probe::Headline { request: 1 }]
        );
        // not while the line has no such item
        let sandbox = play(&[clock(1_000), items(&["model"])]);
        assert_eq!(facts_of(&sandbox).probes(), vec![]);
        // not without a ChatGPT backend sign-in
        let sandbox = play(&[
            screen(json!({"signed_in": false})),
            clock(1_000),
            items(&[ITEM]),
        ]);
        assert_eq!(facts_of(&sandbox).probes(), vec![]);
    }

    #[test]
    fn the_answer_is_kept_and_the_next_read_waits_the_interval() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            clock(1_000 + facts::HEADLINE_REFRESH_SECS - 1),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.headline(), Some("Ship it"));
        assert_eq!(facts.probes(), vec![]);
        assert_eq!(
            facts.headline_due_at(),
            Some(1_000 + facts::HEADLINE_REFRESH_SECS)
        );

        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            clock(1_000 + facts::HEADLINE_REFRESH_SECS),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.headline(), Some("Ship it"));
        assert_eq!(facts.probes(), vec![Probe::Headline { request: 2 }]);
    }

    /// The chat declares the next read as a wake (`chat::wake_at`): the shell's one timer tells
    /// the clock then, and the read is made. Without a ChatGPT sign-in no read would be made, so
    /// no moment is asked for.
    #[test]
    fn the_next_read_is_a_wake_the_chat_declares() {
        let wake = |sandbox: &Sandbox| chat::wake_at(&sandbox.app().screens().chat);
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
        ]);
        assert_eq!(
            wake(&sandbox),
            Some(Now::at_secs(1_000 + facts::HEADLINE_REFRESH_SECS as i64))
        );
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            screen(json!({"signed_in": false})),
        ]);
        assert_eq!(wake(&sandbox), None);
        let sandbox = play(&[clock(1_000), items(&["model"])]);
        assert_eq!(wake(&sandbox), None, "the item is not on the line");
    }

    #[test]
    fn nothing_is_asked_while_a_read_is_in_flight() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            clock(1_000 + 10 * facts::HEADLINE_REFRESH_SECS),
        ]);
        assert_eq!(
            facts_of(&sandbox).probes(),
            vec![Probe::Headline { request: 1 }]
        );
    }

    #[test]
    fn an_answer_of_another_request_is_dropped() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(7, json!({"available": "Old"})),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.headline(), None);
        assert_eq!(facts.probes(), vec![Probe::Headline { request: 1 }]);
    }

    #[test]
    fn a_backend_without_the_feature_is_not_asked_again() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            clock(1_000 + facts::HEADLINE_REFRESH_SECS),
            answer(2, json!("feature_disabled")),
            clock(1_000 + 3 * facts::HEADLINE_REFRESH_SECS),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.headline(), None);
        assert_eq!(facts.probes(), vec![]);
        assert_eq!(facts.headline_due_at(), None);
    }

    #[test]
    fn a_failed_read_keeps_the_old_headline_and_tries_after_the_interval() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            clock(1_000 + facts::HEADLINE_REFRESH_SECS),
            answer(2, json!("failed")),
        ]);
        let facts = facts_of(&sandbox);
        assert_eq!(facts.headline(), Some("Ship it"));
        assert_eq!(facts.probes(), vec![]);
        assert_eq!(
            facts.headline_due_at(),
            Some(1_000 + 2 * facts::HEADLINE_REFRESH_SECS)
        );
    }

    #[test]
    fn leaving_the_item_forgets_the_headline() {
        let sandbox = play(&[
            clock(1_000),
            items(&[ITEM]),
            answer(1, json!({"available": "Ship it"})),
            items(&["model"]),
        ]);
        assert_eq!(facts_of(&sandbox).headline(), None);
    }

    #[test]
    fn the_first_headline_message_is_the_one_shown() {
        let response = |feature_enabled: bool, messages: serde_json::Value| {
            serde_json::from_value(json!({"featureEnabled": feature_enabled, "messages": messages}))
                .expect("response")
        };
        let message = |kind: &str, body: &str| {
            json!({"messageId": "m", "messageType": kind, "messageBody": body,
                   "createdAt": null, "archivedAt": null})
        };
        assert_eq!(
            HeadlineAnswer::from_response(response(false, json!([]))),
            HeadlineAnswer::FeatureDisabled
        );
        assert_eq!(
            HeadlineAnswer::from_response(response(
                true,
                json!([
                    message("announcement", "no"),
                    message("headline", "  "),
                    message("headline", " Hi ")
                ])
            )),
            HeadlineAnswer::Available(Some("Hi".into()))
        );
        assert_eq!(
            HeadlineAnswer::from_response(response(true, json!([]))),
            HeadlineAnswer::Available(None)
        );
    }
}

mod footer {
    use super::*;
    use pretty_assertions::assert_eq;

    const T: &str = "thread-1";

    fn started_script() -> Vec<String> {
        vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_string()]
    }

    fn settings(mode: &str) -> String {
        let message = json!({"method": "thread/settings/updated", "params": {
            "threadId": T,
            "threadSettings": {
                "cwd": "/tmp/project", "approvalPolicy": "on-request", "approvalsReviewer": "user",
                "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
                "model": "gpt-5.1-codex", "modelProvider": "openai", "serviceTier": null,
                "effort": null, "summary": null,
                "collaborationMode": {"mode": mode, "settings": {
                    "model": "gpt-5.1-codex", "reasoning_effort": null, "developer_instructions": null}},
                "personality": null
            }
        }});
        format!(r#"{{"step":"notify","message":{message}}}"#)
    }

    fn tokens() -> String {
        r#"{"step":"notify","message":{"method":"thread/tokenUsage/updated","params":{"threadId":"thread-1","turnId":"turn-1","tokenUsage":{"total":{"totalTokens":10,"inputTokens":10,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0},"last":{"totalTokens":10,"inputTokens":10,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0},"modelContextWindow":null}}}}"#.to_string()
    }

    fn footer(sandbox: &Sandbox) -> chat::Footer {
        let app = sandbox.app();
        facts::footer(&app.screens().chat, app.active_thread().screen_slots())
    }

    #[test]
    fn the_mode_comes_from_the_settings_of_the_thread() {
        let mut lines = started_script();
        let fresh = footer(&play(&lines));
        assert_eq!(fresh.mode_label, Some("Default"));
        assert!(!fresh.plan_indicator);

        lines.push(settings("plan"));
        let plan = footer(&play(&lines));
        assert_eq!(plan.mode_label, Some("Plan"));
        assert!(plan.plan_indicator);

        lines.push(settings("default"));
        let back = footer(&play(&lines));
        assert_eq!(back.mode_label, Some("Default"));
        assert!(!back.plan_indicator);
    }

    #[test]
    fn modes_turned_off_show_no_label_and_no_indicator() {
        let mut lines = started_script();
        lines.push(settings("plan"));
        lines.push(screen(
            json!({"flags": {"collaboration_modes_enabled": false}}),
        ));
        let off = footer(&play(&lines));
        assert_eq!(off.mode_label, None);
        assert!(!off.plan_indicator);
    }

    #[test]
    fn the_side_agent_and_navigation_facts_are_the_ones_written() {
        let mut lines = started_script();
        let plain = footer(&play(&lines));
        assert!(!plain.side_active);
        assert_eq!(plain.side_label, None);
        assert_eq!(plain.agent_label, None);
        assert!(plain.agents_navigation);

        lines.push(screen(json!({"flags": {"side_conversation_active": true}})));
        lines.push(screen(
            json!({"facts": {"side_label": "Side from main thread · ctrl-/ to switch"}}),
        ));
        lines.push(screen(
            json!({"facts": {"agent_label": "Robie [explorer]"}}),
        ));
        lines.push(screen(json!({"facts": {"agents_navigation": false}})));
        let side = footer(&play(&lines));
        assert!(side.side_active);
        assert_eq!(
            side.side_label.as_deref(),
            Some("Side from main thread · ctrl-/ to switch")
        );
        assert_eq!(side.agent_label.as_deref(), Some("Robie [explorer]"));
        assert!(!side.agents_navigation);
    }

    fn resume(app: &mut CodexApp, resumed: bool) {
        apply_thread(
            app,
            &ThreadId::new(T),
            ThreadEv::Screen(ScreenThreadEv::Chat(ChatThreadEv::Metadata {
                name: None,
                forked_from: None,
                cwd: "/tmp".into(),
                path: None,
                parent_owned: false,
                ephemeral: false,
                model: None,
                effort: None,
                provider: "openai".into(),
                daybreak: None,
                resumed,
                fresh: !resumed,
            })),
            &mut AppFx::default(),
        );
    }

    fn pending_in(app: &CodexApp) -> bool {
        facts::usage_pending(app.active_thread().screen_slots())
    }

    #[test]
    fn a_resumed_thread_hides_the_context_until_its_usage_comes() {
        let mut played = build(&started_script().join("\n")).expect("plays");
        assert!(!pending_in(&played.app), "a fresh thread is not pending");

        resume(&mut played.app, true);
        assert!(pending_in(&played.app));
        // a later `thread/started` without turns does not make it fresh
        resume(&mut played.app, false);
        assert!(pending_in(&played.app));

        let mut lines = started_script();
        lines.push(tokens());
        let mut counted = build(&lines.join("\n")).expect("plays");
        resume(&mut counted.app, true);
        assert!(
            !pending_in(&counted.app),
            "usage was counted: nothing is pending"
        );
    }

    #[test]
    fn the_footer_carries_the_pending_usage() {
        let mut played = build(&started_script().join("\n")).expect("plays");
        resume(&mut played.app, true);
        let app = &played.app;
        let got = facts::footer(&app.screens().chat, app.active_thread().screen_slots());
        assert!(got.usage_pending);
    }
}

mod names {
    use super::*;
    use codex_tui_core::screens::select::models::{Catalog, Preset};
    use pretty_assertions::assert_eq;

    fn catalog() -> Catalog {
        let preset: Preset = serde_json::from_value(json!({
            "model": "gpt-5.1-codex", "name": "GPT-5.1 Codex", "efforts": [], "default_effort": "medium",
            "service_tiers": [{"id": "priority", "name": "fast", "description": "Fast inference"}]
        }))
        .expect("preset");
        let mut catalog = Catalog::default();
        catalog.presets.push(preset);
        catalog
    }

    #[test]
    fn the_model_is_named_by_the_catalog_then_by_its_slug() {
        let catalog = catalog();
        assert_eq!(
            facts::model_display_name(&catalog, Some("gpt-5.1-codex")),
            "GPT-5.1 Codex"
        );
        assert_eq!(
            facts::model_display_name(&catalog, Some("my-model")),
            "my-model"
        );
        assert_eq!(
            facts::model_display_name(&catalog, Some("gpt-reserve")),
            "Luna Reserve"
        );
        assert_eq!(facts::model_display_name(&catalog, Some("")), "loading");
        assert_eq!(facts::model_display_name(&catalog, None), "loading");
    }

    #[test]
    fn the_tier_uses_its_catalog_name_and_flex_fallback_needs_a_supported_provider() {
        let catalog = catalog();
        let name =
            |id, fallback| facts::tier_display_name(&catalog, Some("gpt-5.1-codex"), id, fallback);
        assert_eq!(name(Some("priority"), false), Some("fast"));
        assert_eq!(
            facts::tier_display_name(&catalog, Some("unknown"), Some("priority"), true),
            None
        );
        assert_eq!(name(Some("flex"), true), Some("Flex"));
        assert_eq!(name(Some("flex"), false), None);
        assert_eq!(name(Some("other"), true), None);
        assert_eq!(name(None, true), None);
    }
}

mod agent_label {
    use super::*;
    use pretty_assertions::assert_eq;

    fn thread(
        id: &str,
        nickname: Option<&str>,
        role: Option<&str>,
    ) -> codex_app_server_protocol::Thread {
        serde_json::from_value(json!({
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": null, "threadSource": null,
            "agentNickname": nickname, "agentRole": role, "gitInfo": null, "name": null,
            "daybreakEnabled": null, "turns": []
        }))
        .expect("thread")
    }

    // active_agent_label_tracks_current_thread
    #[test]
    fn it_names_the_viewed_agent_and_hides_for_a_single_thread() {
        let main = thread("main", None, None);
        let robie = thread("robie", Some("Robie"), Some("explorer"));
        let both = [main.clone(), robie];
        assert_eq!(
            facts::agent_label(&both, Some("robie"), Some("main")).as_deref(),
            Some("Robie [explorer]")
        );
        assert_eq!(
            facts::agent_label(&both, Some("main"), Some("main")).as_deref(),
            Some("Main [default]")
        );
        assert_eq!(
            facts::agent_label(&[main], Some("main"), Some("main")),
            None
        );
        assert_eq!(facts::agent_label(&both, None, Some("main")), None);
        // metadata missing for the displayed thread: the generic name
        assert_eq!(
            facts::agent_label(&both, Some("ghost"), Some("main")).as_deref(),
            Some("Agent")
        );
    }

    #[test]
    fn a_role_alone_is_bracketed() {
        let both = [
            thread("main", None, None),
            thread("a", None, Some("worker")),
        ];
        assert_eq!(
            facts::agent_label(&both, Some("a"), Some("main")).as_deref(),
            Some("[worker]")
        );
    }
}

#[test]
fn invalid_surface_selection_warnings_wait_for_a_thread_and_are_once_per_app() {
    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::{ScreenEv, agents};
    use codex_tui_core::threads::{Item, LocalWarning};

    let first = ThreadId::new("first");
    let later = ThreadId::new("later");
    let lines = [
        screen(
            json!({"status": {"items": ["model-name", "spinner", "model_name", "spinner"],
                                  "terminal_title": ["project", "spinner", "thread", "project-root"]}}),
        ),
        r#"{"step":"seed","seed":"session","thread":"first"}"#.to_owned(),
        screen(json!({"status": {"items": ["elsewhere"], "terminal_title": ["other-title"]}})),
    ];
    let mut sandbox = play(&lines);
    sandbox.send(AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(
        later.clone(),
    )))));
    let warnings = |id| {
        sandbox
            .app()
            .thread(id)
            .expect("attached thread")
            .transcript()
            .iter()
            .filter_map(|(_, item)| {
                if let Item::LocalWarning(warning) = item {
                    Some(warning.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        warnings(&first),
        [
            LocalWarning::InvalidStatusLineItems(vec!["spinner".into(), "model_name".into()]),
            LocalWarning::InvalidTitleItems(vec!["project-root".into()]),
        ]
    );
    assert!(warnings(&later).is_empty());
}

mod live_git_summary {
    use super::*;
    use codex_tui_core::app::effects::{Ask, Done};
    use codex_tui_core::app::{AppEv, BackendEv, Command, Ingress};
    use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
    use codex_tui_core::ids::{BackendId, ConnEpoch};
    use codex_tui_core::rpc::{Address, Reply};
    use codex_tui_core::screens::{ScreenEv, agents};

    const LOCAL: BackendId = BackendId(0);
    const REMOTE: BackendId = BackendId(1);

    fn event(app: &mut CodexApp, backend: BackendId, ev: AppEv) -> AppFx {
        let epoch = app.epoch_of(backend);
        app.ingest(Ingress::of(backend, epoch, ev))
            .expect("current connector")
    }

    fn open() -> CodexApp {
        let mut app = CodexApp::new();
        app.attach();
        let started = event(&mut app, LOCAL, AppEv::Connected);
        let ticket = started
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("start thread");
        event(
            &mut app,
            LOCAL,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("a"))),
        );
        app
    }

    fn select(app: &mut CodexApp, ids: &[&str]) -> AppFx {
        event(
            app,
            LOCAL,
            AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
                chat::StatusFacts {
                    items: Some(ids.iter().map(|id| (*id).to_owned()).collect()),
                    ..Default::default()
                },
            )))),
        )
    }

    fn workspace(thread: &str, cwd: &str) -> AppEv {
        let message = json!({
            "method": "thread/settings/updated", "params": {
                "threadId": thread, "threadSettings": {
                    "cwd": cwd, "approvalPolicy":"on-request", "approvalsReviewer":"user",
                    "sandboxPolicy":{"type":"readOnly"}, "activePermissionProfile":null,
                    "model":"gpt-5.4", "modelProvider":"openai", "serviceTier":null,
                    "effort":null, "summary":null, "personality":null,
                    "collaborationMode":{"mode":"default","settings":{
                        "model":"gpt-5.4","reasoning_effort":null,"developer_instructions":null
                    }}
                }
            }
        });
        notification(message)
    }

    fn notification(message: serde_json::Value) -> AppEv {
        AppEv::Backend(BackendEv::Notification(Box::new(
            serde_json::from_value(message).expect("server message"),
        )))
    }

    fn git(fx: &AppFx) -> Option<(&Command, u64, &ThreadId, &str, Address)> {
        fx.commands().iter().find_map(|command| match command {
            Command::Effect {
                request,
                ask:
                    Ask::GitSummary {
                        thread,
                        cwd,
                        address,
                    },
                ..
            } => Some((command, *request, thread, cwd.as_str(), *address)),
            _ => None,
        })
    }

    fn result(request: u64, pr: Option<u64>) -> AppEv {
        AppEv::Effect {
            request,
            done: Done::GitSummary(GitSummary {
                pull_request: pr.map(|number| PullRequest {
                    number,
                    url: format!("https://example.com/pr/{number}"),
                }),
                changes: None,
            }),
        }
    }

    fn finish(
        app: &mut CodexApp,
        connector: ConnEpoch,
        address: Address,
        request: u64,
        pr: Option<u64>,
    ) -> Option<AppFx> {
        app.ingest(Ingress::transport_event(
            address.backend,
            connector,
            address.epoch,
            result(request, pr),
        ))
    }

    fn shown(app: &CodexApp) -> Option<u64> {
        app.screens()
            .chat
            .facts
            .git_summary()?
            .pull_request
            .as_ref()
            .map(|pr| pr.number)
    }

    #[test]
    fn only_the_current_workspace_and_transport_can_install_a_summary() {
        let mut app = open();
        let local = app.epoch();
        let remote = app.attach_backend(REMOTE);
        event(&mut app, REMOTE, AppEv::Connected);
        assert!(git(&event(&mut app, LOCAL, workspace("a", "/a"))).is_none());
        let first = select(&mut app, &["branch-changes", "pull-request-number"]);
        let (command, old, thread, cwd, source) =
            git(&first).expect("attached workspace asks only when configured");
        assert_eq!((thread.as_str(), cwd, source.backend), ("a", "/a", LOCAL));
        assert_eq!(
            app.dispatch_route(command, None),
            Some((LOCAL, Some(source)))
        );
        assert!(git(&event(&mut app, LOCAL, workspace("a", "/a"))).is_none());
        assert!(
            app.ingest(Ingress::transport_event(
                REMOTE,
                remote,
                remote,
                result(old, Some(1))
            ))
            .is_none()
        );

        let next = event(&mut app, LOCAL, workspace("a", "/a-next"));
        let (_, second, _, cwd, _) = git(&next).expect("different cwd is a different workspace");
        assert_eq!(cwd, "/a-next");
        assert!(app.dispatch_route(command, None).is_none());
        assert!(finish(&mut app, local, source, old, Some(1)).is_none());
        assert!(finish(&mut app, local, source, second, None).is_some());
        assert!(shown(&app).is_none());
        assert!(
            git(&event(&mut app, LOCAL, workspace("a", "/a-next"))).is_none(),
            "refusal settles that lookup"
        );

        // Observing another workspace never changes the displayed thread's result. When it is
        // shown, the identical path on a different connector still needs its own request.
        let last_local = event(&mut app, LOCAL, workspace("a", "/common"));
        let (_, local_pending, _, _, local_address) = git(&last_local).expect("local lookup");
        event(&mut app, REMOTE, workspace("b", "/common"));
        event(
            &mut app,
            LOCAL,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
                "b",
            ))))),
        );
        let next = event(&mut app, REMOTE, workspace("b", "/common"));
        let (command, stale, thread, _, address) =
            git(&next).expect("remote owner asks on its socket");
        assert_eq!((thread.as_str(), address.backend), ("b", REMOTE));
        assert_eq!(
            app.dispatch_route(command, None),
            Some((REMOTE, Some(address)))
        );
        assert!(
            finish(&mut app, local, local_address, local_pending, Some(1)).is_none(),
            "a result from the abandoned workspace cannot settle the remote lookup"
        );
        let next_stamp = address.epoch.next();
        let after = app
            .ingest(Ingress::transport_replaced(REMOTE, remote, next_stamp))
            .expect("new socket");
        let (_, fresh, _, _, new_address) =
            git(&after).expect("replacement reissues current workspace");
        assert_eq!(new_address.epoch, next_stamp);
        assert!(finish(&mut app, remote, address, stale, Some(2)).is_none());
        assert!(app.dispatch_route(command, None).is_none());
        assert!(finish(&mut app, remote, new_address, fresh, Some(42)).is_some());
        assert_eq!(shown(&app), Some(42));
        assert!(
            finish(&mut app, remote, new_address, fresh, Some(3)).is_none(),
            "duplicate"
        );
        select(&mut app, &["model"]);
        assert!(shown(&app).is_none());
    }

    #[test]
    fn a_terminal_turn_refreshes_once_and_keeps_the_last_summary_in_view() {
        let mut app = open();
        let local = app.epoch();
        event(&mut app, LOCAL, workspace("a", "/work"));
        let started = select(&mut app, &["pull-request-number"]);
        let (_, request, _, _, address) = git(&started).expect("initial workspace request");
        assert!(finish(&mut app, local, address, request, Some(12)).is_some());
        assert!(
            git(&event(
                &mut app,
                LOCAL,
                AppEv::Backend(turn_started("a", "turn-1"))
            ))
            .is_none()
        );
        let fail = |will_retry| {
            notification(json!({
                "method":"error", "params": {"error":{"message":"a fixture failure","codexErrorInfo":null,"additionalDetails":null},"willRetry":will_retry,"threadId":"a","turnId":"turn-1"}
            }))
        };
        assert!(git(&event(&mut app, LOCAL, fail(true))).is_none());
        let terminal = event(&mut app, LOCAL, fail(false));
        let (_, request, _, _, address) = git(&terminal).expect("a real terminal error refreshes");
        assert_eq!(shown(&app), Some(12));
        assert!(
            git(&event(
                &mut app,
                LOCAL,
                AppEv::Backend(turn_completed("a", "turn-1"))
            ))
            .is_none()
        );
        for turn in ["turn-2", "turn-3"] {
            event(&mut app, LOCAL, AppEv::Backend(turn_started("a", turn)));
            let ended = event(&mut app, LOCAL, AppEv::Backend(turn_completed("a", turn)));
            assert!(
                git(&ended).is_none(),
                "do not start a second concurrent lookup"
            );
        }
        let superseded =
            finish(&mut app, local, address, request, Some(28)).expect("pending result");
        let (_, refresh, _, _, address) = git(&superseded).expect("follow-up after both turns");
        assert_eq!(
            shown(&app),
            Some(12),
            "keep the previously displayed result"
        );
        let final_result =
            finish(&mut app, local, address, refresh, Some(42)).expect("fresh result");
        assert!(git(&final_result).is_none());
        assert_eq!(shown(&app), Some(42));
    }
}
