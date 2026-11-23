use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::settings::Edit;
use codex_tui_core::screens::settings::Experimental;
use codex_tui_core::screens::settings::Memories;
use codex_tui_core::screens::settings::Setup;
use codex_tui_core::screens::settings::SetupKind;
use codex_tui_core::screens::settings::USE_THEME_COLORS;
use codex_tui_core::screens::settings::hooks::HookEvent;
use codex_tui_core::screens::settings::hooks::HookTrust;
use codex_tui_core::screens::settings::hooks::Hooks;
use codex_tui_core::screens::settings::{Offer, Which};
use codex_tui_rtwk::sheets::settings::draft::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

const STATUS_LINE: &str =
    include_str!("../../../../src/sheets/settings/fixtures/status_line.jsonl");
const TERMINAL_TITLE: &str = include_str!("../../../../src/sheets/settings/fixtures/title.jsonl");
const SKILLS: &str =
    include_str!("../../../../src/sheets/settings/fixtures/skills_long_names.jsonl");
const MEMORIES: &str = include_str!("../../../../src/sheets/settings/fixtures/memories.jsonl");
const FEATURES: &str =
    include_str!("../../../../src/sheets/settings/fixtures/experimental_discovery.jsonl");
const HOOKS_EVENTS: &str =
    include_str!("../../../../src/sheets/settings/fixtures/hooks_events.jsonl");
const HOOKS_REVIEW: &str =
    include_str!("../../../../src/sheets/settings/fixtures/hooks_events_review_mixed.jsonl");

fn offered(script: &str, which: Which) -> Offer {
    let sandbox = Sandbox::play(script).expect("plays");
    sandbox
        .app()
        .screens()
        .settings
        .offers
        .get(&which)
        .cloned()
        .expect("the dialog is offered")
}

fn setup(script: &str, which: Which) -> Setup {
    match offered(script, which) {
        Offer::Setup(setup) => setup,
        other => panic!("setup expected, got {other:?}"),
    }
}

fn features() -> Experimental {
    match offered(FEATURES, Which::Experimental) {
        Offer::Experimental(dialog) => dialog,
        other => panic!("features expected, got {other:?}"),
    }
}

fn hooks(script: &str) -> Hooks {
    match offered(script, Which::Hooks) {
        Offer::Hooks(hooks) => hooks,
        other => panic!("hooks expected, got {other:?}"),
    }
}

fn order(setup: &Setup) -> Vec<&str> {
    setup.items.iter().map(|item| item.id.as_str()).collect()
}

#[test]
fn a_draft_nobody_touched_reads_as_what_is_offered() {
    let status = setup(STATUS_LINE, Which::StatusLine);
    assert_eq!(SetupDraft::default().view(&status), status);
    let dialog = features();
    assert_eq!(FeaturesDraft::new(&dialog).view(&dialog), dialog);
    let hooks = hooks(HOOKS_EVENTS);
    assert_eq!(HooksDraft::default().view(&hooks).browser, hooks);
}

#[test]
fn toggling_an_item_changes_the_preview() {
    let status = setup(STATUS_LINE, Which::StatusLine);
    let mut draft = SetupDraft::default();
    assert_eq!(draft.toggle(&status, "model-with-reasoning"), None);
    assert_eq!(
        draft.view(&status).preview(),
        "gpt-5-codex · ~/codex-rs · jif/statusline-preview · gpt-5.2-codex medium"
    );
    assert_eq!(draft.toggle(&status, "unknown"), None);
}

#[test]
fn moving_reorders_the_orderable_items_and_stops_at_the_fixed_row() {
    let status = setup(STATUS_LINE, Which::StatusLine);
    let mut draft = SetupDraft::default();
    // The fixed "use theme colors" row neither moves nor is crossed.
    draft.shift(&status, USE_THEME_COLORS, 1);
    assert_eq!(order(&draft.view(&status))[0], USE_THEME_COLORS);
    draft.shift(&status, "model", -1);
    assert_eq!(order(&draft.view(&status))[1], "model");
    draft.shift(&status, "model", 1);
    assert_eq!(
        &order(&draft.view(&status))[1..4],
        ["current-dir", "model", "git-branch"]
    );
    assert_eq!(
        draft.view(&status).preview(),
        "~/codex-rs · gpt-5-codex · jif/statusline-preview"
    );
}

#[test]
fn enter_saves_the_enabled_items_in_order_and_the_color_preference() {
    for use_theme_colors in [false, true] {
        let script = STATUS_LINE.replace(
            "\"use_theme_colors\":true",
            &format!("\"use_theme_colors\":{use_theme_colors}"),
        );
        let status = setup(&script, Which::StatusLine);
        assert_eq!(
            SetupDraft::default().save(&status),
            Edit::SetupSaved {
                kind: SetupKind::StatusLine,
                items: vec!["model".into(), "current-dir".into(), "git-branch".into()],
                use_theme_colors,
            }
        );
    }
    let status = setup(STATUS_LINE, Which::StatusLine);
    let mut draft = SetupDraft::default();
    draft.toggle(&status, "model");
    draft.shift(&status, "git-branch", -1);
    assert_eq!(
        draft.save(&status),
        Edit::SetupSaved {
            kind: SetupKind::StatusLine,
            items: vec!["git-branch".into(), "current-dir".into()],
            use_theme_colors: true,
        }
    );
}

