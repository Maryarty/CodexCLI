# Which rtwk widget for which codex-tui need

The rows are what the oracle (`codex-rs/tui`, ratatui) draws or does, and the rtwk widget that carries it
in `tui-rtwk`. A name in backticks with a path is checked against the pin when this file is generated; a
widget's options, builder methods and example are in `widgets/<group>.md` (`widgets.md` lists the groups).
The concept map of what changes between the programs is `docs/rtwk-port/process.md` Part 1, section 2.

## Screens, dialogs, overlays

| codex-tui | rtwk | note |
| --- | --- | --- |
| a screen of the app (chat, pager, diff, analytics overlay) | [[Navigator]] with a [[PageRoute]] pushed over the chat screen | `widgets/navigation.md`; `guides/recipes.md`: push and result |
| a `BottomPaneView` that asks and completes (selection, approval, elicitation, input) | `show_dialog` with a [[Dialog]] and `close_dialog` | `widgets/overlays.md`; recipe: a request in the model |
| a popup list under the composer (slash commands, file search, mentions) | a [[Popover]] anchored to the composer, with an [[OptionList]] or [[SelectionList]] inside (a [[CommandPalette]] is a whole-app overlay with its own search field, not a popup under a composer) | `widgets/overlays.md`, `widgets/controls.md` |
| the generic selection dialog (`ListSelectionView`) | [[Dialog]] over a [[SelectionList]] (single) or [[MultiSelect]] (many) | the slice S14 is the worked example in process.md section 4 |
| a context or right-click menu | [[ContextMenu]], [[MenuBar]] | |
| a toast or a transient notice | `show_toast` with [[ToastOptions]] | |
| a modal barrier behind a dialog | [[ModalBarrier]] (a `Dialog` already has one) | |
| a floating window | [[WindowLayer]] with [[Window]] | |
| a hint on hover | [[Tooltip]] | |

## Input

| codex-tui | rtwk | note |
| --- | --- | --- |
| `ChatComposer` with the textarea | [[TextArea]] over a retained [[TextEditingController]] | recipe: TextArea with a retained controller |
| a one-line input (search box, rename) | [[TextField]] with a controller, or [[TextFormField]] in a [[Form]] | `widgets/editing.md`, `widgets/controls.md` |
| a yes/no confirm line | [[ConfirmInput]] | |
| a number, date or file input | [[NumberInput]], [[DatePicker]], [[FilePicker]] | |
| completion list for the composer | [[Autocomplete]] | |
| the keymap and `handle_key_event` arms | [[Shortcuts]] and `Actions`, or [[KeyBindings]] (with [[LeaderKey]] for sequences) around a [[Focus]] | `widgets/focus-and-keys.md` |
| focus order, focus traps | [[Focus]], [[FocusScope]], [[ExcludeFocus]] | `guides/app-api.md` for `FocusNode` |
| mouse clicks and drags | [[GestureDetector]], [[MouseRegion]], [[Pressable]] | `widgets/mouse.md` |
| a button | [[Button]] | |
| checkboxes, radio rows, toggles | [[Checkbox]], [[Switch]], [[RadioGroup]], [[SegmentedButton]] | |

## Content

