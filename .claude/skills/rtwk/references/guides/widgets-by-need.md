<!-- Written by tools/rtwk-port/skill/generate.py from rtwk c953f062; do not edit. Source: tools/rtwk-port/skill/widgets-by-need.src.md. -->
# Which rtwk widget for which codex-tui need

The rows are what the oracle (`codex-rs/tui`, ratatui) draws or does, and the rtwk widget that carries it
in `tui-rtwk`. A name in backticks with a path is checked against the pin when this file is generated; a
widget's options, builder methods and example are in `widgets/<group>.md` (`widgets.md` lists the groups).
The concept map of what changes between the programs is `docs/rtwk-port/process.md` Part 1, section 2.

## Screens, dialogs, overlays

| codex-tui | rtwk | note |
| --- | --- | --- |
| a screen of the app (chat, pager, diff, analytics overlay) | `rtwk::navigation::Navigator` with a `rtwk::navigation::PageRoute` pushed over the chat screen | `widgets/navigation.md`; `guides/recipes.md`: push and result |
| a `BottomPaneView` that asks and completes (selection, approval, elicitation, input) | `show_dialog` with a `rtwk::overlay::Dialog` and `close_dialog` | `widgets/overlays.md`; recipe: a request in the model |
| a popup list under the composer (slash commands, file search, mentions) | a `rtwk::overlay::Popover` anchored to the composer, with an `rtwk::overlay::OptionList` or `rtwk::controls::SelectionList` inside (a `rtwk::overlay::CommandPalette` is a whole-app overlay with its own search field, not a popup under a composer) | `widgets/overlays.md`, `widgets/controls.md` |
| the generic selection dialog (`ListSelectionView`) | `rtwk::overlay::Dialog` over a `rtwk::controls::SelectionList` (single) or `rtwk::controls::MultiSelect` (many) | the slice S14 is the worked example in process.md section 4 |
| a context or right-click menu | `rtwk::overlay::ContextMenu`, `rtwk::overlay::MenuBar` | |
| a toast or a transient notice | `show_toast` with `rtwk::overlay::ToastOptions` | |
| a modal barrier behind a dialog | `rtwk::overlay::ModalBarrier` (a `Dialog` already has one) | |
| a floating window | `rtwk::overlay::WindowLayer` with `rtwk::overlay::Window` | |
| a hint on hover | `rtwk::overlay::Tooltip` | |

## Input

| codex-tui | rtwk | note |
| --- | --- | --- |
| `ChatComposer` with the textarea | `rtwk::editing::TextArea` over a retained `rtwk::editing::TextEditingController` | recipe: TextArea with a retained controller |
| a one-line input (search box, rename) | `rtwk::editing::TextField` with a controller, or `rtwk::controls::TextFormField` in a `rtwk::controls::Form` | `widgets/editing.md`, `widgets/controls.md` |
| a yes/no confirm line | `rtwk::controls::ConfirmInput` | |
| a number, date or file input | `rtwk::controls::NumberInput`, `rtwk::controls::DatePicker`, `rtwk::controls::FilePicker` | |
| completion list for the composer | `rtwk::editing::Autocomplete` | |
| the keymap and `handle_key_event` arms | `rtwk::focus::Shortcuts` and `Actions`, or `rtwk::focus::KeyBindings` (with `rtwk::focus::LeaderKey` for sequences) around a `rtwk::focus::Focus` | `widgets/focus-and-keys.md` |
| focus order, focus traps | `rtwk::focus::Focus`, `rtwk::focus::FocusScope`, `rtwk::focus::ExcludeFocus` | `guides/app-api.md` for `FocusNode` |
| mouse clicks and drags | `rtwk::gestures::GestureDetector`, `rtwk::gestures::MouseRegion`, `rtwk::controls::Pressable` | `widgets/mouse.md` |
| a button | `rtwk::controls::Button` | |
| checkboxes, radio rows, toggles | `rtwk::controls::Checkbox`, `rtwk::controls::Switch`, `rtwk::controls::RadioGroup`, `rtwk::controls::SegmentedButton` | |

## Content

