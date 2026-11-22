//! Regression coverage for daily report normalization and billing semantics.

use codex_tui_core::screens::usage::date::NaiveDate;
use codex_tui_core::screens::usage::models::AccountAnalyticsGrouping as Grouping;
use codex_tui_core::screens::usage::models::AccountAnalyticsReport as Report;
use codex_tui_core::screens::usage::models::*;
use codex_tui_core::screens::usage::normalize::{Amount, Error, Gap, has_complete_attribution, history};
use codex_tui_core::screens::usage::report_data::AnalyticsData;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn turn_start_includes_all_features_and_missing_attribution_is_unavailable() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response = serde_json::from_value(json!({"data": [{"date": "2026-01-09", "attribution": [
        {"thread_source": "user", "turn_trigger": "composer", "value": 30},
        {"thread_source": "subagent", "value": 70}
    ]}]}))
    .unwrap();
    assert_eq!(
        history(response, Report::Usage, Grouping::TaskStart, date, date).unwrap(),
        Some(AccountAnalyticsHistory {
            unit: AccountAnalyticsUnit::RelativeUsage,
            updated_at: None,
            data: vec![AccountAnalyticsDay {
                date: "2026-01-09".parse().unwrap(),
                total: 100.0,
                values: vec![
                    AccountAnalyticsValue {
                        key: "start-composer".to_string(),
                        label: "User messages".to_string(),
                        value: 30.0
                    },
                    AccountAnalyticsValue {
                        key: "unknown".to_string(),
                        label: "Unknown".to_string(),
                        value: 70.0
                    }
                ],
            }],
        })
    );
    let response = serde_json::from_value(json!({"data": [{"date": "2026-01-09"}]})).unwrap();
    assert_eq!(
        history(response, Report::Usage, Grouping::Feature, date, date).unwrap(),
        None
    );
}

#[test]
fn signed_credits_keep_daily_activity_when_the_period_nets_to_zero() {
    let start = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 8).unwrap();
    let end = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response = serde_json::from_value(json!({"data": [
        {"date": "2026-01-08", "product_surface": "cli", "credit_amount": 0.004},
        {"date": "2026-01-09", "product_surface": "desktop_app", "credit_amount": -0.004}
    ]}))
    .unwrap();
    assert_eq!(
        history(response, Report::Credits, Grouping::Surface, start, end).unwrap(),
        Some(AccountAnalyticsHistory {
            unit: AccountAnalyticsUnit::Credits,
            updated_at: None,
            data: [
                ("2026-01-08", 0.004, "cli", "CLI"),
                ("2026-01-09", -0.004, "desktop_app", "Desktop app")
            ]
            .into_iter()
            .map(|(date, amount, key, label)| AccountAnalyticsDay {
                date: date.parse().unwrap(),
                total: amount,
                values: vec![AccountAnalyticsValue {
                    key: key.to_string(),
                    label: label.to_string(),
                    value: amount
                }],
            })
            .collect(),
        })
    );
}

#[test]
fn message_surfaces_include_both_desktop_clients_and_unmapped_counts() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response = serde_json::from_value(
        json!({"data": [{"date": "2026-01-09", "totals": {"turns": 10}, "clients": [
            {"client_id": "CODEX_DESKTOP_APP", "turns": 2},
            {"client_id": "CODEX_WORK_DESKTOP", "turns": 3},
            {"client_id": "new_client", "turns": 1}
        ]}]}),
    )
    .unwrap();
    assert_eq!(
        history(response, Report::Messages, Grouping::Surface, date, date)
            .unwrap()
            .unwrap()
            .data,
        vec![AccountAnalyticsDay {
            date: "2026-01-09".parse().unwrap(),
            total: 10.0,
            values: vec![
                AccountAnalyticsValue {
                    key: "desktop_app".to_string(),
                    label: "Desktop app".to_string(),
                    value: 5.0
                },
                AccountAnalyticsValue {
                    key: "other".to_string(),
                    label: "Other".to_string(),
                    value: 5.0
                },
            ],
        }]
    );
}

