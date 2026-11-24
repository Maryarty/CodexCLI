<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# rtwk widgets

Every widget, one line each: its name, the facade path it is public at and what it is, by group. A group's file has each widget's definition, options and the Rust that builds it.

## Text

Paragraphs and spans of styled text, wrapped, cut off, aligned, and right to left. File: `widgets/text.md`.

- Text (`rtwk::text::Text`): A paragraph in one style, its lines broken to fit, cut off and aligned as asked.
- Rich text (`rtwk::style::Span`, `rtwk::text::Text`): Spans in styles of their own: attributes, colors and links, 'text'.bold.fg(…) for short.
- DefaultTextStyle (`rtwk::text::DefaultTextStyle`): The style of the text below it, which a Text merges its own over.
- Right to left (`rtwk::render::BidiText`): Hebrew and Arabic among left-to-right text: put in the order it reads where the app reorders it (TextOrder::Visual), left as stored where the terminal reorders it itself.
- ANSI (`rtwk::ansi::AnsiStream`, `rtwk::ansi::parse_ansi`): Output with ANSI colour codes, from a program or a log, as styled spans: all at once with parse_ansi, or as it streams in with AnsiStream.

## Layout

Boxes, rows and columns, stacks, grids and wraps: where a child goes and how much room it gets. File: `widgets/layout.md`.

- Box (`rtwk::layout::BoxWidget`, `rtwk::layout::Border`): A border, titles on it, padding and a background around a child.
- ColoredBox, DecoratedBox (`rtwk::layout::ColoredBox`, `rtwk::layout::DecoratedBox`): A child's cells filled with a color, and a BoxDecoration (a fill, a border, titles) drawn under the child or over its edges.
- Container (`rtwk::layout::Container`): Align, Padding, a color or a decoration, a size and a margin put together around one child.
- Divider (`rtwk::layout::Divider`): A line across, and a VerticalDivider down; merged, they join the lines drawn before them where they meet.
- Shadow (`rtwk::layout::Shadow`): A drop shadow: the cells beside and below a child, moved by its offset, what is under them darkened.
- Padding (`rtwk::layout::Padding`): Empty cells around a child.
- Align, Center (`rtwk::layout::Align`, `rtwk::layout::Center`): A child placed in the room it is given; Center in the middle of it.
- SizedBox (`rtwk::layout::SizedBox`): A child of a size: SizedBox::expand fills the room, SizedBox::shrink takes none.
- ConstrainedBox (`rtwk::layout::ConstrainedBox`): Constraints a child is laid out within, merged with those it is given.
- LimitedBox, OverflowBox (`rtwk::layout::LimitedBox`, `rtwk::layout::OverflowBox`): Other constraints than the room gives: LimitedBox limits a child only where the room has no limit, OverflowBox and SizedOverflowBox give it others in its place, cut off where it sticks out.
- FractionallySizedBox (`rtwk::layout::FractionallySizedBox`): A child a fraction of the room it is given.
- IntrinsicWidth/Height (`rtwk::layout::IntrinsicWidth`, `rtwk::layout::IntrinsicHeight`): A child as wide or as tall as it would like to be: stretched items all as wide as the widest, boxes all as tall as the tallest.
- Row, Column, Flex (`rtwk::layout::Row`, `rtwk::layout::Column`, `rtwk::layout::Flex`): Children one after another along an axis, aligned along and across it.
- Expanded, Spacer (`rtwk::layout::Expanded`, `rtwk::layout::Spacer`): Children of a Row or Column that share the room the others leave by their flex factors: Expanded fills its share, a Flexible may take less, a Spacer is empty.
- Stack, Positioned (`rtwk::layout::Stack`, `rtwk::layout::Positioned`): Children over one another, pinned to the edges or aligned.
- Transform (`rtwk::layout::Transform`): A child shown moved by whole cells, laid out where it was: Transform::translate, the one transform a terminal has.
- Visibility, Offstage (`rtwk::layout::Visibility`, `rtwk::layout::Offstage`): A child hidden: Offstage keeps it laid out and running without painting it; Visibility takes its room away too, or keeps it.
- ExpansionTile, Accordion (`rtwk::controls::ExpansionTile`, `rtwk::controls::Accordion`): Sections that fold under their headers: Enter, Space or a click opens one, Right and Left open and close it; in an accordion one is open at a time, Up and Down go from header to header.
- Grid, GridPlacement (`rtwk::layout::Grid`, `rtwk::layout::GridPlacement`): Children in the cells of tracks: fixed, a share of the rest, or as long as their content; a child may span several.
- Wrap (`rtwk::layout::Wrap`): Children in runs that go on in the next run where they would not fit: tags, buttons and key hints on a terminal that narrows.
- CustomPaint (`rtwk::layout::CustomPaint`): Cells a CellPainter draws itself, under the child or over it, painted again when what it paints changes.
- SplitView (`rtwk::layout::SplitView`): Panes the mouse or the arrows resize by their dividers; Tab focuses one. The last one collapses.
- DockArea (`rtwk::controls::DockArea`): Panels in splits and tabs: a tab dragged onto a target of a group docks its panel there, and dropped elsewhere floats it; a window's title bar dragged onto one docks it again.
- Scaffold (`rtwk::navigation::Scaffold`): An app's frame: a header, a sidebar beside the body, a status bar and the key hints, each part a zone of the focus; the sidebar collapsed on a narrow screen and resized by a drag.

