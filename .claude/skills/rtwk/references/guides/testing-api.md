<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 5001d2cd; do not edit. -->

# Testing API: rtwk_test

`rtwk_test`: drive an app headless and read the screen. `find::*` finders are free functions in `rtwk_test::find`.

## Tester

- `rtwk_test::Tester` (struct, `crates/rtwk-test/src/tester.rs:427`) Runs an app under test: the real binding on a terminal in memory (`terminal`), the real parser decoding the bytes of the input, an emulator reconstructing the screen from the bytes of the frames.

#### `Tester`: constructors and associated functions

- `pub fn new() -> Tester` - A tester with the default `Options`; the app starts at the first `pump_widget`.
- `pub fn with(options: Options) -> Tester` - A tester that will start its app with `options`.

#### `Tester`: builder methods (take and return `self`)

- `pub fn frames_on_failure(mut self, count: usize) -> Tester` - How many frames that changed the screen a test that fails prints (3 by default; 0 for none).

#### `Tester`: methods

- `pub fn assert_line(&self, line: &str)` - Asserts a row of the screen is `line`, the blanks that end either aside; a row is text as `lines` gives it.
- `pub fn assert_line_in(&self, line: &str, within: impl Into<Region>)` - As `assert_line` looking at the rows of the cells of `within` alone (`lines_in`), from its first column.
- `pub fn assert_line_where(&self, description: &str, test: impl Fn(&str) -> bool)` - Asserts a row of the screen satisfies `test`, described as `description` in the message (twk's `containsLine` given a matcher).
- `pub fn assert_text_at(&self, x: usize, y: usize, text: &str)` - Asserts the screen shows `text` from column `x` of row `y` on: the cells `text` is as wide as show it, a wide glyph once.
- `pub fn assert_text_at_in(&self, x: usize, y: usize, text: &str, within: impl Into<Region>)` - As `assert_text_at`, `x` and `y` counted from the first cell of the region, and the cells read stopping at its edge.
- `pub fn assert_style_at(&self, x: usize, y: usize, style: &CellMatch)` - Asserts the cell at column `x` of row `y` satisfies `style` (`has_attr`, `has_fg`, `has_bg` and their combinations).
- `pub fn assert_cursor(&self, x: usize, y: usize)` - Asserts the screen shows the cursor at column `x` of row `y`: hidden, it matches nowhere.
- `pub fn assert_screen(&self, snapshot: &str)` - Asserts the screen is `snapshot`: its lines are the rows of the screen from the top, as text (`lines`), the blanks that end a line and the empty lines that end the screen left out of both.
- `pub fn assert_screen_styled(&self, snapshot: &str)` - As `assert_screen` comparing styles too: each run of cells of one style is in `[style]…[/]` (`[bold]sent:[/] привет [red on #102030]ok[/]`), a `[` of the text written `[[`.
- `pub fn assert_finds(&self, finder: &Finder, n: usize)` - Asserts `finder` finds exactly `n` widgets.
- `pub fn assert_finds_none(&self, finder: &Finder)` - Asserts `finder` finds no widget.
- `pub fn assert_finds_one(&self, finder: &Finder)` - Asserts `finder` finds exactly one widget.
- `pub fn assert_finds_some(&self, finder: &Finder)` - Asserts `finder` finds one widget or more.
- `pub fn assert_screen_file(&self, path: impl AsRef<Path>)` - Asserts the screen is the one the file at `path` holds (twk's golden file: its size, where its cursor is, the text of each row and the style of each cell).
- `pub fn assert_screen_file_text(&self, path: impl AsRef<Path>)` - As `assert_screen_file` comparing the size, the cursor and the text only; writing the file still writes the styles.
- `pub fn write_screen_file(&self, path: impl AsRef<Path>)` - Writes the screen to the file at `path`, making its directories; the styles keep the characters the file gave them, so that a file written again changes where the screen did.
- `pub fn clock(&self) -> &FakeClock` - The time of the app.
- `pub fn elapsed(&self) -> Duration` - The time on the clock since the test began.
- `pub fn frame_gap(&self) -> Duration` - The gap between two frames: a second over `max_fps` (16.666 ms at 60).
- `pub fn logs(&self) -> &LogSink` - The app's log: what it printed, its warnings, and the errors it reports.
- `pub fn is_running(&self) -> bool` - Whether the app started and has not ended.
- `pub fn stderr_lines(&self) -> &[String]` - What the app wrote to stderr once it ended: the warnings and errors of its log, one per entry (empty while it runs).
- `pub fn terminal(&self) -> &TimedTerminal` - The terminal the app runs on: `frames()` holds every frame the app wrote, including writes waiting behind an output hold.
- `pub fn frame_loop(&self) -> &App<TimedTerminal>` - The frame loop the app runs in (twk's `binding.twkApp`): its presenter, its styles, its log.
- `pub fn frame_loop_mut(&mut self) -> &mut App<TimedTerminal>` - The frame loop, to change (the cursor the presenter shows, a frame asked for).
- `pub fn binding(&self) -> AppBinding` - The binding running the app.
- `pub fn tree(&self) -> Option<&WidgetTree>` - The widget tree the app runs in; none before it started.
- `pub fn tree_mut(&mut self) -> &mut WidgetTree` - The widget tree, to change.
- `pub fn pump_widget(&mut self, widget: impl Into<AnyWidget>)` - Starts the app showing `widget` on a terminal of the `Options`' size; a later call shows `widget` in place of the one shown before (state is kept where widgets match, as in any rebuild).
- `pub fn pump_widget_for(&mut self, widget: impl Into<AnyWidget>, duration: Duration)` - As `pump_widget`, pumping `duration`: `Duration::ZERO` runs only the app's first frame, at time zero.
- `pub fn pump_app<A: 'static>(&mut self, app: AppView<A>)` - Starts the application `app` (a view of a value of your own, as `rtwk::app::run` runs it), or shows it in place of what was shown; `read_app` and `update_app` reach its value.
- `pub fn pump_app_for<A: 'static>(&mut self, app: AppView<A>, duration: Duration)` - As `pump_app`, pumping `duration`.
- `pub fn pump_app_in<A: 'static>(&mut self, app: AppView<A>, wrap: impl FnOnce(AnyWidget) -> AnyWidget)` - As `pump_app`, with the app shown inside what `wrap` makes of it: the root a process gives it (`rtwk::run_app_with`'s `overlay::App`, a theme, localizations, a `GraphicsScope`) put around it, the app keeping its ...
- `pub fn pump_app_in_for<A: 'static>(&mut self, app: AppView<A>, wrap: impl FnOnce(AnyWidget) -> AnyWidget, duration: Duration)` - As `pump_app_in`, pumping `duration`.
- `pub fn pump(&mut self)` - Moves the clock on by one frame gap (a second over `max_fps`): what is due by then runs — timers, frames, a lone ESC resolved — in order, so a frame asked for before the call has run when it returns.
- `pub fn pump_for(&mut self, duration: Duration)` - Moves the clock on by `duration`, running everything due by then in order.
- `pub fn pump_until_idle(&mut self)` - Pumps frame gap after frame gap until the app is idle: a gap passed without a frame written, and nothing of the app waits (no timer, no frame callback, no work for the next frame).
- `pub fn pump_until_idle_within(&mut self, timeout: Duration)` - As `pump_until_idle` with the bound `timeout`.
- `pub fn act<R>(&mut self, f: impl FnOnce(&mut AppHost) -> R) -> R` - Runs `f` on the app's host, as the app's own code runs (the tree and the binding), then pumps a frame (`pump`): how a test changes what the app listens to and sees the frame that follows.
- `pub fn read_app<A: 'static, R>(&self, f: impl FnOnce(&A) -> R) -> R` - Reads the value of the application `A` shown by `pump_app`.
- `pub fn update_app<A: 'static>(&mut self, f: impl FnOnce(&mut A, &mut app::AppCx<'_, '_, A>) + 'static)` - Changes the value of the application `A` from outside, as a handler does; the next frame shows it.
- `pub fn state<S: State, R>(&self, finder: &Finder, f: impl FnOnce(&S) -> R) -> R` - Reads the state of the stateful widget `finder` finds.
- `pub fn input(&mut self, bytes: &[u8])` - Delivers `bytes` as one read of the terminal's input.
- `pub fn type_text(&mut self, text: &str)` - Delivers `text` as typed: its UTF-8 bytes as one read (`input`), which the parser makes a key of each character.
- `pub fn press(&mut self, keys: &str)` - Presses each key of `keys` in turn, as the bytes the terminal sends for it, a read each: keys named as key bindings name them, apart by spaces (`KeyCombo::parse_sequence`): `ctrl+s`, `shift+tab`, `A`, `g g`, `ctrl+x ...
- `pub fn paste(&mut self, text: &str)` - Pastes `text` as a terminal with bracketed paste does: between its start and end markers, in one read.
- `pub fn mouse(&mut self, kind: MouseKind, x: u16, y: u16, button: u8, mods: Mods)` - Delivers the mouse report (SGR, mode 1006) of `kind` at the cell (`x`, `y`), 0-based, with `button` (0 left, 1 middle, 2 right, 8 to 11 the side buttons) for a press, release or drag, and `mods` (Shift, Alt and Ctrl).
- `pub fn click(&mut self, x: u16, y: u16)` - Presses the left button at the cell (`x`, `y`) and releases it there.
- `pub fn click_button(&mut self, x: u16, y: u16, button: u8)` - Presses `button` at the cell (`x`, `y`) and releases it there.
- `pub fn resize(&mut self, width: u16, height: u16)` - The terminal becomes `width` × `height` cells, and the app gets the resize.
- `pub fn hold_output(&mut self)` - Holds subsequently written output until `release_output`.
- `pub fn hold_output_for(&mut self, duration: Duration)` - Holds subsequently written output for `duration` on the tester's clock.
- `pub fn release_output(&mut self)` - Makes held writes available to the emulator.
- `pub fn quit_after_output(&mut self, timeout: Duration) -> bool` - Pumps pending builds and output, then quits once the inline rows have been delivered.
- `pub fn quit(&mut self)` - Quits the app, unless it has ended, and runs it until it has: the terminal is restored, the tree unmounted.
- `pub fn take_errors(&mut self) -> Vec<LogRecord>` - The errors the app logged since the last call, oldest first, taken from the test: each one the test does not take fails it when it ends.
- `pub fn with_screen<R>(&self, f: impl FnOnce(&Screen) -> R) -> R` - Reads the screen, fed every frame delivered so far.
- `pub fn lines(&self) -> Vec<String>` - The rows of the screen as text, without the blanks that end them, a wide glyph once.
- `pub fn screen(&self) -> String` - The screen as twk's snapshot text shows it: its rows from the top, without the empty rows that end the screen — what `insta::assert_snapshot!` stores.
- `pub fn marked_screen(&self) -> String` - The screen as `screen` with each run of cells of one style in `[style]…[/]` (`[bold]sent:[/] привет [red on #102030]ok[/]`).
- `pub fn styled_screen(&self) -> String` - The screen in twk's screen-file format: its size and cursor, the text of each row, the style of each cell as a character and a legend naming them (`insta::assert_snapshot!(t.styled_screen())`).
- `pub fn lines_in(&self, region: CellRect) -> Vec<String>` - The rows of `region`, a rectangle of the screen as `rect` gives one, as `lines` gives the screen's: the text of its cells, a wide glyph once, from the region's first column and without the blanks that end a row; as many ...
- `pub fn cell(&self, x: usize, y: usize) -> Cell` - The cell of the screen at column `x` and row `y`: its text, colors and attributes.
- `pub fn cursor(&self) -> (usize, usize)` - Where the cursor is: column and row of the screen.
- `pub fn cursor_visible(&self) -> bool` - Whether the terminal shows the cursor.
- `pub fn images(&self) -> Vec<crate::Image>` - The images the terminal shows (see `Options::graphics`), top to bottom, each as the emulator reads it (twk's `t.vt.images`).
- `pub fn scrollback(&self) -> Vec<String>` - The lines scrolled off the top of the screen, oldest first, as `lines` gives rows (twk's `scrollback`).
- `pub fn viewport_lines(&self) -> Vec<String>` - The rows of the app's viewport (`Options::viewport`) as `lines` gives the screen's, from its first column: as many of its rows and columns as the screen has (twk's `viewportLines`).
- `pub fn pointer_shape(&self) -> Option<PointerShape>` - The shape the app gave the mouse pointer last (OSC 22): `PointerShape::Arrow` where it shows the terminal's default, `None` before it gave any (twk's `pointerShape`).
- `pub fn written(&self) -> String` - What the app wrote to the terminal, as text: the bytes of the frames decoded as UTF-8, a byte that is not as U+FFFD. Where a test looks for what the screen does not show: a mode set, the clipboard written (OSC 52), a ...
- `pub fn written_since(&self, since: usize) -> String` - As `written`, of the frames from `terminal().frames()[since]` on.
- `pub fn frames(&self) -> Vec<RecordedFrame>` - The frames the app wrote so far, in order: each write to the terminal with its time on the clock and the screen it left, replayed on an emulator of its own that resizes where the terminal did.
- `pub fn write_trace(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()>` - Writes the frames so far to the file at `path` as an output trace (rtwk-core's `Trace`), each at its time on the clock, at the size the terminal had when the app started; it has no resizes.
- `pub fn app_elements(&self) -> Vec<Eid>` - The elements of the app in the order of the tree: that of the widget `pump_widget` showed, and those below it.
- `pub fn element(&self, finder: &Finder) -> Eid` - The element of the widget `finder` finds.
- `pub fn widget<W: rtwk::Widget, R>(&self, finder: &Finder, f: impl FnOnce(&W) -> R) -> R` - The widget `finder` finds, read with `f`.
- `pub fn render_object<O: rtwk_render::RenderObject, R>(&self, finder: &Finder, f: impl FnOnce(&O) -> R) -> R` - The render object of the widget `finder` finds, or of the nearest widget below it that has one, read with `f`.
- `pub fn rect(&self, finder: &Finder) -> CellRect` - The cells of the widget `finder` finds, as its render object was laid out last: those of the screen.
- `pub fn lines_of(&self, finder: &Finder) -> Vec<String>` - The rows of the cells of the widget `finder` finds (`rect`), as `lines_in` gives them.
- `pub fn click_on(&mut self, finder: &Finder)` - Clicks the left button on the widget `finder` finds: on the cell in the middle of those it shows, its own cut off where they are outside one of the render objects above it, as a hit test cuts them.
- `pub fn click_on_button(&mut self, finder: &Finder, button: u8)` - As `click_on` with `button`.
- `pub fn describe_app(&self, screen: bool) -> String` - The screen (`screen` true), the widget tree from the widget `pump_widget` showed down, and the render tree, as a failure message shows them.
- `pub fn widget_tree(&self) -> String` - The widget tree from the widget `pump_widget` showed down, as rtwk's tree dump prints it; `none` before the first frame and once the app ended.
- `pub fn render_tree(&self) -> String` - The render tree from the binding's root render object down.
- `pub fn failure_report(&self) -> String` - The text a test that fails prints: the last frames of the app that changed the screen, then its log.
- `pub fn finish(&mut self)` - Ends the test's app, if it started: quits it, panics with an error it ended with that was not thrown yet, feeds the emulator the rest and fails on bytes it did not take, on the errors not taken and, as the last check, ...


## Options

- `rtwk_test::Options` (struct, `crates/rtwk-test/src/tester.rs:43`) How a `Tester` starts its app: the terminal's size and what it tells the app, the frame loop's pacing, the environment and the app's wall time.

#### `Options`: constructors and associated functions

- `pub fn new() -> Options` - The defaults.

#### `Options`: builder methods (take and return `self`)

- `pub fn size(mut self, width: u16, height: u16) -> Options` - The terminal is `width` × `height` cells.
- `pub fn kitty_keyboard(mut self, on: bool) -> Options` - The terminal has the kitty keyboard protocol on (`Tester::press` then sends what such a terminal sends).
- `pub fn background_rgb(mut self, rgb: u32) -> Options` - The terminal's default background, `0xRRGGBB` (unknown by default).
- `pub fn foreground_rgb(mut self, rgb: u32) -> Options` - The terminal's default text color, `0xRRGGBB` (unknown by default; twk's `pumpWidget(foregroundRgb:)`).
- `pub fn cursor_rgb(mut self, rgb: u32) -> Options` - The terminal's cursor color, `0xRRGGBB` (unknown by default; twk's `pumpWidget(cursorRgb:)`).
- `pub fn max_fps(mut self, fps: u32) -> Options` - The most frames a second the app draws (60 by default; 0 for no cap).
- `pub fn exit_on_ctrl_c(mut self, on: bool) -> Options` - Whether Ctrl+C quits the app (it does by default).
- `pub fn clear_inline_on_exit(mut self, on: bool) -> Options` - Whether an inline app erases its live region, prompt included, when it quits, leaving the cursor where the region began, and keeps what it committed to the history (twk's `pumpWidget(clearInlineOnExit:)`, ...
- `pub fn input_at_once(mut self, on: bool) -> Options` - Whether the frame an input asks for is drawn as the call that brought the input returns, when it is due (the default, as an app runs for a user); false holds it until the clock moves, as twk's tester does, which is how ...
- `pub fn env(mut self, name: &str, value: &str) -> Options` - A variable of the app's environment, which is empty by default so that what the shell running the tests exports (`TWK_DEBUG`, `TWK_SCREEN_READER`, `TWK_LOG`) changes nothing.
- `pub fn start_time(mut self, time: LogTime) -> Options` - The app's wall time when the clock reads zero.
- `pub fn graphics(mut self, protocol: GraphicsProtocol) -> Options` - The terminal shows images with `protocol` (by default none, and the cells of an image are left blank): the app paints them in it, and `Tester::images` reads what the emulator shows (twk's `pumpWidget(graphics:)`).
- `pub fn cell_pixels(mut self, width: u32, height: u32) -> Options` - A cell is `width` × `height` pixels: the terminal reports it to the app, and the emulator takes a sixel image's pixels to cells by it (10 × 20 for the emulator, none reported, by default; twk's ...
- `pub fn pointer_shapes(mut self, on: bool) -> Options` - The terminal shows the mouse pointer's shapes (OSC 22, by their CSS names), which `Tester::pointer_shape` reads (twk's `pumpWidget(pointerShapes:)`).
- `pub fn mode(mut self, mode: ScreenMode) -> Options` - Choose the alternate screen or an inline region of the normal screen.
- `pub fn history(mut self, lines: impl IntoIterator<Item = impl AsRef<str>>) -> Options` - The lines the shell showed before the app opened the terminal.
- `pub fn viewport(mut self, viewport: CellRect) -> Options` - Lay out and draw in this fixed rectangle.
- `pub fn fail_on_pending_timers(mut self, on: bool) -> Options` - Whether the test fails when the app, once it is quit and its widgets disposed, leaves a timer of its clock or a ticker running (twk's `testTwk(failOnPendingTimers:)`; on by default).
- `pub fn fail_on_overflow(mut self, on: bool) -> Options` - Whether the test fails when a flex, a wrap or a grid of the app was overflowed by its children (twk's `testTwk(failOnOverflow:)`; on by default): the warning of the log that the debug marker of the screen shows, named ...
- `pub fn check_intrinsics(mut self, on: bool) -> Options` - Whether the app's render tree checks the intrinsic sizes of what it lays out (`rtwk_render::RenderTree::set_check_intrinsics`; on by default, as twk_core's own render tests have it): an object whose answer is negative, ...
- `pub fn scrollback_lines(mut self, lines: usize) -> Options` - How many lines the emulator keeps of what scrolled off the top of its screen, 10 000 by default.
- `pub fn allow_pending_timers(self) -> Options` - The test means to leave a timer or a ticker running when the app ends: `fail_on_pending_timers` off.
- `pub fn terminal(mut self, f: impl FnOnce(&mut HeadlessOptions)) -> Options` - Whatever else the terminal is made with: its colors, graphics, multiplexer, notifications, pointer shapes, pauses (see `HeadlessOptions`); its size and clock are the tester's.

#### `Options`: methods

- `pub fn size_of(&self) -> (u16, u16)` - The terminal's size, columns and rows.