#[test]
fn enterprise_credit_series_keep_labels_signed_values_and_freshness() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response = serde_json::from_value(json!({
        "series": [{"key": "medium", "label": "Medium", "total": 12.5}, {"key": "high", "label": "High", "total": -0.004}],
        "data_freshness_ts": "2026-01-09T10:00:00Z",
        "data": [{"date": "2026-01-09", "values": {"medium": 12.5, "high": -0.004}}]
    })).unwrap();
    assert_eq!(
        history(response, Report::Credits, Grouping::Reasoning, date, date).unwrap(),
        Some(AccountAnalyticsHistory {
            unit: AccountAnalyticsUnit::Credits,
            updated_at: Some(1767952800),
            data: vec![AccountAnalyticsDay {
                date,
                total: 12.496,
                values: vec![
                    AccountAnalyticsValue {
                        key: "high".into(),
                        label: "High".into(),
                        value: -0.004
                    },
                    AccountAnalyticsValue {
                        key: "medium".into(),
                        label: "Medium".into(),
                        value: 12.5
                    }
                ],
            }],
        })
    );
}

#[test]
fn credit_timestamps_and_legacy_model_usage_follow_app_fallbacks() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let credits = serde_json::from_value(json!({"data": [{"date": "2026-01-09T05:00:00Z", "product_surface": "cli", "credit_amount": 12.5}]})).unwrap();
    let usage = serde_json::from_value(json!({"data": [{"date": "2026-01-09", "models": [{"model": "model-a", "credits": 12.5}]}]})).unwrap();
    let credits = history(credits, Report::Credits, Grouping::Surface, date, date)
        .unwrap()
        .unwrap();
    let usage = history(usage, Report::Usage, Grouping::Model, date, date)
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            credits.data[0].total,
            usage.data[0].total,
            usage.data[0].values[0].key.as_str()
        ),
        (12.5, 12.5, "model-a")
    );
}

#[test]
fn count_reports_preserve_explicit_zero_and_omit_unreported_days() {
    let start = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 8).unwrap();
    let end = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 10).unwrap();
    for (report, field) in [
        (Report::Messages, "totals"),
        (Report::Plugins, "plugin_usage_overviews"),
        (Report::Skills, "skill_usage_overviews"),
    ] {
        let value = if report == Report::Messages {
            json!({"turns": 0})
        } else {
            json!([])
        };
        let response =
            serde_json::from_value(json!({"data": [{"date": "2026-01-08", (field): value}]}))
                .unwrap();
        assert_eq!(
            history(response, report, Grouping::Surface, start, end).unwrap(),
            Some(AccountAnalyticsHistory {
                unit: AccountAnalyticsUnit::Count,
                updated_at: None,
                data: vec![AccountAnalyticsDay {
                    date: start,
                    total: 0.0,
                    values: Vec::new()
                }],
            })
        );
    }
}

#[test]
fn legacy_consumer_history_keeps_daily_products_and_models() {
    let date = "2026-09-01".parse().unwrap();
    let response: AnalyticsData = serde_json::from_value(json!({"units":"relative", "data":[{
        "date":"2026-09-01",
        "product_surface_usage_values":{"work_desktop":12,"work_web":8,"cli":30,"desktop_app":50},
        "models":[{"model":"major","credits":98.5},{"model":"one-percent","credits":1},{"model":"tiny","credits":0.3},{"model":"OTHER","credits":0.2}]
    }]})).unwrap();
    for (grouping, expected) in [
        (
            Grouping::Surface,
            vec![("codex", "Codex", 80.0), ("work", "Work", 20.0)],
        ),
        (
            Grouping::Model,
            vec![
                ("major", "major", 98.5),
                ("one-percent", "one-percent", 1.0),
                ("other", "Other", 0.5),
            ],
        ),
    ] {
        assert_eq!(
            history(response.clone(), Report::Usage, grouping, date, date).unwrap(),
            Some(AccountAnalyticsHistory {
                unit: AccountAnalyticsUnit::RelativeUsage,
                updated_at: None,
                data: vec![AccountAnalyticsDay {
                    date,
                    total: 100.0,
                    values: expected
                        .into_iter()
                        .map(|(key, label, value)| AccountAnalyticsValue {
                            key: key.into(),
                            label: label.into(),
                            value
                        })
                        .collect()
                }],
            })
        );
    }
    let mut credits = response;
    credits.units = Some("credits".into());
    let model = history(credits, Report::Usage, Grouping::Model, date, date)
        .unwrap()
        .unwrap();
    assert_eq!(
        (model.unit, model.data[0].values.len()),
        (AccountAnalyticsUnit::Credits, 4)
    );
}

