# S14 — Selection dialog: specification

**What it is.** One generic list picker that ~30 places open (model,
permissions, review, worktrees, theme, …). The user moves through rows,
optionally filters them by typing, and either picks a row, toggles rows, or
cancels. In tui-rtwk it is a dialog (`show_dialog`) built of rtwk widgets:
`Dialog`, `SelectionList<K>`, `TextField`, `TabBar`.

Oracle: `codex-rs/tui/src/bottom_pane/list_selection_view.rs`, its host
`bottom_pane/mod.rs`, `selection_row_layout.rs`. Evidence per rule: the
behaviour maps (`sonnet-reference.csv` rows `R…`, Haiku round-2 rows `H…`).
Checked against the code by an independent reviewer (round 2); its findings
are fixed here.

## 1. API

```rust
/// Opens the picker from any widget under the Navigator.
/// Resolves to `None` when the user cancels.
pub fn pick<K: Clone + Eq + 'static, S>(cx: &Cx<'_, S>, spec: PickSpec<K>)
    -> DialogResult<Picked<K>>;

pub enum Picked<K> {
    Primary(K),    // accept, a row shortcut or number
    Secondary(K),  // the row's secondary key
}

pub struct PickSpec<K> {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub rows: Rows<K>,                  // Static(Vec<Row<K>>) | Live(fn(&Model) -> Vec<Row<K>>)
    pub tabs: Vec<Tab<K>>,              // empty: no tabs
    pub initial_tab: Option<TabId>,
    pub search: Search,                 // Off | Always { placeholder } | OnDemand { key, placeholder }
    pub search_rows: Vec<Row<K>>,       // flat list a tabbed picker shows while a query is typed
    pub initial_key: Option<K>,         // row to highlight at open
    pub initial_query: Option<String>,  // opens the search with this query (OnDemand too)
    pub numbering: Numbering,           // Auto (1–9) | RowShortcuts
    pub max_rows: usize,                // visible rows; at least 1; default 8
    pub reserve_rows: bool,             // keep the height while filtering / switching tabs
    pub row_display: RowDisplay,        // Wrap | SingleLine (truncated with …)
    pub descriptions: Descriptions,     // Column | HideWhenNarrow(min) | BelowWhenNarrow(min)
    pub name_column_width: Option<usize>,
    pub side: Option<Side>,             // content beside the list (or below when narrow)
    pub footer_note: Option<Text>,
    pub footer_hint: Option<Text>,
    pub allow_cancel: bool,             // default true
    pub surface: Surface,               // Terminal | Panel
    pub on_highlight: Option<Handler<K>>,        // live preview (theme picker)
    pub on_toggle: Option<Handler<(K, bool)>>,   // a toggle was flipped
}
// PickSpec::picker(): Panel surface, column widths measured over all rows,
// descriptions HideWhenNarrow(24).

pub struct Row<K> {
    pub key: K,
    pub name: String,
    pub name_prefix: Vec<Span>,                // drawn before the name
    pub description: Option<String>,
    pub selected_description: Option<String>,  // shown instead while highlighted
    pub category_tag: Option<String>,
    pub search_text: Option<String>,           // None: hidden while a query is typed
    pub current: bool,                         // shows " (current)"
    pub disabled: Option<Disabled>,            // { reason: Option<String>, gutter_marker: Option<&str> }
    pub shortcut: Option<KeyBinding>,
    pub child_letter: Option<char>,            // indented under the row above, lettered
    pub toggle: Option<Toggle>,                // { on, checkbox, pending: u32, placeholder: None | Unavailable | Blocked }
    pub secondary: Option<Secondary>,          // { key: KeyBinding, hint: Text }
    pub close_on_pick: bool,                   // default true
    pub needs_confirm: bool,                   // a shortcut only highlights it
    pub footer_hint: Option<Text>,
    pub search_footer_hint: Option<Text>,      // instead, while the on-demand bar is open
}

pub struct Tab<K> { pub id: TabId, pub label: String, pub rows: Vec<Row<K>>, pub footer_hint: Option<Text> }
```

