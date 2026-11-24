# Test roots written by build.rs (fw-ui-reg-2)

The roots `tests/{screens,pages,oracle,shell}.rs` say `tui_test_mods::<root>!();`; the
macros are written by `codex-rs/tui-rtwk/test-mods/build.rs` from the files of
`tests/{app,pages,sheets,oracle,services}` (`test-mods/scan.rs`). A new test file is a file: a
file of `pages/`, `sheets/`, `app/` is a module of `screens` unless its first lines hold
`// test root: pages`. `adopt.py` is the one-off that wrote the markers from the hand lists of the
old `tests/pages.rs` and moved the `tests/rtwk_*.rs` probes into `tests/pages/`. The rule
`test_roots_have_no_hand_mods` keeps the roots free of `mod` lines.

rustfmt does not follow the modules a macro declares: format these files with
`rustfmt --edition 2024 tests/{app,pages,sheets,oracle,services}/*.rs` from `codex-rs/tui-rtwk`
(`--check` to test; clean at the commit that introduced the lists).
