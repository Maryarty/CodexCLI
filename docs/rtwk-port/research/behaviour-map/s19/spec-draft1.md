# S19 — `/status` card: specification

**What it is.** The slash command `/status` adds a card to the chat
transcript: the session's configuration (model, directory, permissions,
account, session) and its usage (tokens, context window, rate limits,
credits, and for enterprise plans the cost of the thread). The card is
written at once from what the app knows at that moment; its rate-limit rows
(and, on some plans, its thread-usage rows) are filled in place when the
reads that `/status` starts return. In tui-rtwk it is a transcript item (a
`ListView` cell of rtwk `Column`, `Row`/`Flex`, `Text` with spans); the
data is in the model, not in the widget.

Oracle: `codex-rs/tui/src/status/` (`card.rs`, `format.rs`, `helpers.rs`,
`rate_limits.rs`, `remote_connection.rs`, `thread_usage.rs`),
`chatwidget/status_controls.rs` (`add_status_output`,
`finish_status_rate_limit_refresh`), the `SlashCommand::Status` arm of
`chatwidget/slash_dispatch.rs`, `chatwidget/thread_usage.rs`. Evidence: the
behaviour map `spec/map.csv`; a row is written `[H012]` for `S19-H012`.
About a fifth of the rows describe oracle plumbing (see 5. Not carried);
every statement below was checked in the code.

Terms: "value column" is the column where values start, two columns after
the longest label of the card. "Frozen" means captured when `/status` ran
and never changed by later config, model, account or thread changes.

## 1. API

```rust
/// Typed by the composer's slash-command table (not this slice). `/status`
/// takes no argument, is allowed while a turn runs, and is allowed when no
/// thread exists. [H005, H006]
pub enum Intent { ShowStatus { thread: ThreadId } /* , … */ }

pub struct CardId(u64);                         // newtype per id kind (R13)

/// One card, kept in the thread's transcript as a local item (not made
/// from protocol events). Everything shown is a function of this struct
/// plus the clock format and the home directory (R7).
pub struct StatusCard {
    pub id: CardId,
    pub inputs: StatusInputs,                   // frozen
    pub limits: CardLimits,                     // changes once, in place
    pub billing: CardBilling,                   // changes while a usage read is pending
}

pub struct StatusInputs {
    pub captured_at: DateTime<Local>,           // the clock as a model input (R11)
    // Model section
    pub model_display_name: String,             // the status line's model name
    pub model_slug: String,                     // raw slug, for copy targets
    pub wire_api_responses: bool,               // gates the reasoning details
    pub reasoning_effort: Option<String>,       // already resolved, see rule 4
    pub reasoning_summary: Option<String>,      // config value; None = "auto"
    pub model_provider_id: Option<String>,      // Some only when a thread exists; blank → None
    pub remote: Option<RemoteConnection>,       // None for the embedded server
    // Session section
    pub cwd: PathBuf,
    pub permissions: PermissionsInput,
    pub instruction_sources: Vec<PathUri>,      // → "Agents.md"
    pub account: Option<AccountDisplay>,        // workspace already cleared if the provider is not first-party
    pub thread_name: Option<String>,
    pub collaboration_mode: Option<String>,     // None when the mode is hidden
    pub session_id: Option<ThreadId>,
    pub forked_from: Option<ThreadId>,
    // Usage section
    pub total_usage: TokenUsage,                // session totals
    pub context: Option<ContextInfo>,           // { last_total_tokens, window }
    pub show_usage_link: bool,                  // = provider requires OpenAI auth
}

pub enum AccountDisplay {
    ChatGpt { email: Option<String>, plan: Option<String>, workspace: Option<String> },
    ApiKey,
}
pub struct RemoteConnection { pub address: String, pub version: String, pub is_local_daemon: bool }
pub struct PermissionsInput {
    pub active_profile_id: Option<String>,      // built-in ids: read-only, workspace, danger-full-access
    pub profile_is_disabled: bool,              // the profile is `Disabled` (no sandbox)
    pub sandbox_summary: String,                // output of summarize_permission_profile (backend crate, as is)
    pub approval_policy: Approval,              // its own name: "never", "untrusted", "on-request", "granular"
    pub reviewer: Reviewer,                     // User | AutoReview
    pub extra_roots: Vec<String>,               // workspace roots other than cwd, native path strings
}

pub struct CardLimits {
    pub snapshots: Rc<Vec<LimitSnapshot>>,      // the whole account cache, copied at build / finish
    pub evaluated_at: DateTime<Local>,          // "now" of the last build (R11)
    pub pending: Option<Ticket>,                // Some while the card's rate-limit read is out (R1)
}
pub struct LimitSnapshot {                      // per limit id; see 3.4
    pub limit_name: String,                     // display name, falls back to the limit id
    pub captured_at: DateTime<Local>,           // ingestion time
    pub primary: Option<LimitWindow>, pub secondary: Option<LimitWindow>,
    pub credits: Option<Credits>,               // { has_credits, unlimited, balance: Option<String> }
    pub monthly: Option<MonthlyLimit>,          // { captured_at, remaining_percent: u8, used: String, limit: String, resets_at: Option<i64> }
}
pub struct LimitWindow { pub used_percent: f64, pub minutes: Option<i64>, pub resets_at: Option<i64> /* unix s */ }

pub struct CardBilling {
    pub reserve_labels: bool,                   // thread usage is available for this card
    pub estimate: Option<ThreadUsage>,          // protocol type, as is
    pub plan: Option<PlanType>,
    pub pending: Option<Ticket>,                // usage read out for this card
}

// Pure content functions, carried (no layout):
pub fn card_rows(card: &StatusCard, env: &Env) -> Vec<Section>; // Env { clock: ClockFormat, home: PathBuf, now_tz: Local }
pub fn permissions_label(p: &PermissionsInput) -> String;
pub fn limits_state(l: &CardLimits) -> LimitsState; // Available(rows) | Stale(rows) | Unavailable | Missing
pub fn copy_text(card: &StatusCard, env: &Env) -> String;

// The widget. Ids only (R14); `impl_widget!(stateful eq …)`.
pub struct StatusCardCell { pub thread: ThreadId, pub card: CardId }
```

