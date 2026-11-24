# keymap-actions

Run from the repository root: `python3 -I tools/rtwk-port/keymap-actions/gen.py`. Generates
`codex-rs/tui-rtwk/core/src/features/settings/keymap_actions.rs` (the `/keymap` picker's action
table: context, label, action, description, feature gate) from the oracle's
`keymap_setup/actions.rs`; a second run changes nothing.
