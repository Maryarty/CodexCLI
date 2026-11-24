// test root: pages
//! Tests of the chat's footer and status surfaces (S10; oracle
//! `bottom_pane/footer.rs`, `chat_composer/passive_hints.rs`,
//! `chat_composer/status_surface.rs`): what the shards do not say (state
//! and mouse behaviour), and the footer entries one by one while the shard's
//! `footer_snapshots` waits for the entries that need the status line.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, CopyStatus, Done};

use crate::kit::{Owner, Screen};

const IDLE: &str = include_str!("../../src/pages/session/footer/fixtures/footer_idle.jsonl");
const RUNNING: &str = include_str!("../../src/pages/session/footer/fixtures/footer_running.jsonl");

/// The idle session with three retained start-up warnings.
fn warned() -> &'static str {
    static SCRIPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SCRIPT.get_or_init(|| {
        let entries: Vec<String> = (1..=3)
            .map(|i| format!(r#"{{"source":"Startup","details":"warning {i}"}}"#))
            .collect();
        format!(
            "{IDLE}{}\n",
            format_args!(
                r#"{{"step":"screen","dir":"dialogs","ev":{{"kind":"retained","entries":[{}]}}}}"#,
                entries.join(",")
            )
        )
    })
}

fn page(script: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::UiPane)
        .size(width, height)
        .script(script)
        .open()
}

/// One footer snapshot of the oracle's `footer_snapshots`, drawn by the real
/// footer over the model its fixture script makes.
fn footer_matches(name: &'static str, width: u16) {
    let s = Screen::build(Owner::UiPane)
        .size(width, 1)
        .entry("chat", name)
        .open();
    s.assert_snap(name);
}

/// The footers whose facts the model has: hints, the queue hint, the context
/// meter in both spellings, and the transient lines.
#[test]
fn the_footer_entries_equal_the_oracle_snapshots() {
    for name in [
        "footer_shortcuts_default",
        "footer_shortcuts_context_running",
        "footer_context_tokens_used",
        "footer_composer_has_draft_queue_hint_enabled",
        "footer_ctrl_c_quit_idle",
        "footer_esc_hint_idle",
        "footer_esc_hint_primed",
    ] {
        footer_matches(name, 80);
    }
}

/// The footer's labels are the page's keys (ADR-011): a click on the hint
/// presses the key it names.
#[test]
fn a_click_on_the_shortcuts_hint_opens_the_shortcuts() {
    let mut s = page(IDLE, 80, 24);
    s.assert_contains("? for shortcuts");
    s.click_text("? for shortcuts");
    s.assert_contains("Keyboard shortcuts");
}

#[test]
fn a_click_on_the_queue_hint_queues_the_draft() {
    let mut s = page(RUNNING, 80, 24);
    s.type_text("later");
    s.assert_contains("tab to queue message");
    s.click_text("tab to queue message");
    s.assert_contains("Queued follow-up inputs");
    s.assert_contains("↳ later");
    s.assert_absent("tab to queue message");
}

#[test]
fn a_click_on_the_warning_notice_opens_the_warnings() {
    let mut s = page(warned(), 80, 24);
    s.assert_contains("⚠ 3 warnings · f2 to view");
    s.click_text("3 warnings");
    s.assert_contains("warning 1");
}

/// Ctrl+O in the viewer copies the text of the page shown (oracle `AppEvent::CopyWarning`): the
/// host's clipboard is asked, and its answer is said in the gap above the composer.
#[test]
fn ctrl_o_in_the_warnings_copies_the_page_and_says_so() {
    let mut s = page(warned(), 80, 24);
    s.click_text("3 warnings");
    s.assert_contains("warning 1");
    s.keys("ctrl+o");
    let copies: Vec<Command> = s
        .commands()
        .into_iter()
        .filter(|c| matches!(c, Command::Effect { ask: Ask::Copy { text, .. }, .. } if text == "warning 1"))
        .collect();
    assert_eq!(copies.len(), 1, "{}", s.text());
    s.answer_effect(&copies[0], Done::Copy(Ok(CopyStatus::Confirmed)));
    // The result is the composer gap's (`show_selection_copy_result`): the viewer covers it, it
    // is there when the viewer closes.
    s.keys("esc");
    s.assert_contains("Copied 9 chars to host clipboard");
}

