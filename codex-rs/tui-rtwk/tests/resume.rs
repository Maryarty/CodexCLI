//! Screen tests of the `resume` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/resume/`.

use crate::common::slot;

#[test]
fn resume_slot_is_registered_and_empty() {
    slot("resume");
}
