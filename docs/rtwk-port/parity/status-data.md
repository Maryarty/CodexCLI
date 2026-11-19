# status-data: parity pack

Feeds the session page's status line and footer notice from what the app
already has. Oracle: `chatwidget/status_surfaces.rs`,
`bottom_pane/chat_composer/warning_notice.rs` (+ `_tests`), snapshot
`warning_notice_styles`, `custom_model_display_name_status_line`,
`status_line_model_with_reasoning_context_remaining_footer`.

## Layers
- Host (`host/src/facts.rs`, IO): config (model, effort, approval policy, reviewer,
  `tui.status_line`, `tui.status_line_use_colors`, `startup_warnings`), cwd as shown,
  git root name, git branch; sent once as `chat::Ev::Status` and `dialogs::Ev::Retained`.
- Core (`core/src/screens/chat.rs` `StatusFacts::items`, `dialogs.rs` `retained`,
  `Intent::OpenWarnings`): item list/order, default list, texts, aliases, colours flag;
  retained warnings, dismissal on close.
- View (`src/screens/chat/home.rs`, `src/ui/status_line.rs`): reads the projection.

## Oracle test map
| oracle | port |
|---|---|
| `custom_model_display_name_in_status_line_and_fallback` | core/tests/status_data.rs (same name); the catalog display name ("GPT-5.6 Luna") is not carried: the slug shows (no model catalog in the app) |
| `status_line_model_with_reasoning_context_remaining_footer_snapshot` | core `status_line_model_with_reasoning_context_remaining_footer` (context has no value yet, left out) |
| `status_line_reasoning_effort_label`, `approval_mode_display`, `configured_status_line_items` | core tests |
| `warning_notice_styles` (80 cols), `warning_notice_respects_shortcuts_and_interactive_hints` (f2 label, plural, compact) | tests/status_data.rs `warning_notice_*` |
| `status_line_segments_can_disable_theme_colors` | tests/status_data.rs `colours_off_make_the_line_plain` (+ existing `item_style` tests) |
| F2 `open_warnings` | `f2_opens_the_retained_warnings_and_dismissal_updates_the_count`, `f2_without_warnings_opens_nothing` |

## Left out (need the session or a catalog)
context-remaining/used, five-hour/weekly limits (no rate limits are fetched yet), used tokens,
thread name/title, run state, session id, fast mode and the service tier suffix of
model-with-reasoning, permissions, profile, provider, hostname, codex version, pull request /
branch changes (git summary), workspace headline. Items without a value are not drawn, as in
the oracle. Model display names from the catalog. The git branch is read once at the start (the
oracle refreshes it after a turn). Config warnings that the server notifies later are not mapped.

## Visual deltas
None known: the notice is the oracle's row (hints left, notice right, one clear cell at the
edge, full / compact / count by the oracle's budget). Colours are the theme's tokens (warning,
dim, bold), not the oracle's computed RGB.

## Also in this task
`?` and `←` on an empty composer reach the page through the composer's `FocusNode::on_key_event`
(+ `Cx::local_sender`); the `on_changed` hack is gone; G43 is "not a gap".
