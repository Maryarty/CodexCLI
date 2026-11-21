# Process review, 2026-11-21: where StartupPage lost its time, and a lean process

Research for the person (codex-reviewer, Opus). Question: the port takes longer
than the oracle's screens took to write. StartupPage (S01–S06) took about
half a day over about 15 small tasks. Where did the wall-clock time and the
quality go, and what process is fastest without lowering quality? The
architecture rules that tests enforce (ADR-003, layers, R15) stay as they are.

Already decided on 2026-11-21 and taken as given here: the gap check is light
(Flutter, then rtwk, one-line row); StartupPage is review tier LIGHT; one page
or feature per porter, end to end, and the porter fixes its own findings; the
shared target is dropped; planning for speed is the orchestrator's job.

Sources: tm `get_task` on the nine tasks named in the prompt (submissions,
reviews, timestamps); `git log` of `claude/tui-rtwk`, `land/round-2` and the
`claude/<alias>` branches (commit times, `--shortstat`); `docs/rtwk-port/reviews/`
(batch-wave2-startup, batch-fix1-*); `~/.cache/codexcli/handoff.md` and
`handoff-archive.md`; the land-check logs `/tmp/land-r2-{a,b,c}.log`; the
process text (process.md, fleet-brief, codex-porter.md, codex-reviewer.md,
land-check.sh, acargo). `list_tasks` is not in this agent's tool set (the
tm CLI hook sends it back to MCP), so the task list comes from git, the
handoffs and the `found`/`foundDuring` links of the tasks I read.

## Synthesis

The porters were not the bottleneck. They wrote StartupPage's first version
(5 slices, +15.2k lines) in **2 h 44 min** of commit time. Everything after
that took the other **~12 h**:
- **Rework.** 54 % of all porter time and 36 % of all inserted lines went into
  9 fix and follow-up tasks after the first build.
- **Waits between rounds.** Batch review, landing and land-check came three
  times, and each round waited for its slowest member.
- **Process churn.** Six process changes landed in one day, two of them
  reversed within 2.5 h, with porters in flight.
- **Infrastructure.** A load average of 139 on 10 cores; a land-check that
  is red on every run because of known failures; flakes re-verified by every
  porter and reviewer; a shared target that clobbered binaries for 86 min.
- **Five orchestrator sessions in ~6 h.**

Quality was not bought by the extra cycles. Of 11 blocking review findings,
one was a code defect. Two of the three fix-round-1 FIX verdicts were about
the wording of gap rows. The one real gap, no mouse anywhere on StartupPage,
was found by the person in the binary, not by any review. No porter had run
the binary.

The lean process below therefore:
- **Makes the page the unit.** One porter builds the whole page and fixes its
  own findings.
- **Turns the review checklist into tests.** What a reviewer catches by
  reading becomes a rules test, so reviewers look only at behaviour and ADRs.
- **Makes the porter prove the page in the real binary**, beside the oracle.
- **Lands continuously** on a check that is red only for new failures.
- **Caps concurrent builds** to what the machine can do.
- **Freezes process changes** while a page is in flight.

## (a) Measured findings

### A1. Timeline of StartupPage (commit times, 2026-11-20/21)

| window | what | porter commit time | idle / overhead after it |
|---|---|---|---|
| 23:32–02:16 | wave 2: select-dialog, startup-flow, startup-prompts, import, onboarding (5 slices, 21 commits, +15,155 / −619) | 2 h 44 | 02:16 → 08:40 = 6 h 24 (overnight, batch review wave 2, landing 07:08–07:19, gap research G44–G48) |
| 08:40–10:28 | fix round 1: startup-deltas, startup-prompts-rest, providers (partial), import-rest (47 commits, +6,037 / −1,624) | 1 h 48 | 10:28 → 11:44 = 1 h 16 (landing 10:26–10:58 incl. landing fix 451d7698d, shared-target 11:23, batch review fix 1, new tasks 11:16) |
| 11:44–13:09 | round 2: startup-fix-2, import-review-ctrl, select-rtwk-list, startup-mouse, flaky-apply-wait (17 commits, +2,545 / −806) | 1 h 25 | 13:09 → 14:10+ (landing round 2: land-check run 3 times; `land-r2-c` still running at handoff) |
| 14:10– | startup-finish + startup-wizards still running | — | StartupPage not yet green |

