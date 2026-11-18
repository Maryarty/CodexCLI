# S14 behaviour-map judgement: A (127 rows) vs B (155 rows)

Judged on oracle code (list_selection_view.rs, selection_popup_common.rs, selection_row_layout.rs,
selection_picker_layout.rs, picker_*.rs, bottom_pane/mod.rs, theme_picker.rs, keymap.rs) and the tests.
Files: matches.csv, correctness_A.csv, correctness_B_sample.csv, testlinks_A.csv, testlinks_B.csv.

## Coverage (recall of A against B, 155 B rows)
- full 116 (74.8%), partial 15 (9.7%), none 24 (15.5%); full+partial = 131 (84.5%).
- A-only rows (4): real 2 (B120 theme picker sets preserve_side_content_bg; B089 active_tab_id accessor), noise 2 (B043 keymap rebind restatement; B116 60x18 fixture duplicate).
- Rule for verdicts: partial only when a distinct branch/condition/outcome of B is absent; minor detail omissions (e.g. header_gap amount, child-row indent) counted full and noted.

## Correctness of A (127 rows, cited ranges read)
- correct 119, imprecise 8, wrong 0. Citations are accurate (every sampled range lands on the stated code).
- Imprecise: B038 (j/k listed without "not while search visible"), B045 (current row used only in non-searchable lists), B053 (callback fires only if the first selection differs from the preselected item), B077, B106 (arrow placement), B107 (Panel gap is on top, not bottom), B112 (height sum omits indicator rows/gaps), B121 (fixture instead of the width-fit rule).
- Several rows are fixture/test restatements (B101-B104, B116-B119): true but low-information.

## Calibration of B (every 5th row, R005..R150, 30 rows)
- correct 29, imprecise 1 (R090: the "row disabled" case goes to the first enabled row, not to the previously selected item), wrong 0.
- B notes also show its row set includes BottomPane-level wiring (mod.rs) that A did not read.

## Test links (15 rows each, rows that list tests)
- A: match 6, weak (asserts only part of the rule) 7, mismatch 2 (B051 is the B050 case; B112 asserts only SingleLine < Wrapped height). Accurate-or-weak 13/15; fully matching 6/15.
- B: match 7, weak 3, mismatch 5 (R010 BottomPane hint replacement not exercised; R021 test is the R025 rule; R023 covers move_down remap only; R052 and R078 cite tests of the opposite/other branch). Fully matching 7/15.
- Both maps link tests by name overlap more than by asserted content; mismatches are mostly "the test covers the neighbouring branch".

## What A misses most (24 none + 15 partial)
1. Key-precedence and edge branches (8 none): Space bound to a list action wins over toggle (R075); Space on a disabled/toggle-less row (R076); Backspace ignored with the field hidden (R085); plain-char accept/cancel binding is query text during on-demand search (R099); left/right ignored without tabs (R039); search query kept on tab switch when search_items exist (R041); navigation with no rows (R026); all-rows-visible when query empty (R088). Partial: plain-char list bindings typed during search (R027/R028/R074/R098).
2. BottomPane wiring around the view (6 none): standard hint replaced by keymap hint (R010), dropped without allow_cancel (R011), secondary_action dropped when its key is a list action (R012), paste completing the view clears the stack (R106), replace_selection_view inherits dismiss flag (R115), Release key events ignored (R116). A scoped itself to list_selection_view.rs.
3. Row decoration / render details (7 none): selected_description (R128), category_tag (R155), name_prefix_spans (R154), " (key)" shortcut suffix (R069), blank row between tabs and search (R044), reduced row width beside the side panel (R146), no mouse handling (R153).
4. Constructor normalisation (3 none): empty tabs disable tabs (R004), placeholder discarded when not searchable (R008), max_visible_rows clamped to 1 (R009).
Where A does cover a rule it is usually right; its weak spots are render geometry (arrow rows, Panel gap, height sum) and the on_selection_changed construction condition.
