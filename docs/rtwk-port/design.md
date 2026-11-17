# Codex CLI on rtwk — target architecture (design to review)

What the finished phase-1 app looks like, drawn to be looked at, weighed and
changed before the migration starts. Text decisions: [architecture.md](architecture.md).
Rendered: [design.html](design.html). App state follows ADR-003, accepted by the person
2026-11-17: plain model, thread-scoped `apply -> Effects`, intents on shared
state as values, rules R1–R14 (§6b).

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
    r3["widget state: scroll, focus, text editing, selection, animation"]
    r4["coalescing work from other threads, list virtualization"]
    r5["navigation, dialogs, popovers, rich content: Markdown, Diff, Code, charts, images"]
  end
  subgraph A["the app owns"]
    a1["application state: threads, turns, streams, approvals, settings, account, agents"]
    a2["rules: what an event or an action changes, stale-result dropping"]
    a3["derived values: counts, totals, filters, what a screen shows"]
    a4["screens: which rtwk widgets, laid out how, bound to which keys"]
  end
  subgraph B["the backend owns"]
    b1["sessions, model calls, tools, sandbox, config files, auth"]
  end
```

## 3. Crate and module layout

```mermaid
flowchart TB
  subgraph crate["codex-rs/tui-rtwk (own Cargo workspace, binary codex-rtwk)"]
    main["main.rs: CLI, config, tokio runtime, start client, run_app_with"]
    app["app.rs: CodexApp (root state) + Msg"]
    subgraph session["session/"]
      client["client task: notifications and requests to Msg"]
      threads["threads: per-thread state, buffering, replay"]
      stream["streaming: deltas, commit rules (carried-over chunking)"]
      appr["approvals: pending requests with ids"]
    end
    commands["commands/: 69 slash commands, parsing, queue rules"]
    keymap["keymap/: tui.keymap config to rtwk Shortcuts, vim mode"]
    theme["theme.rs"]
    root["ui/view.rs: Theme, providers, Navigator, routes"]
    subgraph screens["screens/ (one directory per slice)"]
      s1["chat, cells, composer, popovers, transcript, select"]
      s2["approvals, settings, pager, backtrack, resume, status, usage"]
      s3["agents, sidebar, dialogs, pets, voice, ui_tree"]
      s4["onboarding, startup, import"]
    end
    inline["inline/: inline screen mode (last)"]
    tests["tests/: screens per slice, session, scenarios, perf/"]
  end
  subgraph reuse["carried over from codex-tui (copied, non-UI)"]
    c1["app_server_session, app_command (protocol seam)"]
    c2["keymap parsing, chunking, markdown_copy, clipboard OS side"]
    c3["analytics data, thread/update/session helpers"]
  end
  subgraph deps["dependencies"]
    d1["codex-app-server-client / -protocol, codex-core config, arg0"]
    d2["rtwk, rtwk-rich, rtwk-data, rtwk-tokio, rtwk-test, rtwk-term"]
  end
  reuse --> session
  reuse --> keymap
  main --> app --> root --> screens
  app --> session
  screens --> commands
  screens --> keymap
  crate --> deps
```

## 4. Runtime: an event from the backend reaches the screen

```mermaid
sequenceDiagram
  participant AS as app-server
  participant T as session client task (tokio)
  participant IN as ingress router (UI thread)
  participant TM as ThreadModel of thread X
  participant V as view(&state)
  participant R as rtwk
  AS->>T: notification (thread X, turn, item, delta)
  T->>IN: ThreadEv for X, stamped with X's attach epoch
  Note over IN: R1 drops events of an older epoch
  IN->>TM: apply(ThreadModel X, ThreadEv, ThreadFx X)
  Note over TM: R3 it can touch only thread X. R4 transcript upsert by (TurnId, ItemId), so a repeat updates and never duplicates
  TM->>TM: Versioned slices it touched bump their revision
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
  participant M as model apply
  participant SH as shell (the only place that spawns)
  participant AS as app-server
  U->>R: Enter in the composer of thread X
  R->>H: action
  H->>M: Intent Submit for thread X (R10, a value in the same ordered stream)
  M->>M: TurnState Idle to Running (R5), pending ticket 7 for (X, RequestId) (R3, R13)
  M-->>SH: Effects: Rpc start turn, ticket 7
  SH->>AS: request
  U->>R: switches to thread Y
  AS-->>SH: reply for ticket 7
  SH->>M: Reply of T for ticket 7
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
    nav: NavState
    settings: Settings
    account: Account
    agents: AgentsState
  }
  class ThreadModel {
    draft and queue: per thread (R2)
    transcript: ordered map by TurnId and ItemId, upsert (R4)
    turn: TurnState (R5)
    pending: map of ThreadId and RequestId to ticket (R1, R3)
    approvals: list of Approval
    tokens: Versioned Usage
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
    id: ApprovalId (R13)
    kind: Exec, Patch, Permissions, Mcp, UserInput
  }
  class Session {
    backend
    connection: watch
    rate_limits: Remote (R6)
    attach_epoch per thread (R1)
  }
  class NavState {
    route
    dialogs
    selection: by item key, not index (R11)
  }
  CodexApp --> ThreadModel
  ThreadModel --> TurnState
  ThreadModel --> Approval
  CodexApp --> Session
  Session --> Remote
  CodexApp --> NavState