## Style

Themes, the terminal's brightness, and colors that run across a background or text. File: `widgets/style.md`.

- Theme (`rtwk::style::Theme`): The styles of an app by what they are for, which the widgets below it draw in: a preset, or the terminal's own colors.
- TerminalBrightness (`rtwk::style::TerminalBrightness`): Whether the terminal is dark or light, as it reports it; Theme::adaptive picks a dark or a light theme by it.
- GradientBackground (`rtwk::style::GradientBackground`): A background whose color changes from cell to cell, along the columns or down the rows, from stop to stop.
- GradientForeground (`rtwk::style::GradientForeground`): Text whose color changes from glyph to glyph along each line, or from line to line down the rows.

## Scrolling

Lists, scroll views and scrollbars, built as they scroll into view. File: `widgets/scrolling.md`.

- SingleChildScrollView (`rtwk::scroll::SingleChildScrollView`): One child scrolled in the room it is given: Tab to it, then the arrows, PageUp and PageDown, or the wheel over it.
- Scrollable (`rtwk::scroll::Scrollable`): What scrolls a viewport: here one of the page's own, a ruler that tells the scrollable how long it is as it is laid out.
- ListView (`rtwk::scroll::ListView`): Items built as they come into view, however many there are; the wheel and the keys scroll it. Separated, reversed as a chat, or kept on what it shows as items come before it, or selected with the mouse as one text.
- LogView (`rtwk::scroll::LogView`): Lines that keep coming, followed at the bottom: levels, a filter and a search, a count of the new lines while it is scrolled up.
- Scrollbar (`rtwk::scroll::Scrollbar`): A bar by the scrollable below it, its thumb where the window is; the thumb dragged and the track clicked scroll it.

## Data

Tables, trees, charts, gauges, sparklines and calendars. File: `widgets/data.md`.

- Sparkline (`rtwk::data::Sparkline`): A series of values as bars an eighth of a cell fine, the newest on the right.
- Gauge, LineGauge (`rtwk::data::Gauge`, `rtwk::data::LineGauge`): How much of something there is: a block filled to a ratio, or a line.
- BarChart (`rtwk::data::BarChart`): Bars in groups, up or across, their values on them and their labels under them.
- Chart (`rtwk::data::Chart`): Datasets of points on two axes, as lines or dots, with a legend.
- DotCanvas (`rtwk::data::DotCanvas`): Shapes drawn finer than the cells, in data coordinates, with labels over them.
- Calendar (`rtwk::data::Calendar`): A month to pick a day of: the arrows move the cursor, PageUp and PageDown a month, Enter or a click picks the day.
- Heatmap, ActivityHeatmap (`rtwk::data::Heatmap`, `rtwk::data::ActivityHeatmap`): Values over a grid as shades or colors, its rows and columns labelled; counts by day over weeks, a column a week. The arrows move a cursor, its tooltip saying what is there.
- Table (`rtwk::data::Table`): Rows in columns that size by their content or a share, sorted by a click on a title; the arrows select a row or a cell.
- DescriptionList (`rtwk::data::DescriptionList`): Terms and their values, the values in a column of their own, aligned group by group; a long value wraps under itself, and on a narrow screen each goes under its term.
- TreeView (`rtwk::data::TreeView`): Nodes under their parents: the arrows move and expand, Enter or a click on the expander opens one; tests/ loads a while.
- TreeTable (`rtwk::data::TreeTable`): A tree in the first column of a table: rows that expand and collapse, sorted within their parents, selected one or many at a time.
- DirectoryTree (`rtwk::data::DirectoryTree`): The files under a directory as a tree for a side panel: each directory listed as it is opened and again as it changes, what .gitignore ignores left out.
- TaskList (`rtwk::data::TaskList`): Tasks and how far they have got: pending, running with a spinner and the time it has run, done, failed or skipped; their steps below.

