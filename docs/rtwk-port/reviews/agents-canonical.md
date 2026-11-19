NEARLY — the screen is built of rtwk widgets (SelectionList, TextField, show_dialog, KeyHintsBar, LayoutBuilder breakpoints, NotificationListener), but its key routing and focus are hand-driven by a mode switch, and two of the five "gaps" are not gaps.

# Agent command center on rtwk standard means (review, 2026-11-19)

Scope: `codex-rs/tui-rtwk/src/screens/agents/*` at `a3e18148e` and its use of `src/ui/page_keys.rs`. rtwk pin
`e3debfda` (`~/.cache/codexcli/rtwk-pin`). The question asked of each piece: how would a Flutter developer write it,
and what is rtwk's counterpart? Each hand piece was checked by a run, in a scratch crate outside the repository:
`/tmp/agents-canon-probe/tests/render.rs` (G36, G37, G38) and `tests/focus.rs` (G35, Refocus, clicks), using
`rtwk_test::Tester` against the pin.

## The five hand pieces

| piece | Flutter way | rtwk way (API, file:line) | verified by run? | real gap? |
| --- | --- | --- | --- | --- |
| `tabs.rs` filter strip (G36) | `TabBar` + `TabController` (`labelPadding`, `isScrollable`) | `TabBar::new` / `strip` (`rtwk-controls/src/tabs.rs:276,298`); no gap option, tabs touch (`:915-935`), marks painted over cells 0 and w-1 (`:961,965`), scrolls by column | yes: at 100 cells `TabBar` draws `   All 1  Needs you 0  Working 0 …`, but the oracle has `   All 1   Needs you 0   Working 0 …`. At 40 cells `… Working 0  Rea›` (oracle `… Working 0   ›`), and active=4 gives `  ‹ 1  Working 1 …` (oracle `  ‹  Working 0   Ready 1   Inactive 1`) | **yes** (gap (a) can be built by composition, (b) and (c) cannot). `strip()` is a pure window function and the strip is a `Row` of `Text`, which is fine. The index should still be held in a `TabController`, not in `ScreenState.filter` |
| `path.rs` path cut (G37) | no such mode in Flutter either: a helper function under `LayoutBuilder` | `TextOverflow::{Ellipsis,EllipsisStart,EllipsisMiddle}` (`rtwk-unicode/src/fit.rs:30`), cut by cell | yes: `EllipsisMiddle` at 24 cells gives `~/hello/the/…s/very/fast`, but the oracle wants `~/hello/the/…/very/fast` | **yes, but legitimate**: carried pure oracle logic (`center_truncate_path`) that a Flutter developer would also write. Keep it, and measure with the tree's segmenter (fix 4) |
| `soft.rs` wrapped-row count (G38) | `TextPainter(text, maxLines).layout(maxWidth: w)` + `computeLineMetrics()` under a `LayoutBuilder` | `rtwk_render::objects::text::TextPainter` (`rtwk-render/src/objects/text/painter.rs:110`, `layout`/`height`/`with_max_lines`) and its wrapper `break_lines` (`rtwk-render/src/objects/text.rs:100`), both public in `rtwk-render`, which the crate already depends on | yes: `break_lines(.., 38, Word)` gives 3 rows, the same 3 rows `Text::new` draws at 38 cells, row for row. `Text::max_lines(2)` cannot show the oracle's elision row | **no**. The mechanism exists and `soft.rs` is the TextPainter idiom; only the facade does not re-export it. Reclassify G38 as "facade export" and use `segmenter_of` (fix 4) |
| `neighbour` / `Act::Step` (G35) | `ListView` + `ScrollController`; j/k through `Shortcuts`→`Intent`→the list's `Action` | `SelectionList` keeps its `ListController` private (`rtwk-controls/src/selection_list.rs:351,386`). Only its own keys scroll (`move_to` → `ensure_index_visible`, `:495-504`), and they are bound by name in `Pressable::bindings` (`:955-967`), with no `Actions` to target. `SelectionController` has no step (`rtwk-data/src/selection_controller.rs:184-258`). `Scrollable::ensure_visible` (`rtwk-scroll/src/scrollable.rs:1302`) needs a mounted row, which a virtualized off-screen row is not | yes: after `set_cursor(Some(12))` in a 5-row list the screen still shows rows 0-4. The list's own Down then jumps to 13 and scrolls | **yes**. j/k/ctrl-n/p past the window leave the highlight off screen (already pending: `tests/agents.rs:669`, `tests/pets.rs:81`) |
| `refocus_after_frame` / `Msg::Refocus` | `autofocus: true` on the widget a mode shows; a removed node gives focus back through the scope; `addPostFrameCallback((_) => node.requestFocus())` only as a last resort | `AppBinding::add_post_frame_callback` (`rtwk-widgets/src/binding/app_binding.rs:759`) is the exact counterpart of the last resort. The standard route is: `Focus::autofocus` ("when its scope has none", `rtwk-framework/src/focus/widgets.rs:191`, settled at `focus/manager.rs:531-541`); recovery of a lost focus to the stop beside it (`manager.rs:525-530,750-753`); `ExcludeFocus` (`widgets.rs:867`); `Offstage` (`rtwk-widgets/src/layout/visibility.rs:222`); and `KeyBindings` on the focused subtree (`focus/shortcuts.rs:1257`). `request_focus` is a no-op on a detached node (`focus/node.rs:436-456`), which is why a request sent from the handler itself fails | yes, two probes. **Current shape** (one key table at the root, chosen by mode; help and empty replace the list): without Refocus, help→browse and empty→list leave the focus on the root, and the list's keys go dead. **Flutter shape** (the keys on the widget of each mode, help and empty each a `Focus` stop with autofocus, the list kept mounted under `Offstage` + `ExcludeFocus`): all 14 transitions land on the right node with **no** Refocus (`scoped_needs_no_refocus`) | **no** |

