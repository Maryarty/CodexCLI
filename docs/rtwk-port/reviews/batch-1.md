# Batch review 1 — `3a9fdb1a5..1c1cc432f` (+ mock-model-all `1c1cc432f..e066e1778`)

**Verdict: FIX** (fix forward; every slice is landed). There are defects against
the oracle in chat-nav's dispatch and history search, an R14 breach in three
screens-dialogs widgets, a test that matches stub text, ported `render()`/scroll
arithmetic in screens-routes, and pack items missing in screens-routes and
screens-dialogs. Nothing crosses a layer (UI ↔ core / protocol / shell / IO).

Slices: mock-model (`873afd119`, `8c1a565a2`), screens-routes (`bc2059fb6`,
`e31e85c84`), screens-dialogs PARTIAL (`11ddcc383`), chat-nav (`1c1cc432f`), and
mock-model-all (`e066e1778`, added late, light look).

## Checks run (main checkout, `codex-rs/tui-rtwk`, at `1c1cc432f`)
- `clippy -q --workspace --all-targets -- -D warnings`: clean.
- `nextest run --workspace`: 362 passed, 1 skipped (`core/tests/script.rs:170`,
  `#[ignore = "delta D1: …"]`, has a reason, not part of this batch).
- `git diff --quiet 3a9fdb1a5 1c1cc432f -- codex-rs/tui codex-rs/Cargo.toml codex-rs/Cargo.lock`
  exits 0. `1c1cc432f..e066e1778` does not touch `codex-rs/`.
- `#[allow(clippy::disallowed_…)]` appears only in test files (the allowed place).
  The other new allows (`derivable_impls`, `too_many_arguments`, `duplicate_mod`)
  are not disallowed-lints.
- Layers: no view names the protocol, `shell`, tokio or std IO. Core names no
  rtwk. Popup, approvals, select, status, pager, resume, usage and agents data
  are all seeded by JSONL scripts through core `apply`. No mock data is handed
  to a widget. `layers.rs` and `rules.rs` pass.
- ADR-002: the new screen and dialog widgets are `impl_widget!(stateful eq …)`.
  `show_dialog` is called from widgets under the Navigator. Dialogs take ids
  (`RequestDialog { key }`) or nothing (`MenuDialog`, `CardDialog`).
- Keymap defaults in `src/screens/chat/keys.rs` equal `keymap.rs:1909-1951`
  (Ctrl+T, F3, F2, F8, Ctrl+R, Ctrl+S, Ctrl+/).
- Registry: the variant order equals `slash_command.rs`. These functions equal
  the oracle's: `supports_inline_args`, `available_in_side_conversation`,
  `available_when_thread_unavailable`, `available_during_task` and `is_visible`.

---

## G1 — chat-nav: dispatch differs from `slash_dispatch.rs` (defect)
Slice: chat-nav. Files: `codex-rs/tui-rtwk/core/src/commands/dispatch.rs`.
1. **`/resume` is not blocked while a turn runs** (`dispatch.rs:236-238`).
   - Oracle `chatwidget/slash_dispatch.rs:163-165`: `matches!(cmd, Resume | Cd)
     && (user_turn_pending_start || agent_turn_running)` gives "'/resume' is
     disabled while a task is in progress."
   - Failing input: `dispatch("/resume", &DispatchCx{task_running: true, ..})`.
     Ours returns `Outcome::Open(Resume)`, so the picker opens mid-turn. The
     oracle returns `Notice::DisabledDuringTask(Resume)`. `/cd` is already
     blocked, by `available_during_task`.
   - Fix: add the `Resume` rule to `dispatch_command`, with a core test.
2. **`/usage` arguments are not parsed as `TokenActivityView::parse` does**
   (`dispatch.rs:357-360`).
   - The oracle (`analytics/activity_chart.rs:37-43`) trims, lowercases, and
     accepts `day|daily|week|weekly|cumulative`.
   - Failing input: `/usage week` or `/usage Weekly`. Ours gives the usage
     line; the oracle opens the dashboard.
   - Fix: carry `parse` with its oracle comment, and port
     `token_activity_view_aliases_parse` (`activity_chart_tests.rs`).
3. **`/model` before startup shows the wrong message** (`dispatch.rs:250-255`).
   - The oracle (`chatwidget/model_popups.rs:19-24`) shows the info line "Model
     selection is disabled until startup completes.". Ours shows "Session is
     still starting; try /model again in a moment."
   - Fix: add a `Notice::ModelStartup` key with the oracle's string.
