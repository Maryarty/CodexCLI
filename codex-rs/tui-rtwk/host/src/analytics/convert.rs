//! The backend's payloads as core's values: the host decodes each endpoint's
//! type into the screen's own (core knows no backend crate). A missing
//! backend value stays missing. Oracle: `analytics/report_data.rs` (`From`
//! impls), `plan.rs` (`PlanLimitHistory`), `summary.rs` (`AccountProfile`).

use codex_backend_client::analytics_models as backend;
use codex_backend_client::{
    AccountProfile, AnalyticsResponse, PlanLimitDimension, PlanLimitHistory, ProfileInvocationKind,
};
use codex_tui_core::screens::usage::plan;
use codex_tui_core::screens::usage::profile;
use codex_tui_core::screens::usage::report_data::{
    AnalyticsClient, AnalyticsCount, AnalyticsData, AnalyticsModel, AnalyticsRecord,
    AnalyticsSeries, AnalyticsTokenGroup, AnalyticsTool, PremiumUsage, UsageAttribution,
};

fn count(value: Option<i64>) -> Option<f64> {
    value.map(|count| count as f64)
}

fn model(value: backend::ModelUsage) -> AnalyticsModel {
    AnalyticsModel {
        model: value.model,
        credits: Some(value.credits),
        uncached_text_input_tokens: count(value.uncached_text_input_tokens),
        cached_text_input_tokens: count(value.cached_text_input_tokens),
        text_output_tokens: count(value.text_output_tokens),
        text_total_tokens: count(value.text_total_tokens),
        turns: count(value.turns),
        on_demand_credits: value.on_demand_credits,
        speed: value.speed,
    }
}

fn models(values: Option<Vec<backend::ModelUsage>>) -> Option<Vec<AnalyticsModel>> {
    values.map(|values| values.into_iter().map(model).collect())
}