#[test]
fn escape_cancels_the_setup_and_reads_the_skills_again() {
    for (script, which, kind) in [
        (STATUS_LINE, Which::StatusLine, SetupKind::StatusLine),
        (TERMINAL_TITLE, Which::Title, SetupKind::Title),
    ] {
        assert_eq!(
            SetupDraft::cancel(&setup(script, which)),
            Edit::SetupCancelled(kind)
        );
    }
    assert_eq!(
        SetupDraft::cancel(&setup(SKILLS, Which::Skills)),
        Edit::SkillsClosed
    );
}

#[test]
fn a_title_without_the_activity_joins_with_bars_and_without_the_prefix() {
    let title = setup(TERMINAL_TITLE, Which::Title);
    let mut draft = SetupDraft::default();
    draft.toggle(&title, "activity");
    assert_eq!(
        draft.view(&title).preview(),
        "my-project | Working | thread title"
    );
    for id in ["run-state", "thread-title", "project-name"] {
        draft.toggle(&title, id);
    }
    assert_eq!(draft.view(&title).preview(), "");
}

/// `skills_toggle_view.rs`: `toggle_selected` asks for each flip to be saved
/// (`AppEvent::SetSkillEnabled`), the skills keep their order.
#[test]
fn each_skill_flip_is_a_request_and_the_skills_keep_their_order() {
    let skills = setup(SKILLS, Which::Skills);
    let id = "polish:superpowers-verification-before-completion";
    let mut draft = SetupDraft::default();
    assert_eq!(
        draft.toggle(&skills, id),
        Some(Edit::SkillToggled(id.to_owned(), true))
    );
    draft.shift(&skills, id, -1);
    let view = draft.view(&skills);
    assert!(view.items[1].enabled);
    assert_eq!(view.items[1].id, id);
    assert_eq!(
        draft.toggle(&skills, id),
        Some(Edit::SkillToggled(id.to_owned(), false))
    );
}

/// Space flips a switch (`toggle_selected`); Enter on one saves both switches as they
/// stand. The confirmation takes neither toggles nor a save.
#[test]
fn memories_toggles_flip_and_enter_saves_both_switches() {
    let Offer::Memories(dialog) = offered(MEMORIES, Which::Memories) else {
        panic!("memories offered");
    };
    let mut draft = MemoriesDraft::new(&dialog, false);
    assert_eq!(
        draft.view(),
        Memories {
            use_memories: true,
            generate_memories: false,
        }
    );
    assert_eq!(draft.reset(), None);
    draft.toggle(2);
    draft.toggle(1);
    draft.toggle(0);
    assert_eq!(
        draft.save(),
        Some(Edit::MemoriesSaved {
            use_memories: false,
            generate_memories: true,
        })
    );

    let mut asking = MemoriesDraft::new(&dialog, true);
    asking.toggle(0);
    assert_eq!(asking.view(), dialog);
    assert_eq!(asking.save(), None);
    assert_eq!(asking.reset(), Some(Edit::MemoriesReset));
}

/// Enter sends what differs from the config and closes only when nothing does; Esc saves
/// what changed (`save_changes`, `on_ctrl_c`).
#[test]
fn the_features_save_what_the_user_flipped() {
    let dialog = features();
    let idle = FeaturesDraft::new(&dialog);
    assert_eq!(idle.save(&dialog), Saving::Close);
    assert_eq!(idle.close(&dialog), None);

    let mut draft = FeaturesDraft::new(&dialog);
    for key in ["future_feature", "network_proxy", "prevent_idle_sleep"] {
        draft.toggle(&dialog, key);
    }
    let updates = vec![
        ("network_proxy".to_string(), false),
        ("future_feature".to_string(), false),
        ("prevent_idle_sleep".to_string(), false),
    ];
    assert_eq!(
        draft.save(&dialog),
        Saving::Write(Edit::FeaturesSave(updates.clone()))
    );
    assert_eq!(
        draft.close(&dialog),
        Some(Edit::FeaturesSave(updates.clone()))
    );
    let shown: Vec<(String, bool)> = draft
        .view(&dialog)
        .features
        .iter()
        .map(|feature| (feature.key.clone(), feature.enabled))
        .collect();
    assert_eq!(shown, updates);

    // a toggle while the write is on its way changes nothing, and Enter closes
    let mut saving = dialog.clone();
    saving.saving = true;
    draft.toggle(&saving, "network_proxy");
    assert!(!draft.view(&saving).features[0].enabled);
    assert_eq!(draft.save(&saving), Saving::Close);
    assert_eq!(draft.close(&saving), None);
}