4. Note: a bare `/cd` builds `Cd { path: "" }`. The oracle dispatches `/cd ~`
   (`slash_dispatch.rs`, the `Cd` arm). Fix: `path: "~"`.
5. Note: `available_during_task` is carried as `!matches!(…)`. The oracle's
   exhaustive `match` forces a decision for every new upstream command; ours
   silently defaults new commands to `true`. Fix: an exhaustive match, or a
   test that reads the oracle's arms the way the order test does.

## G2 — chat-nav: history search keys differ from `history_search.rs` (defect)
Slice: chat-nav. File: `codex-rs/tui-rtwk/src/screens/chat/home.rs`.
1. **Ctrl+S opens a search** (`home.rs:238`: `SearchPrevious | SearchNext` →
   `SearchStart`). In the oracle only `history_search_previous` begins a search
   (`chat_composer.rs:2141`); Ctrl+S outside a search does not open one.
   Failing input: Ctrl+S on the chat shows `reverse-i-search:`.
2. **Up/Down do not walk inside a search** (`home.rs:83-100`). The oracle
   (`history_search.rs:172-184`) treats Up as older and Down as newer. Ours:
   `as_text()?` drops the key.
3. **Enter with no match ends the search** (`home.rs:285-290`: `SearchEnd`
   always). The oracle (`history_search.rs:213-230`) accepts only on `Match`
   and otherwise keeps the search open. The pack row
   `history_search_no_match_restores_preview_but_keeps_search_open` holds only
   in core.
4. Fix: bind Ctrl+R alone to start; add Up/Down to `search_key`; make Accept
   a no-op unless `status == Match`. Add a UI test for each.
5. Note (ADR-002 condition 4 / R10): `SearchKey::Char` builds
   `SearchEdit(format!("{}{text}", search.query))` from a `read` of
   `ScreenStates` (`home.rs:270-282`). It holds today only because the frame
   loop reruns the view between keys of one read
   (`enter_accepts_the_match_into_the_composer` would fail otherwise). The
   decision belongs in `apply`. Fix: `Intent::SearchAppend(String)` and
   `Intent::SearchBackspace`.

## G3 — chat-nav: Tab runs the command (risk; delta N1 is mislabelled)
Slice: chat-nav. Files: `home.rs:222-231`, `docs/rtwk-port/parity/chat-nav.md:385`.
- rtwk `Autocomplete` Tab puts in the shared prefix, and otherwise picks
  (`rtwk-editing/src/autocomplete.rs:652-677`). `ChatMsg::Picked` then runs
  the line.
- Failing input: type `/quit`, press Tab. The app exits. The oracle
  (`slash_input.rs:288-336`, `selected_command_completion`) completes to
  `/quit ` and runs nothing. Only `/skills` dispatches on Tab
  (`slash_input.rs:539`). `/new`, `/clear`, `/compact` and `/logout` are
  equally exposed.
- The brief asked for oracle keys ("Up/Down, Tab, Enter, Esc").
- N1 files this behaviour change under a "look" delta with "proposal: accept".
- Fix: move it to its own behaviour delta with "proposal: follow-up". Add to
  "rtwk needs": `Autocomplete` has to say whether Enter or Tab made the pick
  (or Tab completion has to never pick). Until rtwk has that, the person
  decides whether Picked runs only commands that are safe on Tab.
- Note: the completion lacks the oracle's trailing blank (`/model ` vs
  `/model`).

## G4 — chat-nav: transition tests do not bite on the opener (defect + risk)
Slice: chat-nav.
1. **A test identifies its target by stub text.**
   `tests/popovers.rs:165`,
   `up_and_down_move_the_selection_and_enter_runs_the_selected_command`, has
   `h.t.screen().contains("settings (S15): empty slot")`. It was added in
   `3041ed1a5` and was not converted in `4a295f4eb`. It breaks when S15 lands.
   Fix: `assert_eq!(open::last_opened(), Some(Target::Settings))`.
2. **`shows()` passes when nothing opens.**
   - `open.rs:39` records `LAST` before the `match`. `tests/chat.rs:78-80`
     checks only `LAST`.
   - The `command_opens!` tests then compare the screen after Esc with the chat
     before typing. That holds when the opener is a no-op, and also when the
     arm opens the wrong dir: the composer was cleared, and the first Esc
     primes silently (N8).
   - Failing mutation: `Target::Usage => push(cx, pager::route())`, or replace
     `drop(status::open(cx))` with `()`. Every transition test stays green.
   - Fix: after opening, assert `h.t.screen() != before`. Better, record the
     target inside each dir's opener (or check the route's widget type) rather
     than in `open()` ahead of the match.
