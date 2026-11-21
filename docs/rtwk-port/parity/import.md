# import parity

Slice S06: `/import` from other agents (oracle `codex-rs/tui/src/external_agent_config_migration/`
`{mod,flow,model,render,source}.rs`, 8 snapshots; the two `_windows` ones are `cfg(windows)`).
It is a step of the startup page (`onboarding::Step::Import` renders `import::Screen`), not a page.

## Where the code is

| layer | files |
|---|---|
| core (no rtwk, no words) | `core/src/screens/import/mod.rs` (Model, Phase, Intent, Ev, Request, Outcome, Failure, Started, Finished, `apply`, `seed`), `detect.rs` (Source, Detection), `item.rs` (typed projections of the items: Kind, GroupKind, Sel methods, Places) |
| view | `src/pages/startup/import/mod.rs` (the step: `Screen`, the source prompt, the review), `nav.rs` (Nav: page, focus, cursor, action; Key), `rows.rs` (the review's lines), `labels.rs` (labels, details, descriptions, failures), `notices.rs` (the history lines of a started and a finished import), `strings.rs` (words, English `EN`), `fixtures/*.jsonl` (mock data through core `apply`) |
| generic sheet | multi-select in `core/src/screens/select{,/list}.rs` and `src/sheets/select/` |
| tests | `core/tests/import.rs` (flow), `tests/pages/import.rs` (words and keys), `tests/oracle/import.rs` (the oracle's screen tests, shard `import`), `tests/rtwk_g46.rs` (probe) |

Layers: core holds the flow as typed data (`Phase`, `Outcome::{Started, NoItems, Cancelled,
Failed(Failure)}`, `Group { kind }`, `Finished { by_type }`, `Model::error` is the server's raw
text); every word (group and item labels, nouns, detail lines, the notices of the history, the
errors, the source names) is told in the view from that data (ADR-010). The review's navigation
(page, focus, cursor, highlighted action) is the view's own state (`Nav` in the widget state),
never a field of the model; `Model::review_id` counts the reviews opened so a `Nav` is for the
review it opened on. ADR-003: one model, changed only by `apply`; the view emits intents
(`Open`, `Source(id)`, `Toggle(index)`, `SetAll`, `Proceed`, `Cancel`); answers carry the
request id (`Request::{Detect, Import}.request`), a stale answer is dropped.

## The multi-select API of the selection sheet

* `List.multi: bool` (core `screens::select::list`): the rows carry a check; `List::checked()`
  gives the checked ids; `Intent::Toggle(id)` (Space, a digit or a click on a row in the sheet)
  flips one, `Intent::Confirm` (Enter) answers, `Picked::Items(Vec<String>)` is the answer.
  In a multi list a row click or digit toggles instead of accepting. Tests:
  `core/tests/select.rs` (3), `tests/sheets/select.rs` (2), fixture `multi_select`.
* The source prompt does not use it (one source is chosen): it is a `List` with `numbered`,
  opened as `Menu::List`, its pick returns as `Picked::Item(id)`.

## Oracle tests (shard `tests/oracle/import.rs`, profile `oracle`)

17 oracle tests: 12 green, 1 n/a, 4 pending, 0 red.

| oracle (file::name) | ours | status |
|---|---|---|
| flow_tests.rs::external_agent_config_migration_messages_snapshot | `tests/pages/import.rs::external_agent_config_migration_messages_snapshot` (the kit renders screens; the notices are history lines) | n/a to the kit, green as a view test |
| flow_tests.rs: 5 state tests | `core/tests/import.rs` (detection x3, the flow) | green |
| mod.rs::prompt_snapshot, secondary_source_customize_snapshot | `tests/oracle/import.rs` | green |
| mod.rs::customize_snapshot, customize_action_snapshot | same | pending N1 |
| mod.rs::action_rows_keep_single_line_allocation_and_focus_style | same | pending G46 |
| mod.rs: proceed_returns_selected_items, toggle_item_then_proceed_keeps_remaining_selection, escape_skips_prompt, numeric_shortcuts_*, empty_selection_enter_*, control_exit_shortcuts_*, summary_does_not_toggle_selection | same | green |
| source_tests.rs::external_agent_config_source_prompt_snapshot | same | pending G47 |
| source_tests.rs::..._selects_highlighted_source, ..._can_cancel | same | green |

The state-test twins of the view (`tests/pages/import.rs`: the notices, the rows, the keys) carry
the oracle's names.

## Deltas and gaps

* N1 (rtwk, existing row, extended): `TextOverflow::Ellipsis` drops the blank before the `…`;
  the plugins detail row of the customize view differs by one blank (2 snapshots pending).
* G46 (new row): a blank cell never carries DIM in rtwk; the oracle's `set_style(area, dim)`
  dims the blank at cell 2 of the action row (probe `tests/rtwk_g46.rs`, green).
* G47 (new row, decision of the brief, not an rtwk limit): the source prompt is the generic
  selection sheet, so its frame is not the oracle's top-left screen (bottom anchored, title
  indented two cells, no blank between title and subtitle, two blanks before the list).
  The prompt snapshot is pending against that.
* G45 (select-dialog, earlier commit e3e98f3c6): `TabBar` cuts a tab at the edge; pending, no workaround.
* The review is composed in the page from rtwk primitives (Column, SizedBox, ListView with a
  `ListController`, Text with ellipsis); it does not use the sheet's grid.
* Behaviour added beyond the oracle: Escape and Ctrl+C during the detection and in the source
  prompt cancel the flow (the oracle has no key while it asks the server).

## What others must do (outside this slice)

* Root wiring: a root `Command`/`AppCtx` handler in `core/src/app.rs`, `wire`/`decode`, the fake
  backend and the shell must send what `Model::request()` derives (`Detect` with the request id
  and `cwd`, `Import` with source id and items) and feed `Ev::Detected`/`Ev::Imported`/
  `Ev::Completed` back; the shell also supplies `Places` (home, repository roots).
* The chat cells show `Outcome::Started` and `Model::finished` through `notices::started` and
  `notices::finished` (`Outcome::Failed(Failure)` through `labels::failure`, `NoItems` through
  `strings.no_items`).
* startup-flow advances or pops on `Phase::Done` (the page does `Navigator::maybe_pop`).
* Edits outside the area: `tests/oracle.rs`, `tests/oracle/tools/shards.py`,
  `docs/rtwk-port/test-shards.md` (shard registration), `docs/rtwk-port/rtwk-gaps.md` (N1, G46,
  G47), `Cargo.toml` (`rtwk_g46` test), `tests/app/navigation.rs` (the placeholder assertion of
  `import_is_a_step_of_the_startup_page`), `tests/pages/import.rs` (the slot test file).
