//! The chat is the hub: every screen-opening command and key opens its screen
//! from the chat, and Escape returns with the transcript and the composer's
//! text intact. One test per command and per key action; the commands and
//! chords are the oracle's (`slash_command.rs`, `keymap.rs:1908-1953`), the
//! screen each opens is the trigger table of `parity/chat-nav.md`.

use codex_tui_rtwk::pages::session::open;

use crate::chat::{assert_chat_back, chat_with_history, shows};
use crate::common::{Harness, ms, start};
use codex_tui_core::commands::Target;

fn chat() -> Harness {
    open::reset_last_opened();
    let mut h = start();
    chat_with_history(&mut h);
    h
}

/// Waits until the chat has opened `dir`: the ask goes through the shell and
/// comes back as an event, so the screen is up when the app applied it, not
/// after a fixed span of frames. Then the frames of the fade-in.
fn wait_shows(h: &mut Harness, what: &str, dir: &str) {
    h.wait_until(&format!("{what} to open {dir}"), |h| shows(h, dir));
    if dir == "import" {
        // The route is up before its flow starts; Escape needs the flow.
        h.wait_until("the import flow started", |h| {
            h.import_phase() != codex_tui_core::screens::import::Phase::Idle
        });
    }
    h.t.pump_for(ms(800));
}

/// Escape closes what the chat opened: the page or sheet answers through the
/// shell too (the import page cancels its flow there), so the chat is back
/// when the app applied that, not after a fixed span of frames. Waits for the
/// screen the chat showed before, then the fade-out.
fn escape_closes(h: &mut Harness, before: &str) {
    h.press("escape");
    h.wait_until("the chat back as it was", |h| h.t.screen() == before);
    h.t.pump_for(ms(400));
}

/// Types `line` and sends it; the caller waits for what it opens.
fn run_command(h: &mut Harness, line: &str) {
    h.t.type_text(line);
    h.press("enter");
}

macro_rules! command_opens {
    ($($test:ident: $line:expr => $dir:expr;)*) => {$(
        #[test]
        fn $test() {
            let mut h = chat();
            let before = h.t.screen();
            run_command(&mut h, $line);
            wait_shows(&mut h, $line, $dir);
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $line, $dir, h.t.screen());
            escape_closes(&mut h, &before);
            assert_chat_back(&h, None, &before);
        }
    )*};
}

command_opens! {
    slash_model_opens_the_selection: "/model" => "select";
    slash_permissions_opens_the_selection: "/permissions" => "select";
    slash_review_opens_the_selection: "/review" => "select";
    slash_archive_opens_the_confirmation: "/archive" => "select";
    slash_delete_opens_the_confirmation: "/delete" => "select";
    slash_skills_opens_the_settings: "/skills" => "settings";
    slash_theme_opens_the_settings: "/theme" => "settings";
    slash_title_opens_the_settings: "/title" => "settings";
    slash_statusline_opens_the_settings: "/statusline" => "settings";
    slash_experimental_opens_the_settings: "/experimental" => "settings";
    slash_memories_opens_the_settings: "/memories" => "settings";
    slash_agents_opens_the_agents: "/agents" => "agents";
    slash_resume_opens_the_picker: "/resume" => "resume";
    slash_import_opens_the_import: "/import" => "import";
    slash_usage_daily_opens_the_usage_dashboard: "/usage daily" => "usage";
    slash_usage_weekly_opens_the_usage_dashboard: "/usage weekly" => "usage";
    slash_warnings_opens_the_dialogs: "/warnings" => "dialogs";
    slash_feedback_opens_the_flow: "/feedback" => "feedback";
    slash_pets_opens_the_pets: "/pets" => "pets";
    slash_pet_alias_opens_the_pets: "/pet" => "pets";
    slash_test_approval_opens_the_approvals: "/test-approval" => "approvals";
}