/// With a draft in the composer the footer carries no shortcut hints (the
/// oracle's `ComposerHasDraft`), and a footer with no label is no row
/// (`footer_hint_height` is 0 when no label fits): the composer sits one row
/// lower.
#[test]
fn a_draft_takes_the_shortcut_hints_and_their_row_away() {
    // the status line is on: the context meter is in it, not in the hints
    let mut s = page(status_script("model"), 80, 12);
    s.assert_contains("? for shortcuts");
    let prompt = |s: &Screen| s.rows().iter().position(|r| r.starts_with("› "));
    let before = prompt(&s).expect("composer row");
    s.type_text("hi");
    s.assert_absent("? for shortcuts");
    s.assert_absent("for agents");
    assert_eq!(prompt(&s), Some(before + 1), "{}", s.text());
}

fn status_script(model: &str) -> &'static str {
    let script = format!(
        "{{\"step\":\"screen\",\"dir\":\"chat\",\"ev\":{{\"status\":{{\"model\":\"{model}\",\"items\":[\"model-name\"]}}}}}}\n"
    );
    Box::leak(script.into_boxed_str())
}

/// The status line yields to the draft as the draft grows (`status_budget`:
/// the composer first), and takes its rows back as it shrinks.
#[test]
fn the_status_line_yields_rows_to_a_growing_draft() {
    let mut s = page(status_script("one two three four five"), 7, 10);
    s.paste("a");
    s.assert_in_order(&["one", "two…"]);
    s.paste("\nb\nc\nd\ne");
    s.assert_contains("one…");
    s.assert_absent("two");
    // a key line of the draft taken back: the line has its second row again
    for _ in 0..3 {
        s.keys("backspace");
        s.keys("backspace");
    }
    s.assert_in_order(&["one", "two…"]);
}

/// The status line stays under the composer, at its two cells, while the
/// command popup is open above it (`slash_popup_preserves_footer_and_selection_style`:
/// the popup's bottom is the composer's top, the composer's bottom the status).
#[test]
fn the_status_line_stays_under_the_composer_while_the_popup_is_open() {
    let script: &'static str = Box::leak(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"model","effort":"high","project_root":"fast","items":["model-name","reasoning","project-root"],"colors":false}}}"#
            .to_owned()
            .into_boxed_str(),
    );
    let mut s = page(script, 80, 12);
    s.type_text("/m");
    s.keys("down");
    let row = |s: &Screen, text: &str| s.rows().iter().position(|r| r.contains(text));
    let composer = row(&s, "› /m").expect("composer row");
    let status = row(&s, "model · high · fast").expect("status row");
    assert_eq!(status, composer + 2, "{}", s.text());
    s.assert_row(status, "  model · high · fast");
    s.resize(32, 12);
    let composer = row(&s, "› /m").expect("composer row");
    let status = row(&s, "model · high · fast").expect("status row");
    assert_eq!(status, composer + 2, "{}", s.text());
}

// ---- the terminal title (oracle `chatwidget/status_surfaces.rs`, `terminal_title.rs`) ------------

const ACTION_REQUIRED: &str =
    include_str!("../oracle/fixtures/chat_status_3/action_required.jsonl");

const APPROVAL: &str = r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","itemId":"call-1","approvalId":"call-1","environmentId":null,"command":"bash -lc \"echo hello\"","cwd":"/tmp","reason":"need confirmation","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;

fn title_script(status: &str, turn_running: bool) -> &'static str {
    let mut script = format!(
        "{{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-1\"}}\n{{\"step\":\"screen\",\"dir\":\"chat\",\"ev\":{{\"status\":{status}}}}}\n"
    );
    if turn_running {
        for line in ACTION_REQUIRED
            .lines()
            .filter(|line| line.contains("turn/started"))
        {
            script.push_str(line);
            script.push('\n');
        }
    }
    Box::leak(script.into_boxed_str())
}

