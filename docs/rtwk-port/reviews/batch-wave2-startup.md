# Batch review: wave 2, the StartupPage slices (process.md A5 "each batch")

Candidate: land/startup-wave2, tip db2d8fb38 over 6ba257ced. Read-only review of the
five merges (diff `<merge>^1 <merge>`) against the tm briefs, the parity packs and the
oracle sources; nothing was run except code reading (fmt/clippy/nextest/scenarios run
separately by the lander).

## Verdicts

| slice | verdict | blocks landing |
|---|---|---|
| select-dialog (d3065b403) | FIX | S1 |
| startup-flow (28de38b6d) | FIX (small) | F1, F2 |
| startup-prompts (3af52ac22) | FIX (small) | P1, P2 |
| import (ced389ce4) | FIX | I1 |
| onboarding (db2d8fb38) | FIX | O1, O2, O3 |

"Blocks" = a rule of process.md section 5 or the no-visual-delta rule is broken, or a defect
exists. Every blocker has a cheap route given; the person may instead waive one and
open a debt task. No ADR violation, no layer crossing, no work in codex-tui.

## ADR conformance (all five slices)

- ADR-003: kept. The startup page and the auth flow keep their state in
  `ScreenStates.onboarding` (core, changed only by `apply`); no ThreadModel, no thread RPC:
  account/config RPCs run in `host/src/startup.rs` through `Command::Startup`, not in
  ThreadFx (they are not thread RPCs). Answers carry request numbers and stale ones are
  dropped (`core/src/screens/onboarding.rs` `Ev::Facts`, `TrustSaved/Failed`,
  `auth::apply_result`). Pages keep only widget state (cursor, field, guard).
  Exception to watch: import's `Model::request()` is a polled getter, not a
  `fx.commands` push (see I2); it is consistent with ADR-003 until wired.
- ADR-002: kept. Page routes under the Navigator (`StartupGate` pushes `onboarding::route`),
  dialogs via `show_dialog` in select; widgets are `eq`; ids not data into routes.
