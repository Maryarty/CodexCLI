//! Screen tests of the `onboarding` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/onboarding/`.

use crate::common::slot;

#[test]
fn onboarding_slot_is_registered_and_empty() {
    slot("onboarding");
}
