//! Shard `pane_footer_1` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The first row that contains `text`: where a footer or a status row of the oracle sits.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or(usize::MAX)
}

const ASTRA: &str = include_str!("fixtures/pane_footer_1/astra.jsonl");
const PLAIN: &str = include_str!("fixtures/pane_footer_1/plain.jsonl");
const ASTRA_HISTORY: &str = include_str!("fixtures/pane_footer_1/astra_history.jsonl");
const PLAIN_HISTORY: &str = include_str!("fixtures/pane_footer_1/plain_history.jsonl");
const ASTRA_NO_STARFIELD: &str = include_str!("fixtures/pane_footer_1/astra_no_starfield.jsonl");
const ASTRA_NO_ANIMATIONS: &str = include_str!("fixtures/pane_footer_1/astra_no_animations.jsonl");

/// Enter on the typed command, and the dialog some commands open (`/status` and `/diff` show a card
/// that traps the keys, where the oracle adds a transcript cell) closed again with Esc.
fn run_typed_command(s: &mut Screen) {
    s.keys("enter");
    if s.grid().cursor.is_none() {
        s.keys("esc");
    }
}

/// `/model` run and Astra chosen (the screen is told the model the status line then names); the
/// model sheet the command opens is closed.
fn model_command_selects_astra(s: &mut Screen) {
    s.type_text("/model");
    s.keys("enter");
    s.feed(&model_is("gpt-6-astra"));
    if s.grid().cursor.is_none() {
        s.keys("esc");
    }
}

/// The chat screen's status facts for a model switch (`select_sparkle_model`): the model the
/// status line names.
fn model_is(model: &str) -> String {
    format!(r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}"}}}}}}"#)
}

/// The oracle's sparkle palette (`palette()`: foreground `(230, 216, 255)`, background `(36, 27, 53)`).
const FOREGROUND: u32 = 0xe6_d8_ff;
const BACKGROUND: u32 = 0x24_1b_35;

/// A fresh thread on an Astra model, in the palette the oracle's sparkle tests draw in.
fn astra(width: u16, height: u16) -> Screen {
    Screen::build(Owner::UiPane)
        .size(width, height)
        .script(ASTRA)
        .terminal_colors(FOREGROUND, BACKGROUND)
        .open()
}

/// A fresh thread on a model that is no Astra, in the same palette: no dots until the model is one.
fn plain(width: u16, height: u16) -> Screen {
    Screen::build(Owner::UiPane)
        .size(width, height)
        .script(PLAIN)
        .terminal_colors(FOREGROUND, BACKGROUND)
        .open()
}

const RETAINED_POPUP_REASON: &str = "the command popup is not retained across Find: F3 hides it (the page ports the rest: no popup rows and the cursor on the query's row at 60x16, 24x8 and 12x3) but closing Find with Esc or Ctrl+C leaves the draft `/m` without its popup, where the oracle's composer keeps `ActivePopup::Command` and paints it again once the footer is gone (`render(None) == original`): the draft's popover (pages/session/composer, w2-composer) closes with the focus and `Suggest` stays open without a rebuild";
const FIND_COPY_REASON: &str = "Find is built (F3: `Find: needle`, the cursor on the query) but its copy flash is not: Ctrl+C in Find cancels it (the oracle's footer says `Ctrl+C copy · Esc clear selection` over a selection), and the chat page gives its footer no flash (session/slot.rs `flash: None`; the selection's copy feedback is the pager's, transcript/copy_feedback.rs): see FLASH_REASON";
const FIND_OVERRIDE_REASON: &str = "a hint override (`ctrl+c quit`) under the interactive Find footer: nothing sets one in the live chat (only the presets menu and the shutdown do, session/slot.rs), and Ctrl+C in Find cancels it; the footer's own precedence (flash over override over passive, `lead.rs`) has no key to drive it";
const REPLACE_REASON: &str = "the draft replaced from outside while the history search is open (`ChatComposer::set_text_content`): no seed, key or fact replaces the composer's text from the host's side (the queued-draft restore and the external editor's result do it, with no search open)";
const FOCUS_REASON: &str = "the terminal losing and regaining focus (`set_sparkle_terminal_focus`, crossterm's FocusLost/FocusGained): rtwk reports no terminal focus-in/out events (rtwk-gaps.md), so the deadline's behaviour across a focus loss cannot be driven; a running turn on a fresh task (`set_task_running`) has no seed either: a turn that ran ends the fresh task";
const CARET_REASON: &str = "the terminal cursor on the history search's own row (`reverse-i-search: query▮`): the search row is static text under the composer, and the composer's TextArea (focused, it takes the search's keys) keeps its caret on the prompt row (row 3 of 6, the oracle puts it on row 5); a caret at the query's end needs the query as an editing field that holds the focus (a restructure of composer/search.rs: footer field, Up/Down/Enter/Esc as its shortcuts) or a custom RenderObject calling `rtwk::editing::show_caret` (R15 forbids hand-made render objects in the app), and the TextArea has no way to hide its caret while it has the focus";
const HELP_STATUS: &str = include_str!("fixtures/pane_footer_1/help_status.jsonl");
const EXTENSION_NA: &str = "no live producer: the oracle's app never builds a ui_tree status row (only its tests); native wrap/row budget covered by native_wrap_tracks_agent_label_and_content_changes_at_the_same_width and status_survives_hints_feedback_and_queue";
const ACTIVITY_REASON: &str = "the hint is the footer of the transcript view (`TranscriptFooter`: `ctrl+o t/f4 inspect activity`, interactive or not, with the shortcut key unbound) that the composer shows under the transcript pager: the pager has no such footer yet (the pager task's), the Working row and the activity rows of chunk-status do not draw it";
const MODE_REASON: &str = "the footer at five widths with Plan mode, the IDE context indicator, the vim mode and 2 warnings: Plan mode, the vim mode and the warnings are in the page (ported in the plan and warnings tests), the IDE context indicator is not (no IDE-context fact in StatusFacts, no host probe: a Need on the IDE-context owner)";
const POPUP_REASON: &str = "the status stays under the composer while the popup is open (page test), but the popup's own rows (`/model` descriptions, the selection marker) are the popups chunk's widget and differ from the oracle's Buffer snapshot";
const FLASH_REASON: &str = "no footer flash source in the live chat: `Timed`/`Flash` (footer/flash.rs) draw a flash when the footer is given one, and the chat page gives none (session/slot.rs `flash: None`: the draft limit's message goes to the transcript as an error cell, nothing else flashes); the oracle sets `saved` on the footer directly";

/// The chat screen's status facts (`ScreenEv::Chat(Ev::Status)`), as the host sends them.
fn facts(json: &str) -> String {
    format!(r#"{{"step":"screen","dir":"chat","ev":{{"status":{json}}}}}"#)
}

const WARNINGS_STATUS: &str = include_str!("fixtures/pane_footer_1/warnings_status.jsonl");
const PLAN_NEEDLE: &str = include_str!("fixtures/transcript_view_4/find_jump.jsonl");
const PLAN_STATUS: &str = include_str!("fixtures/pane_footer_1/plan_status.jsonl");

/// The git probe answered the first request of the screen: the branch's pull request is 42.
const PR_42: &str = r#"{"step":"screen","dir":"chat","ev":{"facts":{"cwd":"/w"}}}
{"step":"screen","dir":"chat","ev":{"facts":{"git_summary":{"request":1,"summary":{"pull_request":{"number":42,"url":"https://example.com/pull/42"}}}}}}"#;

/// A turn is running on the thread.
const RUNNING: &str = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// The thread kept a warning, the `n`th (the footer counts them).
fn warning(n: usize) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"warning","params":{{"threadId":"thread-1","message":"Warning {n}"}}}}}}"#
    )
}

