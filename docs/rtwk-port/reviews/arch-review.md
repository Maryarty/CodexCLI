# Architecture review (arch-review)

Design version checked: a2dbf130c (docs/rtwk-port/architecture.md, design.md; rtwk at RTWK_REF 675a98d73 = rtwk HEAD; oracle at c6a41e32f).

## Summary

Verdict: **ready after fixes.** The direction (screen first, plain model, family 1 + effect boundary, R1–R14) holds up against the research, and the rtwk names in the docs exist at the pin (two misnamed). One blocker: the docs build screens with rtwk's imperative `Navigator`/`show_dialog`. Those cannot see `&CodexApp`, and they contradict the plain model, NavState-in-model and the rtwk-weather manner the docs cite. Five "impossible by construction" claims are wider than their mechanisms: R14, R1's lint, R4 against the backend's id synthesis, R3's pending key, and module-scoped lints. ADR-001's layout resolves, but only with codex-rs's `[patch]` copied and its lockfile seeded, and repo-checks CI will police the manifest. The research the design rests on (reuse-map) still describes the earlier architecture. Counts: 1 blocker, 7 major, 10 minor, 5 nit.

## Blocker

**B1. How routes and dialogs are made contradicts the state design and cannot read the app model.**
- Where: architecture.md:129-133 (tree: `Navigator`, "dialogs via show_dialog"), :142 (approvals via `show_dialog`), :154-155 (pre-app screens as routes), :397 (Skeleton `ui/view.rs  root: Theme, Navigator, routes`); design.md:45 (rtwk owns "navigation, dialogs"), :188-192 (`NavState { route, dialogs, selection }` in the model), :316 ("Dialogs ... (show_dialog)").
- Evidence:
  - rtwk `Navigator` is Flutter-1.0 imperative: it has named routes and a route stack kept in `NavigatorState` (rtwk crates/rtwk/src/navigation/navigator.rs:1637-1655; push/pop API from :532). Its route builder is `pub type RouteBuilder = Rc<dyn Fn(&mut BuildCx<'_>) -> AnyWidget>` (navigator.rs:78). It gets no `&A`, and `AppView` provides no inherited app scope (no match for an app-scope/`read_app` provider in crates/rtwk/src/app.rs). So a route's content can only see app data that was cloned into its closure, which is SC6a.
  - `show_dialog(cx: &Cx<'_, S>, builder, options) -> DialogResult<T>` (rtwk-overlay/src/dialog.rs:301) is called from a component's `Cx`, and its future is handled in `cx.spawn` (rtwk crates/rtwk/SKILL.md:162, :245-262). It is not called from `ViewCx`/`AppCx`.
  - The template the docs cite does neither. rtwk-weather draws its dialog in the view as `Stack` + `ModalBarrier` + `Dialog` (rtwk-weather src/ui/view.rs:18-20, :132, :429) and uses no `Navigator` or `show_dialog` (grep: none in src/). The research says the same: "a modal queue in the model ... the view shows the head over rtwk's dialog barrier" (research/state-approaches.md:167-169).
  - With imperative dialogs, two things would hold dialog state at once: the model's `NavState.dialogs`/`approvals` and rtwk's overlay stack. That is the oracle bug bc969b651 ("overlays keep their own request queue"; state-bug-classes.md:141), and it breaks acceptance criterion 1 (architecture.md:273-274).
- Fix: in ADR-002 replace the tree and table rows with this text: "Routes are a model enum (`NavState.route`); the root view matches on it and builds the screen from `&CodexApp`. Dialogs and approvals are a layer in the view (`Stack` + `ModalBarrier` + `Dialog`) built from the model's modal queue, head first (state-approaches §2 (d)). rtwk `Navigator`/`show_dialog` are used only for UI-only flows that read no app data (e.g. a confirm inside one widget)." Change design §2 r5 to "navigation and dialog widgets (barrier, focus return, transitions)", design §8 "Dialogs ... (show_dialog)" to "modal layer from `NavState`/`thread.approvals`", and the Skeleton line to "root: Theme, route match, modal layer".

