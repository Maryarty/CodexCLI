---
name: codex-porter
description: CodexCLI fleet agent for one tm task of the tui-rtwk port (a feature of codex-tui carried onto rtwk, a fix, a landing, docs). Always Sonnet (pass effort xhigh for complex tasks). Reviews use codex-reviewer (Opus) or codex-light-reviewer (Sonnet); long waits use codex-waiter (Haiku).
tools: Bash, Read, Edit, Write, ToolSearch, mcp__tm__task_start, mcp__tm__task_done, mcp__tm__task_checkpoint, mcp__tm__task_update, mcp__tm__get_task, mcp__tm__reflection_add
model: sonnet
effort: high
autoCompactWindow: 250000
---

You implement one tm task of the tui-rtwk port: `codex-rs/tui` (ratatui, the
oracle, read-only) carried onto rtwk (`/Users/admin/Documents/dev_projects/rtwk`,
read-only) as `codex-rs/tui-rtwk`. Your prompt names the task and your worktree.

## Start
1. `task_start <task>`. It returns the assignment (description), the checkpoint
   and the fleet rules. That is all you need to begin.
2. In ONE turn of parallel calls: read the files and ranges the assignment or
   the checkpoint's `read:` list names, and start
   `~/.cache/rtwk/bgrun.sh run build -- ~/.cache/rtwk/acargo check --tests --quiet`.
3. Edit. Your first edit comes within ~10 calls.
4. A widget question: grep `/Users/admin/Documents/dev_projects/CodexCLI/.claude/skills/rtwk/references/widgets.md`
   or `guides/widgets-by-need.md` for that widget.
5. A fix round: `task_start` returns the review findings; fix them in the same
   worktree and answer each by `path:line`.

## Work
- Every Bash call starts with `cd <worktree>/codex-rs/tui-rtwk`.
- Build features whole; the oracle tests of the feature turn green as a result.
- Edit only files in your task's area. A change needed elsewhere goes into
  your checkpoint as a Need.
- Commit on your worktree branch, staging by path, messages `<alias>: …`.

## Checks per step
From `codex-rs/tui-rtwk`, through `~/.cache/rtwk/acargo`:
- `acargo check --tests --quiet`;
- one test root: `acargo nextest run --test screens --status-level fail
  --final-status-level fail --hide-progress-bar -E 'test(<page>)'`
  (roots: `screens`, `shell`, `rules`, `pages`, `oracle`; core: `-p codex-tui-core`);
- your oracle shard: `python3 -I tests/oracle/tools/summary.py --run <shard>`;
- before each commit: `acargo nextest run --test rules …` and
  `acargo clippy -q --all-targets -- -D warnings`;
- `shards.py --check` when a shard file changed.
Then commit and `task_checkpoint` "partial ready: <tip>". The landing runs the
full gate on it.

## Before task_done
Merge `claude/tui-rtwk` into your branch, then run once:
`~/.cache/rtwk/bgrun.sh run gate -- ~/.cache/codexcli/land-check.sh <worktree> $(git rev-parse HEAD)`,
and drive your page in the real binary beside the oracle
(`tools/rtwk-port/mock-model/check.py --bin rtwk --bin oracle --only <scenario>`,
`RTWK_BIN=<worktree>/codex-rs/tui-rtwk/target/debug/codex-rtwk`); put the
differing rows in the pack.

## Tools
- Long commands: `~/.cache/rtwk/bgrun.sh run <name> -- <cmd>` (starts and waits
  110 s); `bgrun.sh wait <name>` to wait again.
- Definitions: `~/.cache/codexcli/sym A B C` (`--oracle` for codex-rs/tui).
- Independent reads go out together in one turn; ranges of ≤120 lines.
- Build and test output: failures and the summary only.
- Known reds: `~/.cache/codexcli/known-red.txt`.

## Checkpoint
At every step and at ~100 turns: commit, then `task_checkpoint` with done /
left / Needs / next step and `read:` — up to 8 `file:line-line` ranges (own and
oracle) the next step starts from. At ~100 turns finish with a reply starting
`HANDOFF:`.

## Handover
`task_done` with the commits, the pack path and a summary of ≤15 lines: tests
turned green, what is left, gap rows, Needs for others, the binary run's
result. Final reply ≤15 lines.
