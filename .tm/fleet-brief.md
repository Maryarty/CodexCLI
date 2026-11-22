# Rules for every agent of the CodexCLI fleet ({FORK})

You are `{FORK}`. The tm hook supplies your session and subagent identity.
Your prompt carries only your task; the brief is the task's description in
tm, the working rules are in your agent file and here.

**How a screen is translated** — `docs/rtwk-port/process.md` Part 1: §2 (the
codex-tui → rtwk concept map) and §3 (the eight steps), with the worked
example in §4. Read them once before your first edit; a porter and a reviewer
both work by them. Appendix A (the fleet's process) is for the orchestrator;
read only the subsection this file points to.

## What the port is
- `codex-rs/tui` (ratatui) is the oracle at the commit in
  `docs/rtwk-port/ORACLE_REF`; `codex-rs/tui-rtwk` is its rtwk twin, one to
  one in behaviour. Architecture and decisions: `docs/rtwk-port/architecture.md`
  (read the sections your task names, by heading).
- rtwk API: `docs/rtwk-port/refs/` first (generated; grep it), rtwk source in
  ranges only for what the references do not answer. rtwk is read-only here.
- Behaviour, not mechanics (the person, 2026-11-18): match what the oracle
  shows and does, never how it does it. Scroll, selection, paging, editing,
  tabs, focus, mouse, widths and ellipsis belong to rtwk widgets and
  controllers (`/Users/admin/Documents/dev_projects/CodexCLI/.claude/skills/rtwk`, `guides/widgets-by-need.md`); no app-side
  shadow of their state, no carried oracle helper that duplicates them. If
  your brief or a research note names such a mechanism, use rtwk instead and
  say so in the handover. Process.md §5.
- Composition vs workaround (the person, 2026-11-21; process.md §5): rtwk
  widgets lay out and paint; the app only picks WHICH widget/spelling from a
  width it measured (`LayoutBuilder` + `segmenter_of(cx)`). Cutting, rewrapping
  or re-measuring strings yourself is a workaround: a gap row + `pending(Gxx)`,
  no code (G48 stays pending). Compositions: `widgets/dots.rs` `dots_end`,
  `pages/agents/tabs.rs`; workaround: `pages/startup/prompts/panel.rs` `words()`.
  Before a gap row, a quick check in your head: how would Flutter do it, then
  the same widget or a composition in rtwk (skill, grep `src/widgets`,
  `src/pages/agents`). No probe files, no file:line per route. A gap row is one
  line: the Flutter way, the rtwk widget tried, what differs (the person,
  2026-11-21). `pending(Gxx)` only on an open gap row.
- One list: rows with a cursor = `widgets/session_list.rs`, numbered options =
  `widgets/options.rs`. Controllers (`ListController`, `SelectionController`,
  `ScrollController`) are the only holders of cursor and scroll: no `top`,
  `window_top` or `rows[top..end]` in a view.
- ADR CONFORMANCE (the person, 2026-11-19): a brief that asks you to simplify
  or bypass an architecture decision (e.g. "own RPC path, no session-core",
  a screen-local copy of thread state) must name the ADR and its debt task; if
  it does not, stop and report instead of building it. ADR-003: CodexApp owns
  one ThreadModel per thread, one writer, changed only through apply.
- THINK AS A FLUTTER DEVELOPER (the person, 2026-11-19): for every piece first ask how
  you would write it in Flutter — which widget, controller (TextEditingController,
  ScrollController, ExpansionController…), FocusNode/Shortcuts/Actions, LayoutBuilder,
  Navigator, showDialog/showModalBottomSheet — then use rtwk's counterpart (rtwk is
  Flutter's model on cells). Never translate the oracle's ratatui code line by line.
  Pages vs modes vs sheets vs widgets: architecture.md §1 Amendment 2026-11-19b.
- How to build a view (the person, 2026-11-18): rtwk's standard means, looked
  up in the reference (rtwk skill + references) and, when needed, the gallery
  (rtwk-gallery pages and their Code snippets). Never copy a landed screen's
  view code — several are hand-made. "rtwk cannot" only after running the
  widget against the oracle's rows; then a one-line gap row (Flutter way,
  rtwk widget tried, what differs) + a pending(Gxx) test, the slice handed over with it,
  no local helper or workaround; the orchestrator tells the person, who
  decides whether rtwk gets it. Read process.md, this brief and the
  skill from the main checkout, not a stale worktree copy.
- The oracle's look and behaviour are the spec; no visual delta is accepted
  (the person, 2026-11-18). Never propose "accept" and never settle for the
  nearest ready widget that looks different (`Dialog`, `Autocomplete`):
  compose rtwk primitives until rows and styles equal the oracle's. Only what
  rtwk truly cannot do goes into the handover as "rtwk needs: …" with the
  rtwk widget you tried and a one-line `docs/rtwk-port/rtwk-gaps.md`
  row; stop and report it rather than writing it off as a delta.
- The oracle's files, snapshots and tests for a screen: its rows in
  `docs/rtwk-port/refs/codex-registry.csv`. The oracle binary for pty
  scenarios: `~/.cache/codexcli/oracle-bin/` (never build codex-tui yourself).
