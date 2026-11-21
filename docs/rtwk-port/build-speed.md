# Build speed: the land-check and the tui-rtwk build/test cycle

Task `build-speed`, 2026-11-21. What was redundant or slow (found by reading the code, then
confirmed on the logs), what was changed, what it saved, what is left. Numbers are wall seconds
on a machine that was running 20 to 35 load (10 cores, other builders): treat them as an order
of magnitude, not as benchmarks.

## 1. Before

Baseline: the last full land-check before this task, `~/.cache/rtwk/bg/land-landcheck.log`
(16:44:18 to 17:11:50, a fresh `land` target). The log has no per-step stamps; the steps that
print their own numbers:

| step | before |
|---|---|
| whole land-check | **1652 s (27.5 min)** |
| nextest, build of the test binaries | 437 s (`Finished test profile in 7m 17s`, cold target) |
| nextest, run | 50.6 s (1089 tests, 50 test binaries) |
| scenarios (38 pty scenarios, one after another) | ~590 s (router 151 s, two known reds 66 s and 65 s) |
| rest: fmt, clippy, manifests, shards, build-bin (incl. one wait for a build slot) | ~574 s |

What the code showed:

1. **codex-core and 61 more crates were compiled twice** per land-check. Cargo's resolver 2
   unifies features per build, and the dev-dependency `insta` turns `similar/inline` on only in
   the graphs that include dev-dependencies. So the test graph (`nextest`, `clippy
   --all-targets`) and the `cargo build -p codex-rtwk --bin codex-rtwk` graph resolved
   `similar` differently, and everything above it (the codex-core family, 62 units by
   `cargo build --unit-graph`) had a second, separate set of rlibs.
2. **50 test binaries**: every `[[test]]` and every file of `core/tests` is a rustc run plus a
   link of the whole dependency set (a few seconds each), relinked after every library edit.
   25 in `core`, 21 in `tui-rtwk`, 4 elsewhere.
3. **The pty scenarios ran serially**: 38 scenarios, each with its own mock server, temp
   `CODEX_HOME` and pty (nothing shared), 590 s one after another.
4. **Three slot waits per land-check**: clippy, nextest and build-bin each queued for one of
   the 3 build slots (`memcap`), so a busy machine made the same land-check wait three times
   (the old log shows one: `memcap: waiting for a build slot`, before build-bin).
5. The steps ran in the order fmt, clippy, nextest, ...: a manifest, oracle or shard failure
   showed only after ~15 minutes.

## 2. What changed

