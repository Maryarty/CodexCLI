# core-slices early review: ADR-012 framework (step0) and the /goal slice (pilot a8e0f8434)

Reviewer: codex-reviewer, 2026-11-23. Read: ADR-012 (architecture.md:721-), step0 as landed
(merge 66b7c81a7), the pilot at a8e0f8434 (913860d68 /goal slice, 5c8945973 template + codemod),
the pilot's checkpoint (queue design). Ran: core `goal::` / `queue::` tests (27 green), the rules
(78 green), oracle shards chat_commands_2 and chat_popups_1 (no red), two probe tests and five
deliberate breakages (all reverted; nothing of them is committed).

## Verdict

**FIX, then GO.** The framework's shape is right and worth keeping: one list line, generated
roots, an Elm-style outbox, read-only views, rules that bite. Two defects must be fixed before
the queue hooks are built on it (F1 is a user-visible regression today, F2 breaks the hook-order
promise the queue depends on). The queue design in the pilot's checkpoint is **not** GO as
proposed: change it as §5 says (thread effects as an outbox arbitrated by the thread, no hub
`Gesture` broadcast; or keep the queue in `threads/` and drop it from ADR-012).

## 1. Defects

**F1. The goal no longer resumes the queue (regression, no gap row).** 913860d68
`core/src/features/goal/mod.rs:259` replaced `apply_thread(.., chat::ThreadIntent::QueueResume)`
(old `app/session/goal.rs:213-215`) with `cx.settled(&thread)`. `Settled` runs every feature's
`on_settled` (`features/mod.rs:460`); no feature implements it, and the queue is still in
`threads.rs`, so nothing sends the next queued message. Oracle: `app/thread_goal_actions.rs:219`
`maybe_send_next_queued_input()` after the goal is set. No test covered it before or after.
Failing input (probe written and run, then removed): turn running; queue `/goal ship it` and
`hello`; turn completes; `QueueRan(Dispatched{Goal})` (the goal stops the queue,
`commands.rs:1022`); `/goal ship it` -> `Get` -> `Goal(None)` -> `Set` -> `Goal(Some)`.
Expected `StartTurn("hello")`, got none.
Fix (pilot, now): `app/features.rs:76` `Out::Settled` also applies
`ScreenThreadIntent::Chat(QueueResume)` until the queue leaves `threads.rs` (or threads.rs
handles the settled event itself, see §5). Add the probe above to `core/tests/goal.rs` as
`a_goal_set_lets_the_queue_go_on`.

**F2. A later feature's hook does not see a turn start said before it.** ADR-012
(architecture.md:765-768) promises "the queue's `on_turn_end` sends the next message; a feature
after it sees the thread already busy". It cannot: `ThreadFx::start_turn` pushes into
`fx.registered` (threads.rs:1791), which is merged into `outgoing` only after `apply_ev` returns
(threads.rs:2209-2213); `ThreadView::busy()` reads `core.outgoing` (threads.rs:1396). In
`end_turn` the thread's own `send_next` (threads.rs:2896) runs before `feature_turn_ended`
(2899), so already today every feature sees `busy() == false` after a start. Probe in
`framework.rs` (fx.start_turn, then `ThreadCx::new(model.view(), &mut fx).view.busy()`): false.
Two turn-end features that both start a turn (queue + plan implementation / goal continuation)
would send two `turn/start` for one turn end: the double-press class.
Fix (before the queue hooks): see §5, item Q1 (thread effects as an outbox, the thread
arbitrates the start). Minimum if Q1 is not taken: `ThreadCx::busy()` = view.busy() ||
fx has a registered `StartTurn`, `view` made private so `cx.view.busy()` cannot be read raw, and
the framework test that ADR-012 describes (probe A starts, probe B sees busy).

## 2. Risks

