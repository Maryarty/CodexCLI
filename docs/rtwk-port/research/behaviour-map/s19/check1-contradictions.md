# S19 spec.md vs the oracle: contradictions

Checked: every statement of use cases, 3.x tables, rules, API comments, 4 and 6,
against `codex-rs/tui/src/` (paths below relative to it). Order: severity.
"spec Ln" = line of spec.md. 14 contradictions (C-01..C-14), 3 wrong or
misleading acceptance/snapshot references (A-01..A-03).

## Defects (the porter would build the wrong thing)

**C-01 The rate-limit cache IS cleared on an account change** (spec L656-660,
Open point 2; also Rule 6 L574-575 and UC-05 L185-190).
Spec: "The code has no production caller that clears it ... the next card after
an account switch can show the previous account's cached limits". Rule 6 lists
only "drains `pending` ... and clears the thread-usage state".
Code: `chatwidget/settings.rs:267` (`update_account_state`) calls
`clear_pending_rate_limit_reset_requests()`, which does
`self.rate_limit_snapshots_by_limit_id.clear()` (`chatwidget/usage.rs:522`);
the drain of pending cards follows at `settings.rs:274-277`. The test
`account_update_rejects_stale_status_rate_limit_snapshots`
(`chatwidget/tests/status_command_tests.rs:192-212`) asserts exactly that the
cache is empty afterwards. The only `on_rate_limit_snapshot(None)` callers are
tests (`usage_notice_tests.rs:310,415`), so the spec's reasoning is right but its
premise (no other clearing path) is wrong. Open point 2 must go; Rule 6 and UC-05
must say "clears the cache". Also UC-05's "(`data not available yet` if the cache
is empty)": the drained card is composed from the empty slice
(`handle.finish_rate_limit_refresh(&[], now)`, `settings.rs:276-278`), not from the
cache: always `Missing` = `data not available yet`, never cached rows.

**C-02 Built-in permission profile ids carry a colon** (spec L77, table L424-433).
Spec: "built-in ids: read-only, workspace, danger-full-access"; table rows keyed
`read-only`, `workspace`, `danger-full-access`.
Code: `protocol/src/models.rs:414-420`: `":read-only"`, `":workspace"`,
`":danger-full-access"`; `status/card.rs:676,684,699` match those constants. A
user profile named `workspace` is a different profile and shows as
`Profile workspace (...)`. Also the table row "any other id (also `workspace` with
another sandbox)" omits `:danger-full-access` with a non-`Disabled` profile: the
guard at `card.rs:699-701` fails and the `Some(id)` arm (`card.rs:708`) gives
`Profile :danger-full-access (<sandbox>, <approval>)`.

**C-03 UC-16 / Rule 9: settlement is conditional, and only waiting cards update**
(spec L308-317, L585-589).
Spec: "A card shows `50 credits`. A turn completes; settlement reads follow at 15 s,
60 s, 120 s; ... the card shows the new value".
Code: `refresh_thread_usage_after_turn` returns unless
`thread_usage_is_selected()` (`chatwidget/thread_usage.rs:98-102`; selected = a
configured status-line or terminal-title item `thread-credits` /
`estimated-thread-cost`, or `provider_needs_snapshot()`, lines 288-311); the due
re-read also cancels itself when not selected (`thread_usage.rs:151-154`). Cards
are updated only while in `status_history_handles`; they leave the list as soon as
no settlement baseline is pending (`thread_usage.rs:322-330`). A card that already
received its reply is not updated by a later turn. The oracle tests set
`local_settings.tui.status_line = ["estimated-thread-cost"]` and create the card
while a settlement is in flight (`thread_usage_tests.rs:239-345`). Neither condition
is in the spec; "Open points" 4 mentions "selected" for retries only.

