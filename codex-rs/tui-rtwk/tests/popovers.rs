//! Screen tests of the `popovers` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/popovers/`.

use crate::common::slot;

#[test]
fn popovers_slot_is_registered_and_empty() {
    slot("popovers");
}
