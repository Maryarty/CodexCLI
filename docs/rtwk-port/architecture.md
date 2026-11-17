# Codex TUI on rtwk — architecture

tm: `tui-rtwk` (root); this document is the deliverable of `port-design`.
Oracle: `codex-rs/tui` at [ORACLE_REF](ORACLE_REF), never edited. rtwk: the
checkout `/Users/admin/Documents/dev_projects/rtwk` at [RTWK_REF](RTWK_REF).
The target architecture drawn: [design.md](design.md) /
[design.html](design.html) (regenerate with `python3 -I docs/rtwk-port/design_html.py`).
Phase 1: one to one in functionality with the oracle. Phase 2: Codex Desktop
features ([desktop-gap.md](research/desktop-gap.md)). Phase 3: our own.

Research (read by section): what the UI does —
[codex-tui-surface.md](research/codex-tui-surface.md),
[codex-tui-features.csv](research/codex-tui-features.csv) (312 rows); how it is
built — [codex-tui-architecture.md](research/codex-tui-architecture.md); what
rtwk offers — [rtwk-for-codex.md](research/rtwk-for-codex.md),
[reuse-map.md](research/reuse-map.md); app state —
[state-survey.md](research/state-survey.md), [app-state.md](research/app-state.md),
[app-state-aopt.md](research/app-state-aopt.md),
[off-thread-state.md](research/off-thread-state.md);
bugs that go away — [bug-classes.md](research/bug-classes.md); tests —
[test-infra.md](research/test-infra.md); performance —
[perf-compare.md](research/perf-compare.md).

## 0. Direction and decisions

**Screen first.** We build the screens of the screen map (§1) as a native
rtwk app, the way a Flutter app is built. codex-tui's reusable code is
carried over (copied) into `tui-rtwk`: the rows that
[reuse-map.md](research/reuse-map.md) marks REUSE (protocol seam
`app_server_session`/`app_command`, keymap, clipboard OS side,
markdown_copy, chunking, analytics data, thread/update/session helpers) and
the non-UI part of ADAPT rows, adjusted to the app model. Its UI code is not
carried over, wrapped or bridged: no seam in codex-tui, no ratatui painter;
screens are built of rtwk widgets. codex-tui itself stays untouched as the
oracle: behaviour spec, tests to re-express, snapshots as the visual
reference. The backend is reused as it is (`codex-app-server-client`,
`codex-app-server-protocol`).