#[test]
fn complete_attribution_wins_consistently_and_incomplete_attribution_is_not_mixed() {
    let date = "2026-09-01".parse().unwrap();
    let response: AnalyticsData = serde_json::from_value(json!({"units":"credits", "data":[{
        "date":"2026-09-01", "product_surface_usage_values":{"cli":999},
        "models":[{"model":"legacy","credits":999}],
        "attribution":[
            {"thread_source":"user","turn_trigger":"composer","surface":"desktop_app","model":"alpha","value":30},
            {"thread_source":"subagent","turn_trigger":"goal","surface":"cli","model":"tiny","value":0.1}
        ]
    }]})).unwrap();
    for grouping in [
        Grouping::Feature,
        Grouping::Model,
        Grouping::Surface,
        Grouping::TaskStart,
    ] {
        let normalized = history(response.clone(), Report::Usage, grouping, date, date)
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                normalized.unit,
                normalized.data[0].total,
                normalized.data[0].values.len()
            ),
            (AccountAnalyticsUnit::RelativeUsage, 30.1, 2)
        );
    }
    let incomplete = serde_json::from_value(json!({"data":[
        {"date":"2026-09-01", "attribution":[]}, {"date":"2026-09-02"}
    ]}))
    .unwrap();
    assert_eq!(
        history(
            incomplete,
            Report::Usage,
            Grouping::Feature,
            date,
            "2026-09-02".parse().unwrap()
        )
        .unwrap(),
        None
    );
}

#[test]
fn duplicate_message_dates_preserve_each_records_other_remainder() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    for grouping in [Grouping::Model, Grouping::Surface] {
        let response = serde_json::from_value(json!({"data": [
            {"date": "2026-01-09", "totals": {"turns": 10}, "models": [{"model": "example", "turns": 8}], "clients": [{"client_id": "CODEX_CLI", "turns": 8}]},
            {"date": "2026-01-09", "totals": {"turns": 10}, "models": [{"model": "example", "turns": 8}], "clients": [{"client_id": "CODEX_CLI", "turns": 8}]}
        ]})).unwrap();
        let (key, label) = if grouping == Grouping::Model {
            ("example", "example")
        } else {
            ("cli", "CLI")
        };
        assert_eq!(
            history(response, Report::Messages, grouping, date, date).unwrap(),
            Some(AccountAnalyticsHistory {
                unit: AccountAnalyticsUnit::Count,
                updated_at: None,
                data: vec![AccountAnalyticsDay {
                    date,
                    total: 20.0,
                    values: vec![
                        AccountAnalyticsValue {
                            key: key.into(),
                            label: label.into(),
                            value: 16.0
                        },
                        AccountAnalyticsValue {
                            key: "other".into(),
                            label: "Other".into(),
                            value: 4.0
                        },
                    ]
                }],
            })
        );
    }
}

#[test]
fn unsupported_message_groupings_fail_even_without_records() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    for grouping in [
        Grouping::Feature,
        Grouping::TaskStart,
        Grouping::Speed,
        Grouping::Reasoning,
        Grouping::TokenType,
    ] {
        for data in [
            json!([]),
            json!([{"date": "2026-01-09", "totals": {"turns": 10}, "clients": []}]),
        ] {
            let response = serde_json::from_value(json!({"data": data})).unwrap();
            assert_eq!(
                history(response, Report::Messages, grouping, date, date),
                Err(Error::UnsupportedMessageGrouping(grouping))
            );
        }
    }
}

fn fail(report: Report, grouping: Grouping, record: serde_json::Value) -> Error {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response: AnalyticsData = serde_json::from_value(json!({"data": [record]})).unwrap();
    history(response, report, grouping, date, date).unwrap_err()
}

#[test]
fn a_record_date_that_is_no_date_names_the_date() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let response: AnalyticsData =
        serde_json::from_value(json!({"data": [{"date": "not a date"}]})).unwrap();
    assert_eq!(
        has_complete_attribution(&response, date, date),
        Err(Error::InvalidDate)
    );
    assert_eq!(
        fail(Report::Usage, Grouping::Model, json!({"date": "yesterday"})),
        Error::InvalidDate
    );
}

