# S19 — `/status` card: specification

**What it is.** `/status` adds a card to the chat transcript: the session's configuration (model,
directory, permissions, account, session) and usage (tokens, context window, rate limits, credits, on
some plans the cost of the thread). It is written at once from what the app knows; its rate-limit rows
and thread-usage rows are filled in place when the reads `/status` starts return. In tui-rtwk it is a
transcript item (`ListView` cell of `Column`, `Row`/`Flex`, `Text` spans) whose data is in the model.

Oracle: `codex-rs/tui/src/status/` (`card.rs`, `format.rs`, `helpers.rs`, `rate_limits.rs`,
`remote_connection.rs`, `thread_usage.rs`),
`chatwidget/{status_controls,thread_usage,rate_limits,usage}.rs`, the `SlashCommand::Status` arm of
`chatwidget/slash_dispatch.rs`. Evidence: `spec/map.csv`; `[H012]` is `S19-H012`.

**Carried as code, not re-specified** (process.md Part 1 §3 step 2; pure functions with their dependency
closure; examples are input → output): `helpers.rs` (`format_tokens_compact`,
`format_directory_display`, `compose_agents_summary`, `compose_model_display`, `format_reset_timestamp`,
`plan_type_display_name`), `rate_limits.rs` (`compose_rate_limit_data_many`,
`render_limit_progress_bar`, `format_status_limit_summary`), `thread_usage.rs` (`thread_cost_micros`,
`format_credit_micros`, `format_estimated_usd_micros`, `format_model_display_name`, `grouped_usage`),
`card.rs` (`status_permissions_label`, `status_approval_label`, `status_permission_summary`),
`remote_connection_status_value`. The renderers (`content_lines`, `rate_limit_row_lines`,
`StatusThreadUsage::lines`, `FieldFormatter`) are not carried: widgets are built from the returned rows.

## 1. API

```rust
pub enum Intent { ShowStatus { thread: Option<ThreadId> } /* … */ }  // from the composer; no args
pub struct CardId(u64);                                              // R13

/// A local item of the thread's transcript (or of the app's, with no thread).
pub struct StatusCard {
    pub id: CardId,
    pub inputs: StatusInputs,        // frozen at /status
    pub limits: CardLimits,          // changes once
    pub billing: CardBilling,        // changes while waiting for usage
}
pub struct StatusInputs {            // raw values; every shown string is derived (R7)
    pub captured_at: DateTime<Local>,                // clock as a model input (R11)
    pub model_display_name: String, pub model_slug: String, pub wire_api_responses: bool,
    pub reasoning_effort: Option<String>,            // resolved, rule 4
    pub reasoning_summary: Option<String>,           // config value; None → "auto"
    pub model_provider_id: Option<String>,           // Some only with a thread, non-blank
    pub remote: Option<RemoteConnection>,            // None = embedded server
    pub cwd: PathBuf, pub permissions: PermissionsInput, pub instruction_sources: Vec<PathUri>,
    pub account: Option<AccountDisplay>,             // ChatGpt{email,plan,workspace} | ApiKey (account slice)
    pub thread_name: Option<String>, pub collaboration_mode: Option<String>,
    pub session_id: Option<ThreadId>, pub forked_from: Option<ThreadId>,
    pub total_usage: TokenUsage, pub context: Option<ContextInfo>,
    pub show_usage_link: bool,                       // provider requires OpenAI auth
}
pub struct LimitSnapshot { /* limit id, name, primary/secondary { used_percent: f64, window_minutes: Option<i64>,
                              resets_at: Option<i64> }, credits, monthly limit { remaining_percent: i32, resets_at: i64, … },
                              captured_at: DateTime<Local> (ingestion time) */ }
pub struct CardLimits {
    pub snapshots: Rc<Vec<LimitSnapshot>>,  // the account cache as copied at build / finish
    pub evaluated_at: DateTime<Local>,      // "now" of that copy (R11)
    pub pending: Option<Ticket>,            // Some while this card's read is out (R1)
}
pub struct CardBilling { pub reserve_labels: bool, pub estimate: Option<ThreadUsage>, pub plan: Option<PlanType> }

// Effects and events (R1: a reply is read only through `pending.resolve`)
Fx::ReadRateLimits { ticket, card: CardId }     Ev::RateLimitsRead { ticket, result: Result<Vec<RateLimitSnapshot>, _> }
Fx::ReadThreadUsage { ticket }                  Ev::ThreadUsageRead { ticket, thread, result: Result<ThreadUsageOutcome, _> }

pub struct StatusCardCell { pub thread: Option<ThreadId>, pub card: CardId }   // ids only (R14); impl_widget!(stateful eq …)
pub fn status_rows(card: &StatusCard, env: &Env) -> Vec<Section>;  // Env { clock: ClockFormat, home, … }; pure, uses the carried functions
pub fn copy_text(card: &StatusCard, env: &Env) -> String;
```

