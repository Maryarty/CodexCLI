# Calls before a porter's first edit (wave 1, 2026-11-22)

Script: `scripts/pre_edit.py <scripts dir> <transcript.jsonl>...` (per-call bucket list before the first edit).

| porter | calls to 1st edit | min | sub-commands | oracle | own | protocol | test infra | other | total calls / wall |
|---|---|---|---|---|---|---|---|---|---|
| status-facts | 63 | 7 | ~166 | 21 | 16 | 12 | 6 | 8 | 89 / 21 min |
| popups (resumed: gate first) | 35 | 19 | ~51 | – | – | – | 3 | 32 build/wait/tm/git | 157 / 43 min |
| host-effects | 29 | 4 | ~83 | – | 21 | – | 2 | 6 | 83 / 23 min |
| cells | 41 | 5 | ~90 | 10 | 14 | – | 8 | 9 | 160 / 32 min |
| composer (resumed) | 27 | 4 | ~51 | 3 | 11 | – | 5 | 8 | 125 / 27 min |
| before (porter-steps report) | 54 | – | – | | | | | | 137 / 44 min per step |

Findings
- Not wasted calls: each call is a compound `cd && sed; rg; rg -A` research of 3-5 commands, sequential (1 call per turn, never the parallel first turn of codex-porter.md).
- status-facts traced all 8 facts (mode, side, agent, token pending, git, headline, model names) end to end before writing a line: per fact the oracle setter and its callers, the protocol struct, our decode/apply site, the fixture format.
- The task's Files named the display ranges of the oracle and a bare `chat.rs` (834 lines, read whole in 3 calls), not where each value is set and where it lands in ours.
- 12 protocol-type lookups by `rg -A`: `sym` did not search the protocol crates.
- Every porter rediscovered the core's wiring (project -> ThreadEv -> apply, Command/AppFx, test script format).
- Fixed overheads: `get_task` + `task_start` (same text), 2-4 git state checks.

Fixes (this commit)
- codex-porter.md Start: task_start only; no git checks; item by item (trace one item, edit, then the next); `sym` for definitions.
- `sym --proto` (app-server-protocol, protocol).
- fleet-brief.md "Where things are": the core's wiring by name and a core test to copy.
- rewrite/FORMAT.md: Files = one line per item, oracle origin file:line -> our landing file:line, ranges <=120 lines; area lists every landing file.