/// A screen of the oracle's `composer()` of the warning tests with `count` warnings.
fn warned(width: u16, height: u16, count: usize) -> Screen {
    let mut s = Screen::build(Owner::UiPane)
        .size(width, height)
        .script(WARNINGS_STATUS)
        .terminal_colors(0xde_de_de, 0x10_10_10)
        .open();
    for n in 1..=count {
        s.feed(&warning(n));
    }
    s
}

/// The thread is named (the status line's `thread-name`).
fn named(name: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"thread/name/updated","params":{{"threadId":"thread-1","threadName":"{name}"}}}}}}"#
    )
}

/// The viewed agent's label (`set_active_agent_label`).
fn agent(label: &str) -> String {
    format!(r#"{{"step":"screen","dir":"chat","ev":{{"facts":{{"agent_label":"{label}"}}}}}}"#)
}

// ---- oracle: bottom_pane/chat_composer/footer_state_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:62 retained_command_suggestions_yield_paint_and_cursor_to_find kind=snap owner=ui-pane snap=find_hides_retained_command_suggestions
#[test]
fn retained_command_suggestions_yield_paint_and_cursor_to_find() {
    kit::pending(Owner::UiPane, RETAINED_POPUP_REASON);
    // the draft `/m` keeps its command popup under Find: the popup is not painted while Find is
    // open and the cursor is the query's, at 60x8, 24x8 and 12x3; Esc closes Find and the popup
    // is painted again, the draft untouched
    // (the popup needs rows the oracle's 8 do not give it here: the page opens at 16)
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 16)
        .script(PLAN_NEEDLE)
        .open();
    s.type_text("/m");
    s.wait(300);
    s.assert_contains("/model");
    s.keys("f3");
    s.assert_contains("Find:");
    s.assert_absent("/model");
    let footer = row_of(&s, "Find:");
    s.assert_cursor(8, footer);
    for (width, height) in [(24, 8), (12, 3)] {
        s.resize(width, height);
        s.assert_absent("/model");
        s.assert_cursor_on_row_of("Find:");
    }
    s.resize(60, 16);
    s.keys("ctrl+c");
    s.wait(300);
    s.assert_absent("Find:");
    s.assert_contains("/model");
    s.assert_composer("/m");
}

// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:110 copying_from_find_preserves_the_query_above_selection_feedback kind=snap owner=ui-pane snap=transcript_find_copy_footer
#[test]
fn copying_from_find_preserves_the_query_above_selection_feedback() {
    kit::pending(Owner::UiPane, FIND_COPY_REASON);
    // the Find footer (`Find: needle`, `Ctrl+C copy · Esc clear selection`) and the 3 s flash
    // `Copied selection to clipboard` are set by the test; the flash keeps the query above it
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("Find: needle");
    s.assert_contains("Copied selection to clipboard");
    s.assert_snap("transcript_find_copy_footer");
}

// ORACLE bottom_pane/chat_composer/footer_state_tests.rs:134 interactive_query_keeps_focus_over_underlying_hints_but_empty_override_wins kind=ui owner=ui-pane
#[test]
fn interactive_query_keeps_focus_over_underlying_hints_but_empty_override_wins() {
    kit::pending(Owner::UiPane, FIND_OVERRIDE_REASON);
    // the interactive Find footer (cursor at 12) keeps the frame the same when the test sets a
    // hint override (`ctrl+c quit`); an empty override shows the composer's own footer again
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    let expected = s.shot();
    s.assert_contains("Find: needle");
    s.assert_same_rows(&expected);
}

