//! Screen tests of the `settings` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/settings/`.

use crate::common::slot;

#[test]
fn settings_slot_is_registered_and_empty() {
    slot("settings");
}