| # | Risk | Severity | Fix |
|---|---|---|---|
| R1 | Thread hooks run without the writer. `feature_turn_ended` / `feature_interrupted` / `feature_reply` (threads.rs:2775-2795) run for `Detached` / `Foreign` threads too; only `ThreadEv::FeatureIntent` is gated (2602). ADR-012 says the part changes "only under the thread's writer". Today `ThreadCx` only has `request`; with the proposed `start_turn` a feature on an observed thread could start a turn: ADR-003's "thread already has an active writer" bug. | high once Q1 lands | Gate in cx.rs: every `ThreadCx` effect is a no-op (or not offered) unless `view.has_writer()`; probe test on a `Detached` thread. |
| R2 | Host effects do not fit. `/export` (named in ADR-012) is `effects::Ask::Export` / `Done::Export` (app/effects.rs:69,104), not a `ClientRequest`; `Feature` has only `wire -> ClientRequest`. A Haiku filler following the template hits this on the first feature after /goal. | high for the migration | Decide now: `AppCx::effect(ask: effects::Ask, ctx)` + `Feature::effect_done(state, ctx, Done, cx)`, the hub keeping `effects::Ask` as the host's list of capabilities (write a file, suggest a title), not features. Add to the template's table. |
| R3 | `FeatureView` has no prompt words. The prompt dialog is not built yet (`src/sheets/dialogs/dialog.rs:85`, chunk-commands); its title/placeholder for `PromptKind::Feature` would make that task name `features::goal` in a shared sheet or fail the rule. | medium | Add `FeatureView::prompt_words(&Self::Prompt, &Strings) -> PromptWords` and `view_dispatch` for it now (goal: `Edit goal`, `Type a goal objective and press Enter`). |
| R4 | The generated roots can regrow the hub unseen. `FeatureThreadIntent::Settled` (features/mod.rs:288) and `Out::Settled` are non-feature variants inside generated enums; `hub_budget.txt` does not count them, and the template (§5 of feature-slices.md) invites "its own `Out` and hook the same way" per nudge. | medium | Count `Out`, the non-feature variants of `FeatureThreadIntent` and the `Feature` trait's methods in `hub_budget.txt` (ratchet up only with an ADR line). |
| R5 | Hook growth per interaction. Every cross-feature nudge becomes a trait method on all features; the queue proposal alone adds 6 hooks and 5 effects for one feature. At ~30 features the trait is the new hub. | medium | Rule of thumb in ADR-012: a hook is added only for a thread/app lifecycle fact (turn ended, resumed, input released), never for one producer/consumer pair; see §5. |
| R6 | `Out::Ask` uses an empty `ThreadId` as "about the app" (cx.rs:170, `backend_of` app.rs:834). Illegal state representable. | low | `thread: Option<ThreadId>` in `Out::Ask` and `Command::Feature`. |
| R7 | No slice test harness. `AppCx::new` / `into_out` are `pub(crate)`; slices are tested only through `CodexApp`. | low | Accept (oracle tests run through the app anyway); optionally a `features::testing::run(step) -> Vec<Out>` for Elm-style "which Cmds" tests. |
| R8 | The codemod's shebang `#!/usr/bin/env python3 -I` does not run as `./slice-codemod.py` (env takes one argument on macOS). | note | Shebang `#!/usr/bin/env python3`; docs already say `python3 -I ...`. |

## 3. The checklist

- **ADR-003**: kept for `thread_intent` (gated, threads.rs:2602) and for replies (own tickets).
  Departure without a test for the hooks: R1. Lifecycle RPCs stay in `ThreadAsk`; the goal asks
  left `ThreadAsk` (budget 24 -> 21); feature RPCs go out only through `AppCx::ask` /
  `ThreadFx::request_feature` (threads.rs:1845).
- **A slice cannot name `CodexApp` / `ThreadModel`**: tried. `use crate::app::CodexApp;` in
  goal/mod.rs fails `features_hold_no_app`; nothing reachable from `AppView` / `ThreadView`
  returns them (only `&ThreadCore`, `&ThreadSlots`, `&ScreenStates`, all shared refs, no
  interior mutability). Text scans, not types: accepted, the evasions (alias, glob) still name
  the word or need `&mut` they cannot get.