#[test]
fn a_failed_write_keeps_the_choices_and_enter_sends_them_again_while_esc_closes() {
    let mut dialog = features();
    let mut draft = FeaturesDraft::new(&dialog);
    draft.toggle(&dialog, "network_proxy");
    // the write was sent and refused: the keys stay unconfirmed
    dialog.unconfirmed = vec!["network_proxy".into()];
    assert!(!draft.view(&dialog).features[0].enabled);
    assert_eq!(draft.close(&dialog), None, "Esc only closes");
    // putting the value back still corrects a write that may have landed
    draft.toggle(&dialog, "network_proxy");
    assert_eq!(
        draft.save(&dialog),
        Saving::Write(Edit::FeaturesSave(vec![("network_proxy".into(), true)]))
    );
    // a dialog that comes with unconfirmed writes shows their values
    let reopened = FeaturesDraft::new(&dialog);
    assert!(!reopened.view(&dialog).features[0].enabled);
}

#[test]
fn a_confirmed_write_clears_the_flips() {
    let mut dialog = features();
    let mut draft = FeaturesDraft::new(&dialog);
    draft.toggle(&dialog, "network_proxy");
    assert!(!draft.sync(&dialog));
    dialog.confirmed += 1;
    dialog.features[0].enabled = false;
    assert!(draft.sync(&dialog));
    assert_eq!(draft.view(&dialog), dialog);
    assert!(!draft.sync(&dialog));
}

#[test]
fn a_read_only_feature_cannot_be_flipped() {
    let mut dialog = features();
    dialog.features[0].writable = false;
    let mut draft = FeaturesDraft::new(&dialog);
    draft.toggle(&dialog, "network_proxy");
    assert!(draft.view(&dialog).features[0].enabled);
    assert_eq!(draft.save(&dialog), Saving::Close);
}

/// A switch is a request (`SetHookEnabled`); managed hooks and those waiting for a review
/// stay as they are (`toggle_selected_hook`), as an unknown key.
#[test]
fn hooks_toggle_flips_a_user_hook_and_leaves_the_others() {
    let offered = hooks(HOOKS_EVENTS);
    let enabled = |draft: &HooksDraft, key: &str| {
        draft
            .view(&offered)
            .hooks
            .iter()
            .find(|hook| hook.key == key)
            .map(|hook| hook.enabled)
    };
    let mut draft = HooksDraft::default();
    assert_eq!(
        draft.toggle(&offered, "path:user-config"),
        Some(Edit::HookEnabled {
            key: "path:user-config".into(),
            enabled: true
        })
    );
    assert_eq!(enabled(&draft, "path:user-config"), Some(true));
    assert_eq!(draft.toggle(&offered, "path:managed"), None);
    assert_eq!(draft.toggle(&offered, "unknown"), None);
    assert_eq!(enabled(&draft, "path:managed"), Some(true));
    draft.toggle(&offered, "path:user-config");
    assert_eq!(enabled(&draft, "path:user-config"), Some(false));

    let review = hooks(HOOKS_REVIEW);
    let mut draft = HooksDraft::default();
    assert_eq!(draft.toggle(&review, "path:untrusted"), None);
    assert_eq!(draft.toggle(&review, "path:modified"), None);
}

/// `t` trusts the hook waiting for a review and leaves its enablement; a trusted one
/// records nothing (`trust_selected_hook`). On the events page it trusts them all in one
/// request (`trust_all_hooks`).
#[test]
fn hooks_trust_vouches_for_one_hook_or_all_of_them() {
    let review = hooks(HOOKS_REVIEW);
    let mut draft = HooksDraft::default();
    assert_eq!(draft.trust(&review, "path:trusted"), None);
    assert_eq!(
        draft.trust(&review, "path:untrusted"),
        Some(Edit::HookTrusted {
            key: "path:untrusted".into(),
            current_hash: "sha256:current".into()
        })
    );
    let view = draft.view(&review);
    let untrusted = view
        .hooks
        .iter()
        .find(|hook| hook.key == "path:untrusted")
        .expect("the hook");
    assert_eq!(
        (untrusted.trust, untrusted.enabled),
        (HookTrust::Trusted, false)
    );
    assert_eq!(view.review_total(), 1);
    assert_eq!(
        draft.trust_all(&review),
        Some(Edit::HooksTrusted(vec![(
            "path:modified".into(),
            "sha256:current".into()
        )]))
    );
    assert_eq!(draft.view(&review).review_total(), 0);
    assert_eq!(draft.trust_all(&review), None);
}

/// The page is the draft's: none the events, an event its hooks.
#[test]
fn the_page_open_is_part_of_the_view() {
    let offered = hooks(HOOKS_EVENTS);
    let mut draft = HooksDraft::default();
    assert_eq!(draft.view(&offered).page, None);
    draft.page = Some(HookEvent::PreToolUse);
    assert_eq!(draft.view(&offered).page, Some(HookEvent::PreToolUse));
}