- Never edit `codex-rs/tui/**`, `codex-rs/Cargo.toml` or `codex-rs/Cargo.lock`.

## A page is done when (process.md §A3, §A4)
1. **The oracle shard is on**: every test of the page's shard green, or
   `pending(Gxx)` on an open gap row; `shards.py --check` clean. Tests the
   shard lacks (state, mouse) are written before the code and shown biting.
2. **Mouse** (ADR-011): every view with selectable rows, tabs or buttons
   takes tap/double-tap/wheel, with rtwk_test mouse tests.
3. **ADR-002 conditions and ADR-003 R1–R14 hold**; the rules tests are green
   (they also hold the review checklist: one list mechanism, no hand
   windowing, no hand width math, pending only on open rows).
4. **You ran it**: your own `codex-rtwk` in the page's pty scenario beside
   the oracle binary, keys and mouse on the main path; the differing rows of
   one checkpoint in the pack (process.md §A4.3).
5. Checks green (below); copied oracle code keeps a comment naming its file.


## Checks: scoped while you work, full once at the end
All from `codex-rs/tui-rtwk`, through `~/.cache/rtwk/acargo`, in your
worktree's own target (`codex-rs/tui-rtwk/target`; never set
CARGO_TARGET_DIR). Your binary is `<worktree>/codex-rs/tui-rtwk/target/debug/codex-rtwk`;
pass it as `RTWK_BIN` to the scenarios.
- While iterating (build-speed-2: 10 to 15 s from edit to result instead of 25 to 110 s): type
  errors first, `acargo check --tests --quiet` (2 to 7 s warm); then ONE test root, not all seven:
  `acargo nextest run --test screens --status-level fail --final-status-level fail
  --hide-progress-bar -E 'test(<page>)'`. The root is where the test file is registered
  (docs/rtwk-port/build-speed.md): `screens` (tests/pages, sheets, app), `shell`, `rules`,
  `pages` (probe files), `oracle`; core tests: `-p codex-tui-core`. Only that binary is
  compiled and linked. Lints: `acargo clippy -q --all-targets -- -D warnings`.
  The old all-roots form `-E 'test(<page>) | binary(rules)'` still works and is the slow one.
  A new worktree's first build is ~1 to 2 min now (sccache hits across worktrees, acargo wires it);
  never export CARGO_TARGET_DIR (it makes every crate miss).
- A partial (a step handed on as `partial ready: <tip>` in a checkpoint)
  carries the scoped checks above only — fmt, check --tests, your test
  root(s), clippy `-q --all-targets`, `shards.py --check` if a shard file
  changed — then commit and checkpoint. NO clippy `--workspace`, whole
  nextest or oracle run per step or per partial: the landing's
  `land-check.sh` is the partial's full gate, and a red there comes back to
  you. A brief that asks for the full gate after each step is outdated —
  follow this rule (measured 2026-11-22: 31 % of porter time).
- Once, before `task_done`: rebase onto the integration tip (the orchestrator
  names it in tm notes), then `acargo fmt -- --check`, clippy `--workspace`,
  the whole nextest, your oracle shard(s) with `--profile oracle`,
  `shards.py --check`, your pty scenario on both binaries. Long runs through
  `~/.cache/rtwk/bgrun.sh`.
- Known reds: `~/.cache/codexcli/known-red.txt`. Do not re-verify them. A
  red not on the list is yours, unless one run on the tip without your
  change shows it red: name it in the handover.


## Layers (process.md §A4.4)
- Core (`codex-tui-core`, no rtwk): models, `apply`, tickets, protocol
  mapping, command parsing. UI: view files only (widgets, `strings.rs`, debug
  entries) that never name the protocol, `shell`, tokio or std IO; UI-only
  state lives in `UiApp.ui`, never in the core model. `shell` runs effects and
  names no widget.
- Mock and fixture state is a JSONL event script applied through core
  `apply`, built from the oracle's test data; never data handed to a widget.
- If the brief seems to ask you to cross a layer, stop and say so in the
  handover; do not build it.

## Where your work lives
- Stay inside your task's area. The brief names each shared file's owner;
  a shared file owned by another task is read only, and what you need from
  it goes into your handover. Never edit `lib.rs`, a parent `mod.rs` or a
  root event/intent enum outside your area.

- A new dependency: a separate commit with `Cargo.toml` + `Cargo.lock`, named
  in the handover.
- Do not commit `docs/rtwk-port/refs/codex-registry.csv` or
  `parity/delta-kinds.md`: the orchestrator rebuilds them.
- Commit on your worktree branch, staging by path, messages `<alias>: …`.
  Never merge (not the integration branch, not a sibling's branch); rebase
  onto the integration tip before your full check and when the orchestrator
  asks. Never push.
- Never accept or reject in tm, your own work included.
- Read the `notes` tm hands you on each call before your next edit.

## Handover
`task_done` with deliverables: the commits, the parity pack path, and a
summary of at most 15 lines: shard counts, what is left and why, gap rows
added, departures with their debt task, new reds (not known ones), the
binary run's result, what others must change. A fix run answers each review
finding by `path:line` and commit. Your brief names the review tier
(process.md §A5): stay inside what it allows.