- `apply(Intent::ShowStatus)` builds the card, appends it, pushes the effects of rules 1–2; nothing
  waits for a read. The account's rate-limit cache is app state (`apply_app`), shared with warnings and
  the status line, surviving a replaced chat; a card copies it at build and finish; "stale" is derived
  in the view from `(snapshots, evaluated_at)` (R7).
- Thread usage is a per-thread `Remote<ThreadUsage>` (R6) with one thread-wide pending ticket and the
  list of waiting cards (rules 8–9); the status line reads the same value (other slice).
- `StatusCardCell` `select`s its card by id. No sender, spawn or state in the widget; no keys or focus.

Flags ("ChatGPT account" means three things in the oracle): **prefetch** = `requires_openai_auth &&
has_chatgpt_account`; **Token usage row** hidden when `account` is `ChatGpt{..}` (shown for API key and
no account); **thread usage** = `has_codex_backend_auth` (rule 2). "First-party provider" =
`provider_uses_first_party_auth_path` (requires OpenAI auth; no env key, bearer token, custom auth or
AWS).

## 2. Use cases

*Accept* = oracle test or snapshot; *new* = write one from the cited code. Snapshots:
`status/snapshots/codex_tui__status__tests__<name>.snap` unless written otherwise; a bare name ending in
`()` is a test. All cards show the echo `/status` first (rule 11).

**UC-01 Open and refresh (ChatGPT login).** Prefetch applies, cache empty. `/status` → the echo and card
appear in the same frame; `Limits:  refresh requested; run /status again shortly.`; one `ReadRateLimits`
with a ticket. The reply (bucket `spark`, primary 300 min used 45 %, secondary 10080 min used 30 %) →
the card changes in place and the item below it moves: `spark limit:` (heading, no value), `5h limit:
[███████████░░░░░░░░░] 55% left (resets soon)`, `Weekly limit:  [██████████████░░░░░░] 70% left (resets
later)`; no `Limits` row; labels realign to the longer one. With a fresh cached snapshot the rows show
from the start and nothing marks the refresh. Accept:
`status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth()`,
`transcript_overlay_remeasures_status_after_rate_limit_refresh()` (snapshot
`transcript_overlay_status_rate_limit_refresh`, before / after);
`status_snapshot_shows_refreshing_limits_notice` shows ordinary rows (the flag has no visible effect on
Available rows). [H001, H048, H055, H068]

**UC-02 Refresh edge cases.** (a) The read fails, or the shared refresh state discards the reply: the
card is finished with the cache it had (no snapshot added): `data not available yet` if empty, else its
rows. (b) Two `/status`: two tickets, no dedup; each reply finishes only its own card; a finishing card
takes the whole cache. (c) An account update arrives (even to the same account): the cache is cleared,
every pending card is finished from an empty slice (always `data not available yet`, never cached rows),
their tickets are void; a late reply changes nothing and writes nothing to the cache. Accept:
`status_command_overlapping_refreshes_update_matching_cells_only()`,
`account_update_rejects_stale_status_rate_limit_snapshots()`,
`status_command_refresh_updates_cached_limits_for_future_status_outputs()`; (a) new. [H066–H074]

**UC-03 No prefetch.** (a) Provider requires OpenAI auth, no ChatGPT account (none, or API key): `Visit`
block and `Token usage` row show; API key adds `Account:  API key configured (run codex login to use
ChatGPT)`. (b) Provider without OpenAI auth: no `Visit` block. Neither starts a read; Limits reads `data
not available yet` (or cached rows). (c) No thread: no `Session`, `Forked from`, `Model provider`, no
thread usage. `/status` is available during a running turn, in a side conversation and without a thread;
it takes no argument; its description is `show current session configuration and token usage`. Accept:
`status_command_renders_immediately_without_rate_limit_refresh()`, snapshots
`status_server_auth_required` (a, no account), `status_server_auth_not_required` (b); API-key row: new.
[H002, H005, H006, H030, H036, H111, H112]

