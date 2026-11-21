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

(Round two, §6, did the cache, the router, the two known reds and the width test below; the
cap is its proposal 1.)

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

## 6. build-speed-2 (2026-11-21, the second round)

The person: "everything must be much faster". Order kept: read the code, change it, then
measure (no new before-measurement was spent; the baseline is §3 and
`~/.cache/codexcli/land-check-times.log`). Load is named with every number: three other
builders ran all day (load 4 to 44 on 10 cores); an unloaded machine is faster than every number
here, and the same run differs by 2x between a load of 5 and a load of 20.

### What was slow, found in the code and then measured

1. **Two compiles of the whole graph**: `clippy` (check mode) and `nextest` (build mode) share
   no artifacts, 280 s + 328 s cold.
2. **sccache did not hit across worktrees for ~340 of ~860 crates**, so every new worktree (every
   porter, every land-check) compiled the codex-core family again (codex-core alone 84 to 106 s,
   a chain of ~30 backend crates behind it). sccache hashes the CONTENTS of each dependency's
   artifact into the key of the crate above it, and two checkouts differed in two kinds of bytes:
   - a crate whose build script generates code that is `include!`d (serde_core, thiserror, ...)
     carries the absolute `OUT_DIR` path (inside the checkout's target) in its rlib;
   - a proc-macro dylib on macOS carries its absolute OUTPUT path in `LC_ID_DYLIB` (and so in the
     `LC_UUID` and the code signature). `ts_rs` (through `ts-rs-macros`) is under almost every
     `codex-*` crate: it differed, and everything above it with it (found by diffing the rlibs of two
     checkouts: 98 of 859 differed, the root being `ts_rs`).
3. **`CARGO_TARGET_DIR` exported by land-check.sh**: sccache hashes every `CARGO_*` variable it
   sees; this one holds the worktree's path. With it set NOTHING hits across worktrees (measured:
   417 s for the lint-build at load 16 to 20; the same tree without it 128 s at load 14 to 19).
4. Small ones: the router scenario (151 s) as the floor of the scenario step, the two known reds
   waiting a full minute each, `agents::the_center_draws_at_widths_one_to_ten` (27 s) as the floor
   of the `screens` run, the scenarios started only after nextest.

### What changed

In the repo:
- `tools/rtwk-port/mock-model/check.py`, `router.py`, `scenario.py`, `selftest.py`: a scenario can
  name `cuts` (the router: before `proposal`, `code`, `compact`); with `--jobs N` > 1 it runs as
  pieces `router#1..#4` (8, 9, 9 and 3 turns), each in a worker of its own, the last one checks
  the sentinel. `--short-budget NAME[,NAME]` gives the named scenarios 20 s of slack per turn
  instead of 60 s (land-check passes the `scenarios:` entries of known-red.txt: a known red only
  waits its budget out, a green one never does). `MOCK_TURN_TIMES=1` prints per-turn seconds.
  `selftest.py` covers both (every turn runs exactly once; the short budget is shorter).
- `codex-rs/tui-rtwk/tests/pages/agents.rs`: the width test is ten tests (`macro_rules!`,
  `the_center_draws_at_width_1` .. `_10`, each 2.4 to 2.6 s), so nextest runs them in parallel.
- `.tm/fleet-brief.md`: the porter's cycle command (below).

Outside the repo (each with a `.bak-build-speed-2` beside it, except the new file):
- `~/.cache/rtwk/remap-bin/rustc` (NEW): `RUSTC` for CodexCLI checkouts: the real rustc plus
  `--remap-path-prefix=<checkout>=/w` and, for proc-macro crates on macOS,
  `-C link-arg=-Wl,-install_name,@rpath/lib<crate><extra-filename>.dylib`. sccache stays
  `RUSTC_WRAPPER` in front of it, so its key sees cargo's arguments, not these. Verified: the
  `ts_rs_macros` dylib has the same md5 in two checkouts, and the rlibs of all 859 crates are
  byte-identical between two checkouts except the two workspace crates compiled by clippy-driver.
- `~/.cache/rtwk/acargo`: for CodexCLI checkouts, the stable toolchain and sccache on, the
  heavy subcommands (build, check, clippy, test, nextest, bench, run, doc) set `RUSTC` to the script
  above (`RTWK_REMAP_TOP` = the checkout; `ACARGO_NO_REMAP=1` switches it off; a `RUSTC` of the caller
  is kept). It also drops a `CARGO_TARGET_DIR` that equals the checkout's default target (item 3).
  A target built before this change is not rebuilt by it; only crates compiled from now on carry
  the remap.
