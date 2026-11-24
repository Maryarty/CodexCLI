//! What the account's rate limits say that the thread has to tell: the one-shot warnings at 50, 75,
//! 90 and 95 percent used, and the window the composer's notice names (oracle
//! `chatwidget/rate_limits.rs` `RateLimitWarningState`, `chatwidget/usage_notice.rs`
//! `UsageNoticeState`). The model is pure: a sample of a snapshot goes in, the warnings that are
//! due come out; nothing here is drawn (ADR-013). The words and the notice line are the view's.
//!
//! A warning is a one-shot signal: each threshold is passed once, upwards, and never again for the
//! account, however the percentage moves (the oracle's `secondary_index` and `primary_index`).

use codex_app_server_protocol as protocol;
use codex_protocol::account::PlanType;

use crate::screens::status::{WindowName, window_name};

/// The thresholds a warning is made at, in percent used.
pub const THRESHOLDS: [i32; 4] = [50, 75, 90, 95];

/// The percent used at which the oracle offers a lower-cost model (`RATE_LIMIT_SWITCH_PROMPT_THRESHOLD`).
#[allow(dead_code)] // read by the UI half (port-map ui-usage-limits)
pub const SWITCH_PROMPT_THRESHOLD: i32 = 90;

/// One window of a sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Used {
    pub percent: i32,
    pub window_minutes: Option<i64>,
    /// Unix seconds.
    pub resets_at: Option<i64>,
}

/// What a snapshot of the account's limits says that a warning depends on. Made of a read's answer
/// (`Source::Read`) or of a rolling notification (`Source::Rolling`, which is sparse).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    /// Whether the snapshot is the plan's own usage (`codex`), the only one that warns.
    pub codex: bool,
    pub plan: Option<PlanType>,
    pub primary: Option<Used>,
    pub secondary: Option<Used>,
    /// Whether the snapshot's credits can be spent (`unlimited` or `has_credits`); none when it
    /// carried no credits.
    pub credits_usable: Option<bool>,
    pub spend_control_reached: Option<bool>,
    /// The kind of limit that was reached: `Some(true)` one of the workspace's, `Some(false)`
    /// any other, none when no limit was reached.
    pub reached: Option<bool>,
    /// The kind itself (`rate_limit_reached_type`), for what decides by it.
    pub reached_type: Option<protocol::RateLimitReachedType>,
}

impl Sample {
    pub fn of(snapshot: &protocol::RateLimitSnapshot) -> Self {
        use protocol::RateLimitReachedType as Reached;
        let used = |window: &protocol::RateLimitWindow| Used {
            percent: window.used_percent,
            window_minutes: window.window_duration_mins,
            resets_at: window.resets_at,
        };
        Self {
            codex: snapshot
                .limit_id
                .as_deref()
                .is_none_or(|id| id.eq_ignore_ascii_case("codex")),
            plan: snapshot.plan_type,
            primary: snapshot.primary.as_ref().map(used),
            secondary: snapshot.secondary.as_ref().map(used),
            credits_usable: snapshot
                .credits
                .as_ref()
                .map(|credits| credits.unlimited || credits.has_credits),
            spend_control_reached: snapshot.spend_control_reached,
            reached_type: snapshot.rate_limit_reached_type,
            reached: snapshot.rate_limit_reached_type.map(|kind| {
                matches!(
                    kind,
                    Reached::WorkspaceOwnerCreditsDepleted
                        | Reached::WorkspaceMemberCreditsDepleted
                        | Reached::WorkspaceOwnerUsageLimitReached
                        | Reached::WorkspaceMemberUsageLimitReached
                )
            }),
        }
    }
}

/// Where a sample came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A full read of the account (`account/rateLimits/read`).
    Read,
    /// `account/rateLimits/updated`: sparse, it keeps what the last read learned. `read_out` is
    /// whether a read of the account is in flight: it started before this update and is the
    /// older, so its answer cannot confirm a recovery. (Of several in flight only the first is
    /// taken as overtaken.)
    Rolling { read_out: bool },
}

/// A warning that is due: the share that is left at the threshold passed, and the window's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warning {
    pub remaining_percent: i32,
    pub window: WindowName,
}

/// The percent used from which a window warns: 50 for a five-hour window of Plus and Team, else 75
/// (oracle `usage_notice::warning_threshold`).
pub fn warning_threshold(plan: Option<PlanType>, window_minutes: Option<i64>) -> i32 {
    let five_hours = window_minutes
        .is_some_and(|minutes| window_name(Some(minutes), false) == WindowName::FiveHours);
    if matches!(plan, Some(PlanType::Plus | PlanType::Team)) && five_hours {
        50
    } else {
        75
    }
}

/// What the account's limits left for the thread, per backend: the warnings already given and the
/// windows the notice reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    secondary_index: usize,
    primary_index: usize,
    plan: Option<PlanType>,
    /// The credits the last full read saw (`rate_limit_snapshots_by_limit_id`).
    credits_usable: Option<bool>,
    spend_control_reached: Option<bool>,
    reached: Option<bool>,
    primary: Option<Used>,
    secondary: Option<Used>,
    notice_credits: Option<bool>,
    /// Reads answered, in the order they were sent; a read up to `stale_through` was overtaken by
    /// a rolling update and is too old to confirm a recovery.
    answered: u64,
    stale_through: Option<u64>,
}

impl Limits {
    /// A read was answered, or failed: the next in line is over.
    fn read_ended(&mut self) -> bool {
        self.answered += 1;
        self.stale_through
            .is_some_and(|through| self.answered <= through)
    }

    /// Whether the read that is answered next was overtaken by a rolling update: its answer is
    /// older than what the app holds of the account's windows.
    pub fn next_read_is_stale(&self) -> bool {
        self.stale_through
            .is_some_and(|through| self.answered < through)
    }