**UC-04 Account rows.** `user@example.com`, `Pro 200`, workspace `Example workspace (account-123)` →
`Account:  user@example.com (Pro 200)`, `ChatGPT workspace:  Example workspace (account-123)`, no `Token
usage`. Email only → `email`; plan only → `plan`; neither → `ChatGPT`. The workspace is dropped when the
card's provider is not first-party (with a remote connection: unless the id is absent or the built-in
`openai`). Email or workspace that load after the card exists do not reach it. The plan label table is
`plan_type_display_name`. Accept: snapshots `status_snapshot_shows_chatgpt_plan_without_email`,
`status_snapshot_includes_credits_and_limits`; `plan_type_display_name_remaps_display_labels()`;
workspace drop: new. [H030, H031, H036]

**UC-05 Narrow terminal.** The enterprise card at 92 columns, then 46; a card with a long path and
session id at widths 80 … 7 → values wrap under the value column, words break only when longer than it,
no line exceeds the width, no text is lost. At 46 the monthly row is `68% left` then `(resets 07:08 on 7
May)` on the next line (bar dropped when bar + summary do not fit). Width 0: no lines. A resize re-wraps
the same card. Accept: `status_snapshot_includes_enterprise_monthly_credit_limit` (92),
`…wraps_enterprise_monthly_credit_details_in_narrow_terminal` (46),
`status_snapshot_truncates_in_narrow_terminal()` (70), `…truncates_halfwidth_kana_in_narrow_terminal()`
(42), `status_wraps_long_paths_and_session_ids_without_losing_text()`. [H007, H008, H012, H042–H044]

**UC-06 Limits states.** Stale: a cached snapshot with `evaluated_at − captured_at > 15 min` (exactly 15
is fresh; the oldest contributing snapshot decides) → rows, then `Warning:  limits may be stale - start
new turn to refresh.` (`…run /status again shortly.` while `pending`). Unavailable: snapshots but no
rows (each has no window, no credits row, no monthly limit) → `Limits:  not available for this account`,
also while pending. Missing: `data not available yet`, or the pending text. Accept:
`status_snapshot_shows_stale_limits_message`, `status_snapshot_shows_unavailable_limits_message`,
`status_snapshot_treats_refreshing_empty_limits_as_unavailable()`,
`status_snapshot_shows_missing_limits_message`. [H049–H053, H055, H056]

**UC-07 Limits content.** Cache `codex` (5h used 45, weekly used 30, credits unlimited) and
`codex-other` (one window) → `5h limit`, `Weekly limit`, `Credits:  Unlimited`, `codex-other 5h limit`.
A non-codex bucket with two windows gets a heading row `codex-other limit:` first; unknown window length
→ `Usage limit`. Each bucket with credits gets its own `Credits` row (`25 credits`, or `Available` for
balance 0 / empty / non-numeric; none when `has_credits` is false and not unlimited). Monthly: `Monthly
credit limit:  [██████████████░░░░░░] 68% left (resets 07:08 on 7 May)` and `8,000 of 25,000 credits
used` under it. Accept: the `status_snapshot_*` credits and limits tests and snapshots
(`…shows_unlimited_credits`, `…respects_unlimited_without_has_credits_flag`, `…shows_positive_credits`,
`…shows_available_credits_without_display_balance`, `…cached_limits_hide_credits_without_flag`,
`…includes_monthly_limit`, `…uses_generic_limit_labels_for_unsupported_windows`),
`non_codex_single_limit_renders_combined_row()`, `non_codex_multi_limit_keeps_group_row()`. [H041, H046,
H058–H065]

