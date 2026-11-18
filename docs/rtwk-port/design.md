# Codex CLI on rtwk — target architecture (design to review)

What the finished phase-1 app looks like, drawn to be looked at, weighed and
changed before the migration starts. Text decisions: [architecture.md](architecture.md).
Rendered: [design.html](design.html). App state follows ADR-003, accepted by the person
2026-11-17: plain model, thread-scoped `apply -> Effects`, intents on shared
state as values, rules R1–R14 (§6b). Corrected after the independent review
[reviews/arch-review.md](reviews/arch-review.md): navigation and dialogs as in
twk — `Navigator` + `PageRoute`, `show_dialog`, live data through
`rtwk::provider` projections of the model (§2, §3, §7, §8), `ThreadUi` for per-thread widget state, held in the UI model, not the core one (§6), the shell
spawns through rtwk-tokio (§4, §5), split pending maps (§6), counts in §6b.

## 1. System context

```mermaid
flowchart LR
  user([User at a terminal])
  subgraph rtwkapp["codex-rtwk (new, codex-rs/tui-rtwk)"]
    ui["rtwk app: screens, dialogs, keys"]
    st["App state"]
    sess["Session layer: app-server client task"]
  end
  subgraph backend["Codex backend (reused as is)"]
    asc["codex-app-server-client: InProcess / Remote / Cloud"]
    srv["app-server + codex-core"]
  end
  model[(Model API)]
  oracle["codex-tui (oracle, untouched): behaviour spec, tests, snapshots"]
  rtwk["rtwk framework (path dep, RTWK_REF)"]
  user <-->|keys, mouse, paste / frames| ui
  ui --> st
  st --> ui
  sess -->|events| st
  st -->|actions, RPC| sess
  sess <--> asc --> srv <--> model
  rtwk -.-> ui
  oracle -.->|carried over: reusable code + behaviour| rtwkapp
```

## 2. Who owns what

```mermaid
flowchart TB
  subgraph R["rtwk owns (never re-implemented in the app)"]
    r1["frame loop, frame pacing, terminal I/O, input decoding"]
    r2["widget tree, rebuild, layout, paint, damage diff"]
    r3["widget state: scroll, focus, text editing, selection, animation (per-thread controllers held in the UI model, UiState.thread_ui)"]
    r4["coalescing work from other threads, list virtualization"]
    r5["navigation (Navigator, PageRoute), dialogs (show_dialog), providers, popovers, rich content: Markdown, Diff, Code, charts, images"]
  end
  subgraph A["the app owns"]
    a1["application state: threads, turns, streams, approvals, settings, account, agents"]
    a2["rules: what an event or an action changes, stale-result dropping"]
    a3["derived values: counts, totals, filters, what a screen shows"]
    a4["screens: which rtwk widgets, laid out how, bound to which keys; which model projections are provided"]
  end
  subgraph B["the backend owns"]
    b1["sessions, model calls, tools, sandbox, config files, auth"]
  end
```

## 3. Crate and module layout

Two crates in one workspace and the arrows that are allowed (§3b says which
rule holds each one): the core has no rtwk and no direct runtime or IO dependency (the protocol crate pulls tokio in transitively); the UI crate holds the
views, the shell and the composition root; the host is the binary's backend.

