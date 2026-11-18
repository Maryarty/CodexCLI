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

**3. Write the tests, red** (first commit). Each oracle test of the slice's
files by its method (reuse-map.md Summary: snapshot replay is the bulk, then
logic tests, vt100 scenarios, cell diffs):
- snapshot test → a screen test at the oracle's size against the oracle
  `.snap` (D1);
- logic test → a state test: events into `apply`, model and effects out; a
  test of carried logic is carried nearly verbatim;
- vt100 scenario → a pty scenario, both binaries, compared at checkpoints
  (D3);
- cell diff → a cell and style diff (D2);
- a test of a replaced renderer's internals (wrap points, line building) →
  n/a with the reason "renderer replaced, ADR-004"; what the user sees of it
  is covered by a screen test, a difference by a delta.

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
- No workaround for an rtwk gap: a delta and "rtwk needs".
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
| `codex-reviewer` (Sonnet; Opus where §A5 says) | one review or one research question; verdict + findings | writes in the repository; accepts or rejects in tm |
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

## A3. A slice, from tm to landed

```
pending ─ brief in tm (§A4) ─ porter claims in a worktree
  ─ commit 1: parity tests, red (§A4.2) ─ commits: the slice, green
  ─ parity pack (§A4.3) ─ scoped + full checks ─ task_done → in_review
  ─ orchestrator sanity check (crate tree, codex-rs untouched, layer and
    rules tests green) ─ candidate on the integration tip (§A6) ─ land
  ─ batch review after the hand-out round's tasks have landed:
    codex-reviewer over the batch diff: MERGE | MERGE-WITH-NOTES | FIX
       FIX → a fix task per finding group (fix forward on the integration
             branch; ≤ 2 fix rounds per batch)
  ─ registry row `built` ─ the person accepts in tm (now or at wave end)
  ─ deltas decided → registry row `parity-accepted`
```

**Review per batch, not per slice** (the person, 2026-11-18): the skeleton
took four review rounds; at that density 20+ slices cost more in review than
in work. A slice lands on green checks and the orchestrator's sanity check;
one review covers a batch, normally the tasks of one hand-out round (§A7).

**Landing does not wait for the person's acceptance**, except for the
skeleton (§A7). Successor slices need their predecessors' code on the
integration branch (a porter never merges another branch), so waiting for the
person on every slice would stall the critical path. The person's gate is the
wave end: a wave fans out only after the person has accepted the previous
wave's tasks. A slice the person rejects after it landed is fixed forward (a
fix porter on the same task), never reverted unless the integration branch is
broken.

tm mechanics: slices use `triggerRule: allSubmitted`, so a successor is on the
front once its predecessors are in review. **The orchestrator hands a
successor out only after its predecessors have landed**, not merely been
submitted.

### Size

A porter run fits ~100 turns / ~250k context. Target ≤ ~1.5k lines of new
code and ≤ ~60 oracle tests to re-express per run (rtwk's cheapest slices
were ~0.8k lines). A larger slice (chat-cells: ~48 cell kinds; settings: ten
dialogs; agents; sidebar) is split by the orchestrator into child tasks before
hand-out (`tm-plan` replan), each with its own area and brief — never left to
`HANDOFF:` chains, which cost a fresh start and a re-read each time.

## A4. What a slice carries in and out

### A4.1 Brief (the task's description in tm)

The prompt of a porter is only: task alias, worktree, "read your task". The
brief lives in tm (`task_update` description) so a fix porter or a
continuation reads the same text. Every name and path in it is grepped first.

```
Goal: <screen id(s)> — <one sentence of behaviour>.
Oracle: registry rows <ids> (docs/rtwk-port/refs/codex-registry.csv);
        beyond them: <file:line ranges> (behaviour the registry does not list).
        Tests to re-express: <oracle test names or globs>; snapshots: <globs>.
Architecture: <sections by heading> (ADR-002 routes/dialogs, ADR-003 rows R…).
State: model parts read; events/intents added (in the screen's own sub-enum);
       mandatory tables extended (R2/R4/R5/R8/R9/R10, §A4.4).
Layers: core files (model, apply, protocol mapping; no rtwk) and view files
        (widgets, strings.rs; no protocol, shell, tokio, IO) named separately;
        mock state = a fixture script applied through core, never data handed
        to a widget (§A4.4 Layers).
rtwk: widgets and refs entries (docs/rtwk-port/refs/…); gaps that apply (G…) →
      listed as deltas, not worked around.
Carry: reuse-map rows copied (REUSE/ADAPT) with their dependency closure.
Manner: copy <finished slice path>.
Area: <paths>. Out of scope: <…>.
Evidence: screen tests, oracle map, deltas, pty scenario <name> | n/a until
          scenario-harness lands.
Review: Sonnet | Opus (why).
```

Briefs of a wave are drafted by one `codex-reviewer` (Sonnet) from the
registry, the features CSV and the oracle into a scratch file; the
orchestrator checks them and writes them into tm. That keeps 18 briefs off the
Opus context.