## Editing

Text fields and what is drawn around them, text areas and completion as the user types. File: `widgets/editing.md`.

- TextField (`rtwk::editing::TextField`): A line of text to edit: Tab to it and type; Enter submits it, Ctrl+Z undoes.
- InputDecoration (`rtwk::editing::InputDecoration`): What is drawn around a field: a label, an icon, a prefix and a suffix, a helper, an error and a counter, in a box, over a line or bare, filled.
- TextArea (`rtwk::editing::TextArea`): Lines of text to edit, wrapped to the width: Enter breaks a line, the area grows from its least lines to its most, then scrolls.
- Autocomplete (`rtwk::editing::Autocomplete`): Suggestions under a field as one types, after a trigger or for the whole text: Up and Down pick one, Enter or Tab puts it in.

## Controls

Buttons, checkboxes and switches, choices, sliders, forms, tabs and status bars. File: `widgets/controls.md`.

- Button (`rtwk::controls::Button`): A label pressed by a click, or by Enter or Space while it has the focus; disabled with no on_pressed.
- Pressable (`rtwk::controls::Pressable`): What the controls are built of: a node of the focus tree that Tab passes by while it is disabled, the keys it takes while it has the focus, a press that focuses it and a tap; it builds its look by whether it has the focus.
- Checkbox, Switch (`rtwk::controls::Checkbox`, `rtwk::controls::Switch`): What is on or off: a click, or Space or Enter, flips it.
- RadioGroup (`rtwk::controls::RadioGroup`): One of several choices: the arrows or a click choose one.
- SegmentedButton (`rtwk::controls::SegmentedButton`): A few values joined in one row, one on at a time or any of them: the arrows, a digit or a click turn one on.
- Chip, ChipList (`rtwk::controls::Chip`, `rtwk::controls::ChipList`): Tags on one row that wraps, with a cursor: the arrows move it, Space or a click toggles a chip, Delete or a click on its × removes it.
- ValueStepper (`rtwk::controls::ValueStepper`): One of a few values between ‹ and ›: Left and Right or a click on an arrow step to the one before or after, stopping at the ends (`wrap(true)` goes round).
- SelectionList (`rtwk::controls::SelectionList`): Items selected by their keys: in a single selection the cursor selects, in a multiple one Space does.
- Select, MultiSelect (`rtwk::controls::Select`, `rtwk::controls::MultiSelect`): A field that opens its choices to pick one, or several, from, or a PopupController from the app; typing filters them.
- ConfirmInput (`rtwk::controls::ConfirmInput`): A question answered by y, n or Enter.
- Slider (`rtwk::controls::Slider`): A value along a track: the arrows, a press or a drag move the thumb, which stands in eighths of a cell.
- NumberInput (`rtwk::controls::NumberInput`): A number typed, or stepped by Up and Down, the wheel or a click on ▴▾; kept within its bounds.
- DatePicker (`rtwk::controls::DatePicker`): A date typed, stepped by Up and Down, or picked from the calendar that Alt+Down, a click on ▾ or a PopupController opens.
- FilePicker (`rtwk::controls::FilePicker`): A file picked from the directory listed under a field that names its path: the arrows and Enter go through it, Tab completes a name; a FilePickerController goes to a directory.
- Form, FormField (`rtwk::controls::Form`, `rtwk::controls::FormField`): Fields validated and submitted together: Enter goes on to the next field, and from the last submits the form.
- TabBar, TabBarView (`rtwk::controls::TabBar`, `rtwk::controls::TabBarView`): Tabs to pick a view by: Left and Right, or a click.
- KeyHintsBar (`rtwk::controls::KeyHintsBar`): The keys that can be pressed now, with their hints, as the focus moves: Tab to the field to see its keys.
- StatusMessage, Badge (`rtwk::controls::StatusMessage`, `rtwk::controls::Badge`): A message of a kind after its icon, in its color; a short label on its color.
- Callout (`rtwk::controls::Callout`): A notice in a box lined in the color of its kind, its title on the top edge; [×] or Escape dismisses it, and buttons below act on it.
- Hyperlink (`rtwk::controls::Hyperlink`): Text linked to a URL, which a terminal that knows hyperlinks opens.
- ListTile (`rtwk::controls::ListTile`): A row of a list: a title over a subtitle, something before and after them; tapped by a click or Enter, lit while focused or under the mouse.
- StatusBar (`rtwk::controls::StatusBar`): A row of three sections, left, centre and right; those that do not fit give way, the centre first.

