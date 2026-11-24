# oracle-strings

Run from the repository root: `python3 -I tools/rtwk-port/oracle-strings/gen.py`. Writes
`codex-rs/tui-rtwk/src/oracle_strings/{transcript,composer,settings,cells,approvals,commands,status,live}.rs`
and `mod.rs`: one `pub const NAME: &str = "..."; // oracle <file>:<line>` per UI string literal of the
oracle (`codex-rs/tui/src`) that the port does not have yet. A second run changes nothing.

- Module -> package: the regexes of `tools/rtwk-port/map-full/modules.py` (first match), owners merged into
  the 8 W2 packages (`PACKAGES` in gen.py, briefs.md part A). Test files (`*_tests.rs`, `tests/`,
  `*test_support*`, `snapshots/`, `bin/`) and `#[cfg(test)]` / `#[test]` items are skipped.
- Dropped: literals in logs (`debug!`, `info!`, ...), `panic!`/`assert*`/`expect`/`context`, attributes, key and
  path calls (`get`, `starts_with`, `Path::new`, ...), `==` / match patterns, `json!`; keys, ids, paths, env
  names, glyph-only and placeholder-only strings (`{}s`, `tui.keymap.{action}`).
- "Port already has it": an exact match with any literal in `src/`, `core/src/`, `host/src/` of the port
  (outside `src/oracle_strings/`). A porter who wires a constant into the view's `strings.rs` with the same
  text makes it drop out on the next run.
- Format strings keep their placeholders (`{err}`, `{}`); names are the module plus the first words, `_2` on
  a clash; the same text in several places is one constant, `(+N more)` in the comment.
- The tables are UI crate only (no English in core); `rules.rs` lets `src/oracle_strings/` hold literals and
  `layers.rs` knows the folder. `#[rustfmt::skip]` on the `mod` lines keeps the generated lines as written.