How it fits the app (ADR-002, ADR-003):
- The opener turns `Picked` into an intent with its ticket; the picker
  itself sends nothing.
- `Rows::Live` reads the model through a provider projection; when the model
  changes (a refresh, a confirmed toggle) the list updates in place: the
  highlight stays on the same key, the query is kept, a closed search bar
  stays closed.
- A flipped toggle shows at once with `pending + 1`; `on_toggle` sends the
  intent; the model lowers `pending` when the backend confirms (R9).
- A row that opens a second picker awaits it: `Some` closes both with the
  child's result, `None` returns to the first picker.

"Searchable" below means `search` is `Always` or `OnDemand` — also while the
on-demand bar is closed. "Typing" means a search field is shown (`Always`,
or the on-demand bar is open) and the key is a plain character.

## 2. Use cases

Each case is a screen test. *Accept* = the oracle test or snapshot that pins
it; *new* = no oracle test, write one from the cited code.

**UC-01 Pick with the keyboard.** Not searchable. Rows A, B (current), C.
Open → B is highlighted. Down → C. Enter → closes with `Primary(C)`.
Accept: new (the look: `snapshot_model_picker_width_80`). [R015, R024, R046–R047]

**UC-02 Disabled rows are skipped.** Rows A, B (disabled), C; A highlighted.
Down → C. Up from A → C (wraps). A disabled current row is not highlighted
at open. Every row disabled → row 0 is selected but drawn without highlight,
and Enter does nothing.
Accept: `disabled_current_rows_skip_default_selection_and_number_shortcuts`,
`page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping`.
[R017, R018, R023, R051]

**UC-03 Filter by typing** (`Search::Always`). Type `gp` → only rows whose
search text contains `gp`, case-insensitive. Backspace → one character
less. Nothing matches → the list shows `no matches`. Enter with no row shown
→ closes with `None`, even with `allow_cancel: false`.
Accept: `renders_search_query_line_when_enabled`,
`empty_searchable_list_keeps_search_and_no_matches_visible`,
`enter_with_no_matches_triggers_cancel_callback`. [R083–R091, R053]

**UC-04 Search on demand** (`Search::OnDemand { key: '/' }`). `/` → the
search bar appears. Type → filters; plain-character accept, cancel and
secondary keys are typed too. Backspace on an empty query keeps the bar.
Enter → picks the highlighted row. Esc → the bar closes, the query clears,
the picker stays open. Opened with `initial_query` → the bar is open with
that query.
Accept: new. [R007, R055, R084, R096, R097, R099]

**UC-05 Shortcuts and numbers** (`Search::Off` only — a searchable picker
has none, and no number gutter). `Numbering::Auto`: enabled non-child rows
of the shown list are numbered `1.`–`9.`. `2` → picks the second numbered
row at once. `0` or a number past the rows → ignored. A row's own shortcut
or child letter matches case-sensitively, never a disabled row. A row with
`needs_confirm` is only highlighted; Enter picks it.
Accept: `automatic_shortcuts_stop_at_nine`. [R060–R067]

**UC-06 Toggle rows.** An enabled row with a toggle. Space → it flips at
once and `on_toggle(key, on)` fires; it shows as pending until the model
confirms. On a disabled row nothing flips. While typing, Space is text —
except in `Always` search with an empty query on a placeholder row, where
Space does nothing. If Space is bound to a list action, the action wins.
Accept: `space_appends_to_active_search_instead_of_toggling_selected_item`,
`configured_cancel_binding_takes_precedence_over_space_toggle`,
`configured_list_binding_takes_precedence_over_search_toggle_key`.
[R072–R081]

**UC-07 Tabs.** Tabs X, Y. Right → Y (wraps to X after the last). Scroll
resets; the highlight goes to Y's current row (not searchable) or its first
enabled row; `on_highlight` fires. The query clears, unless `search_rows` is
set: then the query stays and, while non-empty, the flat search list is
shown across tabs.
Accept: `switching_tabs_changes_visible_items_and_clears_search`,
`tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch`.
[R037–R042, R093]