    /// The plan of the account the app is signed in to (oracle `update_account_state`, which sets
    /// `plan_type` before any snapshot arrives): what a snapshot that names no plan is judged by,
    /// so a Plus or Team account warns at 50% of its five-hour window from the first update. A
    /// snapshot that names a plan still wins.
    pub fn set_plan(&mut self, plan: Option<PlanType>) {
        self.plan = plan;
    }

    /// A read failed; a newer failed read does not invalidate an older successful one.
    pub fn read_failed(&mut self) {
        self.read_ended();
    }

    /// Whether a limit that stops the account (a workspace's limit, spend control, any reached
    /// type) is known: the notice and the warnings give way to the banner.
    pub fn blocked(&self) -> bool {
        self.reached.is_some() || self.spend_control_reached == Some(true)
    }

    /// A sample arrived. Returns the warnings that are due, once each (oracle
    /// `on_rate_limit_snapshot_from`).
    pub fn take(&mut self, sample: &Sample, source: Source) -> Vec<Warning> {
        let stale = match source {
            Source::Read => self.read_ended(),
            Source::Rolling { read_out } => {
                self.stale_through = Some(self.answered + u64::from(read_out));
                false
            }
        };
        if !sample.codex {
            return Vec::new();
        }
        self.plan = sample.plan.or(self.plan);
        if source == Source::Read || sample.spend_control_reached.is_some() {
            self.spend_control_reached = sample.spend_control_reached;
        }
        if source == Source::Read || sample.reached.is_some() {
            self.reached = sample.reached;
        }
        let workspace_blocked =
            self.spend_control_reached == Some(true) || self.reached == Some(true);
        // a rolling update without credits is judged by the credits the last read saw
        let credits = match source {
            Source::Read => sample.credits_usable,
            Source::Rolling { .. } => sample.credits_usable.or(self.credits_usable),
        };
        if source == Source::Read {
            self.credits_usable = sample.credits_usable;
        }
        self.notice_update(sample, source, stale);
        // a reached workspace limit takes the credits' place (oracle `has_workspace_credits`)
        if !workspace_blocked && credits == Some(true) {
            return Vec::new();
        }
        self.warnings(sample)
    }

    // oracle: RateLimitWarningState::take_warnings
    fn warnings(&mut self, sample: &Sample) -> Vec<Warning> {
        let full = |used: &Option<Used>| used.is_some_and(|used| used.percent >= 100);
        if full(&sample.secondary) || full(&sample.primary) {
            return Vec::new();
        }
        let plan = self.plan;
        let mut due = Vec::new();
        for (used, index, secondary) in [
            (sample.secondary, &mut self.secondary_index, true),
            (sample.primary, &mut self.primary_index, false),
        ] {
            let Some(used) = used else { continue };
            let mut highest = None;
            while *index < THRESHOLDS.len() && used.percent >= THRESHOLDS[*index] {
                let threshold = THRESHOLDS[*index];
                if threshold >= warning_threshold(plan, used.window_minutes) {
                    highest = Some(threshold);
                }
                *index += 1;
            }
            if let Some(threshold) = highest {
                due.push(Warning {
                    remaining_percent: 100 - threshold,
                    window: window_name(used.window_minutes, secondary),
                });
            }
        }
        due
    }

    // oracle: UsageNoticeState::update. A read overtaken by a rolling update does not replace the
    // windows; a rolling update fills what it lacks from the window it follows and does not lower
    // a window that was past its threshold.
    fn notice_update(&mut self, sample: &Sample, source: Source, stale: bool) {
        if source == Source::Read && stale {
            return;
        }
        let rolling = matches!(source, Source::Rolling { .. });
        let plan = self.plan;
        for (current, incoming) in [
            (&mut self.primary, sample.primary),
            (&mut self.secondary, sample.secondary),
        ] {
            if !rolling {
                *current = incoming;
                continue;
            }
            let Some(mut window) = incoming else { continue };
            if let Some(previous) = *current {
                window.window_minutes = window.window_minutes.or(previous.window_minutes);
                window.resets_at = window.resets_at.or(previous.resets_at);
                let threshold = warning_threshold(plan, window.window_minutes);
                if previous.percent >= threshold && window.percent < threshold {
                    continue;
                }
            }
            *current = Some(window);
        }
        if !rolling || sample.credits_usable.is_some() {
            self.notice_credits = sample.credits_usable;
        }
    }

    /// The window the composer's notice names: the one with the least capacity left among those
    /// past their threshold, the primary one when both are as far (oracle
    /// `UsageNoticeState::current`). None when credits can be spent or no window is that far;
    /// the flag is whether it is the secondary window. The view shows it unless a banner or a
    /// reached limit takes its place (`Limits::blocked`).
    pub fn notice(&self) -> Option<(Used, bool)> {
        if self.notice_credits == Some(true) {
            return None;
        }
        // the last of the equal ones wins: the primary window has priority
        [(self.secondary, true), (self.primary, false)]
            .into_iter()
            .filter_map(|(window, secondary)| Some((window?, secondary)))
            .filter(|(window, _)| {
                window.percent >= warning_threshold(self.plan, window.window_minutes)
            })
            .max_by_key(|(window, _)| window.percent)
    }

    /// The highest percent used among the windows of the sample the last read left: when it passes
    /// [`SWITCH_PROMPT_THRESHOLD`] the oracle offers a lower-cost model.
    pub fn used_percent(&self) -> Option<i32> {
        [self.primary, self.secondary]
            .into_iter()
            .flatten()
            .map(|used| used.percent)
            .max()
    }
}