/// The dialogs the host offers the catalogue of (`/config`, `/keymap`: the facts of the config
/// the app was started with) open once it has sent it.
fn host_offers_then_opens(offer: serde_json::Value, line: &str) {
    let mut h = chat();
    h.connect();
    let fact = codex_tui_core::features::seed_fact("settings", offer)
        .unwrap_or_else(|err| panic!("the offer: {err:?}"));
    h.inject([codex_tui_core::app::AppEv::Feature(fact)]);
    let before = h.t.screen();
    run_command(&mut h, line);
    wait_shows(&mut h, line, "settings");
    assert!(
        shows(&h, "settings"),
        "{line} did not open the settings:\n{}",
        h.t.screen()
    );
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

#[test]
fn slash_config_opens_the_settings() {
    host_offers_then_opens(
        serde_json::json!({"kind": "config_panel", "rows": []}),
        "/config",
    );
}

#[test]
fn slash_keymap_opens_the_settings() {
    host_offers_then_opens(serde_json::json!({"kind": "keymap_default"}), "/keymap");
}

/// A bare `/usage` is the feature's own sheet (`features/usage_menu`): the usage menu over the chat,
/// Esc gives it up. The analytics page is its first row (`/usage weekly` opens the page directly).
#[test]
fn slash_usage_shows_the_usage_menu() {
    let mut h = chat();
    let before = h.t.screen();
    run_command(&mut h, "/usage");
    h.wait_for("the usage menu", |s| s.contains("Redeem reset"));
    h.t.pump_for(ms(800));
    let screen = h.t.screen();
    assert!(screen.contains("View analytics"), "{screen}");
    escape_closes(&mut h, &before);
}

/// `/rename` is the feature's own prompt (`features/rename`): the feature wishes it, the presenter
/// shows it over the chat, Esc gives it up.
#[test]
fn slash_rename_shows_the_features_name_prompt() {
    let mut h = chat();
    let before = h.t.screen();
    run_command(&mut h, "/rename");
    h.wait_for("the name prompt", |s| s.contains("Name thread"));
    h.t.pump_for(ms(800));
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

/// Item 8: `/subagents` is the picker of this session's subagents (oracle:
/// `slash_dispatch.rs:412`, `AppEvent::OpenAgentPicker`), not the command
/// center: a sheet titled `Subagents` with the main thread's row.
#[test]
fn slash_subagents_opens_the_agent_picker_not_the_command_center() {
    let mut h = chat();
    run_command(&mut h, "/subagents");
    h.wait_for("the sheet", |s| s.contains("Main [default]"));
    h.t.pump_for(ms(800));
    assert_eq!(open::last_opened(), Some(Target::AgentPicker));
    assert!(h.t.screen().contains("Subagents"), "{}", h.t.screen());
}

/// A saved descendant is discoverable from chat without opening Agent Center first.
#[test]
fn slash_subagents_reads_the_active_roots_descendants_across_sparse_and_repeated_cursors() {
    use serde_json::json;

    let mut h = chat();
    disable_collab(&mut h);
    let root =
        h.t.read_app(|app: &codex_tui_rtwk::app::UiApp| app.core.session().active.clone());
    let Some(root) = root else {
        panic!("the chat has an active thread");
    };
    let id = "00000000-0000-0000-0000-000000000021";
    let request_root = root.to_string();
    h.fake.respond(move |request| {
        let params = &request["params"];
        if request["method"] != "thread/list"
            || params["ancestorThreadId"] != request_root
            || params["sourceKinds"] != json!(["subAgentThreadSpawn"])
            || params["useStateDbOnly"] != true
        {
            return None;
        }
        let (threads, next) = if params["archived"] == true {
            (Vec::new(), None)
        } else if params["cursor"].is_null() {
            (Vec::new(), Some("after-empty"))
        } else {
            (
                vec![subagent_thread(id, &request_root, "Saved worker")],
                Some("after-empty"),
            )
        };
        Some(Ok(json!({"data": threads, "nextCursor": next})))
    });
    run_command(&mut h, "/subagents");
    h.wait_for("the descendant read from the backend", |screen| {
        screen.contains("Saved worker [worker]")
    });
    assert!(!h.t.screen().contains("Enable subagents?"));
    let actual: Vec<_> = h
        .fake
        .requests()
        .into_iter()
        .filter(|request| {
            request["method"] == "thread/list"
                && request["params"]["ancestorThreadId"] == root.as_str()
        })
        .map(|request| {
            let params = &request["params"];
            (params["archived"].clone(), params["cursor"].clone())
        })
        .collect();
    assert_eq!(
        actual,
        [
            (json!(false), json!(null)),
            (json!(false), json!("after-empty")),
            (json!(true), json!(null))
        ]
    );
}

/// Sparse pages may advance, but distinct empty pages cannot hold the picker forever.
#[test]
fn slash_subagents_bounds_empty_pages_and_reaches_the_collab_prompt() {
    use serde_json::json;

    let mut h = self::chat();
    disable_collab(&mut h);
    h.fake.respond(|request| {
        let params = &request["params"];
        if request["method"] != "thread/list" || params["ancestorThreadId"].is_null() {
            return None;
        }
        let next = params["cursor"]
            .as_str()
            .and_then(|cursor| cursor.parse::<u64>().ok())
            .unwrap_or_default()
            + 1;
        Some(Ok(json!({"data": [], "nextCursor": next.to_string()})))
    });
    run_command(&mut h, "/subagents");
    h.wait_until("the last bounded archived request", |h| {
        h.fake.requests().iter().any(|request| {
            request["method"] == "thread/list"
                && request["params"]["archived"] == true
                && request["params"]["cursor"] == "99"
        })
    });
    h.wait_for(
        "the offer to enable subagents after both bounded reads",
        |screen| screen.contains("Enable subagents?"),
    );
    let requests = h.fake.requests();
    for archived in [false, true] {
        let cursors = requests
            .iter()
            .filter(|request| {
                request["method"] == "thread/list"
                    && !request["params"]["ancestorThreadId"].is_null()
                    && request["params"]["archived"] == archived
            })
            .map(|request| request["params"]["cursor"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(cursors.len(), 100);
        assert_eq!(cursors.first(), Some(&None));
        assert_eq!(cursors.last(), Some(&Some("99")));
    }
}

fn disable_collab(h: &mut Harness) {
    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::{ScreenEv, chat};
    h.inject([AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
        chat::StatusFacts {
            collab_enabled: false,
            ..chat::StatusFacts::default()
        },
    ))))]);
    assert!(!h.t.read_app(|app: &codex_tui_rtwk::app::UiApp| {
        app.core.screens().chat.status.collab_enabled
    }));
}

