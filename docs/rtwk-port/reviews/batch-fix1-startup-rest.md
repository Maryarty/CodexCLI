# Batch review, fix round 1: startup-prompts-rest, providers (partial), import-rest

Reviewer: codex-reviewer (FULL tier, process.md A5/A3). Tree: claude/tui-rtwk at 451d7698d.
Ranges: startup-prompts-rest 980c08242^..b46cc5439; providers f51e15937..claude/providers (3f06720ff);
import-rest f51e15937..claude/import-rest (78eeaf328). rtwk pin: /Users/admin/.cache/codexcli/rtwk-port-pin
(codex-rs/tui-rtwk/Cargo.toml:10).

Run here (CARGO_TARGET_DIR=target-wave2): `nextest -p codex-tui-rtwk -p codex-tui-core` 958 tests: 953 pass; 5 fail
once under 3 threads (status_data x3, shell_agents cancelling_resume_picker, import_is_a_step_of_the_startup_page),
`slash_import_opens_the_import` failed once in 3 isolated runs and passed 6/6 in a later loop. Oracle profile, shards
startup_prompts + signin: the red set is exactly the declared pending set (startup_prompts 6: prompt_snapshot G48,
narrow_picker G49, provider_picker G50, cwd persistence, 2 inline; signin 8: api_key G48 + 7 unbuilt wizard tests;
plus the Windows sandbox test of chat_popups_2). Rules tests (layers, R-rules) are inside the 958 and green.

## Verdicts
- startup-prompts-rest: **FIX** (docs only: rows G49 and G50 do not carry the three routes with file:line, G49 leaves a
  route unnamed; no code change needed unless the person picks the ZWSP route).
- providers (partial): **MERGE-WITH-NOTES**.
- import-rest: **MERGE-WITH-NOTES**.

## 1. startup-prompts-rest

Done as asked: P1 (`words()` gone, plain Text, prompt_snapshot pending G48, row extended: panel.rs, 980c08242); P2
(Options hanging label `RichRow`/`view_rich`, `arrow()` from the scroll, OSS rows with underlined hotkey and coloured
status; three pending tests on); N-P1 (keys.rs = `bindings(&Prompt) -> (KeyPress, Cmd)` bound with `Focus::bind`; core
keeps `step/number/take`, `Intent::Answer` validated by `takes`); kit `Owner::StartupPrompts`; Ctrl+C of the hooks
review: the oracle handles none, `bindings` has none (keys.rs:96-107). Pending markers: G48/G49/G50 on open rows; the
other three (cwd persistence, 2 inline) are plain "not built" pendings with a reason, none is `pending(Gxx)` on a closed row.

Findings
1. FIX, rtwk-gaps.md G49 and G50 (a80396d39): the 3-route checklist (process.md §5) is not in the rows. I checked both
   gaps in the pin and they are real:
   - G49: `TextWrap::Word` breaks after a space, hyphen, between CJK, never after `/`
     (rtwk-render/src/objects/text.rs:55-63 doc, the break rules in objects/text/). The oracle snapshot
     `cwd_picker_selected_40x16.snap` breaks `(/` | `Users/example/current)`.
   - G50: rtwk's flex gives the whole-cell remainder to the FIRST flex children (rtwk-render/src/objects/flex.rs:927-934,
     `part()` at :273), the oracle's `FlexRenderable::allocate` (tui/src/render/renderable.rs:296-360) gives it to the last;
     no `CustomMultiChildLayout` in the pin (grep of crates/ finds none).
   What is missing: file:line for route 1 (the probe), route 2 (compositions tried) and route 3 (test setting, or "n/a"
   with the reason); G49/G50 have no probe file like `tests/rtwk_g46.rs`, only the oracle test that goes pending.
   G49 route 2 has an unexamined candidate: rtwk's text engine honours U+200B as a break opportunity
   (rtwk-render/src/objects/text/clusters.rs:395, the `ZWSP` flag), so `/` + ZWSP in the label string would break as the
   oracle does. It rewrites the string (the §5 definition of a workaround, and copy/selection would carry ZWSP), so my
   advice is to name it in the row and reject it there, or ask the person; it must not be silently skipped.
2. Risk, src/widgets/options.rs:255 (a80396d39, the pattern predates this slice, G39): `heights` is written inside the
   `LayoutBuilder` closure (state written during build, and derived values stored) and `set()` (:98-125) reads it to
   `jump_to` the whole-row window; `fits()` (:388) is the oracle's `is_selected_visible_in_wrapped_viewport` carried, and
   the `tail` padding (:276) is a trick for it. The skill's table names `widgets/session_list.rs` (ListView +
   ListController + `ensure_index_visible`) for whole-row scroll with the arrows; Options needs a shared Grid column, which
   is why it is not a ListView, but the measure-then-jump shadow of the scroll should get its own gap/route row or be
   moved to a controller listener. The slice made it bigger (+323 lines), so decide before more users depend on it.
