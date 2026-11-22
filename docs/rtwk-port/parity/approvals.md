# approvals — parity (S13)

Oracle `bottom_pane/approval_overlay.rs`, `mcp_server_elicitation.rs`, `request_user_input/`,
`user_verification.rs`, `pending_thread_approvals.rs`. View `src/sheets/approvals/`, core
`core/src/screens/approvals.rs` (+ `approvals/form.rs`, `approvals/input.rs`),
`core/src/protocol/requests.rs`. Sheets over the SessionPage (architecture.md §1 Amendment
2026-11-19b), opened by the app-level request presenter for the head of `CodexApp.requests`; an
answer is `ScreenIntent::Approvals(Intent::Answer | Respond)` through the app's effect path, the
sheet never sends an RPC and holds no thread state (ADR-003).

## Built
- **A, exec / permissions / apply patch**: `present::body` (pieces, highlighted command through
  rtwk-rich `Highlighter`, bold keys, thread label and the `o` key) drawn by `dialog.rs` on
  `Options::view_rich`; a tap moves the cursor, a tap on the cursor row answers (ADR-011).
- **B, MCP elicitation form** (`form.rs`, core `approvals/form.rs`): fields from the schema, the
  approval fallback (tool approval with persist choices, parameter summary), digits, field keys
  (ctrl+p / ctrl+n, PageUp / PageDown, h/l on selects), typed text on a `TextArea` that takes the
  keys after the frame, the length error, the answer's `content` and `_meta`.
- **C, request_user_input** (`user_input.rs`, core `approvals/input.rs`): questions on `Options`
  and a notes `TextArea` (Backspace on empty notes closes them, Tab, shift+enter, ctrl+h/l/j/k
  list keys), the tips line, the unanswered confirmation (Proceed / Go back), a tap moves and a
  second tap answers; a secret question masks its answer with `*` (`TextField.obscure_text`).
- **User verification** (`present::body` + `dialog.rs`, core `UserVerificationPrompt`): the prompt
  (title, thread, server, description, `Verify and approve (y)` / `Cancel this request (c)`) and
  the waiting view (flat, no list: only Esc and `c` answer). `Decision::Verify` is not an
  answer: `answer_of` sends nothing for it, nor for a plain Accept/Decline of a verification
  (the accepted response carries the device's proof, which only the controller has).
- **Pending-thread banner** (`banner.rs` `PendingThreadApprovals`, core `pending_threads`): the
  `! Approval needed in <thread>` rows, `...`, `/subagents to review requests`.
- Debug entries with fixtures for every kind: exec / patch / permissions, 12 `mcp_*`, 10
  `user_input*`, 3 `user_verification*`. Entry rows fit the menu's width (the skeleton test finds
  them by text).

## Shard counts (pass / n/a / pending / red), `summary.py --run` of the four shards
- approvals_1: 15 / 17 / 26 / 0 of 58
- approvals_2: 31 / 14 / 22 / 0 of 67
- chat_approvals_1: 2 / 5 / 47 / 0 of 54
- chat_approvals_2: 0 / 18 / 30 / 0 of 48
- total 48 pass (7 at the start), 54 n/a, 125 pending, 0 red. Probe run: two stale, both stubs
  whose bodies pass vacuously (`user_message_during_user_shell_command_is_queued_not_steered`,
  `ordinary_follow_up_clears_unanswered_questions_after_accepted_input`: "needs feed" comments),
  their pending stays.
- `shards.py --check`: 0 files stale. Own tests: `core/tests/approvals.rs` 33,
  `tests/sheets/approvals.rs` 40 (taps shown biting; the secret mask shown biting).