```mermaid
flowchart TB
  subgraph ws["codex-rs/tui-rtwk (own Cargo workspace, binary codex-rtwk)"]
    subgraph core["codex-tui-core (no rtwk, no direct runtime or IO dependency)"]
      capp["app.rs: CodexApp (root model), AppEv, apply_app, ingest"]
      cthreads["threads.rs: ThreadModel, apply, project (protocol to events), Failure"]
      cscreens["screens/dir.rs, one per slice: model, events, intents, RPCs, wire"]
      ccmd["commands: 69 slash commands, parsing, queue rules"]
      cfix["fixtures/: builders, JSONL event scripts"]
      cids["ids.rs, rpc.rs, wire.rs: ids, tickets, protocol requests and replies"]
    end
    subgraph ui["codex-tui-rtwk (the rtwk app)"]
      uapp["app.rs: UiApp with core: CodexApp, ui: UiState and link: shell Link"]
      root["ui/view.rs: Theme, MultiProvider of model projections and handlers, Navigator (home: chat), request presenter; ui/thread_ui.rs: ThreadUi controllers"]
      subgraph screens["screens/dir/ (view files only, one directory per slice)"]
        s1["chat, cells, composer, popovers, transcript, select"]
        s2["approvals, settings, pager, backtrack, resume, status, usage"]
        s3["agents, sidebar, dialogs, pets, voice"]
        s4["update prompt S01, onboarding S02-S04, startup prompts S05, import S06"]
      end
      keymap["keymap/: tui.keymap config to rtwk Shortcuts, vim mode"]
      theme["theme.rs, l10n/"]
      shell["shell: the only effect runner and spawner; client via rtwk-tokio spawn_tokio / subscribe; epoch stamping; names no widget"]
      run["run.rs: composition root, run_app_with (runtime entered)"]
    end
    host["host/: main.rs (arg0 dispatch, tokio runtime), in-process app-server"]
    inline["inline/: inline screen mode S27 (last)"]
    tests["tests/: screens per slice, layers.rs, rules.rs, fixtures, scenarios, perf/"]
  end
  subgraph reuse["carried over from codex-tui (copied, non-UI; REUSE minus renderers, with dependency closure)"]
    c1["app_server_session, app_command (protocol seam)"]
    c2["keymap parsing, chunking, markdown_copy, clipboard OS side"]
    c3["analytics data, thread/update/session helpers"]
  end
  subgraph deps["dependencies"]
    d1["codex-app-server-client / -protocol, codex-core config, arg0"]
    d2["rtwk, rtwk-rich, rtwk-data, rtwk-tokio, rtwk-test, rtwk-term"]
  end
  uapp --> capp
  root --> capp
  screens --> cscreens
  screens --> capp
  shell --> capp
  shell --> cids
  run --> root
  run --> shell
  host --> run
  host --> cids
  root --> screens
  screens --> keymap
  reuse --> core
  reuse --> keymap
  ws --> deps
```

### 3b. Layers: what may name what

| layer | may name | may not name | held by |
|---|---|---|---|
| core | the protocol crate, serde, std without IO, clock or threads | rtwk, tokio, futures, UI types, IO | the compiler (no rtwk in `core/Cargo.toml`) and an allowlist of direct dependencies; `core/clippy.toml` (calls and types); `layers.rs`: `core_manifest_has_no_ui_or_runtime_dependency`, `core_sources_name_no_ui_runtime_or_io`, `core_sources_hold_no_user_visible_prose`, `the_ui_model_is_the_core_model_plus_ui_state` |
| view (`screens/dir/`, `ui/`, `l10n/`, `theme`) | core, rtwk widgets, its own strings | the protocol, `core::wire`, shell, tokio, IO, `apply` | `layers.rs`: `views_never_name_protocol_shell_tokio_or_io`, `views_never_apply_the_model`, `screen_dirs_hold_view_files_only`, `every_screen_dir_has_a_core_half_and_a_view_half`; `rules.rs`: R14, ADR-010 |
| shell | core, the protocol, tokio, rtwk `Cx` and `DialogResult` (to spawn) | widgets, views, theme | `layers.rs`: `shell_names_no_widget`; `rules.rs`: `r1_spawn_paths_are_the_shells`, `r11_r12_no_clock_and_no_file_system_outside_the_shell` |
| run, host | everything (the composition) | - | `rules.rs`: `r1_the_host_neither_spawns_nor_allows` |

## 4. Runtime: an event from the backend reaches the screen

```mermaid
sequenceDiagram
  participant AS as app-server
  participant T as shell subscription (tokio, rtwk-tokio)
  participant IN as ingress router (UI thread)
  participant TM as ThreadModel of thread X
  participant V as view(&state)
  participant R as rtwk
  AS->>T: notification (thread X, turn, item, delta)
  T->>IN: ThreadEv for X, stamped with the epoch captured when this subscription was created
  Note over IN: R1 drops events of an older epoch. User intents enter the same stream (R10)
  IN->>TM: apply(ThreadModel X, ThreadEv, ThreadFx X)
  Note over TM: R3 it can touch only thread X. R4 transcript upsert by (TurnId, ItemId), so a repeat updates and never duplicates. A resume snapshot replaces its turns wholesale
  TM->>TM: Versioned slices it touched bump their rev (memo deps carry rev plus Rc data)
  IN->>V: one view run for everything applied this flush
  V->>V: derived values computed (R7), memo subtrees with unchanged revisions reused
  V->>R: widget tree
  R->>R: rebuild changed elements, layout, paint, diff
  Note over R: the terminal shows the frame
```

