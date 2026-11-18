//! Screen tests of the `pager` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/pager/`.

use crate::common::slot;

#[test]
fn pager_slot_is_registered_and_empty() {
    slot("pager");
}
