//! What `account/rateLimits/read` gave and why it was asked: the origin of a
//! read, the buckets and reset credits of its answer, and the buckets the app
//! keeps per backend (oracle `RateLimitRefreshOrigin`,
//! `app_server_rate_limit_snapshots`, `rate_limit_snapshots_by_limit_id`).
//! The clock is a fact the caller passes (`now`), never read here.

use std::collections::BTreeMap;

use codex_app_server_protocol as protocol;

use super::limits::{Credits, Snapshot, SpendLimit, Window};
use crate::threads::limits::{self, Sample, Source, Warning};

/// Why a read was asked; the app keeps it with the read's ticket and the
/// answer goes where the origin says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateLimitOrigin {
    /// Cards of `/status` wait for it, the request counts them (oracle
    /// `StatusCommand`).
    Status(u64),
    /// The usage menu opened (oracle `UsageMenu`).
    UsageMenu,
    /// The reset picker opened (oracle `ResetPicker`): the request number the list waits for.
    ResetPicker(u64),
    /// The usage menu opened over a count known to be zero (oracle `UsageMenu`): the request
    /// number the menu waits for.
    Menu(u64),
    /// The startup prefetch that looks for resets to tell the user of (oracle
    /// `StartupPrefetch`): the request number of the check.
    Hint(u64),
    /// The read after a reset was redeemed, for the count that is left (oracle `ResetConsume`).
    ResetConsume,
    /// Nothing waits for it: the prefetch at connect and the refreshes the
    /// app starts itself, after a reconnect or a limit that was reached
    /// (oracle `StartupPrefetch`, `Recovery`).
    Startup,
}

impl RateLimitOrigin {
    /// Whether a read of the same backend that is already in flight answers
    /// this one too. Only a refresh nobody waits for does: a card, a menu or a
    /// picker is asked its own read (the oracle's `start` coalesces `Periodic`
    /// and `Recovery` only).
    pub fn folds_into_a_read_in_flight(self) -> bool {
        matches!(self, Self::Startup)
    }
}

/// What a reset credit is for (`RateLimitResetType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetKind {
    CodexRateLimits,
    Unknown,
}

/// Where a reset credit stands (`RateLimitResetCreditStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditStatus {
    Available,
    Redeeming,
    Redeemed,
    Unknown,
}

/// One reset credit the backend listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetCredit {
    pub id: String,
    pub kind: ResetKind,
    pub status: CreditStatus,
    /// Unix seconds.
    pub granted_at: i64,
    /// Unix seconds; none when it does not expire.
    pub expires_at: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// The reset credits of a read (`RateLimitResetCreditsSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetCredits {
    pub available_count: i64,
    /// The detail rows; none means only the count is known, an empty list
    /// that the details were fetched and no credit is available. The backend
    /// may cap the list below the count.
    pub credits: Option<Vec<ResetCredit>>,
}

/// One bucket of a read under its limit id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bucket {
    pub id: String,
    /// `captured_at` is 0 until the store stamps it (`RateLimitStore::install`).
    pub snapshot: Snapshot,
}

/// The answer of `account/rateLimits/read` as the app keeps it: the buckets,
/// the primary one first, and the reset credits when the backend sent them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RateRead {
    pub buckets: Vec<Bucket>,
    pub reset_credits: Option<ResetCredits>,
    /// What the plan's own bucket (`codex`) says that the warnings depend on; none when the read
    /// named no such bucket.
    pub codex: Option<Sample>,
}

/// The id the oracle gives a bucket whose snapshot names none.
const CODEX: &str = "codex";

impl RateRead {
    // oracle: app_server_session.rs fn app_server_rate_limit_snapshots (the top-level view
    // first, then the map's other buckets, none twice) and chatwidget/rate_limits.rs
    // fn on_rate_limit_snapshot (the key and the name of a bucket)
    pub fn from_response(response: &protocol::GetAccountRateLimitsResponse) -> Self {
        let primary_id = response.rate_limits.limit_id.as_deref();
        let mut buckets = vec![bucket(&response.rate_limits)];
        if let Some(by_id) = &response.rate_limits_by_limit_id {
            // A map has no order: the others go by their keys, as the store keeps them.
            let mut others: Vec<(&String, &protocol::RateLimitSnapshot)> = by_id
                .iter()
                .filter(|(key, snapshot)| {
                    !primary_id.is_some_and(|primary| {
                        primary == key.as_str() || Some(primary) == snapshot.limit_id.as_deref()
                    })
                })
                .collect();
            others.sort_by(|a, b| a.0.cmp(b.0));
            buckets.extend(others.into_iter().map(|(_, snapshot)| bucket(snapshot)));
        }
        let codex = std::iter::once(&response.rate_limits)
            .chain(response.rate_limits_by_limit_id.iter().flat_map(|by_id| by_id.values()))
            .map(Sample::of)
            .find(|sample| sample.codex);
        Self {
            buckets,
            reset_credits: response.rate_limit_reset_credits.as_ref().map(reset_credits),
            codex,
        }
    }
}

