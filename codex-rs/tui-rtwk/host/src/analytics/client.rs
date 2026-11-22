//! The account the reports are read for: the local sign-in's session with
//! the identity checks, the cache of the report payloads and the choice of
//! the backend route per account kind. Cached payloads belong to one local
//! account and user; every report ends on the day the session was opened on
//! (a refresh replaces the session). Oracle: `analytics/client.rs`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use codex_backend_client::{AnalyticsReport, AnalyticsSession, RequestError};
use codex_core::config::Config;
use codex_features::Feature;
use codex_protocol::account::PlanType;
use codex_tui_core::screens::usage::data::GROUPINGS;
use codex_tui_core::screens::usage::models::{
    AccountAnalyticsGrouping as Grouping, AccountAnalyticsHistory as History,
    AccountAnalyticsReport as Report, AccountKind,
};
use codex_tui_core::screens::usage::plot::categories;
use codex_tui_core::screens::usage::report_data::AnalyticsData;
use codex_tui_core::screens::usage::{LoadError, NaiveDate, ReportReply, normalize, tokens};
use tokio::sync::{Mutex, OnceCell};

use super::convert;

/// One open session of the page: what a refresh replaces.
pub struct Live {
    /// An identity check failed: the signed-in account changed under the
    /// session, so the page starts again (`Ev::Invalidated`).
    identity_invalidated: AtomicBool,
    config: Arc<Config>,
    end_date: NaiveDate,
    session: OnceCell<Session>,
}

pub struct Session {
    pub kind: AccountKind,
    pub backend: AnalyticsSession,
    pub credit_groups: Vec<usize>,
    cache: Mutex<HashMap<(AnalyticsReport, String, String), AnalyticsData>>,
}

impl Live {
    pub fn new(config: Arc<Config>, end_date: NaiveDate) -> Self {
        Self {
            identity_invalidated: AtomicBool::new(/*v*/ false),
            config,
            end_date,
            session: OnceCell::new(),
        }
    }

