# tui-rtwk — how codex-tui is translated onto rtwk

tm: `process-design`. Part 1 is the method: how one screen of the oracle
(`codex-rs/tui`) becomes a screen of `codex-rs/tui-rtwk` — what is read, what
is carried, what is rebuilt and in which order, and how parity is shown.
Every porter and reviewer reads §2 and §3 once. Appendix A is how slices run
through the fleet (branches, review, landing, waves, token economy) — for the
orchestrator and the person.

Sources: architecture.md (ADR-002, ADR-003 R1–R14, ADR-004, ADR-006,
ADR-007), design.md §8, research/codex-tui-architecture.md §2–§3,
research/reuse-map.md, research/nav-proto.md, research/test-infra.md D1–D3.

# Part 1 — the translation method

## 1. What changes between the two programs

| | codex-tui (oracle) | tui-rtwk |
|---|---|---|
| structure | a retained, mutable component tree: `App` (~119 fields) owns `ChatWidget`, which owns `BottomPane` with its composer and a stack of `BottomPaneView`s | a plain model (`CodexApp`, `ThreadModel` per thread) and a view function of it; rtwk rebuilds what changed |
| state | spread over the owners and copied between them (config lives in four places); widgets hold application state | one writer: `apply` / `apply_app` on the UI thread; widgets hold only widget state (scroll, focus, selection, editing) |
| change | one `AppEvent` bus, 335 variants, one `match` in `App::handle_event` | intents and backend events into one ordered ingress; results carry their ticket; effects run by the shell |
| drawing | immediate mode: `desired_height(width)` then `render(area, buf)` into a ratatui `Buffer` each frame | widgets laid out by constraints and painted by rtwk; frame pacing, diffing, virtualization are rtwk's |
| screens | a bottom-pane view stack, an `Overlay` on the alt screen, flags on `App` | `Navigator` + `PageRoute`, `show_dialog`, a request presenter (ADR-002) |

So a screen is not translated line by line. From the oracle we take its
**behaviour** (what it shows in each state, what each key does, what it asks
the backend) and its **pure logic** (parsers, policies, formatters, protocol
helpers). We rebuild **where its state lives** and **how it is drawn**.

## 2. Concept map: codex-tui → tui-rtwk

| codex-tui | tui-rtwk | note |
|---|---|---|
| fields of `App` / `ChatWidget` / `BottomPane` that are application data | `CodexApp` or `ThreadModel` fields, changed only in `apply` | one owner; copies collapse into one field and a projection |
| per-thread composer text, scroll, selection | controllers in `ThreadUi`, keyed by `ThreadId` (R2) | survive a thread switch; never seen by `apply` |
| a view's cursor in a list, filter text, focus, hover, animation phase | rtwk widget `State` / controllers (`SelectionController<K>`, `ScrollController`, `TextEditingController`) | rtwk owns widget state |
| values recomputed and stored (counts, filtered rows, labels) | functions in the view, kept by `ViewCx::memo` on `(rev, Rc<T>)` (R7) | never kept in sync by hand |
| `RefCell` render and layout caches, `LayoutCache` | gone | rtwk lays out and caches |
| `AppEvent` "open / show / select" (UI intents, ~109) | the widget acts directly: `Navigator::of(cx).push`, `show_dialog` | no event, no model change |
| `AppEvent` that changes shared or cross-thread state | an intent value through a `Handler` into the ingress (R10) | apply order = arrival order |
| `AppEvent` "*Loaded / Finished / Created" (async results, ~66) with an id such as `model_popup_request_id` (chatwidget.rs:560) | `ThreadEv` / `AppEv` carrying its ticket; `apply` drops a stale one (R1) | the oracle's ad-hoc ids become one mechanism |
| `AppEvent::PersistX` → `ConfigEdit` → `ConfigBatchWrite` | intent → effect (config write) → confirmation event | local change waits for the confirmation (R9) |
| refresh / fetch events, `tokio::spawn` from a view | effects in `ThreadFx` / `AppFx`, run by the shell; data as `Remote<T>` (R6) | no spawn outside the shell (R1 lint) |
| `AppCommand`, `AppServerSession` | carried (REUSE), driven by the shell | the backend seam is kept |
| `AppEventSender` held by a view; `SelectionAction` closures over it (list_selection_view.rs:125) | the dialog returns a value (`close_dialog`, `Navigator::pop_with`); the opener's `Handler` turns it into an intent with its ticket | views hold no sender |
| `FrameRequester::schedule_frame`, frame rate limiter, size monitor | nothing | a model change reruns the view |
| `Renderable::render(area, buf)`, `desired_height(width)`, `Rect` arithmetic | a widget's `build`: `Column` / `Row` / `Flex` / `Padding` / `Stack` under constraints | the measured results (what fits, what truncates) are behaviour, checked by snapshots |
| `ColumnRenderable` / `FlexRenderable` / `InsetRenderable` | `Column` / `Flex` / `Padding` | |
| ratatui `Line` / `Span` / `Style`, `secondary_text_style()`, `accent_style()`, `status_style()` | rtwk `Text` / `Span` with tokens of `theme.rs` | the oracle palette is mapped once, in the theme; no colours in screens |
| `HistoryCell::display_lines(width)` (history_cell/mod.rs:210), `markdown_render/`, `diff_render.rs`, `wrapping.rs`, syntect | a cell widget per kind over the transcript item: rtwk-rich `MarkdownView`, `DiffView`, `CodeView` | not carried (ADR-004); differences are deltas |
| `StreamController`, commit ticks, `chunking.rs` | the live tail item (one, at the end of the `ListView`), settled items as ordinary items; chunking carried where its pace is visible | ADR-004 |
| `TranscriptView`: anchors, follow-latest, selection, search | `ListView::builder` + `ScrollController` (stick to bottom), `SelectionArea` | selection over a virtualized list is gap G9 |
| `BottomPaneView` on the view stack; `is_complete()` / `completion()` → `ViewCompletion::{Accepted, Cancelled}` (bottom_pane_view.rs:16) | `show_dialog` from the asking widget; `close_dialog(Some(v))` / `None` | |
| `Overlay` entered with `enter_alt_screen` (pager, diff, analytics) | a `PageRoute` pushed over the chat screen, reading live data with `select` / `watch` | |
| views for server requests (`approval_overlay.rs`, elicitation, user input) opened from `handle_server_request` | the request presenter under the `Navigator` opens `show_dialog` for the head of the model's request queue; the dialog closes when the model drops the request | the model's queue is the only source |
| `handle_key_event(KeyEvent)` `match` arms | `Shortcuts` / `KeyBindings` actions named after the keymap's actions; text keys to `TextArea` / `TextField`; `Focus` for order | the keymap slice reads `tui.keymap.*` into them (ADR-006) |
| `ChatComposer` + Codex textarea | rtwk `TextArea` over the `ThreadUi` controller; pure composer logic (paste-burst detector, mention codec, history store) carried | missing editing behaviour is gap G12, listed as deltas |
| `Instant::now()` in a view | the clock as a model input (R11) | |
| insta snapshot tests (`render_lines_with_width`-style helpers) | screen tests: a `Tester` at the oracle's size, compared through the replay helper with the oracle's `.snap` | test-infra.md D1 |

