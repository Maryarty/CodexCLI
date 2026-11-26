//! Usage-limit resets (oracle `chatwidget/usage.rs`, `chatwidget/reset_credits.rs`): the credits
//! the account earned, redeemed one at a time with an idempotency key. Core keeps what IO needs
//! (ADR-013): the count last learned, the credits of the list read for the picker, the request
//! records of the reads that wait (by the request number they carry), and the redemption by its
//! DOMAIN key, the idempotency key. What is drawn (the usage menu, the picker, the confirmation,
//! the cursor in them) is the UI's; it says verbs (`Intent::Resets`, `Redeem`, `Retry`) and reads
//! this state. The words are the UI's string tables'.
//!
//! The reads themselves are `account/rateLimits/read` with a `RateLimitOrigin` (the app starts
//! them: `app/reset.rs`); the redemption is an app-scoped account RPC.

use codex_app_server_protocol::ConsumeAccountRateLimitResetCreditOutcome as WireOutcome;
use codex_app_server_protocol::{
    ConsumeAccountRateLimitResetCreditParams, ConsumeAccountRateLimitResetCreditResponse,
};

use crate::ids::BackendId;
use crate::screens::status::{CreditStatus, ResetCredit, ResetCredits};

/// What the backend said of one redemption (`ConsumeAccountRateLimitResetCreditOutcome`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A credit was consumed and the eligible windows were reset.
    Reset,
    /// The same idempotency key already completed a reset: a success, said once more.
    AlreadyRedeemed,
    /// No window is eligible for a reset.
    NothingToReset,
    /// The account has no credit (or not that one).
    NoCredit,
}

impl From<WireOutcome> for Outcome {
    fn from(outcome: WireOutcome) -> Self {
        match outcome {
            WireOutcome::Reset => Self::Reset,
            WireOutcome::AlreadyRedeemed => Self::AlreadyRedeemed,
            WireOutcome::NothingToReset => Self::NothingToReset,
            WireOutcome::NoCredit => Self::NoCredit,
        }
    }
}

crate::rpc_list! {
    /// The account RPC of a redemption.
    pub enum Consume => Outcome;
    /// `account/rateLimitResetCredit/consume` under the idempotency key core chose.
    Redeem { key: String, credit_id: Option<String> }, Self::Redeem { key, credit_id }
        => ConsumeAccountRateLimitResetCredit(ConsumeAccountRateLimitResetCreditParams {
            idempotency_key: key.clone(),
            credit_id: credit_id.clone(),
        })
        -> ConsumeAccountRateLimitResetCreditResponse, |response| Outcome::from(response.outcome);
}

/// When a credit stops being usable, as the picker's detail line needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    /// Unix seconds.
    At(i64),
    /// `Does not expire`.
    Never,
    /// `Expiration unavailable`: a timestamp no date can be made of.
    Unavailable,
}

/// The latest unix second a date exists for (the oracle's `DateTime::from_timestamp` fails after).
const LAST_DATE: i64 = 8_210_266_876_799;

/// One row of the picker (oracle `ResetCreditOption`). `title` and `description` are the credit's
/// own, trimmed; none says the default words (`Full reset`, `Reset your current usage limits`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreditOption {
    /// The credit to redeem; none redeems the backend's next one (the details were not sent).
    pub credit_id: Option<String>,
    pub title: Option<String>,
    /// None for the single row of an account whose details were not sent.
    pub expiry: Option<Expiry>,
    pub description: Option<String>,
}