fn bucket(snapshot: &protocol::RateLimitSnapshot) -> Bucket {
    let id = snapshot
        .limit_id
        .clone()
        .unwrap_or_else(|| CODEX.to_string());
    let window = |window: &protocol::RateLimitWindow| Window {
        used_percent: window.used_percent,
        window_minutes: window.window_duration_mins,
        resets_at: window.resets_at,
    };
    Bucket {
        snapshot: Snapshot {
            name: snapshot.limit_name.clone().unwrap_or_else(|| id.clone()),
            captured_at: 0,
            primary: snapshot.primary.as_ref().map(window),
            secondary: snapshot.secondary.as_ref().map(window),
            credits: snapshot.credits.as_ref().map(|credits| Credits {
                has_credits: credits.has_credits,
                unlimited: credits.unlimited,
                balance: credits.balance.clone(),
            }),
            individual_limit: snapshot.individual_limit.as_ref().map(|limit| SpendLimit {
                captured_at: None,
                remaining_percent: limit.remaining_percent,
                used: limit.used.clone(),
                limit: limit.limit.clone(),
                resets_at: limit.resets_at,
            }),
        },
        id,
    }
}

fn reset_credits(summary: &protocol::RateLimitResetCreditsSummary) -> ResetCredits {
    ResetCredits {
        available_count: summary.available_count,
        credits: summary.credits.as_ref().map(|credits| {
            credits
                .iter()
                .map(|credit| ResetCredit {
                    id: credit.id.clone(),
                    kind: match credit.reset_type {
                        protocol::RateLimitResetType::CodexRateLimits => ResetKind::CodexRateLimits,
                        protocol::RateLimitResetType::Unknown => ResetKind::Unknown,
                    },
                    status: match credit.status {
                        protocol::RateLimitResetCreditStatus::Available => CreditStatus::Available,
                        protocol::RateLimitResetCreditStatus::Redeeming => CreditStatus::Redeeming,
                        protocol::RateLimitResetCreditStatus::Redeemed => CreditStatus::Redeemed,
                        protocol::RateLimitResetCreditStatus::Unknown => CreditStatus::Unknown,
                    },
                    granted_at: credit.granted_at,
                    expires_at: credit.expires_at,
                    title: credit.title.clone(),
                    description: credit.description.clone(),
                })
                .collect()
        }),
    }
}

/// The buckets of one backend's account, as the full reads left them (oracle
/// `rate_limit_snapshots_by_limit_id`): a read replaces the buckets it names
/// and keeps the others, a rolling update never writes them, only an account
/// change clears them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RateLimitStore {
    snapshots: BTreeMap<String, Snapshot>,
    reset_credits: Option<ResetCredits>,
    limits: limits::Limits,
}

impl RateLimitStore {
    /// A read arrived at `now` (unix seconds): its buckets are captured then. The warnings that
    /// became due with it are returned, once each.
    pub fn install(&mut self, read: RateRead, now: i64) -> Vec<Warning> {
        for Bucket { id, mut snapshot } in read.buckets {
            snapshot.captured_at = now;
            self.snapshots.insert(id, snapshot);
        }
        self.reset_credits = read.reset_credits;
        match &read.codex {
            Some(sample) => self.limits.take(sample, Source::Read),
            None => {
                self.limits.read_failed();
                Vec::new()
            }
        }
    }

    /// A rolling update of the codex bucket arrived: the warnings that became due with it. It
    /// never writes the buckets.
    pub fn rolling(&mut self, sample: &Sample) -> Vec<Warning> {
        self.limits.take(sample, Source::Rolling)
    }

    /// The facts of the warnings and the composer's notice.
    pub fn limits(&self) -> &limits::Limits {
        &self.limits
    }

    /// A read is sent, or failed.
    pub fn limits_mut(&mut self) -> &mut limits::Limits {
        &mut self.limits
    }

    /// The account changed: what was read for the old one is not the new one's.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The buckets by limit id.
    pub fn snapshots(&self) -> impl Iterator<Item = &Snapshot> {
        self.snapshots.values()
    }

    /// The reset credits of the last read; none when it carried none.
    pub fn reset_credits(&self) -> Option<&ResetCredits> {
        self.reset_credits.as_ref()
    }
}
