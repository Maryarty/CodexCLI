//! Screen tests of the `voice` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/voice/`.

use crate::common::slot;

#[test]
fn voice_slot_is_registered_and_empty() {
    slot("voice");
}
