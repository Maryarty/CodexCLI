# registry (fw-hubs-c5)

The registration lists are generated from the files, so parallel packages never collide on a list
line: `codex-rs/tui-rtwk/test-mods/registry.rs` (std only, one source of `core/build.rs`,
`tui-rtwk/build.rs` and the rules) writes `feature_list!`, `sheet_list!`, `action_list!`,
`mode_list!`, `cell_list!` and the `pub mod` lines; `test-mods/scan.rs` writes the test roots and
`core_all!` (core/tests/all.rs).

- `headers.py` - one-time migration: the `// cell: <Item pattern>` head lines into cells/*.rs.
- `fmt.sh` - rustfmt for the generated modules (cargo fmt does not follow them); `fmt.sh --check`.
