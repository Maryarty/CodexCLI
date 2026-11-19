//! Screen tests of the `startup` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/startup/`.

use crate::common::slot;

#[test]
fn startup_slot_is_registered_and_empty() {
    slot("startup");
}