**UC-08 Secondary action.** A row with a secondary key. That key →
`Secondary(key)`. While typing (any search field shown) the key is text. On
a disabled row nothing happens.
Accept: `secondary_action_respects_search_disabled_rows_and_dismissal`.
[R056–R059]

**UC-09 Cancel.** Esc or Ctrl-C → closes with `None`. With
`allow_cancel: false` both are ignored, except that they still close an open
on-demand bar. While the bar is open, the first Esc / Ctrl-C only closes the
bar.
Accept: new. [R107–R112]

**UC-10 Paste into the search.** Paste ` foo   bar ` → `foo bar` is
appended to the query (whitespace collapsed) and the list filters. A
whitespace-only paste, or a paste with no search field shown, is ignored.
Accept: `paste_appends_to_search_query_and_filters_items`,
`whitespace_only_paste_is_ignored`. [R103–R105]

**UC-11 Live preview.** `on_highlight(key)` fires when navigation or
filtering moves the highlight to another key; on every tab switch, even to
the same key; on a `needs_confirm` row highlighted by its shortcut; at open
only when the first highlight differs from the preselect (`initial_key`, or
the current row). It does not fire for a shortcut or number that picks at
once.
Accept: `move_down_without_selection_change_does_not_fire_callback`; the
rest new. [R021, R025, R042, R092, R152]

**UC-12 Live rows.** The model changes the rows while the picker is open →
the list updates, the highlight stays on the same key if it is still
there, the typed query stays, a closed on-demand bar stays closed.
Accept: new. [R094, R081, H032]

**UC-13 The opening key does not act.** A picker opened by Enter (or any
key) → that same key press does not pick the first row.
Accept: new. [H004]

## 3. Behaviour

### Keys (keymap context `list`), in precedence order

An earlier line wins for the same key.

| # | key | when | does |
|---|---|---|---|
| 1 | search-toggle key (non-text) | bound to no list action; highlighted row enabled with a toggle | flips the toggle, even while typing |
| 2 | move up / down | not typing | previous / next enabled row, wrapping |
| 3 | page up / down | not typing | one page (the rows on screen), no wrap |
| 4 | jump top / bottom | not typing | first / last row, then the nearest enabled one (rule 7) |
| 5 | move left / right | tabs; not typing | previous / next tab, wrapping |
| 6 | Esc | on-demand bar open | closes the bar, clears the query |
| 7 | Enter | on-demand bar open | picks the highlighted row |
| 8 | Backspace | search field shown | deletes the last character |
| 9 | Space | row enabled with a toggle; Space bound to no list action; not typing, or `Always` search with an empty query | flips the toggle |
| 10 | Space | `Always` search, empty query, placeholder row | nothing |
| 11 | cancel | `allow_cancel`; not a plain character while the bar is open | closes with `None` |
| 12 | accept | not a plain character while the bar is open | picks the highlighted row; on a checkbox row flips it; with no row shown closes with `None`; on a disabled row nothing |
| 13 | row's secondary key | row enabled; not typing | `Secondary(key)` |
| 14 | on-demand key | searchable, bar closed | opens the bar |
| 15 | control character | — | ignored |
| 16 | printable, no Ctrl/Alt | search field shown | appends to the query |
| 17 | printable | `Search::Off` | a row's shortcut or number (UC-05) |
| 18 | anything else | — | ignored |

### Rules

1. **Highlight at open:** `initial_key` if enabled → else the first enabled
   current row (only when not searchable) → else the first enabled row →
   else row 0 without highlight (all disabled) → nothing (no rows).
   [R014–R019]
2. **Filtering:** case-insensitive substring of `search_text`; a row without
   it is hidden while the query is non-empty. After each edit the highlight
   keeps its *position*; if that position is past the end, it goes to the
   previously highlighted key if shown; if the row there is disabled, or
   the key is gone, to the first enabled row. [R087–R090]
