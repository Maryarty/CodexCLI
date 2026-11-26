// test root: pages
//! Browser activation and its interaction with the transcript's pointer gestures.

use rtwk_core::input::{Mods, MouseKind};

use crate::kit::{Owner, Screen};

const LINKS: &str = include_str!("../oracle/fixtures/transcript_view_2/clickable_web.jsonl");
const DOCS: &str = "https://example.com/docs";

fn transcript() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(72, 16)
        .script(LINKS)
        .view_only()
        .open()
}

#[test]
fn a_control_link_click_opens_on_release() {
    let mut s = transcript();
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.mouse_event(MouseKind::Down, 0, 3, y, Mods::CTRL);
    s.assert_opened_urls(&[]);
    s.mouse_event(MouseKind::Up, 0, 3, y, Mods::CTRL);
    s.assert_opened_urls(&[DOCS]);
}

#[test]
fn an_existing_mouse_range_does_not_block_a_fresh_link_click() {
    let mut s = transcript();
    s.selection_copy_policy(false, false);
    let url = u16::try_from(s.row_of("https://example.org/plain")).expect("screen row");
    let docs = u16::try_from(s.row_of("docs")).expect("screen row");
    s.drag(2, url, 6, url);
    s.assert_text_style("https", "reversed");
    s.assert_opened_urls(&[]);
    s.click(3, docs);
    s.assert_opened_urls(&[DOCS]);
}

#[test]
fn a_wheel_at_the_scroll_limit_cancels_a_pending_link_press() {
    let mut s = transcript();
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    let before = s.shot();
    s.mouse_down(3, y);
    s.wheel_up(3, y, 1);
    s.assert_same_rows(&before);
    s.mouse_up(3, y);
    s.assert_opened_urls(&[]);
    s.wait(350);
    s.click(3, y);
    s.assert_opened_urls(&[DOCS]);
}

#[test]
fn changing_a_link_destination_cancels_a_press_held_on_its_label() {
    const STREAM: &str = r#"
{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"itemsView":"full","status":"inProgress"}}}}
{"step":"notify","message":{"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"msg-1-0","delta":"[docs](https://example.com/docs)"}}}
"#;
    const REPLACEMENT: &str = r#"{"step":"notify","message":{"method":"item/completed","params":{"item":{"type":"agentMessage","id":"msg-1-0","text":"[docs](https://example.org/new)","phase":null,"memoryCitation":null,"delivery":null,"questions":null,"moderation":null},"threadId":"thread-1","turnId":"turn-1","completedAtMs":0}}}"#;
    const DESTINATION: &str = "https://example.org/new";
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 16)
        .script(STREAM)
        .view_only()
        .open();
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.assert_link("docs", DOCS);
    s.mouse_down(3, y);
    s.feed(REPLACEMENT);
    assert_eq!(s.row_of("docs"), usize::from(y));
    s.assert_link("docs", DESTINATION);
    s.mouse_up(3, y);
    s.assert_opened_urls(&[]);
    s.wait(350);
    s.click(3, y);
    s.assert_opened_urls(&[DESTINATION]);
}

#[test]
fn only_web_links_open_and_an_alt_click_does_not_activate() {
    let mut s = transcript();
    for label in ["mail", "local"] {
        let y = u16::try_from(s.row_of(label)).expect("screen row");
        s.click(3, y);
    }
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.click_with(3, y, Mods::ALT);
    s.assert_opened_urls(&[]);
    s.wait(350);
    s.click(3, y);
    s.assert_opened_urls(&[DOCS]);
}

#[test]
fn the_copy_mode_owns_clicks_over_link_text() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 18)
        .script(LINKS)
        .owned_transcript()
        .open();
    s.type_text("/copy").keys("enter");
    s.assert_contains("enter copy");
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.click(3, y);
    s.ctrl_click(3, y);
    s.assert_opened_urls(&[]);
}

#[test]
fn a_waiting_approval_prevents_link_activation_in_the_thread() {
    const APPROVAL: &str = r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","itemId":"call-1","approvalId":"call-1","environmentId":null,"command":"echo hello","cwd":"/tmp","reason":"need confirmation","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 20)
        .script(LINKS)
        .open();
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.feed(APPROVAL);
    s.assert_contains("need confirmation");
    s.click(3, y);
    s.ctrl_click(3, y);
    s.assert_opened_urls(&[]);
}

#[test]
fn a_dialog_opened_after_press_prevents_link_activation() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 20)
        .script(LINKS)
        .open();
    let y = u16::try_from(s.row_of("docs")).expect("screen row");
    s.mouse_down(3, y);
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Update Model Permissions");
    s.mouse_up(3, y);
    s.assert_opened_urls(&[]);
    s.keys("escape").wait(350);
    s.click(3, u16::try_from(s.row_of("docs")).expect("screen row"));
    s.assert_opened_urls(&[DOCS]);
}
