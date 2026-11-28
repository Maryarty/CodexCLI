<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Testing

`rtwk-test` runs the real app, the same binding `run_app` uses, on a terminal in
memory under a fake clock. Keys, the mouse and pasted text go in as the bytes a
terminal sends, and the screen is rebuilt from the bytes the app wrote by a strict
terminal emulator, which the asserts read. Timers and animations move only when the
test says, so a test has no sleeps and no flakes.

```sh
cargo add --dev rtwk-test
```

The counter of [Getting started](getting-started.md) and the dialog of
[Recipes](recipes.md#a-dialog-that-asks-a-toast-that-says-it-is-done), tested:

<!-- file: crates/rtwk/tests/recipes/testing.rs -->
```rust

use std::time::Duration;

use rtwk::app::AppView;
use rtwk::controls::Button;
use rtwk::overlay::Overlay;
use rtwk::prelude::Color;
use rtwk::style::Attrs;
use rtwk_test::{Options, Tester, find, has_attr, has_fg};

use crate::counter::{Counter, view};
use crate::dialogs::Home;

fn counter() -> Tester {
    let mut t = Tester::with(Options::new().size(30, 5));
    t.pump_app(AppView::new(Counter::default(), view));
    t
}

#[test]
fn up_counts_and_the_screen_shows_it() {
    let mut t = counter();
    t.assert_line("│ Count: 0                   │");
    // input draws nothing by itself: the frame it changes is pumped
    t.press("up up");
    t.pump();
    t.read_app(|c: &Counter| assert_eq!(c.n, 2));
    t.assert_line("│ Count: 2                   │");
    t.assert_text_at(2, 2, "Count: 2");
    // the label is dim, the number bold and green
    t.assert_style_at(2, 2, &has_attr(Attrs::DIM));
    t.assert_style_at(9, 2, &has_attr(Attrs::BOLD).and(has_fg(Color::GREEN)));
}

#[test]
fn q_quits() {
    let mut t = counter();
    t.press("q");
    t.pump();
    assert!(!t.is_running());
}

#[test]
fn a_button_is_found_by_its_type_and_clicked() {
    let mut t = Tester::with(Options::new().size(40, 12));
    t.pump_widget(Overlay::new(Home));
    t.assert_finds_one(&find::by_type::<Button>());
    t.click_on(&find::by_type::<Button>());
    // a dialog fades in over 160 ms: the clock moves on past it
    t.pump_for(Duration::from_millis(200));
    t.assert_finds(&find::by_type::<Button>(), 3);
    t.assert_line_where("the title of the dialog", |l| {
        l.contains("Delete the file?")
    });
}

#[test]
fn a_toast_shows_for_three_seconds_and_no_test_waits_for_them() {
    let mut t = Tester::with(Options::new().size(40, 12));
    t.pump_widget(Overlay::new(Home));
    t.press("enter");
    t.pump_for(Duration::from_millis(200));
    t.press("shift+tab enter");
    t.pump_for(Duration::from_millis(300));
    assert!(t.screen().contains("Deleted."));
    // three seconds on the app's clock, no time on the wall's
    t.pump_for(Duration::from_secs(3));
    assert!(!t.screen().contains("Deleted."));
    assert!(t.elapsed() >= Duration::from_secs(3));
}
```

`counter` and `dialogs` are the modules of the two examples; in your own
package they are your crate's.

## The tester

- `Tester::with(Options::new().size(columns, rows))` makes the terminal, 80 × 24 by
  default; `.mode(ScreenMode::Inline)` with `.history([..])` lines above it is
  [the inline prompt](screen-modes.md) (`rtwk::binding::ScreenMode`).
  `Options::kitty_keyboard`, `.env(name, value)`, `.start_time(..)` and the
  terminal's own options (`.terminal(|o| ..)`) are set there too.
- `pump_app(AppView::new(state, view))` starts an app of your own state, and
  `read_app(|app: &State| ..)` and `update_app(|app: &mut State, cx| ..)` read and
  change it between steps. `pump_widget(widget)` starts a widget in a root of its
  own, which centers it: an app that fills the screen is `pump_app`, as above.
  `pump_app_in(app, |app| ..)` wraps the app in a widget of yours, a
  `TerminalBrightness` or a `Theme`, as [Theming](theming.md) does.
- Input draws nothing by itself: `t.pump()` after `type_text`, `press`, `paste`,
  `update_app` or a click, before reading the screen. `t.pump_for(duration)` moves
  the clock on by that much, running the app's timers, frames and animations due by
  then.
- What animates in is not there after one `pump`: a dialog fades in over 160 ms
  (the `fade_in` of `DialogOptions`), a tooltip over 100 ms, and a toast slides in over
  200 ms. Pump past it, `t.pump_for(Duration::from_millis(200))`; a dialog shown
  with `fade_in: Duration::ZERO` needs none.
- `type_text("text")` sends text as one read; `press("enter")`,
  `press("tab space")`, `press("ctrl+x ctrl+s")` send each key as a read of its
  own. `click(x, y)`, `click_on(&finder)` and `mouse(MouseKind::WheelDown, x, y,
  0, Mods::NONE)` (`rtwk_core::input`) work full screen; `resize(columns, rows)`.
- `t.act(|host| ..)` runs a closure on the app's host, as the app's own code runs,
  and pumps the frame that follows: for a controller or a listenable the app
  watches.

## Keys

`press` names keys as `KeyBindings` does: the modifiers, then the name, joined by
`+` (`ctrl+shift+left`), and a space between two keys (`"g g"`). A name is one
character, which stands for itself (`a`, `?`, and `A` for `shift+a`), or one of
`space`, `enter`, `tab`, `backspace`, `escape` (`esc` too), `up`, `down`, `left`,
`right`, `home`, `end`, `pageup`, `pagedown`, `insert`, `delete` and `f1` to
`f12`, in lower case. A name that is not one panics, naming the part of the
string. Each key is sent as the bytes a terminal sends for it, so what a terminal
cannot send is not sent: without the kitty protocol `ctrl+i` is Tab, and a key
with no bytes of its own (`ctrl+enter`) panics. Escape without the protocol moves
the clock on by the terminal's input timeout, the pause after which a lone ESC is
the Escape key; with `Options::kitty_keyboard(true)` it does not.

Keys sent by one `press("enter space")` are handled before the next frame is built,
as a burst from a real terminal is: pump between two keys where the second needs
what the first built.

## How many pumps

One `t.pump()` after the input draws everything the input changed.

- A timer, a debounce or a retry of the app is on the clock: `t.pump_for(duration)`
  past it.
- A change that the next frame starts takes a pump more. `t.pump_until_idle()`
  pumps as many as it takes: until a gap passes with no frame written and nothing
  of the app waiting. It runs out the timers that end what a test may look for (a
  toast, which shows for 3 s, is gone when it returns), and an app with a periodic
  timer or a spinner is never idle: it fails after 10 s of the clock, listing what
  waits, so pump such an app by a duration.
- What a `LayoutBuilder` builds is built in the frame after its layout, so what its
  build changes in the key bindings and the key hints is built in the frame after,
  which is asked for at once: after a `resize`, a pump more before reading the hints.
- What the app waits for in real time (a thread, a file, a process, a socket) is not
  on the clock at all: `wait_until("what", || ..)` of `rtwk_test` polls a condition,
  and `wait_for(&receiver, "what")` receives from a channel, each within a bound of
  real time that fails naming `what` instead of a test that hangs.
  `wait_until_with("what", Wait::new().between(|| ..), ..)` runs a closure between
  the polls (a pump, there) and `.state(..)` adds a state to the failure message.
  The futures of an app on `rtwk_tokio::TokioApp` run on the runtime of the test;
  the test then waits for their result as for any other thread's.

## Reading the screen

- `t.lines()` (rows without their trailing blanks), `t.screen()` (those joined),
  `t.marked_screen()` (with each run of one style marked, to compare two themes),
  `t.styled_screen()` (the screen-file format of the goldens), `t.scrollback()` (inline),
  `t.cell(x, y)`, `t.cursor()`.
- One widget's part of the screen: `t.lines_of(&finder)` gives the rows of its
  cells, `t.lines_in(rect)` those of a rectangle (`t.rect(&finder)` gives one), from
  its first column. `assert_line_in(line, finder)` and `assert_text_at_in(x, y,
  text, finder)` read the region alone, `x` and `y` from its first cell: a list's
  row in a screen of two panes that both show it.
- The asserts: `assert_line(line)` (a row, blanks at the ends aside),
  `assert_line_where(what, |line| ..)`, `assert_text_at(x, y, text)`,
  `assert_style_at(x, y, &has_fg(Color::RED))` with `has_attr` and `has_bg`,
  combined with `.and(..)` and `.not()`, `assert_cursor(x, y)`,
  `assert_screen("..")` for the whole screen (`assert_screen_styled` with the
  styles marked `[bold]..[/]`), and `assert_screen_file(path)` for golden files
  written with `TWK_UPDATE_GOLDENS=1`. The path is from the root of the package
  under test, whichever directory `cargo test` runs in; a golden holds the styles,
  `assert_screen_file_text` compares the text alone. Snapshots of
  [insta](https://insta.rs) work as well: `insta::assert_snapshot!(t.screen())`.
- Finders, from `rtwk_test::find`: `by_type::<W>()`, `by_key(key)`, `text("Save")`,
  `text_containing("Sav")`, `text_where(what, |text| ..)`, `widget_where(what, |w|
  ..)`, `descendant(of, matching)` and `ancestor(of, matching)`, narrowed with
  `.first()`, `.last()` and `.at(n)`; `assert_finds(&finder, n)`,
  `assert_finds_one`, `assert_finds_none` and `assert_finds_some`. `find::text`
  finds a `Text` whose whole string is the one given, not one that contains it,
  and only a `Text`: a `Button` draws its label itself, and is found by its type.
- What a finder finds, read: `t.widget::<W, _>(&finder, |w| ..)`,
  `t.state::<S, _>(&finder, |s| ..)`, `t.element(&finder)`,
  `t.render_object::<O, _>(&finder, |o| ..)` and `t.rect(&finder)`, its cells
  (`x`, `y`, `width`, `height`), which turns a widget's text into the cells to look
  at the style of.

The form of [Input and focus](input-and-focus.md), filled in from the keys and
with the mouse. A `Button` draws its label itself, so a finder for it is by its type
(`find::text` finds `Text`s only):

<!-- file: crates/rtwk/tests/recipes/testing_form.rs -->
```rust

use rtwk::app::AppView;
use rtwk::controls::Button;
use rtwk_test::{Options, Tester, find};

use crate::focus_app::{SignIn, view};

fn mounted() -> Tester {
    let mut t = Tester::with(Options::new().size(40, 10));
    t.pump_app(AppView::new(SignIn::default(), view));
    t
}

#[test]
fn the_name_tab_the_password_and_ctrl_s_saves() {
    let mut t = mounted();
    t.type_text("ann");
    t.press("tab");
    t.type_text("secret");
    t.press("ctrl+s");
    t.pump();
    t.assert_line_where("the greeting", |l| l.contains("hi, ann"));
    t.assert_line_where("the password, hidden", |l| l.contains("••••••"));
    // the second field holds what the screen hides
    t.read_app(|app: &SignIn| assert_eq!(&*app.password.text(), "secret"));
}

#[test]
fn a_click_on_the_button_with_no_name_asks_for_one() {
    let mut t = mounted();
    // a Button draws its label itself, so it is found by its type
    t.click_on(&find::by_type::<Button>());
    t.pump();
    t.assert_line_where("the request for a name", |l| l.contains("a name, please"));
    t.read_app(|app: &SignIn| assert_eq!(app.status, "a name, please"));
}
```

A widget on its own, without an app, a clock or input, is a `Shot`: its frames
written to an emulator, one at a time.

## In a real terminal

The tester is the app on a terminal in memory. To see the program as a user does,
`rtwk_test::run_process` and `start_process` run a program with a bound of real
time, killed with the test however the test ends, and [the CLI tools](cli.md)
record a session in a pseudo-terminal (`rtwk record`) and write a test of it
(`rtwk test-from-cast`). A prompt run as a command line is tested with
`rtwk::cli::FakeCliTerminal`.

`run_process` runs a program to its end and answers its output and exit status; it
is for a tool that needs no terminal, whose output is what is wanted. Here it runs
the tool of [Recipes](recipes.md#a-command-line-tool-rtwkcli-not-run_app) with its
output piped, as a script runs it. A package with a binary runs
`env!("CARGO_BIN_EXE_name")`; this one has none, so the test binary runs one test
of its own, which is the program:

<!-- file: crates/rtwk/tests/recipes/testing_process.rs -->
```rust

use std::process::Command;
use std::time::Duration;

use rtwk::cli::Cli;
use rtwk_test::run_process;

use crate::deploy::deploy;

/// The program: what `main` of a tool is. In a package with a binary the test runs
/// `env!("CARGO_BIN_EXE_name")`; this package has none, so the test binary runs this one test.
#[test]
#[ignore = "the program that the test below runs"]
fn the_program() {
    let deployed = deploy(&Cli::default(), "staging").unwrap();
    std::process::exit(if deployed { 0 } else { 1 });
}

#[test]
fn the_tool_on_a_pipe_answers_the_default_and_says_so_in_its_exit_code() {
    let out = run_process(
        Command::new(std::env::current_exe().unwrap()).args([
            "--ignored",
            "--exact",
            "testing_process::the_program",
            "--nocapture",
        ]),
        Duration::from_secs(60),
    )
    .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // nobody to ask: the default, written with the question
    assert!(stdout.contains("? Deploy to staging? no"), "{stdout}");
    assert!(stderr.contains("Not deployed."), "{stderr}");
    assert_eq!(out.status.code(), Some(1));
}
```

`start_process` is the same for a program that is still running when the test
looks at it: its `Process` is killed when it is dropped.

To see an app in a real terminal without writing a test, [the CLI tools](cli.md)
have the commands for it: `rtwk record` runs the app in a terminal and writes the
session, `rtwk play` plays it back, and `rtwk tape` runs a script of what to type
and what to wait for and writes the session and the screens it names (`--check`
runs the tape as a test: exit 1 when a `Wait` times out). A recording is a
regression test too: `rtwk test-from-cast` writes one, which plays it through the
emulator with `rtwk_test::screen_frames`. The program driven from outside, with
its keys as real bytes and its exit and the terminal it gave back checked, is the
crate `rtwk-driver`, not yet a section of this
guide.

## Time

No test waits on the wall clock. Only the app's timers and frames are on the fake
clock: `SystemTime::now()` and `Instant::now()` read real time. A widget that
shows the time reads `AppBinding::of(cx).now()`: the app's start time moved on by
its clock, which in a test starts at `Options::start_time(..)`
(`Options::DEFAULT_START_TIME`, 1 January 2026, by default) and moves as the test
pumps, so the screen is the same on every run. A `Calendar` given no date takes
today from it. `t.elapsed()` is the time the clock has moved on since the app
started.

`t.pump_until_idle()` runs out the timers that end what a test may look for, so a
toast, which shows for 3 s, is gone when it returns. To see one, pump by a
duration shorter than its own; an app with a periodic timer or a spinner is never
idle, and is pumped by a duration instead.

## What a test leaves behind

A test ends by quitting the app, as `t.quit()` does, with the widgets dropped. It
fails then when the app left a timer of its clock or a ticker running, a `Timer`
that nothing cancelled, naming each timer with the call that started it (debug
builds). `Options::allow_pending_timers()` is for a test that means it. There is no
check of controllers that were never disposed: ownership drops them.

## When a test fails

A failing assert shows the screen as text (`assert_style_at` with its styles
marked), the widget tree with each widget's size and the render tree. A failing test
then prints its last 3 frames that changed the screen (`Tester::frames_on_failure(n)`,
0 for none), and the app's log. `pump_until_idle` that fails lists what waits.

What the app prints does not reach the test's output while it passes, as on a real
terminal it would write over the screen: the warnings and errors the app reports (a
layout that overflowed, a build that panicked) are in `t.logs()`, the last of them
printed when the test fails, and what it wrote to stderr once it ended in
`t.stderr_lines()`.

An error the app logged fails the test when it ends, unless the test takes it:
`t.take_errors()` hands over the records at the error level logged since its last
call. A build that panicked is shown in place and the next frame may build over it,
so a test that passes has seen none, or said it expected them. The error the app
ended with is not among the records: the next call of the tester panics with it.
A flex, a wrap or a grid whose children overflowed it fails the test too (`Options::fail_on_overflow(false)`
for a test of what an overflow shows; a layout that clips on purpose says so with
`overflow_intended`).

<!-- not rtwk: SystemTime -->