3. Note, scope: item (1) of the brief (Windows sandbox, security_setup, startup_error) is not built; the pack's "Scope of
   the rest" gives reasons (chat popups owned by dialogs-pane; `Notice::valid` needs `url`, core's dependency list is fixed by
   tests/layers.rs; startup_error is a REUSE row). The reasons hold, but the descoping was the porter's: the orchestrator
   should file or confirm the three owners (dialogs-pane, reuse-carry) so the oracle tests of those files are mapped somewhere.
4. Note, tests first: P2 and N-P1 turn the pending tests on / move the key tests in the same commit as the code (no red
   commit); the expected values are the oracle's `.snap` text, so the bite is intact, but the order is not the A4.2 order.
5. Note: panel.rs `Give::{Notes,Body}` shelves with `overflow_intended` and `Flexible(Loose)` per child are a composition,
   not a hand layout; OK (the reviewer of wave 2 judged the Panel route sound).

ADR conformance: ADR-003 kept (prompt read from `ScreenStates.startup.open` by `select`, widget keeps controller only, answers
via `Intent::Answer` and `apply`); ADR-002 conditions kept (`impl_widget!(stateful eq Panel)`, the panel is pushed under the Navigator
by `prompts::ask`). No departure, no violation.

## 2. providers (O1, O2+N-O5, O3, GCP wizard)

O1: tests red first (26289ebd3, 76 test lines), fix ba202af87 (`choose` ignores Choose while `awaiting`, stale Browser
outcome gets `Ask::CancelLogin`, cancel forgets the key): as briefed. (The oracle itself starts a second login on a second
Enter; the fix is the review's, not the oracle's.) O3: blossom `faded` uses `AnimationController` (no hand timer),
page test present. KeyGuard: 7 tests, `Rc<Cell>` handle, equality by pointer (`eq` widgets), time from the app clock.
GCP wizard: core model pure (no rtwk/IO, `auth::provider::{Wizard,Setup,Intent,Effect,gcp}`), wire arms in
core/src/protocol/onboarding.rs via the existing `login_start`/`login_outcome` (departure from the brief's
host/src/providers.rs: none needed, better). Oracle `gcp_menu_gating_and_setup_flow` on and green; the Bedrock/gateway
pendings are "not built" (no Gxx).

Findings
1. Risk (a defect for the user), host/src/startup.rs:434 `WIZARDS_BUILT = false`: the brief allowed "the `login_policy` line
   that offers a wizard once built"; the slice did not touch it, so in the real binary no provider row is offered and the
   GCP wizard is reachable only from the debug fixtures and the page test. One bool for three wizards: make it per feature
   (GCP true) when startup-deltas' file is free; until then the slice is not "done" end to end.
2. Risk, O2 only half done: signin/mod.rs still draws the menu from hand lines (`Pick::lines`, mod.rs:639-745; `compact`
   rows spliced when `rows_of(..) > height` at :742, `one_word` hand wrap choice :575, `rows_of` :580 re-measuring with
   `measure::rows`). Options is used only as a cursor holder (`Options::new()` also builds a ScrollController nobody uses)
   and its `step`. The brief said the `compact_options` row measuring is the thing to remove; the handover states what
   options.rs lacks (one column, description row, caret), which is the agreed route, but the measuring workaround stays
   until that option exists. Record it as a debt, not as closed.
3. Risk, provider/mod.rs:67-95 `sized`/`frame`: the wizards measure their paragraphs (`rows_of`) to give them a `SizedBox`
   height only because `paragraph()` is an `OverflowBox` that fills its room. A `Column[Expanded(Column(min)[Text..]),
   footer Text]` needs no row count. Bedrock and gateway will copy this; fix it before they do.