## Overlays

Dialogs, toasts, tooltips, popovers, menus and the command palette, over the app. File: `widgets/overlays.md`.

- Overlay (`rtwk::overlay::Overlay`, `rtwk::overlay::OverlayEntry`): A layer over the app that dialogs, toasts, menus and tooltips go in; an entry of its own goes in it too.
- App (`rtwk::overlay::App`): The root of an app: its theme over the Overlay its dialogs show in, over the app and its own keys.
- Dialog (`rtwk::overlay::Dialog`): A box over the app, the rest dimmed behind a barrier; Tab stays in it, Escape closes it.
- Toast (`rtwk::overlay::show_toast`, `rtwk::overlay::ToastOptions`): A message that shows for a while and goes, over a corner of the app.
- Tooltip (`rtwk::overlay::Tooltip`): A message by a widget while the mouse or the keyboard focus rests on it, or while a TooltipController says, centred under it by an AnchorFollower in the overlay; above it where there is no room below.
- Popover (`rtwk::overlay::Popover`): A widget by another while the app says so, placed by an AnchorTarget around it and an AnchorFollower in the overlay; Escape or a click outside closes it.
- MenuBar (`rtwk::overlay::MenuBar`): Menus in a bar: F10 or a click opens one, Alt and a mnemonic too, or a MenuController from the app; the arrows, Enter and the mnemonics choose.
- ContextMenu (`rtwk::overlay::ContextMenu`): A menu at the mouse on a right click, under the focus on Shift+F10, or where a MenuController opens it.
- CommandPalette (`rtwk::overlay::CommandPalette`): What can be done, found by typing: Tab to a button, then Ctrl+K, or press Rust files, whose CommandPaletteController opens it with .rs typed; commands_for adds the files that match. The gallery's own, Ctrl+P, goes to a page.
- OptionList (`rtwk::overlay::OptionList`): Options in rows, one highlighted, the characters a query matched standing out and a detail at the right edge: what an autocomplete, a Select and a command palette list.
- ModalBarrier (`rtwk::overlay::ModalBarrier`): What dims the app under a dialog and keeps the mouse from it.
- WindowLayer (`rtwk::overlay::WindowLayer`): Windows over the app: a title bar drags one, an edge resizes it, its buttons minimise, maximise and close it; Tab to a title bar, then the arrows move it.

## Animation

Spinners, progress, and widgets that animate as their values change. File: `widgets/animation.md`.