3. Note: `open.rs:18-34` adds a `thread_local!` test hook (`last_opened`) to
   production view code. Consider moving it to `UiApp.ui` (debug) or behind
   `#[cfg(any(test, feature = "test-hooks"))]`.

## G5 — screens-dialogs: model data in widget state (R14 defect + missing rule)
Slice: screens-dialogs (`11ddcc383`; commits `3bf4fb078`, `3958fbe02`, `c4f85ef24`).
- `src/screens/select/dialog.rs:27-29`: `MenuDialogState { menu: Option<Menu> }`.
- `src/screens/status/card.rs:23-25`: `CardDialogState { card: Option<Card> }`.
- `src/screens/approvals/dialog.rs:34-36`: `RequestDialogState { prompt: Option<Prompt> }`.
- Evidence: each copies a core model value into State in
  `did_change_dependencies` (`select/dialog.rs:59-64`). The value is
  application data stored in a widget and a derived value kept in State. R14
  and the §2 map forbid both.
- Fix: keep only `shown: bool` (or the request key) to detect the close, and
  `select`/`watch` the value in `build`.
- **Missing rule:** `tests/rules.rs:276-284`,
  `r14_widget_state_fields_hold_no_model_types`, lists only
  `ThreadModel…Pending<`. Screen model types (`Menu`, `Card`, `Prompt`) pass.
  Fix: flag any type exported by `codex_tui_core::screens::*` (or any
  `Option<core type>`) in a `State` field, and show the rule biting on
  `MenuDialogState`.

