# Codex CLI on rtwk — target architecture (design to review)

What the finished phase-1 app looks like, drawn to be looked at, weighed and
changed before the migration starts. Text decisions: [architecture.md](architecture.md).
Rendered: [design.html](design.html). App state follows ADR-003 (plain model,
`apply -> Effects`, chosen on the trade-offs in research/state-approaches.md),
in review with the person.

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
  participant CH as AppCx::channel
  participant M as App state
  participant V as view(&state)
  participant R as rtwk
  AS->>T: notification (thread_id, turn_id, item_id, delta)
  T->>CH: Msg::Thread(id, Event)
  Note over CH: rtwk coalesces everything sent before the next flush
  CH->>M: apply(event) for each item, in order
  M->>M: Versioned slices it touched bump their revision
  CH->>V: one view run for all items
  V->>V: memo subtrees whose revisions did not move are reused
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
  participant M as App state
  participant S as AppCx::spawn / session
  participant AS as app-server
  U->>R: key (e.g. Enter in the composer)
  R->>H: action
  H->>M: apply(Submit) changes state (draft to turn, gen = 7)
  M-->>H: Effects: Rpc(start turn, request id 7)
  H->>S: the shell runs the effect
  S->>AS: request
  U->>R: switches thread (gen becomes 8)
  AS-->>S: reply for 7
  S->>M: Msg::Reply(id 7)
  M->>M: id 7 is stale: dropped
  Note over M: rule: every async result carries its id, apply drops stale ones
```

## 6. App state shape

```mermaid
classDiagram
  class CodexApp {
    session: Session
    threads: ThreadId to Thread map
    active: ThreadId
    nav: NavState
    composer: ComposerState
    settings: Settings
    account: Account
    agents: AgentsState
  }
  class Thread {
    transcript: settled cells, append-only
    tail: the one live cell, optional
    pending_approvals: list of Approval
    status: Running/Idle/Error
    tokens: Usage
    rev: u64
  }
  class Approval {
    request_id
    kind: Exec, Patch, Permissions, Mcp, UserInput
  }
  class Session {
    backend
    connection
    rate_limits
    next_request_id
  }
  class Settings {
    theme
    keymap
    config view
  }
  class NavState {
    route
    dialogs
    popover
  }
  CodexApp --> Thread
  Thread --> Approval
  CodexApp --> Session
  CodexApp --> Settings
  CodexApp --> NavState
```

Derived (computed, never stored by hand): per-thread unread and running,
total tokens, sidebar filter results, agents-center counts, footer hints,
what the status line shows.

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

1. App state (§4–§6) per ADR-003; the person reviews the trade-off analysis
   (`research/state-approaches.md`).
2. Which reusable codex-tui modules are copied in W1 vs with their screen
   (§3 "carried over") — from reuse-map.csv per slice.
3. Sidebar and side panes as part of the chat screen (§8) or a split layout
   of their own.
