<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# The `rtwk` command line

<!-- not rtwk: Try Cargo -->

`rtwk` is the developer tool of the framework: it starts projects, runs and
watches apps, records and replays sessions and makes tests of them. Install
it from a checkout with `cargo install --path crates/rtwk-dev --locked`.
`rtwk --help` lists the commands and `rtwk <command> --help` has the options
of one.

## Commands and exit codes

Every command exits with one of the codes below. A wrong command line (an
unknown option, a missing argument) prints `rtwk: <what is wrong>` and
`Try 'rtwk --help'.` to stderr and exits with **2**; twk's tool uses 64 for
the same. Any other failure of the command itself prints `rtwk: <message>` to
stderr and exits with **1**, where the table does not say otherwise. Help is
printed to stdout and exits with 0.

| Command | Does | Exit codes |
| --- | --- | --- |
| `new <dir>` | Generates a project from an embedded template (`--list`, `--dry-run`, `--force`) | 0; 1 when the destination cannot be written |
| `run` | Launches the selected Cargo application (`--debug`, `--screen-reader`, `--plain`, below) | the application's own status; 128 + n when a signal n ended it; 2 for an unknown debug aid |
| `logs` | Tails the application's local log | 0 when stopped or the pipe closes; 1 |
| `inspect` | Asks a local Inspector | 0; 1 |
| `shell` | Serves this terminal to a local application | 0 when stopped with Ctrl+C or the app ends; 1 |
| `record` | Records a command to asciicast v2 | the command's status; 128 + n by a signal; 127 when it cannot be started |
| `play` | Replays a recording; a small player on a terminal | 0; 130 for Ctrl+C; 1 |
| `tape` | Runs a Tape script, writing asciicast or SVG; `--check` runs it as a test | 0; 1 when a `Wait` times out or the tape cannot be read; 127 when the program cannot be started; 128 + n by a signal |
| `decode` | Names the sequences, keys and frames of a recording or trace | 0; 1 when a file cannot be read or is neither |
| `test-from-cast` | Writes a test of a recording, below | 0; 1 when the recording cannot be read, the files cannot be written, the package lacks the dev-dependencies or the run that writes the screen files fails (the test and the recording stay); 2 for a wrong command line |
| `doctor` | Reports the terminal and the project setup, below (`-C <dir>`, `--json`) | 0; 1 when the rtwk crates of `Cargo.lock` differ in version or source, or the rustc is older than a `rust-version`; 2 for a wrong command line |

`record`, `play`, `tape` and `shell` need Unix for their terminal and
pseudo-terminal parts; `decode` and `test-from-cast` run anywhere.

## Typing a recording into a command: `rtwk record --replay`

```bash
rtwk record --replay demo.cast --replay-speed asap --replay-then end --out again.cast -- cargo run
```

`--replay <file>` types the keys of an earlier recording (one made with
`--keys`) into the command, once the command has written; the terminal's own
answers in that recording are left out (the splitter is twk's) and this
terminal answers the command's queries again. `--replay-speed <n|asap>` is 1
by default: a number types the keys at the recorded times divided by it;
`asap` types each key once the command has shown what it showed at that key
and has been quiet for 100 ms (waiting at most 2 s). `--replay-then
<handover|end>`: `handover` (the default) gives this terminal to the command
after the last key; `end` hangs the command up once it has been quiet for a
second and exits 0. While the replay goes on the person's keys are held back;
Ctrl+C goes through, and `--marker-key` is not read. `--replay-speed` and
`--replay-then` without `--replay`, and a recording that cannot be read, are
usage errors (exit 2). The design is
dev-record-play.md.

## Running an app with the debug aids

```bash
rtwk run --manifest-path Cargo.toml --bin demo --debug overlay,frames --screen-reader
rtwk run --manifest-path Cargo.toml --bin demo --debug rainbow --debug timeline --plain
```

| Flag | The app is run with |
| --- | --- |
| `--debug <aids>` | `TWK_DEBUG` set to the names it had already, those given here (comma-separated, or the flag more than once) and `keys`, each once. The debug keys are always on, as in twk |
| `--screen-reader` | `TWK_SCREEN_READER=1`: the app in its screen-reader mode |
| `--plain` | `TWK_PLAIN=1`: the app writes plain text with no escape sequences |

The aids are `keys`, `overlay`, `console`, `rainbow`, `relayout`, `rebuilds`,
`frames`, `timeline` and `logs`; any other name is a usage error that lists them.

## What the setup is: `rtwk doctor`

```bash
rtwk doctor            # for a person
rtwk doctor --json     # for a tool or an agent: one object
rtwk doctor -C ../app  # for the project the directory is in
```

It prints what this terminal does (the `TERM`, `TERM_PROGRAM` and `COLORTERM` it
has; its name and version as it answers XTVERSION; colors, synchronized output,
kitty keyboard, graphics, links, the size of a cell; how long it took to answer;
a multiplexer; a remote terminal; plain output), then the project: the rustc and
the highest `rust-version` the project's packages and the rtwk crates declare,
the rtwk crates of the project's `Cargo.lock` (their versions and where they come
from) and an app that is running in the project's `target/rtwk` (its local log
and shell, and whether they take a connection). Where stdin or stdout is not a
terminal (a pipe, an agent), only the project part is reported, and `terminal` is
`null` in the JSON.