How it fits the app (ADR-002, ADR-003):
- `/status` is an intent, not a dialog result: `apply(Intent::ShowStatus)`
  builds the `StatusCard` from the model, appends the item, and, when
  `thread usage available` / `prefetch applies` (rules 1, 2), pushes the
  effects. The card shows at once; nothing waits for a read.
- `ThreadFx::ReadRateLimits { ticket, card }` and
  `ThreadFx::ReadThreadUsage { ticket, card }` carry their ticket (R1). The
  card's `pending` holds the ticket; `apply` takes a reply only through
  `pending.resolve(reply)`, so a reply for an unknown, replaced or drained
  ticket is dropped and changes nothing (UC-05, UC-11).
- The account's rate-limit cache is app state (`apply_app`), shared with the
  usage warnings (other slices). A card copies it when it is built and when
  its read finishes; later cache changes do not reach a card. Freshness
  ("stale") is derived from `(snapshots, evaluated_at)` in the view, never
  stored (R7).
- Thread usage (credits, cost) is a per-thread `Remote<ThreadUsage>`
  (R6) in `ThreadModel`, also read by the status line (other slice); the
  card holds its own copy in `billing`, updated by the rules in 3.6.
- `StatusCardCell` is a `ListView` item widget: it `select`s its card by id
  through the thread provider; a change of the card reruns only this cell.
  No sender, no spawn, no state in the widget. The card has no keys, no
  focus, no mouse behaviour beyond transcript selection.

## 2. Use cases

*Accept* = the oracle test or snapshot that pins it; *new* = no oracle test,
write one from the cited code. Snapshots are in `status/snapshots/` as
`codex_tui__status__tests__<name>.snap` unless written otherwise. Every
card in a case is the card item's content; the item also shows the echo
line `/status` before it (rule 14).

**UC-01 Open with a ChatGPT login.** Provider requires OpenAI auth, the
account is ChatGPT, rate-limit cache empty. `/status` → the echo and the
card appear in the same frame; Limits reads `refresh requested; run
/status again shortly.`; one `ReadRateLimits` effect with a fresh ticket
(the first ticket of the session is 0 in the oracle; the value is not
behaviour). The words "refreshing limits" never appear.
Accept: `status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth`,
`transcript_overlay_status_rate_limit_refresh` ("before"). [H001, H055]

**UC-02 The read returns.** The card of UC-01. The reply carries one
snapshot, limit `spark`, primary 300 min used 45 %, secondary 10080 min used
30 %. → the card changes in place (the transcript item re-measures; a
`next message` item below it moves down):
`spark limit:` (no value), `5h limit:  [███████████░░░░░░░░░] 55% left
(resets soon)`, `Weekly limit:  [██████████████░░░░░░] 70% left (resets
later)`; the `Limits` row is gone; the value column grew to fit
`Weekly limit`.
Accept: `transcript_overlay_remeasures_status_after_rate_limit_refresh`
(snapshot `transcript_overlay_status_rate_limit_refresh`, "after").
[H068, H059]

**UC-03 The read fails, or is not accepted.** Same card, the reply is an
error (or the refresh bookkeeping ignores it: older than an applied
request, or a changed generation). → the card finishes with the cache it
already had: no snapshot is added, `pending` clears, and Limits reads
`data not available yet` (cache empty) or the cached rows (cache not
empty; then `Warning:` rows follow UC-09). No error text.
Accept: new. [H067, H069, H070]

