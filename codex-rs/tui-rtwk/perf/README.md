# perf: Codex TUI on rtwk against the original (pty A/B)

Design: `docs/rtwk-port/research/perf-compare.md`, ADR-008 in `docs/rtwk-port/architecture.md`.
Stdlib Python only, run with `python3 -I`. Nothing here touches `~/.codex` or a real model.

| file | what |
|---|---|
| `drive.py` | forks a binary in a pty at the scenario size, answers DA1/CPR/DSR/kitty/DECRQM/OSC 10,11 (`vt.py`), measures, writes `runs.jsonl`, `meta.json`, `casts/*.cast.gz` (asciicast v2, not committed) |
| `vt.py` | the text VT model (grid, scroll regions, alt screen, DEC 2026, mode 2048 in-band resize reports; DECRQM answers 0 for modes it does not implement) |
| `mock.py` | one run's fake model: Codex's own MockResponsesServer (`tools/rtwk-port/mock-model`: server.py, home.py, scenario.py, harness.py, unchanged) with a parameterised text scenario (deltas, pace, words, begin and end tokens) and a fresh CODEX_HOME |
| `scenarios/S*.json` | S1 stream 100/s, S2 1000/s, S5 typing 10 and 30 keys/s while streaming, S9 startup (fresh CODEX_HOME), S9w the same on a warm home (a discarded launch first creates the state DB), S10 idle after a turn, S10w idle with a turn open (model silent), S11 Agent command center (`codex agents --no-daemon`; skipped on codex-rtwk until it has the subcommand), S12 open a session: Enter on a seeded 80-turn session in the center -> chat composer (the seed is built once by the oracle's `codex exec` / `exec resume --last` against the mock, kept as `/tmp/perf-seed-S12.tmpl`, copied per run), S13 resize storm (40 random sizes in 2 s after a streamed answer; `Session.resize` sends TIOCSWINSZ + SIGWINCH, and the in-band `CSI 48;rows;cols;h;w t` report when the app set mode 2048) |
| `stats.py`, `report.py` | metrics, bootstrap ratio, `report.html` (one self-contained file) and `summary.json` |
| `trend.py` | `trend.py add <run> --milestone NAME` appends to `history.jsonl`; draws the milestone trend |
| `results/<name>/` | `runs.jsonl`, `meta.json`, `summary.json`, `report.html` committed; `casts/` ignored |

```
# measure (oracle = the pinned release in ~/.cache/codexcli/oracle-bin/codex)
python3 -I perf/drive.py --a oracle=$HOME/.cache/codexcli/oracle-bin/codex --b rtwk=codex-rs/tui-rtwk/target/release/codex-rtwk \
    --scenarios S1,S2,S5,S9,S10,S10w,S11 --n 15 --out perf/results/<name> [--measure-only] [--wait-quiet 600]
python3 -I perf/report.py perf/results/<name> --history perf/history.jsonl
python3 -I perf/trend.py add perf/results/<name> --milestone <name>
```

Rules: release builds only; runs interleaved A B, B A, ...; one discarded preflight launch per binary;
exit 75 and no data when the 1-minute load is above 1.5 x CPUs unless `--measure-only` (the load before and
after every run is kept and shown). `--n 15` for rate scenarios. A run set can be resumed (same `--out`).

Method notes:
- Black box through the pty. CPU: `proc_pidinfo` every 100 ms (window CPU) and wait4 rusage (whole run);
  RSS: the same samples and `ru_maxrss`. Frames: DEC 2026 pairs, read bursts (gap 2 ms) when absent.
- A key is visible when the VT model shows the typed text (the last 6 typed letters) in the screen; a DEC 2026
  update held longer than 150 ms counts as visible. First-byte latency is kept as the secondary column.
- The mock paces with `sleep()`, so the achieved delta rate is below nominal (the report prints it).
- Colours are not modelled: the side-by-side replay is text only.