| codex-tui | rtwk | note |
| --- | --- | --- |
| styled lines and spans (`Line`, `Span`, `Style`) | `rtwk::text::Text` (`Text::rich` over `rtwk::style::Span`s) with `rtwk::style::Style` tokens from `rtwk::style::ThemeData` | `widgets/text.md`, `widgets/style.md`; no colours in screens |
| a bordered block with titles | `rtwk::layout::BoxWidget` with `Border` and `BorderTitle` | `widgets/layout.md` |
| rows and columns, `Layout::split` with constraints | `rtwk::layout::Row`, `rtwk::layout::Column`, `rtwk::layout::Flex` with `rtwk::layout::Expanded` and `rtwk::layout::Spacer` children | a `Column` centres across: `cross_axis_alignment` |
| padding, fixed size, min and max | `Padding`, `rtwk::layout::SizedBox`, `rtwk::layout::ConstrainedBox`, `rtwk::layout::Align` | |
| stacked or overlapping areas | `rtwk::layout::Stack` with `rtwk::layout::Positioned` | |
| a horizontal rule | `rtwk::layout::Divider`, `rtwk::layout::VerticalDivider` | |
| the transcript (history cells) | `rtwk::scroll::ListView` (`ListView::builder`, `stick_to_bottom`) with one widget per cell | recipe: ListView::builder over a model |
| markdown in a history cell | `rtwk_rich::markdown::MarkdownView` | `widgets/rich.md`; `guides/markdown.md` |
| syntax-highlighted code | `rtwk_rich::highlight::CodeView`, `rtwk_rich::highlight::SyntaxThemeScope` | |
| a diff (`diff_render`) | `rtwk_rich::diff::DiffView` | |
| JSON output | `rtwk_rich::json::JsonView` | |
| ANSI output of a command | `rtwk::ansi::AnsiStream` | `widgets/text.md` |
| large banner text | `rtwk_rich::bigtext::BigText` | |
| a log pane | `rtwk::scroll::LogView` | `widgets/scrolling.md` |
| a pager (`pager_overlay`) | `rtwk::scroll::ListView` or `rtwk::scroll::SingleChildScrollView` in a `rtwk::navigation::PageRoute`; `rtwk::scroll::Scrollbar` for the bar | |
| text selection and copy | `rtwk::gestures::SelectionArea` | the selection over a virtualized list is gap G9 |
| a table (usage, rate limits, analytics) | `rtwk::data::Table` with a `rtwk::data::TableController` | `widgets/data.md` |
| a tree (file list, tasks) | `rtwk::data::TreeView`, `rtwk::data::TreeTable`, `rtwk::data::DirectoryTree`, `rtwk::data::TaskList` | |
| label/value pairs (status card) | `rtwk::data::DescriptionList` | |
| tabs | `rtwk::controls::TabBar` with `rtwk::controls::TabBarView` | |
| a terminal inside the UI | `rtwk_term::TerminalView` (rtwk-term) | |
| an image | `rtwk_rich::image::Image` (rtwk-rich) | |

## Status and feedback

| codex-tui | rtwk | note |
| --- | --- | --- |
| a spinner or the shimmering status | `rtwk::animation::Spinner`; `rtwk::animation::Skeleton` for placeholders | `widgets/animation.md`; a spinner makes `pump_until_idle` fail: pump by duration |
| progress | `rtwk::animation::ProgressBar`, `rtwk::animation::ProgressList` | |
| the footer with key hints | `rtwk::controls::KeyHintsBar` | `widgets/controls.md` |
| the status line | `rtwk::controls::StatusBar` | |
| an inline status or warning message | `rtwk::controls::StatusMessage`, `rtwk::controls::Badge`, `rtwk::controls::Callout` | |
| a hyperlink | `rtwk::controls::Hyperlink` | |
| a list row with title and detail | `rtwk::controls::ListTile` | |
| breadcrumbs, pagination, steps | `rtwk::navigation::Breadcrumbs`, `rtwk::navigation::Pagination`, `rtwk::navigation::Stepper` | `widgets/navigation.md` |

## State and the app

| codex-tui | rtwk | note |
| --- | --- | --- |
| `App`, `ChatWidget` fields | the plain model `A` of `run_app_with(state, view, RunOptions)`; the view is `view(&A, &mut ViewCx<A>)` | `guides/app-api.md`, `guides/state.md` |
| values recomputed and cached | `ViewCx::memo(deps, build)`, never kept in sync by hand | |
| shared data below a point | `rtwk::provider::Provider`, `rtwk::provider::MultiProvider`, `select` / `watch` / `read` | recipe: provider over a projection |
| a reactive value | `rtwk::Obs` | `guides/state.md` |
| an async result | `AppCx::spawn`, `AppCx::channel`, `rtwk_tokio::TokioApp::spawn_tokio` with a ticket | recipe: channel and ticket |
| a rebuild when a listenable changes | `rtwk::foundation::ListenableBuilder`, `rtwk::foundation::ValueListenableBuilder` | `widgets/foundation.md` |
| a widget from the constraints | `rtwk::LayoutBuilder` | `widgets/framework.md` |
| a custom widget with state | `StatefulWidget` and `State` with `impl_widget!(stateful eq ..)` | `guides/app-api.md` |
| a tick or an animation | `rtwk::animation::Ticker`, `rtwk::animation::AnimatedSwitcher`, the animation widgets | `widgets/animation.md` |
| desktop notification, terminal title, inline or fixed viewport | `widgets/app.md` (`AppBinding::notify` or `TerminalStatus::notify`, `rtwk::binding::TerminalTitle`, `Viewport`) | |
| tests: `insta` snapshots, `render_lines_with_width` | `rtwk_test::Tester`, `assert_screen`, `screen()` | recipe: a screen test; `guides/testing-api.md` |