| codex-tui | rtwk | note |
| --- | --- | --- |
| styled lines and spans (`Line`, `Span`, `Style`) | [[Text]] (`Text::rich` over [[Span]]s) with [[Style]] tokens from [[ThemeData]] | `widgets/text.md`, `widgets/style.md`; no colours in screens |
| a bordered block with titles | [[BoxWidget]] with `Border` and `BorderTitle` | `widgets/layout.md` |
| rows and columns, `Layout::split` with constraints | [[Row]], [[Column]], [[Flex]] with [[Expanded]] and [[Spacer]] children | a `Column` centres across: `cross_axis_alignment` |
| padding, fixed size, min and max | `Padding`, [[SizedBox]], [[ConstrainedBox]], [[Align]] | |
| stacked or overlapping areas | [[Stack]] with [[Positioned]] | |
| a horizontal rule | [[Divider]], [[VerticalDivider]] | |
| the transcript (history cells) | [[ListView]] (`ListView::builder`, `stick_to_bottom`) with one widget per cell | recipe: ListView::builder over a model |
| markdown in a history cell | [[MarkdownView]] | `widgets/rich.md`; `guides/markdown.md` |
| syntax-highlighted code | [[CodeView]], [[SyntaxThemeScope]] | |
| a diff (`diff_render`) | [[DiffView]] | |
| JSON output | [[JsonView]] | |
| ANSI output of a command | [[AnsiStream]] | `widgets/text.md` |
| large banner text | [[BigText]] | |
| a log pane | [[LogView]] | `widgets/scrolling.md` |
| a pager (`pager_overlay`) | [[ListView]] or [[SingleChildScrollView]] in a [[PageRoute]]; [[Scrollbar]] for the bar | |
| text selection and copy | [[SelectionArea]] | the selection over a virtualized list is gap G9 |
| a table (usage, rate limits, analytics) | [[Table]] with a [[TableController]] | `widgets/data.md` |
| a tree (file list, tasks) | [[TreeView]], [[TreeTable]], [[DirectoryTree]], [[TaskList]] | |
| label/value pairs (status card) | [[DescriptionList]] | |
| tabs | [[TabBar]] with [[TabBarView]] | |
| a terminal inside the UI | [[TerminalView]] (rtwk-term) | |
| an image | [[Image]] (rtwk-rich) | |

## Status and feedback

| codex-tui | rtwk | note |
| --- | --- | --- |
| a spinner or the shimmering status | [[Spinner]]; [[Skeleton]] for placeholders | `widgets/animation.md`; a spinner makes `pump_until_idle` fail: pump by duration |
| progress | [[ProgressBar]], [[ProgressList]] | |
| the footer with key hints | [[KeyHintsBar]] | `widgets/controls.md` |
| the status line | [[StatusBar]] | |
| an inline status or warning message | [[StatusMessage]], [[Badge]], [[Callout]] | |
| a hyperlink | [[Hyperlink]] | |
| a list row with title and detail | [[ListTile]] | |
| breadcrumbs, pagination, steps | [[Breadcrumbs]], [[Pagination]], [[Stepper]] | `widgets/navigation.md` |

## State and the app

| codex-tui | rtwk | note |
| --- | --- | --- |
| `App`, `ChatWidget` fields | the plain model `A` of `run_app_with(state, view, RunOptions)`; the view is `view(&A, &mut ViewCx<A>)` | `guides/app-api.md`, `guides/state.md` |
| values recomputed and cached | `ViewCx::memo(deps, build)`, never kept in sync by hand | |
| shared data below a point | [[Provider]], [[MultiProvider]], `select` / `watch` / `read` | recipe: provider over a projection |
| a reactive value | [[Obs]] | `guides/state.md` |
| an async result | `AppCx::spawn`, `AppCx::channel`, `rtwk_tokio::TokioApp::spawn_tokio` with a ticket | recipe: channel and ticket |
| a rebuild when a listenable changes | [[ListenableBuilder]], [[ValueListenableBuilder]] | `widgets/foundation.md` |
| a widget from the constraints | [[LayoutBuilder]] | `widgets/framework.md` |
| a custom widget with state | `StatefulWidget` and `State` with `impl_widget!(stateful eq ..)` | `guides/app-api.md` |
| a tick or an animation | [[Ticker]], [[AnimatedSwitcher]], the animation widgets | `widgets/animation.md` |
| desktop notification, terminal title, inline or fixed viewport | `widgets/app.md` (`AppBinding::notify` or `TerminalStatus::notify`, [[TerminalTitle]], `Viewport`) | |
| tests: `insta` snapshots, `render_lines_with_width` | `rtwk_test::Tester`, `assert_screen`, `screen()` | recipe: a screen test; `guides/testing-api.md` |
