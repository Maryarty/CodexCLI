# core-slices review: the early-review fixes, queue option B, readiness of the migration

Reviewer: codex-reviewer, 2026-11-23. Scope (narrowed by the orchestrator): what changed since
the early review ([review-early.md](review-early.md), pilot a8e0f8434): d4083c850 (fixes, queue
option B, ADR-012 text) and 0ce652acc (test module moved), landed as 7399a9bad. R2 (host-effect
door) is out of scope (core-slices-effect-door).

Ran: core `goal|framework|queue` tests (77 green), rules (80 green); then six deliberate
breakages at once (reverted, nothing committed): the `QueueResume` of `Out::Settled` removed,
`ThreadView::busy` without `start_said`, `ThreadCx::new` without the writer check, a 15th
`Feature` method, a 7th `Out` variant, a 2nd non-feature `FeatureThreadIntent` variant. Each
failed its own test: `a_goal_set_lets_the_queue_go_on`,
`every_hook_sees_a_turn_start_that_was_said_before_it`,
`a_thread_this_process_does_not_write_gets_no_feature_context`,
`the_feature_trait_has_the_hooks_the_budget_says`, `hub_enums_only_shrink` (Out and
FeatureThreadIntent both listed).

## Verdict

**GO for the pilot as landed** (F1, F2, R1, R3, R4 fixed with tests that fail without the
fix; queue option B keeps ADR-003 and the oracle's queue behaviour).
**Not GO for core-slices-migrate as written**: the template fits /rename only (/export once
R2 lands). The other seven need the decisions M1-M4 below before a Haiku filler can move them.

## 1. The fixes

| item | fixed as asked | test, bites |
|---|---|---|
| F1 goal set resumes the queue | yes: `core/src/app/features.rs:77` `Out::Settled` applies the features' `Settled`, then `QueueResume` as its own `apply_thread`, so the resume sees a start a hook made (outgoing merged between the two). Same guard as the oracle: `answered` drops a reply for a thread no longer active (goal/mod.rs:198, oracle `thread_goal_actions.rs:211`) | `core/tests/goal.rs` `a_goal_set_lets_the_queue_go_on`, red without the line |
| F2 a hook sees a start said before it | minimum fix: `ThreadView::busy` = `start_said` or the model's busy; `start_said` read from `ThreadFx::start_said` in `ThreadCx::new` (cx.rs:251); `core` / `slots` private | framework.rs `every_hook_sees_a_turn_start_that_was_said_before_it`, red without |
| R1 hooks only under the writer | `ThreadCx::new` returns `None` unless `has_writer()`; `threads::with_features` (threads.rs:2757) runs intents, replies, turn end, interrupt through it | `a_thread_this_process_does_not_write_gets_no_feature_context` (foreign, connection lost), red without |
| R3 prompt words | `FeatureView::prompt_words` + `view_dispatch`; goal's `Edit goal` / `Type a goal objective and press Enter` match the oracle (`custom_prompt_view_tests.rs:361`). No caller yet (the prompt dialog is chunk-commands') | none (nothing renders it yet) |
| R4 ratchets | `enum .. Out 6`, `enum .. FeatureThreadIntent 1`, `methods .. Feature 14` in `hub_budget.txt` | `the_feature_trait_has_the_hooks_the_budget_says`, `hub_enums_only_shrink`, synthetic `the_trait_method_count_bites`; all red on a real addition |
| R5 ADR line | architecture.md "What is not a slice": a hook only for a lifecycle fact, raised with a paragraph | (the R4 ratchet) |

## 2. Queue option B

