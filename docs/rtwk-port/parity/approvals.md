# approvals — parity (S13)

Oracle `bottom_pane/approval_overlay.rs`, `mcp_server_elicitation.rs`, `request_user_input/`,
`user_verification.rs`, `pending_thread_approvals.rs`. View `src/sheets/approvals/`, core
`core/src/screens/approvals.rs` (+ `approvals/form.rs`, `approvals/input.rs`),
`core/src/protocol/requests.rs`. Sheets over the SessionPage (architecture.md §1 Amendment
2026-11-19b), opened by the app-level request presenter for the head of `CodexApp.requests`; an
answer is `ScreenIntent::Approvals(Intent::Answer | Respond)` through the app's effect path, the
sheet never sends an RPC and holds no thread state (ADR-003).

## chunk-approvals (approvals in the live chat), state at the handoff
Brief `research/briefs/chat-approvals-live.md` §6. Branch `worktree-agent-a3b475490f68cf717`, on
claude/tui-rtwk `482b19c76`. Done: steps 6 (view side), 8a, 2 (in-area part), 3d (order), 1 (re-tags),
**9 (guardian: core + view + /auto-review list)** and the **pure parts of 4 and 7**. NOT done: the
wiring of 9, 4, 7 (other owners' files, "Needs" below), 5, 10 and most of 3.

### Built in this job
- **Guardian / auto-review** (step 9). Core `approvals/guardian.rs`: `Review`, `Reviewing`, `Status`,
  `Action` (the protocol's `GuardianApprovalReviewAction` as typed variants), `Denials` (10 newest, one
  per review id), `ThreadPart::apply(ev) -> Option<Ask>` (pure; the history it owes is drained with
  `take_history()`), typed `History` (`Decided`, `Warning`, `NoDenials`, `RetryApproved`, `DenialGone`,
  `Unlocalized`; no prose in core), `Ask::ApproveDenied` (`thread/approveGuardianDeniedAction`),
  `project` (notification -> `ThreadEv`), `Decided::Reviewed` (`decided.rs`: Denied / TimedOut of a
  command is the decision cell with `Actor::Guardian`, the other actions have sentences of their own;
  Approved leaves no cell). View `sheets/approvals/guardian.rs` (the cells: patch, stdin, MCP call,
  network, permissions, `Strings` for every sentence), `review_status` (the Working row from in-progress
  reviews: header, up to the oracle's detail lines, `+N more`), `auto_review.rs` (`Auto-review
  Denials`, searchable, the disabled `Action` / `Rationale` header; `auto_review_menu(denials)` is the
  `Menu::List` and `open_auto_review(cx, hs, denials)` opens it on the select dialog).
- **Typing-idle wait** (step 4, pure part): `sheets/presenter.rs` `Presenter::typed(now)` and
  `Presenter::wait(family, now) -> Option<Duration>` (`TYPING_IDLE_DELAY` 1 s; only `Family::Approval`
  waits, a form, a question and a verification do not; more typing moves the moment out). Not called by
  `present` yet: see need 10.
- **User-verification controller state** (step 7, pure part): core `approvals/verify.rs`
  `Verifications` (`note` keeps the challenge by `RequestKey`, `begin` once per request -> `Attempt`
  with an `AttemptId` generation, `is_pending`, `remove`, `cancel_thread`, `clear`), `Outcome::answer()`
  (accept: proof as `content`; cancel / failure: `_meta openai/userVerificationReason`, the closed
  reason only), `Failure::of_server(data)` / `unreachable()` with a typed `Cause` (10 causes), the
  sentences in `Strings::verification_failure(cause)` + `verify_remote`. Fix in `protocol/requests.rs`:
  Esc / `c` on a user verification now answers `cancel` with `_meta userCancelled` (the oracle's
  `user_verification_cancellation_invalidates_the_inflight_result`; the old core test had `null`).
- **Auto-resolution countdown** (step 8a): `core approvals/input.rs` `auto_resolution(can, snoozed,
  elapsed)` (hidden grace 60 s, visible 60 s, due), `UserInputPrompt.can_auto_resolve` from
  `isBlocking == false`; the sheet runs a periodic timer (`AppBinding::start_periodic`), draws the
  red `auto-resolves in 1m 00s` in the progress row, any key or paste snoozes it, due answers `{}`
  (`requests::answer` accepts an empty map: the oracle's `submit_empty_auto_resolution`).
- **Clipped header** (step 6, view side): `sheets/approvals/clip.rs` `Pane`: a `LayoutBuilder` +
  `segmenter_of(cx)` + `widgets/measure.rs` rows picks the composition (whole column, or the header
  in a `SizedOverflowBox` of the room above the dim `[… N lines] ⌃a view all` row); the gap, then the
  blank row before the hint, give way first (the oracle's `picker_areas` order). Entry `exec_clipped`.
- **Decision cells** (step 2): core `approvals/decided.rs` `Decided::of(prompt, decision)` (exec:
  command or network target, six outcomes, actor User/Guardian; permissions: four plain grants; none
  for another thread, a patch, a form, a question; `AllowHost` has no wire answer so no record),
  `ExecPrompt.network_target`, `ThreadEv::Decided`; view `sheets/approvals/decision_cell.rs`
  `DecisionCell` (mark `✔ `/`✗ `, bold verbs, dim snippet, two-column hanging indent as
  `Row[SizedBox(2), Expanded(Text.rich)]`, `...` after the first line of a multi-line command, 80
  graphemes by `rtwk_core::unicode::grapheme_boundaries`). Wording is the oracle's, byte for byte.
- **Order** (step 3d): core `approvals/order.rs` `to_show(queue)` (the view on top is the last run of
  one kind; approvals newest first, forms and questions oldest first) and the presenter follows it.
- **Re-tags** (step 1): the 35 non-approval `exec_flow` pendings in `chat_approvals_1.rs` name their
  chunk (cells 23, status 4, chat-commands 4, live 3); `apply_patch_request_omits_diff_summary_from_modal`
  runs on a real request (`fixtures/chat_approvals_1/patch_omits_diff.jsonl`) and passes. The 11
  guardian pendings in `chat_approvals_2.rs` name their exact needs (the core and view halves exist).

### Counts (`summary.py --run`, pass / n/a / pending / red)
| shard | start | now |
|---|---|---|
| approvals_1 | 15 / 17 / 26 / 0 | 15 / 17 / 26 / 0 |
| approvals_2 | 31 / 14 / 22 / 0 | 33 / 14 / 20 / 0 |
| chat_approvals_1 | 2 / 5 / 47 / 0 | 3 / 5 / 46 / 0 |
| chat_approvals_2 | 0 / 18 / 30 / 0 | 0 / 18 / 30 / 0 |
| total | 48 / 54 / 125 / 0 | 51 / 54 / 122 / 0 |
Steps 9, 4 and 7 turn on no oracle test by themselves: each of the 11 guardian tests in
`chat_approvals_2.rs` needs a live path (routing, history item, Working row, `/auto-review` command), all of
which are other owners' files (needs 1-4 below); they now name exactly that. Their own tests carry the
behaviour (below).
Checks at the tip: `acargo fmt`, clippy `--workspace --all-targets -D warnings` exit 0, whole nextest
`--workspace` 1774 run / 1774 pass / 17 skipped (the first run after the build showed
`startup::session::{a_remembered_mode_opens_without_a_prompt, ctrl_c_on_the_cwd_prompt_opens_nothing}` red
under load; both pass alone and in the rerun of the whole suite), `shards.py --check` 0 files stale.
Own tests: core `approvals` 81, 80 run (decided 9, order 6, auto_resolution 6, guardian 18, verification
controller 7; the 1 ignored is `#[ignore = "needs(chat-live, popups)"]`, the through-the-app guardian test);
`tests/sheets/approvals.rs` (decision_cell, countdown, clip, reviews, review_status, auto_review,
verification_words 3; one `#[ignore]` for G49 and one `needs(popups)` for the 40-column name ellipsis);
`tests/sheets/presenter.rs` 11 (typing_idle 3).
Binary: `check.py --bin rtwk --bin oracle --only exec-approval` and `--only request-user-input`: PASS on both
binaries (4/4); the oracle capture never holds the dialog at a checkpoint (as before), so no row diff of
the request against the oracle binary; the rows equal the oracle snapshots in the sheet and shard tests.
Guardian has no mock-model scenario, so it is not driven in the binary (its notifications are not routed yet).

### Gap rows (no new row; existing ones used)
- G49 (`TextWrap::Word` does not break after `/`): the decision cell of a path-long command wraps at
  spaces and cuts a path at the width; the oracle breaks after the slash. `#[ignore = "pending(G49)"]`
  in `tests/sheets/approvals.rs`.
- G72, G39b: as before (wrapped URL autolink, a column measured over the rows in view).
- G-new (mouse yield for links in an expanded question) and G55 (secret field reveal): not reached.
- Not a gap, a need of popups: the select rows must cut a long name with an ellipsis (`@…`) the way the
  oracle's `Action` column does; ours wraps it at width 40 (`tests/sheets/approvals.rs auto_review`,
  ignored `needs(popups)`, with a green sibling at the width that fits).
- No gap row for the typing-idle wait: rtwk has the pieces (`AppBinding::start_timer`, the ticker's
  clock); `AppCx` does not give them to `present(app, cx)`, so the wait is a widget's job (need 10).

### Small deltas (one line each)
- Another kind of request does not stack over the shown dialog at once: one dialog is open at a
  time, `to_show` picks the next when it closes (the oracle stacks on arrival; only visible with
  two kinds queued).
- `to_show` has no "current of a covered view" (oracle: A1, A2 queued, E1 stacks over, after E1 the A view
  resumes with A1; ours shows A2, the newest). Needs the presenter to keep the shown stack.
- `ctrl+a` / `ctrl+shift+a` are dead until the static-pager hook exists (`open.rs`, transcript chunk);
  the `⌃a view all` hint is drawn as the oracle does when the header is cut.
- The options rows are measured at the list's width less the cursor mark (an approximation of the
  oracle's `wrap_row_lines`); the reason's newlines are joined (the oracle draws one span).
- The red-countdown oracle test stays pending on the kit's negated-fg check (`tests/kit/grid.rs:256`);
  the sheet test `user_input_the_countdown_is_red` pins it.
- The guardian command cell shows the command as the protocol carries it; the oracle re-quotes an
  argv with `shlex` (`history_cell/approvals.rs`): the two differ only for a redundantly quoted
  command (the cell is built from the already joined command string).
- The guardian warning is a visible history line; ours has `Item::Warning` detailed-only, so the
  line needs a visible kind (need 2).
- The `/auto-review` list's footer hint is the select dialog's own; the oracle's is keymap-driven
  (popups, keymap).
- Thread ids are not canonicalised (the oracle maps a compact id to its dashed form for the
  verification table): our `RequestKey` holds the id the server sent.

### Needs (files of other owners; each is a one-line request)
1. **Routing of the guardian notifications** (`core/src/screens/mod.rs` `project_thread`: an arm for
   `approvals::project`; `core/src/protocol/targets.rs` `notification_thread`: the three arms
   `item/autoApprovalReview/started|completed` and `guardianWarning`). The patch is verified green with
   these two edits (reverted: not my files). chat-live / popups.
2. **History items** (`core/src/threads.rs`): after each `ScreenThreadEv::Approvals` apply, drain
   `slots.approvals.take_history()` and map it: `Decided` to a decision item, `Warning` to a visible
   warning item, `Unlocalized` / `DenialGone` to an error item, `NoDenials` / `RetryApproved` to an info
   item (words: `no_denials(+_hint)`, `retry_approved(+_hint)`, `denial_gone`,
   `unlocalized(err)`); `end_turn` applies `ThreadEv::TurnEnded` to the slot. There is no
   `Item::Decision` yet: cells owner adds it and the `transcript_pane.rs` arm using `DecisionCell`
   (`Item::Agent` keeps `questions` for step 10).
3. **Working row** (`src/pages/session/activity.rs`, chat-status): header / details / max lines from
   `sheets::approvals::review_status(s, &thread.screen_slots().approvals.reviews)`.
4. **`/auto-review`** (popups / `commands/dispatch.rs`): empty list -> `ThreadEv::NoDenials`, else
   `sheets::approvals::open_auto_review(cx, hs, &denials)`; map the pick `Picked::Item(id)` to
   `ThreadEv::ApproveDenied { id }` (there is no `ScreenThreadIntent::Approvals`; the slot's `Ask`
   becomes `ThreadFx::request(thread/approveGuardianDeniedAction)`); the select rows cut the name with
   an ellipsis; `sheets::select` could export a `menu_dialog()` so a debug entry can show a `Menu::List`.
5. `core/src/app.rs` (keymap holds it): in `resolve_request`, when the request's thread is the active
   one, `Decided::of(&prompt, &decision)` -> `ThreadEv::Decided` (Esc/ctrl+c record `Cancel`); label from
   (request thread, active thread) at present time instead of at push; patch changes from the thread's
   `Item::Patch`; `Intent::CancelRequests` on ctrl+c (drops queued approvals unanswered); RUI Esc ->
   `Intent::Interrupt` + drop queued RUI; the stream-deferral flag the presenter reads.
6. **User verification** (`core/src/app.rs`, `protocol/mod.rs`, `src/services/backend/fake.rs`): own one
   `verify::Verifications` in `CodexApp` (ADR-003, one writer): `note(key, &request)` on arrival,
   `Decision::Verify` -> `begin(&key)` -> `Command::VerifyUser { key, attempt }` -> the host sends
   `userVerification/verify { title, description, challenge }` (the pin has the types); the result comes
   back as `AppEv` with the attempt: `is_pending` false -> drop; `Ok(proof)` -> `Outcome::Accepted`;
   `Err(data)` -> `Failure::of_server(data)` / `unreachable()`, a warning line with
   `Strings::verification_failure(cause)`, `Outcome::Failed(details)`; the answer is
   `Outcome::answer()` through `resolve_request`; `remove` when answered or resolved by the server,
   `cancel_thread` on a discarded thread, `clear` on reconnect; set `waiting` on the queued prompt; a
   remote workspace warns with `verify_remote` and answers `Failed { Unavailable ProviderUnavailable }`.
   The oracle's `user_verification_tests.rs` (357 lines) comes with it.
7. `core/src/protocol/mod.rs` (`thread/approveGuardianDeniedAction`, `userVerification/verify`) and
   `src/services/backend/fake.rs` (the fake answers both).
8. `src/pages/session/session_page.rs`: mount `PendingThreadApprovals` (banner) and the question editor
   above the composer.
9. `src/pages/session/open.rs` / transcript chunk: `Target::StaticPager { title, body }` for ctrl+a.
10. **Typing-idle wait** (composer chunk + presenter home): the composer calls
    `app.ui.presenter.typed(now)` for the keys the oracle counts as activity (characters, Backspace,
    Delete, Enter, Tab without ctrl or alt, at press or repeat) with the ticker's clock; a small widget
    in the presenter's home (`home_node`) holds a `Timer` (`AppBinding::start_timer`) for
    `presenter.wait(family, now)` of the head and, when it fires, `cx.send(())` so `present` runs again;
    `present` passes `now` and does not open an approval while `wait` is `Some`. Turns on
    `pane_footer_2` 4 + the two modal tests.
11. `src/widgets/options.rs`: an empty-cursor state (`backspace_in_options_clears_selection`).
12. Kit (`tests/kit/*`): `.keymap(..)` seed (15 keymap tests + the ctrl+g clipped test), `check_style`
    with negated fg/bg, kitty bytes for shift+enter and ctrl+5, a seed that opens the unanswered
    confirmation.
13. `tests/oracle/pane_footer_2.rs` 454-584 (chat-status's file): `s.feed(request)` in the six approval
    tests.

### Next steps for the porter who continues
1. Step 10 (async questions): model in `approvals::ThreadPart` from `AgentMessage.questions` (dedupe by
   message id), editor above the composer; both wait for the composer chunk and need 2 / 8.
2. Wire the needs above as their owners land them; un-pend as they do: chat_approvals_2 guardian 11
   (needs 1-4), approvals_1 3 decision-cell tests and chat_approvals_1 network x3 / `exec_approval_*`
   (need 2, 5), pane_footer_2 approval tests (10, 13), `waiting_snapshot_*` (6).
3. Step 3 rest (a, b, c, e, f) and step 5 sit in `app.rs` / `session_page.rs` (needs 5, 8).

## Built (the mock-dialogs slice, before chunk-approvals)
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

## Shard counts of the mock-dialogs slice (superseded by the chunk-approvals table above)
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
