# core-slices-framework-2: parity pack

Landed at 0c34cc2fd (branch tip 39d308a1c; gated a109e9188: land-check ALL fail=0, scenarios 39/41 with the
known reds error-context-window, multi-agent; fmt-only commit after the gate, fmt clean).
Written by the orchestrator from the task_done deliverable.

No behaviour change: the framework the remaining feature slices need (ADR-012), from the FULL review
(docs/rtwk-port/research/core-slices/review.md).

## What was built
- M1 host facts door: `Feature::on_fact` + `type Fact` through `feature_list!` (`FeatureFact`, `AppEv::Feature`).
  Probe `a_host_fact_reaches_its_feature_and_only_its_feature`; rules allowance FACT_SENDERS =
  host/src/in_process.rs, with bite asserts.
- M2 live menus: `FeatureView::menu_list(menu, &State, words)`; FeatureStates provided to the views
  (src/app/view.rs, debug/seeded.rs); the select dialog watches it; the picker keeps cursor, tab and query
  (tests/pages/feature_menu_live.rs).
- M4 template: refs/feature-slices.md §8 "half a slice" (menu half = slice, thread half stays in threads/;
  table for /review, /permissions args, collaboration modes; provider error dropped from the migrate list),
  plus an ADR-012 paragraph.
- K1: `ThreadCx::new` always, `ThreadCx::request` is None without the writer; replies and lifecycle facts
  reach a part always (probe `a_thread_this_process_does_not_write_hears_the_news_and_has_no_effects`:
  writer, foreign, connection lost, provider error).
- K2: `ThreadFx::start_turn -> Result<Ticket, UserMessage>`, Err when a start was already said in the step
  (`a_second_turn_start_in_one_step_is_refused`); model-busy stays the caller's.
- K3: `ClientRequest::ReviewStart` / `ReviewStartParams` in LIFECYCLE_RPCS, with bite asserts.
- Notes: queue::apply exhaustive (`Result<(), ScreenThreadIntent>`, `takes` removed), goal strings doc
  comments, codemod shebang + executable bit, hub_budget AppEv 18 / Feature 16.

## Tests
core 1527, rules 80, screens+pages+shell 1013 pass; clippy -D warnings clean. New: 3 probes in
core/src/features/framework.rs, tests/pages/feature_menu_live.rs, synthetic asserts in tests/rules/layers.rs.

## Binary
rtwk pty multi-turn, interrupted, plan-tool PASS (turn start, Esc interrupt, queue unchanged). The oracle
binary exits at startup in that environment, so no row-by-row diff; none expected (no UI change).

## Needs
1. The M2 UI test is weak (/goal's rows do not depend on state): the first live slice (/plugins, /presets)
   adds a row-level test beside tests/pages/feature_menu_live.rs (core-slices-migrate).
2. A feature menu opened asynchronously (`Out::Menu` from a reply) is not opened by the UI for non-Select
   commands: block-commands.
3. /export needs a ThreadView/AppView read for transcript text and cwd (core-slices-migrate).
4. effect-door reflection: the ROOT_TESTS exemption in tests/rules/layers.rs.