**UC-08 Thread usage arrives.** Plan Business, backend auth, a thread. `/status` → the card shows at
once without thread-usage rows; one `ReadThreadUsage`. The reply (50 credits, USD 1.82, two groups) adds
after a blank line `Thread usage: 50 credits · ~$1.82`, `Models`, `Reasoning`, `Speed`, `Billed tokens`
rows (3.4); the value column does not move (reserved). Not offered (no row, read or reserved width):
plans Enterprise, Pro, Edu, SelfServeBusinessUsageBased, Unknown, none; no backend auth; no thread;
feature disabled. Offered: Ent26, Business, EnterpriseCbpUsageBased, EnterpriseCbpAutomation. Accept:
`status_command_renders_immediately_and_updates_first_card_without_status_line()` (snapshot
`chatwidget__tests__status_command_estimated_thread_usage` is an excerpt, from `Thread usage:` on, of
the second card, at width 90; the in-place update of the first card is pinned by asserts),
`status_command_renders_credits_and_breakdowns_without_usd_estimate()` (`…_without_usd`),
`…omits_thread_usage_for_unsupported_plans()`,
`…requests_thread_usage_for_remote_connection_metadata()`,
`…requests_thread_usage_with_backend_only_authentication()`,
`pending_status_card_uses_the_corrected_plan_for_its_usage_reply()`. [H003, H004, H079–H091]

**UC-09 Usage read fails, goes stale, settles.** (a) A failed read leaves the card without rows; a
second `/status` sends a new read and its reply updates both cards. (b) A reply after the account or
thread changed, or for a ticket no longer pending, changes nothing. (c) While a status-line or title
item uses thread credits or cost (or the provider needs a snapshot) — "selected", as in the oracle tests
that set `status_line = ["estimated-thread-cost"]` — a failed read is retried after 5 s, 15 s, 60 s;
after a completed turn reads follow at 15 s, 60 s, 120 s until credits (and USD where shown) exceed the
value before the turn; only cards still waiting are updated (one that already got its reply is not);
after the third read the card keeps its last value. (d) A reply of 0 credits / USD 0 after a positive
value keeps the positive value (and the groups if the reply has none). (e) `Disabled` removes the rows
of every waiting card. Accept: `status_command_stays_visible_and_updates_after_thread_usage_retry()`,
`…remains_visible_when_account_changes_during_usage_refresh()`, `…_when_thread_changes_…()`,
`temporary_thread_usage_failures_have_bounded_retries()`,
`status_history_updates_again_after_billing_settles()`,
`status_history_survives_exhausted_billing_retries()`,
`transient_zero_cost_preserves_fresh_credits_and_breakdowns()`,
`transient_zero_credits_preserves_credits_only_estimates()`; (e) new. [H092–H101]

**UC-10 Remote server.** Remote unix target, server `0.133.0` → first row `Server:
unix://…/control.sock (v0.133.0)`, wrapped under the value column, then a blank line. Local daemon →
`Server:  Local background server`. Embedded: no row, no blank line. With a remote connection the Model
row has no `summaries …`. Accept: snapshots `status_snapshot_remote_server`, `…local_background_server`
(both made by the loop inside `status_snapshot_uses_default_reasoning_when_config_empty()`, version
passed already prefixed `v0.133.0`; not separate tests),
`remote_connection_status_value_formats_display_value()`. [H014, H028, H029]

**UC-11 Model, session, tokens.** Effort `high`, summary `detailed` → `gpt-5.1-codex-max (reasoning
high, summaries detailed)`; summary `none`/`off` → `summaries off`; no effort → `reasoning none`. A
forked, named thread → rows in the order `Thread name`, `Collaboration mode`, `Session`, `Forked from`.
Usage input 1,200 (cached 200), output 900, last turn 2,250, window 272,000 → `Token usage:  1.9K total
(1K input + 900 output)`, `Context window:  100% left (2.25K used / 272K)`; with no token info the
window comes from config, `used` is 0. Accept: `status_snapshot_includes_reasoning_details`,
`…uses_default_reasoning_when_config_empty()`,
`status_command_uses_catalog_default_reasoning_when_config_empty()`,
`status_card_token_usage_excludes_cached_tokens()`, `status_context_window_uses_last_usage()`,
`status_snapshot_includes_forked_from`. [H014, H015, H032–H040]