**UC-04 Two `/status` in a row.** Both with ChatGPT login. → two cards, two
tickets (no dedup). The first reply finishes only its own card; the other
still reads `refresh requested…`. A reply finishing a card gives that card
the whole cache (not only the reply's snapshots); the cache also serves
every later `/status`.
Accept: `status_command_overlapping_refreshes_update_matching_cells_only`,
`status_command_refresh_updates_cached_limits_for_future_status_outputs`.
[H001, H066, H068, H073]

**UC-05 The account changes while a read is out.** A card with `pending`. An
account update arrives (any account update, even to the same account). →
every pending card finishes at once with no snapshots (`data not available
yet` if the cache is empty) and its tickets are void. The late reply is
dropped: the cache is not written, no card changes.
Accept: `account_update_rejects_stale_status_rate_limit_snapshots`. [H072, H074]

**UC-06 No ChatGPT account.** (a) Provider requires OpenAI auth, no ChatGPT
account (none, or an API key): the `Visit` block and the `Token usage` row
show; for an API key an `Account:  API key configured (run codex login to
use ChatGPT)` row too; no read starts. (b) The provider does not require
OpenAI auth (e.g. a local provider): no `Visit` block, no read. In both
Limits reads `data not available yet` (or the cached rows).
Accept: `status_command_renders_immediately_without_rate_limit_refresh`,
snapshots `status_server_auth_required` (a, no account, provider
`server-openai`), `status_server_auth_not_required` (b); the API-key row:
new. [H002, H030, H036, H111, H112]

**UC-07 ChatGPT account rows.** Account `user@example.com`, plan `Pro 200`,
workspace `Example workspace (account-123)`. → `Account:  user@example.com
(Pro 200)`, `ChatGPT workspace:  Example workspace (account-123)`, and no
`Token usage` row (the context window row stays). Email only → `email`;
plan only → `plan`; neither → `ChatGPT`. A workspace is not shown when the
provider is not a first-party-auth provider (rule 7).
Accept: snapshots `status_snapshot_shows_chatgpt_plan_without_email`,
`status_snapshot_includes_credits_and_limits`;
`plan_type_display_name_remaps_display_labels`. [H030, H031, H036]

**UC-08 Narrow terminal.** The enterprise monthly-limit card at 92 columns,
then at 46; a card with a long path and session id at widths 80, 40, 25,
24, 18, 17, 12, 7. → every value wraps under the value column; words are
not split unless one is longer than the column (then it breaks); no line is
wider than the terminal; no text is lost. At 46 `Model:` reads
`gpt-5.1-codex-max` / `(reasoning none,` / `summaries auto)` and the
monthly row `68% left` / `(resets 07:08 on 7 May)` (bar dropped, reset text
wrapped). At width 0 the card has no lines. Resizing the terminal re-wraps
the same card.
Accept: snapshot `status_snapshot_includes_enterprise_monthly_credit_limit`
(92) and `…wraps_enterprise_monthly_credit_details_in_narrow_terminal` (46),
`status_snapshot_wraps_in_narrow_terminal` (70),
`status_snapshot_wraps_halfwidth_kana_in_narrow_terminal` (42),
`status_wraps_long_paths_and_session_ids_without_losing_text`.
[H007, H008, H012, H042, H044]

**UC-09 Stale limits.** Cached snapshot captured 20 minutes before the card
was built (or its monthly limit older than 15 minutes). → rows as usual,
then `Warning:  limits may be stale - start new turn to refresh.`; while the
card's read is out the text is `limits may be stale - run /status again
shortly.`. Exactly 15 minutes is not stale.
Accept: `status_snapshot_shows_stale_limits_message`,
`stale_monthly_limit_marks_fresh_rolling_snapshot_stale`. [H049–H051]

**UC-10 Nothing to show.** (a) The cache holds only snapshots with no
window, no credits row and no monthly limit → `Limits:  not available for
this account`, also while the read is out. (b) Cache empty, no read →
`Limits:  data not available yet`. (c) Cache empty, read out → UC-01 text.
Accept: `status_snapshot_shows_unavailable_limits_message`,
`status_snapshot_treats_refreshing_empty_limits_as_unavailable`,
`status_snapshot_shows_missing_limits_message`. [H052, H053, H055, H056]

**UC-11 Credits, monthly limit, other limit buckets.** Cache: bucket `codex`
(primary 5h used 45, secondary weekly used 30, credits unlimited) and a
bucket named `codex-other` with one window. → `5h limit`, `Weekly limit`,
`Credits:  Unlimited`, then `codex-other 5h limit` (one window: no heading
row). A bucket with two windows gets a heading row `codex-other limit:`
with no value and then `Usage limit`, `Secondary usage limit` (unknown
window length). An enterprise monthly limit shows `Monthly credit limit:
[██████████████░░░░░░] 68% left (resets 07:08 on 7 May)` and, under it,
`8,000 of 25,000 credits used`. Credits `balance "25"` → `25 credits`;
balance `"0"`, `""`, `"abc"` or negative → `Available`; `has_credits`
false and not unlimited → no Credits row.
Accept: `status_snapshot_shows_unlimited_credits`,
`status_snapshot_respects_unlimited_without_has_credits_flag`,
`status_snapshot_shows_positive_credits`,
`status_snapshot_shows_available_credits_without_display_balance`,
`status_snapshot_cached_limits_hide_credits_without_flag`,
`status_snapshot_includes_monthly_limit`,
`status_snapshot_includes_enterprise_monthly_credit_limit`,
`status_snapshot_uses_generic_limit_labels_for_unsupported_windows`,
`non_codex_single_limit_renders_combined_row`,
`non_codex_multi_limit_keeps_group_row`. [H041, H046, H058–H065]

**UC-12 Enterprise thread usage arrives.** Plan Business, backend auth, a
thread exists. `/status` → the card shows at once with no thread-usage row
and a `ReadThreadUsage` effect; the reply (50 credits, USD 1.82, two
breakdown groups) adds, after a blank line below the limits:
`Thread usage:  50 credits · ~$1.82`, `  Models:  GPT-5.4 80%, GPT-5 Mini
20%`, `  Reasoning:  Medium 20%, High 80%`, `  Speed:  Fast mode 80%,
Standard 20%`, `  Billed tokens:  120 input (25 cached) + 50 output`. The
rows are indented two columns under the section; the value column was wide
enough from the start (it fits `  Billed tokens`), so existing rows do not
move.
Accept: `status_command_renders_immediately_and_updates_first_card_without_status_line`
(snapshot `codex_tui__chatwidget__tests__status_command_estimated_thread_usage`),
`status_command_renders_credits_and_breakdowns_without_usd_estimate`
(`…_without_usd`), `status_command_drops_stale_usd_when_updated_usage_has_only_credits`.
[H003, H081–H091, H092]

**UC-13 Thread usage is not for this plan.** Plans Enterprise, Pro, Edu,
SelfServeBusinessUsageBased, Unknown, none; or no backend auth; or no
thread; or the feature reported disabled → no thread-usage row, no read, no
reserved width. Plans Ent26, Business, EnterpriseCbpUsageBased,
EnterpriseCbpAutomation with backend auth and a thread → read. Ent26 shows
only `Thread usage:  ~$<usd>` (native USD; no row when none).
Accept: `status_command_omits_thread_usage_for_unsupported_plans`,
`status_command_requests_thread_usage_for_remote_connection_metadata`,
`status_command_requests_thread_usage_with_backend_only_authentication`,
`pending_status_card_uses_the_corrected_plan_for_its_usage_reply`.
[H004, H079, H080, H102]

**UC-14 A usage read fails, then a second `/status`.** The first card has no
usage row after the failure. The second `/status` sends a new read; its
reply updates both cards (every card still waiting for usage).
Accept: `status_command_stays_visible_and_updates_after_thread_usage_retry`.
[H092, H096]

**UC-15 A usage reply is stale.** The account changed, or the thread
changed, or the reply is for another thread id while the card's read is out
→ the reply changes nothing (the card stays as it is).
Accept: `status_command_remains_visible_when_account_changes_during_usage_refresh`,
`status_command_remains_visible_when_thread_changes_during_usage_refresh`.
[H074, H101]

**UC-16 Billing settles after a turn.** A card shows `50 credits`. A turn
completes; settlement reads follow at 15 s, 60 s, 120 s; when credits (and
USD where shown) exceed the value before the turn, the card shows the new
value and stops updating; if they never do, after the third read the card
keeps its last value. A transient zero after a positive value keeps the
positive value (and the groups when the reply has none).
Accept: `status_history_updates_again_after_billing_settles`,
`status_history_survives_exhausted_billing_retries`,
`transient_zero_cost_preserves_fresh_credits_and_breakdowns`,
`transient_zero_credits_preserves_credits_only_estimates`. [H093, H098–H100]

**UC-17 Remote server.** Remote target `unix:///tmp/codex-home/…/control.sock`,
server `0.133.0` → first row `Server:  unix://… (v0.133.0)` (wrapped under
the value column), then a blank line. A local daemon → `Server:  Local
background server`. The embedded server → no Server row and no blank line.
With a remote connection the Model row has no `summaries …` part.
Accept: snapshots `status_snapshot_remote_server`,
`status_snapshot_local_background_server`;
`remote_connection_status_value_formats_display_value`. [H014, H028, H029]

**UC-18 Session rows.** Thread `0f0f3c13-…` forked from `e9f18a88-…`, name
`Clipboard example`, mode Default. → rows `Thread name`, `Collaboration
mode`, `Session`, `Forked from` in that order; `Forked from` only with a
session id; no thread → no `Session`, `Forked from` or `Model provider`.
Accept: snapshot `status_snapshot_includes_forked_from`,
`codex_tui__chatwidget__tests__slash_copy_whole_status`. [H016, H032–H035]

**UC-19 Model details and tokens.** Effort `high`, summary `detailed` →
`gpt-5.1-codex-max (reasoning high, summaries detailed)`. Summary `none` or
`off` → `summaries off`. No effort anywhere → `reasoning none`. Usage input
1,200 (cached 200), output 900, last turn 2,250, window 272,000 →
`Token usage:  1.9K total  (1K input + 900 output)`,
`Context window:  100% left (2.25K used / 272K)`.
Accept: `status_snapshot_includes_reasoning_details`,
`status_snapshot_uses_default_reasoning_when_config_empty`,
`status_command_uses_catalog_default_reasoning_when_config_empty`,
`status_card_token_usage_excludes_cached_tokens`,
`status_context_window_uses_last_usage`. [H014, H015, H036–H040]

**UC-20 Permissions and agents.** See the table in 3.2 (one case per
row). Instruction files `[<cwd>/AGENTS.md, C:\remote\AGENTS.md]` → `AGENTS.md,
C:\remote\AGENTS.md`; none → `<none>`.
Accept: `status_permissions_*` (13 tests), `status_snapshot_shows_auto_review_permissions`,
`status_snapshot_shows_active_user_defined_profile`,
`compose_agents_summary_*` (5 tests),
`status_command_renders_native_and_foreign_instruction_sources`.
[H017–H027]

**UC-21 Copy the card.** After `/status` and a later limits reply, the copy
picker's `Whole status` copies the card as it is then: the card lines at
unlimited width, trailing spaces trimmed per line, joined by newlines,
outer whitespace trimmed (no `/status` echo, no leading indent on the
title); it includes `8% left` of a reply that arrived after the card was
inserted. The picker's other targets are captured at `/status`: Model (raw
slug, not the display name), Directory (raw cwd), Thread name when
non-empty, Session ID when a thread exists. Only the latest card is a copy
source.
Accept: `status_command_refresh_updates_cached_limits_for_future_status_outputs`
(the `8% left` part), snapshots `slash_copy_whole_status`,
`slash_copy_picker_status_fields` (copy slice). [H105–H107]

