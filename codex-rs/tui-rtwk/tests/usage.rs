//! Screen tests of the `usage` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/usage/`.

use crate::common::slot;

#[test]
fn usage_slot_is_registered_and_empty() {
    slot("usage");
}