fn subagent_thread(id: &str, root: &str, name: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "sessionId": root, "parentThreadId": root,
        "name": name, "agentNickname": name, "agentRole": "worker",
        "preview": "", "ephemeral": false, "historyMode": "legacy", "modelProvider": "openai",
        "createdAt": 1700000000, "updatedAt": 1700000000,
        "status": {"type": "idle"}, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "source": "cli", "canAcceptDirectInput": false, "turns": []
    })
}

#[test]
fn slash_subagents_places_the_path_backed_status_in_the_live_transcript() {
    use codex_tui_core::app::AppEv;
    use codex_tui_core::fixtures::builders::{agent_completed, turn_failed, turn_started};
    use codex_tui_core::ids::{BackendId, ThreadId};
    use codex_tui_core::screens::{ScreenEv, agents};
    use serde_json::json;

    let mut h = chat();
    let root = "11111111-1111-4111-8111-111111111111";
    let id = "33333333-3333-4333-8333-333333333333";
    let mut primary = subagent_thread(root, root, "Primary");
    primary["parentThreadId"] = json!(null);
    let Ok(primary) = serde_json::from_value(primary) else {
        panic!("valid primary thread fixture");
    };
    h.inject([
        AppEv::Screen(ScreenEv::Agents(agents::Ev::ListedBy(
            BackendId::default(),
            agents::Page {
                threads: vec![primary],
                unread: Vec::new(),
                next_cursor: None,
                append: true,
                pins: None,
            },
        ))),
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
            root,
        ))))),
    ]);
    h.fake.respond(move |request| {
        let params = &request["params"];
        if request["method"] != "thread/list" || params["ancestorThreadId"] != root {
            return None;
        }
        let mut child = subagent_thread(id, root, "Live worker");
        child["status"] = json!({"type": "active", "activeFlags": []});
        child["source"] = json!({"subAgent": {"thread_spawn": {
            "parent_thread_id": root, "agent_path": "/root/live_worker", "depth": 1
        }}});
        let data = if params["archived"] == true {
            Vec::new()
        } else {
            vec![child]
        };
        Some(Ok(json!({"data": data, "nextCursor": null})))
    });
    run_command(&mut h, "/subagents");
    h.wait_for("the freshly discovered path-backed agent", |screen| {
        screen.contains("Live worker [worker]")
    });
    h.press("escape");
    h.wait_for("the status cell in the active transcript", |screen| {
        screen.contains("Sub-agents running")
            && screen.contains("/root/live_worker")
            && screen.contains("No recent activity yet.")
    });

    let earlier = format!("discarded preview prefix {}", "middle ".repeat(29));
    h.inject([
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
            id,
        ))))),
        AppEv::Backend(turn_started(id, "child-turn")),
        AppEv::Backend(agent_completed(id, "child-turn", "older-item", &earlier)),
        AppEv::Backend(agent_completed(
            id,
            "child-turn",
            "last-item",
            "latest child activity",
        )),
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
            root,
        ))))),
    ]);
    run_command(&mut h, "/subagents");
    h.wait_for("the picker for the active child", |screen| {
        screen.contains("Live worker [worker]")
    });
    h.press("escape");
    h.wait_for("the running child's most recent activity", |screen| {
        screen.contains("latest child activity")
    });
    let screen = h.t.screen();
    assert!(!screen.contains("discarded preview prefix"), "{screen}");
    assert_eq!(
        screen
            .lines()
            .filter(|line| line.contains("middle"))
            .count(),
        2,
        "{screen}"
    );

    h.inject([AppEv::Backend(turn_failed(id, "child-turn"))]);
    run_command(&mut h, "/subagents");
    h.wait_for("the picker after the child's turn failed", |screen| {
        screen.contains("Live worker [worker]")
    });
    h.press("escape");
    h.wait_for(
        "live terminal evidence overriding the backend's stale Active row",
        |screen| screen.contains("No sub-agents running."),
    );
}

