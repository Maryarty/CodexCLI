# chunk-composer-mention-seeds: parity pack

Branch fleet/block-composer (code 4706b5142, merge d20eb8c55, rustfmt 4dedee743; landing candidate dc16116a5 =
4dedee743 + tip). Written by the orchestrator from the task_done deliverable.

## Features
- The unified `@` popup and the `$` list are mounted in the chat page above the composer (`mention::slot`) and in
  the composer-alone harness (`mention::capped`: slot capped to max_height - footer - 3 draft rows; the draft is
  Expanded = oracle Min(3) composer / Max popup / Length footer).
- The `$` list draws the oracle rows: arrow rows from 3 list rows, DIM tag + description, bold/dim selection
  (BOLD|REVERSED when the terminal tone is unknown), hint `enter insert / esc close`, rows clamped 1..8.
- Mention bindings reach submit: `ThreadDraft::submission_with(.., catalog)` (tool_mentions) in run_line and the
  restore path.
- Seeds (popovers debug list, src/pages/session/popovers/fixtures): alone_skills, alone_skills_confirmed,
  alone_plugins, alone_plugin_sample, alone_plugin_opaque, alone_skills_default, alone_mentions_ten,
  alone_mention_types, alone_tasks.

## Tests
- composer shards (composer_popups, composer_1..4): 151 pass / 9 n/a / 111 pending / 1 red. Green now: five
  composer_1 `skill_popup_*` snapshots, `default_unified_mention_popup`, `long_paths_keep_distinguishing_parent_paths`,
  `selection_covers_the_row...`, `category_tags...`, `filesystem_columns...`, `scrolling_mentions_shifts_rendered_window`,
  composer_2 `mention_popup_type_prefixes`, composer_4 `plugin_mention_uses_display_name_for_opaque_id`,
  `submit_captures_recent_mention_bindings_before_clearing_textarea`,
  `reopening_identical_unified_mention_restarts_file_search`.
- The red `composer_1::deleting_prefix_keeps_bang_literal_on_submission` is not from this task.
- tests/pages/chat_live.rs `reversed` answers false for a prompt the popup pushed off screen.

## Pending, with reasons on the tests
- `plugin_mention_popup_snapshot` + three `file_popup_ignores_*`: the oracle draws them with Feature::MentionsV2
  off (legacy lists); the port is V2 only — DECISION for the person: a legacy mode or n/a.
- `plugin_at_mentions_render_with_plugin_accent`, `recalled_plugin_at_mentions_keep_plugin_accent_style`,
  `task_mention_popup_and_selected_composer_snapshots`: G-new-1 (no per-range style in the text area).
- `lowercase_expansion_preserves_match_ranking_and_highlighting`: state unreachable (U+0307 after `$` closes the popup).

## Gate
land-check at d20eb8c55: nextest rc=0, scenarios 39/41 (known reds); fmt red on this task's files only, fixed in
4dedee743.

## Binary beside the oracle (pty 100x24, mock text-100)
- `@sr`: the popup sits in the same rows as the oracle. Differs: the oracle lists Skill Creator and Skill
  Installer (server skills) above `src`, ours files only (chunk-composer-live-mentions); the oracle's banner starts
  one row lower; model line `default fast` vs `default`.
- `$sk`: the oracle lists the two server skills; ours draws no popup (no skill catalog from the host yet:
  chunk-composer-live-mentions).

## Needs
- kit: style_diffs should ignore bold/dim/italic on blank cells.
- theme: `Tokens::selection()` (oracle selection_style fill).
- Gap rows for the person: the oracle overlays the popup over an idle chat's transcript, ours grows the pane; G-new-1.