**UC-12 Permissions and agents.** One case per label of `status_permissions_label` (3.2). Instruction
files `[<cwd>/AGENTS.md, <other-OS path>]` → `AGENTS.md, <other-OS path as its native string>` (the test
path is host-dependent: `C:\remote\AGENTS.md` on Unix hosts, `/remote/AGENTS.md` on Windows); none →
`<none>`. Accept: the 13 `status_permissions_*()`, `status_snapshot_shows_auto_review_permissions`,
`…shows_active_user_defined_profile`, the 5 `compose_agents_summary_*()`,
`status_command_renders_native_and_foreign_instruction_sources()`. [H017–H027]

**UC-13 Copy.** After `/status` and a later limits reply, `Whole status` copies the card as it is then:
lines at unlimited width, trailing spaces trimmed, joined by newlines, outer whitespace trimmed (no
echo), including a reply that came after insertion. Other targets, captured at `/status`: Model (raw
slug), Directory (raw cwd), Thread name when non-empty, Session ID when a thread exists. Only the latest
card is the source. Accept: `status_command_refresh_updates_cached_limits_for_future_status_outputs()`,
snapshots `chatwidget__tests__slash_copy_whole_status`, `…slash_copy_picker_status_fields` (copy slice).
[H105–H107]

## 3. Behaviour

No keys; `/status` comes from the composer.

### 3.1 The card

Plain text, no border. Every row carries a 2-column outer indent; labels dim, aligned in one column;
values start two columns after the longest present label's colon. Sub-rows of thread usage have a label
prefix of two more columns.

| block | content | when |
|---|---|---|
| title | `>_ ` (accent) `OpenAI Codex` (bold) ` (v<version>)` (dim); blank line | always |
| usage link | `Visit https://chatgpt.com/settings/usage for up-to-date` / `information on rate limits and credits` (accent, URL underlined and a hyperlink on every wrapped fragment); blank | `show_usage_link` |
| server | `Server` row; blank | remote connection (UC-10) |
| configuration | `Model`, `Model provider`, `Directory`, `Permissions`, `Agents.md`, `Account`, `ChatGPT workspace`, `Thread name`, `Collaboration mode`, `Session`, `Forked from` | each as in 3.2 |
| usage | blank; `Token usage`; `Context window`; limits rows (3.3) | each as in 3.2/3.3 |
| thread usage | blank; `Thread usage` + sub-rows | the estimate yields rows (3.4) |

### 3.2 Rows

| row | value | shown when |
|---|---|---|
| Model | `compose_model_display`: name + dim ` (reasoning <effort>, summaries <s>)` | always; reasoning part when `wire_api_responses`; summaries part also needs no remote connection |
| Model provider | provider id | thread exists, id not blank |
| Directory | cwd, home as `~`, never truncated (`format_directory_display`) | always |
| Permissions | `status_permissions_label` | always |
| Agents.md | `compose_agents_summary` (`AGENTS.md`, `../AGENTS.md`, `nested/AGENTS.md`, `<none>`) | always |
| Account | `email (plan)` / `email` / `plan` / `ChatGPT`; API key text (UC-03) | an account display exists |
| Thread name | the name | non-empty |
| Collaboration mode | label | collaboration modes enabled and the active mode is TUI-visible |
| Session / Forked from | thread id / source id | a thread; `Forked from` also needs a source |
| Token usage | `<total> total  (<input> input + <output> output)`; total = non-cached input + output; input = non-cached | account is not `ChatGpt` |
| Context window | `<P>% left (<used> used / <window>)`; `used` = last turn's total tokens | a window is known |

Examples of carried functions. `format_tokens_compact`: 999→`999`, 1,200→`1.2K`, 2,250→`2.25K`,
272,000→`272K`, 999,500→`1M`. Context `P`: window ≤ 12,000 → 0; else the share left of (window − 12,000)
after subtracting max(0, used − 12,000), rounded. Permission labels (built-in profile ids carry a
colon): `:read-only` → `Read Only (<approval>)`; `:workspace` → `Workspace[ [extra roots]] (<approval>)`
(a user profile named `workspace`, no colon, is `Profile workspace (…)`); `:danger-full-access` with a
`Disabled` profile → `Full Access` (policy `never`) or `No Sandbox (<approval>)`, without `Disabled` →
`Profile :danger-full-access (<sandbox>, <approval>)`; no profile → `Read Only` / `Workspace` / `Full
Access` / `Custom (…)`. Approval: on-request → `Approve for me` (auto-review) or `Ask for approval`;
other policies print their name.