## 5. Runtime: a user action and a late reply

```mermaid
sequenceDiagram
  participant U as user
  participant R as rtwk (Focus, Shortcuts)
  participant H as handler cx.on
  participant IN as ingress stream
  participant M as model apply
  participant SH as shell (the only place that spawns, rtwk-tokio)
  participant AS as app-server
  U->>R: Enter in the composer of thread X
  R->>H: action
  H->>IN: Intent Submit for thread X, text from the composer controller of ThreadUi (R10)
  IN->>M: in arrival order, after events already received
  M->>M: TurnState Idle to Running (R5), outgoing ticket 7 for (X, Ticket) (R3, R13)
  M-->>SH: Effects: Rpc start turn, ticket 7
  SH->>AS: request
  U->>R: switches to thread Y
  AS-->>SH: reply for ticket 7
  SH->>IN: Reply of T for ticket 7
  IN->>M: Reply of T for ticket 7
  M->>M: pending.resolve(reply) checks the ticket (R1): still current, so applied to X only
  Note over M: had the turn been interrupted, the ticket would be gone and the reply dropped by construction
```

## 6. App state shape

```mermaid
classDiagram
  class CodexApp {
    session: Session
    threads: ThreadId to ThreadModel map
    active: ThreadId
    settings: Settings
    account: Account
    agents: AgentsState
  }
  class ThreadModel {
    queue: submitted messages (R2)
    transcript: ordered map by TurnId and ItemId, upsert; snapshot replaces (R4)
    turn: TurnState (R5)
    outgoing: ThreadId and Ticket to Ctx (R1, R3)
    server_requests: ConnEpoch, ThreadId, ServerRequestId to Approval (R3, R13)
    tokens: Versioned Usage
  }
  class UiApp {
    core: CodexApp
    ui: UiState
    link: shell Link
  }
  class UiState {
    nav_key: GlobalKey
    thread_ui: ThreadUiMap
    lang: Lang
    debug: bool
  }
  class ThreadUi {
    in UiState, written by R14-allowlisted UI modules, never in the core model, not seen by apply
    composer: TextEditingController
    scroll, selection controllers
  }
  class TurnState {
    Idle
    Running(RunningTurn: streams, hooks, prompts, reviews)
  }
  class Remote {
    Loading
    Loaded(value, fetched_at, generation)
    Failed
  }
  class Approval {
    id: ServerRequestId scoped by ConnEpoch (R13)
    kind: Exec, Patch, Permissions, Mcp, UserInput
  }
  class Session {
    state: Starting, Ready, Failed
    backend
    conn_epoch: ConnEpoch, changes arrive as events
    rate_limits: Remote (R6)
    attach_epoch per thread (R1)
  }
  CodexApp --> ThreadModel
  ThreadModel --> TurnState
  ThreadModel --> Approval
  UiApp --> CodexApp : core
  UiApp --> UiState : ui
  UiApp ..> Link : link, the app's end of the shell
  UiState --> ThreadUi : keyed by ThreadId
  CodexApp --> Session
  Session --> Remote
```

Derived (functions of the model, never stored, R7): per-thread unread and
running, total tokens, sidebar filter results, agents-center counts, footer
hints, what the status line shows, `can_submit` and other predicates (R8).
Not allowed: app data in rtwk widget-local state (`Obs`, `use_state`,
hand-written `State`, controllers) outside UI-only modules — R14; IO in
`apply` or views — R12; the clock read in views — R11. `ThreadUi`, kept in the UI model (`UiApp.ui`) and never in `CodexApp`, holds
per-thread controllers (composer text, scroll, selection) so A→B→A keeps
them; the model never mirrors the draft — the text enters `apply` as an
intent on submit. App-scoped events (account, rate limits, app-level MCP
startup) go through `apply_app(&mut CodexApp, AppEv, &mut AppFx)`.

## 6b. Bug classes removed by construction (ADR-003, accepted 2026-11-17)

Checked on codex-tui's real history (research/state-bug-classes.md; the
report for openai/codex developers: [reports/tui-bug-classes.html](reports/tui-bug-classes.html)).
Of 55 state-mechanism fix commits: 20 impossible by construction, 33 caught
by a mandatory test or lint, 2 still possible (after arch-review M2/M4: SC6a
is caught by R14's lint, not impossible; the live+replayed case of SC4a,
6107b9886, is caught by the snapshot-replaces test).

