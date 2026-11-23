# chunk-composer-embedded — parity

Code: `src/pages/session/composer/plain.rs` (`Composer::plain`, `plain::ui()`, harness page `Embedded`,
debug entry `composer/plain`), `composer/mod.rs` (the plain look: no popups, no connection state, no Aura or
effort glyph), `composer/editing.rs` (`!` takes no shell mode, no image paste, Tab submits), `composer/mentions.rs`
(a plain composer closes the `@`/`$` popups), `src/widgets/composer_draft.rs` (`ComposerDraft::plain`: shell
mode and image attach are unrepresentable), `composer/strings.rs` (ADR-010: "Message too long; limit {0}
characters"). Mounted in `src/sheets/approvals/form.rs` and `user_input.rs` (the non-secret answers).

Shard counts (summary.py --run composer_4): pass 20 -> 24. The four targets are green:
`blockquote_paste_embedded_answer_starts_next_block`, `blockquote_paste_embedded_answer_rendered`,
`plain_text_composer_keeps_command_prefixes_literal`, `embedded_composer_shows_validation_error_without_losing_draft`.
Other shards unchanged (approvals_1/2, chat_approvals_1/2, composer_1/2/3, composer_popups, textarea,
small_dialogs_1/2). screens+pages+shell: 989 passed. rules 66, clippy and fmt clean.

Decisions:
- The flash is drawn by the harness page `plain::Embedded` (a red indented line for 5 s over the footer's row),
  because `Footer` has no flash variant. When chunk-status-indicators' footer flash lands, the harness
  uses it (Need, not blocking).
- A secret answer (`user_input.rs`) still uses `TextField` with `EditingKeys`: `Composer::plain` has no mask.
- rtwk's `TextArea` has no wheel scrolling of its own (it follows the caret), so the plain composer has no
  wheel test; the oracle's composer has none either for a one-screen answer.
- Outside the original area: `tests/pages.rs` (one `mod composer_plain;` line).

Binary: own pty driver beside the oracle (the mock `request-user-input` scenario; a throwaway copy of
`check.py` that presses Tab at the question, types `!ls /di ? @src $sk`, pastes `> quoted\nline two`, Enter;
a minimal VT emulator over the captured stream). Screens at notes-open, notes-typed and notes-pasted are
EQUAL between `codex-rtwk` and the oracle: `› Add notes`, then `› !ls /di ? @src $sk` (every prefix literal, no
popup, no shortcut row), the pasted lines under it, and the footer `tab or esc to clear notes | enter to
submit answer`. Both binaries answer the question on Enter (2 model requests). land-check: all fail=0,
scenarios 39/41 (known reds error-context-window, multi-agent).

Rows that differ: none in the sheet. The `composer/plain` debug page is not in the oracle (its harness is
`ChatComposerConfig::plain_text()` under the oracle's composer tests).

Mouse: `tests/pages/composer_plain.rs`: a click puts the caret, a drag selects and typing replaces the selection,
a double click selects the word.
Tests: `tests/pages/composer_plain.rs` (10), `composer/editing.rs` unit tests (2).

Rows for the lander: none (no shard file changed beyond `composer_4.rs`).