fn text(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn option_of(credit: &ResetCredit) -> CreditOption {
    CreditOption {
        credit_id: Some(credit.id.clone()),
        title: text(&credit.title),
        expiry: Some(match credit.expires_at {
            Some(at) if !(-LAST_DATE..=LAST_DATE).contains(&at) => Expiry::Unavailable,
            Some(at) => Expiry::At(at),
            None => Expiry::Never,
        }),
        description: text(&credit.description),
    }
}

/// The rows the picker lists (oracle `reset_credit_options`): the available credits by
/// expiry, soonest first (none last), at most as many as the count says; the one default row when
/// there are none to list.
pub fn options(summary: &ResetCredits) -> Vec<CreditOption> {
    let limit = usize::try_from(summary.available_count.max(0)).unwrap_or(usize::MAX);
    let mut available: Vec<&ResetCredit> = summary
        .credits
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|credit| credit.status == CreditStatus::Available)
        .collect();
    available.sort_by_key(|credit| credit.expires_at.unwrap_or(i64::MAX));
    let mut options: Vec<CreditOption> = available.into_iter().take(limit).map(option_of).collect();
    if options.is_empty() {
        options.push(CreditOption {
            credit_id: None,
            title: None,
            expiry: None,
            description: None,
        });
    }
    options
}

/// The `Redeem reset` entry of the usage menu (oracle `usage_menu_params`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    /// `{n} available`.
    Available(i64),
    /// `Check availability`: nothing is known yet.
    Check,
    /// `None available`, and the entry is disabled.
    NoneAvailable,
}

impl MenuEntry {
    pub fn enabled(self) -> bool {
        matches!(self, Self::Available(_) | Self::Check)
    }
}

/// What the picker's list read stands at (the oracle's loading, picker, message and retry pages).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Listing {
    #[default]
    Idle,
    /// `Checking your available resets...`: the read of this request number is out.
    Checking(u64),
    /// Credits are available: the picker lists `options`.
    Ready,
    /// `You don't have any usage limit resets available.`
    Empty,
    /// `Couldn't load usage limit resets. Please try again.`
    Failed,
}

/// Where a redemption stands (oracle: the consuming popup, its refresh, its end).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// `Using a reset...`: the host's ask is out, and nothing can dismiss it.
    Asking,
    /// `Usage reset. Checking your remaining resets...`: the reset happened; the credits are read
    /// again.
    Refreshing,
    Settled(Settled),
}

/// How a redemption ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settled {
    /// `Usage reset. You have {n} left.`; none when the count could not be read (`Usage reset.`).
    Reset { left: Option<i64> },
    /// `Your usage does not need a reset right now.`
    NothingToReset,
    /// `No usage limit resets are available.`
    NoCredit,
    /// `That reset is no longer available. Refresh to see your current resets.`: the chosen
    /// credit was gone; what is known of the count is dropped.
    Stale,
    /// `Couldn't reset usage. Please try again.`: `Intent::Retry` sends the same key again.
    Failed,
}

/// One redemption, by its idempotency key (oracle `pending_rate_limit_reset_idempotency_key`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redemption {
    pub key: String,
    pub credit_id: Option<String>,
    /// The account on which this idempotency key was first sent.
    pub(crate) backend: Option<BackendId>,
    pub step: Progress,
}

/// What the app does after a step: the reads and asks are the app's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    Nothing,
    /// Read the account's limits for this origin's sake (`RateLimitOrigin`).
    Read,
    /// Ask the host to redeem.
    Redeem {
        key: String,
        credit_id: Option<String>,
    },
}

/// The app-scoped state of the reset flow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// How many resets the account has, as last learned (oracle
    /// `available_rate_limit_reset_credits`); none until known, and after a redemption.
    pub available: Option<i64>,
    /// The credits of the read the picker was built from.
    pub credits: Option<ResetCredits>,
    pub listing: Listing,
    /// The backend used for the current picker, retained through confirmation.
    pub(crate) picker_backend: Option<BackendId>,
    pub redemption: Option<Redemption>,
    /// The usage menu's availability read that is out (its request number).
    availability_read: Option<u64>,
    /// The startup check for the hint that is out.
    hint_check: Option<u64>,
    /// The count of the startup hint that waits for active output to settle before it is told
    /// (oracle `pending_rate_limit_reset_hint`).
    pending_hint: Option<i64>,
    /// Counts the account changes (`clear`): a view opened for an account closes when it moves.
    epoch: u64,
    next_request: u64,
    next_key: u64,
}

impl State {
    fn request(&mut self) -> u64 {
        let request = self.next_request;
        self.next_request = self.next_request.wrapping_add(1);
        request
    }