- **Wall clock**: >14.5 h from the first commit, and the page is not done.
  Porter commit windows add up to 5 h 57. The rest is review, landing,
  research, waiting and the night.
- **Rework share**: 3 h 13 of 5 h 57 of porter time (54 %); 8,582 of 23,737
  inserted lines (36 %).
- **Tasks per screen**: 16 implementation tasks for 6 screens (S01–S06): the
  5 builds, 9 fix/rest tasks, and 2 still running, so ~2.7 tasks per screen.
  Several more were folded or cancelled (providers, startup-orchestration,
  startup-config-reload, select-keymap, gaps-close-w2 narrowed). On top came
  ~9 process or infrastructure tasks the same day: review-tiers,
  rtwk-first-help (with its research), light-gap-check, startup-light,
  shared-target, target-per-worktree, the gap research G44–G48, flaky-status,
  fake-loop-barrier.
- **Cycles per screen**: 3 build→review→fix cycles, with a 4th (startup-finish)
  running. Every screen of the page went through all of them, because the
  rounds were cut by finding type, not by screen.

### A2. What the reviews found: form vs real defects

Wave-2 batch review (`reviews/batch-wave2-startup.md`): 5 FIX verdicts, 9
blockers.

| kind | findings | count |
|---|---|---|
| code defect | O1 (double activation starts two logins, orphan never cancelled) | 1 |
| visible delta vs oracle | P2 (OSS rows' hotkey/status), I1 (source prompt as generic sheet), O3 (logo fade) | 3 |
| rtwk-first / structure (no user-visible effect) | S1 hand windowing, F1 own highlight enum, F2 hand height math, P1 `words()` workaround, O2 hand sign-in menu | 5 |
| real behaviour risks, non-blocking | R-F1 update-slot ordering, R-F3 stuck Saving, R-F5 chat flash, N-O1 empty wizard page, N-O2 blocking browser call | 5 |
| **missed** | no StartupPage view took the mouse (ADR-011, architecture.md:707), on every screen | the person found it |

Fix-round-1 reviews (`reviews/batch-fix1-*`):
- 2 FIX verdicts, **both on gap-row documentation** (the three routes with
  file:line for G49/G50/G51), with 0 code defects; 2 MERGE-WITH-NOTES.
- The FIX turned into startup-fix-2: 7 commits, among them 11 probe tests in
  `tests/rtwk_g49_g51.rs` and file:line per route.
- **51 min after** that work landed, light-gap-check (369185681) declared
  probes and per-route file:line unnecessary.

Reading: the structural findings (S1, F1, F2, O2, P1) are exactly the
patterns the rtwk-first-stumbles research ranks 1–4. All of them could be
rules tests: window state in a view (M2), hand width math (R15 patterns), a
second list mechanism. They were caught one review round late, each costing
a fix task. Real defects (O1, R-F1/F3/F5, N-O2, mouse) are behaviour that
shows when the page is driven: in the binary, or in an in-process test that
presses keys twice, clicks, or drops the connection.

### A3. Landing and check waits

- **The land-check is red on every run** (`== scenarios rc=1` in land-r2-a,
  -b and -c). The cause is the pty scenarios error-context-window and
  multi-agent, which fail on the base and have no baseline file. So
  `ALL fail=1` always, and the orchestrator reads every log by hand.
- **One run of three was red on a known flake**
  (`import_is_a_step_of_the_startup_page`, TRY 2 FAIL).
- **The full nextest is cheap**: 1,037 tests in 44–53 s. The cost is the
  builds (clippy, nextest, the binary) under load, not the tests.
- **A run took over 14 min** under load 139: land-r2-c was still at
  `build-bin`/`scenarios` 14 min after the candidate commit.
- **Land fixes**: a semantic conflict between two fixes of one flow in two
  tasks needed 451d7698d (O1 CancelLogin vs the N-O2 test). Round 2 needed
  glue at landing (`on_accept` in trust.rs and import/source.rs), because
  those files belonged to other running porters.
- **Rounds are synchronous.** A round lands when its slowest member is in, so
  a 12-min task (import-review-ctrl, one commit at 12:01) waited until 12:48
  and 13:41.

### A4. Splitting one page across porters

- **Briefs had to ration files.** "You own host/src/startup.rs this round;
  providers may add only dispatch arm(s)" (startup-deltas). options.rs
  belonged to startup-prompts-rest while import-rest, providers and
  startup-deltas all needed it ("read it, do not edit it"). trust.rs and
  import/source.rs were held by other porters during startup-mouse.
- **providers stalled.** It waited on startup-deltas'
  `core/src/screens/onboarding.rs`, handed over PARTIAL, and was then
  cancelled. Its plan was re-split into startup-wizards.
- **Edits leaked across worktrees twice.** startup-deltas' edits reached the
  main checkout. import-rest's edits landed in the startup-deltas worktree.
  Both are recorded in the handovers.

### A5. Infrastructure breakage and load

- **Shared target**: from 11:23 to 12:49 (86 min), worktrees overwrote each
  other's test binaries and libs. Then reverted (b08e532c0).
- **Disk**: hit 100 % at 10:40 (4.7 G free), and targets were deleted by hand.
- **Release build killed**: rc 137 under 5 parallel porter builds and the
  memory cap. The person's release binary is from Nov 19.
- **Load average 138.95 / 120.11 / 99.80 at 13:59 on 10 cores.** acargo caps
  each agent at 3 jobs, but with 4–5 porters, a land-check and sccache the
  machine runs at roughly 10× its core count. Every build and every test
  timing is several times slower, and the timing flakes come from it: the
  status_data and shell_agents flakes all pass at `--test-threads 1`.
- **Flakes were re-verified by each agent.**
  - Flaky tests are named in 8 of the 9 StartupPage handovers.
  - startup-prompts-rest built a temporary worktree to prove base failures.
  - import-review-ctrl reran at `--test-threads 1`.
  - The fix-1 reviewer reran nextest 3 times and a loop of 6.
  - No file lists the known reds, so the same proof is repeated by every
    agent.

### A6. Orchestrator and process churn

- **Five orchestrator sessions in ~6 h**: Now sections at ~08:35, ~11:20,
  ~12:30, ~13:00 and ~14:10. Each starts with a re-read. Subagent reports
  go to the session that launched them ("their reports reach the old session
  only"), so the next orchestrator polls tm and branches.
- **Process commits on one day**:
  - 736f9610c review tiers (10:07);
  - ffa46cd52 3-route checklist (10:18);
  - 6b1fb3955 shared target (11:23);
  - 369185681 light gap check (12:48), which reverses 10:18;
  - b08e532c0 target per worktree (12:49), which reverses 11:23;
  - 630269c99 StartupPage LIGHT (12:50), which re-tiers the briefs of
    running tasks.
- **Effect of the churn.** Porters started under one rule and handed over
  under another. Reviewers judged by the rule of the day: the fix-1 FIX on
  routes came 2.5 h before the routes were dropped.
- **Text every porter reads before its first edit**: ~450 lines (fleet-brief
  150, porter 141, process §2–§4 ~160). The person's 2026-11-18/19/21
  paragraphs are repeated in 4–5 files (AGENTS.md, fleet-brief, porter,
  reviewer, process §5).
- **Ceremony per slice**:
  - a red-first commit, even where the oracle shard (`tests/oracle/*.rs`
    with `kit::pending`) already holds the oracle's expected values. That
    gave 28 commits for startup-deltas (red/green per item).
  - a 100–166-line parity pack whose snapshot and oracle-test tables repeat
    what `shards.py` and the shard already know;
  - delta-kind entries in a phase where no delta is accepted.

## (b) The lean process: each change, what it solves, what it costs

### L1. Unit of work: one page or feature, one porter, done in the binary
- **Change.** A task is a page (StartupPage) or a feature (composer,
  transcript), end to end: core state, views, host/services, mouse, the
  oracle shard and the pty scenario. The same task keeps its own fix loop.
  A finding comes back to the same task, as a re-grant in the same worktree
  from its checkpoint. No "-rest", "-deltas" or "-fix-2" tasks. "Done" means
  the oracle shard is green (pending only on open gap rows) and the porter
  has driven the page in the real binary (L6).
- **Solves.** The 2.7 tasks per screen, the rationing of files, the stalled
  partials (providers), the landing glue, and fix tasks cut by finding type.
- **Costs.** A page can exceed one agent's ~250k. Then the porter
  checkpoints and a fresh porter continues the same task (one re-read). This
  is cheaper than a new task with a new brief, review and landing. Large
  features (chat cells) are split by disjoint files at planning time (L2),
  not by finding type.

### L2. Parallel split by disjoint areas, decided at planning time
- **Change.** Before a page fans out, the orchestrator writes an area map: for
  each porter its files, and for each shared file exactly one owner.
  Typical shared files: `widgets/options.rs`, `core/src/screens/<page>.rs`,
  `host/src/<page>.rs`, `rtwk-gaps.md`, `shards.py`, `Cargo.lock`.
  - A shared widget two pages need is either built first by its owner, or
    the second page starts after it lands.
  - Append-only shared files (gap rows, shard registration) are written by
    the porter as a section in its pack, and the lander copies them in.
  - Two porters never share a page.
- **Solves.** The rationing clauses in briefs, the cross-worktree leaks, and
  the semantic conflicts at landing (two fixes of one flow in two tasks).
- **Costs.** 10–15 min of orchestrator planning per page, plus sometimes
  less parallelism (a page waits for a shared widget). Today parallelism
  bought little anyway: L5 caps builds at three.

### L3. Review tiers, and when a review may block
- **Change.**
  - **TRIVIAL**: no review.
  - **LIGHT** (the default for pages): one Sonnet pass over the page's diff
    after it lands, and it never holds the landing.
  - **FULL**: before landing; only the risky list (session-core, streaming
    tail, composer, transcript, approvals, pager, inline, perf claims).
  - A review blocks, or sends the task back, only for:
    1. a defect with its failing input;
    2. an ADR violation without a debt task (ADR-003, layers);
    3. a user-visible difference from the oracle with no gap row;
    4. work outside the area, or in codex-tui.
  - Everything else is a note, handed to the page's next task or to the
    wave's follow-up: rtwk-first structure that no rule catches yet, pack
    form, gap-row wording. A structural finding a rule could catch also
    becomes a rules test (L4).
  - Reviews are per page, as soon as it lands, not per round.
- **Solves.** FIX rounds about form (fix-1: 2 FIX verdicts on gap-row text,
  0 code defects), and waits for a whole round to land before review.
- **Costs.** A structural smell can live on the branch for a while as a
  note. It is fixed by the page's owner, or by the rule that catches it.

### L4. The checklist becomes rules tests, so the review reads behaviour
- **Change.** Each recurring review finding becomes a test in `tests/rules`,
  owned by one small task per wave (rules-wave2 already holds M1/M2):
  - **ADR-011**: a view with selectable rows has a mouse test (scan for
    `Options`, `session_list` and `GestureDetector` users vs `tests/pages/*mouse*`).
  - **M2**: no `top`, `window_top` or `rows[a..b]` in a view.
  - **R15 patterns**: `segmenter_of(`, `string_width(`, `measure::`,
    `TextMode::` outside a `LayoutBuilder` branch.
  - **One list mechanism**: no new `Highlight`/cursor enum beside
    `options.rs`/`session_list.rs`.
  - **`pending(Gxx)`** only on an open row (M4).
  - **M3**: every core `Command` has a fake-backend arm.
  - **Pack completeness**: `shards.py --check` already maps every oracle
    test. The pack's tables become its output, not hand-written.
- **Solves.** The 5 structural blockers of wave 2 and the mouse miss, which
  all show up at `nextest` in the porter's own run, not one round later.
  The architecture stays enforced in code, as AGENTS.md demands.
- **Costs.** About one porter-hour per wave for the rules, plus some false
  positives handled by a small allowlist with a reason.

### L5. Test strategy: scoped while working, full once; a baseline of known reds; three builders at most
- **Change.**
  - **While working**: `nextest -E 'test(<page>) | binary(rules)'` (the
    rules tests are seconds) and `clippy -q --all-targets` on the page's
    crate.
  - **Once before handover, on the tip** (the porter rebases onto the
    integration tip first): fmt, clippy `--workspace`, the full nextest
    (~50 s of tests), the oracle shard(s) of the page, and its pty scenario.
  - **Known reds.** The orchestrator keeps one file of known reds,
    `~/.cache/codexcli/known-red.txt`: test names and scenario names, each
    with a tm bug.
    - nextest: a known flake gets retries in a profile, or `#[ignore]` with
      its bug.
    - check.py / land-check: compare against the file, so the result is
      "new red: none".
    - Nobody re-verifies base failures again.
  - **Machine load**: at most 3 concurrent cargo builders on this
    10-core machine, a land-check counting as one. The orchestrator holds
    the rest of the queue.
- **Solves.**
  - the red-on-every-run land-check;
  - flakes re-proved by every agent;
  - the timing flakes caused by load 139;
  - the rc 137 kills;
  - builds several times slower than they need to be.
- **Costs.** A file to maintain, and a flake can be hidden longer: each
  entry carries a tm bug, and the file is reviewed at wave end. Fewer
  porters at once, but each finishes faster. Today 5 porters at load 139
  were not faster than 3 at load ~30.

### L6. Porters run the binary themselves
- **Change.** Before `task_done` the porter:
  1. builds `codex-rtwk` in its own worktree target;
  2. runs the page's pty scenario against both binaries (`check.py --bin rtwk
     --bin oracle --only <page> --capture-dir /tmp/<alias>-cap`, with
     `RTWK_BIN` set to its own binary and `CODEX_HOME=$(mktemp -d)` for
     first-run pages);
  3. walks the page's main path with keys and the mouse: in the pty where
     the driver supports it, otherwise as rtwk_test mouse tests over the
     whole page;
  4. puts the two captures of one checkpoint side by side in the handover:
     5–10 lines, the differing rows only.
- **Solves.** The mouse miss, the "chat drawn before the prompt" kind of bug
  (R-F5), and the update offer that never shows (0.0.0). These are
  behaviour that reading misses. The handover proves the page works,
  instead of claiming it.
- **Costs.** 5–10 min per page. A scenario script per page, which A4.3
  asks for anyway.

### L7. Landing cadence: continuous, per page
- **Change.**
  - A page lands when its porter hands over: no round, no batch.
  - If the tip has not moved since the porter's rebase-and-full-check, its
    check is the candidate check (A6 step 3, "a check that covered the exact
    tree is reused"): fast-forward after the ADR check and the diff-scope
    check.
  - If the tip has moved, the waiter merges and runs land-check once against
    the known-red baseline.
  - Several handovers that arrive together share one candidate.
- **Solves.** The ~1–6 h gaps between porter windows, and the 12-min task
  that waited until 13:41.
- **Costs.** More small landings, each cheap when the tip has not moved. A
  later landing can still be red because of an earlier one. Then the earlier
  page's owner fixes forward (as today).

### L8. The orchestrator plans; process changes only at page boundaries
- **Duties per page.**
  1. A page plan: screens, oracle shards, shared files and their owner (L2),
     what must land first. Count the oracle tests and the snapshots, so the
     porter's run fits.
  2. A brief that front-loads the recurring findings: mouse, one list, no
     hand windowing, composition vs workaround. Each one is named with the
     rules test that now enforces it.
  3. The concurrency cap (L5) and the queue behind it.
  4. Keep `known-red.txt` current.
  5. Land continuously (L7) and route review notes to the page's owner.
  6. Estimate in porter commit time, measured against the A1 table (the
     first version of StartupPage took 2 h 44).
  7. Keep the screen map current (the person's wish).
- **Process changes.** A change the person decides applies to tasks handed
  out after it. Running tasks finish under the rule they started with,
  unless the person says otherwise. The orchestrator collects changes and
  applies them at the next page boundary, in one TRIVIAL docs commit.
- **Orchestrator context.** Read handovers of ≤15 lines and review verdicts
  only (no review texts, no diffs). The land-check goes to a waiter. With
  far fewer tasks, the handoffs fall from five a day to about one per page.
- **Solves.** The two reversals within 2.5 h, porters judged by a rule newer
  than their brief, and five orchestrator sessions in 6 h with reports lost
  to dead sessions.
- **Costs.** A rule the person wants now reaches running porters later.
  Where the person wants it now, the orchestrator says so in a tm note to
  the running task.

### L9. Ceremony to drop
| drop | why | instead |
|---|---|---|
| separate red-first commit when the oracle shard exists | the shard already holds the oracle's expected values (`tests/oracle/*.rs` + `kit::pending`); turning a pending test on is the bite. It yielded 28 commits for startup-deltas | tests first stays for new non-oracle tests (state tests, mouse): red then green inside the branch, or a broken-code bite named in the handover |
| hand-written snapshot and oracle-test tables in the pack | `shards.py --check` and the shard know them | pack = shard counts (generated), a decisions section (departures, gap rows, n/a reasons sampled), the binary capture (L6). ≤40 lines |
| batch rounds and the "≤2 fix rounds per batch" counter | rounds wait for the slowest member and cut fixes by finding type | per-page review (L3), with fixes in the same task |
| delta-kinds work in this phase | no delta is accepted (the person, 2026-11-18) | a gap row or a fix; delta-kinds is kept only for decided renderer kinds |
| FIX for gap-row form, probes, file:line per route | reversed by the person (light-gap-check) | one-line row; a reviewer never asks for probes |
| reviewers re-running the whole suite | the porter's full run and the land-check already ran it | the reviewer runs the page's tests and breaks one thing when a bite is in doubt |
| registry rebuild per landing | a shared-file touch every landing | at wave end |
| repeated person-dated paragraphs in four files | ~450 lines before a first edit | one copy in fleet-brief "The rules" (porter and reviewer files point to it). The architecture rules are already tests |
| briefs drafted by a reviewer agent | an extra agent start per wave | the orchestrator writes the page plan (L8). For a page the brief is short |

### Expected effect (estimate, not a measurement)

| | StartupPage measured | lean target |
|---|---|---|
| implementation tasks per page | 16 (+9 process/infra) | 1–3 (one per disjoint area), plus a continuation where the context runs out |
| build→review→fix cycles | 3–4 | 1 build, with 0–1 fix runs by the same porter |
| blocking review findings that are form | 7 of 11 (S1, F1, F2, P1, O2, two gap-row FIX) | ~0: structure is caught by rules tests in the porter's run |
| land-check result | red on every run (known reds) | red only for new reds |
| wall clock for a StartupPage-sized page | >14.5 h | ~4–5 h: ~3 h porter (first version measured at 2 h 44, plus mouse and finish), ~30 min check and landing, a non-blocking review in parallel |

## (c) Replacement text, ready to paste

Each block replaces the named section whole. Sections not named stay as they
are. Part 1 §5 (what the translation never does) stays unchanged: it carries
ADR-003, the layers, R15 and the composition/workaround boundary.

### C1. process.md, Part 1 §3, step 3 (replace the paragraph "**3. Write the tests, red** …")

```markdown
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
```

### C2. process.md §A3 (replace the whole section, "Size" included)

```markdown
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
```

### C3. process.md §A4.1 (replace the brief template and the two paragraphs after it)

```markdown
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
```

### C4. process.md §A4.2 (replace)

```markdown
### A4.2 Tests and checks — scoped while working, full once

- Expected values come from the oracle (shard, `.snap`, assertions, captures
  of the oracle binary), never from the new code's output.
- While working: `acargo nextest run -E 'test(<page>) | binary(rules)'
  --status-level fail --final-status-level fail --hide-progress-bar` and
  `acargo clippy -q --all-targets -- -D warnings`. The rules tests run every
  time: they are where the architecture and the review checklist live.
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
```

### C5. process.md §A4.3 (replace)

```markdown
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
```

### C6. process.md §A5 (replace the whole section)

```markdown
## A5. Review

| tier | what | review | landing |
|---|---|---|---|
| TRIVIAL | markers, docs, packs, gap rows, renames | none: the orchestrator checks the diff scope (no `src/`, `core/`, `host/`, manifest change) | fast-forward or the next candidate |
| LIGHT (default for pages, sheets, dialogs, StartupPage, settings, pickers) | one page's views, core state and host/services | one Sonnet `codex-reviewer` pass over the page's diff, after it lands | does not wait |
| FULL | the risky list: session-core, chat-cells' streaming tail, composer, transcript, approvals, pager, inline mode, perf claims | `codex-reviewer` with `model: opus`, before landing | waits for the verdict |
| wave | the wave's diff on the integration branch | Opus: drift from the architecture or rtwk's standard means; the R-rule audit | the next wave |

A review returns FIX only for:
1. a defect, with its failing input;
2. an ADR violation without a debt task: ADR-003 state, a layer crossed
   (where no rule catches the crossing, the missing rule is a finding too);
3. a user-visible difference from the oracle with no open gap row;
4. work outside the area, or codex-tui / the codex-rs workspace touched.

Everything else is a note: structure no rule catches yet, pack or gap-row
wording, style. A note goes to the page's next task or to the wave's
follow-up. A structural finding a rules test could catch also goes to the
wave's rules task. Gap rows are one line, and a reviewer never asks for
probes or routes.

On FIX the orchestrator rejects in tm with the findings. The same task is
re-granted in the same worktree, and the porter answers each finding by
`path:line` and commit. A re-review reads only the fix commits. A second
FIX on the same page is a stop (§A9).
```

### C7. process.md §A6 (replace steps 1–4, keep the ADR-check box above them)

```markdown
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
```

### C8. process.md §A7 (replace the table and the paragraph above it)

```markdown
## A7. Pages in parallel

Limits: **at most 3 concurrent cargo builders** on this machine (a
land-check counts as one; load beyond that made every build and timing test
slower, 2026-11-21: load 139 on 10 cores), the orchestrator's landing
throughput, the person's acceptance.

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
```

### C9. process.md §A10, first bullet (replace "**Models.** …")

```markdown
- **Models.** Porters on Sonnet by default; the orchestrator may pass Opus
  for a large page (no hook refuses it since 2026-11-18). Opus reviewers only
  for FULL and wave reviews. Haiku waiters for every wait over ~4 min.
```

### C10. .tm/fleet-brief.md — replace "## A slice is done when", "## Checks: …", the first bullet of "## Where your work lives", and "## Handover"

```markdown
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
- While iterating: `acargo nextest run --status-level fail
  --final-status-level fail --hide-progress-bar -E 'test(<page>) | binary(rules)'`,
  `acargo clippy -q --all-targets -- -D warnings`.
- Once, before `task_done`: rebase onto the integration tip (the orchestrator
  names it in tm notes), then `acargo fmt -- --check`, clippy `--workspace`,
  the whole nextest, your oracle shard(s) with `--profile oracle`,
  `shards.py --check`, your pty scenario on both binaries. Long runs through
  `~/.cache/rtwk/bgrun.sh`.
- Known reds: `~/.cache/codexcli/known-red.txt`. Do not re-verify them. A
  red not on the list is yours, unless one run on the tip without your
  change shows it red: name it in the handover.

## Where your work lives
- Stay inside your task's area. The brief names each shared file's owner;
  a shared file owned by another task is read only, and what you need from
  it goes into your handover. Never edit `lib.rs`, a parent `mod.rs` or a
  root event/intent enum outside your area.

## Handover
`task_done` with deliverables: the commits, the parity pack path, and a
summary of at most 15 lines: shard counts, what is left and why, gap rows
added, departures with their debt task, new reds (not known ones), the
binary run's result, what others must change. A fix run answers each review
finding by `path:line` and commit. Your brief names the review tier
(process.md §A5): stay inside what it allows.
```

### C11. .claude/agents/codex-porter.md — replace "## Cargo", "## Quality bar", "## Handover"; add "## Run it" after "## Quality bar"

```markdown
## Cargo
- `~/.cache/rtwk/acargo <args>` from `codex-rs/tui-rtwk`, in your worktree's
  own target (never set CARGO_TARGET_DIR).
- While iterating: `acargo nextest run --status-level fail
  --final-status-level fail --hide-progress-bar -E 'test(<page>) | binary(rules)'`,
  `acargo clippy -q --all-targets -- -D warnings`.
- Once at the end, after rebasing onto the tip: fmt, clippy `--workspace`,
  the whole nextest, the oracle shard(s) under `--profile oracle`,
  `shards.py --check`.
- No call longer than ~4 min: `~/.cache/rtwk/bgrun.sh start <name> -- <cmd>`
  then `bgrun.sh wait <name>`.
- Known reds: `~/.cache/codexcli/known-red.txt`. Do not prove them again.

## Quality bar
- The oracle's tests are the spec. Turn the page's oracle shard on; its
  expected values are the oracle's. Tests the shard lacks (state tests,
  mouse tests) come before the code they cover and are shown biting once.
- No visual delta (the person, 2026-11-18). Compose rtwk until rows and
  styles equal the oracle's. What rtwk truly cannot do is a one-line gap
  row + `pending(Gxx)`, never a workaround or a loosened test.
- Mouse on every selectable view (ADR-011), with rtwk_test mouse tests.
- Report what the checks actually said. Claim nothing you did not run.

## Run it
Before `task_done`, drive the page in the real binary:
1. `acargo build -q -p codex-rtwk --bin codex-rtwk`;
2. `cd tools/rtwk-port/mock-model && RTWK_BIN=<worktree>/codex-rs/tui-rtwk/target/debug/codex-rtwk
   python3 -I check.py --bin rtwk --bin oracle --only <your scenario>
   --capture-dir /tmp/<alias>-cap` (`CODEX_HOME=$(mktemp -d)` for first-run
   pages);
3. walk the main path with keys and the mouse;
4. put the differing rows of one checkpoint (or "equal") in the pack.
A page that does not work in the binary is not done, whatever the tests say.

## Handover
- Nothing running in the background, no subagents of your own.
- `task_done` with the commits, the pack path and a summary of at most 15
  lines (fleet brief "Handover").
- A fix run (your task re-granted with review findings): fix in the same
  worktree, answer each finding by `path:line` and commit.
- Your final reply: at most 15 lines.
```

### C12. .claude/agents/codex-reviewer.md — replace the "## A review" section up to "## A research question"

```markdown
## A review
- Read the task (`get_task`: the brief, the deliverables) and the page's
  commits (`git show --stat`, then the diff per path). Judge the page against
  the brief and codex-tui's behaviour, not against the porter's summary.
- Run the page's tests and its oracle shard. Do not rerun the whole suite:
  the porter and the land-check ran it. Break the code once only when a
  bite is in doubt. Ignore reds listed in `~/.cache/codexcli/known-red.txt`.
- Look at behaviour first: what the page does on a double key press, a
  dropped connection, a late reply, a narrow terminal, a click and a wheel;
  what the porter's binary capture shows beside the oracle's.
- ADR conformance, per ADR the page touches: kept / departure with its debt
  task / violation (FIX). ADR-003 state ownership, the layers, ADR-002's four
  conditions, R14 ids into routes and dialogs.
- Verdict, first line: `MERGE`, `MERGE-WITH-NOTES` or `FIX`. FIX only for
  (process.md §A5):
  1. a defect, with its failing input;
  2. an ADR violation without a debt task, or a layer crossed (and, where no
     rule catches it, the missing rule as a finding);
  3. a user-visible difference from the oracle with no open gap row;
  4. work outside the area, or codex-tui / codex-rs workspace touched.
- Everything else is a note: structure a rules test could catch (name the
  test to add), pack or gap-row wording, naming. Never ask for probes,
  routes or file:line in a gap row; a gap row is one line.
- A re-review after a fix run reads only the fix commits and the open
  findings.
- A wave review (Opus): drift from the architecture and rtwk's standard
  means over the wave's diff; the R-rule audit (each rule → its lint, test
  or type → present / not yet; each "caught" sub-class in
  `research/state-bug-classes.md` → its test). Write it where the prompt says.
- Findings ranked defect / risk / note, each with `path:line` and commit.
- You write nothing in the repository and never accept or reject in tm.
```

## What the person decides

1. **The unit of work** (L1) and **continuous landing** (L7): both replace
   the batch rounds of §A3 and §A6.
2. **Three concurrent builders** (L5): fewer porters at once, measured
   against load 139.
3. **Dropping the red-first commit** where an oracle shard exists (L9),
   keeping tests first for non-shard tests.
4. **Process changes at page boundaries** (L8): a decision of the person
   reaches running porters only through a tm note.
5. **known-red.txt** as the baseline for porters, reviewers and land-check
   (L5), each entry with a tm bug.