- `~/.cache/codexcli/land-check.sh` (the header of the file lists it): `lint-build` = ONE pass,
  `nextest run --workspace --all-targets --no-run` with clippy-driver as `RUSTC_WORKSPACE_WRAPPER` and
  `CLIPPY_ARGS=-D warnings` (what `cargo clippy -- -D warnings` sets; plants of an unused variable
  and a `needless_return` were caught by it in 6 s; the same lints on lib, bins, tests and examples
  as the old `clippy --workspace --all-targets`) replaces the separate clippy step (`LAND_FUSED=0`
  brings the old two steps back); the scenarios start as soon as the binary exists and run beside
  nextest; `LAND_SCENARIO_JOBS` default 8 (was 4); short slack for the known-red scenarios; a
  scenario that fails and is not a known red gets ONE rerun on its own (a pty startup stalls now
  and then on a loaded machine: two `session_sse` tests and three scenarios at 8 test threads +
  12 scenario jobs; they pass on their own); `CARGO_TARGET_DIR` is no longer exported;
  `LAND_TEST_THREADS` / `LAND_BUILD_JOBS` take acargo's 3 and 3 unless given (8 test threads beside
  the scenarios: 178 s and two flaky tests; 3 threads: 66 s and none).
- Not changed: the sccache cap (see the proposals).

### Numbers

Lint + build of a new worktree (`nextest run --workspace --all-targets --no-run`, one cold checkout
after another, the same sccache; before = no remap):

| run | wall | load |
|---|---|---|
| before: separate clippy + nextest build (the §3 land-check) | 280 + 328 = 608 s | 6 to 18 |
| fused lint-build, no remap, 1st checkout, cold | 475 s | 17 to 44 |
| fused lint-build, no remap, 2nd checkout (what sccache gave a new worktree) | 353 s | 11 to 19 |
| fused + remap, populating the cache | 200 to 305 s | 7 to 15 |
| **fused + remap, a new worktree** | **59 s** | 4 to 9 |
| same, four other runs of a new worktree (jobs 3 or 6) | 69 s, 70 s, 119 s, 128 s | 4 to 19 |
| with `CARGO_TARGET_DIR` exported (what land-check did) | 417 s | 16 to 20 |

`codex-core` 84 to 106 s compiled drops to 2 s (a hit); what is left on the critical path is
`aws-lc-sys`'s build script (31 to 44 s even with every C file a hit) and the workspace crates
(tui-core, tui-rtwk, the 7 test binaries and the bin, ~50 s at 3 jobs; 6 jobs did not help: 69 s).
sccache counters of the server: Rust hit rate 60.1 % after the first remap run (still the old
dylibs), 80.3 % after the fix. One run of a new worktree (19:59, load 9 to 12) missed ~100 crates
for a reason that was not found (255 s; the next two runs of new worktrees hit: 69 s and 70 s); the
cache is full, see the proposals.

Tests and scenarios (load 7 to 16, 1149 tests):

| | before | after |
|---|---|---|
| nextest run, 3 threads | 45 s (the 27 s width test is the floor) | the width test is ten of 2.5 s |
| nextest run, 8 threads | not measured | 19.6 s (load 8), 36 s (load 13, before the split) |
| scenarios alone | 147 s (jobs 4, the router 151 s as one) | 80 s (jobs 8, load 14), 48 s (jobs 16, load up to 28) |
| the two known reds | ~66 s each, holding 2 workers | ~30 s each |
| nextest 3 threads + scenarios 12 jobs together | n/a | 74 s in all (66 s + 63 s overlapped, load ~20) |

