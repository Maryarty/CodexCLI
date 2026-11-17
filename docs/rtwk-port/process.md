# tui-rtwk — how the port is run

tm: `process-design`. This is the execution process for waves W1–W4
(architecture.md ADR-009): how a slice goes from tm to the integration branch,
who reviews what, what the person decides, how the port is measured while it
grows. Agents read their part in `.tm/fleet-brief.md` and their agent file;
this document is for the orchestrator and the person.

Prior art: rtwk's landing process (`rtwk/docs/process/landing.md`: one
maintainer integrates, one host check per candidate, retry a flake once) and
its measured fleet cost (`~/.cache/rtwk/process-cost/report.md`, 185 agents).
What carried over and why is said where it applies.

## 1. Roles

| who | does | never |
|---|---|---|
| the person | accepts tasks in tm (review mode `human`); decides delta kinds, architecture changes, scope, refs bumps, anything outward (§9) | — |
| orchestrator (Opus, main session) | keeps the front moving: writes briefs into tm, launches agents, rejects in tm on a FIX verdict, builds and lands candidates, rebuilds the registry, runs the wave-end checklist, hands over | writes slice code; accepts in tm; pushes |
| `codex-porter` (Sonnet, worktree) | one tm task: tests first, the slice, the parity pack, `task_done` | edits outside its area; merges; touches codex-tui |
| `codex-reviewer` (Sonnet; Opus where §5 says) | one review or one research question; verdict + findings | writes in the repository; accepts or rejects in tm |
| `codex-waiter` (Haiku) | one long command: a candidate check, the scenario suite, a perf run, a cold build | edits, commits, tm writes |

## 2. Branches and worktrees

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
  (§6), so the orchestrator's checkout is never in a half-merged state.
- **The oracle binary** is built once per `ORACLE_REF` by a waiter
  (release, into `~/.cache/codexcli/oracle-bin/`) and used by every pty
  scenario and perf run; no worktree builds codex-tui.

## 3. A slice, from tm to landed

```
pending ─ brief in tm (§4) ─ porter claims in a worktree
  ─ commit 1: parity tests, red (§4.2) ─ commits: the slice, green
  ─ parity pack (§4.3) ─ scoped + full checks ─ task_done → in_review
  ─ codex-reviewer: MERGE | MERGE-WITH-NOTES | FIX
       FIX → orchestrator rejects in tm with the findings → fix porter
             (same worktree) → task_done → review again (≤ 2 fix rounds)
  ─ candidate on the integration tip, in dependency order (§6) ─ land
  ─ registry row `built` ─ the person accepts in tm (now or at wave end)
  ─ deltas decided → registry row `parity-accepted`
```

**Landing does not wait for the person's acceptance**, except for the
skeleton (§7). Successor slices need their predecessors' code on the
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

## 4. What a slice carries in and out

### 4.1 Brief (the task's description in tm)

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
       mandatory tables extended (R2/R4/R5/R8/R9/R10, §4.4).
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

### 4.2 Tests first — the porter writes them, the oracle supplies the answers

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

### 4.3 Parity pack — `docs/rtwk-port/parity/<alias>.md`

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
  to the person (§9), batched per wave unless it blocks a slice. This is risk
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

### 4.4 Rules every slice is held to

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
- Carried oracle code keeps a comment naming its oracle file. codex-tui, the
  codex-rs workspace manifest and lockfile stay untouched.

## 5. Review

| what | reviewer | blocks landing |
|---|---|---|
| every slice | `codex-reviewer`, Sonnet | a defect; a missing pack item; an ADR-002 condition or R-rule broken; work outside the area; codex-tui or the codex-rs workspace touched |
| risky slices: skeleton, session-core, chat-cells (streaming tail), chat-composer, chat-transcript, approvals (security, typeahead G8), pager (live tail), inline-mode, perf-chat and perf-report (claims) | `codex-reviewer` with `model: opus` | as above, plus a doubtful performance or concurrency claim |
| the wave (§8) | `codex-reviewer` with `model: opus`, over the wave's diff | drift from the architecture or the manner; a rule enforced only on paper |
| the person | tm acceptance, deltas, §9 | the next wave; NEW delta kinds only where a slice cannot finish without the decision |

What does **not** block landing: visual deltas of a decided kind, deltas
proposed as accepted (they stay `built`, not `parity-accepted`), rtwk gaps
listed as deltas, review notes.

Verdicts: **MERGE**, **MERGE-WITH-NOTES** (notes go into a batched follow-up
task, §10), **FIX** (defects with `path:line` and the failing input). On FIX
the orchestrator rejects in tm with the findings as comments; a fix porter
takes the task (`task_start` re-grants it) in the same worktree. Two fix
rounds at most; a third is a stop (§9). A fix round's re-review reads only the
fix commits and the open findings.