## 3. Translating one screen: the steps

In this order; the slice's commits follow it.

**1. Read the oracle as a specification.** From the screen's registry rows
(`refs/codex-registry.csv`) and the files they name, answer four questions and
write the answers into the parity pack's skeleton:
- *What does it show?* Every state, from its snapshots (`.snap`) and the
  tests that make them; the sizes they are taken at.
- *What does each key do?* The `handle_key_event` arms and the tests that
  press keys.
- *What state does it read and write, and who owns it?* Follow each field to
  its source of truth; a copy (config in `ChatWidget`, rows built into a view)
  is not state, its source is.
- *What does it ask the backend, and what comes back?* `AppCommand`s sent,
  `AppEvent` results received and the ids that guard them.

**2. Sort the code** of the slice's files by `research/reuse-map.csv`, with
architecture.md §0 over it:
- *Carry* pure logic: REUSE rows and the non-UI part of ADAPT rows (no
  `Buffer`, no `render`, no crossterm event handling). A carried file brings
  its dependency closure: each import is carried too or replaced by an rtwk
  counterpart. Copied code keeps a comment naming its oracle file.
- *Rebuild* everything that draws, measures, or handles terminal events, and
  every place state lives.
- *Drop* what rtwk does: frame scheduling, terminal diff, size monitoring,
  render caches, painted-rect registries.
- *Never carry* the content renderers (history cells' line rendering,
  markdown_render, diff_render, wrapping, the textarea): rtwk-rich and
  `TextArea` replace them.