- ADR-010: kept. Grep of `Text::new/Span::new("...")` in src/pages/startup and
  src/sheets/select outside strings.rs finds no prose. Small exception: the host builds the
  English trust-failure context (`host/src/startup.rs` `"config/batchWrite failed ..."`,
  the oracle's string) and core stores `format!("{other:?}")` in `Error::Unexpected`
  (`core/src/screens/onboarding/auth.rs`, the oracle does the same); note only.
- ADR-006: select-dialog = departure: the sheet hard-codes the oracle's default
  `list.*` keys and reads no configured keymap (15+ oracle tests pending "the keymap slice",
  `parity/select-dialog.md` rows 39-100, "no keymap config yet" line 133); no tm debt task is
  named in the pack. Needs one (note N-S2). The startup pages use fixed keys as the oracle
  does (update_prompt.rs:148, onboarding keys are constants): kept.
- Layers: kept. IO (fs, process, webbrowser, RPC) is in `services/startup.rs` and `host/`;
  views name no protocol type; core imports protocol types only in `core/src/protocol/`.
  See M1 for what no rule catches.

## select-dialog (d3065b403): FIX

S1 (blocks, rtwk-first): the sheet re-implements windowing and scrolling by hand.
`src/sheets/select/dialog.rs:66` keeps `top`, `:268` `window_top(self.top, ..)`
(`core/src/screens/select/list.rs:500`, a port of `ScrollState::ensure_visible`), `:366` slices
`rows[top..end]`, `:348` draws the `more_above`/`more_below` arrows from `top > 0`, `:474-479` hands
the page height `window` to core's `Browse::page`. The brief said "rtwk show_dialog +
SelectionList/OptionList"; neither is used (`rows.rs` is a Grid + GestureDetector list).
There is no gap row justifying it (G44/G45 are about text width and TabBar). rtwk route:
`SelectionList` + its controller (or `ListView` with a `ListController`, as import's review
does and as `widgets/options.rs` does with `ScrollController`), arrows from the controller's
position (as update_prompt.rs `hidden()`), page size from the controller's viewport.
A pending(Gxx) test belongs wherever the oracle's window rule cannot be reached.

Notes:
- N-S1: `tab: Option<usize>` next to a `TabController` (dialog.rs:60-62) is a shadow of the
  controller's index; read it from the controller.
- N-S2: ADR-006 keymap departure without a debt task (above).
- N-S3: 39 of 63 oracle picker tests are pending; the pack lists reasons per row, fine.
  `rtwk-gaps.md` edit is outside the task's area (docs) - harmless.
- Four list mechanisms now exist (select `Rows`, `widgets/options.rs`, trust.rs highlight,
  signin menu). Unify onto one (SelectionList/Options) in a follow-up.

## startup-flow (28de38b6d): FIX (small)

F1 (blocks, rtwk-first): `src/pages/startup/onboarding/trust.rs:55-100` keeps its own
`Highlight` enum and Up/Down/First/Cancel logic for two rows, while the same slice's
`update_prompt.rs` uses the shared `Options` (cursor + `step` with wrap, `widgets/options.rs:99`).
Use `Options` (cursor in its controller) here too.
F2 (blocks, no hand layout, R15 in spirit): `trust.rs:225-285` ports the oracle's
`reserved_height`/`cwd_height = path_height / 2` arithmetic by hand (`inner = width - 4`,
`measure::rows`, `grapheme_rows`, `center_truncate_path`). It is a visual-parity measure, but
no rtwk route is recorded (file:line) and no gap row exists. Either record the route tried
and a row for what rtwk lacks (a max-lines + middle-ellipsis `Text`, or a Flexible chain),
or restructure with `Flexible`/`TextOverflow`. Pattern list of R15 does not see it (M1).

Defects/risks (not blocking):
- R-F1 (risk): `host/src/startup.rs` `RunUpdate` stores the action in a slot read after
  `run_app` returns (`host/src/main.rs`); the slot is set when the shell polls the
  `Command::Startup` future. The page quits one flush later, so in practice the slot is set,
  but there is no ordering guarantee (shell may drop an unpolled command at exit). Make the
  page quit only after the ask is acknowledged, or set the slot synchronously in the shell.
- R-F2 (risk): `InstallMethod::Other` gives no `UpdateAction`; `update_offer` returns None
  when `command` is None, so no prompt is shown: correct. Windows: `run_update` runs the
  Unix `current_dir("/")` path for every platform; the oracle's Windows branch differs
  (note, `services/startup.rs`).
- R-F3: a `TrustSaved` that never arrives (backend reconnect while `Saving`) leaves the step
  waiting with `CancelTrust` blocked for `Saving` (`onboarding.rs` `Intent::CancelTrust`); add a
  reset on `Disconnected`.
- R-F4 (departures, in the pack, no debt task named): trust only for local hosts; launch
  read errors count as "nothing due"; no network refresh of `version.json` (so the update
  prompt can only ever show from a cache another codex wrote); a saved trust does not reload the
  session config (project hooks/sandbox stay untrusted until restart). Attach these to
  `startup-deltas`.
- R-F5: the chat page is drawn first and the page is pushed when the facts arrive
  (`StartupGate`); the oracle shows the prompt before the chat. Short flash; record as a delta.
- Outside-area edits: widgets/options.rs (`scroll()`, `column_gap` 0 when no descriptions),
  widgets/measure.rs (`grapheme_rows`), core app.rs / protocol/mod.rs (`Command::Startup` +
  `connected` hook, origin default only), host Cargo (+exec-server, install-context),
  fake.rs (arm), session/blossom (`stage`, `fresh`), agents `path` made `pub(crate)`:
  all sound and minimal. Note the cross-page imports (`pages::agents::path`,
  `pages::session::blossom`): shared pieces belong in `widgets/`.
- Welcome uses `AnimationController` through the blossom (no hand timers): good.
  `welcome_logo_layout` pending (kit seed) is correctly a pending test.

## startup-prompts (3af52ac22): FIX (small)

Judgement on the open question (Panel over `Options` instead of the brief's selection
sheet): sound departure. The oracle's six prompts are full-screen top-left screens with
their own heading/body/hint shelves (the pack's reasons; import's G47 shows the sheet
cannot reproduce this frame). The departure is recorded in the pack and the handover. It
should, however, be applied consistently: see I1.

P1 (blocks, workaround where a gap row belongs): `panel.rs` `words()` (one `Text` per word in a
`Wrap`) works around the same limit that `onboarding` filed as G48 (`TextWrap::Word`
breaks after a hyphen). The pack says "No gaps row: composition works"; onboarding says
"a hand wrapper would be R15" and keeps a pending(G48) test. One of the two is wrong.
Also `words()` cannot split a word wider than the line and trims runs of blanks. Replace by
a pending(G48) test for the migration heading (and extend the G48 row), or accept
composition and then drop the pending in onboarding; the first matches the rule.
P2 (blocks, visual delta without a row): the OSS provider rows lack the underlined hotkey
and colour-coded status (`OptionRow.label: String`, pack "Needed elsewhere (c)"), and the
arrow rows are blank although `Options::scroll()` now exists (landed by startup-flow;
`panel.rs` uses `blank()`). These are own-widget limits, not rtwk's: finish them in
`widgets/options.rs` or file the debt (`startup-prompts-rest`) and keep the tests pending
(3 hanging-indent pending tests exist; the style delta has none).
Notes:
- N-P1: `Prompt::on_key(cursor, key)` in core takes the widget's cursor index and a `Key`
  enum: key interpretation in core fed with widget state. ADR-006 says keys are rtwk
  `Focus`/`Shortcuts`; prefer view bindings -> intents (`Intent::Answer`), core only
  validating. Not a layer break (pure fn, tested), a design smell.
- N-P2: nothing calls `prompts::ask` yet (startup orchestration not done): the six prompts
  are reachable from the debug menu only. Windows sandbox prompts, security_setup, startup_error
  not done (pack says next slice).
- N-P3: outside area: `tests/app/skeleton.rs`, `tests/oracle.rs` (registration): fine.

## import (ced389ce4): FIX

I1 (blocks, visual delta by decision): the source prompt is opened as the generic
selection sheet and its oracle snapshot is pending as G47 ("NOT an rtwk limit, a decision of the
brief"). The person rejected all deltas (2026-11-18). `rtwk-gaps.md` is for rtwk limits;
G47 is not one. Route: build the source prompt on `prompts::panel` (`Panel` over `Options`),
which already draws the oracle's top-left frame; remove the G47 row. Same for any prompt of
this family.
Notes:
- I2 (risk, known debt `import-rest`): the flow is not wired to the root.
  `Model::request()` (core `screens/import/mod.rs:205`) is a getter nobody polls, there is no
  `Command`, `wire`, shell or fake-backend arm. `/import` (`session/open.rs:47`) pushes the
  route, which then sits in `Detecting` forever (Esc cancels). It should go through
  `fx.commands` like `Command::Startup`, with its request number checked on the answer
  (the model already carries `generation`). Until then keep `/import` out of the command
  list or label it.
- I3 (rtwk-first, note): `pages/startup/import/nav.rs` (258 lines) is a hand-made focus
  and cursor machine (view, `Focus::{Items,Actions}`, highlighted action) held in widget
  state. Flutter: a `FocusTraversalGroup` with a checkbox list and a button row; rtwk:
  `Focus` nodes + `ListController`/`SelectionList` + `ButtonRow`. Keep as debt, name it.
- I4: G46 (import) is a real rtwk limit with a green probe (`tests/rtwk_g46.rs`): good.
  G45/N1 pendings are honest. The multi-select added to the sheet (`List.multi`, `Picked::Items`)
  is inside the select area; fine.
- I5: Escape/Ctrl+C during detection (beyond the oracle) is a behaviour addition; stated in the pack.

## onboarding (db2d8fb38): FIX

O1 (blocks, defect): double activation starts two logins and an orphan is never cancelled.
`core/src/screens/onboarding/auth.rs` `choose()` returns early only when `state != Pick`;
for `Method::ChatGpt` the state stays `Pick` while the ask is out (`awaiting` set), so a second
Enter (or `1`) pushes a second `Ask::Login`, overwrites `awaiting`, and the first answer
`Outcome::Browser { login_id, .. }` then fails the `awaited` check and is dropped
(`apply_result`) - only a stale `DeviceCode` is cancelled; a stale `Browser` login keeps
its local callback server and the host has already opened the browser
(`host/src/startup.rs` opens before core sees the result). The view never reads
`SignIn::awaiting()` (grep: unused in `signin/mod.rs`). Failing input: Pick menu,
ChatGPT highlighted, Enter, Enter within the round trip. Fix: ignore `Choose` while
`awaiting.is_some()`, and `Ask::CancelLogin` for a stale `Browser` outcome; add the two core tests.
O2 (blocks, rtwk-first, brief deviation): the brief says "Option lists use SelectionList and
its controller". `signin/mod.rs:19-22` builds the menu by hand (`highlighted: MenuRow`,
`step()` with `rem_euclid`, digit/y/n bindings, own row measuring `compact_options`), stating
that the oracle wraps and measures; `Options::step` wraps already and `ListController`/
`SelectionList` give cursor/scroll. The hand `Guards` timer for the held-Enter guard is a
legitimate port of `ActivationKeyGuard` (the oracle's rule), but it should sit in a small
reusable `KeyGuard` in `widgets/` with its own tests, not in the page.
O3 (blocks, visual delta without a row): the faded/stopped welcome logo during API-key entry is
missing (pack "Deltas": "Faded welcome logo ... shown animated instead"). The blossom is the
app's own widget, not an rtwk gap: add the fade to `pages/session/blossom` (it already got
`stage`/`fresh` from startup-flow) or record it as a delta kind with the person's agreement.
Notes:
- N-O1: the provider wizards (S03) are not built, yet the host offers them
  (`host/src/startup.rs` `login_policy`: features + target). With a feature on, the menu offers
  "Bedrock/Google Cloud/gateway", Enter lands on an empty page, Esc returns. Set
  `wizards` to none in the host until `providers` lands, or the user hits a dead end.
- N-O2: `webbrowser::open` is a blocking call inside the async host task
  (`host/src/startup.rs`, `Ask::Login`); oracle does the same in a spawned task; use
  `spawn_blocking`. Stale (cancelled) attempts still open the browser.
- N-O3: process order: the `webbrowser` dependency commit came before the red tests (the pack
  admits it).
- N-O4: G48 is correctly a row plus a pending test; `rtwk-gaps.md:74` "Still open" still lists
  "G46 (onboarding)": renumber (G46 import, G47, G48 onboarding; G44/G45 select).
- N-O5: `SignInState::options()` pushes the ChatGPT row even when `offer.chatgpt` is false
  (filtered later in `step`/`Index`); make it one place.
- Secrets: `Login::ApiKey` redacts its Debug; the key sits in the model's `awaiting` until the
  answer: acceptable, clear it on cancel.

## Missing rules (M)

- M1: R15's pattern list (`tests/rules/rules.rs:637`) matches `TextSegmenter`, `.repeat(`,
  `fn *wrap*`, but not `segmenter_of(`, `measure::rows`, `measure::grapheme_rows`,
  `center_truncate_path`, nor `width - N` arithmetic. `trust.rs` and `signin/mod.rs` do hand
  width math and pass. Add `segmenter_of(` and `measure::` to the patterns (allow list:
  `widgets/dots.rs`, `widgets/options.rs`, `pages/agents/tabs.rs`).
- M2: no rule that a view does not hold a scroll offset or window start (`top: usize`,
  `window_top`) - S1 passed all rules. Add a source scan for `fn window_top`/`top: usize` in
  `src/sheets`, `src/pages` outside the allow list.
- M3: no test that every `core` `Command` variant has an arm in the fake backend that is not
  `{}` or a documented "answers none" (`fake.rs` `Command::Startup {} => {}`; import has no
  command at all): a flow can be "wired" in core and silently never answered.
- M4: a gap row must name a probe or a pending test with the same Gxx: enforce
  (`G47` is a row with no probe and not an rtwk limit; `G48` is covered only by the pending test).

## Follow-up tasks suggested

`select-rtwk-list` (S1, N-S1, four-mechanism unification), `select-keymap` (ADR-006 debt),
`startup-deltas` (R-F4, R-F5), `import-rest` (I2, I3), `providers` (N-O1), rules M1-M4.
