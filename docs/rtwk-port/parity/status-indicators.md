# status-indicators: parity pack

Branch fleet/block-status, tip bcefd4f28 (code 2ed215c87 + b5340f290, merge 7de985aed), landed ea69879b9. land-check on bcefd4f28: ALL fail=0 (scenarios only the known reds error-context-window and multi-agent). Written by the orchestrator from the task_done deliverable of chunk-status-indicators.

## Features
- Footer indicators (footer/indicators.rs, words in footer/strings.rs): Plan mode with the cycle hint, goal line per status (usage in the parentheses), side-conversation label (bold opening word, key hints), viewed agent label, Vim mode (Normal accent / Insert success / Replace info). hints::fit picks the widest spelling that fits; the right context (`N% context left`) shows only while the left side fits.
- Timed flash and hint override (footer/flash.rs `Timed`, footer/lead.rs): oracle order flash > hint override > passive footer; the flash expires through a real rtwk timer per epoch; the override renders `key label` items joined by ` · `; the warning notice is hidden under override, flash, popup, history search. Debug entry composer/alone_flash.
- Status items (status_surfaces.rs, widgets/status_line.rs): git branch, pull-request number with link, branch changes, workspace headline, raw output, model display name, context remaining/counted, loading model, title tier raw name.
- Alt+R raw output is the oracle's silent toggle (chat::Intent::RawOutput(bool)); /raw stays the notifying SessionIntent::Raw. tests/pages/cells.rs (outside the area, two tests) follows: no 'Raw output mode' notice after Alt+R.
- tests/pages/status_data.rs: status line of a fresh session reads `Context 100% left` / `0% used` as the oracle's context items do.

## Tests (oracle shards pass / n/a / pending / red)
pane_footer_1 27/0/28/0, pane_footer_2 18/11/45/0, chat_status_1 5/6/47/0, chat_status_2 15/1/43/0, chat_popups_1 9/5/12/0, composer_1 29/1/19/1.
Red (not this task's): composer_1::deleting_prefix_keeps_bang_literal_on_submission — in the live harness the user cell is not in the transcript after Enter (only `Working` shows); test body, kit and shard lines unchanged by this task; cause in the fake backend / transcript append path. The oracle profile is not part of land-check.

## Gap rows
- Facts feed: FactEv::Cwd/Clock/SideLabel/AgentLabel from host/app and the git and headline Probes into Commands. Needs core/src/app.rs, core/src/app/effects.rs (Ask::GitSummary/Done), host/effects.rs, host/facts.rs. In the binary the side/agent labels, PR number, branch changes and headline appear only through fixture facts; git branch and model are live.
- Once-only invalid status item warning (status_line_invalid_items_warn_once): needs the app's on_warning history cell. Pending.
- Terminal-title goal clock: the oracle has no goal item id in its title or status setup; the only goal effect is desired_program_status Working while a goal is active. Not found, not built.
- No page producer for flash/hint override (external editor hint, cloud-switch wait, startup draft); only the mechanism and the alone_flash entry exist. composer/plain.rs keeps its own flash logic and could adopt Timed.
- Provider table is not a fact (chat_status_2::status_line_processing_tier_updates_with_selection).
- `/warnings to view` spelling with the key unbound, and the slash-popup hide of the warning notice, are not wired (ours says `f2 to view`).

## Binary (pty, mock scenarios token-usage, plan-tool, reasoning-raw on rtwk and oracle)
All six runs pass. Footer rows equal in the raw streams: status line `GPT-6-Sol default fast · <cwd>`, hint row `← for agents · ? for shortcuts`, warning row `⚠ 1 warning · f2 to view`, `Working (0s • esc to interrupt)`. Differ: the oracle's status row also carries the thread title `Mock scenario` (facts feed above).

## sweep (left pending for a filler batch)
- pane_footer_1::fullscreen_plan_indicator_keeps_the_cycle_hint_when_it_fits
- pane_footer_1::footer_layout_in_fullscreen_and_scrollback
- pane_footer_1::native_wrap_tracks_agent_label_and_content_changes_at_the_same_width
- pane_footer_1::native_status_wraps_links_within_the_host_budget
- pane_footer_1::provider_status_and_hints_use_separate_rows
- pane_footer_1::slash_popup_preserves_footer_and_selection_style
- pane_footer_1::warning_notice_styles
- pane_footer_1::light_warning_notice_clears_inherited_dimming
- pane_footer_1::warnings_yield_to_queue_controls_and_complete_navigation_hints
- pane_footer_1::effort_transition_never_replaces_a_footer_flash
- pane_footer_1::effort_transition_keeps_the_full_footer_row
- pane_footer_1::status_survives_hints_feedback_and_queue
- composer_1::footer_mode_snapshots
- chat_status_2::status_line_processing_tier_updates_with_selection
- chat_status_2::status_line_invalid_items_warn_once

Several pending reasons in these tests (no Plan fact, no warning count, no flash) are stale: the facts exist now; the sweep rewrites them as it ports each test.