**C-04 Reasoning-effort source** (spec L52, L562-565, Rule 4).
Spec: "the status override (the picked model's effort), else the configured
effort, else the catalog default".
Code: `status_controls.rs:231-236`: `status_reasoning_effort()` (`status_surfaces.rs:985-990`)
= the cloud picker selection's effort, else `effective_reasoning_effort()`
(`settings.rs:485-494`) = the active collaboration-mode mask's effort, else the
current collaboration mode's effort. It is not "the picked model's effort" outside
the cloud picker. Then `config.model_reasoning_effort` (skipped with a cloud
pick, `status_controls.rs:224-229`), then the catalog default of `selected_model()`.

**C-05 Model names in the breakdown / limit headings are not a catalog lookup**
(spec L531, L483).
Spec 3.6: "Model names: the catalog display name, then `gpt-x…` -> `GPT-x…`".
Code: `status/thread_usage.rs:288` calls the free function
`model_catalog::model_display_name` (`model_catalog.rs:18-24`): only
`gpt-reserve` -> `Luna Reserve`, everything else unchanged. The preset lookup
`ModelCatalog::display_name` (`model_catalog.rs:69-75`) is used only for the card's
Model row (via `status_model_display_name`). Same free function names the limit
buckets (`rate_limits.rs:254`). The `gpt-5.4`/`gpt-5-mini` examples are right.

## Wrong in detail

**C-06 `Disabled` does not clear "for good"** (spec L583, Rule 8).
Code: `feature_disabled` is a field of `ThreadUsageState`
(`thread_usage.rs:44`); `clear_thread_usage_state()` (`thread_usage.rs:75-77`)
resets it on a thread change (`thread_usage.rs:90-96`, `161-167`), an account
change (`settings.rs:282`) and a plan that is not billable (`thread_usage.rs:66-67`).
Also: `Disabled` calls `set_thread_usage(None)` on every waiting card
(`thread_usage.rs:217-226`, `322-325`), so the thread-usage rows of those cards
vanish; the spec does not say so.

**C-07 Raw mode drops more than hyperlinks** (spec L601-602, Rule 13).
Spec: "Raw mode shows the same lines without hyperlinks".
Code: `raw_lines` = `plain_lines(display_lines(u16::MAX))` (`status/card.rs:975-977`);
`plain_lines` (`history_cell/mod.rs:187-198`) rebuilds each line from the span
text: no dim, no colour, no underline, no wrapping (unlimited width).

