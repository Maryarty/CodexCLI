//! The usage page's host (`Open::usage_accounts`, `Open::usage_report`): what the `/usage` page's
//! account lookups and reports are answered with, from the first ask on. The page asks as the
//! oracle's client does (`Command::Usage`); the fake backend answers under the ask's session and
//! stamp. A lookup that fails is the host's message; a plan that changes while the page is open
//! is a second answer (the page's refresh, Shift+R, asks again and gets it).
//!
//! `Open::dir("usage")` opens the page live (its row of the debug menu), where a fixture entry
//! is seeded and asks nothing.

use codex_tui_core::screens::usage::{AccountReply, ReportReply};
use codex_tui_rtwk::services::backend::fake::FakeHandle;

use super::seed::Open;

/// The answers a test scripted: JSON of the host's reply (`{"plan":"plus","today":"2026-09-09"}`)
/// or the message of a failed request.
#[derive(Clone, Default)]
pub(super) struct Host {
    accounts: Vec<Result<&'static str, &'static str>>,
    report: Option<Result<&'static str, &'static str>>,
}

impl Host {
    /// Hands the answers to the fake backend, before the page opens.
    pub(super) fn tell(&self, fake: &FakeHandle) {
        if !self.accounts.is_empty() {
            fake.usage_accounts(
                self.accounts
                    .iter()
                    .map(|answer| match answer {
                        Ok(json) => Ok(serde_json::from_str::<AccountReply>(json)
                            .unwrap_or_else(|error| panic!("account reply {json}: {error}"))),
                        Err(message) => Err((*message).to_string()),
                    })
                    .collect(),
            );
        }
        if let Some(answer) = &self.report {
            fake.usage_report(match answer {
                Ok(json) => Ok(serde_json::from_str::<ReportReply>(json)
                    .unwrap_or_else(|error| panic!("report reply {json}: {error}"))),
                Err(message) => Err((*message).to_string()),
            });
        }
    }
}

impl Open {
    /// The usage page's account lookups, one answer per ask in order, the last for every ask after
    /// it: `Ok(json)` of an `AccountReply`, `Err(message)` for a lookup that failed. Two answers
    /// are an account whose plan changes under the open page.
    pub fn usage_accounts(mut self, answers: &[Result<&'static str, &'static str>]) -> Open {
        self.usage.accounts = answers.to_vec();
        self
    }

    /// The answer of every report the usage page asks for (`Ok(json)` of a `ReportReply`); without
    /// one the reports stay loading.
    pub fn usage_report(mut self, answer: Result<&'static str, &'static str>) -> Open {
        self.usage.report = Some(answer);
        self
    }
}