/// The default title while a turn runs: the spinner first (a braille frame, a space), then the
/// project; it is the project alone when the turn is over.
#[test]
fn a_running_turn_puts_the_spinner_before_the_project_in_the_title() {
    let s = page(title_script(r#"{"project_root":"project"}"#, true), 80, 12);
    let title = s.title().expect("a title");
    let frame = title
        .strip_suffix(" project")
        .unwrap_or_else(|| panic!("title {title:?}"));
    assert!(
        ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"].contains(&frame),
        "the spinner frame of {title:?}"
    );
    let idle = page(title_script(r#"{"project_root":"project"}"#, false), 80, 12);
    idle.assert_title("project");
}

/// `terminal_title_action_required_respects_spinner_setting`: the line stands for the spinner, so
/// a title without the activity item never shows it.
#[test]
fn action_required_replaces_the_spinner_item_only() {
    let with = format!(
        "{}{APPROVAL}\n",
        title_script(
            r#"{"project_root":"project","terminal_title":["project"]}"#,
            true
        )
    );
    let s = page(Box::leak(with.into_boxed_str()), 100, 24);
    s.assert_title("project");
    let with = format!(
        "{}{APPROVAL}\n",
        title_script(
            r#"{"project_root":"project","terminal_title":["activity","app-name","status","project"]}"#,
            true
        )
    );
    let s = page(Box::leak(with.into_boxed_str()), 100, 24);
    // the line, then the items that are no spinner and no run state, in their order
    s.assert_title("[ ! ] Action Required | codex | project");
}

/// `Daybreak off` until the thread says it is on, on the status line and in the title.
#[test]
fn daybreak_reads_off_until_the_thread_turns_it_on() {
    let s = page(
        title_script(
            r#"{"items":["daybreak"],"terminal_title":["daybreak"]}"#,
            false,
        ),
        80,
        12,
    );
    s.assert_title("Daybreak off");
    assert!(
        s.rows().iter().any(|row| row.trim() == "Daybreak off"),
        "the status line\n{}",
        s.text()
    );
}

// ---- the `?` shortcut overlay (bottom_pane/shortcut_overlay.rs)

/// The close hint is a button (ADR-011): a click on `esc close` puts the help away and the
/// footer is the idle hints again.
#[test]
fn a_click_on_the_close_hint_closes_the_help() {
    let mut s = page(IDLE, 80, 24);
    s.keys("?");
    s.assert_contains("Keyboard shortcuts");
    s.click_text("esc close");
    s.assert_absent("Keyboard shortcuts");
    s.assert_contains("? for shortcuts");
}

/// The rows are the user's keymap and the turn: a rebound `composer.queue` shows its key, and
/// while a turn runs `Send message` is `Queue message` and `Quit` is `Interrupt`.
#[test]
fn the_help_rows_follow_the_keymap_and_the_running_turn() {
    let row = |s: &Screen, label: &str| {
        s.rows()
            .into_iter()
            .find(|row| row.contains(label))
            .unwrap_or_else(|| panic!("no `{label}` row:\n{}", s.text()))
    };
    let mut idle = Screen::build(Owner::UiPane)
        .size(130, 24)
        .script(IDLE)
        .keymap(&[("composer.queue", "ctrl-q")])
        .open();
    idle.keys("?");
    assert!(
        row(&idle, "Send message").contains("\u{2303}q"),
        "{}",
        idle.text()
    );
    assert!(row(&idle, "Quit").contains("\u{2303}c"));
    idle.assert_absent("Interrupt");
    let mut running = Screen::build(Owner::UiPane)
        .size(130, 24)
        .script(RUNNING)
        .open();
    running.keys("?");
    running.assert_contains("Queue message");
    running.assert_contains("Interrupt");
    running.assert_absent("Send message");
}

/// `escape_closes_shortcut_help_before_inline_banner`: with the help open Esc puts the help
/// away and the banner stays; the next Esc dismisses the banner. (The shard's entry,
/// `chat/inline_banner_short`, is the chat page's to register; this is the same keys on the
/// page's own backend banner.)
#[test]
fn escape_closes_the_help_before_it_dismisses_the_banner() {
    use crate::common::start;
    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::{ScreenEv, chat};
    let mut h = start();
    h.wait_idle();
    h.fake.respond(|request| match request["method"].as_str() {
        Some("account/rateLimits/read") => Some(Ok(serde_json::json!({
            "rateLimits": {},
            "rateLimitUpsell": {
                "banner_type": "credits",
                "title": "Usage limit reached",
                "description": "Add credits to keep working.",
                "ctas": [{"action": "view_usage", "label": "View usage"}],
                "presentation": "dismissible",
            },
        }))),
        _ => None,
    });
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                account_usage: true,
                ..chat::StatusFacts::default()
            },
        )))),
    );
    h.wait_for("the banner", |s| s.contains("Usage limit reached"));
    h.press("?");
    h.settle();
    assert!(
        h.t.screen().contains("Keyboard shortcuts"),
        "{}",
        h.t.screen()
    );
    h.press("esc");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Keyboard shortcuts"), "{screen}");
    assert!(screen.contains("Usage limit reached"), "{screen}");
    h.press("esc");
    h.settle();
    assert!(
        !h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
}

