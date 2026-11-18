# parity-routes: how rtwk gives the oracle's exact look and behaviour

Task `parity-routes`, 2026-11-18. rtwk read at `d47446d1`; the pin `7482e340` has
the same code for every API named here. Oracle: `codex-rs/tui`. Claims marked
**proved** ran in `/tmp/parity-routes-proof/tests/{proof,keys}.rs` (a scratch crate
on the rtwk checkout; the tests named below are green).

## Synthesis

Every rejected delta comes from one mistake: the ready widget (`Dialog`,
`Autocomplete`, a line painter) was used where the oracle's look is a
*composition* that rtwk builds directly.

- **The bottom pane is a slot of the chat view, not a dialog.** The oracle draws an
  inline view *in place of the composer*: the transcript shrinks and its tail stays
  visible. A `Dialog` covers the tail. In rtwk the slot is one widget under
  `Expanded(transcript)` in the chat `Column`, rendering the top of a core view
  stack by id. **Proved:** at 80 columns the rows equal the oracle `.snap` rows,
  `›` included, with no `max_width` trick, and the transcript tail stays on screen.
- **One shared `Picker`**, the twin of `ListSelectionView` (225 constructions of
  `SelectionViewParams` in 62 oracle files, approvals among them). The `›` marker
  is text in the row. The selected row is `BoxWidget::style(sel)` +
  `DefaultTextStyle(sel)`: bold+reverse over the whole width and the wrapped lines
  (**proved**: attrs `BOLD|REVERSE` on the row's last cell). The description
  column wraps with its hanging indent via `Row[SizedBox(name col), Expanded(Text)]`.
- **Only `Centered` views are dialogs** (`SelectionViewParams::confirmation()`).
  `show_dialog`'s builder is any widget, so barrier alpha 0 + `min(w-4, 72)`
  columns is the oracle's `dialog_area` exactly. ADR-002 holds for them.
- **Three "blocking gaps" are not gaps:**
  - G23 (`~` filler): `LayoutBuilder` + `Stack` + a filled `BoxWidget` (**proved**).
  - G22 (row fill): the Picker route (**proved**).
  - G28 (frameless dialog): `show_dialog`'s builder is free.
- **G21 is narrower.** A `ScrollController` listener hears the first extents
  (**proved**: `0/25` before any move), so the pager's `estimate_max` goes.
- **Composer popups cannot use `Autocomplete`** (a floating `Popover`, and Tab
  picks; G24/G25 stay open, off-route). Instead: give the `TextArea` our
  `FocusNode` and `set_on_key_event` on it, which runs *before* the TextArea's
  keys, then hand keys to the chat state via `cx.channel()`. **Proved:** `/m`,
  Down, Tab gives `/mention ` with the cursor on row 2 and nothing run.
- **Mapping error:** `/status` is a *history cell* in the oracle
  (`chatwidget/status_controls.rs:279-282`, `add_to_history(cell)`), not a dialog.
  S19 moves to the transcript (item 7).

**Decision for the person:** amend ADR-002 so inline bottom-pane views are a chat
slot driven by core state (ids only); only `Centered` views and real dialogs use
`show_dialog`. This also removes G14/G16 for inline views: nothing reopens, and no
close watcher is needed.

---

## Item 1 — N01/N02: bottom-pane views (S13, S14; then S15/S23/S24)

**Oracle.**
- `bottom_pane/mod.rs:2489-2510` `renderable_for_views`: the top view replaces the
  composer flex, and its `desired_height` sizes the pane.
- `view_stack.rs:14-27`: a `Centered` view sits over the retained previous view at
  `min(w-4, 72)` columns, centred, with no frame and no dim.
- `picker_presets.rs:13-31`: `confirmation()` → Centered; `picker()` → Panel.
- `list_selection_view.rs:1636-1800`: header inset 2; rows at column 0 with prefix
  `› `/`  `; description column (`compute_desc_col`,
  `selection_popup_common.rs:139`) wrapping with a hanging indent;
  `MAX_POPUP_ROWS = 8` (`popup_consts.rs:14`).
- `selection_popup_common.rs:234-272` `apply_row_state_style`: every line of the
  selected row gets `selection_style()` (full width); disabled rows are dim.
- `style/contrast.rs:35-78`: blue fill + bold when the terminal bg is known, else
  (as in tests) bold + reversed.
- The Panel surface is `user_message_style()` plus a top gap. Approvals are
  `picker()` views (`approval_overlay.rs:341`), so they are inline.

**rtwk route.**
- **Slot.** The chat body becomes `Column[Expanded(1, transcript),
  BottomPane{thread}]` (today `src/screens/chat/home.rs:384-394`). `BottomPane`
  is a stateless `eq` widget. It `select`s `Option<ViewKey>` (top of
  `chat::Model.bottom_pane`) and builds the composer, or that view keyed
  `Key::value(view_key)`. Focus is free: the composer leaves the tree, the view
  autofocuses, and on close the composer returns with its text (in
  `ThreadUi.composer`).
- **Rows.** `Row[SizedBox(desc_col, Text::rich(prefix + name)),
  Expanded(Text(desc).style(dim))]` in a `Column(Min)`. Selected:
  `BoxWidget::new().style(sel).child(DefaultTextStyle::new(sel, row))`
  (`rtwk-widgets/src/layout/box_.rs:269`, `text/widgets.rs:375`). The attrs reach
  the text only through `DefaultTextStyle` (**proved**: without it only blank
  cells reversed).
- **Window.** Up to 8 *items* from `scroll_top`; the pane height is the sum of
  their wrapped heights (`measure_rows_height`, `selection_popup_common.rs:650`).
  A `Column` of the window gives exactly that. The window is the carried pure
  `bottom_pane/scroll_state.rs` in the picker's widget `State` (UI-only, R14).
  `SelectionList`/`ListView` scroll by rows over a bounded height
  (`selection_list.rs:51-56`), so they cannot page by items.
- **Centered.** The presenter (under the `Navigator`) calls `show_dialog` with
  builder `Padding(x 2) > ConstrainedBox(max_w 72) > Picker::of(id)` and
  `DialogOptions{alignment: CENTER, barrier_alpha: Some(0), fade_in: ZERO,
  barrier_dismissible: false}`. The builder is free
  (`rtwk-overlay/src/dialog.rs:301-305`, placed by `Align` `:485-492`); `Dialog`
  (`:584-621`, raised, max 72) is not used.
- **Why not a bottom-aligned dialog:** it overlays the transcript tail instead of
  shortening it, and it floats over every thread.
- **Surface.** Tokens derive `user_message_bg` and `selection_style` from
  `TerminalBrightness::background_of(cx)` (`style/theme.rs:1113`). Without a bg
  they fall back exactly like the oracle (bold+reversed, no fill).

**Sketch** (`src/ui/picker/mod.rs`):
```rust
pub struct Picker { pub view: ViewKey }            // id only (R14); stateful eq
pub struct PickerState { scroll: ScrollState }     // carried, oracle: bottom_pane/scroll_state.rs
fn build(&self, w: &Picker, cx: &mut Cx<'_, Self>) -> AnyWidget {
    let spec = select::<ScreenStates, Rc<PickerSpec>, _>(cx, |s| s.picker_spec(&w.view));
    let t = theme::Tokens::of(cx);
    let (start, end) = self.scroll.window(spec.rows.len(), MAX_POPUP_ROWS);
    let col = desc_column(&spec.rows[start..end], spec.col_mode);   // carried compute_desc_col
    let rows = (start..end).fold(column_min(), |c, i|
        c.child(picker_row(&spec.rows[i], Some(i) == self.scroll.selected, col, &t)));
    let body = column_min().child(Padding::new(EdgeInsets::left(2), header(&spec, &t)))
        .child(rows).child(footer_line(&spec.footer, &t));
    Focus::new(BoxWidget::new().style(t.panel_surface()).child(body)).autofocus()
        .bind(KeyPress::ESC, cx.send(PickMsg::Cancel)).on_key(cx.keys(pick_key)).into()
}
```

**Shared widgets.** `src/ui/bottom_pane.rs` (`BottomPane`); `src/ui/picker/`
(`mod.rs`, `row.rs`, carried `scroll_state.rs`, `layout.rs` with
`compute_desc_col`/`ColumnWidthMode`); tokens in `src/theme.rs`. Core owns
`PickerSpec` (title, subtitle, rows {name, desc, shortcut, disabled, match
indices}, footer) and `bottom_pane: Vec<ViewKey>`.

**Tests.**
- Full oracle rows via `assert_rows` with no `›` stripping, at the oracle size
  (80 for `list_selection_model_picker_width_80`). Seed the view rather than
  growing the screen for the debug menu.
- `assert_style_at` BOLD|REVERSE at x=0, x=79 and on the wrapped line (oracle
  asserts BOLD, `selection_popup_common.rs:913`).
- The transcript's last line stays above the pane; pane height = window height;
  Esc pops the view and the composer returns with its text and focus.

**Gap.** None. G28 → not a gap; drop N01/N02.

## Item 2 — D1–D10 (screens-routes)

- **D1 `~` filler** (`pager_overlay.rs`):
  `LayoutBuilder(c) → SingleChildScrollView(ConstrainedBox(min_h = c.max_height,
  Stack[Column of "~", BoxWidget::new().style(Style::new()).child(lines)]))`.
  **Proved:** `line 1`, ``, `line 3`, `~`, `~`, `~`; a longer page shows no `~`.
  (`Stack[filler, scroll view]` fails: the viewport clears its area.) G23 → not a
  gap; drop `without_tilde`.
- **D2 row fill:** `BoxWidget.style + DefaultTextStyle` (item 1, **proved**); zebra
  rows likewise. G22 → not a gap.
- **D3 inset / D4 82-cell screen:** mirror the oracle areas, not our chrome:
  header/toolbar/search in `Padding(x 1)` (`resume_picker/layout.rs:53-72`); list
  at `left 2` in `SizedBox(list_viewport_width(w))` (`:74-79`); responsive gaps
  (`areas`, `:26-43`) from `LayoutBuilder` → `SizedBox` rows. Tests at 80 columns,
  whole screens.
- **D5 usage panels:** `rtwk-data` `BarChart` (`bar_chart.rs:177`), `Heatmap`
  (`heatmap.rs:220`), `Sparkline` (`sparkline.rs:48`), `Table`. Map each oracle
  panel and compare its snapshot; name any glyph difference per chart. Own slice.
- **D6 toolbar mouse:** `GestureDetector::on_tap` per tab
  (`rtwk-widgets/src/gestures/gesture_detector.rs:211`) → the keys' intent.
- **D7 graphemes:** carry `text_formatting::truncate_text`
  (`tui/src/text_formatting.rs:78`, `"..."`) verbatim with `unicode-segmentation`
  (already in `Cargo.lock`; separate dependency commit). Not `Text` ellipsis:
  rtwk writes `…` and drops the blanks before it (`rtwk-unicode/src/fit.rs:30-46`).
- **D8 agents keys:** bindings now, emitting typed intents that end in typed "not
  yet" notices until session-core lands.
- **D9 `~` label:** the shell sends core the home dir once; core derives the label
  with the carried `format_directory_display`.
- **D10 triggers:** chat-nav `dispatch` → `Navigator::push(PageRoute)` for Ctrl+T,
  `/diff`, `/usage`, `/resume`, `/agents`. Wiring only.

## Item 3 — G6: resume, agents, usage lists; pager estimate

**Oracle.** `resume_picker.rs:2274` `ensure_selected_visible`: `scroll_top` is an
*item* index, grown until `scroll_top..=selected` fits the content rows. `:2515-2519`
`has_more_above/below`: `↑ more`/`↓ more` take list rows. `:3281-3365`: 1–2 line
items plus a spacer (comfortable); the top is always an item boundary.

**rtwk route.**
- `ListView::builder` + `ListController` in the screen's widget `State`. A
  selection key, in `State::update`:
  1. read `ctrl.position().viewport_dimension()` (`rtwk-scroll/src/scrollable.rs:307-330`);
  2. run the carried `ensure_selected_visible` (pure);
  3. call `ctrl.jump_to_index(scroll_top)` (`list_view.rs:134-136`).

  rtwk owns layout, virtualization and paint; the oracle owns which item is on
  top. `ensure_index_visible` (minimal, `:157`) leaves partial items at the top.
- `physics(NeverScrollableScrollPhysics)` (`rtwk-scroll/src/lib.rs:68`); the
  wheel moves the selection, as in the oracle.
- `Column[if scroll_top>0 {↑ more}, Expanded(list), if has_more_below {↓ more}]`.
- Rows: `BoxWidget.style(row).child(DefaultTextStyle(row, Text::rich(spans)))`.
  No `Vec<Line>` painter, no `Rc<Cell>` set in build. Agents: the same; its
  details pane is wrapped `Text` (drop `layout::wrap`/`ellipsize`). Usage: a
  `Column` of widgets (D5); no `screen_lines`.
- **Pager percent:** `ListenableBuilder::new(Rc::new(ctrl.clone()), |_, _|
  percent(ctrl.position().metrics()))`. The controller forwards position
  notifications (`scrollable.rs:873-883`), and `apply_extents` notifies on the
  first extents (`:534-557`). **Proved** `0/25` before any move; `estimate_max`
  goes.
- The R14 rule `r14_controllers_are_made_only_in_the_allowlist` must list pager,
  resume and agents (a controller in widget `State` is UI state). Orchestrator.

**Sketch** (resume `update`):
```rust
if let Msg::Move(step) = msg {
    let n = read::<ScreenStates, _>(cx).resume.filtered().len();     // handler input, no same-flush decision
    self.selected = step.apply(self.selected, n);
    let view = self.ctrl.try_position().map_or(usize::MAX, |p| p.viewport_dimension() as usize);
    self.scroll_top = ensure_selected_visible(self.selected, self.scroll_top, view, &self.heights); // carried
    self.ctrl.jump_to_index(self.scroll_top);
}
```

**Shared widget.** `src/ui/item_list.rs` `ItemList` (ListView + controller +
item-aligned window + more rows + row style), used by resume and agents.

**Tests.** `resume_picker_table` and the more-indicator snapshots as whole 80-col
screens. Step Down past the window: the top row is an item's first line; the
selected style spans the full width. Pager `static_overlay_*` with `~` and the
percent label. Break `jump_to_index` once to see the window test fail.

**Gap.** G21 only for `ScrollNotification`; add the controller-listener route.

## Item 4 — N1 + G3: composer `/` popup

**Oracle.** `command_popup.rs` + `selection_popup_common.rs`: rows
`› /name   description`, match indices bold, rows unavailable in a side
conversation dim. The popup is in the bottom pane *under* the composer, and the
pane grows. `slash_input.rs:288-336`: Tab completes `/name ` and runs nothing (only
`/skills` dispatches, `:539`); Enter runs.

**rtwk route.**
- Not `Autocomplete`: it is a `Popover` (`autocomplete.rs:1063-1110`), Tab picks
  or completes a prefix (`:652-705`), and the marker is its own.
- **Keys.** `TextArea::new().controller(c).focus_node(node)`
  (`rtwk-editing/src/text_area.rs:718-722`). The chat state calls
  `node.set_on_key_event(Some(cb))` (`rtwk-framework/src/focus/node.rs:251-262`).
  `route_node` runs a node's `on_key_event` before that node's `Focus`
  bindings/handler, the TextArea's editing (`focus/manager.rs:971-1000`). A given
  node keeps its callback (`focus/widgets.rs:356-372`). `cb` returns `Handled`
  for Up/Down/Tab/Enter/Esc when the core pure `popup::active(text)` says open,
  and sends `ChatMsg::PopupKey` via `cx.channel()` (`cx.rs:466-480`). **Proved**
  (`keys.rs`).
- **Rows.** Picker rows with `Text::rich`: core candidates carry `match_indices`
  (bold) and `disabled` (dim).
- **Pane.** `BottomPane` builds `Column[composer_row, popup_rows]` while open, so
  the transcript shrinks.
- **State.** The cursor is chat widget state (carried `ScrollState`). Enter sends
  `Intent::Run(cmd id)`. Tab writes `"/{name} "` to the controller and runs
  nothing, except `/skills`.

**Sketch.**
```rust
fn did_change_dependencies(&mut self, _: &Chat, cx: &mut Cx<'_, Self>) {
    if self.armed { return; }
    let (tx, text) = (cx.channel(), self.ui.composer.clone());
    self.ui.node.set_on_key_event(Some(Rc::new(move |_, k: KeyPress| {
        if popup::active(&text.text()) && popup::takes(&k) { let _ = tx.send(ChatMsg::PopupKey(k.code)); KeyEventResult::Handled }
        else { KeyEventResult::Ignored } })));
    self.armed = true;
}
```

**Shared widget.** Picker rows (`src/ui/picker/row.rs`) in a `ComposerPopup`
(`src/screens/popovers/`); the hooked field in `src/ui/composer_field.rs`.

**Tests.** `command_popup_app` / `wrap_boundary` full rows with `›`; bold cells at
the match indices; dim attrs on a disabled row. Tab on `/quit` leaves `/quit ` and
the app running (G3's failing input); Tab on `/skills` dispatches. Pane grows; the
transcript tail stays.

**Gap.** None for parity. **Risk:** channel items apply at the next flush (same
frame), so a character typed in the same burst may reach the text before the
popup key. The oracle is strictly ordered. Test a fast `Down`+`x`.

## Item 5 — N2: `@` mentions v2

**Oracle.** `mentions_v2/render.rs:38-131`, `footer.rs:13-31`:
- `Mentions` bold, then the dim subtitle, inset 2;
- the filled tab bar (`selection_tabs.rs:42`, `active_tab_style` fill+bold;
  fallback bold+underlined in `picker_style.rs`);
- the query (dim placeholder `Type to search mentions`);
- 8 fixed rows;
- footer `enter/tab insert · esc close · ↑/↓ select · ←/→ filter`, chips in
  `key_hint_style` (`key_hint.rs:296`).

**rtwk route.** A Picker header variant:
- header: `Column[Text::rich(bold), Text(dim)]`;
- tabs: `Row` of `Text(" label ")`, the active one `BoxWidget.style(active_tab)
  + DefaultTextStyle`. The visible tab slice comes from the carried pure helper
  of `selection_tabs.rs`;
- query: `Text(..).style(dim).max_lines(1)` with the carried oracle ellipsis;
- rows: `SizedBox` of 8 Picker rows;
- footer: `Text::rich(key_chip spans)`.

←/→ go through the item 4 key hook → `Intent::MentionMode(next)`. Mode and
ranking stay core: carry `search_mode.rs`, `candidate.rs`, `search_catalog.rs`
(pure).

**Tests.** `default_unified_mention_popup`, `plugin_mention_popup` and the
`mentions_v2/render_tests.rs` set (incl. the `active_tab_style` check) as full
rows; `scrolling_mentions_shifts_rendered_window` via the carried `ScrollState`.

**Gap.** None.

## Item 6 — N5 history-search footer, N8 Esc hint

**Oracle.** `footer.rs:752-775` `FooterMode` (`HistorySearch` →
`reverse-i-search: `, secondary style); `chat_composer/history_search.rs:397-401`
query `.cyan()` plus key chips, matches highlighted in the preview;
`footer.rs:914-925` `esc_hint_line`: Esc chip + ` again to edit previous message`.

**rtwk route.** One `Text::rich(spans)` per footer row with tokens `secondary`,
`key_hint`, `accent_cyan`. Carry `FooterMode` selection into core
`chat::footer_mode(props)` (pure); move `esc_primed` from `ChatState` to the core
model so `EscHint` derives. The preview highlight is `Text::rich` split at the
carried match ranges. Ctrl+C in search restoring the draft (N6) is one binding.

**Shared widget.** `src/ui/footer.rs`: `footer_line(mode, &Tokens)`, `key_chip`.
Picker footers and mentions use it.

**Tests.** `footer_mode_history_search` full rows with styles (query cyan, chips in
the key style); `history_search_footer_action_hints_are_emphasized` ported by
name; Esc on an empty composer shows the Esc-hint row exactly.

**Gap.** None.

## Item 7 — N7: `■ …` error as a transcript cell (and S19 `/status`)

**Oracle.** `history_cell/notices.rs:363-369`: `new_error_event(msg)` is a
`PlainHistoryCell` of `format!("■ {message}").red()`, added via `add_to_history`.
`/status` is a history cell too (`status_controls.rs:279-282`).

**rtwk route.** Core plus one row builder:
- core `Item::Notice(Notice::{Error, Usage, Disabled})` and `Item::Status(StatusCard)`,
  upserted under a local `ItemKey` (R4: no `push`); `Model::notice` goes;
- the transcript builder maps `Notice::Error(t)` →
  `Text::new(format!("■ {t}")).style(tokens.error())`;
- the cells slice later puts every kind behind `cells::row(item)`; this adds the
  first two.

**Tests.** Submit `/nope`: the last transcript row is the oracle's words with red
fg, and the composer is empty. `/status`: the
`status_snapshot_includes_reasoning_details` rows appear in the transcript after
the echo line. Core: R4 snapshot and R2 round-trip gain the new kinds.

**Gap.** None.

---

## Plan: fix slices, order, ownership

Layers: views in `src/screens/<d>`; shared UI widgets in `src/ui/`; tokens in
`src/theme.rs`; UI-only carried logic beside its widget (`ScrollState` in
`src/ui/picker/`), model logic in core. Core has no rtwk and gives `PickerSpec`s,
view ids and candidates.

1. **`ui-pane`** (first, small; blocks 2 and 3; needs the ADR-002 amendment).
   `BottomPane`, `Picker` (+ carried `ScrollState`, `compute_desc_col`),
   `footer_line`/`key_chip`, tokens `selection`/`panel_surface`/`key_hint`/
   `active_tab` from `TerminalBrightness`; core `bottom_pane: Vec<ViewKey>` +
   `PickerSpec`; the slot in the chat `Column`. Tests: item 1 rows and styles on a
   seeded spec.
2. **`dialogs-pane`**. S13/S14 into the slot, S19 → transcript cell, then
   S15/S23/S24 on `Picker`. Drop `Dialog`, `max_width(83/84)`, `SizedBox(count*3)`
   and the model copy in State (G5). Centered confirmations via presenter +
   free-builder `show_dialog`.
3. **`chat-nav-pane`** (after 1). Composer key hook; `/`, `@` v2, `$` and file
   popups as pane rows; Tab completes, never runs (G3); footer modes (N5, N8, N6);
   the `Item::Notice` cell (N7); D10 triggers.
4. **`routes-lists`** (parallel with 1; needs only tokens). `ItemList`; resume
   and agents on it; usage `LayoutBuilder` gaps; pager `~` and percent; carried
   `truncate_text` (dependency commit); D9 in core. Then **`usage-panels`**
   (D5/D6, `rtwk-data`).

Order: 1 ∥ 4 → 2 → 3.

**Rules to add with them.** No `Dialog(` in the pane screens outside Centered
presenters; no `Rc<Cell<` in a screen `State` (G6); the R14 controller allowlist
gains pager, resume and agents; `assert_rows` drops its `›` stripping.

**rtwk-gaps.md (orchestrator).** G22, G23, G28 → not a gap (proofs above). G21
stays for `ScrollNotification` only, with the controller-listener route noted.
G24/G25 stay open (Autocomplete), off the parity route. No new row is needed.
