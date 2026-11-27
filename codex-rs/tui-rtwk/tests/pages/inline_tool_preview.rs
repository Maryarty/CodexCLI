// test root: pages
use codex_tui_core::app::{AppEv, BackendEv};
use rtwk::binding::ScreenMode;
use rtwk_test::Options;

#[test]
fn settled_tool_rows_do_not_wake_inline_or_fullscreen() {
    let tool = serde_json::json!({
        "type":"mcpToolCall","id":"tool-1","server":"node_repl","tool":"js",
        "status":"completed","arguments":{"title":"Inspect screenshot","code":"showImage()"},
        "appContext":null,"mcpAppUi":null,"pluginId":null,"readOnlyHint":null,
        "result":{"content":[{"type":"text","text":"Screenshot captured"}],
                  "structuredContent":null,"_meta":null,"isError":false},
        "error":null,"durationMs":5
    });
    let turn = |running: bool| {
        serde_json::json!({
            "id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full",
            "status": if running { "inProgress" } else { "completed" },
            "error":null,"startedAt":null,"completedAt":null,"durationMs":null
        })
    };
    let notice = |method: &str, params: serde_json::Value| {
        AppEv::Backend(BackendEv::Notification(Box::new(
            serde_json::from_value(serde_json::json!({"method":method,"params":params}))
                .expect("known app-server notification"),
        )))
    };

    for (mode, title) in [
        (ScreenMode::Inline, "• Inspect screenshot"),
        (ScreenMode::Fullscreen, "• Called Inspect screenshot"),
    ] {
        let mut h = crate::common::start_driven_with(
            false,
            Options::new()
                .mode(mode)
                .size(80, 24)
                .fail_on_overflow(false),
        );
        h.connect();
        h.quiesce();
        h.inject([
            notice(
                "turn/started",
                serde_json::json!({"threadId":"thread-1","turn":turn(true)}),
            ),
            notice(
                "item/completed",
                serde_json::json!({"threadId":"thread-1","turnId":"turn-1",
                                   "item":tool,"completedAtMs":0}),
            ),
            notice(
                "turn/completed",
                serde_json::json!({"threadId":"thread-1","turn":turn(false)}),
            ),
        ]);
        h.advance(1_300);
        let shown = h.t.screen();
        assert!(shown.contains(title), "{mode:?}: {shown}");
        assert!(shown.contains("Screenshot captured"), "{mode:?}: {shown}");
        let before = h.t.frame_loop();
        let (runs, presented) = (before.frames_run(), before.stats().frames);
        h.t.pump_for(crate::common::ms(3_200));
        let after = h.t.frame_loop();
        assert_eq!(
            (after.frames_run() - runs, after.stats().frames - presented),
            (0, 0),
            "{mode:?}: the completed tool row is still visible"
        );
    }
}