#[test]
fn an_unsupported_attribution_grouping_names_the_grouping() {
    for grouping in [Grouping::Speed, Grouping::Reasoning, Grouping::TokenType] {
        assert_eq!(
            fail(
                Report::Usage,
                grouping,
                json!({"date": "2026-01-09", "attribution": [{"thread_source": "user", "value": 1}]})
            ),
            Error::UnsupportedAttributionGrouping(grouping)
        );
    }
}

#[test]
fn a_number_that_is_not_an_amount_names_which_one() {
    let cases = [
        (
            Report::Usage,
            Grouping::Surface,
            json!({"date": "2026-01-09", "product_surface_usage_values": {"codex": -1}}),
            Amount::Usage,
        ),
        (
            Report::Usage,
            Grouping::Model,
            json!({"date": "2026-01-09", "attribution": [{"model": "a", "value": -1}]}),
            Amount::Attribution,
        ),
        (
            Report::Messages,
            Grouping::Model,
            json!({"date": "2026-01-09", "totals": {"turns": -1}}),
            Amount::MessageTotal,
        ),
        (
            Report::Messages,
            Grouping::Model,
            json!({"date": "2026-01-09", "totals": {"turns": 1}, "models": [{"model": "a", "turns": -1}]}),
            Amount::ModelTurns,
        ),
        (
            Report::Messages,
            Grouping::Surface,
            json!({"date": "2026-01-09", "totals": {"turns": 1}, "clients": [{"client_id": "CODEX_CLI", "turns": -1}]}),
            Amount::ClientTurns,
        ),
        (
            Report::Skills,
            Grouping::Model,
            json!({"date": "2026-01-09", "skill_usage_overviews": [{"display_name": "a", "invocation_counts": -1}]}),
            Amount::ToolInvocations,
        ),
    ];
    for (report, grouping, record, amount) in cases {
        assert_eq!(fail(report, grouping, record), Error::InvalidAmount(amount));
    }
}

#[test]
fn a_credit_amount_that_is_not_finite_names_credits_and_the_day() {
    let date = NaiveDate::from_ymd_opt(/*year*/ 2026, /*month*/ 1, /*day*/ 9).unwrap();
    let mut response: AnalyticsData = serde_json::from_value(json!({"data": [{"date": "2026-01-09",
        "premium_usage_values": {"credit_usage_credits": {"codex": 1}}}]}))
    .unwrap();
    let premium = response.data[0].premium_usage_values.as_mut().unwrap();
    premium
        .credit_usage_credits
        .insert("codex".into(), f64::INFINITY);
    assert_eq!(
        history(response, Report::Credits, Grouping::Surface, date, date),
        Err(Error::InvalidAmount(Amount::Credits))
    );
    let mut response: AnalyticsData = serde_json::from_value(json!({
        "series": [{"key": "a", "label": "A"}],
        "data": [{"date": "2026-01-09", "values": {"a": 1}}]}))
    .unwrap();
    response.data[0]
        .values
        .as_mut()
        .unwrap()
        .insert("a".into(), f64::INFINITY);
    assert_eq!(
        history(response, Report::Credits, Grouping::Surface, date, date),
        Err(Error::InvalidAmount(Amount::Day))
    );
}

#[test]
fn a_number_the_record_lacks_names_which_one() {
    let cases = [
        (
            Report::Usage,
            Grouping::Model,
            json!({"date": "2026-01-09", "models": [{"model": "a"}]}),
            Gap::PlanCredits,
        ),
        (
            Report::Messages,
            Grouping::Model,
            json!({"date": "2026-01-09"}),
            Gap::MessageTotal,
        ),
        (
            Report::Messages,
            Grouping::Model,
            json!({"date": "2026-01-09", "totals": {"turns": 1}, "models": [{"model": "a"}]}),
            Gap::ModelTurns,
        ),
        (
            Report::Plugins,
            Grouping::Model,
            json!({"date": "2026-01-09"}),
            Gap::ToolActivity,
        ),
    ];
    for (report, grouping, record, gap) in cases {
        assert_eq!(fail(report, grouping, record), Error::Unreported(gap));
    }
}
