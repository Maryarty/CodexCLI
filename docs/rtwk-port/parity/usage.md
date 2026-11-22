# usage — parity (S20)

Oracle `codex-rs/tui/src/analytics.rs` + `analytics/` (27 files), `chatwidget/usage.rs`, `reset_credits.rs`.
View `src/pages/usage/`, core `core/src/screens/usage/`. Brief `research/briefs/usage.md` §6. The page is the
app-scoped `usage::Model` in `ScreenStates` (ADR-003: one writer, `apply`; no thread state); the host runs the
loads (not yet, step 8); widget state is cursors and scroll controllers only.

## chunk-usage, state at partial 1
Branch `worktree-agent-afdce575fa199675f`, rebased on claude/tui-rtwk `4361b77b9`. Steps 2 (core), 3 (chrome)
and 4 (Overview) are done; steps 5 (plots), 6 (reports), 7 (modes, mouse), 8 (host), 9 (menu, reset
credits) and 10 (binary run, generated shard) are NOT.

### Built
- **Core model** (`core/src/screens/usage/`): `mod.rs` (`Model`, `step`), `sections.rs` (`Section`,
  `visible_sections`, ranges, `RangeGroup`), `data.rs` (`Load<T>`), `normalize.rs`, `report_data.rs`,
  `tokens.rs`, `models.rs` (model filter), `profile.rs`, `date.rs`, `activity.rs` (heat map + chart
  rows). Loads are pure `Ask{session, stamp, what: AskKind}` values the host will run; a late reply of an
  old stamp is dropped in `apply` (the oracle's generation). `Ev::{Account, Report, Profile, Invalidated, Seed}`,
  `Intent::{RefreshRateLimits, Open{view}, Refresh, Close, Select, Range, Group, Model, ZeroCreditGroups,
  TaskMetric}`. Seeds: `{"step":"screen","dir":"usage","ev":{account, section, histories, profile}}`.
- **Page** (`src/pages/usage/`): heading, `Tabs` (numbered from 80 cells), rule, controls (range, group,
  model: `GestureDetector(Text)`, tappable), `Expanded` body, two footer rows; keys 1-7, Tab, BackTab,
  r, g, m, R, Esc; `z` and `?` are `page_keys::not_yet` popups. Overview: figures, heat map (`Square`
  grid), insights, chart, footer; ready reports of the other tabs draw nothing yet (message only).
- Tests: core 48 (`core/tests/usage/{normalize,tokens,state,seed}.rs`), page 3+ (`tests/pages/usage.rs`:
  `summary_daily_wide` at 140x44 equals the oracle snapshot whole, panel states, digit keys).

### Checks at this tip
`acargo fmt`, clippy `--workspace --all-targets -D warnings` exit 0, whole nextest 857 run / 856 pass; the
one red is the known flake `shell_agents::typing_a_word_asks_the_server_once` (known-red.txt),
`shards.py --check` 0 files stale. R14 (`r14_widget_state_fields_hold_no_screen_model_types`) was red
because two of my core types were named `Cell` and `What` (the scan flags `Rc<Cell<bool>>` of other
owners and a doc comment of agents); renamed to `Square` and `AskKind`, green now.

### Outside the original area (area widened on the record, additive)
- `core/tests/agents_followups.rs:243`: `usage::Ev::Invalidated{session:0}` (the Ev changed shape).
- `tests/app/navigation.rs` `usage_footer_keys_are_bound`: the live page starts at account `Loading`:
  footer `esc back` + `R retry`, R/Tab keep the page (the old test pinned the static mock page).
- `core/tests/usage.rs` (renamed into `core/tests/usage/`).

### Needs of others
- `src/pages/session/open.rs:48` (chunk-transcript) uses `usage::route()` for `/usage weekly|daily|cumulative`
  too; the page has `route_with(view)`: wire it there.
- `core/src/app.rs` + `host/src/in_process.rs` (keymap): `Command::Usage{ask}` arm and reply routing; until
  then the live page stays at Loading.
- Kit palette reader (`tests/oracle_snap/mod.rs`, `tests/kit/compare.rs`) for the 10 plot snaps.

### Left, in order
Plots (CustomPaint), reports (Credits/Tokens, Messages, Plugins, Skills, Chats, Plan), modes (focus, `z`, `?`),
keys and mouse tests, host service `host/src/analytics.rs`, usage menu + reset credits (chat_live_6 26),
English text of core into string tables, the generated shard (after the person's yes), binary run.
