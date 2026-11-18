//! Screen tests of the `chat` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/chat/`.

use crate::common::slot;

#[test]
fn chat_slot_is_registered_and_empty() {
    slot("chat");
}
