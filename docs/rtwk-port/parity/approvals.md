# approvals — parity (S13), in progress: family A of three

Oracle `bottom_pane/approval_overlay.rs`, `mcp_server_elicitation.rs`, `request_user_input/`.
View `src/sheets/approvals/`, core `core/src/screens/approvals.rs`,
`core/src/protocol/requests.rs`. Sheets over the SessionPage (architecture.md §1
Amendment 2026-11-19b), opened by the app-level request presenter for the head of
`CodexApp.requests`; an answer is `ScreenIntent::Approvals(Intent::Answer)` through the
app's effect path, the sheet never sends an RPC (ADR-003).

## Family A done (exec, permissions, apply patch), commit 460306556
- Core: `ExecPrompt`/`PatchPrompt`/`PermissionsPrompt`/`ElicitationPrompt` carry
  `thread_label`; `ExecPrompt.permission_rule`, `.input`; `PatchPrompt.destinations`;
  `Decision::NetworkRule {host, allow}` (wire `applyNetworkPolicyAmendment`);
  `thread_label(threads, thread, active)` = the agent picker's name (`nickname [role]`,
  `Agent (<last id group>)`); default decisions as `default_available_decisions`.
- View: `present::body` returns pieces (`Tone`, `Line::Command` highlighted with
  rtwk-rich `Highlighter`, bold keys, dim words, ` or o to open thread`); `dialog.rs`
  binds Enter/digits/shortcut keys/Esc/`o` with `Focus`, rows are `Options::view_rich`,
  a tap moves the cursor and a tap on the cursor row answers (ADR-011).
- `core/src/app.rs` (outside the area, additive, shared with startup-device-code): the queue
  push sets the thread label; one arm for `ScreenEv::Approvals(Presented)` so a seeded
  fixture replaces the queued request's prompt (a fixture carries the label and the patch
  destinations an app-server request cannot).
- Ten debug entries with fixtures (`exec_request`, `exec_url_reason`, `exec_cross_thread`,
  `exec_snippet`, `exec_additional_permissions`, `exec_network`, `exec_empty_command`,
  `exec_zsh_git_add`, `patch_no_changes`, `patch_thread_label`).

## Shard counts (pass / n/a / pending / red), `summary.py --run` of the four shards
- approvals_1: 8 / 17 / 33 / 0 of 58
- approvals_2: 4 / 14 / 49 / 0 of 67
- chat_approvals_1: 2 / 5 / 47 / 0 of 54
- chat_approvals_2: 0 / 18 / 30 / 0 of 48
- total 14 pass (7 at the start), 54 n/a, 159 pending, 0 red, 0 stale.

## Tests of this branch
- `core/tests/approvals.rs`: network allow rule, additional permissions, agent-picker label,
  a seeded label reaches the queue's prompt.
- `tests/sheets/approvals.rs`: network rows, wrapped URL rows, the `o` hint, a click moves
  and a second click answers (shown biting: the answer handler swapped for a no-op), a digit.
- `tests/oracle/approvals_1.rs`: eight un-pended; the sizes of the entry tests are the
  oracle buffers' heights (the sheet is bottom-aligned over the debug menu), the URL test
  compares the header region.

## Pending and why (family A)
- `approval_headers_preserve_wrapped_url_destinations`: **G72** (rtwk `Text` of a plain
  string has no autolink; rows equal the oracle's, pinned in the sheet test).
- `network_exec_prompt_title_includes_host`: the kit's `oracle_snap::parse_row` returns a
  Buffer row holding an unescaped quote (`"example.com"`) as raw text; the rows equal the
  snapshot (sheet test `network_exec_prompt_rows`). Kit fix: let a row end at the last
  quote of the line (not mine to edit).
- Decision cells (`✔ You approved ...`: `exec_history_cell_*`,
  `network_access_command_history_*`, chat_approvals_1 history rows): a transcript row
  that the thread records when the answer goes out (core ThreadModel items and the cells
  are chat-cells-2's), and an approvals entry draws the sheet over the debug menu.
- `.keymap(..)` tests: the kit's keymap seed does not exist.

## Run in the binary
`check.py --bin rtwk --bin oracle --only exec-approval --capture-dir /tmp/approvals-cap`:
PASS on both (`sent y at an approval prompt; sentinel on screen: yes`, 2 model requests).
The scenario's checkpoints (ready, turn-1-working, turn-1) do not hold the prompt, so no
row diff of the prompt is in this pack; the rows are proved by the snapshot tests above.

## Left (families B, C and the rest, see the tm checkpoint)
MCP elicitation forms (the oracle overlay draws on a menu surface, fills its area and has
`MIN_OVERLAY_HEIGHT` 8: its snapshots are the 16-row stretched layout), request_user_input,
async questions, guardian, user verification, pending thread approvals, ctrl+a diff view.
