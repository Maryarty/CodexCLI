# Core slices: is the core hub the bottleneck, and what would feature slices cost

Task `core-slices-research` (2026-11-23), codex-reviewer. Read-only; measured on `fleet/core-pass` at `ae99b2196` (not yet landed) and `claude/tui-rtwk` at `b3de26c50`. Paths are `codex-rs/tui-rtwk/` on `ae99b2196` unless stated.

## Synthesis

ADR-003 itself is not the problem: one writer per thread and thread RPCs through `ThreadFx` cost nothing measurable. The problem is the **slice granularity**. The core already has a slice mechanism, `screen_list!` with generated dispatch (`core/src/screens/mod.rs:85-316`), but it slices by *screen*, and a screen's functions see only their own part (`screens/mod.rs:279-290`). Chunk features are not screens: `/goal`, `/export`, `/rename`, `/plugins`, `/presets`, review, worktree pickers, permission args are *flows* (slash command -> generic menu/prompt -> app step -> thread note); the queue is *thread state reacting to the turn lifecycle*. Each generic component on that path is a closed enum with a variant per feature, so every feature adds a variant and an arm in 6-10 central places, and needs state its slot cannot see, so its logic lands in `app.rs`/`threads.rs` anyway.

- 14 core-pass features: median 7 shared files changed per feature; shared files carry 34% of non-test lines (2847 of 8370).
- 49% of 288 core commits in the last week touched one of 8 hub files.
- Reading the hub costs ~120k tokens (12.2k lines, 475 KB).
- Conflicts rarer than feared: 23 of 159 merges conflicted, core hub in 7, all append-style (two branches adding a variant / `mod` / `pub use` at the same anchor), none semantic. Cost = serialization, context, core porters editing a UI file — not lost merges.

**Recommendation:** feature slices (option a) on the existing `screen_list!` mechanism: `feature_list!` with core and UI halves; a narrow `AppCx` instead of `&mut CodexApp`; generic menu/prompt/notes carrying generated `Feature*` payloads; lifecycle hooks with default no-ops; ratchet rules so hub enums cannot grow. Framework in one shift while only view tasks run; the ~28 remaining chunk core tasks as slices from the start, parallel by family; migrate existing features when convenient.

## 1. The hub, measured

| file | lines | biggest function |
|---|---|---|
| threads.rs | 3288 | apply_ev :2190 (547); project :1838 (163) |
| app.rs | 2738 | apply_event :961 (530); apply_intent :1817 (216); apply_reply :1615 (202) |
| screens/select.rs | 1676 | apply_choice :1093; wire :1497; decode :1521 |
| screens/chat.rs | 1140 | |
| app/select.rs | 699 | picks :202; answered :522 |
| app/session.rs | 671 | session_intent :86; prompt_submitted :563 |
| commands/dispatch.rs | 605 | |
| src/pages/session/strings.rs (UI) | 1119 | wording of every core note |