| # | change | where | effect |
|---|---|---|---|
| 1 | `similar = { version = "2", features = ["inline"] }` in `[dependencies]` of tui-rtwk (a cargo-hakari pin by hand, comment in the file) | `codex-rs/tui-rtwk/Cargo.toml`, `Cargo.lock` (+1 line) | the test graph and the bin graph resolve the same `similar`: one codex-core family, the 62 units are built once; the bin build after nextest reuses the test graph's rlibs |
| 2 | test targets 50 to 12: `core` 25 to 1 (`core/tests/all.rs`), `tui-rtwk` 21 to 7 (`screens`, `pages`, `shell`, `rules`, `scenarios`, `kit_proof`, `oracle`); `autotests = false` | `core/Cargo.toml`, `core/tests/all.rs`, `Cargo.toml`, `tests/{pages,shell}.rs`, `tests/rules/main.rs` | 38 fewer link steps in a build, 12 binaries to relink after a library edit. No test body changed; the four mouse/draft page files only changed their `mod common;`/`mod kit;` lines to `use crate::...` |
| 3 | a registration guard: `tests/rules/registration.rs` | `tests/rules/` | a test file that no root lists is never built and never fails; the guard fails for it, and for a root with no `[[test]]` |
| 4 | `check.py --jobs N` (default `CHECK_JOBS`, else 1): the pty scenarios run N at a time in forked workers, the slow ones (router, error-context-window, multi-agent) first | `tools/rtwk-port/mock-model/check.py` | scenarios 590 s to **165 s** at `--jobs 4` (36/38 passed, the two known reds; bound by the router's 151 s). The FAIL line is unchanged |
| 5 | `land-check.sh` (outside the repo, backup `~/.cache/codexcli/land-check.sh.bak-build-speed`): per-step seconds and a `== TIMINGS` line (also appended to `~/.cache/codexcli/land-check-times.log`); scenarios with `--jobs $LAND_SCENARIO_JOBS` (default 4); one build slot for the whole compile phase (clippy, nextest, build-bin; released before the scenarios); the cheap steps (manifests, oracle-untouched, shards) first | `~/.cache/codexcli/land-check.sh` | one slot wait instead of three; a cheap red shows in seconds |

`acargo` and `sccache-env.sh` are unchanged.

### How a new test file registers now

A new file is one line in a root, otherwise it is not built (`registration.rs` says so):

| the file | the line |
|---|---|
| `core/tests/<x>.rs` | `mod <x>;` in `core/tests/all.rs` |
| `tests/rules/<x>.rs` | `mod <x>;` in `tests/rules/main.rs` |
| `tests/services/shell_<x>.rs` | `#[path = "services/shell_<x>.rs"] mod shell_<x>;` in `tests/shell.rs` |
| `tests/pages/<x>.rs`, `tests/sheets/<x>.rs`, `tests/app/<x>.rs` | the `#[path]` module line in `tests/screens.rs` (as before) |
| a probe file at the top of `tests/` (`rtwk_g<N>.rs`) | `mod rtwk_g<N>;` in `tests/pages.rs` |
| `tests/oracle/<x>.rs` | `tests/oracle.rs` (as before) |

A page file inside the merged binaries reaches the harness as `use crate::common::...` and
`use crate::kit::...`. A file that declares its own `mod common;` is loaded twice in the one
crate and clippy's `duplicate_mod` rejects it.

A porter's `[[test]]` entry of its own (composer-2 has a `composer_burst` one) is a rebase
conflict in `Cargo.toml`: register the new file as a `mod` line in `tests/pages.rs` instead.
A porter's new `core/tests/*.rs` (chat-cells-2) is a `mod` line in `core/tests/all.rs`.

### The porter's cycle

The fleet brief's command is still right, `binary(rules)` is the rules binary:

    acargo nextest run --status-level fail --final-status-level fail --hide-progress-bar \
        -E 'test(<page>) | binary(rules)'

It builds all 7 test targets of the root package. When only one target matters, name it:
`--test screens`, `--test pages`, `--test rules`: only that binary is compiled and linked.
Edit one file, then nextest (load 25 to 35 during the measurement; the "test run" column is
nextest's own Summary time, the rest is compile and link):

| cycle | wall | of which test run |
|---|---|---|
| edit `src/widgets/age.rs`, all 7 targets (`test(age) \| binary(rules)`) | 79 s | 34.5 s (215 tests; `test(age)` also matches `agents`, whose width test takes 27 s) |
| same edit, `--test screens` only | 46 s | 30.2 s (63 tests) |
| same edit, `--test rules` only | 15 s | 1.3 s |
| edit `core/src/lib.rs`, all 7 targets | 64 s | 33.1 s |

Before this task the same edit relinked 21 test binaries of tui-rtwk (and 25 of core when the
edit was in core). No new before-measurement was spent on the cycle (the old log only shows
`Finished test profile in 30.79 s` for an unrelated warm rebuild).

## 3. After

One full land-check on this branch (rebased on the tip 2de29bbc6), in a fresh worktree with an
empty target (the same cold-target situation as the baseline), with the new `land-check.sh`:
`land-check.sh <worktree> 2de29bbc6`, 2026-11-21 18:46 to 18:59, load 18 to 6 while it ran
(the baseline's load was not recorded; the machine was busier for most of that day).

| step | before | after |
|---|---|---|
| **whole land-check** | **1652 s (27.5 min)** | **808 s (13.5 min)**, -51% |
| manifests, oracle-untouched, shards | not timed (seconds) | 1 s, 0 s, 0 s, first in the run |
| fmt | not timed | 2 s (red, see below) |
| clippy `--workspace --all-targets` | not timed (inside the ~574 s rest) | 280 s |
| nextest, build of the 12 test binaries | 437 s (50 binaries) | 328 s (`Finished test profile in 5m 28s`) |
| nextest, run | 50.6 s (1089 tests) | 44.0 s (1137 tests, 0 failed, 12 binaries) |
| build-bin (`codex-rtwk`) | a long compile (one more codex-core family) plus a slot wait | **2 s** (the rlibs of the test graph are reused) |
| scenarios (38) | ~590 s | **147 s** (`--jobs 4`; 36/38, the two known reds) |
| build-slot waits | 3 (one printed in the old log) | 1 for the whole compile phase |

The run ended `fail=1` for one reason that is not this task's: `fmt` is red on the tip itself,
two files of the startup-wizards landing are not rustfmt-clean
(`host/src/launch.rs:41`, `src/pages/startup/prompts/panel.rs:26`). Every other step was green
(scenarios: only the known reds `multi-agent` and `error-context-window`). The step table of
every future run is appended to `~/.cache/codexcli/land-check-times.log`.

What the cold-target saving is made of: build-bin 2 s instead of a second compile of the
codex-core family (change 1), the scenarios 4 at a time (change 4), 38 fewer links in the nextest
build (change 2), one slot wait (change 5). What is left of the 808 s is the two cold compiles
that cannot share artifacts (clippy checks, nextest builds: 280 s + 328 s) and the router.

## 4. Measured or checked, and left alone

- **The oracle binary in the nextest build**: nextest builds it though the default profile
  excludes it (`binary(oracle)`; the land-check never runs it, only compiles it). Its compile
  is a 7 s unit, not worth a command-line split.
- **Linker / debuginfo / incremental / opt-level**: `debug = false` is already set for the
  workspace and all packages, `incremental = false` is set on purpose (sccache cannot cache
  it, targets reached 40 GB), and no faster linker was available on this machine. Nothing to
  cut there.
- **Fixed settle timers in `check.py`** (`quiet_for`, `pump`): kept; they are what makes the
  scenarios stable. Only the scenarios' parallelism changed.

## 5. What is left

- **The sccache cache is full** (10 GiB cap): a cold build still compiles `aws-lc-sys`'s build
  script (44 s) and the codex-core family; raising the cap (`SCCACHE_CACHE_SIZE`) or pruning old
  worktree targets would help every cold build. It is machine configuration, not repo code.
- **The router scenario (151 s)** is now the floor of the scenario step: its settle waits would
  have to shrink, or it split into two scenarios, to go lower than ~150 s.
- **Two known-red scenarios (`multi-agent`, `error-context-window`)** wait ~66 s each for a
  mock request that never comes; they run first in the pool, so they cost no extra wall time,
  but they hold two of the four workers for a minute. Fixing them (or lowering their timeout)
  frees the pool.
- **`agents::the_center_draws_at_widths_one_to_ten` (27 s)** is the longest test in the whole
  run and sets the floor of `screens`; one test per width would let nextest run it in parallel.
  It is in another area, so it is only noted here.
- **Not tried**: `opt-level = 1` for the rtwk dependencies (faster tests, slower cold build),
  more nextest threads (the memcap cap is 3 for memory reasons).
