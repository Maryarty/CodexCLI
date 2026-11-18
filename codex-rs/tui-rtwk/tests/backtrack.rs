//! Screen tests of the `backtrack` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/backtrack/`.

use crate::common::slot;

#[test]
fn backtrack_slot_is_registered_and_empty() {
    slot("backtrack");
}
