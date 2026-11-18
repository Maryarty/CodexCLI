# rtwk-first-sweep (2026-11-18)

Rule: process.md §5 "No copy of the oracle's mechanics"; fleet brief "Behaviour, not mechanics".
Scope: non-terminal tm tasks, docs/rtwk-port/research and architecture.md, landed code in
codex-rs/tui-rtwk/src and core/src (the agents screen excluded; it is being rewritten).

Tasks read: 40 (live claims skipped: codex-perf-baseline, agents-live, agents-live-ui, screen-test-kit, this task; already amended today: routes-lists, reuse-carry). Amended: 4. Doubtful: 6.

## 1. Changes

| task / file:line | before (short) | after (short) | why |
|---|---|---|---|
| tm `ui-pane` | Picker carries `ScrollState` + `compute_desc_col` | `SelectionList`/`SelectionController`; rtwk layout and widths; own function only where a proof shows rtwk cannot | oracle's scroll and column mechanics |
| tm `chat-nav-pane` | popup cursor = carried `ScrollState` in chat state; carried tab slice helper and ellipsis; history-search query edited in core | `SelectionController`; `TabBar`; `TextOverflow::Ellipsis`; TextField/TextEditingController for the search query | selection, paging, tabs, text edit belong to rtwk |
| tm `dialogs-pane` | (silent) padded label columns via `chars().count()` in the landed views it moves | rtwk layout + rtwk-unicode widths; file:line of the hits added | display width is rtwk-unicode's |
| tm `dialogs-rest-pane` | S15 views on core `cursor/query/tab` + carried `wrap()` ring; `chars().count()` padding | `SelectionController`, `TextField`, `TabBar`; rtwk widths; pets named | selection/text edit state in core |
| research/parity-routes.md:84 (item 1 "Window") | window = carried `scroll_state.rs`; `compute_desc_col` | SUPERSEDED note: SelectionController, rtwk layout | scroll/selection mechanics |
| research/parity-routes.md:160 (D7) | carry `text_formatting::truncate_text` + unicode-segmentation | SUPERSEDED: `TextOverflow::Ellipsis`, rtwk-unicode | width/truncate helper |
| research/parity-routes.md:251 (item 4 "State") | cursor = chat widget state (carried `ScrollState`) | SUPERSEDED: `SelectionController` | selection index |
| research/parity-routes.md:292 (item 5 tabs) | Row of Texts, carried `selection_tabs.rs` slice, carried ellipsis | SUPERSEDED: `TabBar`, `TextOverflow` | hand-drawn tab strip |
| research/parity-routes.md:305 (item 5 tests) | `scrolling_mentions_shifts_rendered_window` via carried `ScrollState` | SUPERSEDED: assert against the controller | scroll math |
| research/parity-routes.md:361 (plan 1) | `ui-pane` with carried `ScrollState`, `compute_desc_col` | SUPERSEDED (as item 1) | same |
| research/parity-routes.md:379 (plan 4) | carried `truncate_text` dependency commit | SUPERSEDED | same as D7 |
| research/reuse-plan.md:103 (§2.1 table) | `text_formatting.rs` + `width.rs` REUSE by path | SUPERSEDED note above the table | width/truncate helpers |
| research/reuse-plan.md:251 (agents-live list) | include `text_formatting.rs` + `width.rs` | SUPERSEDED | same |

Already marked before: parity-routes item 3; reuse-map.md header (items 1, 4, 5, 7-9). architecture.md: no passage prescribes such a mechanism (it already says the composer is rtwk `TextArea`, lists use `SelectionList`/`TabBar`); nothing marked.

## 2. Landed-code hits (agents screen excluded)

| file:line | smell | rtwk replacement | owner |
|---|---|---|---|
| core/src/commands.rs:566-631, :762, :838, :849 | `ScrollState` selected_idx/scroll_top, `ensure_visible`, wrap moves, `MAX_POPUP_ROWS` windows | `SelectionController` in the popup | `chat-nav-pane` (amended) |
| core/src/screens/popovers.rs:271-296 | history-search query `push`/`pop` in core | TextField + TextEditingController in the footer | `chat-nav-pane` (amended) |
| core/src/screens/settings.rs:134-135, :186, :214-215, :289-290, :396-400 | `cursor`, `query`, `tab` indices and `wrap()` ring in core | `SelectionController`, `TextField`, `TabBar` | `dialogs-rest-pane` (amended) |
| src/screens/dialogs/shortcuts.rs:44-53; dialogs/feedback.rs:108 | `chars().count()` padding | rtwk layout + rtwk-unicode | `dialogs-rest-pane` (amended) |
| src/screens/approvals/dialog.rs:155-166 | `chars().count()` label padding | `Row` + measured column, rtwk-unicode | `dialogs-pane` (amended) |
| src/screens/select/dialog.rs:90-94 | `chars().count()` name width | same | `dialogs-pane` (amended) |
| src/screens/status/card.rs:145-156 | padded strings by `chars().count()` | `Row`/`Table` | `dialogs-pane` (amended) |
| src/screens/resume/layout.rs:75-80, :378-436; resume/mod.rs:110-135, :229, :241-275, :302, :353-354 | `truncate`, `ensure_visible`, `scroll_top: Rc<Cell<usize>>` written during build, `selected` index, chars-width | `SelectionList` + `SelectionController`, `TextOverflow` | `routes-lists` (already amended today; hits not added) |
| src/screens/pager/mod.rs:122-176 | `estimate_max` word wrap, `chars().count()` header/label width | `ScrollController` listener, paragraph wrapping | `routes-lists` (already amended) |
| src/screens/usage/mod.rs:116, :210, :234, :273-280 | `chars().count()` widths in hand-laid lines | rtwk-unicode, `Row`/`Table`, rtwk-data charts | `routes-lists` (usage list parts) / usage slice; amended text covers it |
| core/src/screens/pets.rs:52-53, :201; src/screens/pets/mod.rs:91, :267 | picker `cursor` ring in core, `top: Cell<usize>`, `chars().count()` centring | `SelectionController`, `TextField`, rtwk-unicode | **needs a new fix task** (named in `dialogs-rest-pane` only conditionally) |

## 3. Needs a new fix task

- pets picker (core/src/screens/pets.rs, src/screens/pets/mod.rs): cursor, query and `Cell` window onto `SelectionList`/`SelectionController` + `TextField`.
- routes-lists should list the hits above (resume, pager, usage) when it is next touched; it was amended today so I left its text.

## 4. Doubtful (not edited)

- `chat-composer`: lists oracle editing mechanics (kill ring, undo, paste_burst) but says "Build on rtwk TextArea"; arch G12 owns the gap. Fine unless a porter retypes an edit buffer.
- `keymap`: "vim mode state machine for the composer as an input handler" may re-implement TextArea cursor movement; check rtwk `TextEditingController` first.
- `parity-routes` Picker as a hand-composed widget (not `SelectionList`): the person's route to the oracle's look; the carried window helper is marked, the widget itself is left.
- architecture.md:316 "one shared `Picker`": same reason.
- reuse-map.md rows `wrapping & text utils` REUSE (URL-preserving wrap, unicode-width): claimed rtwk gap; the reuse-map header already supersedes part of it.
- src/screens/chat/home.rs:132 `chars().count()` for an ASCII prompt width: trivial, owner screens-chat/chat-live.
