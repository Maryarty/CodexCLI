# perf-first-frame (2026-11-19)

Change: `host/src/main.rs` no longer builds the config, opens the state DB or makes the
EnvironmentManager before `run()`; `InProcess::run` (the backend future, started by the shell's
`on_start`) does it after the first paint, and a failure arrives as `AppEv::StartFailed`
(Flutter: runApp a light root, the heavy init a FutureBuilder-like async start). Also:
`[profile.dist]` (thin LTO, codegen-units 4; default release unchanged), `perf/scenarios/S9w.json`
(warm home: a discarded first launch creates the state DB), `drive.py` dropped `--no-daemon` for
codex-rtwk (it had no daemon; S11 never became ready on rtwk without this). Since the chat
auto-starts the shared background server (`f-chat-daemon-autostart`), `drive.py` passes
`--no-daemon` again for the rtwk chat; a subcommand run keeps its own mode.

Release builds (plain `cargo build --release`, strip symbols), N=15, drive.py, runs with load < 15.
Ready = the composer text (S9, S9w) or "Agent command center" (S11), median ms (min..max).

| scenario | oracle | rtwk before | rtwk after |
|---|---|---|---|
| S9 fresh home | 22.9 (21.4..26.7) | 52.1 (45.8..140.4) | 18.0 (17.1..34.6) |
| S9w warm home | 23.0 (21.5..58.4) | 34.9 (30.1..80.7) | 18.4 (17.3..46.7) |
| S11 command center | 100.5 (89.1..318.9) | 57.0 (50.0..141.5) | 18.2 (17.0..29.3) |
| max RSS S9 (MB) | 128 | 82 | 82 |
| max RSS S11 (MB) | 100 | 66 | 66 |

Raw runs: `codex-rs/tui-rtwk/perf/results/pff-after-2026-11-19` (oracle + after) and
`pff-before-2026-11-19`. "Before" is the parent commit's host code, built the same way.
