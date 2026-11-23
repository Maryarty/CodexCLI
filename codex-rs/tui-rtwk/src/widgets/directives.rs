//! The one-shots a feature leaves for a surface (ADR-013 Ж `Directive`): a restore, a scroll to
//! a place, a focus. The listener reads one by its `seq`, once, so a rebuild or a second reader
//! never carries it out twice; it is the only place that does (rules test
//! `adr013_directives_read_in_one_place`). The pilot of `/plugins` has none: the first family
//! that does (the composer's restore) adds its `Directive<T>` and reads it through `Seen`.

/// What the listener has carried out: the `seq` of the last directive it took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen(u64);

impl Seen {
    /// Whether the directive numbered `seq` is new to this listener; it is then marked taken.
    pub fn take(&mut self, seq: u64) -> bool {
        if seq > self.0 {
            self.0 = seq;
            true
        } else {
            false
        }
    }
}