The rtwk crates of a project must be one release from one source; **the exit
code is 1** where two versions or two sources are in the lock, or the rustc is
older than a `rust-version`. A rustc or cargo that cannot be asked is reported
and is no failure. The JSON has `agree` (the crates are one release) and `ok`
(what the exit code says), and its Terminal part is the one twk's `doctor --json`
prints. rtwk's own terminal answers XTVERSION `rtwk-term <version>`; the design is
dev-doctor.md.

## A test from a recording

A recording of a session can be the test of it: each time the session
showed a screen you care about, the test holds the emulator's screen at that
time to a **screen file** (the text and the style of every cell), and fails
with a diff when it changes.

```bash
rtwk record --marker-key ctrl+] --out tests/demo.cast -- cargo run
# run the app, press Ctrl+] at each screen worth keeping, quit it
rtwk test-from-cast tests/demo.cast
cargo test --test demo_cast
```

`test-from-cast` writes three things, all from the root of the package of
the current directory (or the package of `--out`):

- `tests/demo_cast.rs`, the test: it reads the recording, plays it through
  `rtwk_test::screen_frames` and has one `#[test]` for every marker, named
  after its label (`marker_1_first`, or `marker_2_at_0_500_s` where the marker
  has none), one for the emulator having taken all of the output, and
  nothing of the screens in it;
- `tests/demo.cast`, a copy of the recording beside the test (it is used where
  it is when it is already there; another recording of that name is refused
  with exit 1 unless `--force`);
- `tests/goldens/demo/<k>.txt`, the screen file of the k-th marker (k from 1
  in the order of the markers), or `tests/goldens/demo/last.txt` for a
  recording with no marker, whose last screen is held.

It writes the screen files by running the test once with
`TWK_UPDATE_GOLDENS=1 cargo test --test demo_cast` (with the cargo of
`$CARGO` when that is set; cargo builds the test first, so the first run
takes as long as a build, and the command waits 15 minutes at most), and
lists the files it wrote. `--no-goldens` leaves that to you, and `-o`/`--out` puts the test
elsewhere. A change of the app that is meant is accepted the same way:

```bash
TWK_UPDATE_GOLDENS=1 cargo test --test demo_cast
```

A screen that differs leaves what was shown beside the screen file as
`1.txt.actual` and fails with what differs; add `*.actual` to `.gitignore`.

### What the test holds

The screen of a marker is the **frame** shown at or before its time, where a
frame is the output up to a write that leaves no synchronized update (DEC mode
2026) open: the same rule as `rtwk play`'s frame step and
`rtwk_test::screen_frames`. A marker at the very time a frame was shown holds
that frame. A marker before any frame has nothing to hold: its test passes
and says `skipped: no frame at or before 0.000 s` on stderr (a Rust test has
no way to be skipped by what it finds). The emulator check fails when the
recording has something it cannot take.

A label is written as a comment and in the name of the test only after it is
cut to letters and digits and quoted, with every control, bidirectional or
invisible character as a `\u{..}` escape, so no label can add a line or a
token to the code.

### What the package needs

The test uses `rtwk-test` and `rtwk-dev` (for the recording reader), so the
package lists both under `[dev-dependencies]`. `test-from-cast` writes its
files and then says which is missing and exits with 1; the files stay.
`rtwk-test`'s `Cargo.toml` does not set `publish = false`. The paths below
work inside this repository or beside a checkout of it:

```toml
[dev-dependencies]
rtwk-test = { path = "../rtwk/crates/rtwk-test" }
rtwk-dev = { path = "../rtwk/crates/rtwk-dev" }
```

A package with `autotests = false` does not find `tests/demo_cast.rs`
by itself; the command says so and prints the target to add:

```toml
[[test]]
name = "demo_cast"
path = "tests/demo_cast.rs"
```

### In this repository

`crates/rtwk-dev/tests/{plan,quiet,early}.cast` have their tests and screen
files checked in (`*_cast.rs`, `goldens/`), written by the command and run
by `cargo nextest run -p rtwk-dev` like any test. A unit test holds each
source equal to what the command writes now, so a change of the generator
that changes the source fails until they are written again:

```bash
cd crates/rtwk-dev
for n in plan quiet early; do cargo run -q -- test-from-cast tests/$n.cast; done
```

The oracle is twk's `cast_test_gen.dart`: `tests/oracle/cast_test_gen.dart`
runs twk's generator over thirteen recordings and records what it decides (the
times of the markers, the checks and their screen files, the frames, what
the emulator could not take, the text of the screen files);
`tests/data/cast_test_gen/cast_test_gen.golden.json` is what the Rust test
compares with. To write it again from the checkout `oracle/TWK_REF` names:

```bash
dart --packages="$TWK/.dart_tool/package_config.json" \
  crates/rtwk-dev/tests/oracle/cast_test_gen.dart \
  > crates/rtwk-dev/tests/data/cast_test_gen/cast_test_gen.golden.json
```