Before a brief goes into tm the orchestrator re-reads it for wording that
invites what the architecture forbids: "state, Msg and view in mod.rs", "data
from mock.rs into the widget", "store the controller in the app model". An
agent builds what the brief says; a boundary the brief leaves implicit is not
built and the review, reading against the brief, misses it too (2026-11-18:
the skeleton mixed UI into the model for exactly this reason).

### A4.2 Tests first — the porter writes them, the oracle supplies the answers

The porter of the slice writes the parity tests first, as its first commit
(`<alias>: parity tests (red)`), then the code. A separate test-writing agent
is not used: the expected values come from the oracle (its `.snap` text, its
test assertions, captures of the oracle binary), not from the implementer, so
there is nothing for the implementer to fit; a second agent would only add a
fresh start and a re-read of the same oracle files (rtwk measured 42 % fixed
overhead on small agents). The review checks the order and the source of the
expected values.

- Screen tests (rtwk-weather `tests/screens.rs` manner): a `Tester` at the
  oracle's size over the app on the fake backend; keys and server
  notifications scripted; the screen compared with the oracle snapshot through
  the replay helper (test-infra.md D1), differences only as listed deltas.
- State tests: events in, model and effects out, no UI (ADR-003).
- The red commit may stub the screen's types (`todo!()`); it is never landed
  alone. Tests that go red-then-green across the commits count as shown
  biting; a test added later is shown biting by breaking the code once.
- Foundation tasks without snapshots (skeleton, session-core) write the
  ADR-003 acceptance tests first: late reply dropped, switch mid-stream,
  resize mid-stream, approval during a dialog.

### A4.3 Parity pack — `docs/rtwk-port/parity/<alias>.md`

In the slice's area, committed with it; the reviewer and the person read it.

```
# <alias> — parity
## Oracle tests            every test of the slice's oracle files
| oracle (file::name) | ours (file::name) | ported | n/a: reason | delta Dn |
## Snapshots
| snapshot | size | equal | delta Dn |
## Visual deltas
### Dn — <screen>: <what differs>
kind: K<nn> from parity/delta-kinds.md, or NEW
cause: rtwk-rich renderer | gap G<n> | layout | other: …
oracle / ours: two fenced blocks, only the differing rows ± 2
proposal: accept | fixed here | follow-up task <alias>
## Scenarios (pty)
| scenario | checkpoints | result |       or: n/a until scenario-harness
## Rules
R-rule tables and tests extended: …; ADR-002 conditions: how each is met
## rtwk needs
```

- **Delta kinds** (`docs/rtwk-port/parity/delta-kinds.md`, the orchestrator's
  file): one entry per kind of difference (markdown table borders, wrap
  points, highlight palette, braille → rtwk-data chart, …) with the person's
  decision. A delta of a decided kind needs no new decision; a NEW kind goes
  to the person (§A9), batched per wave unless it blocks a slice. This is risk
  1's "decide once per kind".
- **pty checkpoint**: each screen slice adds at least one scenario script
  under the harness scenario-harness creates, run against both binaries
  (oracle from `oracle-bin`), compared at named checkpoints after
  normalisation (test-infra.md D3). The porter runs its own scenarios once
  before handover (`bgrun`); the whole suite runs at wave end.
- **Registry**: `codex-registry.csv` is rebuilt by the orchestrator after a
  landing (status `built`) and after the person's decisions
  (`parity-accepted`); porters never commit it (rtwk's rule for its registry:
  a shared file every slice touches is a merge conflict every time).

### A4.4 Rules every slice is held to

- **ADR-002 binding conditions** (nav-proto.md): (1) screen and dialog
  widgets `impl_widget!(stateful eq …)`; (2) the request presenter and every
  `show_dialog` caller under the `Navigator`; (3) the provider list fixed, the
  `Navigator` keyed by a `GlobalKey`; (4) the handlers provider
  `.update_should_notify(|_, _| false)`, no decision from a same-flush `read`,
  the presenter never reopens a shown id. Routes and dialogs take ids (R14).
- **ADR-003 R1–R14**: lints come from the skeleton's `clippy.toml`; an
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
  crate `codex-tui-rtwk-core` has no rtwk dependency — models, `apply`,
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