**C-08 UC-05 / Rule 5 "not accepted"** is a mechanism the spec itself removes
(spec L166-173, L566-567 vs L640-642). Rule 5 and UC-03 list "the refresh
bookkeeping ignores it (older than an applied request, a changed generation)" as
a case; section 5 says the bookkeeping is not carried ("the ticket check replaces
it"). In the port nothing produces "not accepted"; either drop the case or keep the
bookkeeping in the spec. (Code: `app/event_dispatch.rs:2240-2262`,
`app/rate_limit_refresh.rs:96-130`.)

**C-09 Section 5 lists rows as plumbing that the spec cites as rules**
(spec L646-647 vs L393, L597, L550, L573, L661-668). H010 and H011 are cited at L393,
H013 at L597 (Rule 11), H066 at L550 and UC-04, H075/H076 under Rule 5 ("rolling
updates never change a card"), H091 is the reserved width of Open point 3 and
Rule 2. A porter reading section 5 skips behaviour that sections 3 and 6 require.
Also L19 "About a fifth of the rows describe oracle plumbing": the list has 14 of
112 rows (an eighth).

**C-10 API `ShowStatus { thread: ThreadId }` vs "allowed when no thread exists"**
(spec L29-32, L605-607). A non-optional `ThreadId` cannot express "no thread".
Code: `add_status_output` reads `self.thread_id` as `Option`
(`status_controls.rs:253-255, 262-266`); with `None` there is no Session, Forked
from, Model provider and no usage read. The type must be `Option<ThreadId>` (or the
card must belong to something other than a thread). Also "[H006]" in the comment
cites the description string, which the spec never states (see gaps G-12).

**C-11 Hidden labels: wrong reason** (spec L661-663, Open point 3).
"`Token usage` ... no visible effect now, as `Context window` is longer". The
reason is that `Permissions` (11 columns, always present, `card.rs:780`) is as wide
as `Token usage` (11); `Context window` is absent when no window is known.
Conclusion (no visible effect) is right. Likewise `Limits` (6) and `Warning` (7)
never widen the column.

**C-12 "Value column ... two columns after the longest label"** (spec L22-23).
Code: `value_offset = indent(2) + longest label + ":" (1) + 2`
(`status/format.rs:25-28`); rows start with a 2-column indent
(`format.rs:63-72`, snapshots: `  Model:  ...`). L391-393 adds "the label's `:` and two
spaces" but no section says that every row (not only the title) carries the
2-column outer indent. Sub-rows `  Models` add a second 2 (label string
`"  Models"`, `thread_usage.rs:33-36`).

**C-13 Monthly limit with its own `captured_at`** (spec L95, L229-231).
UC-09 "(or its monthly limit older than 15 minutes)" and
`MonthlyLimit.captured_at`. Code: `SpendControlLimitSnapshotDisplay::from_limit`
(`rate_limits.rs:185-195`) copies the snapshot's `captured_at`; the cache only
stores snapshots built at ingestion (`chatwidget/rate_limits.rs:418-430`). In
production the two timestamps are always equal; only the unit test
`stale_monthly_limit_marks_fresh_rolling_snapshot_stale` (`status/tests.rs:64`)
builds a divergent one. The case is unreachable; the field can go. Also types:
protocol `remaining_percent: i32`, `resets_at: i64` required
(`app-server-protocol/src/protocol/v2/account.rs:877-883`), spec `u8` and
`Option<i64>`.

**C-14 UC-20 foreign path example** (spec L347-349):
`C:\remote\AGENTS.md` is the display on a Unix host only; on Windows the test
expects `/remote/AGENTS.md` (`chatwidget/tests/status_command_tests.rs:111-125`).
The statement is host-dependent; the spec says "native string" elsewhere.

## Wrong or misleading acceptance tests / snapshots

**A-01 Section 4 row "limits: refreshing"** (spec L621) lists
`…shows_refreshing_limits_notice`. Its snapshot
(`status/snapshots/codex_tui__status__tests__status_snapshot_shows_refreshing_limits_notice.snap`)
shows ordinary `5h limit` / `Weekly limit` rows: the test passes
`refreshing_rate_limits = true` with a fresh cached snapshot (`status/tests.rs:1797-1862`)
and the flag has no visible effect on `Available` rows (`card.rs:469-478`;
map H048 says the same). The snapshot that pins the refreshing text is
`transcript_overlay_status_rate_limit_refresh` ("before"), already in UC-01.
Nothing in the oracle shows a "refreshing" notice under rows.

**A-02 UC-12 / Section 4 `status_command_estimated_thread_usage`** (spec L277-278,
L628) is an excerpt: the lines from `Thread usage:` on, of the SECOND card
(created after the reply, primed from the cache) at width 90
(`chatwidget/tests/status_command_tests.rs:285-298`), not a full card and not the
first card changing in place. Update-in-place of the first card is pinned by the
asserts at L270-272 only, no snapshot. The same holds for
`…_without_usd`.

**A-03 UC-17 snapshots** `status_snapshot_remote_server` and
`status_snapshot_local_background_server` (spec L324-325) are produced inside the
loop of `status_snapshot_uses_default_reasoning_when_config_empty`
(`status/tests.rs:1727-1795`, note `version: "v0.133.0"` is passed already
prefixed); there is no separate test of those names. The spec's Accept line
names no test for them (only the snapshots), which is correct, but the test
listed in UC-19 for `uses_default_reasoning` is the same test; do not count them
as two tests.

All other cited oracle tests were grepped and exist
(`status_command_tests.rs`, `status/tests.rs`, `chatwidget/thread_usage_tests.rs`,
`status/rate_limits.rs` mod tests, `status/helpers.rs` tests,
`status/remote_connection.rs` tests); the 13 `status_permissions_*` and 5
`compose_agents_summary_*` counts are right. Snapshot files named in section 4
exist, including `…truncates_in_narrow_terminal` and
`…truncates_halfwidth_kana…` (the test fns are `…wraps_…`).
