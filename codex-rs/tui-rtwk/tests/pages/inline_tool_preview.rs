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

#[test]
fn a_committed_command_keeps_its_output_and_the_timing_row_in_the_inline_history() {
    // The compact shell row reads the layout width through a `LayoutBuilder`, which the terminal
    // history cannot measure before its first layout: inline the room is known and the committed
    // row, the answer under it and the "Worked for" row all keep their height (the batch
    // overflowed by its last rows before).
    let notice = |method: &str, params: serde_json::Value| {
        AppEv::Backend(BackendEv::Notification(Box::new(
            serde_json::from_value(serde_json::json!({"method":method,"params":params}))
                .expect("known app-server notification"),
        )))
    };
    let turn = |running: bool| {
        serde_json::json!({
            "id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full",
            "status": if running { "inProgress" } else { "completed" },
            "error":null,"startedAt":null,"completedAt":null,
            "durationMs": if running { None } else { Some(125_000) }
        })
    };
    let item = |item: serde_json::Value| {
        notice(
            "item/completed",
            serde_json::json!({"threadId":"thread-1","turnId":"turn-1",
                               "item":item,"completedAtMs":0}),
        )
    };
    let mut h = crate::common::start_driven_with(
        false,
        Options::new().mode(ScreenMode::Inline).size(80, 24),
    );
    h.connect();
    h.quiesce();
    h.inject([
        notice(
            "turn/started",
            serde_json::json!({"threadId":"thread-1","turn":turn(true)}),
        ),
        item(serde_json::json!({
            "type":"commandExecution","id":"call-1","command":"bash -lc 'echo preparing'",
            "cwd":"/tmp/project","processId":null,"source":"agent","status":"completed",
            "commandActions":[{"type":"unknown","command":"echo preparing"}],
            "aggregatedOutput":"preparing\n","exitCode":0,"durationMs":null
        })),
        item(serde_json::json!({
            "type":"agentMessage","id":"msg-1","text":"Final response.","phase":"final_answer",
            "memoryCitation":null
        })),
        notice(
            "turn/completed",
            serde_json::json!({"threadId":"thread-1","turn":turn(false)}),
        ),
    ]);
    h.advance(1_300);
    let shown = h.t.screen();
    let order: Vec<usize> = [
        "• Ran echo preparing",
        "└ preparing",
        "• Final response.",
        "Worked for 2m 5s",
    ]
    .iter()
    .map(|row| shown.find(row).unwrap_or_else(|| panic!("{row}: {shown}")))
    .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{shown}");
}