- Whole nextest: 1445 run, 1444 pass; the one red is `rules::r14_widget_state_fields_hold_no_screen_model_types`
  on `src/pages/agents/mod.rs:131 mode: Mode` (not this slice's, red on the tip). The
  skeleton and startup-gate tests were red until the entry rows were shortened to the menu's width.

## Pending and why
- G72 (rtwk `Text` of a plain string has no autolink; also `TextWrap::Word` breaks a URL after a
  hyphen where the oracle cuts at the width): the wrapped URL tests (`approval_headers_*`,
  `user_verification_long_urls`, question URL).
- G49 (no break after a slash) and G39b (a column measured over the rows in view):
  `hidden_options_footer`, `long_option_text`, the wrapped URL form.
- Kit: no keymap seed (`.keymap`), no kitty bytes for shift+enter, a Buffer row with an
  unescaped quote cannot be parsed, frame headers (`Full height:` / `Clipped:`) are not read.
- Not built, owners elsewhere: decision history cells (`✔ You approved ...`: chat-cells-2), the
  clipped header `[… N lines] ⌃a view all` and the full-screen view (a header cut to the pane,
  ctrl+a opening a pager over the request: needs the pager task's page), the async-questions
  editor and its auto-resolution countdown (`autoResolutionMs` is read by nothing), typed-ahead
  discard when a request arrives (G8), vim mode in the question prompt.
- approvals_1 `user_input_*` tests (long_prompt, markdown, linked_option, history, history_recall,
  twelve, wrap_boundary, named_other, other_clipped, wrapped_other, wrapped_options, other,
  wrapped_url): they come from `async_questions/state_tests.rs`, an editor with no footer and
  history; their bodies are partly unwritten ("not written yet"). They need an entry each AND a
  decision whether the async editor is this sheet; not decided here.
- User verification: `prompt_snapshot`, `waiting` rows and the cancel keys are pinned in
  `tests/sheets/approvals.rs`; the oracle `prompt_snapshot` passes; `waiting_snapshot_...`
  pends on the controller (Enter on Verify), `clipped_prompt_...` on the clipped header,
  `full_screen_pager_...` on the pager.

## Handover (outside this area)
1. `core/src/app.rs` (shared, outside the area; 29 additive lines in this branch, to land with
   it): the **Respond arm** (`Intent::Respond` resolves the request like `Answer`), the **label
   at queue push** (the prompt gets the agent picker's thread label) and the **Presented arm**
   (a seeded fixture replaces the queued prompt). Missing: the **user-verification controller**: on `Decision::Verify` set `waiting` on the queued prompt,
   send `userVerification/verify {title, description, challenge}` (the challenge stays in the
   queued request, never in a view), keep an attempt id and drop a late result after cancel,
   resolve or reconnect, answer the elicitation `accept` with the proof in `content` or `failed`
   with the closed error; remote workspace answers `Unavailable(ProviderUnavailable)` with a
   warning. The oracle spells it in `app/user_verification*.rs` (~700 test lines).
2. The banner needs the page that owns the bottom of the SessionPage to draw
   `PendingThreadApprovals { threads: pending_threads(..) }` above the composer.
3. `src/widgets/options.rs`: `Options` has no "no cursor" state (the oracle's Backspace in an
   options question clears the selection; the sheet keeps its own `selected` flag).
4. Esc on a request whose list has no cancel option (`Dismiss`) maps to the skeleton's dismiss
   dialog; the oracle's mapping per kind is in `dialog.rs` `Msg::Cancel`.
5. Keymap: only the defaults are used (`list.*`, `approval.*`); the configured-keymap tests wait
   for the kit's `.keymap`.
6. rtwk parses byte 0x08 as Backspace+CTRL, so the oracle's ctrl+h arrives as Backspace with
   CTRL: the sheets bind that form (`user_input.rs`, `form.rs`).
7. `rtwk-gaps.md`: G72 row (committed) now also names the URL wrap; G49, G39b, G55 (the secret
   field shows no last character: `TextField` obscure with the reveal belongs to G55), G8 stay
   as they are. No new gap row beyond G72.

## Run in the binary
`check.py --bin rtwk --bin oracle --only exec-approval` and `--only request-user-input`
(captures in `/tmp/approvals-cap`): PASS on both binaries (`sent y at an approval prompt;
sentinel on screen: yes`; `answered a request_user_input question (Enter)`). The rtwk capture
`turn-1-working` shows the exec approval as the bottom pane: `Would you like to run the
following command?`, `Environment: local`, `Reason: ...`, `$ touch approved-by-mock.txt && echo created`,
`› 1. Yes, proceed (y)`, the `p` row with the prefix, `3. No, and tell Codex what to do
differently (esc)`, `Press enter to confirm or esc to cancel`. The oracle's scenario captures
(ready, turn-1-working, turn-1) never hold the dialog (the oracle redraws it between
checkpoints), so there is no row diff of the prompt against the oracle binary; the rows equal the
oracle's snapshots in the sheet and shard tests above. The mouse path is walked by the sheet
tests (tap, second tap), not by the pty scenario.