**3. Turn the oracle's tests on.** The slice's oracle shard
(`tests/oracle/<shard>.rs`, `kit::pending` markers, expected values = the
oracle's `.snap` text and assertions) already holds the parity tests: the
slice turns them on as it builds and the shard's counts are the evidence.
Each oracle test of the slice's files is mapped by `shards.py --check`:
- snapshot test → green in the shard, or `pending(Gxx)` on an open gap row;
- logic test → a state test: events into `apply`, model and effects out; a
  test of carried logic is carried nearly verbatim;
- vt100 scenario → the page's pty scenario, both binaries (D3);
- a test of a replaced renderer's internals → n/a "renderer replaced,
  ADR-004"; what the user sees of it is covered by a screen test.
Tests that are not in the shard (state tests, mouse tests) are written before
the code they cover; each is shown biting once (red before green, or the code
broken and restored), named in the handover.

**4. Place the state.** Every field found in step 1 goes by the first three
rows of §2: model (changed in `apply`), `ThreadUi`, or rtwk widget state;
derived values become functions; caches go. Every `AppEvent` the screen sends
or receives is classified by the `AppEvent` rows of §2. Then write the `apply`
arms, the effects, and extend the mandatory tables (R2, R4, R5, R7, R8,
R9/R10) for each new event, intent or lifecycle state. The state tests turn
green here, before any widget exists.

**5. Build the view from the snapshot, not from `render()`.** Mark the
regions of the oracle's snapshot (header, list, footer hints, …), pick the
rtwk widget for each (§2, design.md §8, `refs/`), compose them with layout
widgets. The oracle's `desired_height` and `Rect` arithmetic is not
translated; its results — what fits at narrow widths, what truncates, which
rows hide — are checked by the screen tests at the oracle's sizes. Styles
come from theme tokens. Screen and dialog widgets are
`impl_widget!(stateful eq …)` and read data only through providers
(`select` / `watch`) or ids passed in (R14).

**6. Wire keys and focus.** Each `handle_key_event` arm becomes a
`Shortcuts` / `KeyBindings` action named after the keymap's action; text
keys go to `TextArea` / `TextField`; focus order is the oracle's; Esc is
back. Completion (`Accepted` / `Cancelled`) becomes `close_dialog(Some(v))`
/ `None` or `Navigator::pop_with`.

**7. Place it in navigation** by where the oracle shows it: a bottom-pane
view → `show_dialog` from the widget that asks; an alt-screen overlay → a
`PageRoute`; a server request → the request presenter; a part of the chat
screen → the chat subtree; a mode (backtrack) → state of the chat screen.
Routes and dialogs get ids; only an editing dialog copies its value as a
draft and returns it. The four ADR-002 conditions hold.

**8. Compare and list the deltas.** Screen tests green except listed deltas;
each delta with both renderings, its kind from `parity/delta-kinds.md` or
NEW, and its cause (renderer, gap, layout). A gap in rtwk is a delta and an
"rtwk needs" line, not a workaround.

## 4. Worked example: S14, the generic selection dialog

Oracle: `bottom_pane/list_selection_view.rs` — `ListSelectionView`, a
`BottomPaneView` opened by ~30 callers with different rows.

1. *Read.* Rows are `SelectionItem` (:170): name, description,
   `is_current`, `is_disabled`, a toggle, `pending_toggle_actions` (:179,
   "toggle actions sent from this row that have not finished saving yet"),
   and `actions: Vec<SelectionAction>` (:188), closures over the
   `AppEventSender`. Keys: `handle_key_event` (:1299). Done:
   `is_complete` / `completion` (:1478, :1482) → `ViewCompletion`. Drawing:
   `desired_height` (:1561), `render` (:1636). Snapshots:
   `bottom_pane/snapshots/codex_tui__bottom_pane__list_selection_view__*.snap`,
   rendered by `render_lines_with_width` (:2111) at fixed widths.
2. *Sort.* `render`, `desired_height` and the row painting are rebuilt;
   pure helpers the rows use (matching, formatting) are carried if they are
   separable.
3. *Tests.* Each `list_selection_view` test and snapshot becomes a screen
   test at its width; the callers' own snapshots go to the callers' slices.
4. *State.* The rows are not dialog state: the opener passes an id of what
   is chosen (a setting, a model list) and the dialog reads the rows through
   a projection. `pending_toggle_actions` becomes a model fact — a change
   pending until the backend confirms (R9) — so "saving" shows from the
   model, not from a counter in the view. The highlighted row, scroll and
   filter text are rtwk state (`SelectionList<K>`'s controller, keyed rows,
   R11). The model-list request guarded by `model_popup_request_id`
   (chatwidget.rs:560, model_popup_state.rs:58) becomes a ticket in
   `outgoing`.
5. *View.* `Dialog` + `SelectionList<K>` + a `TextField` for search + the
   footer hints, laid out as the snapshot shows.
6. *Keys.* The arms of :1299 → `KeyBindings` actions; the digit shortcuts
   (`display_shortcut`) are bindings too.
7. *Navigation.* `show_dialog` from the widget that asks; the dialog closes
   with the chosen key; the opener's handler sends the intent with its
   ticket — the closures over the sender are gone.
8. *Deltas.* Whatever `SelectionList` draws differently from Codex's rows
   (markers, column widths) is a delta of a kind decided once for all ~30
   callers.

## 5. What the translation never does

- No ratatui painter, no `Buffer` bridge, no wrapping of a codex-tui widget;
  `render()` is never translated line by line.
- No state in widgets beyond widget state; no sender in a view; no spawn
  outside the shell.
- No re-implementation of what rtwk owns (frames, coalescing, layout,
  virtualization, scroll, focus, editing).
- No copy of the oracle's mechanics. The spec is the oracle's behaviour and
  look, never how it gets there: its scroll offsets, selection indices, edit
  buffers, width and truncation helpers are ratatui-era plumbing. For every
  mechanism, first the rtwk widget or controller that does it
  (`SelectionList`/`SelectionController`, `ListView`/`ListController`,
  `TextField`/`TextEditingController`, `TabBar`, rtwk-unicode widths and
  `TextOverflow`); only what rtwk lacks is ours, as a small function held to
  the oracle's tests. A brief or research note that names a mechanism rtwk
  already owns is wrong: follow rtwk and say so in the handover (the person,
  2026-11-18).
- No workaround for an rtwk gap. A gap is real only after the rtwk widget
  ran against the oracle's rows; then a one-line row in `rtwk-gaps.md` (Flutter way,
  rtwk widget tried, what differs) and the test marked `pending(Gxx)`; the
  slice is handed over with it. The orchestrator tells the person, who
  decides whether rtwk gets it (the person, 2026-11-18).