## 3. Behaviour

The card has no keys. `/status` is entered through the composer
(slash-command slice).

### 3.1 The card, top to bottom

Everything is plain text, with no border. Lines are listed in order; a row
in brackets is conditional. Labels are dim and end with `:`; values are
default colour unless marked dim.

| # | block | shows | when |
|---|---|---|---|
| 1 | title | `>_ ` (accent) `OpenAI Codex` (bold) ` (v<version>)` (dim), indented two columns | always (indent dropped when width < 4) |
| 2 | blank | | always |
| 3 | usage link | `Visit https://chatgpt.com/settings/usage for up-to-date` / `information on rate limits and credits`, accent colour, URL underlined and a hyperlink; then a blank line | `show_usage_link` |
| 4 | server | `Server:` row, then a blank line | remote connection present (3.3) |
| 5 | configuration | `Model`, [`Model provider`], `Directory`, `Permissions`, `Agents.md`, [`Account`], [`ChatGPT workspace`], [`Thread name`], [`Collaboration mode`], [`Session`], [`Forked from`] | 3.2 |
| 6 | blank | | always |
| 7 | usage | [`Token usage`], [`Context window`], limits rows, | 3.4, 3.5 |
| 8 | thread usage | blank line, then `Thread usage` and its sub-rows | non-empty (3.6) |

