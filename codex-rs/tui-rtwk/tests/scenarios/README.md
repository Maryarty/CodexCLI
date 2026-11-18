# pty scenarios

`codex` (the pinned oracle, `~/.cache/codexcli/oracle-bin/codex`) and
`codex-rtwk` run in a pseudo-terminal (rtwk-term `TerminalSession`: the pty and
an emulator that answers the startup probes) against the same mocked model, in
the same kind of fresh `CODEX_HOME`, driven by the same key script. Screens are
compared at named checkpoints (rows, cursor, screen in use; the scrollback in
inline scenarios) after timers, clock times, ids, versions, spinner frames, temp
paths and ports are normalized. On a difference both screens are written side by
side to `target/tmp/scenarios/<name>/<checkpoint>.txt`; every checkpoint's own
screens are there too (`<checkpoint>.oracle.txt`, `<checkpoint>.rtwk.txt`).

Design: architecture.md ADR-007 §2, research/test-infra.md §D3.

## Run

The scenario tests are `#[ignore]`d, so the normal suite stays fast. From
`codex-rs/tui-rtwk`:

```sh
# the binaries: the oracle is pinned; build ours (debug is enough)
~/.cache/rtwk/acargo build -p codex-rtwk
# every scenario (about a minute)
~/.cache/rtwk/acargo nextest run --run-ignored only --test scenarios
# one scenario, and see what it prints (skips, expected gaps)
~/.cache/rtwk/acargo nextest run --run-ignored only --no-capture --test scenarios -E 'test(agents_parity)'
# a slice that claims parity: expected gaps fail too
SCENARIO_STRICT=1 ~/.cache/rtwk/acargo nextest run --run-ignored only --test scenarios
```

`RTWK_BIN` / `ORACLE_BIN` name other binaries (`tools/rtwk-port/mock-model/bins.py`
looks for a built `codex-rtwk` in this tree, the main checkout, any worktree).
`KEEP_SCENARIO_HOMES=1` keeps the temp roots (`/tmp/rtwk-scn-<pid>`).

The harness's own tests (`unit.rs`: normalizer, script parser, comparison,
report) run in the normal suite.

## Tests per scenario `<name>`

- `<name>_oracle`: the oracle alone reaches every checkpoint and exits 0 (the
  script is valid).
- `<name>_oracle_is_stable`: the oracle twice, strictly compared (nothing
  nondeterministic is left in the comparison).
- `<name>_parity`: oracle against codex-rtwk.
  - A checkpoint with a `"gap":"VS2: why"` that still differs is printed as
    `EXPECTED GAP` and passes; it fails under `SCENARIO_STRICT=1`, and fails
    when it has closed (the marker must then be deleted), so a gap neither
    hides nor outlives its cause. A difference at a checkpoint without a gap
    fails.
  - codex-rtwk ending early with `unknown argument` (it has no `agents` entry
    yet) is a `SKIPPED` with the reason.

## Scenario files (`<name>.jsonl`)

JSONL: a header, then steps (the full grammar is in `tests/support/script.rs`).

```
{"scenario":"agents","mock":"text-100","seed":"agents-3","size":[120,40],
 "oracle_args":["-c","analytics.enabled=false","agents","--no-daemon"],
 "rtwk_args":["agents"],"timeout_ms":15000}
{"check":"overview","until":"Refactor the parser module"}
{"sleep_ms":2000}                      the app takes input only after it settled
{"key":"down"}
{"check":"second-row"}
{"wait_exit":true,"gap":"..."}
```

- `mock`: a scenario of `tools/rtwk-port/mock-model/scenarios` (Codex's own
  `MockResponsesServer`); `seed`: files written into the fresh home
  (`tests/support/seed.rs`): `agents-3` is three saved sessions as rollouts.
- Each binary gets its own server and home made by `home.py`; they run one
  after the other, in the same path.
- After input, a checkpoint waits for the program's output (up to 0.8 s), then
  for 150 ms of quiet outside a synchronized update (up to 5 s).
- A `check`'s `until` text is waited for first; an `until` that never shows is a
  difference at that checkpoint.

## Scenarios

| file | what | state |
| --- | --- | --- |
| `text-100.jsonl` | prompt, streamed answer (600 deltas), `/quit` | oracle green; codex-rtwk streams the answer; all three checkpoints differ (gaps `VS2`: the chat is the skeleton's) |
| `agents.jsonl` | `codex agents --no-daemon` with three saved sessions: overview, move, help, filters, Ctrl+C | oracle green; codex-rtwk skipped until it has the `agents` entry (agents-live) |