## Major

**M1. Composer and per-thread widget state have no owner. rtwk `TextArea` lacks what the composer needs.**
- Where: architecture.md:140 (`TextArea`), :152-153 (widget state is rtwk's), R2 :300 ("per-thread data only in ThreadModel"); design.md:162 (draft and queue per thread), :191 (`selection` in NavState), :313 ("TextArea (+ vim from keymap), attachments").
- Evidence:
  - `TextArea` is uncontrolled. It has no value prop; its text lives in its state or in a shared `TextEditingController` (`Rc<…>`) (rtwk-editing/src/text_area.rs:174-327; text_editing.rs:418).
  - `SelectionList<K>` keeps its selection in a `SelectionController<K>` (`Rc`) (rtwk-controls/src/selection_list.rs:239; rtwk-data/src/selection_controller.rs:72).
  - So the draft and the selection exist twice, in the model (R2, NavState) and in rtwk. Keying the subtree by ThreadId (R2 `thread_scope`) also drops A's widget state on A→B→A. That is the mechanism of SC2b (e4a202ea5; eabb7c91d "kill buffer lost when the composer was replaced", state-bug-classes.md:126-127).
  - rtwk-editing has no vim, kill ring/yank, atomic elements (`[Image #1]`) or paste-burst handling (grep `vim|kill|yank|burst` in rtwk-editing/src: none). Ctrl+Y means redo in rtwk (text_area.rs doc).
  - reuse-map Top-10 #4 says the opposite: keep Codex's editing core, not rtwk TextArea (research/reuse-map.md:44-46).
  - None of these is in the rtwk gaps table (architecture.md:407-421).
- Fix: add a decision "Per-thread widget state". Recommended text: "Per-thread UI controllers (`TextEditingController`, scroll and selection controllers) live in a `ThreadUi` map keyed by ThreadId, in an R14-allowlisted UI module, outside the pure model. Drafts that the model needs (submit, queue) enter `apply` as an intent on submit. On switch the subtree is keyed by ThreadId and reuses that thread's controllers." Then either decide composer = carried Codex textarea core on rtwk primitives, or add gap G12 to the table: "TextArea: atomic elements, kill ring, vim mode, paste-burst, history recall — S10".

**M2. R14 does not make SC6a impossible.** (design.md:236, :260 put R14 under "impossible"; architecture.md:312.)
- Evidence: R14 bans only `Obs` (rtwk-framework/src/obs.rs:23) and `use_state` (rtwk-hooks/src/primitives.rs:44), via `clippy::disallowed_types`. App data can still reach:
  - a hand-written `impl State` (rtwk-framework/src/widget.rs:486, :523: Flutter's copy-in-`init_state` pattern);
  - rtwk controllers (`TextEditingController`, `SelectionController`, `ScrollController` rtwk-scroll/src/scrollable.rs:699);
  - `Rc` clones captured in handlers or route builders (see B1);
  - `TokioApp::follow`, which writes a watch into an `Obs` (rtwk-tokio/src/host.rs:111).

  `use_state` is a function, so `disallowed_types` cannot ban it; it needs `disallowed_methods`.
- Fix: R14 enforcement becomes "disallowed_types `Obs`; disallowed_methods `use_state`, `TokioApp::follow`, controller constructors outside the allowlist; CI grep: no `impl State for` / `impl StatefulWidget for` outside UI-only modules; screens take app data only as `&CodexApp` or memo deps". Relabel SC6a from IMPOSSIBLE to CAUGHT/linted in §6b and in architecture.md:287-289. The 7 SC6a commits move, giving 21 impossible / 32 caught / 2 possible.

**M3. R1's spawn lint covers one of at least six spawn paths, and `AppCx::spawn` is the wrong runner for tokio RPCs.**
- Where: architecture.md:232-235 (effects run "with `AppCx::spawn`"), :299 (lint on `AppCx::spawn`), :148-151 ("RPCs are started from callbacks"); design.md:133, :273.
- Evidence:
  - `AppCx::spawn` polls a `!Send` future on rtwk's executor (rtwk crates/rtwk/src/app.rs:1001-1013). rtwk-tokio's own docs say a future that needs tokio's reactor or timer "panics there unless a runtime's context is entered" (rtwk-tokio/src/lib.rs:4-7).
  - The codex client is tokio-based. Its clonable `AppServerRequestHandle` (codex-rs/app-server-client/src/lib.rs:366-372, :888) belongs in the shell, run through `TokioApp::spawn_tokio` / `subscribe` (rtwk-tokio/src/host.rs:28, :43; lib.rs:15-27).
  - These paths bypass a lint on `AppCx::spawn` alone: `TokioApp::spawn_tokio`, `subscribe`, `subscribe_with`, `rtwk_tokio::spawn_tokio` (component.rs:26), `Cx::spawn`, `std::thread::spawn` + `AppCx::channel` (the rtwk-weather pattern, src/app.rs:234, :269), and `tokio::spawn`.
  - Epoch: R1 says transport events "carry the thread's attach epoch" but not who mints it. If the client task reads a shared epoch at send time, an old subscription's late event gets the new epoch and passes the router.
- Fix: R1 reads "Effects run only in `shell`, through `TokioApp::spawn_tokio`/`subscribe` over the `AppServerRequestHandle`; lint `disallowed_methods` on `AppCx::spawn`, `Cx::spawn`, `TokioApp::{spawn_tokio,subscribe,subscribe_with,follow}`, `rtwk_tokio::{spawn_tokio,subscribe,follow}`, `std::thread::spawn`, `tokio::spawn`, `AppCx::channel` outside `shell`. Each subscription captures its epoch when it is created and stamps every event with that constant." Rewrite ADR-002's bullet at architecture.md:148-151 to match.

**M4. R4 does not make SC4a (live vs replayed duplicates) impossible: the backend synthesizes ids when it rebuilds history.**
- Where: architecture.md:304; design.md:116, :234, :258.
- Evidence:
  - `ThreadHistoryBuilder::next_item_id` returns `format!("item-{}", n)` (codex-rs/app-server-protocol/src/protocol/thread_history.rs:1724-1728).
  - It is used for user messages (:616) and for agent messages without a moderation id (:630-634).
  - Turn ids are generated when the rollout has none: `Uuid::now_v7()` or `rollout-N` (:1600-1606).
  - Live items carry other ids, so an upsert by `(TurnId, ItemId)` will not merge a resumed snapshot with live entries.
  - architecture.md:259-260 already admits that reconnect dedup needs content comparison today.
- Fix: add to R4 "A thread/read or resume snapshot replaces the affected turns wholesale (snapshot wins); it is never merged by id with live entries. Mandatory test: resume and reconnect mid-turn, transcript equal to the snapshot." Relabel the live+replay part of SC4a as CAUGHT.

**M5. The pending-request key misses connection scope, and the design mixes two kinds of pending request.**
- Where: design.md:138, :165 (`pending: map of ThreadId and RequestId to ticket`); architecture.md:301.
- Evidence: 6f47fcb0b, one of the cited SC3 cases: "different threads **or connections** can reuse request IDs ... retain the original app-server reply handle" (`git show 6f47fcb0b`). Client requests (our RPCs, `Reply<T>` tickets) and server requests (approvals, elicitations, user input, which the user answers) are different flows, but design §5/§6 put both under `pending`.
- Fix: in §6 split them into `outgoing: (ThreadId, Ticket) -> Ctx` and `server_requests: (ConnEpoch, ThreadId, ServerRequestId) -> Approval`. Approval replies are sent through the originating connection's handle. Entries of an old epoch are dropped when the epoch changes. R13 gets the example `ServerRequestId` scoped by connection.

**M6. ADR-001: the layout resolves only with codex-rs's `[patch]` and lockfile, and its manifest falls under repo-checks CI.**
- Where: architecture.md:95-114 ("no CI breaks"; the check list).
- Evidence. I ran `cargo metadata --offline` on a scratch workspace in `$TMPDIR` with path deps on app-server-client, -protocol, core, arg0, rtwk, rtwk-tokio and rtwk-rich:
  1. As written: fails. "package `codex-app-server-client` depends on `tokio-tungstenite` with feature `proxy` but `tokio-tungstenite` does not have that feature". The fork comes from codex-rs/Cargo.toml:684-700 `[patch.crates-io]` (crossterm, sqlx-core, sqlx-sqlite, tokio-tungstenite, tungstenite, v8, plus an `ssh://` patch), which Cargo does not apply to another workspace.
  2. With the patch copied, fresh lockfile: fails, "`spin = "^0.9.8"` version 0.9.8 is yanked" (via codex-network-proxy → rama-net → flume).
  3. With the patch and codex-rs/Cargo.lock copied in: resolves. 23 packages added (the rtwk crates). Only `miniz_oxide` and `wit-bindgen` gain a second version; tokio stays single (1.52.3); rtwk has no crossterm.

  `.github/scripts/verify_cargo_workspace_manifests.py:381-386` rglobs every Cargo.toml under codex-rs and requires `version/edition/license.workspace = true`, `[lints] workspace = true`, and no `[features]` or internal feature toggles (:6-9, :119-124; run by .github/workflows/repo-checks.yml:21). Editions (2024/2024) and toolchains (codex-rs rust-toolchain 1.99.1 ≥ rtwk MSRV 1.90) are fine.
- Fix: add to ADR-001 "tui-rtwk's Cargo.toml copies codex-rs `[patch.*]` verbatim (a check diffs them); its Cargo.lock is seeded from codex-rs/Cargo.lock and refreshed at each upstream merge. Either the manifest satisfies verify_cargo_workspace_manifests.py (own `[workspace.package]`/`[workspace.lints]`, `[lints] workspace = true`, no features on codex deps) or the crate lives outside codex-rs (repo-root `tui-rtwk/`)." Decide which before the skeleton.

**M7. The reuse-map the design rests on describes the earlier architecture, and REUSE rows are not copyable unchanged.**
- Where: architecture.md:28-33, design.md:84-88.
- Evidence:
  - reuse-map Top-10 #1 and #5 (history_cell, markdown, diff_render, wrapping, ~27k lines REUSE feeding a `LinesView`), #4 (Codex textarea), #7 ("rtwk `KeyBindings` unused") and #9 (`show_dialog` post-parity) (research/reuse-map.md:36-56) contradict ADR-004:320-323, ADR-006:339-342 and design §8. Its 35.7%/91,908 REUSE figure counts renderers that will not be carried.
  - Closure spot-checks at c6a41e32f. Each file below is REUSE but imports a module that is not:
    - streaming/commit_tick.rs imports `crate::history_cell`;
    - markdown_copy.rs imports `crate::clipboard_copy` (REPLACE) and `crate::terminal_hyperlinks` (REPLACE);
    - analytics/data.rs:5 imports `crate::tui::FrameRequester` (DROP);
    - app_server_session.rs (5,515 lines) needs session_state, backend_id, resume_permissions, legacy_core and lib.rs `start_embedded_app_server_for_picker`;
    - keymap.rs needs key_hint (crossterm `KeyEvent`, which brings the crossterm patch from M6).
- Fix: architecture §0 should name the carried set explicitly: "strict REUSE rows minus [core] content renderers; for each row its dependency closure is carried or replaced". Add a note atop reuse-map.md: "Top-10 #1, #4, #5, #7, #8, #9 superseded by architecture.md ADR-004/006 (2026-11-17)". Add a closure check to reuse_map.py.

## Minor

**m1. S26 is not a route.** `ui_tree` is the status-line element tree (used by bottom_pane/status_line.rs and chatwidget/statusline_reader.rs; ui_tree.rs:1 "Display elements for the terminal UI and their JSON representation"). Inline visualization is a transcript fallback (inline_visualization.rs:1; used from streaming/). Where: architecture.md:88; design.md:288. Fix: S26 becomes "ui_tree in the status line (part of S10 footer) + inline visualization (S11 content kind)"; drop the §7 `ui_tree` edge.

**m2. Startup screens are missing from the map, and the startup flow is drawn wrong.**
- Startup draft (composer usable before the app-server is ready; codex-tui-surface.md:37) and GovCloudGuidance (:39) are not in the map.
- design §7 draws startup as a linear chain ending in chat (design.md:281). That hides the draft composer and the "submit before session ready" race (SC9, f8b6b1db8).
- Fix: add both to S10/S02; add `SessionState { Starting, Ready, Failed }` to design §6 `Session`.

**m3. Clippy lints cannot be scoped to modules from config.** `disallowed_methods/types` are crate-wide. R1, R11, R12 and R14 say "outside the shell", "in views" or "in apply". So: deny crate-wide, `#[allow(clippy::disallowed_methods)]` on the allowlisted modules, plus a CI grep rejecting `allow(clippy::disallowed_` elsewhere. Also, a crate under codex-rs picks up codex-rs/clippy.toml (ratatui/sqlx entries, clippy.toml:8-30) unless it has its own. Where: architecture.md:299, :309-312; design.md:273-275. Fix: state this in the R-table "enforced by" column.

**m4. Stale text after ADR-003 was accepted.**
- architecture.md:43 and :231 have `apply(&mut Model, Ev) -> Effects`, but R3 (:301) has `apply(&mut ThreadModel, ThreadEv, &mut ThreadFx)`.
- :120 ("methods that change state and start work") and :148-151 contradict R1 and R10.
- design.md:67 "threads: per-thread state, buffering, replay" contradicts architecture.md:236-239 (no replay).
- design §6b maps b10 to "caught" (design.md:253, :266), but research says SC10a is IMPOSSIBLE (state-bug-classes.md:102). The diagram omits SC6b2/SC7c/SC4b (still possible).
- No apply is drawn for app-scoped events (account, rate limits, app-level MCP startup, 5a440c03f).
- Fix: unify on the R3 signature plus `apply_app(&mut CodexApp, AppEv, &mut AppFx)`; rename the module to "threads: per-thread models (no replay)"; add a "still possible" node to §6b.

**m5. R10's ordering is unspecified.** Handlers run `apply` at once, while channel items wait for the next flush (app.rs:1145-1151). An intent can therefore overtake backend events that were already received but not yet applied, which is the SC9 race. Fix: "handlers send `Intent` into the same `Sender` as the ingress router; apply order = arrival order".

**m6. `Versioned<T>` + revision memo is not what was measured, and it is not an rtwk type.** `memo` deps are `PartialEq + 'static` and are moved into the cache; the build is a `fn` pointer seeing only `&D` (app.rs:805-839). So a revision alone is useless as a dep; the data must travel too (a clone or `Rc` with copy-on-write). The prototype memoised cloned row values (app-state-proto dash_c.rs:72-78, c.rs:109-113). Fix: architecture.md:241-243 should read "memo deps are cheap per-row values or `(rev, Rc<T>)` over structurally shared items; `Versioned<T>` is an app type".

**m7. A `watch` in the model is outside `apply` and memo.** architecture.md:249 and design.md:184 put a tokio `watch` in the model. Reading it in a view is not tracked, and following it with `TokioApp::follow` lands in an `Obs`, which R14 forbids. Fix: "watch changes enter as events via `subscribe`".

**m8. The RTWK_REF pin is a convention only.** A path dependency builds whatever the rtwk checkout holds (architecture.md:100-101). Fix: depend on rtwk as `git = "file:///…/rtwk", rev = "675a98d7…"`, or add a build/CI check `git -C <rtwk> rev-parse HEAD == RTWK_REF`.

**m9. Numbers and waves disagree.**
- Skeleton is in W0 (design.md:347) but in ADR-009 step 1 (architecture.md:379).
- S14 is in W2 (design.md:348) but in step 3 (architecture.md:384).
- "17 slices" (design.md:349) vs 18-19 ids in S13–S26 + S01–S06.
- "~28 screens" (architecture.md:44) vs 24 rows in §1.
- The gaps rows at architecture.md:420-421 are labelled "ADR-003 option A", but the `Select` row concerns the reserve (family 4).

  Fix: align on one list.

**m10. Main and runtime topology are unstated.** `run_app_with` blocks and returns `ExitCode` (rtwk crates/rtwk/src/platform/app_start.rs:87). `arg0_dispatch_or_else` runs main inside a multi-thread runtime's `block_on` (codex-rs/arg0/src/lib.rs:194-213, :264). `rtwk_tokio::enter` needs a multi-thread runtime (rtwk-tokio/src/lib.rs:32-35, :186). Fix: in the Skeleton, write "main: `arg0_dispatch_or_else` → start client → `run_app_with` on the block_on thread with the runtime entered; shell uses `spawn_tokio`".

## Nit

- **n1.** `Tree` (architecture.md:141) is `TreeView<T>` (rtwk-data/src/tree.rs:1195).
- **n2.** `ListView.builder` (design.md:309) is `ListView::builder(count, Fn(usize) -> AnyWidget)` (rtwk-scroll/src/list_view.rs:310).
- **n3.** Composer popovers: rtwk-editing `Autocomplete` (autocomplete.rs:126) is the caret-anchored candidate; design.md:315 names only OptionList/CommandPalette.
- **n4.** `docs/rtwk-port/refs/` does not exist yet, though AGENTS.md and ADR-007 (architecture.md:361) refer to it.
- **n5.** design §3 screen folders (design.md:76-79) do not name S01 (update prompt) or S27; the S22 placement is listed as open (design.md:359).

## Checked and fine

- Screen homes: S01–S06, S10, S12–S25 appear in design §7; S11 in §8; S27 as `inline/` in §3. No orphan apart from S26's wrong kind (m1).
- §6b totals match research: IMPOSSIBLE 28 = SC1 6 + SC3 1 + SC4a 5 + SC5a 4 + SC6a 7 + SC10a 2 + SC13 3; CAUGHT 25; possible 2 (state-bug-classes.md:84-105, :176-181).
- Spot-checked cases:
  - SC1 c8949e55c and bf7c87a7b are covered by tickets + epoch, if M3 is fixed.
  - SC5a 44403dd3d (hook row left after TurnCompleted) is removed by R5's `Running` variant.
  - SC2a 1096d6453 (double TurnComplete via replay) is gone with no-replay.
  - SC3 6f47fcb0b holds only after M5; SC4a live+replay only after M4; SC6a 333663921 only per M2.
- rtwk names exist at the pin (rtwk HEAD = 675a98d73):
  - `run_app_with(state, ViewFn, RunOptions)` (platform/app_start.rs:87);
  - `ViewCx::on/on_with/memo` (app.rs:622, :637, :805), with memo build a `fn` pointer, so "a kept subtree uses only its keys or fails to compile" holds (app.rs:839-861);
  - `AppCx::spawn` (:1001, then gets `&mut A`), `AppCx::channel` (:1151), `unchanged` (:993); memo cache is a linear `Vec` search (:812-819), as claimed;
  - `overlay::show_dialog` (dialog.rs:301), `overlay::App` (rtwk-overlay/src/app.rs:39), `rtwk::navigation` (Navigator, Scaffold);
  - `Obs` (obs.rs:23), `use_state` (primitives.rs:44);
  - `provider`, rtwk-state `Source` (lib.rs:29), `Select` (:44), `store_provider` (provider.rs:50);
  - `rtwk_test::Tester` (tester.rs:298);
  - ListView, SelectionArea, MarkdownView, DiffView, CodeView, Chart/BarChart/Sparkline, Image, Spinner (rtwk-animation), ProgressBar, Focus/Shortcuts/KeyBindings, CommandPalette, OptionList, `SelectionList<K>`, `Table<T>`, TabBar, TextArea/TextField, Static, `ScreenMode::Inline`, `TerminalSession`, Localizations, Theme;
  - rtwk-tokio `TokioApp`.
- Edition 2024 on both sides; toolchain 1.99.1 ≥ rtwk MSRV 1.90; no duplicate tokio, crossterm or unicode-width versions introduced (M6 run).
- Reuse paths exist at the oracle pin: app_server_session.rs, app_command.rs, keymap.rs, markdown_copy.rs, clipboard_paste.rs, clipboard_copy/{primary,tmux,worker,wsl}.rs, slash_command.rs, updates.rs, thread_label.rs, session_resume.rs, mention_codec.rs, streaming/chunking.rs, analytics/data.rs. Note: clipboard_copy.rs itself is REPLACE, not REUSE.
- Oracle default full screen: `is_none_or(|tui| tui.fullscreen_transcript)` at core/src/config/mod.rs:4929-4932.
- design.html is in sync with design.md: running `design_html.py` rewrote it with no git diff.
- Open decisions listed (architecture.md:442-447, design.md:353-360) are real. Missing, and needed before the skeleton:
  - B1: route and dialog mechanism;
  - M1: composer and per-thread widget state;
  - M3/m10: effect runner and runtime;
  - M6: crate location vs repo-checks, patch and lock policy;
  - M7: which reuse rows are carried.

## Resolution (orchestrator, 2026-11-17)

Every finding is fixed in the docs unless marked otherwise.
- **B1:** reworked on the person's request after checking the twk and rtwk examples. twk's Dart StatefulWidget apps use `Navigator` + `showDialog`. rtwk plain-model apps use no `Navigator`, and they make dialogs either as a modal layer from the model (rtwk-weather, triage corpus view) or as widget-owned `show_dialog` with a ticketed close (triage's shipping view, delete.rs:50 + app.rs:105-118). The rule: route = model enum; live dialogs = modal layer; dialogs captured at open that return a value = `show_dialog` + `opened`/`closed(ticket, value)`. It is in architecture.md ADR-002 "Routes and dialogs" and the widget table, and in design.md §2 r5, §6 NavState/ThreadUi, §7 and §8.
- **M1:** ADR-002 has a new "Per-thread widget state" decision: `ThreadUi` controllers, and composer = rtwk `TextArea` + gap G12. design.md §6 has the ThreadUi class; §5 takes the text from ThreadUi.
- **M2:** R14's enforcement is widened. SC6a is relabelled CAUGHT in architecture.md, design §6b, state-bug-classes.md (correction note) and reports/data. The report is rebuilt: fix set 20 / 33 / 2, issues 8 / 21 / 5.
- **M3:** R1 lists every spawn path, and each subscription captures its epoch. ADR-002 and ADR-003 now say: the shell runs the client through rtwk-tokio.
- **M4:** R4 adds "snapshot replaces" plus the resume test. 6107b9886 and #2886 are now CAUGHT.
- **M5:** `outgoing` and `server_requests (ConnEpoch, ThreadId, ServerRequestId)` are split in ADR-003, R3 and design §6.
- **M6:** ADR-001 has "What the layout needs to resolve": copy `[patch]`, seed the lock, pass the manifest script as `codex-tui-rtwk`, a pin check, its own clippy.toml. The Skeleton lists all of it.
- **M7:** §0 names the carried set and the dependency closure. reuse-map.md has a superseded note. Not done: a closure check in reuse_map.py, left to the W1 refs task.
- **m1:** S26 is now part of S10/S11. **m2:** GovCloud is in S02 and the startup draft in S10. `Session.state` is drawn, and §7 shows S05 as a modal while Starting. **m3:** a note under the R-table. **m4:** signatures are unified, `apply_app` is added, threads are "no replay", and §6b has a still-possible node. R11 stays impossible, matching the research. **m5:** R10 shares the ingress sender. **m6:** memo deps are `(rev, Rc<T>)`. **m7:** watch changes arrive as events. **m8:** a pin test. **m9:** waves W0–W4 are aligned, with 18 slices. **m10:** main/runtime topology is in ADR-002 and the Skeleton.
- **n1–n3:** fixed. **n4:** the registry is created in W1 (`port-refs`). **n5:** design §3 names S01 and S27. S22's placement stays an open point.
