# composer — parity (interim: first agent's handoff)

S10, the SessionPage composer. Oracle `bottom_pane/chat_composer.rs`, `textarea.rs`,
`chat_composer_history.rs`. View `src/pages/session/composer/` with these parts:
- `mod.rs`: the `Composer` widget. It is the prompt, then `TextArea` under `Autocomplete`, then `EditingKeys`.
- `editing.rs`: Codex's keys.
- `draft.rs`: `ComposerDraft`, the per-thread companion of the `TextEditingController` in `ThreadUi::draft`.
- `footer.rs`
- `strings.rs`

The core rule is `screens::composer::large_paste`.

Shard counts (test lines / pending / n/a):
- composer_1: 50 / 48 / 2
- composer_2: 50 / 22 / 5
- composer_3: 56 / 42 / 3
- composer_4: 73 / 74 / 1
- composer_popups: 43 / 19 / 1
- textarea: 59 / 58 / 3

15 oracle tests are green against base. They cover the large-paste placeholders, history recall, Ctrl+C clear, and history_search_esc_resets.

Decisions:
- Placeholders are atomic elements through the controller (EditingKeys and a controller listener), as the person decided on chat-composer 2026-11-19. They are not a gap.
- Local history entries keep their placeholders in `ComposerDraft`, which is UI state. The core history stays `Vec<String>` (ADR-003: submits still go through `run_line` / `Intent::Record`).
- Ctrl+C on a draft clears it into history (`clear_for_ctrl_c`) through `EditingKeys`.
- The textarea shard opens the debug entry `composer/textarea`, which is the bare TextArea.

Pending and why:
- vim → keymap task.
- Paste burst, `!` shell mode, blockquote paste, images, mentions/skills/plugins, mouse selection, input-size limit, external editor → not built yet.
- textarea BARE (31): at the oracle's small rects the debug menu under the entry overflows (`src/app/debug/menu.rs` title and hint wrap; outside this area). The rtwk TextArea wrap and caret rows are not yet compared.

Red on arrival (oracle profile, base): 10 composer_2 tests (tab_submits_when_no_task_running, slash_tab_completion_*, slash_init_*, kill_buffer_persists_after_slash_command_dispatch, handle_paste_small_inserts_text, mention_popup_type_prefixes_snapshot, slash_popup_model_clipping_snapshot, slash_key_completes_*, slash_mention_dispatches_*). On the default profile, status_data::* is flaky.

Unit tests:
- `composer::editing::tests`: the oracle's kill and yank state tests, plus the atomic placeholder keys. Bite shown: breaking the EOL kill and the yank turns them red.
- `composer::draft::tests`

Binary: not run yet.
Mouse: a click inside a placeholder snaps the caret to its end (test_partial_placeholder_deletion).

Rows for the lander: `src/app/debug/menu.rs` must not overflow at tiny sizes (title and hint on one clipped line each), so the textarea BARE tests can run.