```mermaid
flowchart LR
  subgraph bugs["bug sub-class in codex-tui"]
    b1["late or superseded reply applied"]
    b2["thread switch loses or mixes state"]
    b3["event or reply routed to the wrong thread"]
    b4["duplicate items within a stream"]
    b4r["duplicates of live vs replayed items after resume"]
    b5["turn state left behind after the turn ends"]
    b6["widget holds a stale copy of app state"]
    b7["derived value or flag kept by hand"]
    b8["response vs notification order"]
    b9["user input races a backend transition"]
    b10["selection index past changed data, loading vs empty"]
    b11["blocking IO on the event path"]
    b13["wrong id or id scope"]
    bp["refetch policy, distinct items with same text, one bool for two outcomes"]
  end
  subgraph rules["rule"]
    r1["R1 tickets on replies, spawn only in shell, epoch captured per subscription"]
    r2["R2 per-thread data in ThreadModel, controllers in ThreadUi (UI model)"]
    r3["R3 thread-scoped apply and effects, connection-scoped server requests"]
    r4["R4 transcript upsert by TurnId, ItemId"]
    r4s["R4 resume snapshot replaces its turns"]
    r5["R5 transient state inside TurnState Running"]
    r14["R14 no app data in widget state (lint)"]
    r7["R7 R8 derived as functions, lifecycles as enums"]
    r9["R9 first-arrival upsert, confirm local changes"]
    r10["R10 intents as values in the event stream"]
    r11["R11 R6 keyed selection, Remote of T, clock as input"]
    r12["R12 no IO in apply or views"]
    r13["R13 newtype per id kind"]
  end
  b1 --> r1
  b2 --> r2
  b3 --> r3
  b4 --> r4
  b4r --> r4s
  b5 --> r5
  b6 --> r14
  b7 --> r7
  b8 --> r9
  b9 --> r10
  b10 --> r11
  b11 --> r12
  b13 --> r13
  r1 --> imp(["impossible: types, privacy"])
  r3 --> imp
  r4 --> imp
  r5 --> imp
  r11 --> imp
  r13 --> imp
  r2 --> cau(["caught: mandatory tests, lints"])
  r4s --> cau
  r14 --> cau
  r7 --> cau
  r9 --> cau
  r10 --> cau
  r12 --> cau
  bp --> poss(["still possible: product logic, conventions"])
```

Mandatory tests the rules imply: switch round-trip (A→B→A), end-path matrix
(complete / interrupt / fail × feature), event-permutation property tests,
lifecycle × intent table, `Model::assert_consistent()` after every event.
Lints (own `clippy.toml`; crate-wide deny, `#[allow]` only on allowlisted
modules, a CI grep rejects other allows): every spawn path outside the shell
(`AppCx::spawn`, `Cx::spawn`, rtwk-tokio spawn/subscribe/follow,
`std::thread::spawn`, `tokio::spawn`, `AppCx::channel`), `Obs`/`use_state`/
hand-written `State`/controllers outside UI-only modules,
`Instant::now`/`SystemTime::now` in views, blocking IO in `apply`/views.
Plus the resume/reconnect-mid-turn test (transcript equals the snapshot).

## 7. Navigation map (screens of architecture.md §1)

As in twk: the root is a `Navigator` (home: chat) under a `MultiProvider` of
model projections. Screens (solid arrows) are `PageRoute`s pushed by the
widget that opens them and read live data with `select` / `watch`. Dialogs
(dotted) open with `show_dialog` from the asking widget. Backend requests
(approvals, elicitations, user input, startup prompts) open through the
request presenter and close themselves when the model drops the request.
Routes and dialogs get ids; results return to the model as ticketed events.

