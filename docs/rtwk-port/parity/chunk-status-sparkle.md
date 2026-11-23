# chunk-status-sparkle — parity pack

Written at landing by the orchestrator from the porter's task_done deliverable (the porter's area did not include docs/).

## Tests
- pane_footer_1: 24 pass / 31 pending / 0 red. Of the 20 sparkle tests, 16 are green and 4 pending with reasons.
- chat_commands_3::ordinary_and_prepared_image_submissions_consume_the_sparkle_before_rendering: green for the typed pass. The oracle's second pass submits host-prepared images, which have no seed. An image attached in the composer ends the sparkle at the attachment.
- Dots: `assert_stars` compares everything except the dots cell for cell, and allows up to 1/5 of the dots to differ (150 ms tick vs the oracle's exact instants). The unit tests in footer/sparkle.rs pin 0 / 14 / 14.5 / 15 s exactly.
- Fixtures: pane_footer_1/{plain,astra_history,plain_history,astra_no_starfield,astra_no_animations}.jsonl, chat_commands_3/astra.jsonl.

## Real bug found and fixed
In Vim Normal mode, `/` on an empty draft switches to Insert mode and is typed. The oracle calls enter_vim_insert_mode before before_sparkle_key, but ours read the key in the stale Normal mode, so the `/` was swallowed: `/status` was sent as the message `status`. Fix: `vim::opens_slash` (extracted from `vim::early`, composer/vim.rs) feeds `sparkle_key` in editing.rs, and the mode is re-read after `early()`.

## Pending (bodies kept)
- input_history_search_uses_the_final_row: a focused TextArea keeps its caret on the prompt row (row 3, the oracle's is row 5). rtwk cannot hide a focused TextArea's caret, and R15 forbids an own render object for it.
- focus_startup_work_and_configuration_do_not_extend_a_started_deadline: rtwk has no terminal focus-in/out events.
- cancelling_history_search_preserves_a_typed_command_despite_attachment_previews: the popovers history event carries text only.
- fresh_draft_replacement_ends_search_and_dismisses_sparkles: nothing replaces the draft host-side while searching.

## Gap rows (rtwk)
- No terminal focus-in/out events (G-new-focus).
- No hidden-caret option on a focused TextArea.

## Differences
- /status and /diff open a trapping dialog card in ours, where the oracle adds a transcript cell.
- The oracle drew no dots against the mock server in check.py (cause not found). rtwk draws the starfield from the first paint and 0 dots after the submit.

## Gate
land-check at 9f8ad224e: green except fmt (3 files), fixed in 92b106242. clippy is clean.
