// test root: pages
//! The composer's popup (`/` commands, `$` skills, `@` mentions): Up and Down,
//! the wheel and the pointer move the highlight, Enter and a click pick the
//! row (a command runs, a mention goes in as text), Tab completes the
//! highlighted row as text, Esc closes it until the text changes.
//! Oracle: `chat_composer.rs` `handle_key_event_with_slash_popup`; the mouse
//! is ADR-011's.

use crate::kit::{Owner, Screen};

fn chat() -> Screen {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.wait_for("› Ask Codex to do anything");
    s
}

fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›'))
        .unwrap_or(usize::MAX)
}

/// The `/status` command ran: its card is on the screen.
fn status_ran(s: &Screen) -> bool {
    s.rows().iter().any(|r| r.contains("OpenAI Codex"))
}

/// The popup is open: its rows carry the highlight bar.
fn popup_open(s: &Screen) -> bool {
    s.rows().iter().any(|r| r.contains('▎'))
}

/// Where `text` first shows on the screen: (column, row).
fn find(s: &Screen, text: &str) -> (u16, u16) {
    let rows = s.rows();
    let row = rows
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or_else(|| panic!("`{text}` is not on the screen:\n{}", rows.join("\n")));
    let col = rows[row]
        .find(text)
        .map_or(0, |at| rows[row][..at].chars().count());
    (
        u16::try_from(col).unwrap_or(0),
        u16::try_from(row).unwrap_or(0),
    )
}

fn skill() -> Screen {
    Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "../oracle/fixtures/composer_popups/skills_rustdoc.jsonl"
        ))
        .open()
}

#[test]
fn tab_completes_the_command_as_text_and_runs_nothing() {
    let mut s = chat();
    s.type_text("/stat");
    assert!(popup_open(&s));
    s.keys("tab");
    let p = prompt(&s);
    s.assert_row(p, "› /status");
    // the caret is after the completed `/status ` (2 + 8 cells)
    s.assert_cursor(10, p);
    assert!(!status_ran(&s), "{}", s.rows().join("\n"));
    assert!(!popup_open(&s));
}

#[test]
fn enter_runs_the_highlighted_command() {
    let mut s = chat();
    s.type_text("/stat");
    s.keys("enter");
    assert!(status_ran(&s), "{}", s.rows().join("\n"));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

#[test]
fn a_click_on_a_command_row_runs_it() {
    let mut s = chat();
    s.type_text("/stat");
    let (x, y) = find(&s, "/status");
    s.click(x, y);
    assert!(status_ran(&s), "{}", s.rows().join("\n"));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

#[test]
fn down_and_up_move_the_highlight_and_wrap() {
    let mut s = chat();
    s.type_text("/");
    s.keys("down tab");
    s.assert_row(prompt(&s), "› /presets");
    let mut s = chat();
    s.type_text("/");
    s.keys("ctrl+n ctrl+n ctrl+p tab");
    s.assert_row(prompt(&s), "› /presets");
    // Up from the first row goes round to the last
    let mut s = chat();
    s.type_text("/");
    s.keys("up tab");
    let row = s.rows()[prompt(&s)].clone();
    assert!(row.starts_with("› /") && row != "› /model", "{row}");
}

#[test]
fn the_wheel_moves_the_highlight_over_the_popup() {
    let mut s = chat();
    s.type_text("/");
    let (x, y) = find(&s, "/presets");
    s.wheel_down(x, y, 1);
    s.keys("tab");
    let row = s.rows()[prompt(&s)].clone();
    assert!(row.starts_with("› /") && row != "› /model", "{row}");
}

#[test]
fn escape_closes_the_popup_until_the_text_changes() {
    let mut s = chat();
    s.type_text("/stat");
    s.keys("esc");
    assert!(!popup_open(&s));
    s.assert_row(prompt(&s), "› /stat");
    // a changed text opens it again
    s.type_text("u");
    assert!(popup_open(&s), "the text changed: the popup is back");
}

#[test]
fn a_press_outside_the_popup_closes_it_and_keeps_the_draft() {
    let mut s = chat();
    s.type_text("/stat");
    s.click(0, 0);
    assert!(!popup_open(&s), "{}", s.rows().join("\n"));
    s.assert_row(prompt(&s), "› /stat");
}

#[test]
fn tab_on_a_mention_puts_it_in_as_enter_does() {
    let mut s = skill();
    s.type_text("$ru");
    s.keys("tab");
    s.assert_row_starts_with(prompt(&s), "› $rustdoc");
    assert!(!popup_open(&s));
}

#[test]
#[ignore = "pending(mount): MentionPopup is placed by session_page.rs (a Need of that file); the popup's rows are not on the page until the line is in"]
fn a_click_on_a_mention_row_puts_it_in_and_sends_nothing() {
    let mut s = skill();
    s.type_text("$ru");
    s.click_text("rustdoc");
    s.assert_row_starts_with(prompt(&s), "› $rustdoc");
    assert!(!popup_open(&s));
}

#[test]
fn up_and_down_are_the_drafts_again_once_the_popup_is_closed() {
    let mut s = chat();
    s.type_text("/stat");
    s.keys("esc up tab");
    // closed: Up is the field's (the caret moves), Tab completes nothing
    assert!(!popup_open(&s));
    s.assert_row(prompt(&s), "› /stat");
}

#[test]
fn tab_on_skills_runs_it_as_enter_does() {
    // oracle `selected_command_dispatches_immediately_on_tab`: the one command Tab runs
    let mut s = chat();
    s.type_text("/skills");
    s.keys("tab");
    assert!(
        s.rows().iter().any(|r| r.contains("settings (S15)")),
        "{}",
        s.rows().join("\n")
    );
    assert!(
        s.rows()
            .iter()
            .any(|r| r.starts_with("› Ask Codex to do anything")),
        "the draft is cleared:\n{}",
        s.rows().join("\n")
    );
}

#[test]
fn a_mention_after_an_image_keeps_the_image() {
    for pick in ["tab", "enter"] {
        let mut s = skill();
        let path = s.attach_image("mention.png");
        s.type_text(" see $ru");
        s.keys(pick);
        s.assert_row_starts_with(prompt(&s), "› [Image #1] see $rustdoc");
        s.keys("enter");
        s.wait_for("see $rustdoc");
        let turns = s.turns();
        assert_eq!(turns.len(), 1, "{pick}");
        assert_eq!(
            turns[0]
                .images
                .iter()
                .map(|i| i.path.clone())
                .collect::<Vec<_>>(),
            vec![path],
            "{pick}: the image goes with the turn"
        );
    }
}

#[test]
#[ignore = "pending(mount): MentionPopup is placed by session_page.rs (a Need of that file); the popup's rows are not on the page until the line is in"]
fn a_click_on_a_mention_row_after_an_image_keeps_the_image() {
    let mut s = skill();
    let path = s.attach_image("click.png");
    s.type_text(" see $ru");
    s.click_text("rustdoc");
    s.keys("enter");
    s.wait_for("see $rustdoc");
    let turns = s.turns();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].images[0].path, path);
}