```mermaid
flowchart LR
  start((start)) --> upd["S01 update prompt"] --> onb["S02-S03 onboarding: welcome, auth, GovCloud, provider"] --> trust["S04 directory trust"] --> chat
  chat["S10 main chat (home); composer usable as a draft while Session is Starting"]
  chat -.presenter while Starting.-> prompts{{"S05 startup prompts"}}
  chat -->|Ctrl+T, /diff| pager["S16 pager"]
  chat -->|/resume, /fork| resume["S18 resume picker"]
  chat -->|/usage| usage["S20 usage dashboard"]
  chat -->|/agents| agents["S21 agents center"]
  chat -->|/import| imp["S06 import"]
  chat -.dialog.-> appr{{"S13 approvals"}}
  chat -.dialog.-> sel{{"S14 selection dialogs"}}
  chat -.dialog.-> set{{"S15 settings"}}
  chat -.dialog.-> stat{{"S19 /status"}}
  chat -.dialog.-> small{{"S23 small dialogs"}}
  chat -.popover.-> pop(["S12 / @ $ popovers"])
  chat -.mode.-> bt(["S17 backtrack"])
  chat -.part.-> side(["S22 sidebar, side panes"])
  chat -.part.-> pets(["S24 pets"])
  chat -.part.-> voice(["S25 voice"])
  chat -.part.-> uit(["S26 ui_tree in the status line, inline visualization in cells"])
```

## 8. Main chat screen: widget tree

```mermaid
flowchart TB
  scaffold["Navigator home route: Scaffold (theme, focus scope, Shortcuts from keymap)"]
  scaffold --> row["Row"]
  row --> sidebar["Sidebar (S22): TabBar + panes, slide animation"]
  row --> col["Column (main)"]
  col --> list["Transcript: ListView::builder(count, item) over thread.transcript + tail; SelectionArea (S11 cells: MarkdownView, DiffView, CodeView)"]
  col --> banners["Banners: backend / connection"]
  col --> pending["Pending input preview, queued messages"]
  col --> working["Working indicator: Spinner, elapsed, interrupt hint"]
  col --> composer["Composer: TextArea over the ThreadUi controller, attachments (G12: atomic elements, kill ring, vim, paste-burst)"]
  col --> footer["Footer hints + status line (ui_tree elements)"]
  scaffold -.overlay.-> popovers["Popovers: / @ $ (Autocomplete, OptionList, CommandPalette)"]
  scaffold -.show_dialog.-> dialogs["Dialogs: selection, settings, confirms, /status from the asking widget; answers as ticketed events"]
  scaffold -.presenter.-> reqs["Request presenter: show_dialog for the head of the model's request queue (approvals, elicitation, user input); closes when the model drops it"]
  scaffold -.push.-> routes["PageRoutes over chat: pager, resume, usage, agents (select / watch projections)"]
```

## 9. Tests, parity and performance

```mermaid
flowchart LR
  subgraph fixtures["one set of fixtures"]
    fm["mocked model (core_test_support wiremock SSE)"]
    ks["key scripts, recorded event streams"]
  end
  subgraph unit["inside codex-rtwk"]
    t1["state tests: events in, state out, races"]
    t2["screen tests: rtwk_test::Tester at oracle sizes"]
  end
  subgraph pty["both binaries in a pty (rtwk-term)"]
    o["codex (oracle)"]
    p["codex-rtwk"]
  end
  fixtures --> unit
  fixtures --> pty
  pty --> cmp["checkpoints compared, side by side on failure"]
  pty --> perf["perf/drive.py: 11 scenarios, interleaved A/B"]
  perf --> rep["report.html: distributions, ratios, side-by-side casts, milestone trend"]
  snaps["oracle .snap files"] -.->|visual reference, deltas listed per slice| t2
```

## 10. Migration waves (tm `tui-rtwk`)

```mermaid
flowchart LR
  w0["W0: research, architecture, this design, arch-review, process design"] --> w1["W1 foundation: skeleton, session core, slash commands, keymap, scenario harness, perf baseline of the original, refs"]
  w1 --> w2["W2 main chat (critical path): S10 layout, S11 cells and streaming, composer, S12 popovers, transcript, S14 selection dialog, perf milestone"]
  w2 --> w3["W3 screens in parallel: 18 slices (S13, S15-S25, S01-S06), 4-5 agents"]
  w3 --> w4["W4: inline mode, rtwk gaps, parity sweep, final perf report"]
```

## Open points on this design

1. App state (§4–§6b) per accepted ADR-003 and the arch-review fixes; the
   module split is settled in `skeleton-layers` (§3, §3b): `ThreadModel` in the
   core crate, `ThreadUi` in the UI model, the shell in the UI crate.
2. Which reusable codex-tui modules are copied in W1 vs with their screen
   (§3 "carried over") — from reuse-map.csv per slice.
3. Sidebar and side panes as part of the chat screen (§8) or a split layout
   of their own.