## G6 — screens-routes: ported `render()` and re-implemented scroll (defect, translation method)
Slice: screens-routes (`dbd0c43b6`, `7f107eccf`, `9662c5691`, `662de8740`).
process.md §5: "`render()` is never translated line by line"; "No
re-implementation of what rtwk owns (… layout, virtualization, scroll …)".
- `src/screens/resume/mod.rs:239-300`: `screen_lines` ("oracle:
  `resume_picker/layout.rs::render`") paints the whole picker as `height`
  lines inside a `LayoutBuilder`.
  - The scroll comes from `layout::ensure_visible`/`more_below`/`scroll_percent`
    (`resume/layout.rs:365-430`), not from a `ListView`.
  - `scroll_top.set(top)` (`:275`) mutates State during layout/build through
    `Rc<Cell<usize>>`.
- `src/screens/agents/mod.rs:301-373`: the same pattern (`layout::scroll`,
  `scroll.set(start)` in build; `layout::wrap` and `ellipsize` re-implement
  text wrapping).
- `src/screens/usage/mod.rs:341-410`: `screen_lines` / `body_lines` paint
  lines in a `LayoutBuilder`.
- `src/screens/pager/mod.rs:138`: `estimate_max` (greedy word wrap) stands in
  for a missing rtwk metric. That is a workaround of the gap the pack lists as
  G-new.
- Fix (follow-up per screen): rebuild the lists as `ListView`/`SelectionList`
  rows with rtwk wrapping and scroll. Keep only the pure text helpers (`age`,
  `hint_row`, `progress_label`, `truncate`), each with its oracle comment. Any
  difference left becomes a delta or an rtwk need. Mark `estimate_max` as an
  rtwk-gap workaround and remove it when G-new is fixed.
- **Missing rule** (note): no rule catches a `Cell` mutated in
  `build`/`LayoutBuilder`, or a `Vec<Line>` painter. Fix candidate: ban
  `Rc<Cell<` in `State` fields under `src/screens/`, outside an allowlist.

## G7 — screens-routes: parity pack items (missing pack item)
Slice: screens-routes. File: `docs/rtwk-port/parity/screens-routes.md`.
1. The `:44` n/a reason, "need the app-server/sessions IO", is false for oracle
   tests whose logic this slice carried into `resume/layout.rs`:
   - `relative_time_formats_zero_seconds_as_now` and
     `long_relative_time_uses_words` (`age`);
   - `hint_line_compacts_on_narrow_width` and
     `hint_line_preserves_primary_action_labels_when_very_narrow` (`hint_row`);
   - `picker_footer_progress_label_*` and `picker_footer_percent_*`
     (`progress_label`, `scroll_percent`);
   - `moving_to_last_card_scrolls_when_cards_exceed_viewport` and
     `up_scrolls_only_after_crossing_top_edge` (`ensure_visible`).
   Fix: port them by name (or drop the helpers under G6), and give each row
   its own line.
2. D2 and D5–D10 lack the two fenced blocks (oracle / ours, the differing rows
   ± 2) that §A4.3 asks for. D1 gives its rows inline.
3. D3 and D4 state `kind: layout`. That is a cause, not a kind: use K<nn> or
   NEW.
4. D10 ("real triggers not wired") is now done by chat-nav. Fix: mark it
   resolved.

## G8 — screens-dialogs: parity pack and tests-first (missing pack item)
Slice: screens-dialogs. File: `docs/rtwk-port/parity/screens-dialogs.md`.
1. There is no oracle-test table for the done screens. Only four
   `approval_overlay.rs` tests are named. `list_selection_view.rs` tests are
   left "out of scope" without a row each. Status tests have no rows.
2. N01 and N02 have no oracle / ours blocks and no proposal. The `Scenarios
   (pty)` section ("n/a until scenario-harness") is missing. The `Rules`
   section (R14, ADR-002 conditions met) is missing.
3. Tests first: S14 (`3958fbe02`) and S19 (`c4f85ef24`) put the tests and the
   code in one commit. Only S13 has a red commit (`bb7c288f1`). Fix: none for
   the landed code; the remaining S15/S23/S24 should keep red-first.
4. Note: `tests/navigation.rs` (a skeleton file, outside the area) was edited
   in `3bf4fb078`. The pack declares the edit, and the change is right.
5. Fix: complete the pack in the slice's continuation task, before S15, S23
   and S24 are added.

## G9 — R-mandatory test harnesses absent (missing rule; skeleton, not these slices)
- New intents and events in this batch have no R2/R4/R5/R7/R8/R9/R10 test to
  extend. The batch adds `chat::Intent::{Session, Notice, ClearNotice}`,
  `popovers::Intent::{Record, Search*}`, `select::Intent::{Pick, Close}`, the
  approvals answer and `popovers::Ev::{Files, Mentions, History}`.
- Neither `core/tests` nor `tests` has an `assert_consistent`, end-path matrix,
  lifecycle × intent or permutation suite (grep finds none).
- Fix: an orchestrator task to create the harnesses (session-core or the wave
  audit). Each slice then adds its variants.

## G10 — notes (batched follow-up)
- `core/src/screens/popovers.rs` (`apply_intent`, `SearchEdit`/`SearchStep`)
  clones the whole history on every search key. That is O(n) per keystroke.
  Fix: split the borrow (`let Model { history, search, .. } = model`).
- `src/screens/chat/home.rs:135`: `review_mode: false` is hard-coded. It is
  correct until review exists; add a TODO naming session-core.
- `src/screens/chat/keys.rs:56`: the Ctrl+/ chord. Without the kitty protocol
  most terminals send Ctrl+_ or Ctrl+7, which the oracle aliases
  (`keymap.rs:730-735`). No UI test presses it.
- `src/screens/status/card.rs:145-150` pads labels with spaces to align the
  values (`FieldFormatter`). A two-column rtwk layout would replace this.
  Minor (part of G6's manner).
- Ctrl+C during a history search quits the app (pack N6). The oracle restores
  the draft. The quit binding comes from the skeleton, but the delta should
  name the owner slice.

## G11 — mock-model / mock-model-loop / mock-model-all (tools only; notes)
Slices: mock-model (`873afd119`, `8c1a565a2`), mock-model-all (`e066e1778`).
- `tools/rtwk-port/mock-model/server.py:122,141-146` (at `e066e1778`): in
  `--loop` mode `load()` runs again when the queue empties and appends every
  `matched` response to `_matched` a second time, even when the earlier copy
  is still there. `_matched` grows by duplicates, and `queued()` reports them.
  Fix: reset `_matched` in `load()`, or skip matched responses on reload.
- `server.py:41,67`: subclasses the private internals of Codex's harness
  (`harness.H._ResponsesHandler`, `_server`, `_responses`, `_seen`). An
  upstream change breaks them silently. Fix: have `check.py` check that these
  names exist and fail with a clear message.
- No `codex-rs/` file is touched. The pack (`docs/rtwk-port/parity/mock-model.md`)
  documents its sources, coverage and limits.

---

## What the orchestrator and the person decide
- G3: whether a Tab pick may run a command until rtwk tells Tab from Enter
  (the safety of `/quit` and `/new`).
- G6: whether the line-painter screens (resume, agents, usage) are rebuilt now
  or left as accepted method debt until their slices grow.
- Fix-forward tasks, one per group:
  - chat-nav fix: G1, G2, G4;
  - chat-nav delta: G3;
  - screens-dialogs R14 and rule: G5;
  - screens-routes method: G6;
  - screens-routes pack: G7;
  - screens-dialogs pack (in its continuation): G8;
  - R-harness: G9;
  - notes: G10, G11.
