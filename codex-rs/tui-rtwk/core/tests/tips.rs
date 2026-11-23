#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The tips catalog and the pick (chunk-cells-session-info). Expected values are the oracle's
//! (`tooltips.rs`, `tooltips/keybinding_tests.rs`, `assets/tooltips.txt`): a placeholder takes
//! the action's primary shortcut in a padded code span, a tip whose shortcut is unbound or whose
//! placeholder is malformed is left out, and the pick is one of the resolved tips.

use std::collections::{BTreeMap, BTreeSet};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::{Keymap, KeymapSpec};
use codex_tui_core::screens::startup::launch;
use codex_tui_core::screens::tips;
use serde_json::json;

/// `open_presets` set to an empty list: the user unbound it.
fn unbound() -> Keymap {
    let spec = KeymapSpec::from_json(&json!({"chat": {"open_presets": []}}));
    Keymap::resolve(&spec).expect("a valid keymap")
}

fn remapped() -> Keymap {
    let spec = KeymapSpec::from_json(&json!({
        "global": {
            "open_transcript": ["ctrl-x t", "f12"],
            "find_transcript": "ctrl-x f",
            "copy": "ctrl-x `",
        },
        "chat": {"open_presets": "ctrl-x p"},
    }));
    Keymap::resolve(&spec).expect("a valid keymap")
}

const TRANSCRIPT: &str = "Press {key:global.open_transcript} to open the full transcript.";

#[test]
fn a_placeholder_takes_the_primary_shortcut_in_a_code_span() {
    assert_eq!(
        tips::render(TRANSCRIPT, &Keymap::defaults()).as_deref(),
        Some("Press `` ⌃t `` to open the full transcript.")
    );
    // a hint lists the single keys before the chords; a chord reads as its strokes apart
    assert_eq!(
        tips::render(TRANSCRIPT, &remapped()).as_deref(),
        Some("Press `` f12 `` to open the full transcript.")
    );
    assert_eq!(
        tips::render("Search with {key:global.find_transcript}.", &remapped()).as_deref(),
        Some("Search with `` ⌃x f ``.")
    );
    // a backtick key needs the padded span
    assert_eq!(
        tips::render("Copy with {key:global.copy}.", &remapped()).as_deref(),
        Some("Copy with `` ⌃x ` ``.")
    );
}

#[test]
fn a_tip_without_a_placeholder_is_kept_as_written() {
    let tip = "Use **/new** to start fresh.";
    assert_eq!(tips::render(tip, &Keymap::defaults()).as_deref(), Some(tip));
}

#[test]
fn an_unbound_or_malformed_placeholder_leaves_the_tip_out() {
    let keymap = Keymap::defaults();
    // the user unbound `chat.open_presets`; a binding brings the tip back
    let presets = "Use {key:chat.open_presets}, then 0-9.";
    assert_eq!(tips::render(presets, &unbound()), None);
    assert_eq!(
        tips::render(presets, &remapped()).as_deref(),
        Some("Use `` ⌃x p ``, then 0-9.")
    );
    // no such action, no dot, no closing brace
    assert_eq!(tips::render("Press {key:global.nothing}.", &keymap), None);
    assert_eq!(tips::render("Press {key:global}.", &keymap), None);
    assert_eq!(tips::render("Press {key:global.copy", &keymap), None);
}

#[test]
fn the_resolved_pool_follows_the_keymap() {
    let defaults = tips::resolved(&Keymap::defaults());
    let unbound = tips::resolved(&unbound());
    assert!(
        defaults
            .iter()
            .any(|tip| tip.contains("Start a fresh idea with **/new**"))
    );
    // the preset tip is skipped while `open_presets` is unbound
    let presets = |pool: &[String]| pool.iter().any(|tip| tip.contains("then 0–9"));
    assert!(presets(&defaults));
    assert!(!presets(&unbound));
    // no placeholder is left unfilled and comment lines are not tips
    for tip in defaults.iter().chain(&unbound) {
        assert!(!tip.contains("{key:"), "{tip}");
        assert!(!tip.starts_with('#'), "{tip}");
    }
}

#[test]
fn the_pick_is_one_of_the_resolved_tips_and_stable_per_thread() {
    let keymap = Keymap::defaults();
    let mut pool: BTreeSet<String> = tips::resolved(&keymap).into_iter().collect();
    // the desktop-app tip is in the pool of the machines that have one
    pool.extend(tips::templates().map(str::to_owned));
    let mut seen = BTreeSet::new();
    for index in 0..60 {
        let thread = ThreadId::new(format!("thread-{index}"));
        let tip = tips::pick(&thread, &keymap).expect("a tip");
        assert!(pool.contains(&tip), "{tip}");
        assert_eq!(tips::pick(&thread, &keymap), Some(tip.clone()));
        seen.insert(tip);
    }
    assert!(seen.len() > 1, "every conversation drew the same tip");
}