- The boundary between composition and workaround (the person, 2026-11-21).
  Composition: rtwk widgets lay out and paint; the app only chooses WHICH
  widget or spelling to build from a width it measured (`LayoutBuilder` +
  `segmenter_of(cx)`, a branch, a `Row`/`Grid`/`ListView` of widgets).
  Workaround: the app cuts, rewraps, pads or re-measures strings itself and
  hands rtwk the result. Composition is the Flutter way and needs no row;
  a workaround is a gap row + `pending(Gxx)`, never code (G48 stays pending).
  Worked pair: `widgets/dots.rs` `dots_end` and `pages/agents/tabs.rs` are
  compositions (they measure, then lay out widgets); `pages/startup/prompts/
  panel.rs` `words()` is a workaround (a hand wrapper for G48). A landed
  exception (G37 `pages/agents/path.rs`) is a closed row with its oracle
  tests, not a precedent.
- Before a gap row, a quick check in your head (the person, 2026-11-21):
  how would Flutter do it, then find the same widget or a composition in
  rtwk (the skill, grep `src/widgets`, `src/pages/agents`). That is all: no
  probe files, no file:line per route, no review rounds about gap rows. A
  gap row is one line: the Flutter way, the rtwk widget tried, what differs.
  The skill's cookbook, "When the one widget differs from the oracle", lists
  the compositions found so far. `pending(Gxx)` stands only on an open row.
- No edit of codex-tui: it stays the oracle.

# Appendix A — running the slices through the fleet

Prior art: rtwk's landing process (`rtwk/docs/process/landing.md`: one
maintainer integrates, one host check per candidate, retry a flake once) and
its measured fleet cost (`~/.cache/rtwk/process-cost/report.md`, 185 agents).
What carried over and why is said where it applies.

## A1. Roles

| who | does | never |
|---|---|---|
| the person | accepts tasks in tm (review mode `human`); decides delta kinds, architecture changes, scope, refs bumps, anything outward (§A9) | — |
| orchestrator (Opus, main session) | keeps the front moving: writes briefs into tm, launches agents, rejects in tm on a FIX verdict, builds and lands candidates, rebuilds the registry, runs the wave-end checklist, hands over | writes slice code; accepts in tm; pushes |
| `codex-porter` (Sonnet, worktree) | one tm task: tests first, the slice, the parity pack, `task_done` | edits outside its area; merges; touches codex-tui |
| `codex-reviewer` (Opus) | a FULL or wave review, architecture, research that picks a direction; verdict + findings | writes in the repository; accepts or rejects in tm |
| `codex-light-reviewer` (Sonnet) | a LIGHT review, a short fact-finding question; verdict + findings | writes in the repository; accepts or rejects in tm |
| `codex-waiter` (Haiku) | one long command: a candidate check, the scenario suite, a perf run, a cold build | edits, commits, tm writes |

## A2. Branches and worktrees

- **Integration branch `claude/tui-rtwk`**, cut by the orchestrator from the
  W0 head when W1 starts; the main checkout sits on it. `claude/tui-rtwk-w0`
  stays as W0's record. `main` follows `upstream` and is not touched in
  phase 1; nothing is pushed without the person.
- **One worktree per porter**: the Agent tool's `isolation: "worktree"`
  (`.claude/worktrees/<name>/`, a branch cut from the integration tip at
  launch). `codex-rs/tui-rtwk` reaches rtwk by `../../../rtwk`, resolved there
  by the symlink `.claude/worktrees/rtwk` (README.md). Each worktree has its
  own `target/`; compilation is shared through sccache in `acargo`.
- A fix round reuses the slice's worktree and branch. A worktree is removed
  after its branch has landed.
- **Landing worktree** `.claude/worktrees/land`: candidates are built there
  (§A6), so the orchestrator's checkout is never in a half-merged state.
- **The oracle binary** is built once per `ORACLE_REF` by a waiter
  (release, into `~/.cache/codexcli/oracle-bin/`) and used by every pty
  scenario and perf run; no worktree builds codex-tui.

## A3. A page, from tm to landed