```

Derived (functions of the model, never stored, R7): per-thread unread and
running, total tokens, sidebar filter results, agents-center counts, footer
hints, what the status line shows, `can_submit` and other predicates (R8).
Not allowed: app data in rtwk widget-local state (`Obs`, `use_state`) — R14;
IO in `apply` or views — R12; the clock read in views — R11.

## 6b. Bug classes removed by construction (ADR-003, accepted 2026-11-17)

Checked on codex-tui's real history (research/state-bug-classes.md; the
report for openai/codex developers: [reports/tui-bug-classes.html](reports/tui-bug-classes.html)).
Of 55 state-mechanism fix commits: 28 impossible by construction, 25 caught
by a mandatory test, 2 still possible.

```mermaid
flowchart LR
  subgraph bugs["bug sub-class in codex-tui"]
    b1["late or superseded reply applied"]
    b2["thread switch loses or mixes state"]
    b3["event or reply routed to the wrong thread"]
    b4["duplicate items after resume or reconnect"]
    b5["turn state left behind after the turn ends"]
    b6["widget holds a stale copy of app state"]
    b7["derived value or flag kept by hand"]
    b8["response vs notification order"]
    b9["user input races a backend transition"]
    b10["selection index past changed data, loading vs empty"]
    b11["blocking IO on the event path"]
    b13["wrong id or id scope"]
  end
  subgraph rules["rule"]
    r1["R1 tickets on replies, spawn only in shell, attach epoch"]
    r2["R2 per-thread data in ThreadModel"]
    r3["R3 thread-scoped apply and effects"]
    r4["R4 transcript upsert by TurnId, ItemId"]
    r5["R5 transient state inside TurnState Running"]
    r14["R14 no app data in widget state"]
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
  b5 --> r5
  b6 --> r14
  b7 --> r7
  b8 --> r9
  b9 --> r10
  b10 --> r11
  b11 --> r12
  b13 --> r13
  r1 --> imp(["impossible: types, privacy, lints"])
  r3 --> imp
  r4 --> imp
  r5 --> imp
  r14 --> imp
  r13 --> imp
  r2 --> cau(["caught: mandatory tests"])
  r7 --> cau
  r9 --> cau
  r10 --> cau
  r11 --> cau
  r12 --> cau
```

Mandatory tests the rules imply: switch round-trip (A→B→A), end-path matrix
(complete / interrupt / fail × feature), event-permutation property tests,
lifecycle × intent table, `Model::assert_consistent()` after every event.
Lints: `AppCx::spawn` outside the shell, `Obs`/`use_state` outside UI-only
modules, `Instant::now`/`SystemTime::now` in views, blocking IO in
`apply`/views.

## 7. Navigation map (screens of architecture.md §1)

```mermaid
flowchart LR
  start((start)) --> upd["S01 update prompt"] --> onb["S02-S03 onboarding: welcome, auth, provider"] --> trust["S04 directory trust"] --> prompts{{"S05 startup prompts"}} --> chat
  chat["S10 main chat (home)"]
  chat -->|Ctrl+T, /diff| pager["S16 pager"]
  chat -->|/resume, /fork| resume["S18 resume picker"]
  chat -->|/usage| usage["S20 usage dashboard"]
  chat -->|/agents| agents["S21 agents center"]
  chat -->|/import| imp["S06 import"]
  chat -->|ui_tree| uit["S26 ui_tree viewer"]
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
```

## 8. Main chat screen: widget tree

```mermaid
flowchart TB
  scaffold["Scaffold (theme, focus scope, Shortcuts from keymap)"]
  scaffold --> row["Row"]
  row --> sidebar["Sidebar (S22): TabBar + panes, slide animation"]
  row --> col["Column (main)"]
  col --> list["Transcript: ListView.builder over thread.transcript + tail; SelectionArea (S11 cells: MarkdownView, DiffView, CodeView)"]
  col --> banners["Banners: backend / connection"]
  col --> pending["Pending input preview, queued messages"]
  col --> working["Working indicator: Spinner, elapsed, interrupt hint"]
  col --> composer["Composer: TextArea (+ vim from keymap), attachments"]
  col --> footer["Footer hints + status line"]
  scaffold -.overlay.-> popovers["Popovers: / @ $ (OptionList, CommandPalette)"]
  scaffold -.overlay.-> dialogs["Dialogs: approvals, selection, settings (show_dialog)"]
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
  w0["W0: research, architecture, this design, skeleton"] --> w1["W1 foundation: session core, slash commands, keymap, scenario harness, perf baseline of the original, refs"]
  w1 --> w2["W2 main chat (critical path): layout, cells and streaming, composer, popovers, transcript, selection dialog, perf milestone"]
  w2 --> w3["W3 screens in parallel: 17 slices, 4-5 agents"]
  w3 --> w4["W4: inline mode, rtwk gaps, parity sweep, final perf report"]
```

## Open points on this design

1. App state (§4–§6b) per accepted ADR-003; the exact module split of
   `ThreadModel` and the shell is settled in the skeleton.
2. Which reusable codex-tui modules are copied in W1 vs with their screen
   (§3 "carried over") — from reuse-map.csv per slice.
3. Sidebar and side panes as part of the chat screen (§8) or a split layout
   of their own.
