//! Screen tests of the `transcript` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/transcript/`.

use crate::common::slot;

#[test]
fn transcript_slot_is_registered_and_empty() {
    slot("transcript");
}