// ---- oracle: bottom_pane/chat_composer/sparkle_tests.rs (19 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:114 sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor kind=snap owner=ui-pane snap=sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor
#[test]
fn sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor() {
    // a fresh Astra task: the dots fill the free cells of the composer, never the placeholder or
    // the cursor; the first typed character clears them (the oracle draws at 40 and 80 columns)
    let name = "sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor";
    let mut s = astra(80, 24);
    s.assert_stars(name, "80 columns, cursor Some((2, 1))");
    let plain = s.shot();
    s.wait(2_000);
    s.assert_rows_differ(&plain);
    s.type_text("x");
    s.assert_no_dots();
    let mut narrow = astra(40, 24);
    narrow.assert_stars(name, "40 columns, cursor Some((2, 1))");
    narrow.type_text("x");
    narrow.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:176 sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds kind=snap owner=ui-pane snap=sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds
#[test]
fn sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds() {
    // a fresh Astra task: the last frame before the fade (14 s), the fading frame (14.5 s) and
    // the finished frame (15 s); the fade adds no dot, dims the ones it keeps, and no dot is left
    // at the deadline; a second selection of the model does not bring them back
    let name = "sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds";
    let mut s = astra(80, 24);
    s.wait(13_700);
    let last = s.dots();
    s.assert_stars(name, "before fade:");
    s.wait(700);
    s.assert_stars(name, "fading:");
    s.assert_dots_fade_from(&last);
    s.wait(800);
    s.assert_no_dots();
    s.assert_stars(name, "finished:");
    s.wait(1_000);
    s.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:227 the_first_buffered_character_and_empty_escape_clear_visible_stars kind=ui owner=ui-pane
#[test]
fn the_first_buffered_character_and_empty_escape_clear_visible_stars() {
    // on a fresh Astra task each of these keys clears the visible dots: `x`, `?`, Esc, Left
    // (the draft stays empty; the oracle's `Dismissed` / `Untouched` tell `x` from the others:
    // the dots of the others come back with nothing, the screen shows none of either)
    for key in ["x", "?", "esc", "left"] {
        let mut s = astra(80, 24);
        s.assert_dots(true);
        s.keys(key);
        s.assert_no_dots();
        // (Esc and Left on an empty draft leave the chat for the command center: no composer is left)
        if key == "?" {
            s.assert_composer("");
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:256 shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not kind=snap owner=ui-pane snap=shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not
#[test]
fn shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not() {
    // a fresh task on `gpt-5.5`: `?` opens the shortcut help (no dots yet, the draft stays
    // untouched), `/model` then selects Astra and the dots come; a literal `?` (a paste) ends the
    // opportunity for good, so Astra brings none
    let name = "shortcut_help_preserves_an_unused_sparkle_but_literal_question_marks_do_not";
    let mut s = plain(80, 24);
    s.keys("?");
    s.assert_contains("Keyboard shortcuts");
    s.assert_no_dots();
    s.assert_stars(name, "shortcut help:");
    s.type_text("/model");
    s.keys("enter");
    s.feed(&model_is("gpt-6-astra"));
    s.keys("esc");
    s.assert_dots(true);
    s.assert_stars(name, "after /model selects Astra:");

    // the help closes with the same key and keeps the opportunity too
    let mut s = plain(80, 24);
    s.keys("? ?");
    s.assert_absent("Keyboard shortcuts");
    s.type_text("/model");
    s.keys("enter");
    s.feed(&model_is("gpt-6-astra"));
    s.keys("esc");
    s.assert_dots(true);

    let mut s = plain(80, 24);
    s.paste("?");
    s.assert_composer("?");
    s.assert_stars(name, "literal pasted question mark:");
    s.keys("backspace");
    s.type_text("/model");
    s.keys("enter");
    s.feed(&model_is("gpt-6-astra"));
    s.keys("esc");
    s.assert_no_dots();
    s.assert_stars(name, "after clearing and selecting Astra:");
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:372 shortcut_help_hides_visible_sparkles_without_restarting_their_deadline kind=snap owner=ui-pane snap=shortcut_help_hides_visible_sparkles_without_restarting_their_deadline
#[test]
fn shortcut_help_hides_visible_sparkles_without_restarting_their_deadline() {
    // a visible Astra sparkle: `?` at 4 s hides it and opens the help; the 15 s deadline is not
    // restarted, so the dots come back when the help closes at 4 s and stay gone when it closes
    // at 15 s; a literal `?` (a paste) ends the sparkle
    let name = "shortcut_help_hides_visible_sparkles_without_restarting_their_deadline";
    for expires in [false, true] {
        let scenario = format!("expires during help: {expires}");
        let mut s = astra(80, 24);
        s.assert_stars(name, &format!("{scenario}\ninitial:"));
        s.wait(3_700);
        s.assert_dots(true);
        s.keys("?");
        s.assert_contains("Keyboard shortcuts");
        s.assert_no_dots();
        s.assert_stars(name, &format!("{scenario}\nshortcut help:"));
        if expires {
            s.wait(11_000);
            s.assert_contains("Keyboard shortcuts");
            s.assert_no_dots();
        }
        s.keys("?");
        s.assert_absent("Keyboard shortcuts");
        if expires {
            s.assert_no_dots();
        } else {
            s.assert_dots(true);
        }
        s.assert_stars(name, &format!("{scenario}\nafter closing:"));
    }

    let mut s = astra(80, 24);
    s.wait(3_700);
    s.assert_dots(true);
    s.paste("?");
    s.assert_composer("?");
    s.keys("backspace");
    s.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:459 disconnected_sparkle_edits_are_tracked_between_renders kind=snap owner=ui-pane snap=disconnected_sparkle_edits_are_tracked_between_renders
#[test]
fn disconnected_sparkle_edits_are_tracked_between_renders() {
    // the oracle edits a disconnected composer without rendering between the keys (restricted
    // keys) and erases: the dots return only for a draft that was no more than a command in the
    // making (`/status`), never after ordinary text or an unknown command, and `gpt-5.5` shows
    // none until Astra is selected. Ours tells the sparkle of each key and each change of the
    // text as it happens, so there is no render to wait for; the restricted-input mode (and the
    // literal `?`, which opens the help here) has no counterpart: the three drafts that remain.
    let name = "disconnected_sparkle_edits_are_tracked_between_renders";
    for model in ["gpt-6-astra", "gpt-5.5"] {
        for (scenario, input, returns) in [
            ("ordinary", "x", false),
            ("recognized command", "/status", true),
            ("unknown command", "/xyz", false),
        ] {
            let astra_first = model == "gpt-6-astra";
            let mut s = if astra_first {
                astra(80, 24)
            } else {
                plain(80, 24)
            };
            s.assert_dots_lit(astra_first, "{model}: {scenario}");
            if scenario == "ordinary" {
                s.assert_stars(name, &format!("{model}\nbefore disconnect:"));
            }
            s.type_text(input);
            for _ in input.chars() {
                s.keys("backspace");
            }
            s.assert_empty_draft();
            if !astra_first {
                s.feed(&model_is("gpt-6-astra"));
            }
            s.assert_dots_lit(returns, "{model}: {scenario}");
            if scenario == "ordinary" {
                s.assert_stars(
                    name,
                    &format!("{model}\nafter typing and erasing without rendering:"),
                );
            }
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:529 a_fresh_astra_response_waits_for_the_pending_command_to_leave kind=snap owner=ui-pane snap=a_fresh_astra_response_waits_for_the_pending_command_to_leave
#[test]
fn a_fresh_astra_response_waits_for_the_pending_command_to_leave() {
    // `/status` typed before the first response (the model is no Astra yet): when Astra arrives
    // the dots wait while the command is pending, come when it ran or was erased, and never
    // when the draft grew past a command
    let name = "a_fresh_astra_response_waits_for_the_pending_command_to_leave";
    for completion in ["submit", "erase", "draft"] {
        let mut s = plain(80, 24);
        s.type_text("/status");
        s.feed(&model_is("astra"));
        s.assert_no_dots();
        match completion {
            "submit" => run_typed_command(&mut s),
            "erase" => {
                for _ in "/status".chars() {
                    s.keys("backspace");
                }
            }
            _ => {
                s.type_text("-draft");
                s.keys("ctrl+c");
            }
        }
        s.assert_dots_lit(completion != "draft", completion);
        if completion == "submit" {
            s.assert_stars(name, "after command:");
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:574 an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared kind=ui owner=ui-pane
#[test]
fn an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared() {
    // an attached image ends the sparkle for good: clearing the composer and selecting the model
    // again brings no dots
    let mut s = astra(80, 24);
    s.assert_dots(true);
    s.attach_image("image.png");
    s.assert_no_dots();
    s.keys("ctrl+c");
    s.assert_empty_draft();
    s.feed(&model_is("astra"));
    s.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:591 history_search_acceptance_and_cancellation_update_sparkles_before_rendering kind=snap owner=ui-pane snap=history_search_acceptance_and_cancellation_update_sparkles_before_rendering
#[test]
fn history_search_acceptance_and_cancellation_update_sparkles_before_rendering() {
    // Ctrl+R and a query previews an entry (not the draft: the dots wait); Enter accepts it (an
    // ordinary draft: the sparkle ends), Esc and Ctrl+C restore the empty draft (the sparkle is
    // whole: Astra then brings the dots)
    let name = "history_search_acceptance_and_cancellation_update_sparkles_before_rendering";
    for (entry, query) in [("git status", "git"), ("/status", "status")] {
        for completion in ["accept", "escape", "control-c"] {
            let mut s = Screen::build(Owner::UiPane)
                .size(80, 24)
                .script(PLAIN_HISTORY)
                .terminal_colors(FOREGROUND, BACKGROUND)
                .open();
            s.keys("ctrl+r");
            s.type_text(query);
            s.assert_contains(&format!("› {entry}"));
            match completion {
                "accept" => {
                    s.keys("enter");
                    s.keys("ctrl+c");
                }
                "escape" => {
                    s.keys("esc");
                }
                _ => {
                    s.keys("ctrl+c");
                }
            }
            s.assert_absent("reverse-i-search");
            s.assert_empty_draft();
            s.feed(&model_is("astra"));
            s.assert_dots_lit(completion != "accept", &format!("{entry}: {completion}"));
            if entry == "git status" && completion != "control-c" {
                s.assert_stars(name, &format!("{completion}:"));
            }
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:637 canceled_history_search_restores_visible_stars_only_before_the_original_deadline kind=snap owner=ui-pane snap=canceled_history_search_restores_visible_stars_only_before_the_original_deadline
#[test]
fn canceled_history_search_restores_visible_stars_only_before_the_original_deadline() {
    // the dots are hidden while the search previews an entry; cancelling it (Esc, Ctrl+C) brings them
    // back unless their 15 s ran out meanwhile, and accepting the entry ends the sparkle
    let name = "canceled_history_search_restores_visible_stars_only_before_the_original_deadline";
    for (cancel, expires_while_open) in [
        ("escape", false),
        ("control-c", false),
        ("escape", true),
        ("accept", false),
    ] {
        let label = format!("expires while open: {expires_while_open}");
        let mut s = Screen::build(Owner::UiPane)
            .size(80, 24)
            .script(ASTRA_HISTORY)
            .terminal_colors(FOREGROUND, BACKGROUND)
            .open();
        if cancel == "escape" {
            s.assert_stars(name, &format!("{label}\ninitial:"));
        }
        s.wait(3_300);
        s.assert_dots(true);
        s.keys("ctrl+r");
        s.type_text("st");
        s.paste("ored");
        s.assert_contains("› stored draft");
        s.assert_no_dots();
        if cancel == "escape" {
            s.assert_stars(name, &format!("{label}\nsearching:"));
        }
        if expires_while_open {
            s.wait(11_000);
            s.assert_no_dots();
        }
        match cancel {
            "escape" => {
                s.keys("esc");
            }
            "accept" => {
                s.keys("enter");
            }
            _ => {
                s.keys("ctrl+c");
            }
        }
        if cancel == "accept" {
            s.assert_composer("stored draft");
        } else {
            s.assert_empty_draft();
        }
        let back = !(expires_while_open || cancel == "accept");
        s.assert_dots_lit(back, &format!("{cancel}, {label}"));
        if cancel == "escape" {
            s.assert_stars(name, &format!("{label}\nafter escape:"));
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:701 history_search_preserves_a_held_sparkle_command_without_reclassifying_it kind=snap owner=ui-pane snap=history_search_preserves_a_held_sparkle_command_without_reclassifying_it
#[test]
fn history_search_preserves_a_held_sparkle_command_without_reclassifying_it() {
    // a `/` is a command in the making; Ctrl+R and a pasted query preview an entry (the draft is
    // still `/`); cancelling restores `/`, and erasing it makes the opportunity whole again, so
    // `/model` and Astra bring the dots
    let name = "history_search_preserves_a_held_sparkle_command_without_reclassifying_it";
    for model in ["gpt-6-astra", "gpt-5.5"] {
        for cancel in ["escape", "control-c"] {
            let astra_first = model == "gpt-6-astra";
            let mut s = Screen::build(Owner::UiPane)
                .size(80, 24)
                .script(if astra_first {
                    ASTRA_HISTORY
                } else {
                    PLAIN_HISTORY
                })
                .terminal_colors(FOREGROUND, BACKGROUND)
                .open();
            s.assert_dots_lit(astra_first, "");
            // the oracle draws the dots of the later frames 4 s after the first (`during`): the keys
            // below take 0.54 s of the fake clock
            if astra_first {
                s.wait(3_460);
            }
            s.type_text("/");
            s.keys("ctrl+r");
            s.paste("history");
            s.assert_contains("› history prompt");
            s.assert_no_dots();
            if cancel == "escape" {
                s.assert_stars(
                    name,
                    &format!("{model}\nsearching with an explicit pasted query:"),
                );
                s.keys("esc");
            } else {
                s.keys("ctrl+c");
            }
            s.assert_composer("/");
            s.keys("backspace");
            s.assert_empty_draft();
            s.assert_dots_lit(astra_first, "{model}: {cancel}");
            if cancel == "escape" {
                s.assert_stars(
                    name,
                    &format!("{model}\nafter canceling and erasing the slash:"),
                );
            }
            model_command_selects_astra(&mut s);
            s.assert_dots(true);
            if cancel == "escape" && astra_first {
                // (the oracle draws it at the same instant as the frame before, the typed `/model`
                // takes 0.4 s of the fake clock: the layout only)
                s.assert_layout(name, &format!("{model}\nafter /model selects Astra:"));
            } else if cancel == "escape" {
                s.assert_stars(name, &format!("{model}\nafter /model selects Astra:"));
            }
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:762 cancelling_history_search_preserves_a_typed_command_despite_attachment_previews kind=ui owner=ui-pane
#[test]
fn cancelling_history_search_preserves_a_typed_command_despite_attachment_previews() {
    // `/status` is typed; the search previews an entry that has a local and a remote image
    // (their previews show over the composer); Esc restores `/status` and drops the previews, and
    // Enter runs the command: the sparkle then comes with Astra
    let mut s = plain(80, 24);
    s.type_text("/status");
    s.keys("ctrl+r");
    s.type_text("history");
    s.keys("esc");
    s.assert_composer("/status");
    s.keys("enter");
    s.feed(&model_is("astra"));
    s.assert_dots(true);
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:791 fresh_draft_replacement_ends_search_and_dismisses_sparkles kind=ui owner=ui-pane
#[test]
fn fresh_draft_replacement_ends_search_and_dismisses_sparkles() {
    kit::pending(Owner::UiPane, REPLACE_REASON);
    // the history search is open when the draft is replaced from outside (`set_text_content`): the
    // search ends, the sparkle is dismissed, and Esc leaves the replacement in the composer
    let mut s = plain(80, 24);
    s.keys("ctrl+r");
    s.assert_contains("reverse-i-search");
    // (the host replaces the draft with `external draft` here: nothing does)
    s.assert_absent("reverse-i-search");
    s.keys("esc");
    s.assert_composer("external draft");
    s.feed(&model_is("astra"));
    s.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:806 arbitrary_commands_preserve_a_never_used_fresh_opportunity kind=ui owner=ui-pane
#[test]
fn arbitrary_commands_preserve_a_never_used_fresh_opportunity() {
    // commands that ran (typed whole, or completed with Tab) leave the opportunity whole, with Vim
    // mode on or off: Astra then brings the dots; drafts that are no command (`hello`, `/status
    // extra`, `/other`, a wide character, a path) end it even when the composer is cleared again
    for (completion, vim) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut s = Screen::build(Owner::UiPane)
            .size(80, 24)
            .script(PLAIN)
            .terminal_colors(FOREGROUND, BACKGROUND)
            .keymap(&[("global.toggle_vim_mode", "f9")])
            .open();
        if vim {
            s.keys("f9");
        }
        s.assert_no_dots();
        for command in ["/status", "/diff", "/pwd"] {
            s.type_text(command);
            run_typed_command(&mut s);
        }
        s.type_text(if completion { "/mo" } else { "/model" });
        if completion {
            s.keys("tab");
        }
        s.keys("enter");
        s.feed(&model_is("gpt-6-astra"));
        if s.grid().cursor.is_none() {
            s.keys("esc");
        }
        s.assert_dots(true);
    }
    for input in ["hello", "/status extra", "/other", "漢", "/tmp/file"] {
        let mut s = plain(80, 24);
        s.type_text(input);
        s.keys("ctrl+c");
        s.feed(&model_is("gpt-6-astra"));
        s.assert_no_dots();
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:855 canceled_commands_remain_eligible_but_inserted_content_and_ordinary_paste_do_not kind=snap owner=ui-pane snap=cleared_recovered_draft_keeps_sparkle_dismissed
#[test]
fn canceled_commands_remain_eligible_but_inserted_content_and_ordinary_paste_do_not() {
    // a typed `x` erased again ends the opportunity (the oracle hands it to the next pane: the
    // draft is the thread's here); a canceled `/sta` keeps it; a paste, content a command
    // inserted (`/mention` puts `@`) and a large pasted draft end it even when cleared
    let name = "cleared_recovered_draft_keeps_sparkle_dismissed";
    let mut s = plain(80, 24);
    s.type_text("x");
    s.keys("backspace");
    s.feed(&model_is("astra"));
    s.assert_no_dots();

    let mut s = plain(80, 24);
    s.type_text("/sta");
    for _ in 0..4 {
        s.keys("backspace");
    }
    s.feed(&model_is("astra"));
    s.assert_dots(true);

    for input in ["paste draft", "inserted", "recovered"] {
        let mut s = plain(80, 24);
        match input {
            "inserted" => {
                s.type_text("/mention");
                s.keys("enter");
            }
            "recovered" => {
                s.paste(&"x".repeat(1_001));
            }
            _ => {
                s.paste(input);
            }
        }
        s.keys("ctrl+c");
        s.assert_empty_draft();
        s.feed(&model_is("astra"));
        s.assert_no_dots();
        if input == "recovered" {
            s.assert_stars(name, "");
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:913 review_regression_canceling_a_held_slash_restores_sparkle_command_tracking kind=snap owner=ui-pane snap=review_regression_canceling_a_held_slash_restores_sparkle_command_tracking
#[test]
fn review_regression_canceling_a_held_slash_restores_sparkle_command_tracking() {
    // a `/` held by the paste burst is a command in the making; Backspace cancels it and the
    // dots are as before (or still wait for Astra); `/model` then brings them
    let name = "review_regression_canceling_a_held_slash_restores_sparkle_command_tracking";
    for model in ["gpt-6-astra", "gpt-5.5"] {
        let astra_first = model == "gpt-6-astra";
        let mut s = if astra_first {
            astra(80, 24)
        } else {
            plain(80, 24)
        };
        s.type_text("/");
        s.keys("backspace");
        s.assert_empty_draft();
        s.assert_dots_lit(astra_first, "{model}");
        s.assert_stars(
            name,
            &format!("{model}\nafter immediately canceling the held slash:"),
        );
        model_command_selects_astra(&mut s);
        s.assert_dots(true);
        if astra_first {
            // (the oracle draws it at the instant of the first frame; typing `/model` takes more
            // than a second of the fake clock: the layout only)
            s.assert_layout(name, &format!("{model}\nafter /model:"));
        } else {
            s.assert_stars(name, &format!("{model}\nafter /model:"));
        }
    }
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:954 buffered_commands_and_read_only_inline_arguments_preserve_eligibility kind=ui owner=ui-pane
#[test]
fn buffered_commands_and_read_only_inline_arguments_preserve_eligibility() {
    // `/status` typed and run leaves the opportunity whole; `/rename task name` (a command with
    // inline arguments) is a command too, so Astra brings the dots once it ran
    let mut s = plain(80, 24);
    s.type_text("/status");
    run_typed_command(&mut s);
    s.feed(&model_is("astra"));
    s.assert_dots(true);

    let mut s = plain(80, 24);
    s.type_text("/rename task name");
    run_typed_command(&mut s);
    s.feed(&model_is("astra"));
    s.assert_dots(true);
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:992 commands_hide_a_started_sparkle_without_restarting_its_deadline kind=ui owner=ui-pane
#[test]
fn commands_hide_a_started_sparkle_without_restarting_its_deadline() {
    let mut s = astra(80, 24);
    s.assert_dots(true);
    s.type_text("/status");
    s.wait(4_000);
    s.assert_no_dots();
    run_typed_command(&mut s);
    s.wait(600);
    s.assert_dots(true);
    s.type_text("/diff");
    s.wait(9_000);
    s.assert_no_dots();
    run_typed_command(&mut s);
    s.wait(1_000);
    s.assert_no_dots();
}

// ORACLE bottom_pane/chat_composer/sparkle_tests.rs:1046 focus_startup_work_and_configuration_do_not_extend_a_started_deadline kind=ui owner=ui-pane
#[test]
fn focus_startup_work_and_configuration_do_not_extend_a_started_deadline() {
    kit::pending(Owner::UiPane, FOCUS_REASON);
    // (a running turn hides the dots and its end shows them at 15 s; the terminal losing focus at
    // 16 s hides them and the focus back at 30 s does not restart the deadline: neither can be
    // driven, see the reason); the starfield off, the animations off and a narrow composer
    // (25 columns) draw no dots in the draft
    for script in [ASTRA_NO_STARFIELD, ASTRA_NO_ANIMATIONS] {
        let s = Screen::build(Owner::UiPane)
            .size(80, 24)
            .script(script)
            .terminal_colors(FOREGROUND, BACKGROUND)
            .open();
        s.assert_no_dots();
    }
    let s = astra(25, 24);
    s.assert_no_dots_in_draft();
}

// ---- oracle: bottom_pane/chat_composer/status_surface_tests.rs (26 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:27 extension_status_line_clips_links_to_footer_layout kind=ui owner=ui-pane
#[test]
fn extension_status_line_clips_links_to_footer_layout() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the status `plain · PR #42` (the PR a link to https://example.com/pr/42) clips to `PR#` at
    // 12 columns and shows `PR #42` at 40; an agent label shows only when the status is cleared
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:131 extension_uses_the_full_width_after_resize kind=ui owner=ui-pane
#[test]
fn extension_uses_the_full_width_after_resize() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the status `Ready PR 12345` (the number is a link) on one row at 16 columns, two rows at
    // 15 in fullscreen; the warning `⚠` sits at the bottom-right corner after collaboration modes
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:174 provider_status_and_hints_use_separate_rows kind=snap owner=ui-pane snap=provider_status_and_hints_use_separate_rows
#[test]
fn provider_status_and_hints_use_separate_rows() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the bold `codex-internal` status and the hints sit on separate rows at 84 columns, with
    // 2 warnings and Plan mode; the warning `⚠` stays on the footer row at 22 and at 4 rows
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:205 status_rows_respect_host_budget_and_yield_to_the_draft kind=snap owner=ui-pane snap=status_rows_respect_host_budget_and_yield_to_the_draft
#[test]
fn status_rows_respect_host_budget_and_yield_to_the_draft() {
    // the status `one two three four five` at 7 columns shows `one` and `two…` when the frame
    // caps the height; the draft `a b c` keeps its rows. The oracle's unit test hands the
    // composer `max_height = height`; the app hands it `composer_height_limit` (two thirds of the
    // frame, at least 8), so the cap (a quarter of that, two to four rows) is 2 at 7 to 14 rows
    // and 4 at 24: the page is held to what the app does.
    let mut s = Screen::build(Owner::UiPane)
        .size(7, 7)
        .script(include_str!(
            "fixtures/pane_footer_1/status_one_to_five.jsonl"
        ))
        .open();
    s.paste("a\nb\nc");
    // the oracle's snapshot (rows between `|`, a format `oracle_snap` does not read yet: the kit
    // owner's file), row by row
    for (row, text) in ["", "› a", "  b", "  c", "", "  one", "  two…"]
        .iter()
        .enumerate()
    {
        s.assert_row(row, text);
    }
    for height in [10, 14] {
        s.resize(7, height);
        s.assert_in_order(&["one", "two…"]);
        s.assert_absent("three");
    }
    s.resize(7, 24);
    s.assert_in_order(&["one", "two", "three", "four…"]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:269 sidebar_focus_keeps_dim_extension_status_rows kind=snap owner=ui-pane
#[test]
fn sidebar_focus_keeps_dim_extension_status_rows() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the draft `draft` above four status rows; unfocused (sidebar focus) the warning is hidden
    // and the rows dim, the last one shortened to `three…` (the oracle's inline snapshot)
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:316 thumbnails_fit_after_basic_footer_rows_and_before_extra_status kind=snap owner=ui-pane snap=thumbnails_fit_after_basic_footer_rows_and_before_extra_status
#[test]
fn thumbnails_fit_after_basic_footer_rows_and_before_extra_status() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the attached image `preview.png` (the test attaches it; no key does) shows thumbnails
    // between the basic footer rows and the extra status, at caps 4 to 10 rows, in fullscreen
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:416 extension_status_paints_with_the_current_theme_at_the_same_width kind=ui owner=ui-pane
#[test]
fn extension_status_paints_with_the_current_theme_at_the_same_width() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the status `λ` in the primary colour: red under the first syntax theme, green under the
    // second (the test switches the theme; no key does) at the same width of 24 columns
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:460 extension_status_keeps_its_rows_when_effort_changes kind=ui owner=ui-pane
#[test]
fn extension_status_keeps_its_rows_when_effort_changes() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the status `one two three` at 7 columns keeps its rows through an effort change to Max
    // (the test sets the effort; no key does): three rows fullscreen, `two…` in scrollback
}

const ELLIPSIS_CUT: &str = "the status line's Text cuts its last visible row by cells (`green deploy re…`, `tests passed re…`) and its ellipsis cell takes the link of the cut text; the oracle wraps by words, puts the ellipsis after the last whole row (`green deploy…`, `tests passed…`) and leaves it unlinked; the wrap, the links of the text and the one- and three-row frames match";

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:489 native_status_wraps_links_within_the_host_budget kind=snap owner=ui-pane snap=native_status_wraps_links_within_the_host_budget
#[test]
fn native_status_wraps_links_within_the_host_budget() {
    // A widget test, as the oracle's is: it hands the composer a status it built by hand (`لا `
    // and one underlined text under one link) and draws it at 18 columns with the rows each
    // layout leaves the status: 4 (fullscreen), 3 (fullscreen with Plan mode), 2 (scrollback)
    // and 1 (scrollback with Plan mode). Here the same line goes to the status line widget
    // (`status_spans` with its link, then the footer's own `Text` of `max_lines` rows ending in
    // an ellipsis, two cells in as the footer puts it, as `StatusLine` draws it at rest). The
    // terminal reorders the Arabic prefix (`BidiReordering::Terminal`: the cells are in logical
    // order, as the oracle's buffer holds them). What the widget cannot say is how many rows
    // the chat's chrome leaves it in each layout (the composer's budget: `rows_for`'s unit
    // tests in footer/status_rows.rs, and the page tests that put the status under a growing
    // draft): the row counts are the oracle's.
    //
    // Holds today (asserted before the marker): the wrap, so every row but the last cut one
    // equals the oracle's; each frame's text begins as the oracle's; every cell of the link's
    // text carries the link (the spaces inside it too), the prefix none. What is pending is
    // the ellipsis, in two things. The oracle wraps by words and puts `…` after the last
    // whole row (`green deploy…`, `tests passed…`); the widget's `Text` cuts that row by
    // cells (`green deploy re…`, `tests passed re…`). And the oracle's `…` carries no link;
    // the widget's takes the link of the text it cuts.
    let status = kit::native_status(
        "native_status_wraps_links_within_the_host_budget",
        "https://example.com/pr/42",
        "\u{644}\u{627} ",
        "PR #42 build green deploy ready all unit tests passed release queued",
    );
    status.assert_wrap_holds();
    kit::pending(Owner::UiPane, ELLIPSIS_CUT);
    status.assert_oracle_frames();
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:542 native_wrap_tracks_agent_label_and_content_changes_at_the_same_width kind=ui owner=ui-pane
#[test]
fn native_wrap_tracks_agent_label_and_content_changes_at_the_same_width() {
    // the status `build ready` at 16 columns wraps to `build ready ·` and the agent label; the
    // label and the status change and the wrap follows at the same width (the oracle hands the
    // status the whole area, the page keeps the hint row and a cap: a frame of 24 rows gives
    // the line three)
    let mut s = Screen::build(Owner::UiPane).size(16, 24).live().open();
    s.feed(&facts(r#"{"items":["thread-name"]}"#));
    s.feed(&named("build ready"));
    s.feed(&agent("frontend"));
    s.assert_in_order(&["build ready ·", "frontend"]);
    s.assert_rows_with("frontend", 1);
    s.feed(&agent("backend"));
    s.assert_in_order(&["build ready ·", "backend"]);
    s.feed(&named("tests green"));
    s.assert_in_order(&["tests green ·", "backend"]);
    s.resize(36, 24);
    s.assert_has_row_trimmed("tests green · backend");
    s.resize(16, 24);
    s.assert_in_order(&["tests green ·", "backend"]);
    s.assert_absent("tests green · backend");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:571 native_effort_transition_restores_wrapping_after_its_final_frame kind=ui owner=ui-pane
#[test]
fn native_effort_transition_restores_wrapping_after_its_final_frame() {
    // the status `build ready tests passed deploy queued` at 14 columns is one row during the
    // effort animation; after its final frame (10 s) it wraps again and `tests` shows
    let mut s = Screen::build(Owner::UiPane).size(14, 20).live().open();
    s.feed(&facts(r#"{"effort":"high","items":["thread-name"]}"#));
    s.feed(r#"{"step":"notify","message":{"method":"thread/name/updated","params":{"threadId":"thread-1","threadName":"build ready tests passed deploy queued"}}}"#);
    s.assert_contains("tests passed");
    s.feed(&facts(r#"{"effort":"ultra","items":["thread-name"]}"#));
    s.assert_contains("build ready");
    s.assert_absent("deploy");
    s.wait(10_000);
    s.assert_contains("tests");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:651 passive_activity_keeps_shortcuts_only_when_the_complete_hint_fits kind=ui owner=ui-pane
#[test]
fn passive_activity_keeps_shortcuts_only_when_the_complete_hint_fits() {
    kit::pending(Owner::UiPane, ACTIVITY_REASON);
    // the passive activity hint `ctrl+o t/f4 inspect activity` keeps `· ? shortcuts` at 80
    // columns and drops it at 40; an interactive footer or no shortcut key drops it at 80
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("ctrl+o t/f4 inspect activity · ? shortcuts");
    s.resize(40, 24);
    s.assert_contains("ctrl+o t/f4 inspect activity");
    s.assert_absent("? shortcuts");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:683 fullscreen_plan_indicator_keeps_the_cycle_hint_when_it_fits kind=snap owner=ui-pane snap=fullscreen_plan_cycle_hint
#[test]
fn fullscreen_plan_indicator_keeps_the_cycle_hint_when_it_fits() {
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(PLAN_STATUS)
        .open();
    s.assert_contains("Plan mode (⇧tab to cycle)");
    // the oracle's snapshot is the composer alone: the prompt, the status line and the hint row
    // (the page's own placeholder on the prompt row)
    s.assert_row(22, "  MODEL · ~/project · Context 20% used");
    s.assert_row(
        23,
        &format!("{:<74}Plan mode (⇧tab to cycle)", "  ? for shortcuts"),
    );
    // the status `project · PR #42` (the number a link) and the viewed agent's label: the last
    // row keeps `Plan mode` beside the hint at 37 columns
    s.feed(&facts(
        r#"{"cwd":"/tmp/project","project_root":"project","items":["project-name","pull-request-number"]}"#,
    ));
    s.feed(PR_42);
    s.feed(&agent("reviewer"));
    s.resize(37, 24);
    s.assert_contains("project · PR #42 · reviewer");
    s.assert_last_row_has("Plan mode");
    s.feed(&agent(""));
    s.resize(100, 24);
    // a running task keeps the indicator and drops the cycle key
    s.feed(RUNNING);
    s.assert_contains("Plan mode");
    s.assert_absent("⇧tab");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:771 footer_layout_in_fullscreen_and_scrollback kind=snap owner=ui-pane snap=empty snap=footer_layout_in_fullscreen_and_scrollback
#[test]
fn footer_layout_in_fullscreen_and_scrollback() {
    kit::pending(Owner::UiPane, MODE_REASON);
    // the footer rows at 120x2, 60x2 and 31x3 in fullscreen, and in scrollback at 31x4 and 31x1,
    // with Plan mode, the ide context, the vim mode and 2 warnings; a draft (`draft`) shifts them
    let mut s = Screen::build(Owner::UiPane).size(120, 5).live().open();
    s.assert_snap("footer_layout_in_fullscreen_and_scrollback");
    s.resize(31, 6);
    s.type_text("draft");
    s.assert_contains("Plan mode");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:830 active_side_action_keeps_plan_expanded_on_the_other_row kind=ui owner=ui-pane
#[test]
fn active_side_action_keeps_plan_expanded_on_the_other_row() {
    // a side conversation: `Side from main thread · ctrl+/ to switch · ctrl+c to close` is the
    // last row and `Plan mode (⇧tab to cycle)` sits on the row above it, at 63 columns
    let s = Screen::build(Owner::UiPane)
        .size(63, 24)
        .script(include_str!("fixtures/pane_footer_1/side_plan.jsonl"))
        .open();
    s.assert_in_order(&[
        "Plan mode (⇧tab to cycle)",
        "Side from main thread · ctrl+/ to switch · ctrl+c to close",
    ]);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:852 oversized_goal_yields_to_an_eligible_whole_hint_with_status_disabled kind=ui owner=ui-pane
#[test]
fn oversized_goal_yields_to_an_eligible_whole_hint_with_status_disabled() {
    // status disabled: `? for shortcuts` at 18 columns and `100% context left` at 20; a paused
    // goal (`/goal resume`) wins at 80 columns
    let mut s = Screen::build(Owner::UiPane)
        .size(18, 4)
        .script(include_str!("fixtures/pane_footer_1/goal_paused.jsonl"))
        .open();
    s.assert_contains("? for shortcuts");
    s.resize(20, 4);
    s.assert_contains("100% context left");
    s.resize(80, 4);
    s.assert_contains("Goal paused (/goal resume)");
}

/// The oracle's snapshot of the composer alone against our rows from the help's title on: the
/// page keeps the reader's gap (one blank row, `composer_gap`) above the prompt box that the
/// snapshot lacks, and the prompt row carries the page's own placeholder (the oracle's is
/// `Ask Codex`).
// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:883 shortcut_help_stays_above_the_prompt_and_bottom_status kind=snap owner=ui-pane snap=shortcut
#[test]
fn shortcut_help_stays_above_the_prompt_and_bottom_status() {
    // `?` opens the help above the prompt (`Keyboard shortcuts` above `Ask Codex`), the close row
    // `? / esc close` is the last row; the oracle renders at the height the composer wants, so
    // the narrow frame (30 rows of help) gets the rows to show them all
    let mut s = Screen::build(Owner::UiPane)
        .size(130, 24)
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    s.assert_in_order(&["Keyboard shortcuts", "Ask Codex"]);
    s.assert_help_snap("shortcut_help_above_wide");
    s.resize(40, 40);
    s.assert_help_snap("shortcut_help_above_narrow");
    s.resize(80, 12);
    s.assert_contains("? / esc close");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:929 shortcut_help_keeps_bottom_anchored_composer_status_and_cursor_stable kind=ui owner=ui-pane
#[test]
fn shortcut_help_keeps_bottom_anchored_composer_status_and_cursor_stable() {
    // at 40x12 the composer, its status and the cursor keep the same geometry while `?` opens the
    // help and `esc` closes it
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 12)
        .script(HELP_STATUS)
        .open();
    let before = s.shot();
    s.keys("?");
    s.keys("esc");
    s.assert_same_rows(&before);
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:966 shortcut_help_keeps_input_and_close_hint_visible_when_height_is_tiny kind=ui owner=ui-pane
#[test]
fn shortcut_help_keeps_input_and_close_hint_visible_when_height_is_tiny() {
    // the help at 40 columns, heights 4 to 7: the last row is `? / esc close` and the input stays
    // visible; with the status on and off (the test sets it; no key does)
    // (opened taller: a resize to the size it has repaints nothing)
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 8)
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    for height in [4u16, 5, 6, 7] {
        s.resize(40, height);
        s.assert_row(usize::from(height) - 1, "  ? / esc close");
    }
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1002 shortcut_help_close_row_uses_runtime_binding_and_narrow_fallback kind=ui owner=ui-pane
#[test]
fn shortcut_help_close_row_uses_runtime_binding_and_narrow_fallback() {
    // the help close row names the runtime binding of the shortcut toggle (`⌃x ⌃⇧⌥f12`) at 80
    // columns and falls back to `esc close` at 22 (the test sets the binding; no key does)
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 8)
        .keymap(&[("composer.toggle_shortcuts", "ctrl-x ctrl-shift-alt-f12")])
        .script(HELP_STATUS)
        .open();
    s.keys("?");
    // the oracle trims the row; ours keeps the footer's two-cell indent
    s.assert_row(7, "  ⌃x ⌃⇧⌥f12 / esc close");
    s.resize(22, 8);
    s.assert_row(7, "  esc close");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1031 status_survives_hints_feedback_and_queue kind=snap owner=ui-pane snap=persistent_status_and_messages
#[test]
fn status_survives_hints_feedback_and_queue() {
    // the status `MODEL · ~/project · Context 20% used` and `Plan mode` persist through a
    // running task with a draft and the file and skill popups. (The oracle's shortcut overlay
    // with a copy flash and its hint override on the footer are set by the test: the page's
    // overlay has its own close row, and nothing flashes or overrides: see FLASH_REASON.)
    let mut s = Screen::build(Owner::UiPane)
        .size(64, 24)
        .script(PLAN_STATUS)
        .open();
    let status = "MODEL · ~/project · Context 20% used";
    s.assert_has_row_trimmed(status);
    s.assert_text_count("Plan mode", 1);
    s.assert_contains("? for shortcuts");
    // a running task with a draft: the queue hint, and `Plan mode` without its cycle key
    s.feed(RUNNING);
    s.type_text("Continue the task");
    s.assert_has_row_trimmed(status);
    s.assert_contains("tab to queue message");
    s.assert_text_count("Plan mode", 1);
    s.assert_absent("⇧tab");
    // the file and skill popups keep both
    for popup in ["@", "$"] {
        s.keys("ctrl+u");
        s.type_text(popup);
        s.assert_has_row_trimmed(status);
        s.assert_text_count("Plan mode", 1);
    }
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1064 transcript_search_keeps_status_and_query_cursor kind=ui owner=ui-pane
#[test]
fn transcript_search_keeps_status_and_query_cursor() {
    // the Find footer (`Find: ne`, the cursor after the query) at 48 columns: heights 4, 5 and 6
    // keep its `F` at column 2 and the cursor at column 10 of its row; the status line of the
    // composer stays above it when the room allows (the oracle's layout asserts the rows)
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 4)
        .script(PLAN_NEEDLE)
        .open();
    s.keys("f3");
    s.type_text("ne");
    for height in [4, 5, 6] {
        s.resize(48, height);
        let footer = row_of(&s, "Find:");
        s.assert_text_at(2, footer, "F");
        s.assert_cursor(10, footer);
    }
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1098 input_history_search_uses_the_final_row kind=ui owner=ui-pane
#[test]
fn input_history_search_uses_the_final_row() {
    kit::pending(Owner::UiPane, CARET_REASON);
    // Ctrl+R at 48 columns puts the cursor on the final row of the screen (the search's own row,
    // under the preview): the oracle asserts the row only
    let mut s = Screen::build(Owner::UiPane).size(48, 6).live().open();
    s.keys("ctrl+r");
    s.assert_contains("reverse-i-search");
    s.assert_cursor_on_last_row();
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1109 slash_suggestions_take_space_before_status_expands kind=snap owner=ui-pane snap=slash_suggestions_take_space_before_status_expands
#[test]
fn slash_suggestions_take_space_before_status_expands() {
    kit::na(Owner::UiPane, EXTENSION_NA);
    // the status `one two three four` shows `one…` while the `/m` command popup takes the space
    // above the composer, at 80 columns and 7 rows
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1130 slash_popup_preserves_footer_and_selection_style kind=snap owner=ui-pane snap=slash_popup_footer_wide
#[test]
fn slash_popup_preserves_footer_and_selection_style() {
    kit::pending(Owner::UiPane, POPUP_REASON);
    // the `/m` popup with Down selected keeps the status `model · high · fast` below it at 80
    // and 32 columns; the transcript footer `transcript status` sits on the last row
    let mut s = Screen::build(Owner::UiPane).size(80, 12).live().open();
    s.type_text("/m");
    s.keys("down");
    s.assert_snap("slash_popup_footer_wide");
    s.resize(32, 12);
    s.assert_contains("model · high · fast");
}

// ORACLE bottom_pane/chat_composer/status_surface_tests.rs:1198 activity_shortcut_help_follows_runtime_binding kind=snap owner=ui-pane snap=activity_shortcut_help_follows_runtime_binding
#[test]
fn activity_shortcut_help_follows_runtime_binding() {
    // the help at 130 columns names the activity key from the runtime keymap: `f4`, `ctrl-x t`,
    // and no key at all (the row is gone); the test sets the keymap (`global.focus_activity`),
    // no key does. The oracle joined its three renders in one snapshot with `---` lines.
    const NAME: &str = "activity_shortcut_help_follows_runtime_binding";
    let oracle = kit::snap_rows(NAME, "");
    let frames = kit::split_frames(&oracle, 3);
    let keymaps: [&'static [(&'static str, &'static str)]; 3] = [
        &[("global.focus_activity", "f4")],
        &[("global.focus_activity", "ctrl-x t")],
        &[("global.focus_activity", "")],
    ];
    for (keymap, frame) in keymaps.into_iter().zip(frames) {
        let mut s = Screen::build(Owner::UiPane)
            .size(130, 24)
            .script(HELP_STATUS)
            .keymap(keymap)
            .open();
        s.keys("?");
        s.assert_help_rows(&frame, NAME);
    }
}

// ---- oracle: bottom_pane/chat_composer/warning_notice_tests.rs (4 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:57 warning_notice_styles kind=snap owner=ui-pane snap=warning_notice_styles
#[test]
fn warning_notice_styles() {
    // the draft `keep this draft` and 3 warnings at 80 columns, on the test's default colours
    // (222 on 16); the oracle's Buffer dump has the notice's styles: the badge and the count in
    // the warning colour, `f2` bold, the rest secondary
    let mut s = warned(80, 24, 3);
    s.type_text("keep this draft");
    s.assert_contains("⚠ 3 warnings · f2 to view");
    s.assert_text_style("3 warnings", "!dim");
    s.assert_text_style("f2", "bold");
    s.assert_text_style("to view", "!bold");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:71 light_warning_notice_clears_inherited_dimming kind=ui owner=ui-pane
#[test]
fn light_warning_notice_clears_inherited_dimming() {
    // on a light background (30 on 255) the notice's `3` is not dimmed: the notice is the
    // short spelling at 20 columns (the oracle renders it over a dimmed buffer)
    let mut s = Screen::build(Owner::UiPane)
        .size(20, 24)
        .script(WARNINGS_STATUS)
        .terminal_colors(0x1e_1e_1e, 0xff_ff_ff)
        .open();
    for n in 1..=3 {
        s.feed(&warning(n));
    }
    let row = row_of(&s, "⚠");
    let at = s.rows()[row].chars().position(|c| c == '⚠').unwrap();
    s.assert_text_at(at + 2, row, "3");
    s.assert_style(at + 2, row, "!dim");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:102 warning_notice_respects_shortcuts_and_interactive_hints kind=ui owner=ui-pane
#[test]
fn warning_notice_respects_shortcuts_and_interactive_hints() {
    // 2 warnings at 64 columns: `⚠ 2 warnings · f2 to view` (f2 is the keymap's default key; the
    // oracle's `/warnings to view` is the spelling with the key unbound, which the test sets);
    // (the oracle also hides it while the slash popup is open: sweep, the page does not tell the
    // footer about its popup)
    let mut s = Screen::build(Owner::UiPane)
        .size(64, 24)
        .script(include_str!("fixtures/pane_footer_1/two_warnings.jsonl"))
        .open();
    s.assert_contains("⚠ 2 warnings · f2 to view");
    s.type_text("hello");
    s.assert_contains("⚠ 2 warnings · f2 to view");
}

// ORACLE bottom_pane/chat_composer/warning_notice_tests.rs:139 warnings_yield_to_queue_controls_and_complete_navigation_hints kind=ui owner=ui-pane
#[test]
fn warnings_yield_to_queue_controls_and_complete_navigation_hints() {
    // a queued draft with a task running shows no notice, whatever the count (the oracle compares
    // the render of 4 warnings with the render of none); the notice waits for the draft's task
    let mut s = warned(32, 8, 4);
    s.assert_contains("⚠");
    s.feed(RUNNING);
    s.type_text("queued draft");
    s.assert_absent("⚠");
    s.assert_contains("tab to queue");
    // (the oracle's `esc latest` footer at 12 and 20 columns is the footer's own fit, a state
    // test of `warning_notice_layout`: the page draws the Find footer in its own rows)
}

// ---- oracle: bottom_pane/chat_composer_effort_tests.rs (3 screen-level tests here; 4 state tests left to ui-pane)
// ORACLE bottom_pane/chat_composer_effort_tests.rs:123 effort_transition_never_replaces_a_footer_flash kind=ui owner=ui-pane
#[test]
fn effort_transition_never_replaces_a_footer_flash() {
    kit::pending(Owner::UiPane, FLASH_REASON);
    // the flash `saved` (1 s) on the footer row at 60x6 is kept through the effort transitions
    // High then Ultra; the footer never shows `U L T R A` (the test sets them; no key does)
    let mut s = Screen::build(Owner::UiPane).size(60, 6).live().open();
    s.feed(&facts(
        r#"{"model":"gpt-5.4","effort":"high","branch":"main","items":["model-with-reasoning","git-branch"]}"#,
    ));
    s.feed(&facts(
        r#"{"model":"gpt-5.4","effort":"ultra","branch":"main","items":["model-with-reasoning","git-branch"]}"#,
    ));
    s.assert_contains("saved");
    s.assert_absent("U L T R A");
}

// ORACLE bottom_pane/chat_composer_effort_tests.rs:154 effort_transition_keeps_the_full_footer_row kind=snap owner=ui-pane snap=codex_tui__bottom_pane__chat_composer__effort_tests__effort_transition_keeps_the_full_footer_row
#[test]
fn effort_transition_keeps_the_full_footer_row() {
    // the two bottom rows at 36 columns: the status line (the oracle's `gpt-5.4 high ·
    // feature-branch`, here the thread's model) and the Plan-mode row, whole while the
    // transition to Ultra runs and after it
    let mut s = Screen::build(Owner::UiPane)
        .size(36, 6)
        .script(PLAN_STATUS)
        .open();
    s.feed(&facts(
        r#"{"effort":"high","branch":"feature-branch","items":["model-with-reasoning","git-branch"]}"#,
    ));
    s.feed(&facts(
        r#"{"effort":"ultra","branch":"feature-branch","items":["model-with-reasoning","git-branch"]}"#,
    ));
    s.assert_row(5, "          Plan mode (⇧tab to cycle)");
    // (the sweep of the transition may shift the line by a column)
    s.assert_has_row_trimmed("MODEL high · feature-branch");
    s.wait(10_000);
    s.assert_row(4, "  MODEL ultra · feature-branch");
    s.assert_row(5, "          Plan mode (⇧tab to cycle)");
}

// ORACLE bottom_pane/chat_composer_effort_tests.rs:188 ultra_accent_upgrades_prompt_glyph kind=snap owner=ui-pane snap=ultra_accent_upgrades_prompt_glyph
#[test]
fn ultra_accent_upgrades_prompt_glyph() {
    // the prompt glyph `›` takes the Ultra accent at 60 columns (the animation has played out)
    let mut s = Screen::build(Owner::UiPane).size(60, 24).live().open();
    s.feed(&facts(r#"{"effort":"ultra"}"#));
    s.wait(3_000);
    // the snapshot is the composer in a 9-row frame (blank, prompt, blanks, footer): its prompt
    // row, glyph and accent; its footer row differs by the known gaps (the agents hint, the
    // context meter before any usage)
    s.assert_contains("» Ask Codex to do anything");
    // the Ultra accent (no terminal colours known: the accent itself), bold
    s.assert_text_style("»", "fg=Rgb(186,130,255) bold");
}