The label column: the value column starts after the longest of the labels
of rows present, plus the label's `:` and two spaces. Hidden rows do not
widen it, with the exceptions in Open points 3. [H010, H011]

### 3.2 Configuration rows

| row | value | condition |
|---|---|---|
| Model | `<model_display_name>` + ` (` `reasoning <effort lowercased>`, `summaries <summary lowercased>` joined by `, ` `)` (the parenthesis dim) | parenthesis omitted when both parts are absent |
| — reasoning part | `reasoning <effort>`; effort `none` when no effort resolved | `wire_api_responses` |
| — summaries part | config value, `auto` when unset; `none`/`off` (any case) → `summaries off`; blank → omitted | `wire_api_responses` and no remote connection |
| Model provider | the provider id | a thread exists and the id is not blank |
| Directory | cwd; the home directory prefix shown as `~`; never truncated | always |
| Permissions | `permissions_label` (below) | always |
| Agents.md | instruction files joined by `, `; `<none>` when none | always (frozen) |
| Account | ChatGPT: `email (plan)`, `email`, `plan` or `ChatGPT`; API key: `API key configured (run codex login to use ChatGPT)` | an account display exists |
| ChatGPT workspace | the workspace name | ChatGPT account, with a workspace, provider is first-party-auth |
| Thread name | the name | non-empty |
| Collaboration mode | the mode label | the label exists |
| Session | the thread id | a thread exists |
| Forked from | the source thread id | a session id and a source exist |

Agents.md file names: a file in cwd → its name; in an ancestor → `../` per
level + name; a descendant → the path relative to cwd; any other native
path → the full path with `~` for home; a path of the other OS convention
(or one that cannot be made absolute) → its full native string. [H018, H019]

Permissions label (`<approval>` = the approval label below; `[roots]` =
` [r1, r2]` for workspace roots other than cwd, else empty; `sandbox` =
the summary reduced to `read-only[ with network access]`,
`workspace[ with network access]`, `custom permissions with network access`,
else as is):

| active profile | condition | label |
|---|---|---|
| `read-only` | | `Read Only (<approval>)`, or `Read Only with network access (<approval>)` |
| `workspace` | sandbox `workspace` / `workspace with network access` | `Workspace[roots] (<approval>)` / `Workspace with network access[roots] (<approval>)` |
| `danger-full-access` | profile `Disabled` | `Full Access` when approval policy is `never`, else `No Sandbox (<approval>)` |
| any other id (also `workspace` with another sandbox) | | `Profile <id> (<sandbox with roots>, <approval>)` |
| none | sandbox `read-only` | `Read Only (<approval>)` |
| none | policy on-request and sandbox `workspace` | `Workspace[roots] (<approval>)` |
| none | policy `never` and profile `Disabled` | `Full Access` |
| none | otherwise | `Custom (<sandbox with roots>, <approval>)` |

"Sandbox with roots" appends `[roots]` only when the sandbox text starts with
`workspace`. Approval label: policy on-request → `Approve for me` (reviewer
auto-review) or `Ask for approval` (user); every other policy → its own
name. [H020–H027]

### 3.3 Server row

`Local background server` for a local daemon; otherwise `<address> (<version>)`
with the parenthesis dim. Address: a websocket URL without user, password,
query and fragment (an unparseable one reads `<invalid websocket URL>`);
`unix://<path>` for a unix socket; `Cloud` for the cloud target. Version:
`v<server version>` or `unknown` (cloud: the bare version or `unknown`, see
Open points 7). The value wraps under the value column. [H028, H029]

