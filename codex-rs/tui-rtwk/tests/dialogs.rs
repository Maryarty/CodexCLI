//! Screen tests of the `dialogs` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/dialogs/`.

use crate::common::slot;

#[test]
fn dialogs_slot_is_registered_and_empty() {
    slot("dialogs");
}