| # | decision | in one line |
|---|---|---|
| ADR-001 | crate and pins | `codex-rs/tui-rtwk`, its own Cargo workspace; rtwk by path at `RTWK_REF`; codex-rs workspace, lockfile, Bazel and CI untouched |
| ADR-002 | app structure | a Flutter-style rtwk app in the manner of `rtwk-weather`: model + `Msg` + `view(&App, &mut ViewCx)` + `cx.on`, routes, rtwk widgets |
| ADR-003 | app state | plain model owned by the rtwk app: one writer, `apply(&mut Model, Ev) -> Effects` per feature with request ids, derived values via `ViewCx::memo` on versioned slices; entities+notify as reserve; chosen on per-approach trade-offs |
| ADR-004 | screens and content | ~28 screens (§1) as routes, dialogs and popovers of rtwk widgets; rtwk-rich Markdown/Diff/Code; rtwk-data charts, rtwk-rich Image |
| ADR-005 | screen modes | owned full screen first (the oracle's default), inline last |
| ADR-006 | input | rtwk `Focus` / `Shortcuts` / `KeyBindings`; Codex's keymap config read into them; input never frozen by an RPC |
| ADR-007 | parity | functional (screen tests + pty scenarios against the oracle) and visual (oracle snapshots and captures as the reference, deltas listed per screen) |
| ADR-008 | performance | the pty A/B harness of perf-compare.md, baseline of the original first, report.html + side-by-side casts + milestone trend |
| ADR-009 | migration | foundation → main chat screen → screen slices in parallel → inline mode and rtwk gaps |

## 1. What we build (facts and the screen map)

- The oracle: ~259k production lines, 6,329 tests, 1,549 text snapshots
  (`app/` 130k, `chatwidget/` 90k, `bottom_pane/` 85k). It is a mutable
  component tree, one `AppEvent` bus and one `select!` loop; one foreground
  task mutates all state, background tasks report over the bus.
- The backend sits fully behind `codex_app_server_client` (InProcess, Remote,
  Cloud) and `codex_app_server_protocol`: the frontend is a pure swap.
- Default screen mode is owned full screen: `tui.fullscreen_transcript`
  is true when unset (`codex-rs/core/src/config/mod.rs:4932`).

Screen map (sources: codex-tui-surface.md §1–§13; ids are the plan's):

| id | screen | kind | oracle |
|---|---|---|---|
| S01 | update prompt | route before main | update_prompt.rs |
| S02 | onboarding: welcome, auth (browser, device code, API key) | routes | onboarding/welcome.rs, auth.rs, auth/ |
| S03 | onboarding: provider, gateway, Bedrock, GCP setup | routes | onboarding/gateway*.rs, bedrock.rs, gcp.rs, provider_setup.rs |
| S04 | directory trust | route | onboarding/trust_directory.rs |
| S05 | startup prompts: cwd, model migration, startup hooks, unarchive, OSS provider, daemon recovery, startup error, Windows sandbox | dialogs | cwd_prompt.rs, model_migration.rs, startup_hooks_review.rs, unarchive_prompt.rs, oss_selection.rs, daemon_recovery.rs, startup_error.rs |
| S06 | import from other agents (`/import`) | route | external_agent_config_migration/ |
| S10 | **main chat**: transcript, banners, pending input, working indicator, composer, footer, status line | home route | chatwidget/, bottom_pane/, transcript_view/, app/owned_transcript.rs |
| S11 | streaming and history cells (~48 kinds), markdown, diff, exec output | transcript items | history_cell/, streaming/, markdown_render/, diff_render.rs |
| S12 | composer popovers: `/` commands, `@` files, mentions, `$` skills, history search | popovers | command_popup.rs, file_search_popup.rs, mentions_v2/, skill_popup.rs, chat_composer/history_search*.rs |
| S13 | approvals: exec, permissions, patch, MCP elicitation, request-user-input, async questions | dialogs | approval_overlay.rs, mcp elicitation, request_user_input |
| S14 | generic selection dialog (~30 parameterisations: model, permissions, review, worktrees, ...) | dialog | list_selection_view.rs |
| S15 | settings dialogs: `/config`, status line, title, theme, keymap setup/capture/debug, experimental, memories, skills, hooks, provider editor | dialogs | config_panel/, status_line_setup, keymap_setup/, ... |
| S16 | transcript pager (Ctrl+T), static pager (`/diff`), search | route | pager_overlay.rs |
| S17 | backtrack (Esc Esc) | mode of S10 | app_backtrack.rs |
| S18 | resume / fork picker | route | resume_picker/ |
| S19 | `/status` card | dialog | status/card.rs |
| S20 | `/usage` analytics dashboard | route | analytics/ |
| S21 | agents command center, subagent switcher | route | app/agents_overview*.rs, multi_agents.rs |
| S22 | sidebar and side conversation panes | parts of S10 | app/sidebar*.rs, app/side*.rs |
| S23 | small dialogs: shortcut overlay, warnings, daemon menu, app link, custom prompt, user verification, feedback (+ note, consent) | dialogs | bottom_pane/* |
| S24 | pets | overlay | pets/ |
| S25 | voice / realtime strip and settings | part of S10 + dialogs | voice_strip.rs, chatwidget/realtime*.rs |
| S26 | `ui_tree` and visualization viewer | route | ui_tree/ |
| S27 | inline mode (screen mode, not a screen) | mode | tui.rs, insert_history.rs, app/resize_reflow.rs |

Behaviour without a screen of its own is carried by the screen that needs
it: slash commands (69, slash_command.rs) by S10/S12, IDE context
(ide_context/) and notifications by S10, keymap runtime by ADR-006.

## ADR-001 — Crate layout and pins

**Decision.** `codex-rs/tui-rtwk` is its own Cargo workspace (`[workspace]`
in its Cargo.toml, its own Cargo.lock), producing the binary `codex-rtwk`.
It depends on codex crates by path (`../app-server-client`,
`../app-server-protocol`, `../core` for config, `../arg0`, test support as
dev-deps) and on rtwk crates by path pinned to `RTWK_REF`. It does not depend
on `codex-tui`.

**Why.** The codex-rs workspace, its Cargo.lock, Bazel (`crate_universe`,
the PR gate) and `rust-ci-full` stay as upstream has them, so merges from
`upstream` never conflict with the port and no CI breaks on a path that
leaves the repository. Joining the codex-rs workspace (and a `--frontend
rtwk` switch in `codex`) is decided at the end of phase 1, when rtwk can be
referenced by git or registry.

**Check in the skeleton.** Path dependencies on crates that inherit
`workspace = true` fields from codex-rs resolve from an outside workspace;
the cold build fits `bgrun`; worktrees reach rtwk (`.claude/worktrees/rtwk`
symlink).

## ADR-002 — App structure: a Flutter-style rtwk app

Template: `/Users/admin/Documents/dev_projects/rtwk-weather` (5k lines):
`model.rs` (domain types), `app.rs` (the app struct, `Msg` from background
work, methods that change state and start work), `ui/view.rs`
(`view(&App, &mut ViewCx<App>) -> AnyWidget`, callbacks `cx.on(|app, cx| ..)`
and `cx.on_with`), `theme.rs`, `tests/screens.rs` (`rtwk_test::Tester` over
`overlay::App::new(AppView::new(app, view))` on made-up data). Background
results arrive as `Msg` values carrying an id (`Slot.id`), and stale ones are
dropped.

```
codex-rtwk main ─ CLI, config, tokio runtime (rtwk-tokio), app-server client
 └ run_app: overlay::App → AppView(CodexApp, view)
    └ Theme · Localizations · providers (config projection, keymap)
       └ Navigator: onboarding/* · chat (home) · pager · resume · usage · agents · ...
          └ screens = functions of &CodexApp built of rtwk widgets
             dialogs via show_dialog, popovers via the overlay, toasts
```

| need | rtwk |
|---|---|
| transcript | `ListView` with an item builder over the model's cells (rtwk mounts only visible items: virtualization is the default, not a task); `SelectionArea` for selection and copy |
| message content | rtwk-rich `MarkdownView`, `DiffView`, `CodeView` |
| composer | `TextArea` (`TextField` for single-line inputs) |
| lists, pickers, commands | `SelectionList`, `OptionList`, `CommandPalette`, `Table`, `Tree`, `TabBar` |
| approvals, settings | `show_dialog` + the widgets above |
| progress, working | `Spinner`, `ProgressBar`, rtwk-animation |
| analytics | rtwk-data `Chart`, `BarChart`, `Sparkline` |
| pets, images | rtwk-rich `Image` |
| keys | `Focus`, `Shortcuts`, `KeyBindings` |

- Backend events: one task owns the app-server client and turns server
  notifications and requests into `Msg` sent through `AppCx::channel`; the
  app handles each `Msg` on the UI thread. RPCs are started from callbacks
  and answer with a `Msg` carrying the request id.
- rtwk owns widget state (scroll, focus, text editing, selection, hover,
  animation); the app model holds only application state (§ADR-003).
- Standalone pre-app screens of the oracle (onboarding, trust, prompts)
  become routes of the same app, shown before the chat route.

## ADR-003 — App state

**Scope.** Application state and how views follow it; widget state is
rtwk's.

**Measured** ([app-state.md](research/app-state.md), prototype
`~/.cache/codexcli/research/app-state-proto`): C, a plain model owned by the
rtwk app (one writer on the UI thread, events via `AppCx::channel`,
`ViewCx::memo` on revisions): 1.1 allocs/delta, 7.4 ms per scenario. A,
reactive_graph + reactive_stores used naively (a signal write per delta
through an Arc store across threads, keyed transcript): 160 allocs/delta,
58.6 ms; cross-thread store reads panic, contrary to rtwk's
`docs/guide/state.md:94` (reported:
`~/.cache/codexcli/rtwk-issues/rtwk-state-reactive-graph.md`). B
futures-signals: rejected (deadlock-prone wake, no batching, unmaintained
since 2024-07). Rebuild isolation was the same for all three.

**Round 3, optimized** ([app-state-aopt.md](research/app-state-aopt.md);
the person's criterion: cost *with* optimizations, and maintainability of
derived state across panels). Same harness, release; times as ratios (load
was high):

| variant | allocs/delta | scenario | derived-panel change |
|---|---|---|---|
| C plain model | 7.3 | 1.0x | 14.5 µs, 17 allocs |
| A-sig: reactive_graph signal per field, UI thread, per-frame coalescing | 15.6 | 1.5x | — |
| A-opt: reactive_stores store, UI thread, coalesced | 41.8 (28.0 with one write per frame) | 3.6x (2.0x) | 22.5 µs, 178 allocs |
| A naive (round 1) | 170.6 | 10.5x | — |

Maintainability (40 threads, 4 derived panels): C 69 lines vs A-opt 95;
adding a derived value and a panel: C 3 places / 10 lines, A 5 places / 5
lines. In C a cached subtree can use only what it is keyed on — anything
else fails to compile. In A a forgotten dependency is an untracked read: the
panel stays stale until something else changes, caught only by a test; A
also carries six rules that fail silently (UI thread only, keep widgets
across rebuilds, no `batch`, ...). UI-thread confinement removes the
cross-thread panic; the rtwk fixes in the issue file do not change A-opt's
cost (it is reactive_stores' own per-write work). New rtwk finding: a
`Select` notified inside a frame wakes the loop again (+13 ms first frame
after idle; cause inferred).

**Landscape of Rust state libraries** ([state-survey.md](research/state-survey.md)):
reactive_graph/reactive_stores, dioxus-signals, futures-signals,
eyeball/eyeball-im, rxRust, redux-style crates, yewdux, relm4, tui-realm,
iced, Crux, bevy_ecs, salsa, actor crates, tokio `watch`, statig. None
handles request ids and stale results — that is app code in Zed, Helix,
gitui, yazi and cosmic-files, which all keep a plain owner and bring async
results in by message. Variant E (an entity + notify pattern written by
hand) measured 8.1 allocs/delta, 2.5 ms against C's 7.3 and 2.4 ms; its
risk is a forgotten notify (a stale view), a recurring bug class in Zed.

**Trade-offs of the approach families** ([state-approaches.md](research/state-approaches.md):
for each — why it exists, what it solves, what it brings, eight Codex cases,
verdict; matrix in §10):

| family | solves | brings | verdict |
|---|---|---|---|
| 1 plain model, whole-view rerun, `ViewCx::memo` | views and derived values cannot go stale (a kept subtree can use only its keys — otherwise no compile); `apply` is a plain function, deterministic tests; cheapest (7.3 allocs/delta) | root view reruns each changed frame; a wrong revision key or a misused `unchanged()` gives a stale subtree; `update` grows large | **core** |
| 2 reducers + effects as data (redux-style, Crux) | late replies are events; race tests need no runtime | boilerplate, indirection; crates dead or breaking each minor | **partial**: take its `apply -> Effects` boundary |
| 3 fine-grained signals (reactive_graph, dioxus-signals, futures-signals) | automatic dependency tracking; only subscribers rebuild | untracked reads go stale silently; cross-thread read panic; lost batch notification; six silent rules; 2.1–5.7x cost | does not fit |
| 4 entities + notify / diffs (Zed, eyeball, rtwk `Select`) | panel isolation at family 1's cost (8.1) | a forgotten notify is a stale view (44 Zed PRs); derived state synced by observers | **reserve** |
| 5 actors (ractor, kameo, actix) | isolating services | UI sees only snapshots; `Send` everywhere; no order across mailboxes | not for UI state |
| 6 ECS (bevy_ecs) | iterating many entities per tick | schedule decides order; unread messages dropped silently | does not fit |
| 7 incremental computation (salsa) | heavy derived results on demand | nothing pushes, a view cannot follow it; cancellation by panic | not now; maybe one heavy query later |
| 8 streams / Rx | `switch_map`-style dropping of stale results | a second scheduler beside rtwk's; state hidden in closures | ingress only, not a store |

**Decision (2026-11-17): family 1 with family 2's effect boundary; family 4
as the reserve.** Reasons: (1) it removes the desync bug class (~22 points of
the oracle's fix history, bug-classes.md) with the narrowest failure mode —
one wrong memo key, and many mistakes do not compile; (2) request ids and
race tests are cheapest with one writer and a pure `apply`; (3) it is rtwk's
own model, the cheapest measured, and what Helix, gitui, yazi and iced do.

- Rust structs owned by the rtwk `AppView`; one writer, the UI thread.
- Per feature `apply(&mut Model, Ev) -> Effects`, threads keyed by
  `ThreadId`; events enter through one `AppCx::channel`; effects (RPCs,
  config writes) are values the shell runs with `AppCx::spawn`, their
  results come back as events carrying the request or generation id, and
  `apply` drops stale ones. Tests: events in, model and effects out, no UI.
- Every thread's model is kept current as its events arrive, so switching
  threads needs no replay in the UI; the user-visible behaviour of the
  oracle's replay-on-switch (what a switched-to thread shows) is the parity
  target. Reconnect/resume still replays from the backend.
- Derived values are pure functions computed in the view (measured cheap:
  10 derived values scanning 1,000 threads add 9 µs per frame) and kept by
  `ViewCx::memo` on per-slice revisions; slices are `Versioned<T>` whose
  `get_mut` bumps the revision. A counter kept in `apply` is allowed only
  inside one collection type, with a test that recomputes it.
- Rules: `unchanged()` is banned in feature code (CI grep); long lists use
  `ListView` builders; only mounted screens render; memoise one level per
  region; a per-release budget test on the scale scenario.
- Flows (approvals, onboarding) are plain enums; `statig` only if a flow
  outgrows them. Config and connection state may follow a tokio `watch`.
- Switch a panel to the reserve (entities + notify inside `update`,
  followed through `Select`) only if a phase-2 profile shows the root view
  above ~1 ms per frame after these rules.
- Measured scaling (state-approaches.md §11): 12–19 µs per frame at 1,000
  threads / 30 panels; the cost grows with the square of memo calls (81 →
  9.5 µs, 331 → 97 µs, 1,031 → 648 µs) because rtwk's memo cache is a
  linearly searched `Vec` (rtwk crates/rtwk/src/app.rs:812-819) — reported
  to rtwk with the nested-memo release (an outer memo hit releases the inner
  entries).
- Later, not phase 1: per-thread sequence numbers in app-server
  notifications would let reconnect deduplicate without comparing content.

**Off-thread readers: none need a snapshot**
([off-thread-state.md](research/off-thread-state.md)). In the oracle,
notifications, the status line and the title are decided on the UI thread;
background work reports over channels. Four places touch UI state
off-thread (onboarding sign-in state, the `voice_only` send filter, the
interrupt retry task, the dynamic-tools MCP handle); in the rtwk app each
becomes a message handled on the UI thread, or a `watch` of the connection
handle. Work started off-thread gets config and theme cloned at spawn. So
the model has one owner and no published copy.

**Acceptance criteria (every screen and the state layer).**
1. Views are derived from state only: no "refresh the UI" calls, no widget
   holding a copy of application state.
2. Derived values are computed (pure functions or memos), never kept in sync
   by hand.
3. Every async result carries a generation or request id; a stale one is
   dropped.
4. Races are tested: late RPC replies, thread switches mid-stream, resize
   mid-stream, approvals arriving during a dialog.

With these, ~32% of the oracle's historical fixes go away as a class
([bug-classes.md](research/bug-classes.md)).

## ADR-004 — Screens and content

- Each screen of §1 is a slice: its route or dialog, the state it reads, the
  behaviour and the reusable code carried over from the oracle (named by
  oracle `file:line` and test names in the slice's task; copied code keeps a
  comment naming its oracle file), its screen tests.
- Content is rendered by rtwk-rich (`MarkdownView`, `DiffView`, `CodeView`),
  not by re-implementing Codex's line renderers. Where the result differs
  visually (wrapping, table borders, highlight palette), the difference is
  listed in the slice and accepted by the person or fixed.
- Charts (`/usage`) use rtwk-data; pets and images use rtwk-rich `Image`.
  Visual deltas from the oracle's braille plots and image protocol code are
  accepted (the person, 2026-11-17).
- The streaming tail is one live item at the end of the transcript list,
  updated at most once per frame; settled messages become ordinary items.

## ADR-005 — Screen modes

Owned full screen first (the default; alt screen, mouse, the transcript as an
rtwk `ListView`). Inline mode last: `ScreenMode::Inline` with settled items
committed through `Static`, after the rtwk gaps it needs (G1, G2, G5, G7).

## ADR-006 — Input and keys

rtwk decodes the terminal (kitty flags, bracketed paste, mouse, focus,
resize). Keys are handled by rtwk `Focus`, `Shortcuts` and `KeyBindings`.
Codex's keymap configuration (`tui.keymap.*`: 9 contexts, chords, a leader
key, vim mode) is read into those bindings, so user configs keep working;
the oracle's keymap tests are the spec. Input is never frozen while an RPC
is in flight (the person, 2026-11-17). Ctrl+Z suspends; the external editor
suspends the app. Typeahead quarantine before security prompts needs G8.

## ADR-007 — Parity: functional and visual

1. **Screen tests** in the rtwk-weather manner (`tests/screens.rs`): a
   `Tester` of the oracle's size over the app with a fake backend, keys and
   server notifications scripted, the screen asserted. Each slice re-expresses
   the oracle tests that define its behaviour.
2. **Scenarios, both binaries.** `codex` (oracle) and `codex-rtwk` driven in a
   pty (rtwk-term `TerminalSession`) against the same mocked model
   (`core_test_support::responses`, wiremock SSE), the same `CODEX_HOME` and
   key script; screens compared at named checkpoints, timers and paths
   normalized ([test-infra.md](research/test-infra.md) §D3).
3. **Visual reference.** The oracle's `.snap` files and pty captures are the
   reference a slice is reviewed against, side by side. Byte equality is not
   required; every difference is listed in the slice (ADR-004) and accepted
   or fixed.
4. **Registry.** `docs/rtwk-port/refs/codex-registry.csv`: screen id →
   oracle files, snapshots, tests → status (`todo` / `built` /
   `parity-accepted`).

## ADR-008 — Performance, shown visually

[perf-compare.md](research/perf-compare.md): `perf/drive.py` runs a binary in
a pty at fixed sizes against `perf/fake_model.py`, 11 scenarios (stream
100/s and 1000/s, typing while streaming, long transcript scroll, resize
storm, startup, idle, ...), interleaved A/B runs N=15, load gate, raw JSONL.
`perf/report.py` writes one `report.html` (distributions, ratio table with
bootstrap intervals, side-by-side asciicasts); `perf/trend.py` keeps the
milestone trend against the original's baseline. The baseline of the
original (S1, S2, S5, S9, S10 at 120x40) is taken first, before any port
code. No speed claim from a microbenchmark alone.

## ADR-009 — Migration order

1. **Foundation.** Skeleton (below); session core: app-server client task,
   thread start/resume, streaming into the model, approvals round trip,
   slash command dispatch; perf baseline of the original; scenario harness.
2. **Main chat screen** S10 + S11 + S12 (critical path).
3. **Screen slices in parallel** (4–5 `codex-porter` agents in worktrees,
   rtwk-weather manner): S13–S26, pre-app routes S01–S06.
4. **Inline mode** S27 and the rtwk gaps it needs; parity sweep over the
   registry; performance report.

## Skeleton (tm `port-skeleton`)

```
codex-rs/tui-rtwk/          own workspace, binary codex-rtwk
  Cargo.toml                codex crates by path, rtwk by path (RTWK_REF)
  src/main.rs               CLI, config, tokio runtime, app-server client, run_app
  src/app.rs                CodexApp (model), Msg, methods
  src/backend.rs            client task: notifications → Msg over AppCx::channel
  src/theme.rs
  src/ui/view.rs            root: Theme, Navigator, routes
  src/ui/chat.rs            empty chat screen: transcript ListView, TextArea, footer
  tests/screens.rs          Tester over the app on a fake backend
```

Done when: `codex-rtwk` starts against the in-process app-server, shows the
empty chat screen, sends one prompt to the mocked model and shows the
streamed answer as plain text, quits; `tests/screens.rs` green; the codex-rs
workspace and lockfile untouched.

## rtwk gaps (work in the rtwk repository, its own tm)

| gap | needed by |
|---|---|
| G1 enter / leave the alternate screen at runtime from an inline app | inline overlays (pager, diff) |
| G2 re-emit the app's own committed scrollback on resize | inline resize reflow |
| G4 image placements inside a region, if rtwk-rich `Image` falls short | pets, `view_image` |
| G5 Zellij and Windows Terminal scrollback strategies | inline on those terminals |
| G7 committed rows keep OSC 8 links and images | inline history |
| G8 discard pending input | typeahead quarantine before approvals |
| G9 selection, search and copy-source over a virtualized transcript | S10, S16 |
| G10 tmux probes (mouse option, pane-local graphics) | S10, S24 |
| G11 clipboard breadth (native, WSL, X11 PRIMARY beyond OSC 52) | copy |
| rtwk-state: reactive_graph cross-thread panic, `batch` loss (the issue file) | ADR-003 option A |
| rtwk-state: `Select` notified inside a frame wakes the loop again (app-state-aopt.md §5) | ADR-003 option A |

G3 (a `!Send` controller task) and G6 (shared width model with Codex's
renderers) no longer apply: there is no carried-over controller and rtwk
lays out all text.

## Risks

1. rtwk-rich renderers differ from Codex's (markdown, diff, highlight); many
   small visual deltas to review. Mitigation: list per slice, decide once
   per kind.
2. Behaviour is spread across the oracle's 130k-line `app/` and 90k-line
   `chatwidget/`; a slice can miss a rule. Mitigation: each slice names the
   oracle tests it re-expresses; pty scenarios catch the rest.
3. Transcript performance at 10³–10⁴ items and 1,000 deltas/s; keep the
   live tail one item, settle into ordinary items, measure (ADR-008).
4. Inline mode depends on rtwk gaps landing.
5. Fork-specific areas (agents center, sidebar, analytics, `ui_tree`) are
   large and lightly documented.
6. Windows: rtwk's evidence is partial.

## Open decisions for the person

1. When the port joins the codex-rs workspace and how `codex` exposes it
   (`--frontend rtwk` or `codex-rtwk`) — end of phase 1.
2. Whether the rtwk gaps go into rtwk's own tm now or when a slice needs
   them.
