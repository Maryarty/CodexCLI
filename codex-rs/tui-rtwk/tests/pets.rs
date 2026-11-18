//! Screen tests of the `pets` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/pets/`.

use crate::common::slot;

#[test]
fn pets_slot_is_registered_and_empty() {
    slot("pets");
}