- Spinner (`rtwk::animation::Spinner`): Frames shown in turn while something goes on; spinners of the same frames step together.
- TickerMode (`rtwk::animation::TickerMode`): Whether the animations below tick: off, they ask for no frames and stand still, and go on when it is on again; Offstage and Visibility turn it off for what they hide.
- ProgressBar (`rtwk::animation::ProgressBar`): A bar filled as far as its value, in eighths of a cell; a block going to and fro while the value is not known.
- ProgressList (`rtwk::animation::ProgressList`): Many tasks at once, a row each: its mark, a bar, how much is done, its rate and the time it has left, from a ProgressTracker the work counts on.
- Skeleton, Skeletonize (`rtwk::animation::Skeleton`, `rtwk::animation::Skeletonize`): Placeholders while content loads: blocks where it will be, or a subtree drawn as blocks in the layout it has, a shimmer sweeping over them as a SkeletonTheme says.
- AnimationController (`rtwk::animation::AnimationController`): An animation of the page's own: a controller from 0 to 1 and back, a frame at a time, through a curve and a tween.
- AnimatedBuilder (`rtwk::animation::AnimatedBuilder`): A builder run again at each tick of an animation, the part that does not change built once and passed in as its child.
- AnimatedAlign, AnimatedPadding (`rtwk::animation::AnimatedAlign`, `rtwk::animation::AnimatedPadding`): A child that runs to the place its alignment or its padding gives it, a cell at a time.
- AnimatedPositioned (`rtwk::animation::AnimatedPositioned`): A child pinned in a stack that runs to the edges it is pinned to.
- AnimatedBox (`rtwk::animation::AnimatedBox`): A box whose size, padding, background and border colors run to the new ones; a border of another type switches halfway.
- AnimatedOpacity, Opacity (`rtwk::animation::AnimatedOpacity`, `rtwk::animation::Opacity`): A child shown at an opacity, mixed cell by cell with what is under it; the one on the left runs to the opacity chosen.
- AnimatedDefaultTextStyle (`rtwk::animation::AnimatedDefaultTextStyle`): The style of the text below it, its colors running to the new ones.
- AnimatedSize (`rtwk::animation::AnimatedSize`): As big as its child, running to the new size when the child grows or shrinks.
- TweenAnimationBuilder (`rtwk::animation::TweenAnimationBuilder`): Builds with a value that runs to the one it is given, through the tweens it makes.
- AnimatedSwitcher (`rtwk::animation::AnimatedSwitcher`): A child that gives way to another as the transition shows them: faded, slid or dissolved.
- Dissolve, FractionalTranslation (`rtwk::animation::Dissolve`, `rtwk::animation::FractionalTranslation`): What the transitions are made of: a share of the cells of a child, and a child moved a share of its size.
- Effect (`rtwk::animation::Effect`): A child shown through an effect over its cells, run on the app's clock: recolored, swept, dissolved, typed or glitched in, while the child repaints underneath.

## Mouse

Clicks, drags, hovers and the wheel, and where the mouse is let through. File: `widgets/mouse.md`.

- GestureDetector (`rtwk::gestures::GestureDetector`): Taps, double taps, long presses and drags of the mouse over a widget.
- Listener, MouseRegion (`rtwk::gestures::Listener`, `rtwk::gestures::MouseRegion`): The mouse events over a widget as they come: presses, releases, motion and the wheel; the mouse coming in and going out, the pointer a hand over it where the terminal shows that.
- IgnorePointer, AbsorbPointer (`rtwk::gestures::IgnorePointer`, `rtwk::gestures::AbsorbPointer`): A subtree the mouse passes by: IgnorePointer lets it through to what is under it, AbsorbPointer stops it there.
- SelectionArea (`rtwk::gestures::SelectionArea`): Text selected with the mouse as in a terminal, and copied to the clipboard: drag over it, or click twice or three times; a painted Selectable is selected in too, and a drag past its edge scrolls it.
- Draggable, DragTarget (`rtwk::gestures::Draggable`, `rtwk::gestures::DragTarget`): Data dragged from a widget with the mouse and dropped on another that accepts it.

## Focus and keys

The focus, moved by Tab and the mouse, and the keys bound where it is. File: `widgets/focus-and-keys.md`.