3. **Row text:** `current` → name + ` (current)`. Disabled → dim; with a
   reason the name gets ` (disabled)` and the description becomes
   `disabled: <reason>` (no description) or `<description> (disabled:
   <reason>)`; with `HideWhenNarrow` / `BelowWhenNarrow` just the reason.
   Highlighted → `selected_description` replaces the description.
   `category_tag` stays when descriptions hide. Toggle prefix `[*]` on,
   `[x]` on for a checkbox, `[ ]` off; a placeholder shows `[-]`
   (unavailable) or `[!]` (blocked). [R080, R124–R128, R155]
4. **Footer hint**, first that exists: the highlighted row's
   `search_footer_hint` (bar open) → the row's `footer_hint` → its
   secondary key's hint → the active tab's hint → `footer_hint` → the
   standard `enter select · esc back`, built from the keymap's accept and
   cancel keys; with `allow_cancel: false` the standard hint is left out.
   [R010, R011, R136]
5. **Footer note:** `footer_note`; with `BelowWhenNarrow` in effect, the
   highlighted row's description replaces it. [R134, R135]
6. **Side content:** beside the list when the panel gets its minimum width
   and the list keeps at least 40 columns; otherwise below the list.
   Accept: `side_layout_width_half_falls_back_when_list_would_be_too_narrow`,
   `stacked_side_content_is_used_when_side_by_side_does_not_fit`.
   [R143, R144]
7. **Landing on a disabled row** after page / jump: page down and jump top
   look for the nearest enabled row below, then above; page up and jump
   bottom above, then below; none → stay. [R031–R034]
8. **More rows than fit:** the list scrolls with the highlight; `↑` / `↓`
   mark hidden rows above / below. With `reserve_rows` the panel keeps its
   height. [R036, R137, R138]
9. **Secondary keys** that equal a bound list action (accept, cancel, move,
   …) are dropped from the rows when the picker opens. [R012, H030]

## 4. How it looks

The oracle snapshots are the reference, at their sizes; layout (heights,
column positions, gaps) comes from rtwk and every difference is a delta in
the parity pack.

| state | snapshot (`codex_tui__bottom_pane__list_selection_view__…`) |
|---|---|
| plain list, with / without subtitle | `tests__list_selection_spacing_with_subtitle`, `…_without_subtitle` |
| model picker at 80 columns | `tests__list_selection_model_picker_width_80` |
| narrow width, wide characters | `tests__list_selection_narrow_width_preserves_rows`, `tests__list_selection_halfwidth_sound_marks_narrow` |
| empty search | `tests__list_selection_empty_searchable` |
| footer note wrapping | `tests__list_selection_footer_note_wraps` |
| column widths while scrolling | `tests__list_selection_col_width_mode_auto_visible_scroll`, `…_auto_all_rows_scroll` |
| picker presentation, tabs, short height, paging | `picker_tests__shared_menu_presentation_at_wide_and_narrow_sizes`, `picker_tests__short_browser_keeps_header_tabs_search_and_footer`, `picker_tests__browser_page_navigation_uses_rows_that_fit` |

## 5. Not carried

The bottom-pane view stack and its completion flags (`is_complete`,
`dismiss_after_child_accept`, `take_last_selected_index`), rebuilding the
view to refresh it (`replace_selection_view`, `set_search_query`), action
closures over the event sender, the per-row pending counter kept in the
view, key-release filtering and control-character normalisation (rtwk
input), and all height and column arithmetic (rtwk layout). Each is replaced
by the API above or done by rtwk.

## 6. Open points (the person decides: parity or fix)

1. Rule 2 keeps the highlight's position, not its row, after filtering.
2. The oracle's shortcut keys match the top-level rows but select in the
   active tab's rows (differs when a tabbed picker also has top-level
   rows); rule 9 does not check tab rows.
3. The oracle reopens a closed on-demand bar when a refresh rebuilds the
   picker (H032); UC-12 as written fixes it.
4. UC-13: does rtwk's dialog already ignore the key that opened it, or does
   the port need the oracle's guard?
5. Mouse: the oracle has none; rtwk's `SelectionList` may react to clicks.