## mod.rs and view.rs, line by line (what a Flutter developer would not write)

Ranked as defect, risk or note. `view.rs` lines are prefixed with `view.rs:`. Every other line number is in `mod.rs`.

1. **Defect, mechanism shown by a probe: clicks are not refused during a rename.** The screen ignores a row click
   while renaming (887-889). But `ExcludeFocus` only removes focus: the `SelectionList` still takes the click and
   moves its cursor itself. Probe `excluded_list_still_takes_clicks`: with the line focused, a click moves the
   cursor from `None` to `Some(3)`. rtwk route: `SelectionList::enabled(!renaming)` (`selection_list.rs:270`). The
   rows draw their own fill, so nothing changes visually.
2. **Risk, found by reading (not run): the usage of the highlighted task is fetched late.** `NeedUsage` is sent only
   from `sync` (629-653), which runs in `did_change_dependencies` (on a model change) and on a tab change.
   `Msg::Cursor` (871-878) only rebuilds. A cursor moved by arrows, j/k or a click therefore asks only at the next
   model change, at worst the 20 s clock tick (`shell.rs:116`). The oracle asks on the repaint that follows the
   move. Flutter: in the selection listener. Fix: run the usage check from `Msg::Cursor`, and add a test.
3. **Focus juggling.** `Msg::Refocus`, `refocus_after_frame` (661-667), `root_focus`, `listed` (654-657), and the
   four mode tables picked in `build` (434-477, 1003-1015) re-implement focus-scoped key routing. Flutter puts a
   `Shortcuts` on each mode's widget; rtwk puts a `KeyBindings` around the list (or the empty pane), around the
   `TextField`, and around the help's `Focus`. The probe shows the result is equivalent and needs no post-frame
   request.
4. **Two flags for one mode.** `help: bool` and `editing: Editing` (311-324) are mutually exclusive in practice.
   One `enum Mode { Browse, Help, Search, Rename(id) }` would remove the `self.help` / `editing_now()` cross
   checks (766-798, 859-868).
5. **State written during build.** `row_taps: RefCell` is filled in `build` (945-962), against the port pitfall
   "a view never writes state". Fix: compute the keys and their taps where the center changes
   (`did_change_dependencies` / `update`). The tab taps are new closures on every build (963-968), so the header
   never compares equal.
6. **Hand measuring with the wrong segmenter.** `path.rs:13`, `soft.rs:14`, `tabs.rs:38` and `help.rs:29` all
   measure with `TextSegmenter::default()` or `string_width(Graphemes, 1)`. rtwk's own rule
   (`rtwk-widgets/src/text/segmenter.rs:12-16`) is to measure with `segmenter_of(cx)`, the one `Text` lays out
   with. Probe: `‹ All 3 … ● ›` is 13 columns by default and 15 with ambiguous=2. A CJK terminal would then cut
   paths and count prompt rows differently from what `Text` draws.
7. **Side effects in `did_change_dependencies`.** It pops the Navigator (696), opens the failure dialog (714-731)
   and sends `NeedUsage`. Flutter would use a listener (`BlocListener` / `ref.listen`). rtwk-provider has no
   listener widget (`read`/`watch`/`select` only, `inherited_provider.rs:1896-1915`), so the edge detectors
   `usage_seen`, `entered`, `search_seen` and `failing` are the available route. Note only, and a candidate gap
   row ("a provider listener").