// ---- the reasoning-effort animation (oracle `bottom_pane/effort_*`)

/// The chat screen's status facts, as the host sends them.
fn facts(json: &str) -> String {
    format!(r#"{{"step":"screen","dir":"chat","ev":{{"status":{json}}}}}"#)
}

/// The facts of a status line that reads the model with its effort (the
/// event replaces the facts whole).
fn effort_facts(effort: &str, extra: &str) -> String {
    facts(&format!(
        r#"{{"model":"gpt-5.4","effort":"{effort}","items":["model-with-reasoning"]{extra}}}"#
    ))
}

/// A live chat whose status line reads the model with its effort.
fn effort_chat(effort: &str) -> Screen {
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 8)
        .live()
        .dark()
        .open();
    s.feed(&effort_facts(effort, ""));
    s
}

/// A change to Ultra after the first baseline plays the status line's
/// transition (the old label scrolls out, `U L T R A` assembles) and ends on
/// the new label, the line wrapping again (`effort_status_line.rs`).
#[test]
fn a_change_to_ultra_plays_the_transition_and_ends_on_the_new_label() {
    let mut s = effort_chat("high");
    s.assert_contains("gpt-5.4 high");
    s.feed(&effort_facts("ultra", ""));
    s.assert_contains("gpt-5.4 high");
    // 620 ms of scroll, then the letters assemble
    s.wait(1_300);
    s.assert_contains("U L T R A");
    s.assert_absent("gpt-5.4");
    s.wait(3_000);
    s.assert_contains("gpt-5.4 ultra");
    s.assert_absent("U L T R A");
}

/// The transition is for a genuine change to Max or Ultra: a thread that
/// starts on Ultra (the baseline), a lowering and reduced motion play nothing.
#[test]
fn a_baseline_a_lowering_and_reduced_motion_play_nothing() {
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 8)
        .script(Box::leak(
            format!("{}\n", effort_facts("ultra", "")).into_boxed_str(),
        ))
        .open();
    s.assert_contains("gpt-5.4 ultra");
    for _ in 0..30 {
        s.wait(100);
        s.assert_absent("U L T R A");
        s.assert_contains("gpt-5.4 ultra");
    }
    // a lowering
    s.feed(&effort_facts("medium", ""));
    for _ in 0..30 {
        s.wait(100);
        s.assert_absent("U L T R A");
    }
    s.assert_contains("gpt-5.4 medium");
    // reduced motion: the animation setting off
    s.feed(&effort_facts("max", r#","animations":false"#));
    for _ in 0..30 {
        s.wait(100);
        s.assert_absent("M A X");
    }
    s.assert_contains("gpt-5.4 max");
}

/// Each look of the ignition sweeps a band of colour under the draft, keeps
/// the draft's rows as typed, and leaves the composer as it was when its run
/// ends (`effort_ignition_styles.rs`).
#[test]
fn every_ignition_look_tints_under_the_draft_and_ends() {
    for name in [
        "effort_ignition_ultra",
        "effort_ignition_ultra_aurora",
        "effort_ignition_ultra_pulse",
        "effort_ignition_max_wave",
    ] {
        let mut s = Screen::build(Owner::UiPane)
            .size(44, 3)
            .dark()
            .entry("chat", name)
            .first_frame()
            .open();
        let band =
            |s: &Screen| -> Vec<String> { (0..44).map(|x| s.grid().style(x, 1).bg).collect() };
        let resting = band(&s);
        let mut tinted = false;
        for _ in 0..40 {
            s.wait(50);
            s.assert_row_starts_with(1, "  > keep my draft exactly as typed");
            tinted |= band(&s) != resting;
        }
        assert!(
            tinted,
            "{name}: no cell of the draft row changed its background\n{}",
            s.text()
        );
        assert_eq!(band(&s), resting, "{name}: the band is gone after its run");
    }
}

/// In the composer a change to Ultra takes the prompt glyph and its accent,
/// and the draft is kept while the band sweeps and after.
#[test]
fn the_composer_keeps_its_draft_through_the_ignition() {
    let mut s = effort_chat("high");
    s.type_text("keep this draft");
    s.assert_text_style("›", "bold");
    s.feed(&effort_facts("ultra", ""));
    for _ in 0..40 {
        s.wait(50);
        s.assert_contains("keep this draft");
        s.assert_contains("» keep this draft");
    }
    s.assert_text_style("»", "bold");
}
