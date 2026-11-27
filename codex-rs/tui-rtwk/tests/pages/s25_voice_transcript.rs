// test root: pages
//! Canonical voice captions alongside ordinary chat on the native terminal screen.

use crate::kit::{self, Owner, Screen};
use serde_json::json;

#[test]
fn live_and_final_voice_captions_can_share_wide_and_narrow_chat_with_a_stream() {
    for columns in [80, 32] {
        let mut s = Screen::build(Owner::UiPane).size(columns, 26).live().open();
        s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"animations":false}}}"#);
        s.voice_available().type_text("/voice").keys("enter");
        let session = s.connect_voice();
        s.feed(r#"{"step":"notify","message":{"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"ordinary","delta":"Agent answer arriving"}}}"#);
        s.wait_for("Agent answer arriving");

        s.voice_caption(false, &session, "speech-1", "user", "");
        s.voice_delta("speech-1", "pick a number");
        s.assert_absent("pick a number");
        s.voice_caption(false, &session, "speech-2", "assistant", "");
        s.voice_delta("speech-2", "thinking aloud");
        s.wait_for("thinking aloud");
        s.assert_contains("Agent answer arriving");

        s.voice_caption(true, &session, "speech-1", "user", "pick a number");
        s.voice_caption(true, &session, "speech-2", "assistant", "spoken answer");
        s.voice_caption(true, &session, "speech-2", "assistant", "spoken answer");
        s.voice_delta("speech-2", "stale delta");
        s.wait_for("› pick a number");
        assert_eq!(s.text().matches("spoken answer").count(), 1, "{}", s.text());
        s.assert_contains("Agent answer arriving");
        s.assert_absent("thinking aloud");
        s.assert_absent("stale delta");
    }
}

#[test]
fn replay_positions_captions_around_ordinary_rows_and_deduplicates_by_id() {
    let first = json!({"type": "agentMessage", "id": "first", "text": "Earlier ordinary answer"});
    let last = json!({"type": "agentMessage", "id": "last", "text": "Later ordinary answer"});
    let turn = kit::replay_turn("turn-1", json!([first, last]));
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .replay(vec![turn.clone()])
        .replay_paginated()
        .open();
    s.history_page(vec![turn], None);
    let caption = json!({
        "type":"realtime", "position": 3,
        "item": {"id":"speech", "realtimeSessionId":"call-1", "type":"transcriptSegment",
                 "role": "user", "text": "remember this"}
    });
    s.voice_timeline_page(
        None,
        json!([
            caption,
            {"type":"item","position":4,"turnId":"turn-1","item":last},
        ]),
        Some("voice-older"),
    );
    s.voice_timeline_page(
        Some("voice-older"),
        json!([
            {"type":"turnStarted","position":1,"turnId":"turn-1","startedAt":null},
            {"type":"item","position":2,"turnId":"turn-1","item":first},
            caption,
        ]),
        None,
    );
    s.wait_for("› remember this");
    s.assert_in_order(&[
        "Earlier ordinary answer",
        "› remember this",
        "Later ordinary answer",
    ]);
    assert_eq!(s.text().matches("remember this").count(), 1, "{}", s.text());
}
