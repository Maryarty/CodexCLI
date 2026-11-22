# status-card — parity (chunk 1: the card in core, `StatusCell` from seeds)

Slice: S19 `/status`. Oracle: codex-tui `status/` (`card.rs`, `helpers.rs`, `rate_limits.rs`,
`format.rs`), tests `status/tests.rs`. Brief: `research/briefs/status-card.md` §6 chunk 1.
No app wiring in this chunk: the live `/status` still opens the dialog.

## Shard `status` (oracle profile)

| | pass | n/a | pending | red | total |
|---|---|---|---|---|---|
| before | 0 | 7 | 27 | 0 | 34 |
| after | 24 | 7 | 3 | 0 | 34 |

Stale probe (`KIT_PROBE_PENDING=1 … --list stale`): none. Pending, by reason:
- `status_snapshot_wraps_halfwidth_kana_in_narrow_terminal`: G44 (kana + sound mark is one cell) and a free-text plan
  (the oracle names `ｶﾞﾊﾟ plan`; our account holds the plan enum): no entry yet.
- `transcript_overlay_remeasures_status_after_rate_limit_refresh`: S16 pager and chunk 3 (the refresh handle).
- `status_snapshot_includes_enterprise_monthly_credit_limit`: the 92-column half is live; the 46-column half differs
  in two wraps: G75 (UAX #14: `used / 272K)` stays whole) and G76 (optimal-fit wrap of `(resets 07:08 on 7 May)`).
n/a (7): the oracle asserts helpers' return values; 6 carried by name in `core/tests/status.rs`; the 7th
(`format_directory_display_truncates_halfwidth_sound_marks`) is not: the card passes `max_width = None`.

## What is here
- Core: `core/src/screens/status.rs`, `status/{helpers,limits,card}.rs` (typed rows: plan, agents, limits, credits,
  permissions, reset times; `now` and the clock preference are facts, no prose); `core/tests/status.rs` (33).
- View: `pages/session/cells/status/{cell.rs,strings.rs,mod.rs,fixtures/}`: `StatusCell` = `Column` + `Grid`
  (`content`, `fr(1)`, gap 2) + `Text::rich`; one `LayoutBuilder` + `segmenter_of` picks the spelling from the
  measured value room (bar or percentage, reset inline or under), `measure::rows` flows a row whose glyph is
  wider than its track (oracle's un-indented re-wrap). 32 cell entries + `card_dialog` (the dialog's old fixture).
- Tests: 4 page tests (`tests/pages/status.rs`; the bar rule shown biting once), the 3 dialog tests retargeted.
- Mouse: the cell has no selectable rows (ADR-011 n/a); the usage URL is an OSC 8 `Span::link`, walked by the shard.

## Departures and debt
- ADR-003: `ScreenStates.status.card` + `CardDialog` stay; added seed-only `Model.cell`, `Ev::CellShown`, seed kind
  `status_card` (the cell is not yet a transcript item through `apply`). Debt task to create (this agent has no tm
  create call): "status-card-adr003: the /status card as a transcript cell through apply, drop ScreenStates.status.card".
- Core test fixes of the rules: the three-word `// oracle:` comments of core/status carry `(adapted…)` so the
  carried-fragment rule does not read them as verbatim carries; the status `Sandbox` enum is `SandboxKind` (R14 clash
  with `fixtures::sandbox::Sandbox`).

## Handover (chunks 2-5)
- `Item::Status` + `/status` through `apply` (drops the dialog path); host facts (clock preference, rate-limit
  snapshots, account, permissions); `copy_text` and the hyperlink lines of the cell; `thread_usage` rows
  (`thread_usage.push_labels/lines`, plan type) not carried; the refresh handle + transcript overlay (S16).
- Registration outside the area: none needed (`pub use cell::StatusCell` in `status/mod.rs`).
- rtwk gaps added: G75, G76 (`rtwk-gaps.md`); G44 existing.

## Binary
`codex-rtwk --debug-screens` (pty 92x40) to `status/enterprise_monthly_credit`: rows 1-16 equal the oracle snapshot
`status_snapshot_includes_enterprise_monthly_credit_limit`. `check.py --only token-usage`: PASS both binaries (no
scenario types `/status`; the dialog path is untouched).
