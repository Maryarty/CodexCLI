//! Screen tests of the `composer` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/composer/`.

use crate::common::slot;

#[test]
fn composer_slot_is_registered_and_empty() {
    slot("composer");
}