- **Layers**: core/src/features pure (protocol mapping only, as core does); src/features names
  no protocol. Covered by `the_layer_scans_cover_the_feature_dirs`.
- **Every new rule bites** (real breakage, not only the synthetic bite tests): `ThreadFx` in a
  slice -> `feature_slices_ask_only_through_the_outbox`; `features::goal::Ask` in app/session.rs
  -> `features_are_named_only_through_the_list`; `CodexApp` in a slice -> `features_hold_no_app`;
  NoteKey budget 18 -> 17 -> `hub_enums_only_shrink`; `ClientRequest::TurnStart` in goal/wire.rs
  -> `thread_rpcs_only_in_threads`. 5/5 failed as they should.
- **Hook order**: deterministic (macro expansion order) and documented; the promise about what a
  later hook sees is wrong (F2). App hooks (`on_resumed`) see one snapshot for all features and
  their outboxes are carried after all ran; thread hooks share one `fx`. The two semantics differ
  and neither is written down: pick one (§5 Q1).
- **Exhaustiveness**: kept. Generated dispatch is exhaustive per list; hub enums carry one
  `Feature` arm each; `Never<F>` makes missing components uninhabited and distinct per feature.
  The goal's `(_, Ok(_)) => {}` (goal/mod.rs:269) is inside the slice: acceptable.
- **Re-entrancy / single writer**: holds. `app/features.rs::run` ends the `&mut FeatureStates`
  borrow before carrying the outbox, so a carried `Out::Thread` -> `apply_thread` ->
  `threads::apply` that leads back into `features::resumed` gets a fresh `run`; a thread hook has
  no `AppCx`, so it cannot re-enter the app; work it queues is an RPC answered by a later event.
- **Template + codemod**: enough for /rename-like features (RPC + prompt + note), measured on
  `hub export` (33 lines, readable). Not enough for /export (R2) or anything with host IO, and
  the template's §5 "add your own Out and hook" is the wrong default (R4/R5). A Haiku filler
  needs R2 and R3 decided first.

## 4. The architecture itself (the person's questions)

- **Scale to ~30 features.** Fine. Per line: 10 enums gain a variant, 9 `From` impls, ~14 match
  arms; at 30 that is ~420 generated arms, negligible for rustc. No slice touches a hub match.
  `FeatureCommand` / `FeatureMenu` sizes are the largest variant (box a large payload in the
  slice). The cost that does scale is the `Feature` trait (R5).
- **Two features without naming each other.** Possible through lifecycle hooks; today only
  `on_settled`. Hidden ordering bugs: yes, F2 (stale view between hooks). Fixed by Q1.
- **Single writer under re-entrancy.** Holds (§3); the open hole is hooks on threads without a
  writer (R1).
- **RPCs, prompts, menus, notes without special cases.** RPCs, menus, notes: yes (the goal moved
  with no special case except the host's `asks_thread` / `backend_of` seams, now generic).
  Prompts: core yes, view no (R3). Host effects: no (R2).
- **Testing a slice in isolation.** Through `CodexApp` only (R7); acceptable.
- **The UI side.** Clean for goal: `src/features/goal` words + menu rows, reached through two
  generic calls (`sheets/select/lists.rs:29`, `pages/session/strings.rs:659`). It breaks for a
  feature whose UI is a gesture or a live view in a shared page (the queue: its keys, the
  pending preview, `session_page.rs:446,539`): the rule forbids the page to name the slice, which
  is what pushed the pilot to a hub `Gesture` enum. See §5 Q3.
- **A simpler design with the same guarantees?** For app-scoped flows (/goal, /export, /rename,
  /plugins): no; this is already the elm-spa shape and the outbox is the minimum. For thread
  lifecycle state (the queue, steers): yes, simpler is to not make it a slice (§5 option B).

## 5. The queue design (pilot checkpoint)

The proposal: (a) `hold_submit`, (b) `on_start_refused`, (c) `ThreadCx` `start_turn`,
`give_back`, `recovered`/`set_recovered`, `take_rejected_steer`, (d) hub `Gesture` +
`on_gesture`, (e) generic `pending`, (f) `QueueRan` as a feature thread intent.

Judgement: (a), (b), (e) are queue-shaped hooks on every feature (R5); (c) writes `ThreadCore`
fields, which contradicts `ThreadCx`'s `view: &ThreadCore` (a feature cannot hold a shared view
of the core and mutate it in the same call); (d) renames the queue's intents into the hub and
broadcasts them, so the hub grows again under another name and every feature sees every key;
(e) loses the preview's types (message / command / shell, steers vs queued). The queue touches
~20 sites interlocked with `recovered`, `pending_steers`, `rejected_steers`, `provider_error`,
`last_submit`, `plan.turn_ended(waiting)` and the interrupt drain: it is the thread's input
pipeline, a lifecycle concern like `turn/start`, not a flow.