```
pending ─ page plan + brief in tm (§A4.1) ─ porter claims in a worktree
  ─ oracle shard turned on, state and mouse tests first, the page green
  ─ rebase onto the integration tip ─ full check once (§A4.2)
  ─ the binary driven beside the oracle (§A4.3) ─ task_done → in_review
  ─ landing (§A6): fast-forward when the tip has not moved, else one candidate
  ─ review per tier (§A5): LIGHT after landing, FULL before
       FIX → the same task, re-granted in the same worktree
  ─ the person accepts in tm (now or at wave end)
```

**One page or feature, one porter, end to end** (the person, 2026-11-21):
core state, views, host/services, mouse, the oracle shard and the pty
scenario of a page are one task, and its review findings come back to the
same task. No "-rest", "-deltas" or "-fix" tasks cut by finding type.

**Landing does not wait for the person's acceptance.** The person's gate is
the wave end. A page the person rejects after it landed is fixed forward in
the same task, never reverted unless the integration branch is broken.

### Size
A porter run fits ~100 turns / ~250k context. A page larger than that is
continued from its `task_checkpoint` by a fresh porter on the same task
(the same worktree, the same brief). A feature too large for one porter at
a time (chat cells, settings) is split at planning time by disjoint files
(§A7 area map), never by finding type.

## A4. What a slice carries in and out

### A4.1 Brief (the task's description in tm)

The prompt of a porter is only: task alias, worktree, "read your task". The
brief lives in tm. Every name and path in it is grepped first.

```
Goal: <page / feature> — <screens S..> — <one sentence of behaviour>.
Oracle: shard(s) <names> (tests/oracle/…); registry rows <ids>; beyond them
        <file:line ranges>.
Architecture: <sections by heading>. ADR departures: none | ADR-xxx: <what>
        → debt task <alias> (a departure not named here makes the brief
        invalid; the porter stops and reports).
State: core parts read and added; events/intents in the page's own sub-enum.
rtwk: per mechanism the widget/controller (named API); shared widgets to use
      (`widgets/options.rs`, `widgets/session_list.rs`, …).
Area: <paths>. Shared files and their owner: <file → this task | task X
      (read only) | append via pack section>. Out of scope: <…>.
Done: shard <counts expected>; mouse tests for every selectable view; pty
      scenario <alias>-<name> run on both binaries; review tier
      TRIVIAL | LIGHT | FULL (why).
Known reds: ~/.cache/codexcli/known-red.txt (do not re-verify them).
```