### 3.4 Usage rows

| row | value | condition |
|---|---|---|
| Token usage | `<total> total  (<input> input + <output> output)`; the parenthesis part dim; total = non-cached input + output (cached input is not counted); input = non-cached input | account is not ChatGPT (also none, API key) |
| Context window | `<P>% left (<used> used / <window>)`, the parenthesis part dim | a window is known |

`P` = 0 when window ≤ 12,000, else round((window − 12,000 − max(0, used −
12,000)) × 100 / (window − 12,000)) clamped to 0–100, where `used` is the
last turn's total tokens. The window is the token info's window, or the
config's when there is no token info (then `used` is 0). Counts are compact:
0 → `0`; under 1,000 as is; from 1,000 `K`, `M`, `B`, `T` with two decimals
below 10, one below 100, none above, trailing zeros and a bare `.` removed
(`1.2K`, `2.25K`, `272K`, `1K`); a value that would print as 1000 of its
unit moves up a unit (999,500 → `1M`). Negative → 0. [H036–H040]

### 3.5 Limits rows

State, from `limits_state(CardLimits)`:

| state | when | rows |
|---|---|---|
| Missing | no snapshots | `Limits:` `refresh requested; run /status again shortly.` when `pending`, else `data not available yet` (both dim) |
| Unavailable | snapshots, but no rows come out | `Limits:  not available for this account` (dim) |
| Available | rows, none stale | the rows |
| Stale | rows, and a contributing snapshot or monthly limit has `evaluated_at − captured_at > 15 min` | the rows, then `Warning:` (dim) `limits may be stale - run /status again shortly.` when `pending`, else `limits may be stale - start new turn to refresh.` |

Rows are built per snapshot, snapshots with limit name `codex` (any case)
first, the others in cache order. A snapshot with no windows, no credits row
and no monthly limit is skipped (no heading, no staleness). Per snapshot, in
order:

| row | label | value |
|---|---|---|
| heading | `<name> limit` where name is the model display name of the snapshot's name | snapshot name is not `codex`, and not exactly one window; empty value |
| primary | `<Window> limit`; with the name in front (`<name> <Window> limit`) when it is non-codex with exactly one window | a primary window |
| secondary | likewise | a secondary window |
| Credits | `Credits` | `Unlimited` when unlimited (regardless of `has_credits`); else when `has_credits`: `<N> credits` (balance parses to a positive integer, or a positive finite number rounded) or `Available`; else no row |
| Monthly credit limit | `Monthly credit limit` | a window row with percent used = 100 − remaining, and the detail below |

`<Window>`: minutes within ±5 % of 300 → `5h`, 1,440 → `Daily`, 10,080 →
`Weekly`, 43,200 → `Monthly`, 525,600 → `Annual`; unknown or absent →
`Usage` (primary) / `Secondary usage` (secondary).

A window row's value: `[` + 20 cells + `]` + ` ` + `<R>% left`, where
`remaining` = clamp(100 − used, 0, 100), filled cells = round(remaining ×
20 / 100) of `█`, the rest `░`, `R` = remaining formatted with `{:.0}`.
When bar, space and summary do not fit the value column, only `<R>% left`
shows. Then the reset text `(resets <T>)` (dim): on the same line after a
space when the whole line fits the terminal width; else the value line
stands alone and the reset text follows on continuation lines at the value
column, wrapped at word boundaries, never broken inside a word. A monthly
limit's detail `<used> of <limit> credits used` (dim) follows on its own
continuation lines. No reset time → no reset text.

`<T>`: the reset instant in the local time zone, `%H:%M` (24 h clock) or
`%-I:%M %p` (12 h clock; an unknown system preference means 12 h), plus
` on %-d %b` when its date differs from the snapshot's `captured_at` date.
Credit amounts: parsed as a finite non-negative number, rounded, with
thousands separators; otherwise the monthly limit is dropped (no row, no
staleness). [H041–H065]

### 3.6 Thread usage rows

Only when the card's billing estimate exists; none while `estimate` is
`None` (no placeholder). Plan Ent26: one row `Thread usage:  <usd>` from the
native USD (no row when absent). Otherwise:

| row | value |
|---|---|
| Thread usage | `<credits> credits`, and ` · <usd>` (dim separator) when a USD estimate exists |
| `  Models` | `name pct%` joined by `, `, sorted by credits descending, then name |
| `  Reasoning` | likewise, in the order None, Minimal, Light, Medium, High, Extra High, Max, Ultra, unknown last, then by name |
| `  Speed` | likewise, in the order Fast mode, Ultrafast, Standard, unknown last, then by name |
| `  Billed tokens` | `<in> input (<cached> cached) + <out> output` |

