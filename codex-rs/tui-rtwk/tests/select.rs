//! Screen tests of the `select` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/select/`.

use crate::common::slot;

#[test]
fn select_slot_is_registered_and_empty() {
    slot("select");
}