    pub fn end_date(&self) -> NaiveDate {
        self.end_date
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn invalidated(&self) -> bool {
        self.identity_invalidated.load(Ordering::Relaxed)
    }

    pub async fn session(&self) -> Result<&Session, String> {
        self.session
            .get_or_try_init(|| async {
                let session = AnalyticsSession::from_config(
                    self.config.as_ref(),
                    self.config.http_client_factory(),
                    self.config
                        .features
                        .enabled(Feature::RuntimeAgentInstances)
                        .then_some(self.config.runtime_auth_mode),
                )
                .await?;
                let credit_groups = Grouping::credit_groupings(session.account().plan_type)
                    .iter()
                    .filter_map(|group| GROUPINGS.iter().position(|candidate| candidate == group))
                    .collect();
                Ok(Session {
                    kind: AccountKind::from(session.account().plan_type),
                    credit_groups,
                    backend: session,
                    cache: Mutex::new(HashMap::new()),
                })
            })
            .await
    }

    /// Every report shares the same privacy boundary, cached loads too.
    pub async fn ensure_identity(&self) -> Result<(), String> {
        let identity = match self.session().await {
            Ok(session) => session.backend.ensure_identity().await,
            Err(error) => Err(error),
        };
        if identity.is_err() {
            self.identity_invalidated
                .store(/*val*/ true, Ordering::Relaxed);
        }
        identity
    }

    pub async fn plan_history(
        &self,
    ) -> Result<Option<codex_tui_core::screens::usage::plan::Data>, String> {
        let session = self.session().await?;
        if session.kind != AccountKind::Consumer {
            return Ok(None);
        }
        let history = session
            .backend
            .request(|client| async move { client.get_plan_limit_history().await })
            .await;
        self.ensure_identity().await?;
        Ok(history.map_err(request_error)?.map(convert::plan_history))
    }

    pub async fn profile(
        &self,
    ) -> Result<Option<Box<codex_tui_core::screens::usage::profile::Profile>>, String> {
        let session = self.session().await?;
        let profile = session
            .backend
            .request(|client| async move { client.get_account_profile().await })
            .await;
        self.ensure_identity().await?;
        profile
            .map(|profile| Some(Box::new(convert::profile(profile))))
            .map_err(request_error)
    }

    /// The history of `report` over `days` days ending on the session's day,
    /// grouped by `grouping`; `model_filter` is the token report's model.
    pub async fn history(
        &self,
        report: Report,
        days: u32,
        grouping: Grouping,
        model_filter: Option<&str>,
    ) -> Result<ReportReply, LoadError> {
        let end = self.end_date;
        let start = days
            .checked_sub(/*rhs*/ 1)
            .and_then(|offset| end.checked_sub_days(u64::from(offset)))
            .ok_or_else(|| "Invalid analytics date range.".to_string())?;
        let session = self.session().await?;
        self.ensure_identity().await?;
        let enterprise_tokens = report == Report::Usage
            && matches!(
                session.kind,
                AccountKind::Enterprise | AccountKind::Business
            );
        let grouping =
            if enterprise_tokens && !matches!(grouping, Grouping::Model | Grouping::TokenType) {
                Grouping::TokenType
            } else {
                grouping
            };
        let route =
            route(report, grouping, session.backend.account().plan_type).ok_or_else(|| {
                "This credit breakdown is not supported for this account type.".to_string()
            })?;
        // Credit events have no range parameters. Other grouping changes
        // reuse the same payload.
        let key = if route == AnalyticsReport::Credits {
            (route, String::new(), String::new())
        } else {
            (route, start.to_string(), end.to_string())
        };
        let cached = session.cache.lock().await.get(&key).cloned();
        let response = if let Some(response) = cached {
            Ok(response)
        } else {
            session
                .backend
                .request(|client| {
                    let (_, start, end) = key.clone();
                    async move { client.get_account_analytics(route, &start, &end).await }
                })
                .await
                .map(convert::data)
        };
        self.ensure_identity().await?;
        let response = response.map_err(request_error)?;
        let mut reply = ReportReply {
            history: None,
            token_models: None,
            attributed_usage: None,
        };
        let history = (|| -> Result<Option<History>, LoadError> {
            if enterprise_tokens {
                let models = tokens::history(response.clone(), Grouping::Model, start, end)?;
                reply.token_models = Some(
                    categories(&models)
                        .into_iter()
                        .map(|model| model.key)
                        .collect(),
                );
                tokens::filtered_history(response.clone(), grouping, start, end, model_filter)
                    .map(Some)
                    .map_err(LoadError::from)
            } else {
                let grouping = if report == Report::Usage {
                    let attributed = normalize::has_complete_attribution(&response, start, end)?;
                    reply.attributed_usage = Some(attributed);
                    if !attributed && matches!(grouping, Grouping::Feature | Grouping::TaskStart) {
                        Grouping::Surface
                    } else {
                        grouping
                    }
                } else {
                    grouping
                };
                normalize::history(response.clone(), report, grouping, start, end)
                    .map_err(LoadError::from)
            }
        })();
        let mut cache = session.cache.lock().await;
        if history.is_ok() {
            cache.insert(key, response);
        } else {
            cache.remove(&key);
        }
        reply.history = history?;
        Ok(reply)
    }
}

// Account type selects billing semantics, never activity-report eligibility.
fn route(report: Report, grouping: Grouping, plan: Option<PlanType>) -> Option<AnalyticsReport> {
    let kind = AccountKind::from(plan);
    let enterprise = kind == AccountKind::Enterprise;
    Some(match report {
        Report::Usage if enterprise => AnalyticsReport::EnterpriseTokens,
        Report::Usage if kind == AccountKind::Business => AnalyticsReport::WorkspaceCredits,
        Report::Usage => AnalyticsReport::Usage,
        Report::Messages => AnalyticsReport::Messages,
        Report::Credits if !Grouping::credit_groupings(plan).contains(&grouping) => return None,
        Report::Credits if enterprise => AnalyticsReport::EnterpriseCredits {
            breakdown: match grouping {
                Grouping::Surface => "product",
                Grouping::Model => "model",
                Grouping::Speed => "speed",
                Grouping::Reasoning => "reasoning_effort",
                Grouping::Feature | Grouping::TaskStart | Grouping::TokenType => return None,
            },
        },
        Report::Credits if kind == AccountKind::Business => AnalyticsReport::WorkspaceCredits,
        Report::Credits => AnalyticsReport::Credits,
        Report::Plugins => AnalyticsReport::Plugins {
            limit: if enterprise { 8 } else { 10 },
        },
        Report::Skills => AnalyticsReport::Skills {
            limit: if enterprise { 6 } else { 10 },
        },
    })
}

/// Keeps untyped server and transport diagnostics out of the reports.
pub fn request_error(error: RequestError) -> String {
    match error.status().map(|status| status.as_u16()) {
        Some(401) => "Sign in again to load this report.",
        Some(403) => "Access denied for this report.",
        Some(404) => "This report endpoint is unavailable. Press R to retry.",
        _ => "Report request failed. Press R to retry.",
    }
    .into()
}
