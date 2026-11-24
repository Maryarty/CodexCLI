# oracle-seeds: how much of a fixture an oracle test lets a script infer

`survey.py` measures, `gen.py` writes fixtures from a recorder log of the oracle's own run.
Result of the survey (`survey-2026-11-24.txt`, rerun to refresh): the share is small.

- 2599 oracle tests, 1016 pending (59 are bare stubs), 548 tests use a fixture file.
- Pending by blocker (first match on the reason): other 399, feature missing 327, no debug
  entry 140, kit cannot set the state 82, keymap 49, **fixture/script missing 19**.
- The 19 fixture-class tests are not fixture-only either: each still names a missing screen,
  cell or core step (`no script shape` = a core `ThreadEv` the chat does not apply). A fixture
  un-pends none of them. The 276 reasons that merely name a seed/entry are 80% a feature or a
  kit state (queued messages, a disconnect, an image file), not server messages.
- Oracle form of the pending bodies: 33 build `ServerNotification` literals, ~590 drive a chat
  widget through helpers (`handle_turn_started`, `begin_exec`, ...), ~190 build a history cell
  directly, ~200 build a view directly (bottom pane, picker, transcript view): nothing a
  fixture holds. Of the 548 existing fixture-using tests only 9 have `ServerNotification`
  literals in the oracle body; the hand-written fixtures are semantic translations (helpers,
  `screen` / `fact` steps), which a script cannot infer from the body without interpreting Rust.
- mock-model's `events.py` builds the model's Responses API events, not app-server
  notifications, so it is not a source for these fixtures.

So: no un-pends from this task (oracle pass unchanged). Stopped at what is inferable.

## What is exact: record the oracle's own run

~200 `handle_server_notification` calls in the oracle's chat tests (plus helpers that call it)
carry the exact `ServerNotification` values. A recorder lets `gen.py` write those as `notify`
steps. It needs a `cfg(test)` hook in `codex-rs/tui/src/chatwidget/protocol.rs` (orchestrator
owned, not applied here):

```rust
// first statement of handle_server_notification (and the same in handle_server_request,
// with "request": true)
#[cfg(test)]
if let Ok(path) = std::env::var("ORACLE_RECORD") {
    use std::io::Write;
    let test = std::thread::current().name().unwrap_or("").rsplit("::").next().unwrap_or("").to_string();
    let line = serde_json::json!({"test": test, "replay": replay_kind.map(|k| format!("{k:?}")), "message": &notification});
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(path)
        .and_then(|mut f| writeln!(f, "{line}"));
}
```

Run: `ORACLE_RECORD=/tmp/oracle.log cargo test -p codex-tui -- chatwidget` (one process per
test under nextest, appending is safe), then
`python3 -I tools/rtwk-port/oracle-seeds/gen.py /tmp/oracle.log`.
`gen.py` never overwrites a fixture (a `.gen.jsonl` lands beside it for a diff), skips tests
with a replayed notification (the script has no replay step), and `--selftest` regenerates a
hand-written fixture from a synthetic log.

Use it for a W2 package that builds a chat feature: record, generate, diff against the hand
fixture, then edit the `screen`/`fact` steps by hand.

## Files
- `survey.py [--list CLASS]`: the table above; `--list fixture` the 19 tests.
- `gen.py LOG [--out DIR] [--force]`, `gen.py --selftest`.
