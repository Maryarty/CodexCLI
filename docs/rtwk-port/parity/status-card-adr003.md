# status-card-adr003: parity pack

Branch fleet/block-status (code 2a9005a89, merge da25dffff, fmt aeb6d2def; landing candidate 8dde4a118 =
aeb6d2def + tip). Written by the orchestrator from the task_done deliverable (the area had no docs/).

## Features
- /status is a session command (`SessionIntent::Status`, oracle `SlashCommand::Status`):
  core/src/app/session.rs `status_command` builds a StatusCard from the thread's facts (model, effort, provider,
  cwd, permissions, token/context slot, rate limits) and appends `Item::Status(Arc<StatusCard>)` through
  `apply_thread(ThreadEv::Appended)` (ADR-003). `StatusCell` is a transcript cell (cells/dispatch.rs, Gap::Rows in
  text.rs). A refresh of the account's limits follows (ReadRateLimits).
- Gone: CardDialog, `ScreenStates.status.card/cell`, `status::Intent::Open/Close`, `Ev::Shown/CellShown`,
  `Target::Status`, the `card_dialog` fixture, `format.rs`, the dialog's strings. `status::Ev::Card` (seed kind
  `status_card`) puts a seeded card into the active thread; `status::Ev::Clock` carries now / utc offset from the
  shell's ambient loop.

## Tests
- oracle shard status: 24 pass / 7 n/a / 3 pending / 0 red (unchanged; pending: G44 kana plan, S16 pager
  overlay, G75/G76 wraps).
- core: model.rs, status.rs, commands.rs, adr003.rs moved to the session command. pages: tests/pages/status.rs
  `slash_status_puts_the_card_into_the_transcript_and_opens_no_dialog`,
  `the_card_stays_in_the_transcript_when_the_chat_goes_on`; the three dialog tests deleted; transitions: the
  /status row removed, the popup pick test now picks /usage.
- land-check on da25dffff: nextest 3016 passed, scenarios only the known reds; fmt fixed in aeb6d2def.

## Departures and debt
- tests/rules/hub_budget.txt: SessionIntent 43 -> 44 (reason written beside it): the slash list stays
  SessionIntent's and `Target::Status` (no budget) went. The slash list has no generic Feature variant yet —
  Need of the ADR-012 owner (core-slices-migrate).
- Out-of-area edits (small, additive): core/src/{threads,app}.rs, app/session.rs, commands/dispatch.rs,
  src/pages/session/{actions,open}.rs, cells/{dispatch,text}.rs, services/shell.rs, tests/pages/{chat,transitions}.rs,
  core/tests/{model,adr003,commands}.rs.
- Later children: host card facts (network, extra roots, account, catalog display name and reasoning detail,
  remote, instruction sources, forked_from, collaboration mode, hour24), thread name, request-id refresh of a
  shown card, copy data, thread-usage rows, server row, limit warnings, RateLimitStore (reference bd241cb48 rates.rs).

## Binary (pty 100x40, mock token-usage turn then /status)
Both binaries show the card as a transcript cell under `/status`, no dialog. Equal rows: Model provider,
Directory, Agents.md, Session, Token usage `10K total (9K input + 1K output)`, Context window
`100% left (12K used / 258K)`, Limits `data not available yet`. Differ (all for chunk-status-card):
- title `(v0.0.0)` vs `(v0.170.0)` (crate version);
- Model: `gpt-6-sol` vs `GPT-6-Sol (reasoning medium, summaries auto)`;
- Permissions: `Read Only (never)` vs the oracle's profile-id label;
- `Thread name: …` and `Collaboration mode: Default` rows absent in ours.
