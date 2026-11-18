//! Screen tests of the `ui_tree` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/ui_tree/`.

use crate::common::slot;

#[test]
fn ui_tree_slot_is_registered_and_empty() {
    slot("ui_tree");
}
