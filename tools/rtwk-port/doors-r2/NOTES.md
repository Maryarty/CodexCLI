# doors-r2: notes the deliverable has no room for

Item 12 (chat layout at the oracle's 32x6): the premise "the transcript list gets 0 rows" does not hold
for `transcript_view_4/sm_entry_1.jsonl`. At 32x6 the live chat gives the list 1 row (the `Worked for`
separator, bottom-anchored) and the bottom pane 5 (blank, blank, composer, blank, footer); at 32x8 the
list has 3 rows and `• bottom` shows. No code change. If an oracle test needs the cell at 32x6, the
question is the bottom pane's two blank rows over the composer (an owner decision for the layout, not a
door), or the select tests run over the Ctrl+T transcript sheet.

Item 4 (`oracle_snap::header` takes `external (80 columns)`): done in doors-r1's kit
(`a_frame_header_may_be_a_size_and_a_name_or_a_name_and_a_width`, tests/pages/doors.rs). The `kit::pending`
and its `KIT GAP` comment at tests/oracle/small_dialogs_1.rs `feedback_disclosure_matches_audience_at_normal_and_narrow_widths`
are stale: remove them (the dialogs-pane's file).

Item 17b, the pending `pane_footer_1/2` rows, by the fact each one needs. "In core" = the data exists, the
row needs a script or fixture (test side, its package), not a door.

| reason const | fact | in core now |
|---|---|---|
| MODE_REASON, QUEUE_REASON, FLASH_REASON, LINK_REASON (Plan part), PROVIDER_REASON | collaboration mode | yes: `ThreadPart::mode()` (`thread/settings/updated`), `chat::facts` `Footer`; script a `thread/settings/updated` notify. The side label: `flags.side_conversation_active` |
| AGENT_REASON | the viewed agent's label | yes: `StatusFacts.agent_label` (`Ev::AgentLabel`, `facts::agent_label`) |
| LINK_REASON (PR part) | pull request | yes: `GitSummary.pull_request` (`StatusFacts`), the host probe fills it; seed it by a `screen` step |
| WARN_REASON | retained warnings count | yes: `states.dialogs.retained` (seed kind `retained`/`warnings`); the notice is drawn in `session/slot.rs` as `N warnings · f2 to view` (the port's key), the oracle's is `/warnings to view`: a words difference to decide |
| FIND_REASON | Find footer | the UI (`pages/session/find`, tests/pages/find_footer.rs) is built: drive it by keys (`ctrl+r`/`/` in the transcript), no fact |
| EXTENSION_REASON, THUMBNAIL_REASON (extension row) | `FooterContent::extension` | no: only the oracle's tests build one; a person's decision (brief section 7) |
| REPLACE_REASON | host replaces the draft | door built: kit `Open::draft` / `Screen::draft` (set the composer text from a script, doors-r2 item 21) |
| FOCUS_REASON | terminal focus in/out | no: rtwk reports none (rtwk-gaps.md) |

Needs for the packages (what a door opens, left to them):
- recap: implement `structured_wanted` (item 1); title: `on_user_message` (2).
- rate limits: `on_rate_limits_read`, `AppFacts::backend_banner()` / `rate_limits()`, `fault_words`,
  `PublicVerb::NotifyOwner` (6, 7, 8, 9); its own `Ask::Nudge` path is removable.
- permissions: the Windows auto-pick test needs the Windows facts in the live fake (5).
- side (w2-commands): call `cx.quiet_interrupts(&thread, true)` for its thread (19); the composer placeholder
  and `select::centered` / `centered_route` (20) are in. The quick-dial confirmations are NOT Centered: the
  oracle's `confirm_quick_dial` asks for `SelectionViewParams::picker()` (the snapshots of
  `saved_quick_dials_confirm_*` are the bottom pane), so `select::quick` keeps them at the bottom; the door is
  for the sheets whose oracle view is `SelectionViewParams::confirmation()`.
- `TuiSettings` gained `transcript_owned`: struct literals need `..Default::default()` (13).
- feedback note: the text area has no paste burst (the `#[ignore]`d test in `burst_into_the_feedback_note`):
  core's `kit::paste_burst::PasteBurst` is the detector to put in front of it (21).
- item 10 (w2-status): the plan fallback is NOT in. An account read filling the limits' plan (the status model's
  `Account::ChatGpt { plan }`) contradicts the oracle test `rate_limit_usage_warnings_early_threshold_is_scoped_and_deduplicated`
  (its ACCOUNT_USAGE fact is a Plus account, the case `(None, 300, false)` wants no early warning): r1b's 9acaece01
  removed the same hunk. The `account/updated` notification's plan (`set_plan`) stays. Left to w2-status: say which
  fact the oracle's `update_account_state` has in that test, or make the test's account plan-less. The kit's request
  hold exists (`Screen::hold_request`); the `fake.rs`/`follow.rs`/`bottom.rs` hooks could not be told apart from
  the task's text.