8. **The filter index is plain state.** `filter: usize` (316) is a mode of one widget. Under the 2026-11-19b rule
   it belongs in a `TabController` (`tabs.rs:106`), even while the hand strip paints it pending G36.
9. **The rows learn they are selected through a listener of their own.** Each row has a
   `ListenableBuilder(selection)` (`view.rs:671`, the same in resume `mod.rs:282`), because `SelectionList`'s item
   builder gets no selected or cursor flag (its dressing is styles only, `selection_list.rs:660-750`), while the
   row needs the `›` marker. That is a fair workaround and a candidate gap row (Flutter's `ListTile.selected`).
10. **Notes, all canonical:** the `LayoutBuilder`s read only the oracle breakpoints (989-999, `view.rs:384,1007`).
    `details` is LayoutBuilder + line counting, the TextPainter idiom (`view.rs:775-884`). The overflow arrows use
    `NotificationListener<ScrollNotification>` + `ValueNotifier` (`view.rs:698-748`). Both dialogs go through
    `show_dialog` from handlers. The footer is `KeyHintsBar` (`page_keys.rs:29`).
11. **R14 tension.** `view::Ask` carries the task name, which is data, into the confirm dialog
    (`view.rs:1114-1119`), because the dialog sits above a seeded screen's providers. `FailureDialog` meanwhile
    reads the model through `select` (`dialogs.rs:151-160`). One of the two is wrong for seeded screens: pick one
    rule.

## Reuse (architecture.md §1, Amendment 2026-11-19b)

- **A session list widget, `ui::session_list`.** Agents (`view.rs:630-748`) and Resume (`resume/mod.rs:255-320`)
  repeat one pattern: `SelectionList` + a `ListenableBuilder` per row for the selected flag + bar and styles off +
  `NotificationListener` → `Scrolled` `ValueNotifier` → overflow marks. `Scrolled` is defined twice
  (`view.rs:242`, `resume/mod.rs:121`). Share the list, with an item builder `(index, selected)` and the edge
  marks; each page keeps only its row content.
- **One relative-age formatter** (oracle `format_relative_time`). It exists twice: `view.rs:307-323` and
  `resume/layout.rs:51` with `resume/strings.rs:165`.
- **A shared confirmation picker.** `ConfirmDialog` and `FailureDialog` (`dialogs.rs`) are the oracle's
  `ListSelectionView` with `confirmation()` / `LifecycleHeader`. `select/dialog.rs` is the same view bound to a
  core menu. Per the 2026-11-18 amendment ("one shared `Picker`"), all three should be parameterisations of one
  picker widget (title, description, numbered rows, digits, hint).
- **The footer** is already shared (`page_keys::footer` / `footer_fit`): keep it.

## Fixes, in order

1. Fix 1: rename clicks. Add `SelectionList::enabled(!renaming)` and a test that a click while renaming leaves the
   highlight where it was.
2. Fix 2: usage on cursor moves. Run the usage check from `Msg::Cursor`, and add a test that an arrow move asks
   for the new task's usage within the same flush.
3. Fix 3: keys by focus. Put a `KeyBindings` on each mode's widget; give help and empty a `Focus` stop with
   autofocus; keep the list mounted under `Offstage` + `ExcludeFocus`; delete `Msg::Refocus`,
   `refocus_after_frame`, `root_focus` and `listed`. The shape to follow is the `Scoped` widget in
   `/tmp/agents-canon-probe/tests/focus.rs`.
4. Fix 4: one segmenter. Measure with `segmenter_of(cx)` in `path.rs`, `soft.rs`, `tabs.rs` and `help.rs`,
   passing it in from the builder.
5. Fix 5: state out of build. Build the row taps outside `build`, merge the two flags into one `Mode` enum, and
   hold the filter in a `TabController`.
6. Fix 6: reuse. Add `ui::session_list` and a shared age formatter (Agents and Resume), and a shared confirmation
   picker (agents dialogs and `select`).
7. Fix 7: gap table. Change G38 to "not a gap: `rtwk_render` `TextPainter`/`break_lines`; facade export only".
   Keep G35 and G36 (both confirmed by run) and G37 (rtwk lacks the cut; the helper is legitimate). Candidate new
   rows: a SelectionList item builder that is told selected/cursor; a provider listener.