| what | reviewer | blocks (the next batch's hand-out; a landed defect is fixed forward) |
|---|---|---|
| landing a slice | the orchestrator: sanity check only (crate tree, codex-rs untouched, layer and rules tests, the porter's checks) | a red check, a layer or rules test failing, codex-rs touched |
| each batch (one hand-out round) | `codex-reviewer`, Sonnet, over the batch diff | a defect; a missing pack item; an ADR-002 condition or R-rule broken; a layer crossed (UI type in core, protocol/shell/IO in a view, widget in shell, mock data handed to a widget) even where no rule catches it yet — then the missing rule is a finding too; work outside the area; codex-tui or the codex-rs workspace touched |
| a batch with a risky slice: skeleton, session-core, chat-nav, chat-cells (streaming tail), chat-composer, chat-transcript, approvals (security, typeahead G8), pager (live tail), inline-mode, perf-chat and perf-report (claims) | `codex-reviewer` with `model: opus` | as above, plus a doubtful performance or concurrency claim |
| the wave (§A8) | `codex-reviewer` with `model: opus`, over the wave's diff | drift from the architecture or the manner; a rule enforced only on paper |
| the person | tm acceptance, deltas, §A9 | the next wave; NEW delta kinds only where a slice cannot finish without the decision |

What does **not** block landing: visual deltas of a decided kind, deltas
proposed as accepted (they stay `built`, not `parity-accepted`), rtwk gaps
listed as deltas, review notes.

Verdicts: **MERGE**, **MERGE-WITH-NOTES** (notes go into a batched follow-up
task, §A10), **FIX** (defects with `path:line` and the failing input). On FIX
the orchestrator rejects in tm with the findings as comments; a fix porter
takes the task (`task_start` re-grants it) in the same worktree. Two fix
rounds at most; a third is a stop (§A9). A fix round's re-review reads only the
fix commits and the open findings.

## A6. Landing on the integration branch

One lander: the orchestrator. Order: the tm dependency graph first, then
readiness. Several reviewed slices are combined in one candidate (rtwk:
"combine in dependency order on a temporary branch and check once").

1. In `.claude/worktrees/land`: a temporary branch at the integration tip;
   merge the slice branches `--no-ff` in dependency order (the merge commit
   names the alias). A conflict in the slice's own files → back to a fix
   porter (rebase onto the tip, allowed only then); a lockfile conflict → the
   orchestrator regenerates it.
2. The candidate check, run by a `codex-waiter` from `codex-rs/tui-rtwk`:
   `acargo fmt -- --check`; `acargo clippy -q --all-targets -- -D warnings`
   (the R-rule lists); `acargo nextest run` (whole crate: screen, state,
   `tests/pin.rs`, the rules test); `python3
   .github/scripts/verify_cargo_workspace_manifests.py`; `git diff --quiet
   <tip> -- codex-rs/tui codex-rs/Cargo.toml codex-rs/Cargo.lock`. The pty
   scenarios of the slices in the candidate run too; the whole suite only at
   wave end.
3. A failing test is retried once in the same run; a pass on retry lands,
   logged as FLAKY with a tm bug. A second failure blocks. A red candidate is
   split to find the slice. A check that covered the exact tree is reused.
4. Green: fast-forward `claude/tui-rtwk` to the candidate; rebuild the
   registry; remove landed worktrees.

The skeleton asks one thing more of itself for this: the CI-grep parts of R1,
R14 and the `unchanged()` ban as a test (`tests/rules.rs`), so nextest
enforces them in every worktree and candidate without a separate script.

## A7. Waves: agents in parallel

Limits: the machine (each porter builds; `acargo` caps memory and jobs), the
orchestrator's review and landing throughput, the person's acceptance.

| wave | in parallel | order |
|---|---|---|
| W1 | **skeleton alone** (Opus review, then the person accepts it: every slice copies its manner); beside it `codex-perf-baseline` (measures the oracle only) and `port-refs`; then session-core, keymap, scenario-harness (3); slash-commands after session-core. rtwk-gaps: coordination by the orchestrator with rtwk's orchestrator, no porter | ≤ 3 porters |
| W2 | chat-screen, chat-cells (split), chat-composer (3); then chat-popovers, chat-transcript, select-dialog; perf-chat after them | ≤ 3 porters |
| W3 | 18 slices: **4 porters**, 5 when ≥ 5 are ready and no more than 2 reviewed slices wait for landing. Risky and large first: approvals, settings (split), pager, resume, agents, sidebar; then the rest. Small ones (status-card, import, pets, small-dialogs) go two to one porter in sequence (one claim at a time) | 4–5 porters |
| W4 | inline-mode (Opus review) with the rtwk gaps it needs; parity-sweep as read-only Sonnet reviewers per screen group; perf-report by a waiter plus one porter | ≤ 3 |

The front decides what is free; the table caps how much runs at once.

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
   with its own orchestrator): wait, list as delta, or ask rtwk.
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

- **Models.** Porters on Sonnet (a hook refuses an override; rtwk: Opus
  implementers cost 4.7× per kLOC). Opus only in §A5's risky reviews and the
  wave reviews. Haiku waiters for every wait over ~4 min (0.06 M per wait vs
  ~1.4 M to re-write a Sonnet context).
- **Agent size.** ~100 turns / ~250k context, then `task_checkpoint` +
  `HANDOFF:`; rtwk: turns past 100 were 59 % of implementation cost, so
  slices are sized to fit (§A3) instead of chained.
- **Lean starts.** Briefs in tm and short (§A4.1); shared rules in the
  fleet-brief and agent files, not in prompts; references (`refs/`, the
  registry) before oracle and rtwk source; a finished slice as the manner.
- **No double checks.** A porter runs the crate's full check once before
  `task_done`; the candidate check runs once per candidate; the scenario
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