    /// The entry the usage menu shows for an account that is (not) a ChatGPT one.
    pub fn menu_entry(&self, chatgpt_account: bool) -> MenuEntry {
        match (chatgpt_account, self.available) {
            (true, Some(count)) if count > 0 => MenuEntry::Available(count),
            (true, None) => MenuEntry::Check,
            _ => MenuEntry::NoneAvailable,
        }
    }

    /// The rows of the picker.
    pub fn options(&self) -> Vec<CreditOption> {
        self.credits.as_ref().map(options).unwrap_or_default()
    }

    /// Whether a redemption is under way: nothing may dismiss its popup.
    pub fn redeeming(&self) -> bool {
        matches!(
            self.redemption,
            Some(Redemption {
                step: Progress::Asking | Progress::Refreshing,
                ..
            })
        )
    }

    /// The usage menu opened (oracle `open_usage_menu`): a hint that is waiting goes, and a count
    /// known to be zero is checked again (the read of the returned request).
    pub fn open_menu(&mut self) -> Option<u64> {
        self.drop_hint();
        if self.available != Some(0) {
            return None;
        }
        let request = self.request();
        self.availability_read = Some(request);
        Some(request)
    }

    /// The availability read of the menu ended: the count, when the read carried one.
    pub fn menu_read(&mut self, request: u64, result: Result<Option<ResetCredits>, ()>) {
        if self.availability_read != Some(request) {
            return;
        }
        self.availability_read = None;
        if let Ok(Some(credits)) = result {
            self.available = Some(credits.available_count);
        }
    }

    /// The reset flow opened (oracle `show_rate_limit_reset_loading_popup`): the key of an
    /// earlier attempt is dropped and the credits are read; the request number of the read.
    pub fn open_resets(&mut self) -> u64 {
        self.drop_hint();
        self.redemption = None;
        let request = self.request();
        self.listing = Listing::Checking(request);
        request
    }

    /// The picker's read ended (oracle `finish_rate_limit_reset_credits_refresh`): a read of
    /// another request than the one waited for changes nothing.
    pub fn list_read(&mut self, request: u64, result: Result<Option<ResetCredits>, ()>) {
        if self.listing != Listing::Checking(request) {
            return;
        }
        match result {
            Ok(Some(credits)) => {
                self.available = Some(credits.available_count);
                self.listing = if credits.available_count > 0 {
                    Listing::Ready
                } else {
                    Listing::Empty
                };
                self.credits = Some(credits);
            }
            // The answer carried no credits: the oracle's "did not include rateLimitResetCredits".
            Ok(None) | Err(()) => self.listing = Listing::Failed,
        }
    }

    /// The user confirmed a reset (oracle `show_rate_limit_reset_confirmation`'s key and the
    /// `Yes, use reset` action): a new key, and the host asked. A redemption under way is
    /// not started twice.
    pub fn redeem(&mut self, credit_id: Option<String>) -> Next {
        if self.redeeming() {
            return Next::Nothing;
        }
        self.drop_hint();
        self.next_key = self.next_key.wrapping_add(1);
        let key = [self.next_request.to_string(), self.next_key.to_string()].join("-");
        self.redemption = Some(Redemption {
            key: key.clone(),
            credit_id: credit_id.clone(),
            backend: None,
            step: Progress::Asking,
        });
        Next::Redeem { key, credit_id }
    }

    /// `Try again` after a failed redemption: the same key, so the backend redeems once
    /// (`rate_limit_reset_retry_reuses_idempotency_key`).
    pub fn retry(&mut self) -> Next {
        match &mut self.redemption {
            Some(
                redemption @ Redemption {
                    step: Progress::Settled(Settled::Failed),
                    ..
                },
            ) => {
                redemption.step = Progress::Asking;
                Next::Redeem {
                    key: redemption.key.clone(),
                    credit_id: redemption.credit_id.clone(),
                }
            }
            _ => Next::Nothing,
        }
    }