- Focus, FocusScope (`rtwk::focus::Focus`, `rtwk::focus::FocusScope`): Nodes the keys go to: Tab moves the focus from one to the next, a click takes it; in a scope Tab goes round its nodes, and Tab into it focuses the one it had last.
- ActiveItem (`rtwk::focus::ActiveItem`): The item a focused list's keys act on: the terminal's hidden cursor rests on it, which iTerm2's cursor guide marks and macOS zoom, a screen reader or a braille display follows.
- ExcludeFocus (`rtwk::focus::ExcludeFocus`): A subtree Tab passes by: its nodes take no focus while it excludes them, and give it up when it starts to.
- KeyBindings, LeaderKey (`rtwk::focus::KeyBindings`, `rtwk::focus::LeaderKey`): Keys and sequences bound to callbacks while the focus is below them: Tab to the box, then +, -, g g or the leader , and r.
- Shortcuts, Actions (`rtwk::focus::Shortcuts`, `rtwk::focus::Actions`): Keys bound to intents, which the nearest action for them does: Tab to the box, then Up or Down; a key whose action is off passes on.

## Navigation

Routes, a scaffold of panes, steppers, breadcrumbs and pages. File: `widgets/navigation.md`.

- Navigator (`rtwk::navigation::Navigator`): Screens on a stack: a route pushed covers the one under it and takes the focus, Escape or Back pops it.
- PopScope, NavigatorObserver (`rtwk::navigation::PopScope`, `rtwk::navigation::NavigatorObserver`): A route that asks before it is popped, by Escape or by the back key, a dialog too; and an observer told of every push, pop and removal.
- Breadcrumbs (`rtwk::navigation::Breadcrumbs`): Where a screen is in a hierarchy, each level a link up: the arrows and Enter or a click go to one, and the middle collapses into … where the row does not fit.
- Pagination (`rtwk::navigation::Pagination`): The pages of a long list on one row, the current one among its siblings: the arrows, PageUp and PageDown, a page number typed or a click go to another.
- Stepper (`rtwk::navigation::Stepper`): A wizard of steps under their headers, each left forward once its validation takes it: Next or PageDown validates, Back or PageUp goes back, a header clicked goes to it.

## Foundation

Notifiers and listenables that widgets build on. File: `widgets/foundation.md`.

- ListenableBuilder (`rtwk::foundation::ListenableBuilder`): What is built again each time a listenable notifies; ValueListenableBuilder is given its value.
- FutureBuilder, StreamBuilder (`rtwk::foundation::FutureBuilder`, `rtwk::foundation::StreamBuilder`): What is built from the latest snapshot of a future or a stream: waiting, done, or each event.

## Framework

Stateful and inherited widgets, builders, keys and the element tree. File: `widgets/framework.md`.

- LayoutBuilder (`rtwk::LayoutBuilder`): What is built by the constraints it is laid out within: side by side when wide, one under another when narrow.
- NotificationListener (`rtwk::NotificationListener`): The notifications that bubble up to it from below, such as those of a list as it scrolls: the wheel over it, or Tab to it and the arrows.
- Slot, SlotScope (`rtwk::slots::Slot`, `rtwk::slots::SlotScope`, `rtwk::slots::SlotRegistry`): Places the app names that plugins fill with the entries of a SlotRegistry: Tab to a plugin, then Space puts its entry in the status bar or takes it out.

## App

The app around it all: the screen modes, a fixed viewport, output kept in the scrollback, the window title, notifications and a command-line tool's console. File: `widgets/app.md`.

- InputListener (`rtwk::binding::InputListener`): The input no widget took: keys nothing is bound to, the mouse where no Listener is, pastes, focus and resizes.
- ErrorWidget (`rtwk::binding::ErrorWidget`): An error shown in place of what failed to build, as the app does by itself.
- Static (`rtwk::binding::Static`): Output that is finished once shown: in place here; in the inline mode above the app, in the terminal's history.
- TerminalProgress (`rtwk::binding::TerminalProgress`): The program's progress in the terminal's tab or taskbar, where the terminal shows one (Ghostty, kitty, Windows Terminal, ConEmu).
- Viewport (`rtwk::app::Viewport`, `rtwk::app::ScreenMode`): The app in a fixed rectangle of the screen (Viewport::fixed): nothing outside it is written, so what the terminal showed around it stays.
- Window title (`rtwk::binding::TerminalTitle`): The title of the terminal's window or tab while the app runs; the user's own comes back when it ends.
- Notifications (`rtwk::binding::AppBinding`): A desktop notification of a title and a body, where the terminal shows one (OSC 9, 777 or 99); nothing where it does not.
- Cli (`rtwk::cli::Cli`): A command-line tool's console (the Cli widget) as it runs: questions that leave their answer as a line, a spinner, tasks with bars, a traceback.

