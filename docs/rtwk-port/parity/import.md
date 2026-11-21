# import parity

Slice S06: `/import` from other agents (oracle `codex-rs/tui/src/external_agent_config_migration/`
`{mod,flow,model,render,source}.rs`, 8 snapshots; the two `_windows` ones are `cfg(windows)`).
It is a step of the startup page (`onboarding::Step::Import` renders `import::Screen`), not a page.

## Where the code is

| layer | files |
|---|---|
| core (no rtwk, no words) | `core/src/screens/import/mod.rs` (Model, Phase, Intent, Ev, Request, Outcome, Failure, Started, Finished, `apply`, `seed`), `detect.rs` (Source, Detection), `item.rs` (typed projections of the items: Kind, GroupKind, Sel methods, Places) |
| view | `src/pages/startup/import/mod.rs` (the step: `Screen`, the review on rtwk `Focus` nodes and a `ListController`), `source.rs` (the source prompt: `SourcePrompt` over the shared `Options`, the startup prompts' look), `rows.rs` (the review's lines), `labels.rs` (labels, details, descriptions, failures), `notices.rs` (the history lines of a started and a finished import), `strings.rs` (words, English `EN`), `fixtures/*.jsonl` (mock data through core `apply`) |
| generic sheet | multi-select in `core/src/screens/select{,/list}.rs` and `src/sheets/select/` |
| services | `src/services/backend/import.rs` (the runner of the flow's RPCs, shared by the fake and the host), `src/services/backend/fake.rs` (the `Command::Import` arm), `host/src/import.rs` + one arm in `host/src/in_process.rs` (the real server, the places of the paths) |
| tests | `core/tests/import.rs` (flow), `tests/pages/import.rs` (words, keys, `mod flow`: the flow through the shell on the scripted backend, the focus walk), `tests/oracle/import.rs` (the oracle's screen tests, shard `import`), `tests/rtwk_g46.rs` (probe) |

Layers: core holds the flow as typed data (`Phase`, `Outcome::{Started, NoItems, Cancelled,
Failed(Failure)}`, `Group { kind }`, `Finished { by_type }`, `Model::error` is the server's raw
text); every word (group and item labels, nouns, detail lines, the notices of the history, the
errors, the source names) is told in the view from that data (ADR-010). The review's navigation
(summary or checklist, the cursor, the highlighted action) is the view's own state (widget
state of `Screen`; focus is two `FocusNode`s, the checklist and the actions, with `KeyBindings`,
the cursor of each region a `SelectionController<usize>`, the checklist's scroll a `ListController`:
the controllers are the only holders of cursor and scroll
layers), never a field of the model; `Model::review_id` counts the reviews opened so the state
is for the review it opened on. ADR-003: one model, changed only by `apply`; the view emits intents
(`Open`, `Source(id)`, `Toggle(index)`, `SetAll`, `Proceed`, `Cancel`); answers carry the
request id (`Request::{Detect, Import}.request`), a stale answer is dropped.

## The multi-select API of the selection sheet

* `List.multi: bool` (core `screens::select::list`): the rows carry a check; `List::checked()`
  gives the checked ids; `Intent::Toggle(id)` (Space, a digit or a click on a row in the sheet)
  flips one, `Intent::Confirm` (Enter) answers, `Picked::Items(Vec<String>)` is the answer.
  In a multi list a row click or digit toggles instead of accepting. Tests:
  `core/tests/select.rs` (3), `tests/sheets/select.rs` (2), fixture `multi_select`.
* The source prompt does not use the sheet (import-rest I1): it is `source::SourcePrompt`,
  the startup prompts' composition (the prompt surface at the top left, the shared `Options`
  controller for the cursor), and it answers with the flow's own intents (`Source(id)`,
  `Cancel`). The oracle's title is not indented, which the prompts' `Panel` does for its
  titles (it is bound to the startup model's prompts), so the prompt is composed beside it.

## Oracle tests (shard `tests/oracle/import.rs`, profile `oracle`)

17 oracle tests: 16 green, 1 n/a, 0 pending, 0 red (startup-finish turned on N1 x2 and G46).

| oracle (file::name) | ours | status |
|---|---|---|
| flow_tests.rs::external_agent_config_migration_messages_snapshot | `tests/pages/import.rs::external_agent_config_migration_messages_snapshot` (the kit renders screens; the notices are history lines) | n/a to the kit, green as a view test |
| flow_tests.rs: 5 state tests | `core/tests/import.rs` (detection x3, the flow) | green |
| mod.rs::prompt_snapshot, secondary_source_customize_snapshot | `tests/oracle/import.rs` | green |
| mod.rs::customize_snapshot, customize_action_snapshot | same | green: the detail rows are `dots_end` (N1 is not a gap) |
| mod.rs::action_rows_keep_single_line_allocation_and_focus_style | same | green: the kit reads the painted surface (`Screen::assert_painted`), G46 closed |
| mod.rs: proceed_returns_selected_items, toggle_item_then_proceed_keeps_remaining_selection, escape_skips_prompt, numeric_shortcuts_*, empty_selection_enter_*, control_exit_shortcuts_*, summary_does_not_toggle_selection | same | green |
| source_tests.rs::external_agent_config_source_prompt_snapshot | same | green (import-rest I1) |
| source_tests.rs::..._selects_highlighted_source, ..._can_cancel | same | green |

The state-test twins of the view (`tests/pages/import.rs`: the notices, the rows, the keys) carry
the oracle's names.

## Deltas and gaps

* N1 (not a gap): the detail rows go through `widgets/dots.rs` `dots_end`, which keeps the
  blank before the `…` (startup-finish).
* G46 (closed, not a gap): the painted surface keeps DIM on a blank, the presenter leaves it
  out on the terminal; the kit reads the surface (`assert_painted`, probe
  `probe_g46_the_painted_surface_keeps_dim_on_the_blank`).
* Mouse (ADR-011, startup-finish): a tap on a checklist item moves the cursor there, a tap on
  the item under the cursor toggles it; the same for the actions (a tap on the action under
  the cursor runs it). Tests: `tests/pages/startup_mouse.rs` `import_review_*`.
* G45 (select-dialog, earlier commit e3e98f3c6): `TabBar` cuts a tab at the edge; pending, no workaround.
* The review is composed in the page from rtwk primitives (Column, SizedBox, `Focus` +
  `KeyBindings` per region, `ListView` with a `ListController`, Text with ellipsis); it does
  not use the sheet's grid. Not `SelectionList` and not the startup prompts' `Panel`, by a
  probe of the rows (decisions, not rtwk gaps): `SelectionList` draws a one-cell cursor
  (`Marks::CURSOR`) before the mark and a fixed mark style, the oracle's row is `› [x] Label`
  with the checkbox part of the highlighted text; the `Panel` indents its titles and is bound
  to the startup model's prompts.
* Focus keys (I3): Up/Down move in the checklist and walk into the actions past its ends (and
  back), Tab is rtwk's own; Enter toggles in the checklist and runs in the actions. The step is
  `session_list::target(.., Move::Step, ..)` over the region's `SelectionController`; a step that
  wrapped hands the focus to the other region (summary actions just go round).
* import-review-ctrl: the rows read their cursor in a `ListenableBuilder` over the region's
  controller (as `widgets/session_list.rs`); the list is the `Flexible` (loose) of the review's
  `Column`, so the room comes from the layout, not from counting the other lines. `ListView`
  has no shrink-wrap (a loose `Flexible` around it fills the room: probe, blank rows under a
  short summary at 80x18), so its box is `SizedBox(height = number of rows)` (one cell per
  row, `item_extent(1)`) that the `Flexible` clamps to the room. Test:
  `tests/pages/import.rs::flow::the_checklist_scrolls_to_the_cursor_in_the_room_the_other_lines_leave`
  (80x13: the room is three rows, the window follows the cursor).
* `source.rs` stays as it is, not the startup prompts' `Panel` (import-review-ctrl, checked): the
  `Panel` reads `ScreenStates.startup.open` (a `Prompt` of the startup model) and its keys, rows
  and answers are that model's `Cmd`/`Reaction`; the import's source rows and answers
  (`Source(id)`, `Cancel`) are the import model's, and the oracle's title is not indented. Using
  it would need a startup `Prompt` variant for the import (core change in the startup model) or a
  generic panel extracted from `Panel`; neither is inside this task's area. What the two share
  is already shared: `Options` (cursor and scroll) and `Tokens::prompt_surface`.
* Behaviour added beyond the oracle: Escape and Ctrl+C during the detection and in the source
  prompt cancel the flow (the oracle has no key while it asks the server).

## Wiring (import-rest I2, I3, (3))

* The flow runs through `fx.commands`: `apply` pushes `Command::Import { ask }` (the `Request`
  with the request number, `Model::generation`); the shell sends it to the backend (the fake:
  answers only when a responder is scripted, else the flow stays where it is as the oracle
  waits on the server; the host: the real server) and the answer returns as
  `AppEv::Screen(ScreenEv::Import(Ev::Detected | Imported))`; a stale answer is dropped by
  `apply`. The server's completion notification (`ExternalAgentConfigImportCompleted`) is matched
  on `import_id` by `screens::import::observe` (a shared daemon broadcasts other clients'
  completions) and feeds `Ev::Completed`.
* The StartupPage pops (Msg::Leave) when the flow is Done at a later request number than when
  the page opened on the import step; the import `Screen` no longer pops itself.
* Tests added: `core/tests/import.rs` (flow, import id), `tests/pages/import.rs` `mod flow`
  (`slash_import_asks_every_source_and_reviews_what_one_found`,
  `importing_sends_the_checked_items_and_leaves_the_page_for_the_chat`, the focus walk, back and
  toggle tests), the source prompt snapshot (I1).

## What others must do (outside this slice)

* The chat cells show `Outcome::Started` and `Model::finished` through `notices::started` and
  `notices::finished` (`Outcome::Failed(Failure)` through `labels::failure`, `NoItems` through
  `strings.no_items`). Not done (import-rest (2)): the chat's transcript cells are an empty
  stub and `ThreadEv` has no history-notice event; owned by `chat-cells`. The interface is ready.
* Known limitation: the oracle's remote-cwd override is not applied; the detection asks with
  `config.cwd`.
* Edits outside the area: `host/src/import.rs`, one arm in `host/src/in_process.rs`,
  `mod import;` in `host/src/main.rs` (startup-deltas' area, to be merged together),
  `tests/oracle.rs`, `tests/oracle/tools/shards.py`, `docs/rtwk-port/test-shards.md`,
  `docs/rtwk-port/rtwk-gaps.md` (N1, G46), `Cargo.toml` (`rtwk_g46` test), `tests/app/navigation.rs`,
  `src/pages/startup/onboarding/mod.rs` (the Done step, `StartupPageState.import_from`; startup-flow's area).
* Flaky tests, not from this slice: `slash_import_opens_the_import` and
  `import_is_a_step_of_the_startup_page` press Escape after `pump_for`, before the shell has applied
  the page's `Open` (real-thread timing; Escape then finds the phase Idle and does nothing);
  `status_data::*` fail the same way on load.
