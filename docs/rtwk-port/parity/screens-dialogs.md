# Parity pack: screens-dialogs (S13, S14, S19 done; S15, S23, S24 open)

State: PARTIAL. Done and green: S13 approvals, S14 selection, S19 status. Not
started (still the skeleton's empty slots): S15 settings (4 variants), S23
small dialogs (shortcut overlay, warning, feedback), S24 pets. A fresh agent
continues from the `task_checkpoint`.

Method of every row: the variant's fixture script (JSONL, oracle test data)
plays through core `apply` (`Sandbox::play` / the `screen` step); the rtwk
view renders the model at the oracle's terminal size; `dialogs::oracle::assert_rows`
(`codex-rs/tui-rtwk/tests/dialogs.rs`) compares the oracle `.snap` rows, in
order, trimmed, with the `›` marker dropped.

## Deltas
- N01 (NEW kind): codex-tui draws these views as frameless bottom-pane panes with a
  `›` cursor marker; tui-rtwk draws them in a `Dialog` (frame, title, style
  highlight for the cursor row). Compared: rows in order, marker stripped. Oracle
  rows are never loosened beyond that.
- N02 (NEW kind): the oracle wraps at the pane width (80); the dialog has
  `max_width` 83 (select) / 84 (approvals, status) so the wrap points equal the
  oracle's at the oracle's column count. Terminal sizes of the tests are wider
  than the oracle's (100) because the frame and the debug menu need room.
- Patch approval: the oracle snapshot is the variant where the app-server request
  carries no changes (`Destination: unavailable`); the fixture is that request. A
  patch with changes (diff preview in the header) is not covered: no oracle
  snapshot of it in the required rows.
- Seeds are the oracle's test data, not protocol JSON, for S14 and S19
  (`menu`, `card` events); S13 seeds carry the protocol's own `ServerRequest` JSON.

## S13 approvals
| Variant (debug entry) | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| exec_command | `src/screens/approvals/fixtures/exec_command.jsonl` | `chatwidget/snapshots/...approval_modal_exec.snap` | `approvals::approval_modal_exec` |
| patch_edits | `.../patch_edits.jsonl` | `...approval_overlay_patch_destination_unavailable.snap` | `approval_overlay_patch_destination_unavailable` |
| permissions | `.../permissions.jsonl` | `...approval_overlay_permissions_prompt.snap` | `permissions_prompt_snapshot` |
| mcp_elicitation | `.../mcp_elicitation.jsonl` | `bottom_pane/snapshots/...mcp_server_elicitation_approval_form_without_schema.snap` | `approval_form_tool_approval_snapshot` |
| user_input | `.../user_input.jsonl` | `...request_user_input_options.snap` | `request_user_input_options_snapshot` |
| exec_approval (skeleton) | `exec_approval.jsonl` | none (fallback) | `a_request_without_a_prompt_falls_back_to_the_dismiss_dialog` |

Oracle-test map (approval_overlay.rs, request_user_input): `shortcut_triggers_selection`,
`configured_list_cancel_aborts_exec_approval`, `enter_confirms_the_highlighted_option`,
`deny_shortcut_submits_denied_exec_decision` are ported by name in `tests/approvals.rs`.
Core state tests: `core/tests/approvals.rs` (prefix, shell wrapper stripped, default
decisions of exec/network/permissions/patch, answer is a model fact, bad seed names its line).

## S14 selection
| Variant | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| model_picker | `src/screens/select/fixtures/model_picker.jsonl` | `bottom_pane/snapshots/...list_selection_model_picker_width_80.snap` | `select::snapshot_model_picker_width_80` |
| permissions_menu | `.../permissions_menu.jsonl` | `chatwidget/snapshots/...approvals_selection_popup.snap` | `select::approvals_selection_popup_snapshot` |

Own tests (no oracle name): `enter_picks_the_highlighted_row_and_closes`,
`a_number_key_picks_its_row`, `esc_closes_without_a_pick`; core: `core/tests/select.rs`.
Not mapped (oracle `list_selection_view.rs` tests for search, scrolling, side
content, footer notes): out of scope of the two required parameterisations.

## S19 status
| Variant | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| reasoning_details | `src/screens/status/fixtures/reasoning_details.jsonl` | `status/snapshots/...status_snapshot_includes_reasoning_details.snap` | `status::status_snapshot_includes_reasoning_details` |

`format_tokens_compact` (status/helpers.rs:106) carried in `src/screens/status/format.rs`.
Own test: `esc_closes_the_card`; core: `core/tests/status.rs`.

## Edits outside the area (shared tests of the skeleton)
- `tests/navigation.rs::a_dialog_closes_on_escape` opened the empty `select` slot;
  it now opens the filled `select/model_picker`. The skeleton's slot test pattern
  ("empty slot") stays for the slots still empty.
- Debug entry descriptions are kept short: `skeleton::debug_menu_lists_the_directories`
  needs `name - description` on one 80-column row.

## Handover (rtwk needs and what others must change)
- `PendingRequest` holds no params, so approvals got an own `Ev::Presented` plus a
  `screen` seed step with the request JSON. `apply_app` (app.rs, outside this area)
  should forward server requests to approvals, and a real answer command is needed:
  today the answer is recorded in the model (`answered`) only.
- rtwk: `Dialog` has no frameless / bottom-pane mode (N01).

## Open (next agent)
S23 dialogs (shortcut overlay, warning, feedback), S15 settings (config panel,
theme picker, status line setup, keymap setup), S24 pets; each: core half, fixture
script from the oracle's test data, view with `strings.rs`, screen test with
`assert_rows`, core state test, rows here. Then fmt, clippy, full nextest, `task_done`.