Breakdown values: groups with negative credits are skipped; groups are
summed by name (`Other` when missing); the percentage is the half-up share
of total credits (0 when the total is not positive); a dimension with no
group is omitted. Names: reasoning `none`→None, `minimal`→Minimal,
`low`→Light, `medium`→Medium, `high`→High, `xhigh`→Extra High, `max`→Max,
`ultra`→Ultra; speed `fast`→Fast mode, `ultrafast`→Ultrafast,
`standard`→Standard; others pass through. Model names: the catalog display
name, then `gpt-x…` → `GPT-x…`; the following `-` segments are joined by a
space when the first segment starts with a digit (else `-`), capitalised,
`oai` → `OAI` (`gpt-5.4` → `GPT-5.4`, `gpt-5-mini` → `GPT-5 Mini`).
`Billed tokens`: shown when groups exist and the sum of input or of output
is present (a sum is present only when every group has the value); cached
only after input; `+` only when both exist.
Credits: micro-credits ÷ 10⁶; with a suffix `K`/`M`/`B`/`T` from 10³ / 10⁶ /
10⁹ / 10¹² credits; one decimal rounded half up, `.0` dropped (`50`, `1.5K`).
USD micros: negative → no USD; 1–99 → `~$0.` + six digits; below 10,000 →
`~$0.` + four digits rounded; else `~$D.CC` rounded to cents; 0 → `~$0.00`.
[H080–H090]

### Rules

1. **Prefetch:** `/status` starts a rate-limit read when the provider
   requires OpenAI auth and the account is ChatGPT; the card is created with
   `pending = Some(ticket)`. Otherwise no read and `pending = None`. Reads
   are never deduplicated and take no hold of the periodic or recovery
   refresh. [H001, H002, H066, H112]
2. **Usage read at creation:** when thread usage is available (plan, auth,
   thread, feature: UC-13) the card is created with `reserve_labels`, the
   thread's current estimate and plan, and a `ReadThreadUsage` effect. A card
   created while a read for the thread is already out waits for that read.
   When the plan becomes billable while a card is pending (its own rate-limit
   read reveals the plan), the card reserves and requests then. [H003, H004,
   H102]
3. **Frozen content:** configuration rows, token totals and the context
   window are captured at `/status` and never change; only limits and thread
   usage rows change (rules 5–9). The Agents.md summary is composed at
   creation. [H106]
4. **Reasoning effort:** the status override (the picked model's effort),
   else the configured effort, else the selected model's catalog default,
   else none; with a cloud model picked, the configured effort is skipped.
   [H015]
5. **Finishing a limits read:** a reply for the card's ticket (success,
   error, or not accepted) takes the account cache as it is, after adding the
   reply's snapshots when it was accepted, copies it into the card with
   `evaluated_at = now`, and clears `pending`. A failed or ignored read adds
   nothing. A reply with no card for its ticket is dropped and the cache is
   not written. Only reads started for a card (origin status command) are
   applied to cards; rolling rate-limit updates never change a card.
   [H067–H076]
6. **Account change:** drains `pending` of every card with an empty reply and
   clears the thread-usage state of the account (UC-05, UC-15). [H074]
7. **ChatGPT workspace** is shown only when the provider of the card is a
   first-party-auth provider (with a remote connection: the built-in OpenAI
   id or none); otherwise the account is shown without it.
8. **Thread usage updates:** a usage reply for the pending ticket and
   thread replaces the estimate (UC-12). A reply that fails the checks (not
   pending, other thread, other account) changes nothing. A reply naming
   another thread is logged and retried. `Disabled` clears the estimate for
   good. Cards waiting for usage are all updated by any update; a card is
   released when no settlement is pending (UC-14, UC-16). [H092–H095, H101]
9. **Retries and settlement:** a failed read is retried after 5 s, 15 s,
   60 s, then not again; a success resets the count. After a completed turn
   (not a replayed one) settlement reads run at 15 s, 60 s, 120 s. The fetch
   is cut at 65 s and counts as failed. Delays are effects with tickets in
   the shell; the clock is a model input. [H096–H100]
10. **Zero after positive:** a reply with 0 credits (or USD 0) after a
    positive value keeps the previous positive value, and the previous groups
    when the new ones are empty. [H093]
11. **Wrapping:** each line is word-wrapped to the terminal width with
    continuation lines at the value column; a line that still has a part
    wider than the terminal after that is wrapped again without the
    continuation indent. At widths too small for the label column, the
    continuation is the outer indent. [H012, H013]
12. **Hyperlinks:** with the usage link shown, the underlined URL, in every
    fragment of a wrapped line, links to `https://chatgpt.com/settings/usage`;
    without it nothing links. [H109]
13. **Copy and raw text:** `copy_text` = the card lines at unlimited width.
    Raw mode shows the same lines without hyperlinks. [H105, H108]
14. **The echo:** the item is the line `/status` (magenta) and the card, with
    one blank line between. [H008]
15. **During a turn:** `/status` is allowed while a turn runs and without a
    usable thread; with no thread there is no `Session`, `Forked from`,
    `Model provider` and no thread usage. [H005]

## 4. How it looks

The oracle snapshots are the reference at their sizes; layout comes from
rtwk and every difference is a delta in the parity pack. The bar may be
drawn with rtwk's `ProgressBar` if it paints the same 22 cells; otherwise
text.

