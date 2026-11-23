//! What a feature and a surface say to each other without knowing each other (ADR-013 Ж and И).
//!
//! A feature that wants something shown (the setup page of an installed plugin) does not open a
//! page: it queues a `SurfaceWish`, and the surface's presenter shows it once and says so
//! (`Intent::WishShown`). A surface that shows a feature's data does not ask for it: it says it
//! is interested (`Intent::Interest`), and the feature loads on the first and may let go on the
//! last. After a reconnect every interest still held is asked again.

use std::collections::VecDeque;

use super::{FeatureInterest, FeatureWish};

/// One thing a feature wants shown, once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWish {
    /// Numbers the wishes in the order they were made; what a presenter says it showed.
    pub seq: u64,
    /// What the wish is about (a directory): a wish whose scope is gone is shown by no one.
    pub scope: String,
    pub wish: FeatureWish,
}

/// The wishes nobody has shown yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wishes {
    queue: VecDeque<SurfaceWish>,
    seq: u64,
}

impl Wishes {
    pub(crate) fn push(&mut self, scope: String, wish: FeatureWish) {
        self.seq += 1;
        self.queue.push_back(SurfaceWish {
            seq: self.seq,
            scope,
            wish,
        });
    }

    /// The wish `seq` was shown (or is given up): it is gone from the queue.
    pub(crate) fn shown(&mut self, seq: u64) {
        self.queue.retain(|wish| wish.seq != seq);
    }

    /// The wishes still to show, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = &SurfaceWish> {
        self.queue.iter()
    }
}

/// What a surface says of its interest: made only by the `Interest` widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterestStep {
    Open(FeatureInterest),
    Close(FeatureInterest),
}

/// The interests surfaces hold, with how many surfaces hold each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Interests {
    held: Vec<(FeatureInterest, u32)>,
}

impl Interests {
    /// One more surface wants `interest`; true when it is the first.
    pub(crate) fn raise(&mut self, interest: &FeatureInterest) -> bool {
        match self.held.iter_mut().find(|(held, _)| held == interest) {
            Some((_, count)) => {
                *count += 1;
                false
            }
            None => {
                self.held.push((interest.clone(), 1));
                true
            }
        }
    }

    /// One surface let go of `interest`; true when it was the last.
    pub(crate) fn lower(&mut self, interest: &FeatureInterest) -> bool {
        let Some(at) = self.held.iter().position(|(held, _)| held == interest) else {
            return false;
        };
        if self.held[at].1 > 1 {
            self.held[at].1 -= 1;
            return false;
        }
        self.held.remove(at);
        true
    }

    /// How many surfaces want `interest`.
    pub fn count(&self, interest: &FeatureInterest) -> u32 {
        self.held
            .iter()
            .find(|(held, _)| held == interest)
            .map_or(0, |(_, count)| *count)
    }

    /// Every interest held, for the asks a reconnect makes again.
    pub(crate) fn all(&self) -> Vec<FeatureInterest> {
        self.held.iter().map(|(interest, _)| interest.clone()).collect()
    }
}
