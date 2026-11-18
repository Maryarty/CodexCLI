//! Screen tests of the `agents` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/agents/`.

use crate::common::slot;

#[test]
fn agents_slot_is_registered_and_empty() {
    slot("agents");
}
