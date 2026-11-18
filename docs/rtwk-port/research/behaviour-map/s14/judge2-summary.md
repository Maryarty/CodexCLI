# Judge round 2: A (176 rows) vs B (155 rows), S14

Coverage of B by A (matches.csv): full 118, partial 29, none 8 -> full+partial 147/155 = 94.8% (full alone 76.1%).
  none: R004 tabs empty, R013 picker() preset, R044 tab/search gap, R102 idle bar reserves no row, R119 zero-size render, R146 reduced width, R147 theme subtitle, R153 no mouse.
A-only rows: 49 (127 of 176 A rows match a B row). Real 2 (H004 activation-key guard; H032 replace-if-active reopens an on-demand search bar - defect verified in code). Noise 47 (host/approval/offline/replace-by-id/plumbing/test pointers).
Correctness of A (correctness_A.csv): correct 164, imprecise 11, wrong 1 (H094: construction callback also for initial/current highlight, which the code does not fire). Imprecise: H002 H039 H045 H073 H088 H100 H144 H147 H174 H175 H176. Render rows H148-H168 were read for the cited ranges only in part (spot checks), the rest rated on the keys/host code that I read in full.
Classes (classes.csv)   product / mechanism / layout / input / caller / fixture
  A: 71 / 56 / 22 / 6 / 11 / 10   (product 40%)
  B: 109 / 14 / 25 / 5 / 2 / 0    (product 70%)
Spec check (spec_gaps.md): 9 contradictions, 15 gaps.
Top 5: (1) UC-11 says on_highlight fires on a shortcut / at open / only on change; oracle: not for a shortcut accepted at once, not when the preselect is the current row, and a tab switch fires unchanged (L:1025-1040, 737-742, 856).
(2) Rule 1 "else nothing highlighted" + "nothing highlighted cancels": all-disabled picker selects row 0 and Enter does nothing (L:723-731, 991-994).
(3) Rule 3 disabled text: " (disabled)" only with a reason; "disabled: <reason>" only without a description (selection_row_layout.rs:47-64,122).
(4) Secondary key / plain-char cancel and accept are typed while the on-demand bar is open, not only in Always search (L:1304-1315, 1403-1411).
(5) Rule 2 fallback and key #15: previous key used only when the position is out of range; shortcuts/numbers vanish for any searchable picker, incl. OnDemand with closed bar (L:708-731, 1436).
Gaps worth a spec line: Space swallowed on placeholder rows, initial_query opens OnDemand bar, secondary-key dropped if bound to a list action, activation-key guard.