Edit to test result in one worktree (load 9 to 21; "run" = nextest's own time):

| cycle | wall | of which run |
|---|---|---|
| edit in `src/widgets/age.rs`, the fleet's `-E 'test(age) \| binary(rules)'` over all 7 roots | 111 s (load 21) | 16.9 s |
| same edit, `--test rules` | 9.4 s | 1.0 s |
| same edit, `--test screens -E 'test(age)'` | 21.9 s | 12.2 s |
| edit in `tests/pages/agents.rs`, `--test screens` (10 width tests) | 15.0 s | 11.3 s (3 threads) |
| same edit, all 7 roots | 25.5 s | 12.3 s |
| `acargo check --tests --quiet` after a test-file edit, warm | 2.5 to 6.8 s | |

The porter's fastest command is therefore: `acargo check --tests --quiet` for type errors, then
`acargo nextest run --test <root> ... -E 'test(<name>)'` (the root where the file is registered:
`screens` for tests/pages, sheets, app; `shell`; `rules`; `pages` for probe files; `oracle`; core:
`-p codex-tui-core`): only that binary is built and linked, 10 to 15 s from edit to result when
the filtered tests are short. `.tm/fleet-brief.md` says so.

Land-check of this branch (fresh worktree, empty target, `land-check.sh <worktree> 960e4f2b4`,
2026-11-21 20:55 to 20:59, load 23 to 17 from the log line; code tip `0a85e8d4d`, the doc commit
after it changes no code):

| step | build-speed (§3) | build-speed-2 |
|---|---|---|
| **whole land-check** | **808 s** | **189 s (3.2 min), -77 %** (`fail=0`: fmt is green on this tip) |
| manifests, oracle-untouched, shards, fmt | 1 s, 0 s, 0 s, 2 s | 0 s, 0 s, 1 s, 1 s |
| clippy `--workspace --all-targets` | 280 s | gone (in lint-build) |
| compile: lint + build of all targets | 328 s (nextest build) | **105 s** (clippy lints and test build in one pass, warm cache) |
| build-bin | 2 s | 1 s |
| nextest run (1210 tests, 0 flaky, 3 threads, beside the scenarios) | 44 s | 51 s (48.3 s run) |
| scenarios (39/41, the two known reds) | 147 s, after nextest | 80 s, beside nextest |

The first land-check of this branch ran with 8 test threads and 12 scenario jobs: 301 s, nextest
178 s, three flaky tests (two `session_sse`, one known): the settings in §6 "What changed" are
the ones that came out of that run. The 105 s are the compile: `aws-lc-sys` (31 to 44 s) and the
workspace chain; a first land-check after a repin or a new dependency compiles what is not in the
cache (200 to 300 s, see the table above).

### Proposals (they need the person's yes)

1. **sccache cap 10 -> 16 GiB** (`~/.cache/rtwk/sccache-main/config`, `size`; `sccache-config.sh`
   writes 20 by default). The cache is full: 10229 entries, all written in the last ~2.5 hours, i.e.
   three builders turn the whole cache over in about 2.5 hours, and one graph is 3.8 GiB raw. With
   the fix a populate is worth keeping for the whole working day. The data volume had 95 GiB free
   after this task's scratch targets were deleted (it was down to 117 MiB while they existed: keep
   scratch trees to two). Not applied: it is the person's disk.
2. **Profile and linker**: not tried, `codex-rs/tui-rtwk/Cargo.toml` is held by rtwk-repin-4.
   `incremental` per package (the workspace crates only, so sccache is not asked to cache them) may
   shorten the edit cycle further. `rust-objcopy` aborts on every link (a SIGABRT with a missing
   `libLLVM.dylib`, while stripping debuginfo: the toolchain's component is broken, the build goes
   on); `strip = "none"` in the dev profile would skip the failing subprocess.
3. **A fast land path** is not needed any more: see the land-check numbers above (808 s before).
   The scenarios already run beside nextest, so skipping them would not shorten the critical path.

### Found on the way, not fixed

- Pty startups stall now and then under load: a scenario or a `session_sse` test sees the splash
  for 60 s and serves 0 of N requests; they pass on their own. Not the harness's fixed wait
  (`quiet_for(1.5, 30)`): the process is stuck, not late. Outside this task's area. The land-check
  reruns a failed scenario once and nextest has `--retries 1`.
- `rust-objcopy` SIGABRT warning on every link (proposal 2).

### Applied 2026-11-22 (the person's yes)

- Proposal 1: sccache cap 16 GiB (`~/.cache/rtwk/sccache-main/config` `size`, `sccache-config.sh` default 16); takes effect at the next start of the sccache server.
- Proposal 2: `[profile.dev] strip = "none"`; `incremental = true` for `codex-tui-rtwk`, `codex-tui-core`, `codex-rtwk` only. `acargo check --tests` and the rules root green.
