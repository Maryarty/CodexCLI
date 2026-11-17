# Performance comparison: Codex TUI on rtwk vs original (ratatui + crossterm)
Status: research, read-only on both repos. Date 2026-11-17.
Test-infra study (fixtures, scripted backends): `$TMPDIR/codexcli-research/test-infra.md`
did not exist when this was written; this file only points at the mock-model
pieces it needs (section 2) and does not cover fixtures.

## Summary
The harness in 10 lines:
1. One Python driver (`perf/drive.py`, stdlib only) forks a binary in a real pty
   at a fixed size (120x40, plus 200x60 and 80x24), answers DA1/CPR/DSR as a
   terminal would, and feeds the pty output to a terminal model (vt100) as it
   goes. It is the same method as rtwk's `tools/responsiveness/pty_latency.py`.
2. Both binaries (original `codex`, port `codex` with `--frontend rtwk`) talk to
   the SAME local fake Responses-API server (`perf/fake_model.py`, SSE with
   scripted deltas at a fixed rate), with a fresh `CODEX_HOME` and `analytics.enabled=false`.
3. A scenario file (JSON: size, prompt, SSE script, key/resize/wheel script with
   timestamps) is replayed identically against A and B.
4. Per run the driver records: keystroke-to-visible (emulator shows the expected
   cell, not first byte), every output chunk (t, bytes), frames (a frame = one
   DEC 2026 synchronized-update pair, or a read burst when absent), CPU and
   peak RSS (`getrusage` of the child, `ps` samples), time-to-settle.
