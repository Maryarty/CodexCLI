# Round 2: new contradictions, gaps and ambiguities in spec.md (paths relative to codex-rs/tui/src/)

## Defects
N-01 Rule 4 "the default model's name if empty": code shows `loading` (chatwidget.rs:172, settings.rs:656-662). `gpt-reserve` -> `Luna Reserve` (model_catalog.rs:18-24) even if not in the catalog.
N-02 UC-05 at width 46: spec says `68% left` then `(resets 07:08 on 7 May)` on the next line. Snapshot `wraps_enterprise_monthly_credit_details_in_narrow_terminal`: `(resets 07:08 on` / `7 May)` on two lines (textwrap, break_words(false), at value_width).
N-03 UC-05 Accept names tests `status_snapshot_truncates_in_narrow_terminal()` / `..._truncates_halfwidth_kana_in_narrow_terminal()`; they do not exist. Fns are `status_snapshot_wraps_in_narrow_terminal` and `..._wraps_halfwidth_kana_in_narrow_terminal`, asserting snapshots named `truncates`.
N-04 UC-11 example `Token usage:  1.9K total (1K input + 900 output)` has one space before `(`; code and snapshots have two (`total  (`, card.rs spans `"total "` + `" ("`), contradicting the spec's own 3.2. FAILS when run.
N-05 3.2 permission examples: `:read-only` -> `Read Only (<approval>)` and `:workspace` -> `Workspace[...]` are false with network access (`Read Only with network access`, `Workspace with network access`); when the sandbox summary does not match the workspace arms they fall through to `Profile ...` (card.rs:666-731).
N-06 C-08 not fixed: UC-02(a) vs section 5 (see findings_status.md).
N-07 UC-09(c) "reads follow at 15 s, 60 s, 120 s ... after the third read the card keeps its last value": code also issues an IMMEDIATE read after the turn (thread_usage.rs:96-120), so 4 reads in total (test `unchanged_thread_usage_has_bounded_settlement_refreshes`); delays are relative to the previous reply, first from the turn.
N-08 Rule 8 "a reply for another thread id is logged and retried": ticket/thread mismatch returns false silently (no log, no retry); only a payload thread-id mismatch is logged and retried, and only if selected (thread_usage.rs:202-343).
N-09 UC-01 `(resets soon)` / `(resets later)`: oracle fixtures inject preformatted strings with no unix time; spec `LimitSnapshot.resets_at: Option<i64>` cannot reproduce those snapshot texts, and the carried `compose_rate_limit_data_many` consumes preformatted `Option<String>`.
N-10 StatusInputs has no "provider is first-party" input: card needs `config.model_providers[provider_id or config.model_provider_id]`; with no thread `model_provider_id` is None and the fallback to `config.model_provider_id` (card.rs:357-376) is lost. Workspace row is dropped by this.
N-11 3.3 "named `codex` ... the others in ascending limit id": sort key is `limit_name == codex` (stable sort over BTreeMap-by-id order), headings use `model_display_name(limit_name)` (rate_limits.rs:227-366).
N-12 3.3 "a non-codex bucket with two windows gets a heading": heading unless exactly one window; a zero-window bucket with credits or monthly only also gets one.
N-13 Rule 10 raw mode: composite raw lines include the `/status` echo (history_cell/base.rs); not stated.
N-14 UC-07 Accept lists `..._includes_monthly_limit` under the monthly CREDIT limit; that test is a 30-day `Monthly limit` window.
N-15 Open point 10 (copy source lifetime): `last_status_copy_targets` is also cleared by any non-`/copy` slash command (slash_dispatch.rs:58,179,672,789), by `on_user_message_display`, compaction/review/shell, `record_agent_markdown`, `reset_copy_history`.
N-16 Rule 8 / UC-08: update at reply time uses the CURRENT plan (`set_thread_usage(estimate, self.plan_type)`); unstated though `pending_status_card_uses_the_corrected_plan_for_its_usage_reply` is cited.
N-17 Rule 2: a card that joins late on a plan change also reserves label width (thread_usage.rs:72).

## Gaps (unstated behaviour)
- Ent26: `Thread usage: <usd>` only; no row at all when native usd is None.
- Billed tokens: `X input (Y cached) + Z output`; cached sum independent of the input sum (try_fold over groups).
- Collaboration mode labels `Plan` / `Default`, condition vacuous.
- Server row wraps with hyphen splitting (`app-server-` / `control.sock`); rtwk Text does not: list as a delta.

## Carried-code hidden dependencies
`compose_agents_summary(&Config, ..)` takes `Config`; `remote_connection_status_value` needs tui type `AppServerTarget`; `compose_rate_limit_data_many` needs `chatwidget::rate_limits::{limit_label_for_window, fallback_limit_label}`, `capitalize_first`, `model_display_name`, `format_with_separators`.

## Oracle tests not mapped in UC Accept lines
`status_command_drops_stale_usd_when_updated_usage_has_only_credits`, `status_uses_server_provider_id_and_auth_requirement` (hyperlink fragments at 24/42/120), 3 tests in `status/thread_usage_tests.rs`, `estimated_cost_formats_dollars_and_sub_cent_values`, `rolling_quota_cannot_restore_an_obsolete_billing_plan`. `pending_status_card_uses_the_corrected_plan_for_its_usage_reply` snapshot is an inline string.

## Carried-function example runs (by reading code/snapshots)
Pass: format_tokens_compact 999 / 1.2K / 2.25K / 272K / 1M; format_credit_micros 50M -> 50, 1.5B -> 1.5K; format_estimated_usd_micros 1,820,000 -> `~$1.82`, 50 -> `~$0.000050`, 0 -> `~$0.00`, negative -> none; context percent; format_model_display_name `gpt-5.4` -> `GPT-5.4`, `gpt-5-mini` -> `GPT-5 Mini`; agents-summary examples. Fail: N-04 Token usage example; N-05 permission variants.

## Ambiguities (10)
1. UC-09(c) "reads follow at 15 s, 60 s, 120 s": cumulative or relative; is there an immediate read?
2. Rule 8 "a reply for another thread id is logged and retried": payload's thread id or the request's?
3. Rule 5 "No ticket -> dropped, cache not written": does it also drop the shared ingestion effects (notices, quota plan, banner)? In the oracle those already ran.
4. UC-02(a) "or the shared refresh state discards the reply" vs section 5 "'not accepted' is no case".
5. 3.3 "the port may derive it in the view (same result)" vs carried code over preformatted strings.
6. "carried as code" vs `compose_agents_summary(&Config)` and similar signatures needing adaptation.
7. UC-03 "it takes no argument": rejected, sent as a message, or args ignored? (Oracle: args ignored for non-inline commands; live behaviour not fully confirmed.)
8. 3.3 "Snapshots named `codex` ... the others in ascending limit id": name or id?
9. 3.2 "Collaboration mode ... collaboration modes enabled": condition vacuous, labels not given.
10. Rule 4 "the active collaboration mask's effort, else the current mode's": Option<Option> semantics (explicit mask None wins).
