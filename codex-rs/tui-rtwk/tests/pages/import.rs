//! Screen tests of the `import` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/import/`.

use crate::common::slot;

#[test]
fn import_slot_is_registered_and_empty() {
    slot("import");
}
