//! Screen tests of the `sidebar` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/sidebar/`.

use crate::common::slot;

#[test]
fn sidebar_slot_is_registered_and_empty() {
    slot("sidebar");
}
