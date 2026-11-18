//! Screen tests of the `approvals` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/approvals/`.

use crate::common::slot;

#[test]
fn approvals_slot_is_registered_and_empty() {
    slot("approvals");
}
