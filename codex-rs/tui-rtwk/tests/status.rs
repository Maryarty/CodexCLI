//! Screen tests of the `status` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/status/`.

use crate::common::slot;

#[test]
fn status_slot_is_registered_and_empty() {
    slot("status");
}
