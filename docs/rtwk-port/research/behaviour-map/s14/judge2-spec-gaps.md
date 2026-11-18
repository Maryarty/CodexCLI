# S14 spec.md checked against the maps and the code

Ids: A = A.csv (S14-Hnnn), B = B.csv (S14-Rnnn). Line numbers are
`codex-rs/tui/src/bottom_pane/` unless a path is given; `L:` = list_selection_view.rs.

## (b) Statements in spec.md the code contradicts

1. UC-11 / rule list: "`on_highlight` fires whenever the highlighted key changes
   — arrows, filtering, **a shortcut**, and at open when the first highlight is
   not `initial_key` — and not when it stays the same."
   - A shortcut accepted at once does not fire the callback; only a
     `require_explicit_confirmation` row fires it (L:1025-1040; B R152, A H091/H092).
   - At open the oracle compares with the preselect (initial index OR the enabled
     current row), so a current-row highlight does not fire either (L:673-690,
     737-742; B R021).
   - A tab switch fires unconditionally, also when the highlighted row is
     unchanged (L:856 `self.fire_selection_changed()`; B R042). "Not when it stays
     the same" is false there.
2. Rule 1: "→ else the first enabled row → **else nothing highlighted**" together
   with key #11 "with nothing highlighted cancels". With every row disabled the
   oracle selects visible row 0 (L:723-731 `.or_else(|| (len > 0).then_some(0))`,
   B R018): drawn without highlight, but Enter does nothing (accept returns on a
   disabled row, L:991-994), it does not cancel. Only an empty/no-match list
   cancels (L:1017-1022).
3. Rule 1 / UC-07: the "first enabled **current** row" preselect (and Y's current
   row after a tab switch) applies only when the list is not searchable
   (L:683 `(!self.is_searchable)`, B R015, R042). A searchable picker goes to the
   first enabled row.
4. Rule 2: "…else returns to the previously highlighted key if shown, else the
   first enabled row." The previous key is used only when the old position is out
   of range (L:708-731). If the position is in range but that row is disabled the
   result is the first enabled row, not the previous key (B R090 states it
   loosely too; the code is the authority).
5. Rule 3: "Disabled → name + ` (disabled)`, dim, description `disabled:
   <reason>`" for `disabled: Option<Disabled{reason}>`. The suffix is added only
   when a reason exists (selection_row_layout.rs:122-124); a row disabled without
   a reason is only dimmed (A H144 wrongly adds is_disabled). The description is
   `disabled: <reason>` only with no description; with one it is `<desc>
   (disabled: <reason>)`, and under HideWhenNarrow / BelowWhenNarrow the bare
   reason (selection_row_layout.rs:47-64; B R127).
6. Key table #12 / UC-08: "row's secondary key | row enabled; not typing in
   **`Always`** search". The guard is `allow_plain_char_navigation` = search field
   *visible* with a plain key, i.e. also while the on-demand bar is open (L:1304-1308,
   1403-1411; B R058, A H058). Likewise #10/#11 omit that, with the on-demand bar
   open, a plain-character cancel/accept binding is typed into the query
   (`allow_plain_char_actions`, L:1309-1315; B R099).
7. Key table #15 "printable | **no search** | a row's shortcut or number": the
   arm is `!self.is_searchable` (L:1436-1462). A picker with `Search::OnDemand`
   and the bar closed is searchable, so it has no number or letter shortcuts, and
   no number gutter (L:771-773; B R064, R067, A H139). UC-05 "(no search…)" hides
   this.
8. Rule 5: side content goes beside the list "when the panel gets at least its
   minimum width". The oracle also needs the list part to be at least 40 columns
   (L:78-115; B R143). Layout, so a delta at most.
9. UC-09: "`allow_cancel: false` both are ignored". Ctrl-C while the on-demand
   bar is open still closes the bar (on_ctrl_c checks the bar first, L:1536-1543;
   B R110, A H077). Minor.

## (a) Product rules the maps state and the spec does not (a porter needs them)

| rule | rows | why a porter needs it |
|---|---|---|
| Space on an enabled placeholder row, Immediate search, empty query: swallowed (neither toggles nor types) | B R077, A H055 | row 9 / UC-06 say nothing; a plain `TextField` would type the space |
| `initial_query` opens the search bar even for `OnDemand` | B R007, R094, A H097, H109 | `Search::OnDemand{..}` + `initial_query` has no stated meaning |
| An item's secondary key that equals a bound list action (accept/cancel/move/…) is dropped; tab items are not checked | B R012, A H030 | defines which secondary keys are legal; the tab quirk is a decision |
| Direct shortcut / child-letter match is case-sensitive, over the top-level items, only when not searchable, never a disabled row; the number counts enabled non-child rows of the active list | B R060, R062, A H063, H064 | UC-05 states numbers only; with tabs + items the oracle matches one list and selects in another (A H063 risk) |
| search-toggle key is shadowed by any bound list action, and needs an enabled toggle row | B R078, A H042 | row 1 says only "highlighted row has a toggle" |
| Page / jump landing on a disabled row: page_down/jump_top look below then above, page_up/jump_bottom above then below, else stay | B R031-R034, A H045-H048, H100 | UC-02 only points at a test |
| Tab switch: resets scroll, fires the highlight callback, re-applies the filter; query kept when `search_rows` exists | B R040-R042, A H095 | partly in UC-07; callback and reset missing |
| Toggle is ignored on a disabled row; Space with no toggle row falls through to typing / nothing | B R076, A H083 | UC-06 |
| On-demand: Backspace does not close the bar (only Esc / Ctrl-C do) | B R084 | UC-04 |
| Paste: `normalize_pasted_search_query` splits on any whitespace; ignored when no bar visible | B R103-R105, A H010-H011 | stated in UC-10 (covered); listed for completeness |
| The key press that opened the picker must not act on it (activation guard, 1 s window) | A H004 (mod.rs:911-913, 2065-2098, activation_key_guard.rs) | an Enter that opens a picker would otherwise pick row 1; rtwk dialogs may or may not give this |
| Footer note shows the selected row's `description` under `BelowWhenNarrow` and replaces `footer_note` | B R134, A H149 | spec §1 has the enum, not the rule |
| `reserve_result_rows`: the panel keeps its height while filtering or switching tabs | B R137, A H161 | not in the spec; product-visible (jumping height) |
| `disabled_gutter_marker`, `name_prefix_spans` | B R071, R154, A H121, H120 | caller-specific row fields absent from `Row<K>` |
| On-demand bar never reopens by itself when a refresh rebuilds the picker (oracle defect: reopens) | A H032 (verified in code) | the spec's `Rows::Live` avoids it; worth a test (UC-12: "bar stays closed") |
| `picker()` preset: Panel surface, stable columns, hide descriptions below 24 cells | B R013 | the defaults of `Surface`, `Descriptions` |

## Not gaps (checked, the spec is right)

Precedence of the key table matches the match arms (L:1317-1462); highlight at
open order; case-insensitive `search_text`, `None` hides the row; paste
normalisation; accept with no rows cancels even with `allow_cancel: false`;
hint priority (L:625-649); numbering stops at 9; toggle prefixes and
placeholders.
