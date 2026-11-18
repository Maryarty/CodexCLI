//! Screen tests of the `settings` slot (S15). The slot is still the empty
//! form: the four views (configuration panel, syntax theme picker,
//! status-line setup, keymap picker) wait for the dialogs-pane slice, which
//! draws the oracle's bottom pane. Their parity tests (the oracle's rows,
//! `config_panel_renders_each_group_at_a_stable_height`,
//! `config_global_search`, `setup_view_snapshot_uses_runtime_preview_values`,
//! `model_theme_files_preview_select_and_restore`, `picker_content_snapshot`
//! and the key tests) are listed as open in
//! `docs/rtwk-port/parity/screens-dialogs.md`; the state tests that carry
//! them today are in `core/tests/settings.rs`.

use crate::common::slot;

#[test]
fn settings_slot_is_registered_and_empty() {
    slot("settings");
}