4. GCP view (provider/gcp.rs): one `TextField` per stage keyed (`with_key`), its own `TextEditingController`/`FocusNode` in
   the State, `autofocus(true)` plus a post-frame `request_focus` (`AppBinding::add_post_frame_callback`, :80), an idle
   `Focus` node holding focus while configuring, `guarded_keys` on the nodes' `set_on_key_event`. This is rtwk-standard
   (Flutter's `addPostFrameCallback(() => node.requestFocus())` and `FocusNode.onKeyEvent`); no ported render arithmetic.
   The `autofocus` and the post-frame focus are redundant on a fresh mount; the parity pack records the post-frame one as
   what the shard test bites on, so keep, and drop `autofocus` only with a bite test. The field text goes to core only in
   `Intent::Next(Fields)` on submit (no application data kept in the widget beyond the typed buffers).
5. Note, docs/rtwk-port/parity/onboarding.md: the "Each test below was seen failing..." sentence is duplicated (Bites).
   The shard test waits 1100 ms for the guard where the oracle sends a key release (disclosed in the pack).
6. Note: signin/mod.rs is 935 lines (`Pick::lines`, device code, success, API key); the next wizard slice should split it.

ADR conformance: ADR-003 kept (wizard state is `SignIn.wizard` inside `ScreenStates.onboarding`, changed by `apply`; the
view keeps controllers and nodes only). Layers kept (core has no UI/IO, services untouched, host untouched). ADR-002 kept
(`impl_widget!(stateful eq ..)` with `KeyGuard: Eq`).

## 3. import-rest (I1, I2, I3, (3); (2) blocked on chat-cells)

I1: the source prompt snapshot is green (parity test commit 334b57d6f first), G47 row removed, "Still open" line updated.
It is built beside `Panel` (source.rs, 209 lines, `Options::view` on the prompt surface) rather than on it; reason in the
pack (Panel indents its titles and is bound to the startup model's `Prompt`). Sound for now. I2: `Command::Import` through
`fx.commands`, request number = `Model::generation`, stale answers dropped by `apply`, completion matched on `import_id`
(core `observe`, apply_event arm app.rs:~771), RPC mapping pure in core/protocol/import.rs, the RPC loop in
src/services/backend/import.rs shared by the fake (`fake.rs` arm) and the host, the file-system part (`places`, `config.cwd`)
in host/src/import.rs. The split is the right one: IO only in host/services, the view sends only intents, no thread state.
(3): StartupPage leaves the import step when the flow is Done at a later generation (onboarding/mod.rs, `import_from`).

Findings
1. Risk, test flake: `slash_import_opens_the_import` failed once in three isolated parallel runs (3 threads) and
   `import_is_a_step_of_the_startup_page` failed in the full run: they press Escape after `pump_for` while the shell applies
   the page's `Open` on its own thread (the pack says so). The wiring made the tests thread-dependent; wait on a condition
   (as `tests/pages/import.rs` `mod flow` does), not on time, before the next landing gate trips on it.
2. Risk, I3 is halfway (pages/startup/import/mod.rs): nav.rs is gone and focus is two `FocusNode`s + `KeyBindings` +
   `ListController` (good, rtwk-standard, and the focus walk is tested), but the two cursors are `item: usize` /
   `action: usize` in the State (:150-151) moved by hand (`Msg::ItemUp/Down`, `go_to_actions`, `go_to_items`, `settle`,
   :380-415) and the list height is hand arithmetic (`fixed`, `room`, `list_height`, :506-510). Flutter/rtwk: a
   `SelectionController` per region (cursor, step, clamp on its own) and the list in a `Flexible`. The handover's reason for
   not using `SelectionList` (its `Marks::CURSOR` cell and fixed mark style vs the oracle's `› [x] Label`) is a decision,
   not a gap, and is documented; a `SelectionController` without the list widget avoids it.
3. Note: the pack says `SelectionList`/`Panel` were probed; no probe test is kept (G46's `rtwk_g46.rs` shows the manner).
4. Note: `IMPORT_FAILED`/`DETECT_FAILED` drop the server's message (`map_err(|_| ..)`, services/backend/import.rs): the
   comment says it is "only logged", but nothing logs it there.
5. Note, red first: bcb1587c0 ("parity tests (red)") also adds the protocol/import.rs mapping and the core arms (skeleton to
   compile); I3 and (3) tests land with their code. The wire tests assert oracle-derived params (JSON of
   `ExternalAgentConfigDetectParams`), fine. Item (2) chat cells: blocked on chat-cells (no history-notice `ThreadEv`);
   interface ready, not a defect of this slice; keep the task item open in tm.
6. Note: outside-area edits are announced in the pack (host/src/in_process.rs one arm, host/src/main.rs `mod import;`,
   onboarding/mod.rs, tests/app/navigation.rs, Cargo.toml `rtwk_g46`); codex-tui, codex-rs/Cargo.toml/lock, registry untouched.

ADR conformance: ADR-003 kept (the import model is a `ScreenStates` slot changed only by `apply`; the completion
notification passes `apply_event`; no thread state). ADR-002: `Screen` is `eq`, the sheet/dialog presenters sit under the
nested Navigator of `hosted` (debug) and the startup page's. No departure, no violation.

## Cross-cutting
- Layers/R-rules: no `#[allow(clippy::disallowed_..)]` in the three diffs; the only `Rc<Cell>`/`RefCell` additions are
  `KeyGuard` (input-state handle with its tests) and `Options.heights` (finding 1.2).
- Three slices lean on a shared Options (startup prompts, sign-in cursor, import source prompt) and each composes its own
  prompt surface (panel.rs, source.rs, signin menu): a generic prompt panel (title, subtitle, choices, hint, key map as
  parameters) is the Flutter shape and would also retire the copies; propose as one task before Bedrock/gateway.
- Not run: pty `check.py` (needs the built binary; the handovers report 36/38 with two base failures), mutation bites
  (the red-first commits of O1 and I1/I2 serve as the bite evidence).
