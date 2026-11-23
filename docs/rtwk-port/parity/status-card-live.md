# status-card-live (chunk-status-card): parity pack

Branch fleet/block-status. Code commits 84ea2f024 (host deps, Cargo.toml + Cargo.lock) and e7f765d7d (the card), then the
collaboration-mode commit and this pack.

## Features
- `/status` appends the card to the shown thread's transcript (`Item::Status { card, request }`); the card is composed
  purely (`core/src/screens/status/compose.rs` `StatusCard::compose(Inputs)`) from the thread's facts, the config's
  `CardFacts` (cwd, permission profile, sandbox, network, extra roots, approval, reasoning, summary, first-party provider,
  context window), the account, the model catalog, the remote connection and the instruction sources.
- Rate limits: `RateLimitStore` keeps a bucket per limit id and the reset credits per backend; a refresh asked by `/status`
  carries `RateLimitOrigin::Status(request)`; the answer finishes only the cell whose request id matches
  (`status::ThreadEv::Refreshed`, applied to the transcript in `threads::apply_ev`: no new hub variant); a later card reuses
  the cached limits; an account change finishes waiting cards with empty snapshots; a session without a ChatGPT account (or a
  provider that needs no OpenAI auth) reads nothing.
- Host pushes (`host/src/facts.rs`, `host/src/in_process.rs`): `card_account` (account/read: email, plan, workspace as
  `Name (id)`, API key), `card_catalog` (model/list, hidden included: display name and default effort), `card_remote`
  (address without credentials and the server's version), `card(config)` (the permission summary classified as the oracle's).
  At startup and after a reconnect. `thread/start` carries the instruction sources into the thread.
- Collaboration mode row (`collaboration_mode_label`): shown while the modes are on and the mode is visible.

## Tests
- Oracle shard chat_status_2: 20 pass / 1 n/a / 38 pending / 0 red (was 15/1/43). Green now:
  status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth,
  status_command_refresh_updates_cached_limits_for_future_status_outputs,
  status_command_uses_catalog_default_reasoning_when_config_empty,
  status_command_renders_native_and_foreign_instruction_sources,
  status_command_overlapping_refreshes_update_matching_cells_only.
- Core: `mod live` of core/tests/status.rs (16 tests: request ids, cached limits, account change, catalog, thread effort,
  forked thread, collaboration mode, no read without ChatGPT), model.rs reads through the usage menu; core 1553 pass,
  rules 80 pass, clippy green.
- Kit: `Open::thread_start(response)` (tests/kit/env.rs, seed.rs) gives the session's `thread/start` answer; the fake backend
  answers `thread/start` through the responder when it scripts one (src/services/backend/fake.rs).

## Binary (pty 100x40, mock turn then /status, rtwk beside the oracle)
Equal rows: Model `GPT-6-Sol (reasoning medium, summaries auto)`, Model provider, Directory, Permissions `Read Only (never)`,
Agents.md `<none>`, Collaboration mode `Default`, Session, Token usage `10K total (9K input + 1K output)`, Context window
`100% left (10K used / 258K)`, Limits `data not available yet`. Differ:
- title `(v0.0.0)` vs `(v0.170.0)`: the version of the build (this crate's package version).
- `Thread name: Mock scenario` absent in ours: the oracle generates the title with a model request (1 title request), the
  port makes none (thread title generation is not built).

## Gap rows
- Facts feed of status-indicators (FactEv::Cwd/Clock/SideLabel/AgentLabel, git summary and headline probes as Commands): NOT
  built. Adding the probes as `Ask`/`Done` variants (hub budget Ask 11, Done 11) or `Command` (budget 20) fails
  `hub_enums_only_shrink`; it is a feature slice (ADR-012) or a budget decision of the ADR-012 owner. SideLabel has no model
  source (the side conversation is not built); AgentLabel needs the agent navigation state (multi-agent threads).
  The git summary is oracle branch_summary.rs (git + `gh`, ~400 lines): a core step machine like `diff::Diff` plus a host driver.
- Later children, as the status-card-adr003 pack lists them: copy data for the card (`/copy` of the whole status), thread-usage
  rows (account/usage/read), server row details, limit warnings.

## Departures (outside the task's area, all small)
core/src/{threads.rs (Item::Status variant fields, transcript refresh), app.rs, app/session.rs, protocol/mod.rs, rpc.rs,
screens/chat.rs}, src/pages/session/cells/{dispatch.rs,text.rs,status/strings.rs}, src/services/backend/fake.rs,
tests/kit/{env.rs,seed.rs}, core/tests/{model.rs,status.rs,status_facts.rs,app_wiring.rs}, Cargo.lock and host/Cargo.toml
(codex-model-provider, codex-model-provider-info, codex-utils-sandbox-summary; own commit).

## sweep (left pending for a filler batch)
- chat_status_2::status_command_renders_immediately_and_updates_first_card_without_status_line
- chat_status_2::status_command_omits_thread_usage_for_unsupported_plans
- chat_status_2::status_command_stays_visible_and_updates_after_thread_usage_retry
- chat_status_2::status_command_renders_credits_and_breakdowns_without_usd_estimate
- chat_status_2::status_command_drops_stale_usd_when_updated_usage_has_only_credits
- chat_status_2::inline_status_surface_commands_are_session_only_and_atomic
(the thread-usage ones wait for the `account/usage/read` child.)