/// The command typed in the popup and picked with Enter, not typed in full.
#[test]
fn a_command_picked_in_the_popup_opens_its_screen() {
    let mut h = chat();
    let before = h.t.screen();
    h.t.type_text("/usag");
    h.t.pump_for(ms(100));
    h.press("enter");
    // a bare `/usage` is the usage menu over the chat (`features/usage_menu`)
    h.wait_for("the usage menu", |s| s.contains("Redeem reset"));
    h.t.pump_for(ms(800));
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

macro_rules! chord_opens {
    ($($test:ident: $keys:expr => $dir:expr;)*) => {$(
        #[test]
        fn $test() {
            let mut h = chat();
            h.t.type_text("a draft");
            h.t.pump_for(ms(100));
            let before = h.t.screen();
            h.press($keys);
            wait_shows(&mut h, $keys, $dir);
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $keys, $dir, h.t.screen());
            escape_closes(&mut h, &before);
            assert_chat_back(&h, Some("a draft"), &before);
        }
    )*};
}

chord_opens! {
    f2_opens_the_warnings: "f2" => "dialogs";
}

/// Escape on an empty composer, twice, is no page any more (`app_backtrack.rs`): the
/// detailed transcript opens in place of the chat's (`tests/pages/chat_live.rs`).
#[test]
fn esc_esc_opens_no_page() {
    let mut h = chat();
    h.press("escape");
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_ne!(
        open::last_opened(),
        Some(Target::Backtrack),
        "{}",
        h.t.screen()
    );
    assert!(!shows(&h, "backtrack"), "{}", h.t.screen());
}

/// Escape with text in the composer does not start backtrack.
#[test]
fn esc_with_a_draft_does_not_start_backtrack() {
    let mut h = chat();
    h.t.type_text("a draft");
    h.press("escape");
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_ne!(
        open::last_opened(),
        Some(Target::Backtrack),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("a draft"), "{}", h.t.screen());
}

#[test]
fn f8_toggles_voice_which_opens_the_voice_strip() {
    let mut h = chat();
    let before = h.t.screen();
    h.press("f8");
    wait_shows(&mut h, "f8", "voice");
    assert!(shows(&h, "voice"), "{}", h.t.screen());
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

#[test]
fn quit_and_exit_leave_the_app() {
    for line in ["/quit", "/exit"] {
        let mut h = crate::common::start_quittable(false);
        chat_with_history(&mut h);
        h.t.type_text(line);
        h.press("enter");
        h.wait_until("the app to end", |h| !h.t.is_running());
        assert!(!h.t.is_running(), "{line} did not quit");
    }
}
