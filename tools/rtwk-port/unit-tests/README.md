# Moving the in-src unit tests into the test roots

Why: every `#[cfg(test)] mod` in a src file is part of that crate's `lib(test)` build, which
cargo builds (and links) on every call that names no test root (build-speed.md section 8).
The same tests as modules of a root that is built anyway cost nothing extra.

## Run

```
python3 -I tools/rtwk-port/unit-tests/move.py --list files.txt     # one src path per line
python3 -I tools/rtwk-port/unit-tests/move.py codex-rs/tui-rtwk/src/widgets/age.rs ...
```

Paths are from the repo root or from `codex-rs/tui-rtwk`. Rerunnable: a file with no
`#[cfg(test)] mod` left in it is skipped; a file in `kept.txt` is tried again (so a rerun after a
visibility change in src picks it up). `--no-check` writes without the compile check: do not use it
(the files that do not compile stay moved and broken).

What it does, per listed file (core: `core/src`, `core/tests/unit`, root `core/tests/all.rs`;
UI crate: `src`, `tests/unit`, root `tests/screens.rs`):

1. Each top-level `#[cfg(test)] mod name { .. }` (or `mod name;` with its file) becomes
   `<unit dir>/<path below src>.rs` (a second module of one file: `<path>__<name>.rs`).
   `use super::*` becomes `use <crate>::<module path>::*` (module paths come from the `mod x;`
   declarations below `lib.rs`, `#[path]` and macro-declared modules included); `super::` and
   `crate::` in the body are rewritten; the parent's private `use` lines the body names are
   copied over; `include_str!("rel")` is re-pointed at the old place.
2. `<unit dir>/mod.rs` is generated: one `#[path]` line per moved file (flat module names, so a
   test is `unit::<path_with_underscores>::<test>`), with
   `#![allow(unused_imports, clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]`
   (the tests directory may use them: clippy.toml). `mod unit;` is added to the root once.
3. `acargo check -q --tests -p <crate>` runs on the whole batch. Every module whose unit file
   (or src file) has a compile error is put back into its src file exactly as it was, the unit file
   is deleted, and the check runs again until clean. Errors outside the moved modules stop the
   script with the batch applied.

## kept.txt

`<src file>: <first compiler error>`: the file's tests stay in src because they do not compile
from outside the crate: a private module / function / method / field / type, a
`#[non_exhaustive]` struct literal (all E0603, E0624, E0616, E0425 on a private item, E0638).
Opening them is a visibility decision, not mechanics: the script never changes `pub` in src. The
error line points at the unit file the module had when it failed (deleted again).

## The files of running families

The 65 files the first run covered are those no family branch touched. The rest
(`git grep -l '^#\[cfg(test)\]' -- 'codex-rs/tui-rtwk/**/src/**/*.rs'` minus the files already
done) belongs to task `unit-tests-to-roots`: run

```
git grep -l -E '^#\[cfg\(test\)\]' -- 'codex-rs/tui-rtwk/src' 'codex-rs/tui-rtwk/core/src' > files.txt
python3 -I tools/rtwk-port/unit-tests/move.py --list files.txt
```

after the families have landed (merge first: the script rewrites src files, a family edit of the
same file would conflict).

## Check that it worked

```
acargo nextest run -p codex-tui-core -p codex-tui-rtwk --test all --test screens --test rules \
  --status-level fail --final-status-level fail --hide-progress-bar \
  -E 'test(/unit::/) | binary(rules)'
acargo nextest run -p codex-tui-core -p codex-tui-rtwk --lib ...   # what is left in src
```
