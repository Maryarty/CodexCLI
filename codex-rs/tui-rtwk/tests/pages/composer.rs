//! Screen tests of the `composer` directory. Its editing keys are unit
//! tests of `src/pages/session/composer/editing.rs` (the oracle's
//! `textarea.rs` state tests), its screens the oracle shards
//! `composer_1..4`, `composer_popups` and `textarea`.

use crate::common::slot;

#[test]
fn composer_slot_is_registered() {
    slot("composer");
}
