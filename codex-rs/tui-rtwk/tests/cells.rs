//! Screen tests of the `cells` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/cells/`.

use crate::common::slot;

#[test]
fn cells_slot_is_registered_and_empty() {
    slot("cells");
}
