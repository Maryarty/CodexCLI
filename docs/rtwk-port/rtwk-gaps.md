# rtwk gaps

What tui-rtwk needs from rtwk and does not get today: one row per gap, each
with the tui-rtwk code that works around it and a minimal repro against the
rtwk API. Written 2026-11-18 from `docs/rtwk-port/architecture.md` ("rtwk
gaps"), the two issue files in `~/.cache/codexcli/rtwk-issues/`, the "rtwk
needs" sections of `parity/*.md` and today's slices.

Every row was checked against rtwk's source at `d47446d1` (the pin
`7482e340` plus 13 commits: `memo_keyed`, `attach_in`, Table docs, perf
baselines; none of them touches a row below but the memo ones). Two repros
were also run (G21, G26) and one compiled (G20); the others are read from the
code and say so.

## How to file

- One rtwk tm task per row (rows G1..G36), the "Repro" of the row attached,
  the "Needed by" and the workaround path in the task's description. A row
  with status "by design" or "not a gap" is not filed as a bug; file it only
  if the person wants a docs or ergonomics task.
- Filing is the person's call: this table has an empty `rtwk task` column
  until then. Nothing was created in rtwk's tm.
- **G13 and G16 are fixed in the pin (rtwk `9f28b7fb`, task `rtwk-repin`).**
  `shell::dialog_closed` and the `CloseWatcher` are gone; `ui::presenter`
  shows the request dialog with `AppCx::show_dialog_from` from the kept handle
  of the Navigator home's memo node and decides in `closed` on the model;
  `shell` no longer names `rtwk::Cx` / `DialogResult`.
- Statuses: **open** (confirmed in rtwk's source), **by design** (documented
  behaviour; the workaround is the intended use), **not a gap / conditional**,
  **fixed in rtwk `<commit>`** (in the pin).

Tally at `e3debfda`: 36 rows plus N1..N4; 17 closed in rtwk, 3 partly, 8 not a gap or ours, 10 open (G35, G36, N1, N2, N3, G1, G9, G24, G8, N4); see "Status at rtwk e3debfda".

## rtwk's answers (2026-11-18)

rtwk's orchestrator answered rows G13–G28 with code checked on rtwk branch
`claude/app-dialogs` (not yet landed): `docs/rtwk-port/refs/rtwk-answers-2026-11-18.md`.
Where the answers and the table below differ, the answers win:
- **Not gaps (app work):** G21 (listen to the `ScrollController`, which
  notifies on the first layout; only the initial `ScrollNotification` is
  absent, as in twk), G22 (styled `BoxWidget` in a stretch column, or
  `SelectionList` styles), G23 (`LayoutBuilder` + `~` rows), G26 (by design as
  in twk: `Text::new("› ").wrap(TextWrap::None)`), G27 (a passing handler,
  `on_key(cx.keys(..).passing())`), G28 (an unframed `show_dialog` builder; a
  pane that replaces the composer is drawn in the page, ADR-002 amendment).