    /// The host's answer to the redemption of `key` (oracle `finish_rate_limit_reset_consume`):
    /// an answer for another key, or one that is not waited for, changes nothing. A reset asks
    /// for the credits again (`Next::Read`).
    pub fn redeemed(&mut self, key: &str, result: Result<Outcome, String>) -> Next {
        let Some(redemption) = self
            .redemption
            .as_mut()
            .filter(|redemption| redemption.key == key && redemption.step == Progress::Asking)
        else {
            return Next::Nothing;
        };
        let settled = match result {
            Ok(Outcome::Reset | Outcome::AlreadyRedeemed) => {
                redemption.step = Progress::Refreshing;
                self.available = None;
                return Next::Read;
            }
            Ok(Outcome::NothingToReset) => Settled::NothingToReset,
            Ok(Outcome::NoCredit) if redemption.credit_id.is_some() => {
                self.available = None;
                Settled::Stale
            }
            Ok(Outcome::NoCredit) => {
                self.available = Some(0);
                Settled::NoCredit
            }
            Err(_) => Settled::Failed,
        };
        redemption.step = Progress::Settled(settled);
        Next::Nothing
    }

    /// The read after a reset ended (oracle `finish_post_consume_reset_credits_refresh`): the
    /// count left, or only the news that the reset happened when it could not be read.
    pub fn after_reset_read(&mut self, result: Result<Option<ResetCredits>, ()>) {
        let Some(redemption) = self
            .redemption
            .as_mut()
            .filter(|redemption| redemption.step == Progress::Refreshing)
        else {
            return;
        };
        let left = match result {
            Ok(Some(credits)) => Some(credits.available_count),
            Ok(None) | Err(()) => None,
        };
        if left.is_some() {
            self.available = left;
        }
        redemption.step = Progress::Settled(Settled::Reset { left });
    }

    /// Whether the startup check for the hint is out: a second startup fact asks no second one
    /// (the oracle checks once, at startup, and the answer of an overtaken one is dropped).
    pub fn hint_out(&self) -> bool {
        self.hint_check.is_some()
    }

    /// The startup check for the reset hint begins: the request number of the read.
    pub fn start_hint(&mut self) -> u64 {
        self.pending_hint = None;
        let request = self.request();
        self.hint_check = Some(request);
        request
    }

    /// The startup read ended (oracle `finish_rate_limit_reset_hint_refresh`): the count to
    /// tell the user of, when there is one to use. The count is kept either way it was read.
    pub fn hint_read(
        &mut self,
        request: u64,
        result: Result<Option<ResetCredits>, ()>,
    ) -> Option<i64> {
        if self.hint_check != Some(request) {
            return None;
        }
        self.hint_check = None;
        let count = result.ok().flatten()?.available_count;
        self.available = Some(count);
        (count > 0).then_some(count)
    }

    /// How many times the account changed; the popups of an account are dismissed when it moves
    /// (oracle `dismiss_view_by_id` in `clear_pending_rate_limit_reset_requests`).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The hint is overtaken (oracle `clear_pending_rate_limit_reset_hint`): the check that is
    /// out and the count that waits are both dropped.
    fn drop_hint(&mut self) {
        self.hint_check = None;
        self.pending_hint = None;
    }

    /// The hint's count is read but active output is on screen: it waits (oracle
    /// `usage_history_insertion_blocked`).
    pub fn park_hint(&mut self, count: i64) {
        self.pending_hint = Some(count);
    }

    /// Whether a hint waits for the output to settle.
    pub fn hint_waits(&self) -> bool {
        self.pending_hint.is_some()
    }

    /// The count of the hint that waited, once the output has settled: it is told once.
    pub fn take_hint(&mut self) -> Option<i64> {
        self.pending_hint.take()
    }

    /// The account changed (oracle `clear_pending_rate_limit_reset_requests`): nothing asked
    /// of the old one is the new one's.
    pub fn clear(&mut self) {
        *self = Self {
            epoch: self.epoch.wrapping_add(1),
            next_request: self.next_request,
            next_key: self.next_key,
            ..Self::default()
        };
    }
}