## Screen readers

What the widgets are to a screen reader. File: `widgets/screen-readers.md`.

- Semantics (`rtwk::semantics::Semantics`): What a widget is to a screen reader: a role, a label, a value and a state, read as a line of text. TWK_SCREEN_READER=1 reads an app out so.

## Debugging

The log console, the frame overlay, the repaint rainbow and repaint boundaries. File: `widgets/debugging.md`.

- Console (`rtwk::debug::Console`): Log records listed over the app, newest last; the app has one of its own, which Ctrl+G then c opens.
- FrameOverlay (`rtwk::debug::FrameOverlay`): Each frame's numbers over the app: the time its build, layout, paint and write took and the bytes it wrote; Ctrl+G then o.
- Repaint rainbow (`rtwk_render::RenderTree`): Each repaint boundary tinted a new colour when it paints, and relayouts marked, to see what a change redraws; Ctrl+G then r, and l.
- RepaintBoundary (`rtwk::layout::RepaintBoundary`): A subtree painted on its own layer: what changes in it is painted again without what is around it, and the other way round.

## Rich

Markdown, code, diffs, JSON, big text, QR codes and program output with its colors. File: `widgets/rich.md`.

- MarkdownView (`rtwk_rich::markdown::MarkdownView`): A Markdown document as widgets, a blank row between the blocks; its links the app's with on_link_tap, a MarkdownController scrolling to a heading.
- CodeView (`rtwk_rich::highlight::CodeView`): Source code highlighted for its language, with line numbers, its long lines wrapped or cut off at the width; a CodeViewController scrolls it to a line, highlights lines and finds text.
- SyntaxThemeScope (`rtwk_rich::highlight::SyntaxThemeScope`): The styles of the code below it, a style for each kind of token, in place of those derived from the theme: a SyntaxTheme of its own, or the derived one with a few changed.
- DiffView (`rtwk_rich::diff::DiffView`): The changes between two texts, unified or side by side, the words that changed marked and both sides highlighted; its lines are selected and copied without their numbers.
- JsonView (`rtwk_rich::json::JsonView`): JSON, or a JsonValue built in code, as a tree that folds, each kind of value in its color: c copies the path of the value selected, y the value, / filters the keys.
- BigText (`rtwk_rich::bigtext::BigText`): Text in letters several cells tall, drawn with full or half blocks or with box drawing, in a font of its own.
- QrCode (`rtwk_rich::qr::QrCode`, `rtwk_rich::qr::QrCodeWidget`): Text as a QR code a phone's camera reads: in half blocks, two modules to a cell, black on white whatever the theme, with as much of the quiet zone around it as the rows allow.

## Images

PNG and JPEG pictures, in the terminal's graphics or in block characters. File: `widgets/images.md`.

- Image (`rtwk_rich::image::Image`): A PNG, JPEG, GIF or APNG fitted to its cells; a GIF or APNG with frames plays on the app’s clock, each frame drawn as it comes, and kitty, which plays an animation itself, is sent all the frames once. A TickerMode that is off stands it on the frame it shows. Where the terminal has a graphics protocol (kitty, sixel, iTerm2) its pixels are placed over the cells (drawn: protocol); where it has none, or with force_blocks, it is drawn in characters: half blocks, quadrants, sextants (Unicode 13), octants (Unicode 16), braille or plain ASCII. Left to detected, the default, it is sextants where the terminal is known to draw them itself (kitty, foot, Ghostty, WezTerm, Konsole, VTE and a few more) and half blocks elsewhere; TWK_IMAGE_BLOCKS names a mode for all images.

## Terminal

A program running in a widget, on a pseudo-terminal. File: `widgets/terminal.md`.

- TerminalView (`rtwk_term::TerminalView`): A program's terminal in a view: keys, pastes, the mouse and resizes sent to it, its scrollback scrolled; here a scripted shell, not a program in a pseudo-terminal.