| state | snapshot |
|---|---|
| full card, enterprise, monthly credit limit | `status__tests__status_snapshot_includes_enterprise_monthly_credit_limit` |
| the same, narrow | `…wraps_enterprise_monthly_credit_details_in_narrow_terminal` |
| reasoning details, forked, ChatGPT plan | `…includes_reasoning_details`, `…includes_forked_from`, `…shows_chatgpt_plan_without_email` |
| limits: refreshing, missing, unavailable, stale | `…shows_refreshing_limits_notice`, `…shows_missing_limits_message`, `…shows_unavailable_limits_message`, `…shows_stale_limits_message` |
| limits: credits, monthly, generic labels | `…includes_credits_and_limits`, `…includes_monthly_limit`, `…uses_generic_limit_labels_for_unsupported_windows`, `…cached_limits_hide_credits_without_flag` |
| narrow widths | `…truncates_in_narrow_terminal` (70), `…truncates_halfwidth_kana_in_narrow_terminal` |
| server rows | `…local_background_server`, `…remote_server` |
| provider, no usage link | `…status_server_auth_not_required`, `…status_server_auth_required` |
| permissions, profile | `…shows_auto_review_permissions`, `…shows_active_user_defined_profile` |
| in place after the read | `…transcript_overlay_status_rate_limit_refresh` (before / after) |
| thread usage | `chatwidget__tests__status_command_estimated_thread_usage`, `…_without_usd` |
| copy text | `chatwidget__tests__slash_copy_whole_status` |

## 5. Not carried

The history-cell classes (`CompositeHistoryCell`, `PlainHistoryCell`,
`display_lines`/`raw_lines`/hyperlink-line hooks), the shared handle
(`StatusHistoryHandle`, `Arc<RwLock<…>>`, `AtomicBool`) and the lists
`refreshing_status_outputs` and `status_history_handles` (replaced by `CardId`
and tickets in the model), `FieldFormatter` and all label-width and
value-offset arithmetic (rtwk layout; the label set is a rule only where it
is visible, Open points 3), the request-id counter and its wrapping add,
the refresh bookkeeping (`rate_limit_refresh.rs` generation, applied id,
holds; the ticket check replaces it), `on_rate_limit_snapshot(None)`
(no caller), the thread-usage state's per-thread re-keying (usage is per
thread in the model), key handling, frame requests, `request_redraw`, the
`textwrap`/`word_wrap_lines` calls (rtwk text wrapping; the result is a
delta where it differs), and the unreachable `Available` with no rows.
Map rows judged to describe these: H009, H010, H011, H013, H054, H057, H066
(as plumbing), H075, H076, H077, H078, H091, H103, H110.

## 6. Open points (the person decides: parity or fix)

1. **Dialog or transcript item.** `architecture.md` §1 lists S19 as a
   dialog; the oracle inserts a history cell that updates in place and stays
   in the scrollback, is a copy source, and is read in the transcript pager.
   This spec follows the oracle (transcript item, ids not data, update by
   ticket). A dialog would lose the history, the copy and the pager.
2. **The rate-limit cache survives an account change.** The code has no
   production caller that clears it (`on_rate_limit_snapshot(None)` has
   none), so the next card after an account switch can show the previous
   account's cached limits (marked stale after 15 minutes) until a read
   replaces them. Confirm with a test; parity or clear it on account change.
3. **Hidden labels widen the value column.** `Token usage` is in the label
   set even when the row is hidden (ChatGPT accounts; no visible effect now,
   as `Context window` is longer); for non-Ent26 plans with groups `Models`,
   `Reasoning`, `Speed` and `Billed tokens` are in the set; for Ent26 they
   are in it though no such rows show, and `Billed tokens` is reserved
   before any reply. Parity (min value-column width = the longest of these)
   or fix (measure visible rows only; the column then shifts when the reply
   arrives).
4. **A failed usage read is retried only when thread usage is "selected"**
   (a status-line or terminal-title item uses thread credits or cost, or the
   provider needs a snapshot). Otherwise the card stays without rows and only
   a later `/status` or turn tries again. Parity or always retry for a card.
5. **No timeout for the limits read.** The oracle awaits the `/status`
   request without a TUI timeout (the reset-request origins have one); if the
   server never answers, the card says `refresh requested…` for good. Parity
   or a timeout effect that finishes the card as failed.
6. **Freshness is evaluated only when the card is built and when its read
   finishes.** A card left open does not turn stale as time passes (nor does
   a reset time pass). Parity or re-evaluate on a clock tick (R11).
7. **Cloud server version** has no `v` prefix (`Cloud (0.133.0)`), the other
   targets do (`(v0.133.0)`). Parity or add the `v`.
8. **A card primed with another thread's estimate.** The oracle copies the
   chat's current estimate into the card before it checks the thread, so
   right after a thread switch the card can show the previous thread's
   thread usage until the reply arrives. Per-thread usage in the model makes
   this unreproducible; fix (recommended).
9. **Unavailable shows at once.** With a cache of only metadata snapshots
   the card says `not available for this account` even while its read is
   out (UC-10a); the result of the read may then replace it. Parity or show
   the refresh text.
10. **Hyperlink on the usage URL** needs rtwk text spans with a link
    destination across wrapped fragments; if rtwk lacks it, it is a gap and a
    delta (the URL still shows, underlined).
11. **Copy source lifetime.** A later user message, agent reply or other
    copy source clears the status copy source in the oracle (copy slice). The
    model needs `last_status_card: Option<CardId>` cleared by the same events;
    check with the copy slice.
12. **Warning wording.** While `pending`, a stale card says `run /status
    again shortly`, though `/status` is what started the refresh. Parity.
