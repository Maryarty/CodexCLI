<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Details and pitfalls

How an app runs, what the tester does, the debug aids, and the mistakes that
bite, in full. Where a page of its own says more, the line links to it;
[Coming from Flutter](coming-from-flutter.md) lists the names that differ from
the framework you may know.

## How it works

- Widgets, elements and `State` behave as in a retained widget tree: a change
  asks for a frame, a frame builds what changed, at most `AppOptions::max_fps`
  (60 by default) a second, and writes only the cells that changed. An app that
  changes nothing writes nothing ([Performance](performance.md)).
- `run_app` runs full screen on the alternate screen; inline, in rows below the
  shell's prompt, and the mouse stays the terminal's there. [Screen
  modes](screen-modes.md) has both, and `Static` for output that is finished once
  it is shown: a chat's history, a build log. Its items are told apart by
  their index, so only append to them.
- A key goes to the focused widget first, then to the layers above it
  (`KeyBindings`, `Shortcuts`), nearest first, until one takes it. Tab and
  Shift+Tab move the focus. Ctrl+C that nobody takes quits, unless
  `AppView::exit_on_ctrl_c(false)`; `cx.quit()` quits ([Input and
  focus](input-and-focus.md), [Running an app](running.md)).
- The app owns the terminal: `println!` would write into its screen. Log with the
  records the debug console shows (`console` below); a log goes to the file
  `TWK_LOG` names too. What the framework reports (a panic in a handler, a failed
  task, a check a debug build makes) is told to `AppView::on_error` and goes to the
  log, and where it is recoverable the app goes on.
- Without a `Theme`, widgets draw in the terminal's own colors; `Theme::adaptive`
  picks a dark or a light theme by the terminal's brightness, and a `ThemeData`
  preset (`ThemeData::NORD`) pins one ([Theming](theming.md)).