mod opening {
    use codex_tui_core::app::facts::TuiSettings;
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, apply_app};
    use codex_tui_core::ids::{ThreadId, Ticket};
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::cells::{Ev, SessionInfo, Startup};
    use codex_tui_core::threads::{Item, Notice};

    fn no_tip() -> Option<String> {
        panic!("no tip is asked for");
    }

    #[test]
    fn the_first_run_opens_with_the_help_and_no_tip() {
        // oracle session_info_first_event_respects_tooltip_preference
        let info = SessionInfo::opening(true, true, no_tip, "requested", "used");
        assert_eq!(
            info,
            Some(SessionInfo {
                help: true,
                tip: None,
                model_changed: None,
            })
        );
        // tooltips off: the header stands alone, and no model note either
        assert_eq!(
            SessionInfo::opening(true, false, no_tip, "requested", "used"),
            None
        );
    }

    #[test]
    fn a_later_run_opens_with_a_tip_and_the_model_note() {
        let tip = || Some("Use **/new**.".to_owned());
        let info = SessionInfo::opening(false, true, tip, "gpt-5", "gpt-5").expect("a card");
        assert!(!info.help);
        assert_eq!(info.tip.as_deref(), Some("Use **/new**."));
        assert_eq!(info.model_changed, None);

        let info = SessionInfo::opening(false, true, tip, "asked", "used").expect("a card");
        assert_eq!(
            info.model_changed,
            Some(("asked".to_owned(), "used".to_owned()))
        );
        // tooltips off (oracle session_info_hides_tooltips_when_disabled): no tip, the note stays
        let info = SessionInfo::opening(false, false, no_tip, "asked", "used").expect("a card");
        assert_eq!(info.tip, None);
        assert!(info.model_changed.is_some());
        assert_eq!(
            SessionInfo::opening(false, false, no_tip, "same", "same"),
            None
        );
    }

    fn started(facts: Vec<AppEv>) -> CodexApp {
        let mut app = CodexApp::new();
        for ev in facts {
            apply_app(&mut app, ev, &mut AppFx::default());
        }
        let mut fx = AppFx::default();
        apply_app(&mut app, AppEv::Connected, &mut fx);
        let ticket: Ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread command");
        let reply = Reply::thread_started(ThreadId::new("thread-1"));
        apply_app(&mut app, AppEv::Reply(ticket, reply), &mut AppFx::default());
        app
    }

    fn startup(first_run: bool, tooltip: Option<&str>) -> AppEv {
        AppEv::Screen(ScreenEv::Cells(Ev::Startup(Startup {
            first_run,
            tooltip: tooltip.map(str::to_owned),
        })))
    }

    fn cards(app: &CodexApp) -> Vec<SessionInfo> {
        app.active_thread()
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Notice(Notice::SessionInfo(info)) => Some(info.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_conversation_opens_with_a_card_once_the_host_has_said_how_the_run_started() {
        assert_eq!(cards(&started(Vec::new())), Vec::new(), "no fact, no card");

        let help = cards(&started(vec![startup(true, None)]));
        assert_eq!(help.len(), 1);
        assert!(help[0].help && help[0].tip.is_none());

        let tip = cards(&started(vec![startup(false, None)]));
        assert_eq!(tip.len(), 1);
        assert!(!tip[0].help);
        assert!(tip[0].tip.is_some(), "the catalog's pick");
    }

    #[test]
    fn the_availability_notice_takes_the_place_of_the_pick() {
        // oracle session_info_uses_availability_nux_tooltip_override
        let card = cards(&started(vec![startup(
            false,
            Some("Model just became available"),
        )]));
        assert_eq!(card.len(), 1);
        assert_eq!(card[0].tip.as_deref(), Some("Model just became available"));
        // ... and on a first run it is taken without being shown
        let card = cards(&started(vec![startup(
            true,
            Some("Model just became available"),
        )]));
        assert!(card[0].help && card[0].tip.is_none());
    }

    #[test]
    fn tooltips_off_leave_the_notice_and_the_tip_out() {
        // oracle session_info_hides_tooltips_when_disabled
        let off = AppEv::Tui(TuiSettings {
            show_tooltips: false,
            ..TuiSettings::default()
        });
        let app = started(vec![
            off,
            startup(false, Some("Model just became available")),
        ]);
        assert_eq!(cards(&app), Vec::new());
    }

    #[test]
    fn facts_that_come_after_the_conversation_attached_still_give_it_its_card() {
        let mut app = started(Vec::new());
        assert_eq!(cards(&app), Vec::new());
        apply_app(
            &mut app,
            startup(false, Some("Model just became available")),
            &mut AppFx::default(),
        );
        let card = cards(&app);
        assert_eq!(card.len(), 1);
        assert_eq!(card[0].tip.as_deref(), Some("Model just became available"));
        // a second delivery does not add a second card
        apply_app(&mut app, startup(false, None), &mut AppFx::default());
        assert_eq!(cards(&app).len(), 1);
    }
}

#[test]
fn the_first_model_with_a_notice_decides_and_stops_after_four_showings() {
    // oracle `select_model_availability_nux` over `ModelPreset::availability_nux`
    let entry = |name: &str, nux: Option<&str>| {
        json!({
            "id": name, "model": name, "upgrade": null, "upgradeInfo": null,
            "availabilityNux": nux.map(|message| json!({"message": message})),
            "displayName": name, "description": "", "hidden": false,
            "supportedReasoningEfforts": [], "defaultReasoningEffort": "high",
            "isDefault": false
        })
    };
    let models = launch::models(json!({"data": [
        entry("plain", None),
        entry("new-one", Some("Model just became available")),
        entry("newer", Some("Another one")),
    ]}));
    assert_eq!(models.len(), 3);
    let shown = |count: u32| BTreeMap::from([("new-one".to_owned(), count)]);
    assert_eq!(
        tips::availability_nux(&models, &BTreeMap::new()),
        Some(tips::Nux {
            model: "new-one".into(),
            message: "Model just became available".into(),
            shown: 1,
        })
    );
    assert_eq!(
        tips::availability_nux(&models, &shown(3)).map(|nux| nux.shown),
        Some(4)
    );
    // the count ran out: no notice, and the next model's is not looked at
    assert_eq!(tips::availability_nux(&models, &shown(4)), None);
    assert_eq!(tips::availability_nux(&models[..1], &BTreeMap::new()), None);
}
