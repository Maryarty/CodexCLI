# chunk-popups-review — parity

Code: `src/sheets/select/review.rs` (list builders for the presets, base branches and commits; the
instructions prompt), `review/strings.rs` (ADR-010 words), `dialog.rs` (routes `Menu::Review`,
`Head.cursor`), `src/pages/session/actions.rs` (`/review` -> `Intent::OpenReview{cwd}`). Core, the git
asks and `Picked::Review` were core-pass's; the app starts the review.

Shard counts (shards.py / summary.py --run): chat_popups_4: pass 13 / n/a 7 / pending 40 (dialogs-pane) / red 1.
Green now: `review_picker_filtered_accept_and_cancel_preserve_parent_and_draft` (live flow: f3 selects,
f2 goes back, the commit pick starts `ReviewTarget::Commit`). The one red,
`reasoning_up_shortcuts_reach_max_in_default_and_plan_modes` ("M A X" animation frames), is not this task's.
chat_live_5: pass 18 / pending 20, unchanged. Its three review picker tests stay `pending` and did NOT flip
(chunk-live's; probed with KIT_PROBE_PENDING=1): `review_custom_prompt_escape_...` asserts
`assert_absent("Custom review instructions")` after Esc, which the presets row `4. Custom review instructions`
keeps on screen (the oracle reads the header row only); `review_branch_picker_escape_...` and
`review_commit_picker_shows_subjects_...` need the git answer: `Screen::answer_effect(&command, Done::GitBranches/GitLog)`
(new kit helper) after the ask in `s.commands()`.

Decisions:
- The prompt is its own small widget (TextArea over a `Dialog::bare` column). chunk-commands builds the
  generic `Open::Prompt` (`src/sheets/dialogs/prompt.rs`): when it lands, the review prompt moves onto it (dedupe).
- The oracle test's "draft kept" is not assertable: `/review` consumes the composer (the slash command
  is removed from it, as in the oracle's own flow); the test asserts an empty composer after the pick.
- No panel tint for the child pickers (the picker has none either); no short-terminal gap degradation
  for the prompt (the oracle's view drops its blank rows on a short terminal).
- Outside the original area (area widened): `tests/kit/mod.rs` `Screen::answer_effect`.

Binary: own pty driver beside the oracle (`/tmp` throwaway: temp repo with 2 commits, branch `feature/parser`,
100x36, multi-turn mock home; rtwk `codex-rtwk` vs oracle `codex --no-daemon`). Steps: `/review`,
base branch picker (enter, down, esc), commit picker (enter, type `Fix`, `zz`, esc), instructions prompt
(enter blank, type, esc), esc closes. Text of every step is EQUAL (presets with the cursor on the opening row,
`Current branch: feature/parser`, `Type to search branches`, `no matches`); only the footer's `fast` differs
(status line, not this task). Pick of `Review uncommitted changes` starts the review in both
(`>> Code review started: current changes <<`), rows that differ afterwards (review lifecycle, chunk-live):
rtwk draws an empty `•` row after the start line, `<< Code review finished: <text> >>` where the oracle
prints `<< Code review finished >>` then the agent message, the `Working` row stays after the finish, and the
`Worked for` time shows `8:09 AM` against `08:09`. land-check's mock scenarios: 39/41, the two known reds (error-context-window, multi-agent).

Mouse: `tests/pages/review.rs`: tap on a preset starts the review, tap on a commit row picks it, the wheel
scrolls a 40-commit list and a tap picks a row it brought in.
Tests: `tests/pages/review.rs` (11): presets order, preset pick, tap, branch picker (subtitle, filter, pick),
detached HEAD + Esc cursor row, commit pick/filter by sha/wheel, instructions submit/blank/Esc, `/review <text>`.

Rows for the lander: none (no shard file changed beyond `chat_popups_4.rs`; `shards.py --check` clean).
