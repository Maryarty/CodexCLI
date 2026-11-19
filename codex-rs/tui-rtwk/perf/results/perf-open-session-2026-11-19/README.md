# perf-open-session (2026-11-19): S12, Enter on a seeded 80-turn session in the Agent command center -> chat page ready

Release builds, N=15, interleaved, load < 15 (6-7 at the runs). `runs.jsonl` = after the fix, `before/runs.jsonl` = the
same binary before it (oracle rows are the same measurement). t_open = Enter -> the chat's composer on screen.

| binary | open, median (min-max) ms |
|---|---|
| oracle `codex agents --no-daemon` | 85 (82-103) |
| rtwk release, before | 77 (72-118) |
| rtwk release, after (`thread/resume` with `exclude_turns`) | 56 (52-70) |
| rtwk DEBUG build (the old try binary), N=7 | 214 (209-235) |

The slowness the person saw is the debug build (2.8x release). The chat page of codex-rtwk does not replay the history
yet (no transcript on screen after Enter; the oracle shows it): not part of the timing.