- The marks widgets draw (check, cross, cursor bar, a scrollbar's thumb, the
  spinner) are the theme's `Glyphs`, named by meaning: `Glyphs::UNICODE` by
  default, `Glyphs::ASCII` for a terminal or a font without them, for the whole
  app or one part under a `Theme` ([Theming](theming.md#glyphs)). A widget's own
  parameter is above the theme's.
- `run_app` places an `Overlay` at the root, for dialogs, toasts, menus and
  tooltips, and a `Navigator` holds one of its own. A widget that shows one needs
  an `Overlay` above it: in a test, mount `Overlay::new(app)`.

## The tester

`rtwk_test::Tester` runs the real app on a terminal in memory, under a fake
clock: input goes in as the bytes a terminal sends, and the screen is rebuilt from
the bytes the app wrote ([Testing](testing.md)).

- Input draws nothing by itself: `t.pump()` after `t.type_text(..)`, `t.press(..)`
  or a click, before reading the screen. `t.pump_for(duration)` moves the clock on
  by that much, running the app's timers, frames and animations due by then.
- Nothing is focused to begin with. A `TextField` takes the characters only once
  Tab, or a click, has given it the focus, so `t.press("tab")` comes before
  `t.type_text("ann")`. `t.press("tab tab space")` sends each key as a read of its
  own, named as `KeyBindings` names them, in lower case.
- A dialog fades in over 160 ms: one `t.pump()` shows it at its start, not yet
  drawn. Pump past it (`t.pump_for(Duration::from_millis(200))`), or show the
  dialog with the `fade_in` of its `DialogOptions` zero.
- The root of the tester's app is given tight constraints, the whole screen, which
  a `SizedBox` or a `ConstrainedBox` inside it cannot override. Put a `Column`
  with `CrossAxisAlignment::Start` around the widget under test to see its own size.
- A page pushed on a `Navigator` covers the one under it, which stays built:
  `find::text` still finds a row of the list under a page. Read the screen
  (`t.screen()`) for what is shown.
- A `Button` draws its label itself: it is found by its type
  (`find::by_type::<Button>()`), and `find::text` finds `Text`s only.
- Only the app's timers and frames are on the fake clock: `std::time`'s `now()`
  functions read real time. A widget reads the time from
  `AppBinding::of(cx).now()`.
- A failing assert shows the screen as text, the widget tree with each widget's
  size, and the render tree.

## Debug

- `TWK_DEBUG` turns the aids on from the start (`rtwk run --debug` sets it):
  `overlay` (frame times and bytes), `console` (the log), `rainbow` (what
  repaints), `relayout`, `keys` (the keys below), `logs` (serve the log to `rtwk
  logs`) and `frames` (each frame as text below the last). A debug build knows
  `overflow` and `rebuilds` too.
- With any of those on, Ctrl+G and then `o`, `c` or `r` turns the overlay, the
  console and the rainbow on or off while the app runs.
- `TWK_LOG=/tmp/app.log` appends every record of the log to that file.
- A row or column whose children need more room than it has is an overflow the
  log reports; [Layout](layout.md) says how a row is made to fit.

## Pitfalls

- Before writing a widget of your own that rtwk has, read what it already sets:
  [the widget catalog](../widgets.md) names every option, and many looks
  that seem to need a widget of their own are an option of one that is there.
- A mark drawn by hand (`Text::new("✓ done")`) stays Unicode whatever the theme
  says. Take it from the theme's `Glyphs`, or from the widget that draws it, so
  that an app in `Glyphs::ASCII` is ASCII all through.
- Give a generic widget its type: the columns of a `Table` written in the call get
  no type from its rows, so `|e: &Entry| ..` carries it ([Recipes](recipes.md)).
- A row of a list, a label cut with `…` and something fixed after it, is a `Row`
  of an `Expanded` `Text::line` and the fixed part; columns are a `SizedBox` of a
  `Text::line`. No cell arithmetic by hand ([Layout](layout.md#a-row-of-a-list)).
- A space at the end of a `Text` is lost with the default `TextWrap::Word`: it
  hangs past the line, uncounted and undrawn, so `Text::new("◐ ")` in a `Row` is
  one cell and the next child touches the mark. `.wrap(TextWrap::None)` keeps it.
  Cut and pad a `String` to cells with `fit_text` and `pad_cells`, never by
  `len()` ([Layout](layout.md#text-in-cells)).
- `ListView`, `Table`, `TreeView`, `SelectionList` and `Navigator` take all the
  room they get. In a `Column` put them in an `Expanded`, or they take the
  whole height and push the rest off the screen. Inline, a list asks for no rows
  and shows nothing: give it a height.
- Dialogs, toasts, menus, popovers, tooltips and `Select` need an `Overlay`
  above them. The context that shows one must be below it.
- A focused `TextField` takes the characters typed, so `("q", quit)` in a
  `KeyBindings` above it never sees `q`. Bind keys with Ctrl, as `ctrl+s`, where
  a field is on the screen ([Input and focus](input-and-focus.md)).
- The keys of one read (fast typing, a terminal's burst) are handled one after
  another with no build between them. A handler reads the state it changes, not
  what a build or a layout made of it. Test such keys typed together.
- `Column` and `Row` centre their children across, unless
  `cross_axis_alignment` says otherwise (`CrossAxisAlignment::Stretch` or `Start`), and they are as
  long as their room along the axis (`MainAxisSize::Max`), also inside another
  `Column`: a header made of a `Column` above an `Expanded` list takes the whole
  height and leaves the list no rows. Give what is to be as big as its children
  `MainAxisSize::Min`.
- A handler that belongs to a row of a lazily built list has no context to make
  it from: the row is a widget of its own, or its handler is made once and bound
  to the row's index ([Coming from Flutter](coming-from-flutter.md#lists-and-navigation)).
- A `Handler` belongs to the element that made it. Make it in the `view` or
  `build` of the widget that holds the state it changes, not in a closure that
  runs later.
- Write nothing to stdout or stderr while the app runs.

## Where to look

[The widget catalog](../widgets.md) has a page per topic and a section per
widget, naming every option and its default. The gallery (`cargo run -p rtwk-gallery`)
shows every widget live. Whole apps to copy from are in `crates/rtwk/examples`:
`agent_chat.rs`, a chat with a composer and a transcript, and `file_manager`,
`monitor` and `triage`, each with its test.