## 6. Landing on the integration branch

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

## 7. Waves: agents in parallel

Limits: the machine (each porter builds; `acargo` caps memory and jobs), the
orchestrator's review and landing throughput, the person's acceptance.

| wave | in parallel | order |
|---|---|---|
| W1 | **skeleton alone** (Opus review, then the person accepts it: every slice copies its manner); beside it `codex-perf-baseline` (measures the oracle only) and `port-refs`; then session-core, keymap, scenario-harness (3); slash-commands after session-core. rtwk-gaps: coordination by the orchestrator with rtwk's orchestrator, no porter | ≤ 3 porters |
| W2 | chat-screen, chat-cells (split), chat-composer (3); then chat-popovers, chat-transcript, select-dialog; perf-chat after them | ≤ 3 porters |
| W3 | 18 slices: **4 porters**, 5 when ≥ 5 are ready and no more than 2 reviewed slices wait for landing. Risky and large first: approvals, settings (split), pager, resume, agents, sidebar; then the rest. Small ones (status-card, import, pets, small-dialogs) go two to one porter in sequence (one claim at a time) | 4–5 porters |
| W4 | inline-mode (Opus review) with the rtwk gaps it needs; parity-sweep as read-only Sonnet reviewers per screen group; perf-report by a waiter plus one porter | ≤ 3 |

The front decides what is free; the table caps how much runs at once.

## 8. Every wave end: the checklist

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
   reason is a stop (§9). No number reaches the person from a loaded machine.
5. **Bug-class proof** (state-bug-classes.md, `reports/tui-bug-classes.html`):
   the wave reviewer audits each R-rule → its enforcement in code (lint entry,
   test name, type) → present / not yet, and each sub-class counted "caught"
   → the mandatory test exists. The audit goes into the report's data
   (`reports/data/`) and the page is rebuilt (`build_report.py`), so the
   20 / 33 / 2 claim is backed by code at every wave, not only by design.
   Every state bug found in the port during the wave is classified against the
   SC taxonomy and added; one in a class counted "impossible" is a stop.
6. Opus wave review (§5).
7. The person: accepts the wave's tasks in tm; decides the NEW delta kinds;
   sees the perf report and the refreshed bug-class page.
8. Orchestrator hand-over: a fresh session per wave (§10).

## 9. When the orchestrator stops and asks the person

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
8. Moving `ORACLE_REF` or `RTWK_REF`, merging `main`/`upstream` into the
   integration branch, joining the codex-rs workspace.
9. Pushing, a PR, anything outward.
10. A wave's token spend above 1.5× the previous comparable wave (§10).
11. Wave gates: the skeleton's acceptance before W1 fans out; each wave's
    acceptance before the next fans out.

The orchestrator decides without asking: splitting a slice, the order inside a
wave, the number of porters within §7, review model per §5, landing, fix
rounds 1–2, batching follow-ups.

## 10. Token economy

From the economy rules (memory `codex-economy-rules`, task `token-economy`)
and rtwk's measurement:

- **Models.** Porters on Sonnet (a hook refuses an override; rtwk: Opus
  implementers cost 4.7× per kLOC). Opus only in §5's risky reviews and the
  wave reviews. Haiku waiters for every wait over ~4 min (0.06 M per wait vs
  ~1.4 M to re-write a Sonnet context).
- **Agent size.** ~100 turns / ~250k context, then `task_checkpoint` +
  `HANDOFF:`; rtwk: turns past 100 were 59 % of implementation cost, so
  slices are sized to fit (§3) instead of chained.
- **Lean starts.** Briefs in tm and short (§4.1); shared rules in the
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
  archive; W1 calibrates, later waves are compared with it (§9.10).

## 11. Additions to existing tasks this process needs

For the orchestrator to put into the briefs when the tasks start:
- `port-skeleton`: module stubs for every screen and a root variant per
  screen in the event and intent enums (§4.4); `tests/rules.rs` (§6);
  `docs/rtwk-port/parity/` with `delta-kinds.md` started; the first parity
  pack (`parity/port-skeleton.md`) as the format's example.
- `port-refs`: the registry tool also renders the parity packs into one
  side-by-side page (`reports/parity.html`) for the person's wave review.
- `scenario-harness`: scenarios named `<alias>-<name>`, runnable per slice
  (`-E 'test(<alias>)'`), the oracle taken from `~/.cache/codexcli/oracle-bin/`.
- `codex-perf-baseline`: the trend appends per wave (§8.4) with the load
  recorded.