pub fn data(response: AnalyticsResponse) -> AnalyticsData {
    match response {
        AnalyticsResponse::Usage(response) => AnalyticsData {
            units: response.units,
            data_freshness_ts: response.data_freshness_ts,
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    product_surface_usage_values: Some(record.product_surface_usage_values),
                    premium_usage_values: record.premium_usage_values.map(|value| PremiumUsage {
                        credit_usage_credits: value.credit_usage_credits,
                    }),
                    attribution: record.attribution.map(|values| {
                        values
                            .into_iter()
                            .map(|value| UsageAttribution {
                                thread_source: Some(value.thread_source),
                                turn_trigger: Some(value.turn_trigger),
                                model: Some(value.model),
                                surface: Some(value.surface),
                                value: value.value,
                            })
                            .collect()
                    }),
                    models: models(record.models),
                    groups: record.groups.map(|values| {
                        values
                            .into_iter()
                            .map(|value| AnalyticsTokenGroup {
                                dimensions: value.dimensions,
                                is_other: value.is_other.unwrap_or(/*default*/ false),
                                uncached_text_input_tokens: count(value.uncached_text_input_tokens),
                                cached_text_input_tokens: count(value.cached_text_input_tokens),
                                text_output_tokens: count(value.text_output_tokens),
                            })
                            .collect()
                    }),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        AnalyticsResponse::Credits(response) => AnalyticsData {
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    credit_amount: Some(record.credit_amount),
                    product_surface: Some(record.product_surface),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        AnalyticsResponse::EnterpriseCredits(response) => AnalyticsData {
            data_freshness_ts: response.data_freshness_ts,
            series: Some(
                response
                    .series
                    .into_iter()
                    .map(|value| AnalyticsSeries {
                        key: value.key,
                        label: value.label,
                    })
                    .collect(),
            ),
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    values: Some(record.values),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        AnalyticsResponse::Messages(response) => AnalyticsData {
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    totals: Some(AnalyticsCount {
                        turns: record.totals.turns as f64,
                    }),
                    clients: Some(
                        record
                            .clients
                            .into_iter()
                            .map(|value| AnalyticsClient {
                                client_id: value.client_id,
                                turns: value.turns as f64,
                            })
                            .collect(),
                    ),
                    models: models(record.models),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        AnalyticsResponse::Plugins(response) => AnalyticsData {
            data_freshness_ts: response.data_freshness_ts,
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    plugin_usage_overviews: Some(
                        record
                            .plugin_usage_overviews
                            .into_iter()
                            .map(|value| AnalyticsTool {
                                display_name: value.display_name,
                                invocation_counts: value.invocation_counts as f64,
                            })
                            .collect(),
                    ),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        AnalyticsResponse::Skills(response) => AnalyticsData {
            data_freshness_ts: response.data_freshness_ts,
            data: response
                .data
                .into_iter()
                .map(|record| AnalyticsRecord {
                    date: record.date,
                    skill_usage_overviews: Some(
                        record
                            .skill_usage_overviews
                            .into_iter()
                            .map(|value| AnalyticsTool {
                                display_name: value.display_name,
                                invocation_counts: value.invocation_counts as f64,
                            })
                            .collect(),
                    ),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
    }
}

pub fn plan_history(history: PlanLimitHistory) -> plan::Data {
    plan::Data {
        data_as_of: history.data_as_of,
        coverage_start: history.coverage_start,
        coverage_complete: history.coverage_complete,
        approximate: history.approximate,
        periods: history
            .periods
            .into_iter()
            .map(|period| plan::PeriodData {
                id: period.id,
                window_minutes: period.window_minutes,
                starts_at: period.starts_at,
                ends_at: period.ends_at,
                accounting_complete: period.accounting_complete,
                used_basis_points: period.used_basis_points,
                breakdowns: period.breakdowns.map(|breakdowns| {
                    breakdowns
                        .into_iter()
                        .map(|breakdown| plan::Breakdown {
                            dimension: match breakdown.dimension {
                                PlanLimitDimension::ThreadSource => plan::Dimension::ThreadSource,
                                PlanLimitDimension::TurnTrigger => plan::Dimension::TurnTrigger,
                                PlanLimitDimension::Model => plan::Dimension::Model,
                                PlanLimitDimension::Surface => plan::Dimension::Surface,
                                PlanLimitDimension::Unknown => plan::Dimension::Unknown,
                            },
                            rows: breakdown
                                .rows
                                .into_iter()
                                .map(|row| plan::Share {
                                    key: row.key,
                                    basis_points: row.basis_points,
                                })
                                .collect(),
                        })
                        .collect()
                }),
            })
            .collect(),
    }
}

pub fn profile(value: AccountProfile) -> profile::Profile {
    let stats = value.stats;
    profile::Profile {
        identity: value.profile.map(|identity| profile::Identity {
            display_name: identity.display_name,
            username: identity.username,
        }),
        metadata: value.metadata.map(|metadata| profile::Metadata {
            stats_as_of: metadata.stats_as_of,
            stats_error: metadata.stats_error,
        }),
        stats: profile::Stats {
            lifetime_tokens: stats.tokens.lifetime_tokens,
            peak_daily_tokens: stats.tokens.peak_daily_tokens,
            longest_running_turn_sec: stats.tokens.longest_running_turn_sec,
            current_streak_days: stats.tokens.current_streak_days,
            longest_streak_days: stats.tokens.longest_streak_days,
            daily_usage_buckets: stats.tokens.daily_usage_buckets.map(|buckets| {
                buckets
                    .into_iter()
                    .map(|bucket| profile::Bucket {
                        start_date: bucket.start_date,
                        tokens: bucket.tokens,
                    })
                    .collect()
            }),
            fast_mode_usage_percentage: stats
                .fast_mode_usage_percentage
                .and_then(|number| number.as_f64()),
            most_used_reasoning_effort: stats.most_used_reasoning_effort,
            most_used_reasoning_effort_percentage: stats
                .most_used_reasoning_effort_percentage
                .and_then(|number| number.as_f64()),
            unique_skills_used: stats.unique_skills_used,
            total_skills_used: stats.total_skills_used,
            total_threads: stats.total_threads,
            top_invocations: stats.top_invocations.map(|invocations| {
                invocations
                    .into_iter()
                    .map(|invocation| profile::Invocation {
                        kind: match invocation.kind {
                            ProfileInvocationKind::Plugin => profile::InvocationKind::Plugin,
                            ProfileInvocationKind::Skill => profile::InvocationKind::Skill,
                            ProfileInvocationKind::Unknown => profile::InvocationKind::Unknown,
                        },
                        plugin_name: invocation.plugin_name,
                        skill_name: invocation.skill_name,
                        usage_count: invocation.usage_count,
                    })
                    .collect()
            }),
        },
    }
}