- **Fixed in the pin `9f28b7fb` and in use:** G13 (show from a memo node under
  the Navigator; `show_dialog_from` for queued requests), G16
  (`AppCx::show_dialog(builder, options, closed)`; `shell::dialog_closed`,
  the `CloseWatcher` and the shell's `DialogResult` are gone). G14 is
  by design: decide on the model in `closed`.
- **Intended fix already in use:** G15 (`GlobalKey` in app state).
- **Answered:** G17 (`run_app_with` takes `RunOptions { theme, locale }`),
  G18 (app-lifetime work on the runtime, `main` owns shutdown), G19
  (`Tester::pump_app` gives an `AppHandle`; tokio via `rtwk_tokio::enter`),
  G24/G25 (the pick order is twk's; act on `Suggestion.text`; a pane-growing
  popup is `inline_suggestions()`, own marker and match ranges need our own
  list; tui-rtwk draws its own list in the bottom pane, research
  `parity-routes` item 4).
- **Real, coming in rtwk's gap batch:** G20 (`rtwk::style::{Attrs, Color}`).

## Status at rtwk `e3debfda` (2026-11-18, task rtwk-repin-2)

From `docs/rtwk-port/reports/rtwk-framework-gaps.md` (probes on `e3debfda`; the
status column of the rows below is updated from it). Closed: G2, G7, G13, G16,
G17, G19, G20, G25, G28, G29..G34; recipes: G14, G18. Partly: G5, G11, G12. Not a
gap or ours: G4, G10, G15, G21, G22, G23, G26, G27. Open: G1, G8, G9, G24, G35,
G36 and four new rows, N1..N4 (below, with the report's section for each).

| id | gap (details: the report section of the same id) | status |
|---|---|---|
| N1 | `TextOverflow::Ellipsis` / `EllipsisStart` drop blanks next to `…` (`rtwk-unicode/src/fit.rs:236-262`): `" All 1 "` at 6 cells gives `" All…"`, the oracle `" All …"`; workaround `agents/layout.rs` `ellipsize`, `front_truncate` | open |
| N2 | `SelectionList` item builder does not know selected / cursor (`selection_list.rs:202-213`); workaround `ListenableBuilder` per row (`agents/view.rs`) | open |
| N3 | `ThemeData` has no extension slot (like Flutter's `ThemeExtension`) | open |
| N4 | `TextWrap::Word` does not break after `/` (10 of 63 probe cases, paths and URLs; optional) | open |

## Open and by-design rows, by what blocks the port soonest

W1 = the screens and dialogs wave in progress; W2 = chat composer and
transcript (S10, S16); W4 = inline mode.

| id | gap | needed by | workaround in tui-rtwk | minimal repro | rtwk task | status |
|---|---|---|---|---|---|---|
| G16 | `show_dialog` has no on-closed message: the caller awaits the `DialogResult` future, so a spawned watcher is needed and spawning is the shell's (R1) | request presenter, every dialog whose result the model needs (W1) | was `src/shell.rs` `dialog_closed`, `src/run.rs`, `src/ui/view.rs`, `src/ui/presenter.rs` (removed) | `DialogOptions` has no callback and `DialogResult` is only `await`/`take` (`rtwk-overlay/src/dialog.rs:47-64,80-95`); see R16 below | | fixed in rtwk `9f28b7fb` (`AppCx::show_dialog` / `show_dialog_from`; `shell::dialog_closed` and `CloseWatcher` removed) |
| G17 | `run_app_with(state, view: ViewFn, RunOptions)` takes no `AppView`: no `on_start`, no way to give the overlay root the app's theme, no handle; an app that starts a shell must copy the body of `run_process` | the composition root, every run of the app (W1) | `src/run.rs:1-5` (doc), `:30` (`app_view`), `:35` (`root`, the theme on `overlay::App`), `:46-70` (`run`: terminal, frame loop, `AppHost::of_widget`, localizations, graphics scope) | `AppView::on_start` exists (`rtwk/src/app.rs:282`) but `run_app_with` (`rtwk/src/platform/app_start.rs:87`) takes a `ViewFn` and `RunOptions` (`:23-40`) has no `on_start`; see R17 | | fixed in rtwk `06cf797e` (`run_app_view`) |
| G18 | `spawn_tokio` aborts its task when the element unmounts (by design) and there is no app-lifetime task or `on_stop` / async teardown hook; the backend task is dropped at an `.await` on quit, so nothing after its loop runs | shell: backend session, `client.shutdown()` (W1) | `src/shell.rs:105` (`cx.spawn_tokio(task, ..)`), `host/src/in_process.rs:43-52` (`HandBack` returns the client from the dropped future), `host/src/main.rs:42-46` (`main` owns the shutdown) | `rtwk-tokio/src/lib.rs:50-52` states the abort; `AppView` has `on_start` but no `on_stop` (`rtwk/src/app.rs:270-300`); see R18 | | by design; recipe (app-lifetime work on the runtime, `main` owns shutdown), checked at `e3debfda` |
| G19 | `Tester` has no `AppHandle` and no tokio: `pump_widget(overlay::App::new(AppView))` gives the `AppView` no handle so `AppCx::quit` does nothing; `spawn_tokio` / `subscribe` need a runtime entered on the UI thread and `Tester` has none; `pump_app` has no way to set the overlay theme or localizations | every screen test (W1) | `tests/common/mod.rs:57-65` (`start_quittable` uses `pump_app`), `:41-49,76-80` (own runtime, `rtwk_tokio::enter`, `pump_widget` with `ui::run::root` and `localized`) | `Tester::pump_widget_for` starts `AppHost::of_widget(.., HostOptions::default())` (`rtwk-test/src/tester.rs:584-600`, no handle); `rtwk-test/Cargo.toml` has no tokio; see R19 | | fixed in rtwk `673732d1`, `3352d721` (`Tester::pump_app_in`) |
| G20 | `rtwk::style` does not re-export `Attrs` (`rtwk_core::style::Attrs`), so `Style::attrs(Attrs::BOLD)` needs a direct `rtwk-core` dependency. (`Brightness` is re-exported, `rtwk-widgets/src/style/mod.rs:25`; the earlier note that it is not was wrong, `src/theme.rs:11` can import it from `rtwk::style`.) | every screen that styles a row (W1) | `src/theme.rs:12,39,47`, `src/screens/resume/mod.rs:18,387`, `src/screens/agents/mod.rs:21,472`, `src/screens/usage/mod.rs:18,419`; dependency `Cargo.toml:11,81` | compiled in `/tmp`: `use rtwk::style::Attrs;` gives E0432 "no `Attrs` in `style`" (R20) | | fixed in rtwk `06cf797e` (`rtwk::style::{Attrs, Color}`) |
| G21 | `Scrollable` sends no initial `ScrollNotification` (only `Update` and `Overscroll`, after a move): a listener cannot know the largest offset of a page that has not moved | pager (S16, W1), any "N% / more below" indicator | `src/screens/pager/mod.rs:93-98` (state `None` until the first move), `:136` (largest offset estimated from the wrapped lines), `:216-225` | `rtwk-scroll/src/scrollable.rs:1699-1711` dispatches only `Pending::Update` / `Pending::Overscroll`; ran: 0 notifications after `pump_until_idle` (R21) | | not a gap (checked at `e3debfda`: `ScrollController` notifies on first layout) |
| G24 | `Autocomplete::on_selected` is told a pick after the text was replaced and later (a message): the pick cannot be vetoed or taken instead of inserted, so a command picked in the popup runs from the composer's text after the replacement | composer slash commands (W1, chat-nav) | `src/screens/chat/home.rs:375` (`on_selected` -> `ChatMsg::Picked`), `:222` (runs the text as it stands), `:122-126` (ignores an empty line) | `autocomplete.rs:637-648` (`pick`: `c.replace(..)`, then `selected.push`) and `:909-918` (`update` emits it) in `rtwk-editing`; see R24 | | open (narrow: pick cannot be taken instead of inserted) |
| G25 | `Suggestion` is `text` / `label` / `detail` only: no per-row `disabled`, the marker is a fixed `▎` (`autocomplete.rs:963`), match highlighting is rtwk's own `fuzzy_match` on the label, not the caller's ranges, the popup does not grow as a bottom pane (N1) | composer popups: commands, skills, mentions, files (W1) | `src/screens/chat/popup.rs` (rows turned into `Suggestion`), `parity/chat-nav.md:372-390` (N1 deltas) | `rtwk-editing/src/autocomplete.rs:42-49` (the three fields) and `:956-985` (render); see R25 | | fixed in rtwk `e16f72e0`, `0161be70` (`Suggestion::enabled/marks`, `Autocomplete::marker`) |
| G26 | `Text` takes a trailing blank off its width: a prompt `"› "` laid out in a `Row` is one cell wide | chat composer prompt (W1); any prefix glyph + space | `src/screens/chat/home.rs:113-117` (`SizedBox` of the character count) | ran in `/tmp`: `Row[Text("› "), Text("X")]` on a 10x1 `Tester` renders `"›X"`, not `"› X"` (R26) | | not a gap (checked at `e3debfda`): `Text::new("› ").wrap(TextWrap::None)` |
| G28 | `Dialog` always draws a surface with a cell of room (`PanelLook::Raised`, `rtwk-overlay-base/src/panel.rs:13-22`) or a border: no frameless / bottom-pane mode; codex-tui's approval, select and status views are frameless bottom-pane panes with a `›` marker (N01, N02) | approvals, select, status, dialogs (W1) | `src/screens/approvals/mod.rs:90,97`, `src/screens/select/mod.rs:41`, `src/screens/dialogs/mod.rs:28`, `src/screens/status/mod.rs:32` (`Dialog` with `max_width` 83/84), `parity/screens-dialogs.md:15-21,77` | `show_dialog`'s builder takes any widget and `DialogOptions` has `alignment` and `barrier_alpha` (`dialog.rs:47-64`), so an unframed widget at the bottom is possible; what is missing is a `Dialog` without surface/padding (`Dialog` fields `dialog.rs:584-606`) and a docked bottom pane that replaces the composer | | fixed in rtwk `1296f507` (`Dialog::bare`) |
| G23 | No `~` filler: `SingleChildScrollView` / `ListView` end at their content; no option to fill unused viewport rows with a glyph (D1) | pager (W1) | `tests/pager.rs:27-33` (`without_tilde` drops the `~` rows from the oracle), `parity/screens-routes.md:100-105` | `SingleChildScrollView::new(Text::new("line 5"))` in a 40x10 `Tester`: rows 2..9 are blank, the oracle has `~`; no filler option in `rtwk-scroll` | | not a gap: `LayoutBuilder` + `~` rows (cosmetic; optional `SingleChildScrollView::fill_glyph`) |
| G27 | `Focus::bind(Esc, ..)` takes the Esc before the Navigator's back key: a `Focus` binding consumes the key (`Handler<()>` has no pass-through) | chat as a page over another (W1) | `src/screens/chat/home.rs:255-259` (`Navigator::maybe_pop` by hand), `:405` | `Focus::bind` (`rtwk-framework/src/focus/widgets.rs:258`); the Navigator's back key is bound in a layer above and "a route's own bindings take the key first" (`navigator.rs:1693-1701`). `Focus::on_key(cx.keys(..).passing())` exists (`focus/keys.rs:376`) and may remove the workaround | | not a gap: `on_key(cx.keys(..).passing())`, applied |
| G13 | `show_dialog` without a Navigator above falls back to the `overlay::App` overlay, which sits above the app's providers: the dialog content cannot `watch` them (nav-proto gap 1) | dialogs (W1) | presenter under the Navigator: `src/ui/view.rs` (providers around `Navigator`, the home a memo node), `src/ui/presenter.rs` | `rtwk-overlay/src/app.rs:19-23` documents "the app's providers go above `App`"; repro was nav-proto test `q2_dialog_shown_without_a_navigator_above_is_outside_the_app_providers` (the prototype crate is no longer in `~/.cache/codexcli/research/`) | | fixed in rtwk `9f28b7fb` (`AppCx::show_dialog_from`; `ui/presenter.rs` `home_node`, `present`) |
| G14 | A provider projection lags the model until the view reruns: no same-flush read (nav-proto gap 2) | presenters (W1) | `src/ui/presenter.rs:30-33,57-63` (last shown request id, never reopen it) | nav-proto q2 first test: a handler clears `pending`, a same-flush `read::<Slice>()` still returns the old value; inherent to an inherited widget's per-run snapshot | | by design; recipe: decide on the model in `closed` (checked at `e3debfda`) |
| G15 | Changing the provider list silently drops an unkeyed Navigator's stack (nav-proto gap 3) | root (W1) | `src/app.rs:21,48` (`nav_key: GlobalKey`), `src/ui/view.rs:70` (`.with_key`), `tests/rules.rs:118` | nav-proto q5 `changing_the_provider_list_remounts..`; Flutter semantics (an element of another type at the position is remounted) | | by design (Flutter semantics); our side: `GlobalKey` in app state |
| G12 | `TextArea` lacks: atomic elements (pasted blocks, mentions), kill ring and yank, vim mode, paste-burst detection, prompt-history recall (arch-review M1) | S10 composer (W2) | none yet (composer behaviour is carried in core, `core/src/screens/composer`) | `rtwk-editing/src`: no hit for kill ring, yank, vim, paste burst, element; the only "history" is undo (`text_editing.rs:400`) | | partly: recipes exist; atomic elements (pasted blocks, mentions) still open (`TextArea`) |
| G9 | Selection, search and copy-source over a virtualized transcript: selection across items scrolled out of a `ListView` stays per widget; no `SelectionArea`-level mechanism | S10 transcript, S16 pager (W2) | none yet | `docs/design/transcript.md:80-86` in rtwk says so ("stays per widget ... until a third case justifies it, by an ADR") | | open |
| G10 | tmux probes: mouse option, pane-local graphics (only the `TMUX` / `STY` env test and DCS passthrough exist) | S10, S24 (W2/W3) | none yet | `rtwk-core/src/terminal/env.rs:183-193`, `sequences.rs:516`: no `show-options` / `display-message` query | | not a framework gap: spawning `tmux` is the app's (shell layer) |
| G11 | Clipboard breadth: only OSC 52 (`Terminal::copy_to_clipboard`); no native (pbcopy, wl-copy, xclip, clip.exe), WSL, X11 PRIMARY | copy (W2) | none yet | `rtwk-core/src/terminal/mod.rs:1313-1323`, `sequences.rs:22-36` | | partly: OSC 52 `ClipboardSelection` + `AppView::on_copy` in rtwk (`06cf797e`, `fa4e1bb7`); native copy is ours by rtwk's decision |
| G4 | Image placements inside a region, "if rtwk-rich `Image` falls short" | pets (S24), `view_image` (W3) | none yet | conditional: `rtwk-rich/src/image/widget.rs:144` exists and the presenter keeps images in a region (`docs/design/images.md`); re-test when the pets slice builds | | not a gap yet (conditional); recheck on the pets slice (`rtwk-rich` `Image`) |
| G22 | Selected-row background fill missing (D2, resume): the row is painted as spans, the fill under blanks is not reproduced | resume, agents, usage rows (W1) | `src/screens/resume/mod.rs:296-305` (marker + `Look::Bold`), `:387-395` (style map), `parity/screens-routes.md:107-110` | rtwk has `ColoredBox` (`rtwk-widgets/src/layout/container.rs:108`) and `Container::color` (`:460`): wrap the selected line in one under a stretch cross axis. A tui-rtwk follow-up, not an rtwk gap | | not a gap (our work; N2 closes part) |
| G1 | Enter / leave the alternate screen at runtime from an inline app (`ScreenMode` is fixed at start; `App::suspend` hands the terminal to another program, it does not switch modes) | inline overlays: pager, diff (W4) | none yet | `rtwk-core/src/presenter/modes.rs:62-100` (modes), `app.rs:3005-3030` (`suspend`); no `set_mode`/`enter_alt` anywhere in `rtwk-core/src` | | open |
| G2 | Re-emit the app's own committed scrollback on resize: committed rows are copied once, cut or padded to the new width (`recut_committed`), never laid out again | inline resize reflow (W4) | none yet | `rtwk-core/src/presenter/modes.rs:445-470` (docs) and `:520` (`recut_committed`); `Static` (`rtwk-widgets/src/binding/statics.rs:59`) has no resize handling | | fixed in rtwk `e3debfda` (`AppCx::clear_history`, rtwk task appcx-clear-history) |
| G7 | Committed rows keep OSC 8 links and images: links probably survive (a committed row copies style ids, and a style carries its link, `rtwk-core/src/style/table.rs:644-667`; `tests/inline.rs:152` commits a linked row but asserts text only); images do not (`copy_rows` copies cells and styles, not `Surface::images`) | inline history (W4) | none yet | `rtwk-core/src/presenter/modes.rs:515-540` (`copy_rows`), `app.rs:1314` (`CommittedRows { surface, from }`); a 10-line check in the inline slice settles the links | | fixed in rtwk `015fb54b` (links and images in committed rows) |
| G5 | Zellij and Windows Terminal scrollback strategies | inline on those terminals (W4) | none yet | no `zellij` anywhere in `rtwk-core/src`; `WT_SESSION` only in plain-terminal detection (`terminal/env.rs:76`) | | partly: Zellij closed in rtwk main (`Multiplexer::Zellij`, `hardware_scroll`); Windows Terminal unverified (a check, not code) |
| G8 | Discard pending input (typeahead quarantine before an approval) | approvals during a running turn (W2/W4) | none yet | no discard / flush in `rtwk-core` or `rtwk-input`; only `rtwk-term/src/pty/raw_tty.rs:131-143` (`restore(discard_input)`) | | open |

## Fixed or resolved in rtwk (in the pin)

| id | gap | needed by | workaround in tui-rtwk | minimal repro | rtwk task | status |
|---|---|---|---|---|---|---|
| G29 | rtwk-state `reactive_graph`: reads panic ("already been disposed") while another thread writes (issue file `rtwk-state-reactive-graph.md` §1) | family 3, not used by the port | none: the port keeps no `reactive_graph` (ADR-003) | `signal_contention` in the old `app-state-proto`: 12,524 of 153,618 reads panicked | | resolved by contract in rtwk `3c6247fe`: "the graph is the UI thread's", `Derived` reports a foreign-thread write once; documented in `rtwk-state/src/reactive.rs:1-22` and `docs/guide/state.md:108` (`reactive_graph` itself is unchanged) |
| G30 | `reactive_graph::effect::batch` gave no notification through `Derived` (§2) | same | none | `batch_repro`: two signals in a `batch`, 0 notifications | | fixed in rtwk `2c09bd07` (`rtwk-state/src/reactive.rs:38,124-127`) |
| G31 | `reactive_stores`: a keyed field written before its first read panics; keyed `Vec` push cost grows with size (§3) | same | none | `store_write_then_read` | | fixed in rtwk `2c09bd07` as documentation (the guide states the keyed-store traps; the library behaviour is unchanged) |
| G32 | A `Select` notified inside a flush wakes the frame loop again (+13 ms first frame after idle) (§4; architecture.md "rtwk-state `Select`") | ADR-003 reserve (family 4) | none | `frame_phase_after_idle` in the old `app-state-proto` | | fixed in rtwk `71ffcb7e` (a flush no longer wakes the loop for work it consumes itself) |
| G33 | `ViewCx::memo`: cache lookup grows with the square of the memo calls (81 calls 9.5 µs, 1,031 calls 648 µs) (`rtwk-viewcx-memo.md` §1) | long lists, W2 transcript | none (the port memoises rows) | `src/scale.rs` of the old `app-state-proto` | | fixed in rtwk `40352a88` (merge of `viewcx-memo-scaling`); `ViewCx::memo_keyed` added in `3bc4bfdf` (after the pin: repin to use it) |
| G34 | `ViewCx::memo`: an outer hit released the inner entries, so the next outer miss rebuilt every inner subtree (§2) | same | none | same | | fixed in rtwk `40352a88` (`tests/viewcx_memo.rs`, "an outer hit keeping its inner entries"); the issue files in `~/.cache/codexcli/rtwk-issues/` are closed by it |
| G35 | `SelectionList` keeps its `ListController` private (`rtwk-controls/src/selection_list.rs:351` `ListCore.list`, built at `:386`; only `move_to` scrolls, `:503`). The app cannot scroll the list to a cursor it moved itself through `SelectionController::set_cursor`, cannot put a heading row above the first task in view, cannot wrap in `move_selection`, and cannot add keys (j/k, ctrl-n/ctrl-p). Wanted: `SelectionList::list_controller(ListController)` (or `scroll_to_cursor`), a `wrap(bool)` and a key table | agents (command center: oracle `move_selection`, `ensure_selected_visible`, the first group heading, `j`/`k`/ctrl-n/ctrl-p of the agents key table) | `src/screens/agents/{mod,view}.rs` (UI porter, `agents-live-ui`) takes the arrow, page and home/end keys the widget owns and steps the highlight by core's `Select` for `j`/`k`/ctrl-n/ctrl-p/ctrl-j/ctrl-k (it only steps one row; page keys `ctrl-f`/`ctrl-b`, wrap-around and page keys while the search line has the focus are not reachable: tests `live_center_page_keys_and_wrap_follow_the_list_keymap` and `live_center_pages_visible_rows_while_reconnecting` are pending on it, `parity/agents-live.md`, UI section) | `SelectionList::new(..).controller(c)`, then `c.set_cursor(Some(far_key))` in a 5-row tester: the row stays out of view (no handle for `ensure_index_visible`) | | open |
| G36 | `TabBar` cannot draw the oracle's filled tab bar (the center's filter row, `agent_center/render.rs` with `tabs`): (a) its tabs touch (`starts[i+1]` is the next tab's start, `rtwk-controls/src/tabs.rs:915-935`), the oracle's are cut by one unfilled cell, so the selected tab's fill ends before the blank that separates it; (b) the edge marks `‹`/`›` are painted over the first and last cell of the row (`tabs.rs:960`) where the oracle keeps two cells for them outside the tabs (`left` and `right` in `layout::strip`); (c) it scrolls by a column `offset` moved by the wheel and the marks, the oracle shows the whole tabs of a window chosen by `occupied(start, end) <= width` around the active one and clips the rest; (d) the row has the theme's text style, the oracle's active tab is `selection` + bold on a plain row. Wanted: a `TabBar::gap(1)`, `TabBar::marks_outside(true)` and a whole-tab window mode (or a `TabStrip` render object taking a `Strip`-like layout) | agents (command center filters row, snapshots `agents_overview*`, `live_center_*`, 5 tabs at 40..160 cells) | `src/screens/agents/tabs.rs` (`strip`, the oracle window as a pure function unit-tested at 44 and 76 cells, and a `Row` of `Text::line` tabs with a `GestureDetector` per tab): the keys are the center's own (Tab/Shift+Tab cycle the filter in core), so the bar needs no focus | run (agents-view-rest, `tests/agents.rs` `agents_overview_tab_strip_draws_oracle_filled_tabs_on_tabbar`, pending(G36), KIT_PROBE_PENDING=1): `TabBar::new(controller, ["All 2", "Needs you 1", "Working 0", "Ready 0", "Inactive 1"])` at 96 cells draws ` All 1  Needs you 0  Working 0  Ready 1  Inactive 0`-shaped rows (tabs one blank apart, no fill gap; oracle `   All 2   Needs you 1   Working 0   Ready 0   Inactive 1`); read from the source: a `TabBar::new(controller, ["All 1", "Needs you 0", "Working 0", "Ready 1", "Inactive 0"])` paints each tab as its label with a blank on either side, one right after the other (`tabs.rs:915-935`), and at a width under their sum the marks overwrite the row's end cells (`tabs.rs:960`); the oracle's snapshot `agents_overview_attention` row 1 is `  All 1   Needs you 0   Working 0   Ready 1   Inactive 0` with each tab ` label ` apart by one cell | | open |
| N1 | `TextOverflow::Ellipsis` / `EllipsisStart` drop the blanks next to the `…` (`rtwk-unicode/src/fit.rs:236-262` `skip_blanks`, `:46` `is_cut_blank`); the oracle keeps them (`truncate_line_with_ellipsis_if_overflow`, `line_truncation.rs:79-105`; `front_truncate`, `text_formatting.rs:152-170`) | agents (clipped filter tab at 12 cells; every row title, heading and details line cut with an ellipsis) | none: the view uses `Text::line` / `Text::rich_line` as they are; `older_server_notice_truncates_in_narrow_overview` is `pending(Rtwk)` | `Text::line(" All 1 ")` in 6 cells: rtwk ` All…`, oracle ` All …` (tests/agents.rs, `older_server_narrow_overview` row 1: oracle `  All … ›`, ours `  All…  ›`) | | open |
| G37 | No `TextOverflow` mode cuts a path by its segments: first segments and the last two kept, the middle ones behind a `…` (oracle `center_truncate_path`, `text_formatting.rs`; the group heading of the list) | agents (heading of a directory group) | `src/screens/agents/path.rs`, one function with its helpers, held to the oracle's four tests; allowlisted in R15 (`tests/rules.rs`) | `TextOverflow::EllipsisMiddle` on `~/hello/the/fox/is/very/fast` in 24 cells cuts inside segments, the oracle gives `~/hello/the/…/very/fast` | | open |
| G38 | The app cannot learn how many rows a `Text` takes at a width: the details pane keeps two rows of the prompt and puts an elision on its last visible row, so it needs the rows before it draws. `rtwk_render::objects::text::break_lines` is the engine `Text` uses, public in `rtwk-render`, but the facade (`rtwk::`) does not export it, so the crate depends on `rtwk-render` directly (own commit) | agents (details pane, oracle `render_details`) | `src/screens/agents/soft.rs`, fifteen lines over `break_lines`; allowlisted in R15 | `break_lines(&TextSegmenter::default(), text, 38, TextWrap::Word)` gives the rows `Text` draws at 38 cells | | open: facade export of `break_lines` (or a measure on `Text`) |
| G39 | A selectable list of rows with a shared content-sized label column, wrapped variable-height description cells and a pane that is as tall as its rows up to the room there is (then scrolls) cannot be built: `Table` is a `ListView`, so inside a `Column` it is as tall as the room, not its rows, and its intrinsic height is 0 (lazily built rows); `Table::row_height(None)` gives rows of the tallest cell but only when the height is given from outside; `SelectionList` items are independent (no shared label column, `SelectionController` has no movement API); `Grid` has the shared tracks and wrapped rows of any height (content-sized) but no cursor, no keys, no scroll | approvals (`request_user_input` options: long and wrapped options, tight height, scrolling options with the `option 4/5` footer prefix) and select (`approvals_selection_popup`, `list_selection_model_picker_width_80`): the oracle's `ListSelectionView`/`request_user_input` rows `› 1. label  description` with the label and the description each wrapped in its own column | `src/screens/approvals/dialog.rs` and `src/screens/select/dialog.rs` keep a `SizedBox` of a computed row count (one line per row, labels cut with an ellipsis): both stay in R15_NOT_YET | run (rework-dialogs, `tests/oracle/approvals_2.rs` `request_user_input_long_option_text_snapshot`, pending(G39), KIT_PROBE_PENDING=1): the options `Table` with `row_height(None)`, no `SizedBox`: the table fills the room and pushes the hint off the pane; the same inside `IntrinsicHeight`: the table is 0 rows tall and the options vanish; with the fixed `SizedBox(count)` (kept): rows one line, `1. Job: running/completed/failed/expired; Run/Experiment: succeeded/failed/unknown (Recommended when triaging lon…` where the oracle wraps the label at 78 cells and the description in the right column (6 rows for 2 options) | | open: a content-sized `Table` (`shrink_wrap`, height = its rows up to the room) or a selection/cursor on `Grid` rows |
| G40 | No gauge draws a bar of whole cells with a filled and an unfilled glyph: the oracle's limit bar is `[██████░░░░░░░░░░░░░░]`, 20 cells, `round(left * 20 / 100)` of them `█` and the rest `░` (`status/helpers.rs` `render_limit_progress_bar`); `Gauge` draws partial eighths (`▋`) and no unfilled glyph (`rtwk-data/src/gauge.rs`), `LineGauge::segmented` a column a cell `▍` (`gauge.rs:317`) | status card (S19, `status_snapshot_includes_reasoning_details`: `5h limit:  [██████░░░░░░░░░░░░░░] 28% left (resets 3:14 AM)`) | `src/screens/status/format.rs` `progress_bar` (`"█".repeat`), the bar is a text inside the localized value; in R15_NOT_YET | run (rework-dialogs, `tests/status.rs` `limit_bar_is_a_rtwk_gauge_of_whole_cells`, ignored as pending(G40), `--run-ignored all`): `Row[Text("["), SizedBox(20,1, Gauge::new(0.28)), Text("] 28% left")]` draws `[█████▋              ] 28% left` and with `LineGauge::new(0.28).segmented()` `[▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍▍] 28% left`; the oracle `[██████░░░░░░░░░░░░░░] 28% left` | | open: a `Gauge::cells(filled, unfilled)` glyph pair (whole cells, rounded) |

## Repros

Each is short and meant to be pasted into a test of the named rtwk crate. R20,
R21 and R26 were run or compiled against rtwk `d47446d1` in a scratch crate
(`/tmp/rtwk-gaps-repro`, outside the rtwk repo); the others are sketches read
from the source and say what the code shows.

**R16 (G16)** crate `rtwk-overlay`. Wanted: a message on close.
```rust
// today: the only ways to hear the close
let result = show_dialog::<bool, _>(cx, |_| Dialog::new(Text::new("ok?")).into(), DialogOptions::default());
cx.spawn(async move { let _ = result.await; Some(Msg::Closed) }); // needs a spawn in the caller
// wanted: DialogOptions::default().on_closed(cx.send(Msg::Closed)) or result.then(msg)
```

**R17 (G17)** crate `rtwk`. Wanted: `run_app_with` over an `AppView`.
```rust
let view = AppView::new(state, view_fn).on_start(start); // start runs as the root mounts
// run_app_with(view, RunOptions::default()) does not exist: it is run_app_with(state, view_fn, options)
```

**R18 (G18)** crate `rtwk-tokio`. Quit unmounts and aborts; nothing runs after the awaited loop.
```rust
cx.spawn_tokio(async { let _guard = Flag; loop { tick().await } }, |_, (), _| {});
// on quit the future is dropped at `tick().await`: code after the loop never runs.
// wanted: AppView::on_stop(async/sync hook) or a task that outlives its element until the app ends.
```

**R19 (G19)** crate `rtwk-test`.
```rust
let mut t = Tester::with(Options::new().size(40, 5));
t.pump_widget(overlay::App::new(AppView::new(app, view))); // AppHandle: none -> cx.quit() does nothing
// and: spawn_tokio in the app panics/does nothing without rtwk_tokio::enter(&runtime) on this thread
// wanted: Tester::pump_app with the overlay root (theme, localizations) and a tokio runtime option
```

**R20 (G20)** compiled: `use rtwk::style::Attrs;` is E0432 ("no `Attrs` in `style`"). Wanted: `pub use rtwk_core::style::Attrs;` in `rtwk-widgets/src/style/mod.rs`.

**R21 (G21)** crate `rtwk-scroll`; ran: prints 0 and the assertion holds (the gap).
```rust
let seen = Rc::new(Cell::new(0));
let s = seen.clone();
let body: AnyWidget = NotificationListener::<ScrollNotification>::new(
    SingleChildScrollView::new(Text::new((0..30).map(|i| format!("l{i}")).collect::<Vec<_>>().join("\n"))))
    .on_notification(move |_| { s.set(s.get() + 1); false }).into();
let mut t = Tester::with(Options::new().size(10, 5));
t.pump_widget(body);
t.pump_until_idle();
assert_eq!(seen.get(), 0); // wanted: >= 1, with the metrics (pixels 0, max_scroll_extent 25), as Flutter's on layout
```

**R24 (G24)** crate `rtwk-editing`. In `Autocomplete::pick` the text is replaced before `on_selected` is queued.
```rust
Autocomplete::new(controller, |before: &str| suggestions(before), area)
    .on_selected(cx.map(|s: Suggestion| Msg::Picked(s.text))); // Msg arrives after c.replace(..)
// wanted: a hook that takes the pick instead of inserting it (on_pick returning handled)
```

**R25 (G25)** crate `rtwk-editing`. `Suggestion::new("/app").label("/app").detail("continue this session")` has no `.disabled(true)`, `.match_ranges(..)` or marker style; a 72x2 popup renders `▎/app ...` where the oracle has `› /app ...`.

**R26 (G26)** crate `rtwk-widgets`; ran: the row is `"›X"`.
```rust
let mut t = Tester::with(Options::new().size(10, 1));
t.pump_widget(Row::new().child(Text::new("› ")).child(Text::new("X")));
assert_eq!(t.screen().trim_end(), "› X"); // today "›X"
```

**R28 (G28)** crate `rtwk-overlay`. `Dialog::new(child)` has no `frameless()` / `padding(0)` + no-surface; `PanelLook` has only `Raised` and `Bordered` (`panel.rs:13-22`). Wanted: `PanelLook::None` or a `Dialog::bare()`.

**R23 (G23)** crate `rtwk-scroll`. `SingleChildScrollView::new(Text::new("line 5"))` in a 40x10 tester: rows below line 5 are blank. Wanted: `.fill_glyph("~")` on the viewport.

**R12 (G12)**, **R9**, **R10**, **R11**, **R1**, **R2**, **R5**, **R8**: feature requests with no snippet; the "minimal repro" column names the code that shows the absence, and the codex-tui behaviour to match is the oracle row of the slice that needs it (S10 composer for G12, S10/S16 for G9, inline mode S27 for G1, G2, G5, G7).
