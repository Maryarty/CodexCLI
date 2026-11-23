//! The `/status` cells of a transcript: the card of a command is an entry of
//! the thread it ran in, found again by the refresh of the limits it asked for
//! and by the copy picker.

use std::sync::Arc;

use super::{Item, Transcript};
use crate::screens::status::{Limits, StatusCard};

impl Transcript {
    /// The card of the newest `/status` entry (oracle `last_status_copy_targets`:
    /// the copy picker offers the last one).
    pub fn last_status(&self) -> Option<&Arc<StatusCard>> {
        self.order
            .iter()
            .rev()
            .find_map(|key| match self.item(key) {
                Some(Item::Status { card, .. }) => Some(card),
                _ => None,
            })
    }

    /// The refreshes the cells of this transcript still wait for.
    pub fn waiting_status(&self) -> Vec<u64> {
        self.order
            .iter()
            .filter_map(|key| match self.item(key) {
                Some(Item::Status { request, .. }) => *request,
                _ => None,
            })
            .collect()
    }

    /// The refresh `request` ended: its cell takes `limits` and stops waiting.
    /// A request nobody waits for (an earlier account's, a cell already
    /// finished) changes nothing (oracle `finish_status_rate_limit_refresh`).
    pub(super) fn finish_status(&mut self, request: u64, limits: Limits) {
        let Some(key) = self
            .order
            .iter()
            .find(|key| {
                matches!(
                    self.item(key),
                    Some(Item::Status { request: Some(waiting), .. }) if *waiting == request
                )
            })
            .cloned()
        else {
            return;
        };
        let Some(Item::Status { card, .. }) = self.item(&key) else {
            return;
        };
        let mut card = card.clone();
        Arc::make_mut(&mut card).limits = limits;
        self.upsert(
            key,
            Item::Status {
                card,
                request: None,
            },
        );
    }
}