### 3.3 Limits rows

`compose_rate_limit_data_many` over `snapshots` at `evaluated_at` returns Available(rows) | Stale(rows)
| Unavailable | Missing, shown per UC-06. Snapshots named `codex` (any case) come first, the others in
ascending limit id. A row is a text row (`Credits`, a heading) or a window row: `[` + 20 cells + `]`
(`█` = round(remaining × 20/100), rest `░`) + ` <R>% left`, remaining = clamp(100 − used, 0, 100), then
dim `(resets <T>)` if the line fits, else on continuation lines. Window label: minutes within ±5 % of
300 / 1,440 / 10,080 / 43,200 / 525,600 → `5h`, `Daily`, `Weekly`, `Monthly`, `Annual`; else `Usage` /
`Secondary usage`. `<T>` = reset instant in the local zone, 12 h (`7:08 AM`, default when the system is
unknown) or 24 h (`07:08`), plus ` on 7 May` when its date differs from the snapshot's `captured_at`;
the oracle formats it once at ingestion, the port may derive it in the view from the unix time,
`captured_at` and the session's clock format (same result). Examples: used 45 → `55% left`, 11 cells;
balance `"25"` → `25 credits`, `"0"` → `Available`; used `8000`, limit `25000` → `8,000 of 25,000
credits used`; an unparseable or negative amount drops the whole monthly limit.

### 3.4 Thread usage rows

Ent26 → `Thread usage:  <usd>` only. Otherwise `Thread usage:  <credits> credits[ · <usd>]`
(`format_credit_micros`: 50,000,000 → `50`, 1,500,000,000 → `1.5K`; `format_estimated_usd_micros`:
1,820,000 → `~$1.82`, 50 → `~$0.000050`, 0 → `~$0.00`, negative → none), then `Models`, `Reasoning`,
`Speed` (`name pct%` joined by `, `; groups with negative credits skipped; model names through the free
function `model_display_name` (only `gpt-reserve` → `Luna Reserve`; not the preset lookup of the Model
row) then `format_model_display_name`: `gpt-5.4` → `GPT-5.4`, `gpt-5-mini` → `GPT-5 Mini`), then `Billed
tokens` = `<in> input (<cached> cached) + <out> output`, summed over all groups including
negative-credit ones, shown when groups exist and the input or output sum exists (a sum exists only if
every group has the value).

### Rules

1. **Prefetch:** when `requires_openai_auth && has_chatgpt_account` the card is created with `pending =
   Some(ticket)` and `ReadRateLimits`; otherwise no read. Reads are never deduplicated. [H001, H002,
   H066]
2. **Usage at creation:** thread usage is offered when `has_codex_backend_auth`, the feature is not
   disabled, a thread exists and the billing plan (`quota_plan` else `account_plan`) is Ent26, Business,
   EnterpriseCbpUsageBased or EnterpriseCbpAutomation. Then the card gets `reserve_labels`, the thread's
   current estimate and plan, and joins the waiting list (one thread-wide read; a card created while one
   is out waits for it). When the plan becomes billable while a card is pending, it joins then; when it
   stops being billable the thread state is cleared. [H003, H004, H102]
3. **Frozen:** all rows but the limits (rule 5) and thread-usage rows (rules 8–9) are captured at
   `/status`. [H106]
4. **Reasoning effort:** the cloud picker's effort if a cloud model is picked, else the effective effort
   (the active collaboration mask's effort, else the current mode's); else the configured effort
   (skipped under a cloud pick); else the picked model's catalog default; else none. The Model name is
   the catalog display name of the selected model, the raw slug if not in the catalog, the default
   model's name if empty. [H015]
5. **Finishing a read:** the card's reply (success, error, or discarded) adds its snapshots to the cache
   when applied, copies the cache into the card with `evaluated_at = now`, clears `pending`. No ticket →
   dropped, cache not written. The reply also feeds the usage notices, quota plan and warnings (shared
   ingestion, other slices); rolling rate-limit updates never change a card. The cache holds only
   account-usage snapshots, keyed by limit id (absent id = `codex`), replaced per id, iterated in
   ascending id; a snapshot's `captured_at` is its ingestion time. [H067–H076]