Big enums (variants): SessionIntent 43 (fixed by oracle's slash list), select::Note 38, ThreadEv 29, ThreadModel fields 27, ThreadAsk 24 (lifecycle RPCs mixed with feature RPCs), select::Intent 24, Item 23 (essential), Command 19, NoteKey 18, chat::ThreadIntent 17, AppEv 17, Intent 14, select::Picked 14, AppCtx 13, effects::Ask/Done 11/11, select::Menu 10, PromptKind 4.

Commit traffic (7 days): core commits 288; 119 (41%) touch app.rs/threads.rs/screens/chat.rs, 142 (49%) one of 8 hub files. Per file: app.rs 79, threads.rs 64, strings.rs 37, chat.rs 35, select.rs 31, app/select.rs 21, app/session.rs 18, dispatch.rs 17. Fleet merges (21): app.rs in 7, threads.rs 5, chat.rs 4, strings.rs 4.

Conflicts (git merge-tree over every two-parent merge since 2026-11-14): 23 of 159; core hub in 7 (app.rs x4, threads.rs x2, select.rs x1, app/select.rs x1); most conflicted file overall is the UI hub src/pages/session/session_page.rs (x5). All inspected core-hub conflicts were appends at one anchor (62e466eb5, db9a19821, a57636e98, 03630e6bc, d3f1c8b36). core-pass ran alone, so this understates what parallel core porters would hit.

Per feature (hub = 17 shared files incl. strings.rs):

| feature | commit | hub lines / files | own lines | tests |
|---|---|---|---|---|
| provider error | 10e11ba68 | 161 / 9 | 56 | 264 |
| image prep failure | 2d687da1d | 51 / 4 | 3 | 140 |
| inline /review | fee20a7aa | 43 / 3 | 0 | 66 |
| /review popup | 3e5278c4d | 336 / 9 | 262 | 350 |
| catalog provider | 5063dbb71 | 40 / 2 | 21 | 54 |
| worktree pickers | d5264ed4a | 251 / 5 | 338 | 301 |
| /presets | 708906e2b | 349 / 8 | 840 | 608 |
| /plugins | 287598494 | 256 / 7 | 2375 | 643 |
| /rename prompt | 8e833b157 | 164 / 7 | 87 | 239 |
| /export | ee2b52c25 | 132 / 10 | 189 | 303 |
| /permissions args | ee322fdc1 | 298 / 7 | 421 | 777 |
| queue | bb64155db | 167 / 4 | 220 | 431 |
| /goal | 07d3b8623 | 325 / 10 | 539 | 486 |
| collaboration modes | 435a43acf | 274 / 8 | 172 | 310 |
| sum | | 2847 | 5523 | 4972 |

strings.rs is in 9 of 14 core commits: the UI's exhaustive wording match fails to compile on each new Note variant.

## 2. The coupling, named

Structural:
- S1 variant in a generic component's enum (Menu, Picked, Command, Reply, Ctx, PromptKind, NoteKey, select::Note) — /goal added 7.
- S2 arm in the component's own match (apply_choice :1093, wire :1497, decode :1521, modal list :345).
- S3 arm in the app's router for the component's outbox (picks :202, answered :522, session_intent :86, prompt_submitted :563) — Elm's OutMsg/translator problem.
- S4 call in a lifecycle spot (no hooks): goal::after_resume app.rs:2430, :2521; queue in end_turn :2806, interrupted :2846, send_next :3115.
- S5 field in ThreadModel + arms in threads::apply_ev: the slot contract hands a screen only `&mut slots.chat` + a hand-threaded `busy` (screens/mod.rs:23-37), so 15 chat::ThreadIntents are matched in threads::apply_ev (threads.rs:2290-3157); queue fields threads.rs:999-1001.
- S6 wording arm in UI strings.rs (goal_note strings.rs:1027, +106 lines).
- S7 feature RPCs in ThreadAsk (rule thread_rpcs_only_in_threads, tests/rules/layers.rs:423-455) beside the lifecycle RPCs ADR-003 protects.

Incidental: `pub use`/`mod` anchors; features inside crate::app taking &mut CodexApp (/goal writes another slot: app/session/goal.rs:122-128); a feature spread over 3-4 dirs by layer; SessionIntent arms are one-line moves, not growth.

Essential: Item + threads::project (one ordered transcript); ADR-003's writer.

## 3. Alternatives (each keeps ADR-003 and the layers rules)

- **(a) Feature slices on feature_list!** — core/src/features/<f>/ + src/features/<f>/, one registration line generating root enums and dispatch. Solves S1-S7, narrows context (a feature cannot name CodexApp), one folder per feature, hub read drops. Brings: a framework step (~1 shift, serial), fixed hook list, one more indirection, folder rules need features/. ADR-003 kept. **Recommended.**
- **(b) Typed event bus / registry** (TypeId, Box<dyn Any>) — solves S1-S5 but loses exhaustiveness, PartialEq/serde on events (tests, JSONL fixtures), implicit order, unprovable single writer. **Rejected.**
- **(c) Split hub files + generated dispatch** — textual relief only; S1-S6 stay. Complement for project/Item.
- **(d) Keep, parallel core porters merging by git** — throughput now; append conflicts each merge, 120k-token hub read per porter, core porters edit strings.rs, hub keeps growing (threads.rs +302, app.rs +98 in core-pass). Stopgap only for families mapping to their own screen dir.

## 4. The chosen shape on two real features

Registration: `feature_list! { goal => Goal; queue => Queue; }` generates FeatureStates, FeatureThreadParts, enum FeatureIntent/ThreadIntent/Command/Reply/Ctx/Menu/Prompt/Note, apply_intent, apply_thread_intent, apply_reply, wire, decode, on_resumed/on_turn_end/on_interrupted. Contract: `trait Feature` with associated types and default no-op methods (intent, answered, thread_intent, on_resumed, on_turn_end, on_interrupted).

Enabling changes: AppCx (state + read-only AppView + outbox ask/open/prompt/note/thread; no &mut ThreadModel — a thread change is a queued ThreadEv::Feature applied via threads::apply); ThreadModel { core: ThreadCore, slots, features } (split borrow, retires `busy: bool`); generic payloads Menu::Feature, PromptKind::Feature, NoteKey::Feature, UI routing FeatureNote to src/features/<f>/strings.rs (Flutter's showDialog<T>: the opener owns the continuation).

/goal: core/src/features/goal/{mod,menu,wire,note,model}.rs + src/features/goal/strings.rs; tests unchanged. Hub edits: feature_list! +1 and one dispatch stub row (was 10 shared files).

Queue: ThreadPart {items, due, due_seq} out of ThreadModel; ThreadIntent out of chat::ThreadIntent; thread_intent = threads.rs:2448-2542; on_turn_end = send_next; on_interrupted = drain back; fx.start_turn stays ThreadFx.

Rules (tests/rules/layers.rs): hub_enums_only_shrink (ratchet, tests/rules/hub_budget.txt); threads_apply_matches_no_screen_or_feature_intent (15 -> 0); features_are_named_only_through_the_list; features_hold_no_app; thread_rpcs_only_in_threads keeps the lifecycle list only.

## 5. Migration cost and order

- Step 0 framework: one porter, ~1 shift, owns the hub, +600-900 lines, no behaviour change (guard: 1509 core tests + oracle shards). Runs beside the ~16 Wave A view tasks (no core edits).
- Step 1 pilots: goal + queue, ~1 shift.
- Step 2 Wave B as slices from the start (~28 tasks: transcript-2 5, composer 5, status 6, cells 8 [Item/project essential; (c) helps], popups 2, live/usage 3, debts 3), parallel by family; only shared edit is one feature_list! line.
- Step 3 rest (export, rename, plugins, presets, review, worktree, permission args, collaboration modes) 0.2-0.5 shift each when convenient.
- Without step 0: (d) works for status, transcript-2, composer; cells and popups would collide on threads.rs / select.rs.

## 6. Draft ADR paragraph

> **ADR-012: core feature slices.** A feature of the core is a slice: core/src/features/<f>/ (its intents, thread intents, RPCs, notes, app state and per-thread part, and the hooks it needs) and src/features/<f>/ (its words and views). It is registered once in feature_list!, which generates the root enums and their dispatch, as screen_list! does for screens. A slice reads the app through AppCx/ThreadView and changes it only through that outbox. A slice never names CodexApp or ThreadModel. Its per-thread part is a field of ThreadModel, written only by threads::apply under the thread's writer (ADR-003 unchanged). Its RPCs go out through ThreadFx::request. The thread lifecycle RPCs (resume, fork, turns, subscribe) stay in ThreadAsk. The generic menu, prompt and notes carry FeatureMenu/FeaturePrompt/FeatureNote, never a per-feature variant. Enforced by hub_enums_only_shrink, threads_apply_matches_no_screen_or_feature_intent, features_are_named_only_through_the_list and features_hold_no_app. Departures: none; existing features migrate when convenient, and the budget records how far that has gone.

## 7. What the person decides

1. Accept (a) with step 0 now, or run (d) for Wave B and revisit.
2. If (a): step 0 as the next core-pass shift or a separate task.
3. Whether the UI hub session_page.rs (most conflicted file) gets the same treatment (out of scope here).
