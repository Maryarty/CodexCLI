# S19 spec.md: gaps (product behaviour in map.csv or the code that the spec neither states nor lists as not carried)

Paths relative to `codex-rs/tui/src/`. 13 gaps (G-01..G-13), ranked.

**G-01 What a `/status` rate-limit read does besides filling the card.**
`app/event_dispatch.rs:2240-2360`: the reply goes through the same ingestion as
any usage read: usage notice and banner, quota plan, spend-control flags, a
workspace-switch prompt, warning history cells (`chatwidget/rate_limits.rs:224-445`).
The spec treats the reply as card-only (UC-03, Rule 5). Either state "a /status
read also feeds the usage notices (S-other)" or list it under not carried.

**G-02 Model row derivation.** `status_controls.rs:212-250` plus
`settings.rs:640-690` `model_display_name`: preset display name from the catalog,
else the raw slug; empty model gives the default model's name; the row carries
the `(reasoning X, summaries Y)` details and the service-tier note. The spec gives
the detail pieces (Rule 4) but not where the name comes from or the empty-model case
(map rows on `compose_model_display`).

**G-03 Account row construction.** `status/card.rs:299-426`,
`subscription.rs`: the plan label table (Plus, Pro, Team, Business, Enterprise,
Edu, Free, ...), workspace shown as `name (id)`, email and workspace arriving later
(`on_account_email_loaded`) and not retro-fitted into old cards, API-key and other
auth modes giving no Account row (but a `Token usage` row for ChatGPT only).
Spec UC-07 gives three examples only.

**G-04 Collaboration mode row.** `settings.rs:640-690` `collaboration_mode_label`:
shown only when collaboration modes are enabled and the label is not the default
mode; label text. The spec names the row without its condition.

**G-05 Slash command availability and description.** `slash_dispatch.rs:542-556`,
`slash_command.rs`: `/status` is dispatched also in a side conversation and while a
task runs; the popup description string (cited as H006, never stated). Not stated
in the spec's availability rule.

**G-06 The cache outlives the widget and is keyed by limit id.**
`session_lifecycle.rs:628` copies `rate_limit_snapshots_by_limit_id` into the
replacement widget (new session, resume); a snapshot without `limit_id` is stored
under `codex`; BTreeMap order is ascending id and replaces per id; only
AccountUsage-source snapshots fill it. Spec says "cache order" and "the cache".

**G-07 Billing plan source and `Disabled` effect on existing cards.**
`chatwidget/thread_usage.rs:60-70`: billable plan = `quota_plan.or(account_plan)`;
a non-billable plan clears state and removes the rows of waiting cards (see C-06).

**G-08 Raw mode styling and width** (map rows on `raw_lines`): unstyled, unwrapped
(see C-07).

**G-09 Reset text is computed at ingestion, not at render.**
`chatwidget/rate_limits.rs:418-430`: `resets_at` is turned into text with the clock
format (12/24 h, date shown when not today) at ingestion time and kept; a card
rendered later by the same snapshot shows that text. A porter who formats at view
time changes behaviour. Spec: "resets 14:25" examples only.

**G-10 Credits rows repeat per snapshot.** `rate_limits.rs` `compose_rate_limit_data_many`:
the `Credits:` row is emitted for each snapshot carrying `credits`, each with its
own heading when more than one bucket; combined headings only when all buckets
have the same windows. The spec states heading rules but not the repetition.

**G-11 Window labels.** `chatwidget/rate_limits.rs:103-140`: `5h limit`, `Weekly
limit`, `<n>h limit`, `<n>d limit` rounding rules for odd minutes. The spec lists
the two usual labels only.

**G-12 The Status request has no timeout.** `app/background_requests.rs`: the 65 s
usage timeout does not apply to the `StatusCommand` origin; the card stays
`refreshing` until a reply or an account change. Not stated (UC-04 says "reply
or failure"; a never-arriving reply is the third case).

**G-13 History insertion.** The card is added as one history cell composed of the
`/status` echo and the card; a card added while a task is running goes to the
transcript queue like other cells (`chatwidget/status_controls.rs:269-302`). The
spec has the composite, not the ordering with a running turn.