5. Runs are release builds, interleaved A B A B ... N=15 (20 for latency), load
   (`uptime`) printed before and after; exit 75 and no verdict if load > 1.5x CPUs
   (rtwk's gate rule).
6. Raw results go to `perf/results/<date>-<commit>-<machine>/runs.jsonl`
   (one line per run, all samples kept); nothing is summarised on the way in.
7. `perf/report.py` (stdlib + one vendored JS chart file, no pip) writes ONE
   self-contained `report.html`: per scenario distributions (strip plot + box with
   p50/p95/p99 marked), A vs B, a ratio table with a bootstrap 95% interval.
8. Side-by-side recordings: the driver also writes asciicast v2 per run; the
   report embeds them in a two-pane replayer (asciinema-player JS, vendored).
9. `perf/trend.py` appends each milestone's summary to `perf/history.jsonl` and
   draws the milestone trend (original baseline = the fixed reference line).
10. Rules inherited from rtwk: release builds, load stated, interleaved runs,
    p50/p95/p99, no speed claim from a microbenchmark alone (micro numbers go in
    an appendix section of the report, labelled as such).

First baseline to take now (no port code needed): build the original release
binary (`cargo build --release -p codex-tui`/`codex-cli` once the cold build
finishes; same binary reused for all later comparisons), write the fake model
server and driver, run scenarios S1 (stream 100/s), S2 (stream 1000/s), S5
(typing while streaming), S9 (startup), S10 (idle) with N=15 at 120x40, and
commit the JSONL plus the rendered HTML as `baseline-original`. The original needs
no modification for this baseline: all of it is black-box through the pty.

## 1. rtwk's existing performance method (what we reuse)
- `docs/PERFORMANCE.md`: targets table (unchanged frame writes 0 bytes, 0 allocs
  at idle); foundation bench against ratatui and the Dart core on identical
  workloads, with hash-verified identical final screens before any timing
  (`rtwk-bench --verify`); p50/p95/p99 from frame-timing batches; 3 runs per
  binary, order alternated; relative noise band (max deviation over runs, 1%
  floor); bytes and allocator calls compared as exact integers (the exact-count
  rule is what makes "bytes per frame" a strong metric, since it does not depend
  on load); machine-specific `bench/baselines/<machine>/baseline.json` with
  commit, compiler, load before/after; regression gate `cargo xtask bench-check`.
  Reuse the idea, not the code: the foundation bench is in-process on
  `Surface`, and cannot see Codex.
- `docs/design/responsiveness.md`: keystroke latency table (120x40 pty, `dist`
  build, uptime stated; key spaced 150 ms, autorepeat 33 ms, wheel, paste, idle,
  startup; "first byte is not seen"). The later gate measures event to VISIBLE
  through an emulator: timestamp just before the pty write, endpoint when the
  expected row/highlight/viewport appears in the emulator; DEC 2026 state held
  until reset or 150 ms. Budgets: input during streaming p50 16 / p99 32 ms,
  other input 6 / 30 ms, startup 35 / 75 ms, idle <= 2% CPU and 0 bytes.
  Scroll audit has the Codex stream model (3 rows a tick, 16 ms) as a row in its
  matrix (`--scroll-model=streams`); its metrics (mean rows moved per display
  refresh at 60/120 Hz, cv, stalls, time to settle, reversals) are reusable for
  S6 (scrolling).
- Tools: `tools/responsiveness/pty_latency.py` (pty driver, the template for
  `drive.py`), `scroll_replay.py` (replays recorded input at recorded times, both
  engines trace themselves), `scroll_trace` example (strict terminal model,
  `tests/support/vt.rs`), `timeline.py` (inside-frame stages), `cargo xtask gate`
  (`xtask/src/gate.rs`: fresh child at 80x24, preflight launch, three measured
  rounds with rotated app order, emulator answers probes and verifies state, exit
  75 on loaded host). `crates/rtwk/examples/agent_chat_latency.rs` is a
  Codex-shaped micro app (spinner, 30-row transcript, composer; gate case "key
  while streaming at 5 ms per token"). `docs/design/rust-native.md` §2.4: 1,000
  messages, a token every 5 ms, naive vs observable variants, counts (rebuilds,
  allocs, bytes, frames) do not depend on load, timings do.
- `bench/tools/ab.sh` (two builds in turn) is the A/B template; `profile.sh` (samply) for where time goes.
- Not installed: `asciinema`, `agg`, `vhs`, `gnuplot`, `hyperfine`, matplotlib; present: python3, `ffmpeg`.
- `rtwk record`/`play` (`crates/rtwk-dev/src/record.rs`, `play.rs`): asciicast of
  a command in a pty, `--keys` option. It records the port; for the original use
  our own driver's cast writer (below) so both are produced by the same code.
## 2. The original Codex TUI: what exists to measure it
- No TUI benches. `just bench` = `cargo bench --workspace --bench '*'`; `divan`
  is in the workspace but used only by `utils/image`. `just bench-e2e` runs
  Bazel target `//codex-rs:e2e-benchmarks`, which holds only
  `//codex-rs/cli:codex-help-bench` (process start of `codex --help`). Useful
  only as a cheap process-start reference, not for the TUI.
- Timing hooks: none for frames. `tui/src/tui/frame_rate_limiter.rs`
  (`MIN_FRAME_INTERVAL` 8.33 ms), `frame_requester.rs`, `tui.rs::draw` (line
  ~1449, inside a synchronized update) have no span or counter. Telemetry that
  exists (`terminal_size_telemetry.rs`, `statusline_metrics.rs`) is OTEL buckets
  of terminal size and statusline events, not latency. Log file: `codex-tui.log`
  under `CODEX_HOME/log`, RUST_LOG-controlled; only warn/info/debug around
  terminal probing.
- So black-box capture covers everything except frame CPU time. For frame time we
  need ONE minimal hook (see 3.3).
- Driving non-interactively with a scripted model (found, no need to invent):
  - `tui/tests/suite/resize_reflow.rs` already launches the real `codex` binary
    in tmux at 120x40 with `CODEX_HOME=<tempdir>`, `OPENAI_API_KEY=dummy`,
    `-c analytics.enabled=false`, `--no-alt-screen`, a config pointing a
    `[model_providers.X] base_url="<mock>/v1" wire_api="responses"
    requires_openai_auth=false` and `write_auth`; wiremock `MockServer` with
    `mount_resize_response`. `provider_defaults.rs` shows the same config shape.
    Our driver copies that config/auth recipe (read `write_config`, `write_auth`
    in that file) but uses a pty instead of tmux.
  - Mock model: `core/tests/common/responses.rs` (`sse`, `mount_sse_once`,
    `mount_sse_sequence`) is wiremock based: one body per response, no pacing.
    `core/tests/common/streaming_sse.rs` (`start_streaming_sse_server`,
    `StreamingSseChunk{gate, body}`) delivers per-chunk gated SSE: closer to what
    we want but needs a Rust test harness. `app-server-test-client/src/
    loopback_responses_server.rs` is a plain TcpListener, not paced.
    Decision: a ~100-line Python SSE server (`perf/fake_model.py`) emitting
    `response.created`, N x `response.output_text.delta` at a scripted rate and
    `response.completed`; the event shapes are copied from the `sse(...)` helpers
    so both TUIs accept them. Python keeps pacing exact and lets one server serve
    both binaries, and it needs no codex-rs build. If the test-infra study names a
    ready fake backend, prefer it and keep only the pacing.
  - The port runs the same core/app-server (ADR scope: only the frontend moves),
    so model, config and server code are identical on both sides; any difference
    measured is the frontend. State this in the report header.
  - Long transcript start state: `codex resume` over a generated rollout in the fixture `CODEX_HOME` (test-infra owns rollout fixtures); exec output and pickers: a scripted `shell` tool call printing 10k lines; `/resume` over a big fixture.
- Claims rule (PERFORMANCE.md): a microbenchmark alone never becomes a speed claim; the headline table is the pty scenarios only.

## 3. The comparison harness
### 3.1 Scenarios (Codex-shaped; each has a fixed script and a fixed final screen)
| id | scenario | what it stresses (original) |
|---|---|---|
| S1 | stream 100 deltas/s (~12 tokens each) into a 500-cell transcript, 20 s | whole-frame re-render per Draw, markdown re-parse per delta |
| S2 | stream 1,000 deltas/s, 10 s | adaptive chunking, 120 fps cap, event coalescing |
| S3 | markdown + fenced code blocks (rust, py, diff) streamed at 100/s | syntect highlight + markdown re-parse per delta |
| S4 | exec output, 10k lines (tool call) | history cell insertion, wrapping, scrollback cost |
| S5 | typing 10 keys/s (and 30 keys/s autorepeat) while S1 streams | keystroke-to-screen under load, the case users feel |
| S6 | picker: open `/resume` (or file search) over 5,000 entries, type a 6-char filter | list build, filter per key |
| S7 | scroll 1,000-cell owned transcript (wheel 8 ms and 150 ms trains, PgUp/PgDn) | owned alt-screen path; original in `--no-alt-screen` has terminal scrollback instead (see 3.4) |
| S8 | resize storm: 40 size changes in 2 s (TIOCSWINSZ + SIGWINCH) between 80x24 and 200x60, then rest | scrollback rebuild under row cap, debounce, time-to-settle |
| S9 | startup to first frame (fresh process; also with a 1,000-cell resumed session) | init, DA1 probe, history load |
| S10 | idle after a finished turn, 30 s (with and without status indicator) | frames, bytes, CPU at rest |
| S11 | paste 20 KB bracketed paste into composer | paste burst handling |

Sizes: 120x40 primary; S8 and S4 also at 200x60; S9/S10 at 80x24. Everything with a
seeded RNG for text; the SSE script and the key script are files in the scenario.
### 3.2 Metrics
| metric | how | from both binaries |
|---|---|---|
| keystroke to screen (p50/p95/p99, ms) | timestamp before pty write; endpoint when the vt model shows the typed char in the composer row (also "first byte" as a secondary column); DEC 2026 hold honoured as rtwk's gate does | black box, identical |
| frame time (ms) | a) black box: interval between frame ends (sync-update end markers) = frame period; b) inside: CPU time to draw = hook (3.3) | a identical; b original needs hook, rtwk has `FrameStats` (`last_loop_micros`, `last_write_micros`, `last_flush_micros`) |
| frames per second, bytes per second, bytes per frame | count sync-update pairs and chunk bytes in the driver; bytes per frame is load-independent: the headline exactness metric (as in rtwk) | black box |
| CPU % | child `ru_utime+ru_stime` over the window (wait4 rusage) plus 100 ms `ps` samples for the curve; report CPU-seconds per scenario as well as % | black box |
| RSS | peak (`ru_maxrss`, macOS bytes) and RSS at end from `ps`, sampled 1 Hz; growth over the run for S1/S10 (leak shape) | black box |
| allocations | not black box. Original: not measurable without rebuild; optional later with `DYLD_INSERT_LIBRARIES` malloc counter or a counting allocator feature. rtwk side has allocator counters in its bench. Mark "n/a" in report until an equal method exists | optional |
| time to settle after resize | from last TIOCSWINSZ to the last output byte AND vt-model screen equal to the final screen (hash); also number of full-screen repaints in the storm | black box |
| scroll smoothness (S7) | rtwk's `scroll_trace` metrics: rows moved per display refresh at 60/120 Hz, cv, stalls, settle | black box through the vt model |
| final-screen equality | text hash of final screen per scenario (cell+style hash when the oracle diff harness is available); runs where A and B differ are flagged in the report, not silently timed (rtwk's verify-before-timing rule) | both |
| startup | fork to first visible frame containing the composer prompt | black box |
### 3.3 Capturing from the original, minimal modification
Level 0 (the baseline, no change to codex-rs): everything black box in the table.
Level 1 (optional, one small patch kept on a local branch, NOT upstreamed): in
`tui.rs::draw` and the frame scheduler, record `Instant` before layout/render and
after flush, and the number of Draw notifications coalesced by the limiter; emit
as `tracing::trace!(target="perf", ...)` fields or append to a file named by
`CODEX_PERF_TRACE`. About 20 lines, behind an env check. It gives draw CPU time
and "requests vs frames" which black box cannot separate from the stream
timing. The patch is applied to a copy of the pinned oracle (`docs/rtwk-port/
ORACLE_REF`) built into a separate target dir, so the repo's original stays
untouched and the baseline binary is reproducible. Record in the report which
level each column came from. No allocation hook in the original.
rtwk side: `FrameStats` is already there (`--timeline` in its demos; the port's
binary should expose the same through an env var, `RTWK_TRACE_OUTPUT`/timeline
style as in responsiveness.md), so no patch is needed.
### 3.4 Method rules (parity of the two sides)
- Same pty size, same TERM (`xterm-256color`), same terminal model: the driver's
  vt100 (Python `pyte`-class state machine written in stdlib, or reuse the strict
  model of rtwk `tests/support/vt.rs` through a small rust bin `perf-vt`
  that reads a chunk log; recommended: the Rust one, since it is already strict
  about DEC 2026, alt screen, scroll regions and is the model of the gate).
  The emulator is a model, not a GPU terminal: say so; render time of a real
  terminal is out of scope, but byte volume is the proxy.
- Same scripted backend and timing; same `CODEX_HOME` fixture copied per run;
  same prompt; analytics off; no network; same cwd; `NO_COLOR` unset.
- Screen mode: ADR-005 order. Pair the port's owned alt-screen with the original's
  alt-screen (`--alt-screen` default) for S1-S3, S5-S10: both own the screen, same
  scenario. The original's `--no-alt-screen` (inline, scrollback) is a second
  column set, compared to the port's inline mode when it exists. Never compare
  alt-screen A against inline B; S7 and S8 are mode-dependent and are reported per
  mode, with a mode label on every chart.
- Release builds only: the original `cargo build --release` with the profile the
  repo ships (state `lto`, `codegen-units`, `strip`; rtwk numbers use `dist`;
  build the port with the same profile family and state both). Record `rustc -V`,
  commit hashes, `uptime` before/after, `sysctl hw.model`, power state.
- Interleaving: A B A B per round, order flipped each round (rtwk's gate rotates
  app order); a preflight launch (cold cache: first launch cost 300+ ms seen by
  rtwk) is discarded and reported separately.
- N: latency scenarios 20 runs x 100+ keys, so 2,000 samples per distribution; rate
  scenarios 15 runs. Report run-to-run spread (median of per-run percentiles with
  min-max), not just pooled percentiles; a ratio is "significant" only when the
  per-run bootstrap interval excludes 1.0 and the noise band of the baseline.
- Quiet host: exit 75 when 1-min load > 1.5x CPUs; `--measure-only` to take data
  anyway with the load printed (the machine is shared with other agents' builds).
### 3.5 Micro-benchmarks
Support only, in an appendix labelled as such: criterion benches under our `perf/` (markdown stream to lines, one 1,000-cell frame render+diff, syntect per block) to explain a gap, never to prove one.

## 4. Visual presentation
What the person opens: `perf/results/<run>/report.html`, one file, no server, works
from disk. Regenerated by one command:

```
python3 perf/run.py --scenarios all --a $ORIG_BIN --b $PORT_BIN --n 15   # measure
python3 perf/report.py perf/results/<run>                               # html
python3 perf/trend.py                                                    # trend page
```

Report layout, top to bottom:
1. Header: machine, load before/after, commits, profiles, screen mode, N, a
   "what is compared" sentence (same core, only the frontend differs), pass/fail
   of final-screen equality per scenario.
2. Verdict table: one row per scenario and metric: A p50, B p50, ratio B/A with
   95% interval, a coloured cell (green faster, grey within noise, red slower),
   and the budget if any (rtwk gate budgets as thresholds).
3. Per scenario card: (a) keystroke latency distribution, strip plot of all
   samples plus box and the p50/p95/p99 ticks, A and B on one axis, log scale
   toggle; (b) frame-interval and bytes-per-frame histograms; (c) time series of
   CPU % and RSS over the run, A and B overlaid, with the stream start and end
   marked; (d) for S8, a timeline of bytes per 10 ms around the resize storm
   (spikes visible), with time-to-settle bracketed.
4. Side-by-side player: two panes playing the recorded asciicast of the same
   scenario in lock step (A left, B right), a shared scrubber, the stream-rate
   and key events drawn as ticks on the scrubber. This shows differences a number
   misses (flicker, stalls, redraw).
5. Trend section: the port milestones on the x axis, baseline of the original as a
   horizontal reference line per metric (small multiples, one per scenario);
   from `perf/history.jsonl`.
6. Appendix: micro-benchmarks (labelled), raw run table, and the exact commands.

Tools (all small, stdlib-first; none is installed beyond python3 and ffmpeg):
- Data and stats: python3 stdlib (`statistics`, `random` for the bootstrap,
  `json`, `resource`, `pty`). No numpy needed.
- Charts: hand-written inline SVG from Python (strip plots, box plots, lines,
  histograms are about 150 lines); this is dependency-free and matches how rtwk
  keeps its reports plain. Alternative if richer interaction is wanted: vendor
  one JS file (uPlot ~50 KB, or Chart.js) into `perf/vendor/` and inline it;
  rtwk uses no charting library today (matplotlib is not installed here, and
  the bench dir's `hawk-dashboard` is a UI demo, not a chart tool).
- Recordings: the driver writes asciicast v2 itself (`{"version":2,"width":..}`
  header, `[t,"o",data]` lines; the format rtwk-dev's record also produces, so
  `rtwk play` can replay them and either cast opens in any asciicast player).
  HTML player: vendor `asciinema-player` (one JS + CSS, BSD-style licence; check
  before vendoring). For GIFs/MP4 (docs, PRs): `agg` (cargo install agg or
  `brew install agg`) turns a cast into a GIF; `asciinema` and `vhs` are not
  installed and are not required. `ffmpeg` is present: `agg` GIFs to MP4 or
  `ffmpeg hstack` to put A and B side by side in one file.
  - Where things live (in OUR repo, not codex-rs, not rtwk): `perf/` at the
  top (scripts, `scenarios/*.json`, `vendor/`), `perf/results/<date>-<commit>-
  <machine>/` for runs (JSONL + report.html + casts; casts are large: keep casts in
  `.gitignore` or git-lfs, commit JSONL, HTML and summary only), `perf/history.jsonl`
  (one summary line per milestone, committed) and `perf/baseline/` with the
  frozen original-baseline JSON and the binary's commit hash. Same pattern as
  rtwk's `bench/baselines/<machine>/baseline.json`.

## 5. Phased plan
P0. Baseline of the original (now, no port code, black box only).
  - Build the original release binary from the pinned oracle; record profile.
  - `perf/fake_model.py` (paced SSE), `perf/drive.py` (pty, DA1/CPR, vt feed,
    cast writer, rusage), scenario files S1, S2, S5, S9, S10 first, then S3, S4,
    S6, S8, S11, S7.
  - Dry run: A vs A (original against itself) to measure the harness noise floor;
    the report must show ratios of 1.0 within the band, otherwise fix the harness
    before comparing anything. Commit as `baseline-original` + the first HTML.
P1. Milestone M1 (bridge: ratatui::Buffer to rtwk surface, ADR-005 owned alt-screen).
  Run all scenarios, A original vs B port on the bridge. Expect the bridge to be
  slower than native on S1-S3 (a Buffer copy per frame); the report records it as
  the starting point of the trend, not as a failure. Final-screen equality per
  scenario is the gate that the number is meaningful (oracle snapshots pass).
P2. Native regions milestones (history/transcript as `LinesView`, composer, footer,
  pickers): re-run after each region lands (S1/S3 after transcript, S5 after
  composer, S6 after picker, S7/S8 after owned scroll). Each run appends to
  `history.jsonl`; trend page updates. A region may not regress its scenario's p99
  beyond the baseline noise band without a recorded reason (bench-check rule).
P3. Final (bridge removed, inline mode second per ADR-005): full matrix both modes,
  both sizes, N=20, quiet machine, optional Level 1 hook; publish report + GIFs
  (agg) for S1, S5, S8; set the regression thresholds (as rtwk's PERFORMANCE.md
  "Regression thresholds") and a `cargo xtask`-style `perf-check` using the frozen
  baseline JSON; wire into the ADR-007 "performance gate".

## 6. Open questions
1. Which binary is B in the milestones: `codex --frontend rtwk` or a separate
   `codex-rtwk` (architecture.md decides "then")? The driver takes two paths, so it
   does not matter, but the report labels need the choice.
2. Does the port keep the original's `--no-alt-screen` inline behaviour at M1? If
   not, only the alt-screen column set is comparable until inline lands (ADR-005).
3. Reuse test-infra's fake backend or the Python SSE server? Pacing is the only
   hard requirement; a rollout fixture for the long-transcript start state is theirs.
4. Terminal model: rust strict model (`vt.rs`, cross-repo dependency, recommended) or stdlib Python vt100?
5. Machine: all numbers on this shared M1 Max under agent load; do we require a
   quiet run (load < 1.5x CPUs) for any number the person sees, as the gate does?
7. Allocation counts: accept "n/a" for the original, or a counting-allocator build of both?