**Option A (keep the queue in ADR-012), with these changes:**
- **Q1** `ThreadCx` becomes an outbox like `AppCx`: `start_turn`, `give_back`, `request` push a
  `ThreadOut`; threads.rs applies each feature's outbox right after its hook, before the next
  feature's hook, and refuses a second `StartTurn` while the thread is busy (the thread arbitrates
  starts, so no hook order can double-start). Fixes F2; R1's writer gate lives in the same apply.
- **Q2** No `Gesture` broadcast. Let views name a slice's public API: the rule
  `features_are_named_only_through_the_list` allows `features::<f>::{Intent, ThreadIntent}` and
  `crate::features::<f>::` widgets in pages/sheets, and keeps forbidding it in the core hub and in
  another slice. That is Flutter feature-first (a page reads the feature's bloc); the hub
  guarantee (hub enums only shrink, core hub names no feature) is unchanged.
- **Q3** The pending preview is the queue's widget in `src/features/queue/`, composed by the
  session page (allowed by Q2), reading the typed `ThreadPart` through the thread's projection.
- **Q4** `hold_submit` / `on_start_refused` only as lifecycle facts (`on_submit` returning
  `Hold`, `on_start_refused`), each with an ADR line and a probe test, counted per R4.

**Option B (recommended): the queue stays in `threads/` (`threads/queue.rs` already exists);
ADR-012 is amended "thread lifecycle state (queue, steers, turn starts) is the thread's, not a
slice".** Move its five arms out of `apply_ev` into `threads::queue::apply` (the
`ScreenThreadIntent::` ratchet 14 -> 9 all the same), keep `AppCx::settled` as the generic
"input released" event that threads.rs turns into the queue's resume (fixes F1 for good), and
spend the migration on the app-scoped features where slices pay. No new hooks, no `Gesture`, no
`ThreadCx` writes to the core; F2 still needs the minimum fix (or Q1) for future turn-end
features.

## 6. Items

| item | who | when |
|---|---|---|
| F1 settled -> QueueResume + regression test `a_goal_set_lets_the_queue_go_on` | core-slices-pilot | now, before landing |
| F2 busy after a start (Q1, or the minimum fix) + the probe test ADR-012 describes | core-slices-pilot | before the queue hooks |
| R1 writer gate on `ThreadCx` effects + probe on a Detached thread | core-slices-pilot | with F2 |
| R2 host-effect door for /export, added to the template | task (framework), before the migration fleet | before fillers |
| R3 `FeatureView::prompt_words` | core-slices-pilot or chunk-commands | before the prompt view |
| R4 count `Out` / non-feature generated variants / trait methods in the budget | core-slices-pilot | with the next framework change |
| R5 ADR-012 line: hooks only for lifecycle facts | orchestrator (ADR text) | with the queue decision |
| R6 `Option<ThreadId>` in `Out::Ask` / `Command::Feature` | migration, any slice task | low |
| Queue: option A (Q1-Q4) or option B | the person | before the pilot's queue half |
