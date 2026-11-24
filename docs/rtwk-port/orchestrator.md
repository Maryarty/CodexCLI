# Orchestrator rules (main session only)

The SessionStart hook `.claude/hooks/orchestrator_context.py` injects this file
into the main session and stays silent in subagents (its input carries
`agent_type` there). Fleet agents run with `omitClaudeMd: true`, so AGENTS.md,
CLAUDE.md and the auto-memory index do not reach them either: what an agent
needs is in its agent file (`.claude/agents/<type>.md`) and the fleet brief
(`.tm/fleet-brief.md`, returned by `task_start`).

## What we build
THINK AS A FLUTTER DEVELOPER (the person, 2026-11-19): for every piece first ask how
you would write it in Flutter — which widget, controller (TextEditingController,
ScrollController, ExpansionController…), FocusNode/Shortcuts/Actions, LayoutBuilder,
Navigator, showDialog/showModalBottomSheet — then use rtwk's counterpart (rtwk is
Flutter's model on cells). Never translate the oracle's ratatui code line by line.
Pages vs modes vs sheets vs widgets: architecture.md §1 Amendment 2026-11-19b.

Screen first: the screens of the screen map (architecture.md §1) built as a
native rtwk app in the manner of `/Users/admin/Documents/dev_projects/rtwk-weather`.
codex-tui's reusable code (reuse-map.md REUSE/ADAPT rows: protocol and
session helpers, keymap, chunking, clipboard, markdown_copy, analytics data,
...) is carried over into `tui-rtwk`; its UI code is not — screens are built
of rtwk widgets. codex-tui itself is never edited. Backend crates are used
as they are. rtwk in one screen: `docs/rtwk-port/refs/rtwk-in-one-screen.md`.

## Architecture is enforced, not described
The architecture exists to make whole classes of bugs impossible, so every
decision that can be enforced by the compiler or a rules test IS enforced —
words in architecture.md are not enough (the person, 2026-11-19, after the
command center bypassed ADR-003 and hit "thread already has an active writer").
- Make illegal states unrepresentable: constructors and command variants
  exist only where the decision allows them (`pub(crate)`, module privacy,
  sealed enums); a screen cannot even name what it must not do.
- ADR-003, state: `CodexApp` owns one `ThreadModel` per thread, one writer,
  changed only through `apply`; `ThreadModel` is created only in
  `core/src/threads.rs`; thread RPCs (resume, turns, subscribe) exist only in
  `ThreadFx`; a screen asks the app (`OpenThread(id)`) and never resumes a
  thread or keeps its own copy of thread state.
- Layers (core pure, IO only in services/host, views without protocol) and
  rtwk-first (no hand layout, R15) are rules tests in `tests/rules` and
  `tests/rules/layers.rs`; a new decision comes with its test.
- Where a rule cannot be a type or a test yet, the brief says so and a debt
  task carries it (process.md A4.1 "ADR departures").

## Agents and tokens
- Types (the type fixes the model; `.claude/hooks/agent_models.py` refuses
  `model` on an Agent call and general-purpose agents, the person 2026-11-21):
  `codex-porter` (Sonnet, implementation), `codex-reviewer` (Opus: FULL and
  wave reviews, architecture, research), `codex-light-reviewer` (Sonnet: LIGHT
  reviews, short questions), `codex-waiter` (Haiku: long builds and waits),
  `codex-filler` (Haiku: one uniform batch item from a sample).
- One source per rule (2026-11-23 start-context review): porter process lives
  only in `.claude/agents/codex-porter.md`; the brief holds the rules all types
  share; a launch prompt or task description points to them and never
  restates them. Every restatement drifted.
- Needs first (the person 2026-11-24): per round ONE doors task builds the
  packages' actual NEEDS (core, the UI shared files, the test kit) as doors, an
  entry point, a hook, an accessor or a kit helper with one test each, and lands
  before any package starts; no package starts on an open need. A package that
  meets a need after it started writes it into its checkpoint (`Needs:`) and
  stops at it; the orchestrator appends those needs to the next round's doors
  task. A door is never a package feature: what a need turns out to be a feature
  of is said in the doors task's deliverable and left to its package
  (doors-r1: `tools/rtwk-port/doors-r1/`).
- A launch prompt is `tm task <alias>; worktree <path>; task_start`. The task
  description is the assignment, in this shape: Goal (1 line); Items, numbered,
  each with oracle origin and landing `file:line`; `read:` up to 8
  `file:line-line — what is there`; Sample (a finished commit or file to copy,
  not a design doc); Done-when (rule names, key tests); Area. A task without
  `read:` ranges makes the porter explore (settings-slice: first edit at call
  79 vs 14 with ranges).
- Launch agents with the session's cwd at the main checkout root, never inside
  a worktree: an agent inherits that cwd, and tm's PreToolUse hook then adds
  "needs no `cd`" (false for the agent) to every Bash call that starts with
  `cd` — 177 times in one porter.
- Agent files are tracked; after editing one, run
  `python3 -I tools/rtwk-port/agent-ctx-check/check.py` (tool lists vs what the
  harness gives, contradictions, sizes). A running session caches agent
  definitions: edits reach agents of sessions started after the edit.
- Cargo through `~/.cache/rtwk/acargo`, quiet output (`-q`, nextest
  `--status-level fail --final-status-level fail --hide-progress-bar`), no
  call over ~4 min (`~/.cache/rtwk/bgrun.sh`). Read big files in ranges.
- Each checkout builds in its own target (`codex-rs/tui-rtwk/target`; a shared
  target was dropped 2026-11-21: worktrees overwrote each other's test
  binaries). Backend crates come from `~/.cache/codexcli/backend-pin`
  (docs/rtwk-port/README.md), the same paths everywhere. acargo picks kache for
  a checkout with no target yet (since 2026-11-22) and keeps sccache for one
  that already has a target; both hit across checkouts. Never set
  CARGO_TARGET_DIR.
- An agent hands off at ~100 turns / ~250k context through
  `task_checkpoint`; the orchestrator at the end of a wave or ~230k (never
  past 350k) with a ≤40-line `## Now` handoff (`~/.cache/codexcli/handoff.md`).