Before a brief goes into tm the orchestrator re-reads it for wording that
invites what the architecture forbids ("state in the widget", "data from
mock.rs into the widget", "store the controller in the app model"). It also
checks that no running task owns a file in the brief's area. An agent builds
what the brief says; a boundary the brief leaves implicit is not built.

### A4.2 Tests and checks — scoped while working, full once

- Expected values come from the oracle (shard, `.snap`, assertions, captures
  of the oracle binary), never from the new code's output.
- While working: `acargo nextest run --test screens -E 'test(<page>)'
  --status-level fail --final-status-level fail --hide-progress-bar` (one
  test root, fleet brief) and `acargo clippy -q --all-targets -- -D warnings`;
  `--test rules` before each commit: the rules tests are where the
  architecture and the review checklist live.
- A partial (`partial ready: <tip>`) carries these scoped checks only; the
  candidate check (`land-check.sh`) is its full gate, and a red there goes
  back to the same porter. The full gate below runs once, before task_done.
- Once, before `task_done`, after rebasing onto the integration tip:
  `acargo fmt -- --check`, `acargo clippy -q --workspace --all-targets --
  -D warnings`, the full `acargo nextest run --workspace`, the page's oracle
  shard(s) under `--profile oracle`, `shards.py --check`, the page's pty
  scenario (§A4.3).
- Known reds live in `~/.cache/codexcli/known-red.txt` (test or scenario
  name, tm bug). A red on that list is not investigated again; a red not on
  it is the porter's, unless the porter shows it red on the tip without its
  change (one run, named in the handover).
- Foundation tasks without snapshots (session-core) write the ADR-003
  acceptance tests first: late reply dropped, switch mid-stream, resize
  mid-stream, approval during a dialog.

### A4.3 Evidence — the parity pack and the binary

`docs/rtwk-port/parity/<alias>.md`, at most ~40 lines:
```
# <alias> — parity
Shard counts: <shard>: green n / pending n (Gxx…) / n/a n   (from shards.py)
Decisions: departures with their debt task; n/a reasons; gap rows added
           (one line each: Flutter way, rtwk widget tried, what differs)
Binary: scenario <alias>-<name>, both binaries, checkpoint <name>:
        the differing rows of the two captures, or "equal"
Mouse: the views and their mouse tests
Rows for the lander: shard registration, gap rows (when another task owns
                     rtwk-gaps.md / shards.py)
```
- No delta is accepted (the person, 2026-11-18). A difference is fixed, or
  is an open gap row with `pending(Gxx)`.
- The porter drives the page in the real binary before handover: its own
  build (`codex-rs/tui-rtwk/target/debug/codex-rtwk` in its worktree) in the
  page's pty scenario against the oracle binary
  (`~/.cache/codexcli/oracle-bin/`), with `CODEX_HOME=$(mktemp -d)` for
  first-run pages, keys and the mouse on the main path.
- The registry is rebuilt by the orchestrator at wave end.

### A4.4 Rules every slice is held to

- **ADR-002 binding conditions** (nav-proto.md): (1) screen and dialog
  widgets `impl_widget!(stateful eq …)`; (2) the request presenter and every
  `show_dialog` caller under the `Navigator`; (3) the provider list fixed, the
  `Navigator` keyed by a `GlobalKey`; (4) the handlers provider
  `.update_should_notify(|_, _| false)`, no decision from a same-flush `read`,
  the presenter never reopens a shown id. Routes and dialogs take ids (R14).
- **ADR-003 R1–R14**: lints come from the skeleton's `clippy.toml`; R14's
  controller rule is as in Flutter (tests in `tests/rules/rules.rs`): a
  controller (`ScrollController`, `ListController`, `SelectionController`,
  `TextEditingController`, `TabController`, `FocusNode`, …) is made in a
  widget's `State` (`create_state` / `init_state`) and goes with it, never in
  `build`/`view`/a builder closure, never in core; no allowlist, no
  `::default()` workaround needed; an
  `#[allow(clippy::disallowed_…)]` outside the allowlisted modules is a
  defect. A slice that adds events, intents or lifecycle states extends the
  mandatory tests: switch round-trip (R2), resume/reconnect snapshot (R4),
  end-path matrix (R5), `assert_consistent` after every event (R7),
  lifecycle × intent table (R8), event permutations (R9, R10).
- **Conflict hygiene for parallel slices**: the skeleton registers a module
  stub for every screen and a root variant per screen in the event and intent
  enums, so a slice adds variants only to its own sub-enum and never edits
  `lib.rs`, a parent `mod.rs` or a root enum. A new dependency is a separate
  commit (`Cargo.toml` + `Cargo.lock`) named in the handover; the orchestrator
  resolves lockfile conflicts at landing.
- **Layers** (architecture.md "Layers", task `skeleton-layers`): the core
  crate `codex-tui-core` has no rtwk dependency — models, `apply`,
  tickets, protocol mapping, command parsing; the UI crate's model is
  `UiApp { core, ui }`, UI-only state (navigator key, controllers, language,
  debug) in `ui`; screen dirs hold view files only (widgets, `strings.rs`,
  debug entries) and never name `codex_app_server_protocol`, `shell`, tokio or
  std IO; `shell` runs effects and names no widget. The compiler (core's
  dependencies) and `tests/rules.rs` enforce it; a new boundary comes with its
  rule shown biting once. Mock and fixture state is a JSONL event script
  applied through core `apply` (oracle test data, test-infra.md §D), so a
  screen's code is final from its first commit.
- Carried oracle code keeps a comment naming its oracle file. codex-tui, the
  codex-rs workspace manifest and lockfile stay untouched.

## A5. Review

| tier | what | review | landing |
|---|---|---|---|
| TRIVIAL | markers, docs, packs, gap rows, renames | none: the orchestrator checks the diff scope (no `src/`, `core/`, `host/`, manifest change) | fast-forward or the next candidate |
| LIGHT (default for pages, sheets, dialogs, StartupPage, settings, pickers) | one page's views, core state and host/services | one `codex-light-reviewer` pass over the page's diff, after it lands | does not wait |
| FULL | the risky list: session-core, chat-cells' streaming tail, composer, transcript, approvals, pager, inline mode, perf claims | `codex-reviewer`, before landing | waits for the verdict |
| wave | the wave's diff on the integration branch | Opus: drift from the architecture or rtwk's standard means; the R-rule audit | the next wave |

A review returns FIX only for:
1. a defect, with its failing input;
2. an ADR violation without a debt task: ADR-003 state, a layer crossed
   (where no rule catches the crossing, the missing rule is a finding too);
3. a user-visible difference from the oracle with no open gap row;
4. work outside the area, or codex-tui / the codex-rs workspace touched.

Checklist line of every review of a diff that adds a field, a variant or a type to core's domain
(`CodexApp`, `ThreadModel`, a screen's or a feature's state): the semantic half of ADR-013's IO
test, which no scan can decide (the rules `adr013_*` see names and types, not the question).
For each added value ask in order, as full §1.3 does: is it a server/host fact or derived from one
(core); the key of a request or the status of an operation (core, by domain key); a lifecycle step or a
one-shot domain signal (core); an interest counter (core)? If none: is it a *key or a cursor* — the
parameter a late answer is matched by (`query`, `PluginKey`, `cwd`), or a position that only the
gestures of one surface write (`cursor`, `selected`, `tab`, `page`, `expanded`, `open`, the text of a
field before submit)? The second is the widget's `State`, a rtwk controller or `ThreadUi`; in core it
is a finding under item 2, even when the scan passes it (a renamed `cursor` is still a cursor).

Everything else is a note: structure no rule catches yet, pack or gap-row
wording, style. A note goes to the page's next task or to the wave's
follow-up. A structural finding a rules test could catch also goes to the
wave's rules task. Gap rows are one line, and a reviewer never asks for
probes or routes.

On FIX the orchestrator rejects in tm with the findings. The same task is
re-granted in the same worktree, and the porter answers each finding by
`path:line` and commit. A re-review reads only the fix commits. A second
FIX on the same page is a stop (§A9).

## A6. Landing on the integration branch

> **ADR check at landing (the person, 2026-11-19).** Before a candidate lands the
> orchestrator checks the slice against each ADR it touches (state ownership
> ADR-003 above all: one CodexApp, one ThreadModel per thread, one writer, no
> screen-owned copy of thread state or its own RPC path around it). A departure
> without a named debt task in tm blocks the landing.

One lander: the orchestrator; long checks by a `codex-waiter`.

1. **Tip unchanged since the porter's rebase**: the porter's full check
   covered this exact tree. The orchestrator checks the diff scope (area,
   codex-tui untouched) and the ADRs, then fast-forwards `claude/tui-rtwk`.
2. **Tip moved**: in `.claude/worktrees/land`, a branch at the tip; merge
   the page branches that are ready (`--no-ff`, the merge names the alias).
   A conflict in the page's own files goes back to its porter (rebase).
   A lockfile conflict: the orchestrator regenerates it.
3. Candidate check by the waiter: `~/.cache/codexcli/land-check.sh <worktree>
   <tip>`. It compares failures with `~/.cache/codexcli/known-red.txt`:
   green = no new red. A new red is retried once; a pass on retry lands and
   goes on the list with a tm bug; a second failure splits the candidate.
4. Green: fast-forward; remove the landed worktrees.

Landing is continuous: a page lands when it is handed over and its check is
green, never held for a round. FULL pages land after their review.

The skeleton asks one thing more of itself for this: the CI-grep parts of R1,
R14 and the `unchanged()` ban as a test (`tests/rules.rs`), so nextest
enforces them in every worktree and candidate without a separate script.

## A7. Pages in parallel

Limits: **at most 3 concurrent cargo builders** on this machine (a
land-check counts as one; load beyond that made every build and timing test
slower, 2026-11-21: load 139 on 10 cores), the orchestrator's landing
throughput, the person's acceptance. **At most ~5 porters in their build
phase at once**; more porters only while they read or plan.

Before a page or wave fans out, the orchestrator writes an **area map** into
the parent task: each running task's files, and each shared file with one
owner (`widgets/options.rs`, `core/src/screens/<page>.rs`, `host/src/<page>.rs`,
`rtwk-gaps.md`, `shards.py`, `Cargo.lock`). A shared widget two pages need is
built first by its owner, or the second page starts after it lands. Two
porters never share a page. Append-only files are written through the pack's
"Rows for the lander".

Process changes the person decides apply to tasks handed out after them;
running tasks finish under their brief unless the person says otherwise.
The orchestrator batches them into one TRIVIAL docs commit at a page
boundary.

## A8. Every wave end: the checklist

1. Every task of the wave landed and reviewed; open notes in one follow-up
   task.
2. Full candidate check + the whole scenario suite on the integration tip
   (waiter).
3. Registry rebuilt; delta-kinds catalog brought up to date; the wave's NEW
   kinds listed for the person with both renderings side by side.
4. **Perf trend** (ADR-008), on a quiet machine — no porter builds, load
   under the harness's gate — so it runs here and nowhere else: the scenarios
   the wave made meaningful (W2: S1, S2, S5, long scroll, resize storm, as
   perf-chat; W3: pickers and pager; W4: all 11, both modes) A/B against the
   oracle baseline, appended to the trend, `report.html` rebuilt. A scenario
   whose p99 regressed beyond the baseline's noise band without a recorded
   reason is a stop (§A9). No number reaches the person from a loaded machine.
5. **Bug-class proof** (state-bug-classes.md, `reports/tui-bug-classes.html`):
   the wave reviewer audits each R-rule → its enforcement in code (lint entry,
   test name, type) → present / not yet, and each sub-class counted "caught"
   → the mandatory test exists. The audit goes into the report's data
   (`reports/data/`) and the page is rebuilt (`build_report.py`), so the
   20 / 33 / 2 claim is backed by code at every wave, not only by design.
   Every state bug found in the port during the wave is classified against the
   SC taxonomy and added; one in a class counted "impossible" is a stop.
6. Opus wave review (§A5).
6a. **Oracle move** (the person, 2026-11-18: at every wave end, not frozen
   for phase 1; upstream codex-tui runs ~14 commits a day). Fetch upstream,
   move `ORACLE_REF` to its tip, rebuild the oracle binary (waiter). List the
   upstream changes in `codex-rs/tui` since the previous ref per screen
   (registry rows): a landed screen whose oracle files changed gets a
   re-parity follow-up task; a pending screen's brief is updated before
   hand-out. Retake the oracle perf baseline so the next trend compares with
   the current original. Screens upstream is still reworking go last in
   their wave.
7. The person: accepts the wave's tasks in tm; decides the NEW delta kinds;
   sees the perf report and the refreshed bug-class page.
8. Orchestrator hand-over: a fresh session per wave (§A10).

## A9. When the orchestrator stops and asks the person

Stop and ask (the work that depends on the answer waits; the rest goes on):
1. A deviation from the architecture: an ADR-002 condition that cannot be
   met, an R-rule that needs an exception, a module structure the
   architecture does not have.
2. A bug found in a class the proof counts "impossible".
3. A slice cannot finish without an rtwk change (rtwk is another repository
   with its own orchestrator): after research shows no exact route, a
   `rtwk-gaps.md` row for the person to hand over; the slice waits or uses
   the closest exact workaround.
4. Dropping an oracle behaviour outright (more than a visual delta), or
   anything outside phase 1.
5. A NEW delta kind that blocks a slice (otherwise: batched at wave end).
6. A perf regression beyond the noise band at a milestone.
7. A third fix round; a task failed twice (tm `maxAttempts` is 3); a
   candidate red twice for reasons outside the slices (flakes,
   infrastructure).
8. Moving `ORACLE_REF` mid-wave (the wave-end move of §A8 6a is decided)
   or `RTWK_REF`, merging `main`/`upstream` into the
   integration branch, joining the codex-rs workspace.
9. Pushing, a PR, anything outward.
10. A wave's token spend above 1.5× the previous comparable wave (§A10).
11. Wave gates: the skeleton's acceptance before W1 fans out; each wave's
    acceptance before the next fans out.

The orchestrator decides without asking: splitting a slice, the order inside a
wave, the number of porters within §A7, review model per §A5, landing, fix
rounds 1–2, batching follow-ups.

## A10. Token economy

From the economy rules (memory `codex-economy-rules`, task `token-economy`)
and rtwk's measurement:

- **Models.** Fixed by the agent type (the person, 2026-11-21): porters Sonnet,
  `codex-reviewer` Opus (FULL, wave), `codex-light-reviewer` Sonnet (LIGHT), waiters Haiku; `.claude/hooks/agent_models.py`
  refuses a `model` on an Agent call. Haiku waiters for every wait over ~4 min.
- **Agent size.** ~100 turns / ~250k context, then `task_checkpoint` +
  `HANDOFF:`; rtwk: turns past 100 were 59 % of implementation cost, so
  slices are sized to fit (§A3) instead of chained.
- **Lean starts.** Briefs in tm and short (§A4.1); shared rules in the
  fleet-brief and agent files, not in prompts; references (`refs/`, the
  registry) before oracle and rtwk source; rtwk's reference and gallery as
  the model for a view, never a landed screen.
- **No double checks.** A porter runs the crate's full check once before
  `task_done`, never per step or per partial: a partial carries the scoped
  checks only and the candidate check (`land-check.sh`) is its full gate; the candidate check runs once per candidate; the scenario
  suite and perf only at wave end. No porter waits on its own long command:
  `bgrun`.
- **Batching.** Small tasks two to one porter; review notes and doc fixes
  into one follow-up task per wave; several slices per candidate.
- **Orchestrator.** Out of the per-slice loop (rtwk: 14–34 % of all tokens
  went to the orchestrator): it reads verdicts and handover summaries (≤ 15
  lines), not diffs; a fresh session per wave or at ~230k, never past 350k,
  with the ≤ 40-line `## Now` in `~/.cache/codexcli/handoff.md`.
- **Measured per wave.** At hand-over the orchestrator records the wave's
  spend (agents by kind and model, fix rounds, landings) in the handoff
  archive; W1 calibrates, later waves are compared with it (§A9.10).

## A11. Additions to existing tasks this process needs

For the orchestrator to put into the briefs when the tasks start:
- `port-skeleton`: module stubs for every screen and a root variant per
  screen in the event and intent enums (§A4.4); `tests/rules.rs` (§A6);
  `docs/rtwk-port/parity/` with `delta-kinds.md` started; the first parity
  pack (`parity/port-skeleton.md`) as the format's example.
- `port-refs`: the registry tool also renders the parity packs into one
  side-by-side page (`reports/parity.html`) for the person's wave review.
- `scenario-harness`: scenarios named `<alias>-<name>`, runnable per slice
  (`-E 'test(<alias>)'`), the oracle taken from `~/.cache/codexcli/oracle-bin/`.
- `codex-perf-baseline`: the trend appends per wave (§A8.4) with the load
  recorded.