6. **Account update:** clears the cache, finishes pending cards from an empty slice, clears the
   thread-usage state. [H074]
7. **No timeout** on the limits read (Open points).
8. **Usage reply** for the pending ticket and the current thread replaces the estimate and updates every
   waiting card; a reply for another thread id is logged and retried, not applied; `Disabled` sets the
   thread's flag (reset by a thread, account or plan change), clears the estimate and the rows of every
   waiting card. [H092–H095, H101]
9. **Retry and settlement** as UC-09(c); delays are effects with tickets; a fetch is cut at 65 s and
   counts as failed. [H096–H100]
10. **Raw mode** (`/raw`) is the card at unlimited width as plain text: no style
    (dim, colour, underline), no wrapping, no hyperlinks. [H108, H109]
11. **Echo:** the item is `/status` (magenta), a blank line, then the card; its
    place in the transcript behind a running turn's queue is the transcript
    slice's. [H008]

## 4. How it looks

The oracle snapshots named in the use cases are the reference at their sizes; layout comes from rtwk and
each difference is a delta in the parity pack. The bar may use rtwk's `ProgressBar` if it paints the
same 22 cells. Snapshots by state: UC-01 (refresh, before), UC-03 (provider), UC-04 (account), UC-05
(wrapping), UC-06 (limit states), UC-07 (credits, monthly, labels), UC-08 (thread usage), UC-10
(server), UC-11 (reasoning, forked), UC-12 (permissions), UC-13 (copy).

## 5. Not carried

History-cell classes, the shared handle and `Arc<RwLock>`, `refreshing_status_outputs` /
`status_history_handles` (replaced by `CardId`, tickets and the waiting list), `FieldFormatter` and all
width and wrapping arithmetic, the request-id counter, the refresh bookkeeping of
`rate_limit_refresh.rs` (shared slice; the ticket replaces it, so a reply "not accepted" is no case),
`on_rate_limit_snapshot(None)` (tests only), per-thread re-keying of usage state, frame requests, the
unreachable `Available` with no rows, the monthly limit's own `captured_at`. Rows judged noise: H009,
H054, H077, H078, H103, H110.

## 6. Open points (parity or fix)

1. **Dialog or transcript item.** architecture.md §1 lists S19 as a dialog; the oracle inserts a history
   cell that stays in the scrollback, updates in place, is a copy source and shows in the pager. This
   spec follows the oracle.
2. **Hidden labels widen the value column** (`Models`, `Reasoning`, `Speed` for Ent26 although never
   shown; `Billed tokens` reserved before any reply). `Token usage` hidden for ChatGPT does not matter
   (`Permissions` is as wide; `Limits` and `Warning` never widen). Parity (min column width) or measure
   visible rows only.
3. **Retry and settlement only when thread usage is "selected"**, so a card without a status-line or
   title item never retries (UC-09c). Parity or always.
4. **No timeout** for the limits read; the card can say `refresh requested…` for good. Parity or a
   timeout effect.
5. **Freshness is evaluated only at build and finish**; a card left open never turns stale. Parity or a
   clock tick (R11).
6. **Cloud server version has no `v`** (`Cloud (0.133.0)`; others `v0.133.0`).
7. **A card is primed with the previous thread's estimate** right after a thread switch (the oracle
   copies it before checking the thread); unreproducible with per-thread usage: fix.
8. **Unavailable shows at once** while pending when the cache has only metadata snapshots. Parity or
   show the pending text.
9. **Usage-URL hyperlink** across wrapped fragments may be an rtwk gap (delta).
10. **Copy source lifetime:** a later user message, agent reply or other copy
    source clears it in the oracle; the model needs `last_status_card:
    Option<CardId>` cleared by the same events (copy slice).
11. **`Disabled` is not for good:** the flag resets on thread, account or plan
    change (rule 8), though a card that lost its rows does not get them back.

Check findings not followed. G-10 ("combined headings only when all buckets have the same windows"):
`compose_rate_limit_data_many` emits a heading for each non-codex snapshot unless it has exactly one
window; the repeated `Credits` row per snapshot is stated (UC-07). G-03 ("an API-key account gives no
Account row"): `card.rs` shows `API key configured (run codex login to use ChatGPT)` (UC-03); the later
arrival of email or workspace is stated (UC-04).