- **ADR-003 kept.** The queue's state stays in `ThreadModel`; `queue::{takes, apply, enqueue,
  send_next}` are `pub(super)`, reached only from `threads::apply_ev` under
  `model.has_writer()` (threads.rs:2487). The five arms moved verbatim (diffed line by line:
  same busy checks, same `suppress_queue_autosend` guards, same `due` handshake); the non-writer
  case falls through to the same no-op arm as before. `ScreenThreadIntent::` arms in `apply_ev`
  14 -> 9 is a move, not a reduction, as option B said.
- **Oracle behaviour kept**: queue tests green; F1's test covers the goal-stops-then-resumes
  path end to end.

## 3. Findings

**Risks**

- **K1 Replies dropped without the writer** (threads.rs:2732, 2757; d4083c850). R1 gates the
  whole step, not only the effects: a feature's thread RPC answered after the thread lost its
  writer to a provider refusal (`has_writer` reads `provider_error`) never reaches the part, so a
  part waiting on it (a spinner, a pending flag) stays stuck. No feature has a thread part today,
  so latent. Fix when the first thread-part feature lands: deliver replies and lifecycle facts
  to the part always, gate only the effects (`ThreadCx` with an effect door that is a no-op, or
  absent, without the writer); keep the Detached probe and add a "reply after provider error"
  one.
- **K2 Start arbitration is words, not code** (cx.rs:243, architecture.md recipe "a start must
  refuse while `cx.view.busy()`"). `ThreadFx::start_turn` (threads.rs:1796) itself refuses
  nothing, and the queue's busy checks read `outgoing`, not `start_said`. Holds today (no
  `ThreadCx` start effect exists); becomes the double-start class the day plan implementation or
  goal continuation adds one. Fix then, or now cheaply: make the refusal part of
  `ThreadFx::start_turn` (returns `Option<Ticket>`, none while `start_said()` or the model is
  busy) with a framework test "A starts, B's start is refused".
- **K3 `ReviewStart` is not a lifecycle RPC in the rules** (tests/rules/layers.rs:424).
  `review/start` starts a turn and switches review mode (threads.rs:1697, 3167), yet
  `thread_rpcs_only_in_threads` does not list `ClientRequest::ReviewStart` / `ReviewStartParams`,
  so a filler moving /review could build it in `features/review/wire.rs` and send it through
  `request_feature`, which `is_start_turn` does not count as busy. Add both to `LIFECYCLE_RPCS`.

**Notes**

- `queue::apply` repeats `takes` and ends in `let .. else return` + `_ => {}`
  (queue.rs:175, 223): an intent added to `takes` and not to `apply` is dropped silently. One
  function `apply(model, intent, fx) -> Result<(), ScreenThreadIntent>` (giving back what is not
  the queue's) keeps the match exhaustive.
- `src/features/goal/strings.rs:124-127`: `prompt` was inserted under `menu`'s doc comment, so
  `prompt` carries both docs and `menu` none.
- R8 not taken: the codemod's shebang is still `#!/usr/bin/env python3 -I` (does not run as
  `./slice-codemod.py` on macOS).

## 4. Is the recipe enough for the migration list?

Each feature of core-slices-migrate against the template (RPC + generic menu + prompt + note +
`on_resumed`):

| feature | fits | what is missing |
|---|---|---|
| /rename 8e833b157 | yes | (`ThreadSetName` is already a feature RPC) |
| /export ee2b52c25 | after R2 | the host effect (`effects::Ask::Export`), core-slices-effect-door |
| /review 3e5278c4d fee20a7aa | menu half only | host effects `GitBranches`/`GitLog` (R2), a live page (M2); the start `ThreadAsk::Review` stays the thread's (K3, M3) |
| worktree pickers d5264ed4a | no | host effects `Worktrees`/`WorktreeRemove` (R2), a live page with stale answers (M2) |
| /plugins 287598494 | no | a live page (`Menu::Plugins` reads `Model::plugins.menu()`), a host-pushed fact (`AppEv` -> `plugins::Ev::Config`, app.rs:1347) (M1, M2) |
| /presets 708906e2b | no | host-pushed `AppEv::Presets` (app.rs:1344) (M1), the editor page (M2) |
| /permissions args ee322fdc1 | menu half only | `HoldQueue` / `queue_held` are the queue's (M3) |
| collaboration modes 435a43acf | menu half only | `SetMask` / `CycleMode` / `switch_mode` / `ThreadFx::modes` are turn settings in `apply_ev` (M3) |
| provider error 10e11ba68 | no | thread lifecycle by ADR-012's own amendment (`provider_error` is in `has_writer` and `send_next`) (M3) |

**Fixes before the migration fleet** (each a small framework or docs task; M1, M2 are code):

- **M1 host-pushed facts.** A door for a fact the host sends unasked (config feed, presets),
  e.g. `Feature::on_fact` keyed by a generated `FeatureFact`, a lifecycle-fact hook under R5
  (one `Feature` method, budget 14 -> 15 with its paragraph). Can go with R2's `effect_done`.
- **M2 live menus.** `Menu::Feature` renders from the menu value alone
  (`src/sheets/select/lists.rs:29` `menu_list(menu, s)`); the typed families get a `Page` built
  from state. A slice whose rows change while open (install progress, stale answers, an editor)
  can only re-`open`, which replaces the stack (loses cursor and tab: a user-visible difference).
  Add `FeatureView::menu_list(menu, &State, words)` or a `Feature::page(state, menu)`, or rule
  these four out of the migration.
- **M3 classify in the brief** what of /review, /permissions args and collaboration modes is a
  slice (the menu and its picks) and what stays in `threads/` (review start, the hold, the
  mask), and drop provider error from the list (ADR-012 "What is not a slice").
- **M4 template step** in `refs/feature-slices.md`: "a feature whose core writes `ThreadModel`
  fields or sits in `apply_ev` is split: the menu half moves, the thread half stays" with the
  /review example, so a Haiku filler stops instead of inventing a `ThreadCx` write.

The orchestrator decides: run core-slices-migrate now with /rename (and /export after R2) only,
or wait for M1-M4 and move the rest in the same batch.
