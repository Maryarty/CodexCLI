#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S19: the card a script's `screen` step shows holds the raw
//! values; the blended total is the oracle's (`TokenUsage::blended_total`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::status::Approval;
use pretty_assertions::assert_eq;

const CARD: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"screen","dir":"status","ev":{"kind":"status_card","version":"0.0.0","model":"gpt-5.1-codex-max","directory":"/w","permissions":{"sandbox":"workspace","approval":"on_request"},"tokens":{"input":1200,"cached_input":200,"output":900,"total":2250}}}"#;

/// The card a script seeds is a cell of the shown thread's transcript.
fn card(sandbox: &Sandbox) -> std::sync::Arc<codex_tui_core::screens::status::StatusCard> {
    sandbox
        .app()
        .active_thread()
        .transcript()
        .last_status()
        .expect("a status card in the transcript")
        .clone()
}

#[test]
fn the_card_keeps_the_values_it_was_shown_with() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    let card = card(&sandbox);
    assert_eq!(card.model, "gpt-5.1-codex-max");
    assert_eq!(card.permissions.approval, Approval::OnRequest);
}

#[test]
fn the_blended_total_leaves_out_cached_input() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    assert_eq!(card(&sandbox).tokens.blended_total(), 1_900);
}

#[test]
fn a_card_missing_a_field_is_a_bad_seed() {
    let err = Sandbox::play(r#"{"step":"screen","dir":"status","ev":{"kind":"status_card"}}"#)
        .err()
        .expect("fails");
    assert_eq!(err.line, 1);
}

// The cell's card (S19 chunk 1): the helpers carried from the oracle's
// `status/helpers.rs`, `status/rate_limits.rs` and `status/tests.rs`, named as
// the oracle's tests. The words are the view's string table's, so the values
// are the typed ones the view spells.
mod cell {
    use codex_protocol::account::PlanType;
    use codex_tui_core::fixtures::sandbox::Sandbox as Play;
    use codex_tui_core::screens::status::{
        Account, AgentsSource, Approval, ApprovalLabel, CreditsText, LimitData, LimitRow,
        ModelDetail, Permissions, PermissionsLabel, PlanLabel, Reasoning, ResetDate, ResetTime,
        RowLabel, RowValue, SandboxKind, SandboxText, StatusCard, WindowName,
        compose_agents_summary, model_details, plan_label, reset_time,
    };
    use pretty_assertions::assert_eq;

    /// 2024-01-02 03:04:05 UTC.
    const CAPTURED: i64 = 1_704_164_645;

    fn card(extra: &str) -> StatusCard {
        let json = format!(
            r#"{{"version":"0.0.0","model":"gpt-5.1-codex","directory":"/workspace",
            "permissions":{{"sandbox":"workspace","network":true,"approval":"on_request"}},
            "tokens":{{"input":0,"cached_input":0,"output":0,"total":0}}{extra}}}"#
        );
        serde_json::from_str(&json).expect("a card")
    }

    fn limits_card(snapshots: &str, now: i64) -> StatusCard {
        card(&format!(
            r#","limits":{{"now":{now},"snapshots":{snapshots}}}"#
        ))
    }

    fn rows_of(card: &StatusCard) -> Vec<LimitRow> {
        match card.limit_data() {
            LimitData::Available(rows) | LimitData::Stale(rows) => rows,
            other => panic!("unexpected limits: {other:?}"),
        }
    }

    fn labels_of(snapshots: &str) -> Vec<RowLabel> {
        rows_of(&limits_card(snapshots, CAPTURED))
            .into_iter()
            .map(|row| row.label)
            .collect()
    }

    fn permissions(profile: Option<&str>, sandbox: SandboxKind, network: bool) -> Permissions {
        Permissions {
            profile: profile.map(str::to_string),
            sandbox,
            network,
            disabled: false,
            approval: Approval::OnRequest,
            auto_review: false,
            extra_roots: Vec::new(),
        }
    }

    fn text(sandbox: SandboxKind, network: bool, roots: &[&str]) -> SandboxText {
        SandboxText {
            sandbox,
            network,
            roots: roots.iter().map(ToString::to_string).collect(),
        }
    }

    // ORACLE status/helpers.rs plan_type_display_name_remaps_display_labels
    #[test]
    fn plan_type_display_name_remaps_display_labels() {
        let cases = [
            (PlanType::Free, PlanLabel::Free),
            (PlanType::Go, PlanLabel::Go),
            (PlanType::Plus, PlanLabel::Plus),
            (PlanType::Pro, PlanLabel::Pro200),
            (PlanType::ProLite, PlanLabel::Pro100),
            (PlanType::ProMax, PlanLabel::Pro500),
            (PlanType::Team, PlanLabel::Business),
            (PlanType::SelfServeBusinessUsageBased, PlanLabel::Business),
            (PlanType::Business, PlanLabel::Enterprise),
            (
                PlanType::EnterpriseCbpAutomation,
                PlanLabel::EnterpriseAutomation,
            ),
            (PlanType::EnterpriseCbpUsageBased, PlanLabel::Enterprise),
            (PlanType::Enterprise, PlanLabel::Enterprise),
            (PlanType::Edu, PlanLabel::Edu),
            (PlanType::EduPlus, PlanLabel::EduPlus),
            (PlanType::EduPro, PlanLabel::EduPro),
            (
                PlanType::SelfServeBusinessProLite,
                PlanLabel::BusinessPremium,
            ),
            (PlanType::Sci, PlanLabel::AcademicResearchers),
            (PlanType::SciPlus, PlanLabel::AcademicResearchersPlus),
            (PlanType::Unknown, PlanLabel::Unknown),
        ];
        for (plan, label) in cases {
            assert_eq!(plan_label(plan), label, "{plan:?}");
        }
    }

    // ORACLE status/helpers.rs
    // compose_agents_summary_collapses_home_and_preserves_project_relative_paths
    #[test]
    fn compose_agents_summary_collapses_home_and_preserves_project_relative_paths() {
        let sources = [
            "/home/me/.codex/AGENTS.md",
            "/home/me/workspace/AGENTS.md",
            "/home/me/workspace/project/AGENTS.md",
            "/home/me/workspace/project/nested/AGENTS.md",
        ]
        .map(|path| AgentsSource::Native(path.to_string()));
        assert_eq!(
            compose_agents_summary("/home/me/workspace/project", Some("/home/me"), &sources)
                .join(", "),
            "~/.codex/AGENTS.md, ../AGENTS.md, AGENTS.md, nested/AGENTS.md"
        );
    }

    // ORACLE status/helpers.rs
    // compose_agents_summary_shows_relative_native_and_full_foreign_paths
    #[test]
    fn compose_agents_summary_shows_relative_native_and_full_foreign_paths() {
        let sources = [
            AgentsSource::Native("/work/AGENTS.md".to_string()),
            AgentsSource::Foreign(r"C:\remote workspace\AGENTS.md".to_string()),
        ];
        assert_eq!(
            compose_agents_summary("/work", None, &sources).join(", "),
            r"AGENTS.md, C:\remote workspace\AGENTS.md"
        );
    }

    // ORACLE status/helpers.rs compose_agents_summary_includes_global_agents_path
    // and compose_agents_summary_names_global_agents_override
    #[test]
    fn compose_agents_summary_includes_global_agents_path() {
        for name in ["global.md", "override.md"] {
            let path = format!("/codex/{name}");
            assert_eq!(
                compose_agents_summary("/work", None, &[AgentsSource::Native(path.clone())]),
                vec![path]
            );
        }
        assert!(compose_agents_summary("/work", None, &[]).is_empty());
    }

    // ORACLE status/helpers.rs compose_agents_summary_orders_global_before_project_agents
    #[test]
    fn compose_agents_summary_orders_global_before_project_agents() {
        let sources = [
            AgentsSource::Native("/codex/global.md".to_string()),
            AgentsSource::Native("/work/project.md".to_string()),
        ];
        assert_eq!(
            compose_agents_summary("/work", None, &sources),
            vec!["/codex/global.md".to_string(), "project.md".to_string()]
        );
    }

    // ORACLE status/rate_limits.rs non_codex_single_limit_renders_combined_row
    #[test]
    fn non_codex_single_limit_renders_combined_row() {
        let credits = |balance: u32| {
            format!(r#""credits":{{"has_credits":true,"unlimited":false,"balance":"{balance}"}}"#)
        };
        let snapshots = format!(
            r#"[{{"name":"codex","captured_at":{CAPTURED},"primary":{{"used_percent":10,"window_minutes":300}},{}}},
               {{"name":"codex-other","captured_at":{CAPTURED},"primary":{{"used_percent":20,"window_minutes":300}},{}}}]"#,
            credits(25),
            credits(99)
        );
        assert_eq!(
            labels_of(&snapshots),
            vec![
                RowLabel::Window {
                    bucket: None,
                    name: WindowName::FiveHours
                },
                RowLabel::Credits,
                RowLabel::Window {
                    bucket: Some("codex-other".to_string()),
                    name: WindowName::FiveHours
                },
                RowLabel::Credits,
            ]
        );
    }

    // ORACLE status/rate_limits.rs non_codex_multi_limit_keeps_group_row
    #[test]
    fn non_codex_multi_limit_keeps_group_row() {
        let snapshots = format!(
            r#"[{{"name":"codex-other","captured_at":{CAPTURED},
               "primary":{{"used_percent":20,"window_minutes":60}},
               "secondary":{{"used_percent":40,"window_minutes":120}}}}]"#
        );
        assert_eq!(
            labels_of(&snapshots),
            vec![
                RowLabel::Bucket("codex-other".to_string()),
                RowLabel::Window {
                    bucket: None,
                    name: WindowName::Generic { secondary: false }
                },
                RowLabel::Window {
                    bucket: None,
                    name: WindowName::Generic { secondary: true }
                },
            ]
        );
    }

    // ORACLE status/tests.rs stale_monthly_limit_marks_fresh_rolling_snapshot_stale
    #[test]
    fn stale_monthly_limit_marks_fresh_rolling_snapshot_stale() {
        let snapshots = format!(
            r#"[{{"captured_at":{CAPTURED},"primary":{{"used_percent":20,"window_minutes":300}},
               "individual_limit":{{"captured_at":{},"remaining_percent":68,"used":"8000","limit":"25000","resets_at":{}}}}}]"#,
            CAPTURED - 20 * 60,
            CAPTURED + 3600
        );
        assert!(matches!(
            limits_card(&snapshots, CAPTURED).limit_data(),
            LimitData::Stale(_)
        ));
    }

    // ORACLE status/tests.rs status_snapshot_shows_stale_limits_message,
    // _missing_limits_message, _unavailable_limits_message
    #[test]
    fn a_snapshot_older_than_fifteen_minutes_is_stale_and_none_is_missing() {
        let one = r#"[{"captured_at":1000,"primary":{"used_percent":1,"window_minutes":300}}]"#;
        let data = |now| limits_card(one, now).limit_data();
        assert!(matches!(data(1000 + 15 * 60), LimitData::Available(_)));
        assert!(matches!(data(1000 + 15 * 60 + 1), LimitData::Stale(_)));
        assert_eq!(limits_card("[]", 0).limit_data(), LimitData::Missing);
        assert_eq!(
            limits_card(r#"[{"captured_at":1000}]"#, 1000).limit_data(),
            LimitData::Unavailable
        );
    }

    fn credit_values(balance: Option<&str>, has_credits: bool, unlimited: bool) -> Vec<RowValue> {
        let balance = balance.map_or(String::new(), |b| format!(r#","balance":"{b}""#));
        let snapshots = format!(
            r#"[{{"captured_at":{CAPTURED},"credits":{{"has_credits":{has_credits},"unlimited":{unlimited}{balance}}}}}]"#
        );
        match limits_card(&snapshots, CAPTURED).limit_data() {
            LimitData::Available(rows) => rows.into_iter().map(|row| row.value).collect(),
            LimitData::Unavailable => Vec::new(),
            other => panic!("unexpected: {other:?}"),
        }
    }

    // ORACLE status/tests.rs status_snapshot_shows_unlimited_credits and
    // status_snapshot_respects_unlimited_without_has_credits_flag
    #[test]
    fn unlimited_credits_show_without_the_has_credits_flag() {
        for has_credits in [true, false] {
            assert_eq!(
                credit_values(None, has_credits, true),
                vec![RowValue::Credits(CreditsText::Unlimited)]
            );
        }
    }

    // ORACLE status/tests.rs status_snapshot_shows_positive_credits
    #[test]
    fn status_snapshot_shows_positive_credits() {
        assert_eq!(
            credit_values(Some("12.5"), true, false),
            vec![RowValue::Credits(CreditsText::Balance("13".to_string()))]
        );
        assert_eq!(
            credit_values(Some(" 7 "), true, false),
            vec![RowValue::Credits(CreditsText::Balance("7".to_string()))]
        );
    }

    // ORACLE status/tests.rs status_snapshot_shows_available_credits_without_display_balance
    #[test]
    fn status_snapshot_shows_available_credits_without_display_balance() {
        for balance in [None, Some(""), Some("0"), Some("NaN"), Some("inf")] {
            assert_eq!(
                credit_values(balance, true, false),
                vec![RowValue::Credits(CreditsText::Available)],
                "{balance:?}"
            );
        }
    }

    // ORACLE status/tests.rs status_snapshot_cached_limits_hide_credits_without_flag
    #[test]
    fn status_snapshot_cached_limits_hide_credits_without_flag() {
        assert!(credit_values(Some("40"), false, false).is_empty());
    }

    // ORACLE status/tests.rs status_snapshot_includes_enterprise_monthly_credit_limit
    #[test]
    fn status_snapshot_includes_enterprise_monthly_credit_limit() {
        let snapshots = format!(
            r#"[{{"captured_at":{CAPTURED},"individual_limit":{{"remaining_percent":68,"used":"8000","limit":"25000","resets_at":{}}}}}]"#,
            CAPTURED + 3 * 86_400
        );
        let rows = rows_of(&limits_card(&snapshots, CAPTURED));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, RowLabel::MonthlyCreditLimit);
        let RowValue::Window {
            percent_used,
            resets_at,
            details,
        } = &rows[0].value
        else {
            panic!("a window row")
        };
        assert_eq!(*percent_used, 32);
        assert_eq!(
            *resets_at,
            Some(ResetTime {
                hour: 3,
                minute: 4,
                date: Some(ResetDate { day: 5, month: 1 })
            })
        );
        let details = details.as_ref().expect("a details line");
        assert_eq!(
            (details.used.as_str(), details.limit.as_str()),
            ("8,000", "25,000")
        );
    }

    // ORACLE status/tests.rs reset_timestamps_follow_clock_preference: the
    // clock reading and the date off the capture day (the clock preference is
    // the view's).
    #[test]
    fn reset_timestamps_follow_clock_preference() {
        // 2026-09-21 00:00 UTC
        let captured = 1_789_948_800_i64;
        let at = |hours: i64| reset_time(captured + hours * 3_600, captured, 0);
        assert_eq!((at(0).hour, at(0).minute, at(0).date), (0, 0, None));
        assert_eq!((at(12).hour, at(12).minute, at(12).date), (12, 0, None));
        assert_eq!((at(23).hour, at(23).minute, at(23).date), (23, 0, None));
        assert_eq!(
            at(24),
            ResetTime {
                hour: 0,
                minute: 0,
                date: Some(ResetDate { day: 22, month: 9 })
            }
        );
    }

    #[test]
    fn a_reset_is_read_in_the_local_time_zone() {
        // 03:04 UTC is 08:34 at +05:30, the same local day as the capture.
        let t = reset_time(CAPTURED, CAPTURED, 5 * 3_600 + 1_800);
        assert_eq!((t.hour, t.minute, t.date), (8, 34, None));
        // Captured at 23:00 and reset at 01:00: the next day.
        let captured = CAPTURED - CAPTURED % 86_400 + 23 * 3_600;
        let t = reset_time(captured + 2 * 3_600, captured, 0);
        assert_eq!((t.hour, t.date), (1, Some(ResetDate { day: 3, month: 1 })));
    }

    // ORACLE status/tests.rs status_permissions_non_default_workspace_write_uses_workspace_label
    #[test]
    fn status_permissions_non_default_workspace_write_uses_workspace_label() {
        assert_eq!(
            permissions(None, SandboxKind::Workspace, true).label(),
            PermissionsLabel::Custom {
                sandbox: text(SandboxKind::Workspace, true, &[]),
                approval: ApprovalLabel::Ask
            }
        );
    }

    // ORACLE status/tests.rs status_permissions_named_read_only_profile_shows_builtin_label
    // and _read_only_profile_shows_additional_writable_roots
    #[test]
    fn status_permissions_named_read_only_profile_shows_builtin_label() {
        let mut p = permissions(Some(":read-only"), SandboxKind::ReadOnly, false);
        p.extra_roots = vec!["/workspace/extra".to_string()];
        assert_eq!(
            p.label(),
            read_only(false),
            "the roots stay out of the built-in read-only label"
        );
        assert_eq!(
            permissions(Some(":read-only"), SandboxKind::ReadOnly, true).label(),
            read_only(true)
        );
        assert_eq!(
            permissions(Some(":read-only"), SandboxKind::Workspace, false).label(),
            read_only(false)
        );
    }

    // ORACLE status/tests.rs status_permissions_named_workspace_profile_shows_builtin_label,
    // _broadened_workspace_profile_shows_builtin_label
    #[test]
    fn status_permissions_named_workspace_profile_shows_builtin_label() {
        for network in [false, true] {
            assert_eq!(
                permissions(Some(":workspace"), SandboxKind::Workspace, network).label(),
                PermissionsLabel::Workspace {
                    network,
                    roots: Vec::new(),
                    approval: ApprovalLabel::Ask
                }
            );
        }
    }

    // ORACLE status/tests.rs status_permissions_workspace_auto_review_shows_reviewer_label
    #[test]
    fn status_permissions_workspace_auto_review_shows_reviewer_label() {
        let mut p = permissions(Some(":workspace"), SandboxKind::Workspace, false);
        p.auto_review = true;
        assert_eq!(
            p.label(),
            PermissionsLabel::Workspace {
                network: false,
                roots: Vec::new(),
                approval: ApprovalLabel::ApproveForMe
            }
        );
    }

    // ORACLE status/tests.rs status_permissions_workspace_roots_show_additional_directories
    // and _workspace_roots_include_profile_defined_directories
    #[test]
    fn status_permissions_workspace_roots_show_additional_directories() {
        let mut p = permissions(Some(":workspace"), SandboxKind::Workspace, false);
        p.extra_roots = vec!["/workspace/shared".to_string()];
        assert_eq!(
            p.label(),
            PermissionsLabel::Workspace {
                network: false,
                roots: vec!["/workspace/shared".to_string()],
                approval: ApprovalLabel::Ask
            }
        );
        let mut p = permissions(None, SandboxKind::Workspace, false);
        p.extra_roots = vec!["/workspace/extra".to_string()];
        assert_eq!(
            p.label(),
            PermissionsLabel::Workspace {
                network: false,
                roots: vec!["/workspace/extra".to_string()],
                approval: ApprovalLabel::Ask
            }
        );
    }

    // ORACLE status/tests.rs status_permissions_user_defined_profile_shows_name
    #[test]
    fn status_permissions_user_defined_profile_shows_name() {
        assert_eq!(
            permissions(Some("locked"), SandboxKind::ReadOnly, false).label(),
            PermissionsLabel::Profile {
                id: "locked".to_string(),
                sandbox: text(SandboxKind::ReadOnly, false, &[]),
                approval: ApprovalLabel::Ask
            }
        );
    }

    // ORACLE status/tests.rs status_permissions_named_profile_shows_additional_writable_roots
    #[test]
    fn status_permissions_named_profile_shows_additional_writable_roots() {
        let mut p = permissions(Some("mine"), SandboxKind::Workspace, false);
        p.extra_roots = vec!["/a".to_string(), "/b".to_string()];
        let PermissionsLabel::Profile { sandbox, .. } = p.label() else {
            panic!("a profile")
        };
        assert_eq!(sandbox, text(SandboxKind::Workspace, false, &["/a", "/b"]));
        // a read-only sandbox has no workspace to add the roots to
        let mut p = permissions(Some("mine"), SandboxKind::ReadOnly, false);
        p.extra_roots = vec!["/a".to_string()];
        let PermissionsLabel::Profile { sandbox, .. } = p.label() else {
            panic!("a profile")
        };
        assert_eq!(sandbox, text(SandboxKind::ReadOnly, false, &[]));
    }

    // ORACLE status/tests.rs status_permissions_include_executor_profile_root:
    // the root of another system arrives spelled by the host.
    #[test]
    fn status_permissions_include_executor_profile_root() {
        let mut p = permissions(Some("executor"), SandboxKind::Workspace, false);
        p.extra_roots = vec![r"\\server\share\foreign".to_string()];
        assert_eq!(
            p.label(),
            PermissionsLabel::Profile {
                id: "executor".to_string(),
                sandbox: text(SandboxKind::Workspace, false, &[r"\\server\share\foreign"]),
                approval: ApprovalLabel::Ask
            }
        );
    }

    // ORACLE status/tests.rs status_permissions_full_disk_managed_with_network_is_danger_full_access
    // and _without_network_is_external_sandbox
    #[test]
    fn status_permissions_full_disk_managed_is_a_custom_sandbox() {
        for (sandbox, network) in [
            (SandboxKind::DangerFullAccess, true),
            (SandboxKind::ExternalSandbox, false),
        ] {
            assert_eq!(
                permissions(None, sandbox, network).label(),
                PermissionsLabel::Custom {
                    sandbox: text(sandbox, network, &[]),
                    approval: ApprovalLabel::Ask
                }
            );
        }
    }

    #[test]
    fn the_disabled_profile_is_full_access_or_no_sandbox() {
        let mut p = permissions(
            Some(":danger-full-access"),
            SandboxKind::DangerFullAccess,
            false,
        );
        p.disabled = true;
        assert_eq!(
            p.label(),
            PermissionsLabel::NoSandbox {
                approval: ApprovalLabel::Ask
            }
        );
        p.approval = Approval::Never;
        assert_eq!(p.label(), PermissionsLabel::FullAccess);
        // without a named profile, `never` over no sandbox is full access too
        p.profile = None;
        assert_eq!(p.label(), PermissionsLabel::FullAccess);
    }

    // ORACLE status/card.rs status_approval_label
    #[test]
    fn a_policy_other_than_on_request_names_itself() {
        let mut p = permissions(None, SandboxKind::Workspace, true);
        p.approval = Approval::UnlessTrusted;
        let PermissionsLabel::Custom { approval, .. } = p.label() else {
            panic!("custom")
        };
        assert_eq!(approval, ApprovalLabel::Policy(Approval::UnlessTrusted));
    }

    // ORACLE status/helpers.rs compose_model_display
    #[test]
    fn the_model_details_name_the_effort_and_the_summaries() {
        let reasoning = |effort: &str, summary: Option<&str>| Reasoning {
            effort: effort.to_string(),
            summary: summary.map(str::to_string),
        };
        assert_eq!(
            model_details(&reasoning("High", Some("detailed"))),
            vec![
                ModelDetail::Reasoning("high".to_string()),
                ModelDetail::Summaries("detailed".to_string())
            ]
        );
        assert_eq!(
            model_details(&reasoning("none", Some(" OFF "))),
            vec![
                ModelDetail::Reasoning("none".to_string()),
                ModelDetail::SummariesOff
            ]
        );
        // a remote session leaves the summaries to the server
        assert_eq!(
            model_details(&reasoning("medium", None)),
            vec![ModelDetail::Reasoning("medium".to_string())]
        );
        assert!(card("").model_details().is_empty());
    }

    // ORACLE status/card.rs: no token row for a ChatGPT account; the fork row
    // needs the session's own id.
    #[test]
    fn the_card_hides_token_usage_for_chatgpt_accounts_and_forks_need_a_session() {
        assert!(card("").shows_token_usage());
        let chatgpt = card(r#","account":{"chat_gpt":{"plan":"pro"}}"#);
        assert!(!chatgpt.shows_token_usage());
        assert_eq!(
            chatgpt.account.as_ref().and_then(Account::plan_label),
            Some(PlanLabel::Pro200)
        );
        assert!(card(r#","account":"api_key""#).shows_token_usage());
        assert_eq!(card(r#","forked_from":"a""#).forked_from(), None);
        assert_eq!(
            card(r#","session_id":"s","forked_from":"a""#).forked_from(),
            Some("a")
        );
    }

    #[test]
    fn a_status_card_script_step_puts_the_cell_into_the_transcript() {
        let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"screen","dir":"status","ev":{"kind":"status_card","version":"0.0.0","model":"m","directory":"/w","permissions":{"sandbox":"read_only","approval":"never"},"tokens":{"input":1,"cached_input":0,"output":2,"total":3}}}"#;
        let sandbox = Play::play(script).expect("plays");
        let cell = sandbox
            .app()
            .active_thread()
            .transcript()
            .last_status()
            .expect("a cell")
            .clone();
        assert_eq!(cell.model, "m");
        assert_eq!(cell.permissions.approval, Approval::Never);
        assert_eq!(cell.tokens.blended_total(), 3);
    }

    /// The built-in read-only label with the usual approval.
    fn read_only(network: bool) -> PermissionsLabel {
        PermissionsLabel::ReadOnly {
            network,
            approval: ApprovalLabel::Ask,
        }
    }
}

// What a read of the account's limits gives and why it was asked (oracle
// `RateLimitRefreshOrigin`, `rate_limit_snapshots_by_limit_id`,
// `app_server_rate_limit_snapshots`): the app keeps every bucket of a read per
// backend, a read replaces only the buckets it names, a rolling update never
// writes them, an account change forgets them.
mod rates {
    use codex_tui_core::app::{
        AppEv, AppFx, BackendEv, CodexApp, Command, Intent, Now, apply_app,
    };
    use codex_tui_core::ids::{BackendId, ThreadId};
    use codex_tui_core::protocol;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::status::{Credits, ResetCredits, Window};
    use codex_tui_core::screens::{ScreenEv, ScreenIntent, chat, usage};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    pub(super) fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    pub(super) fn ready() -> CodexApp {
        let mut app = CodexApp::new();
        let fx = run(&mut app, AppEv::Connected);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a thread starts on connect");
        run(
            &mut app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        );
        app
    }

    /// The clock the shell moves on (`AppEv::Clock`), an hour east of UTC.
    pub(super) fn clock(app: &mut CodexApp, now: i64) {
        run(app, AppEv::Clock(Now::at_secs(now).in_zone(3600)));
    }

    /// The reads of the account's limits in `fx`.
    pub(super) fn reads(fx: &AppFx) -> Vec<Command> {
        fx.commands()
            .iter()
            .filter(|command| matches!(command, Command::ReadRateLimits { .. }))
            .cloned()
            .collect()
    }

    /// A usage menu asks for a read.
    fn ask(app: &mut CodexApp) -> Vec<Command> {
        let fx = run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Usage(
                usage::Intent::RefreshRateLimits,
            ))),
        );
        reads(&fx)
    }

    pub(super) fn answer(app: &mut CodexApp, read: &Command, response: serde_json::Value) {
        let (ticket, reply) = protocol::reply(read, Ok(response)).expect("a reply");
        run(app, AppEv::Reply(ticket, reply));
    }

    /// The one read in `commands` and the backend it goes to.
    fn the_read(commands: Vec<Command>) -> (Command, BackendId) {
        let [read] = commands.try_into().expect("one read");
        let Command::ReadRateLimits { backend, .. } = read else {
            panic!("a read of the limits: {read:?}");
        };
        (read, backend)
    }

    fn window(used_percent: i32, minutes: i64, resets_at: i64) -> serde_json::Value {
        json!({"usedPercent": used_percent, "windowDurationMins": minutes, "resetsAt": resets_at})
    }

    fn two_buckets() -> serde_json::Value {
        let codex = json!({"limitId": "codex", "primary": window(30, 300, 2_000_000_000)});
        json!({
            "rateLimits": codex,
            "rateLimitsByLimitId": {
                "codex": codex,
                "spark": {
                    "limitId": "spark",
                    "limitName": "GPT-5 Spark",
                    "primary": window(10, 60, 2_000_000_100),
                    "credits": {"hasCredits": true, "unlimited": false, "balance": "5"},
                },
            },
            "rateLimitResetCredits": {
                "availableCount": 2,
                "credits": [{
                    "id": "credit-1",
                    "resetType": "codexRateLimits",
                    "status": "available",
                    "grantedAt": 1_700_000_000,
                    "expiresAt": null,
                    "title": "Reset",
                    "description": null,
                }],
            },
        })
    }

    fn names(app: &CodexApp, backend: BackendId) -> Vec<String> {
        app.session().rate_snapshots[&backend]
            .snapshots()
            .map(|snapshot| snapshot.name.clone())
            .collect()
    }

    fn first_used(app: &CodexApp, backend: BackendId) -> Option<i32> {
        let store = &app.session().rate_snapshots[&backend];
        let codex = store.snapshots().next().expect("codex");
        codex.primary.map(|window| window.used_percent)
    }

    #[test]
    fn a_read_keeps_every_bucket_once_with_the_captured_moment() {
        let mut app = ready();
        clock(&mut app, 1_704_164_645);
        let (read, backend) = the_read(ask(&mut app));
        answer(&mut app, &read, two_buckets());

        // `codex` is the top-level view and the map's own entry: one bucket.
        assert_eq!(names(&app, backend), ["codex", "GPT-5 Spark"]);
        let store = &app.session().rate_snapshots[&backend];
        let spark = store.snapshots().nth(1).expect("the second bucket");
        assert_eq!(spark.captured_at, 1_704_164_645);
        assert_eq!(
            spark.primary,
            Some(Window {
                used_percent: 10,
                window_minutes: Some(60),
                resets_at: Some(2_000_000_100),
            })
        );
        assert_eq!(
            spark.credits,
            Some(Credits {
                has_credits: true,
                unlimited: false,
                balance: Some("5".to_owned()),
            })
        );
    }

    #[test]
    fn a_read_keeps_the_reset_credits_it_carried() {
        let mut app = ready();
        let (read, backend) = the_read(ask(&mut app));
        answer(&mut app, &read, two_buckets());

        let credits: &ResetCredits = app.session().rate_snapshots[&backend]
            .reset_credits()
            .expect("credits");
        assert_eq!(credits.available_count, 2);
        let listed = credits.credits.as_ref().expect("details");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "credit-1");
        assert_eq!(listed[0].title.as_deref(), Some("Reset"));
        assert_eq!(listed[0].expires_at, None);
    }

    #[test]
    fn a_later_read_replaces_the_buckets_it_names_and_keeps_the_others() {
        let mut app = ready();
        let (read, backend) = the_read(ask(&mut app));
        answer(&mut app, &read, two_buckets());
        let (again, _) = the_read(ask(&mut app));
        answer(
            &mut app,
            &again,
            json!({"rateLimits": {"limitId": "codex", "primary": window(60, 300, 2_000_000_500)}}),
        );

        assert_eq!(names(&app, backend), ["codex", "GPT-5 Spark"]);
        assert_eq!(first_used(&app, backend), Some(60));
    }

    #[test]
    fn a_rolling_update_never_writes_the_buckets() {
        let mut app = ready();
        let (read, backend) = the_read(ask(&mut app));
        answer(&mut app, &read, two_buckets());
        let update = serde_json::from_value(json!({
            "method": "account/rateLimits/updated",
            "params": {"rateLimits": {"limitId": "codex", "primary": window(99, 300, 2_000_000_900)}},
        }))
        .expect("a notification");
        run(
            &mut app,
            AppEv::Backend(BackendEv::Notification(Box::new(update))),
        );

        assert_eq!(first_used(&app, backend), Some(30));
        // the status line's summary moves with it
        let summary = app.session().rate_limits[&backend].primary;
        assert_eq!(summary.map(|window| window.used_percent), Some(99));
    }

    #[test]
    fn an_account_change_forgets_the_buckets() {
        let mut app = ready();
        let (read, backend) = the_read(ask(&mut app));
        answer(&mut app, &read, two_buckets());
        let changed = serde_json::from_value(json!({
            "method": "account/updated",
            "params": {"authMode": "chatgpt", "planType": "pro"},
        }))
        .expect("a notification");
        run(
            &mut app,
            AppEv::Backend(BackendEv::Notification(Box::new(changed))),
        );

        assert_eq!(names(&app, backend), Vec::<String>::new());
        assert_eq!(app.session().rate_snapshots[&backend].reset_credits(), None);
    }

    #[test]
    fn a_usage_menu_read_is_asked_every_time() {
        let mut app = ready();
        assert_eq!(ask(&mut app).len(), 1);
        assert_eq!(ask(&mut app).len(), 1);
    }

    #[test]
    fn a_background_refresh_folds_into_the_read_in_flight() {
        let mut app = ready();
        let facts = || {
            AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
                chat::StatusFacts {
                    account_usage: true,
                    ..chat::StatusFacts::default()
                },
            ))))
        };
        assert_eq!(reads(&run(&mut app, facts())).len(), 1);
        assert_eq!(reads(&run(&mut app, facts())).len(), 0);
        // a read the menu asks is its own, whatever is in flight
        assert_eq!(ask(&mut app).len(), 1);
    }

    #[test]
    fn the_clock_event_moves_the_status_clock() {
        let mut app = ready();
        clock(&mut app, 1_704_164_645);
        let status = &app.screens().status;
        assert_eq!((status.now, status.utc_offset_secs), (1_704_164_645, 3600));
    }
}

/// `/status` as a cell of the active thread's transcript (ADR-003): the card
/// is appended through the thread's `apply`, a read of the limits answers the
/// cells of its own request only.
mod live {
    use std::sync::Arc;

    use super::rates::{answer, clock, reads, ready, run};
    use codex_protocol::account::PlanType;
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent};
    use codex_tui_core::commands::{SessionIntent, SlashCommand};
    use codex_tui_core::ids::{BackendId, ThreadId};
    use codex_tui_core::protocol;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::status::{self, Account, AgentsSource, StatusCard};
    use codex_tui_core::screens::{ScreenEv, ScreenIntent, chat};
    use codex_tui_core::threads::Item;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const NOW: i64 = 1_704_164_645;

    fn facts(app: &mut CodexApp) {
        let facts = chat::StatusFacts {
            model: Some("gpt-5.5".into()),
            provider: Some("openai".into()),
            version: Some("0.0.0".into()),
            home_directory: Some("/home/u".into()),
            account_usage: true,
            card: status::CardFacts {
                cwd: "/repo".into(),
                reasoning_summary: Some("auto".into()),
                ..status::CardFacts::default()
            },
            ..chat::StatusFacts::default()
        };
        run(
            app,
            AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(facts)))),
        );
    }

    fn account(app: &mut CodexApp, account: Option<Account>) {
        run(
            app,
            AppEv::Screen(ScreenEv::Status(status::Ev::Account(account))),
        );
    }

    fn chatgpt() -> Account {
        Account::ChatGpt {
            email: Some("user@example.com".into()),
            plan: Some(PlanType::Pro),
            workspace: None,
        }
    }

    /// A session whose account is a ChatGPT one and whose clock is set.
    fn signed_in() -> CodexApp {
        let mut app = ready();
        facts(&mut app);
        account(&mut app, Some(chatgpt()));
        clock(&mut app, NOW);
        app
    }

    fn status(app: &mut CodexApp) -> AppFx {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
                thread: Some(ThreadId::new("thread-1")),
                command: SlashCommand::Status,
                intent: SessionIntent::Status,
            }))),
        )
    }

    /// The cells of `/status` in the active thread, oldest first, with the
    /// request each still waits for.
    fn cells(app: &CodexApp) -> Vec<(Option<u64>, Arc<StatusCard>)> {
        app.active_thread()
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Status { card, request } => Some((*request, card.clone())),
                _ => None,
            })
            .collect()
    }

    fn limits(used: i32) -> serde_json::Value {
        let codex = json!({
            "limitId": "codex",
            "primary": {"usedPercent": used, "windowDurationMins": 300, "resetsAt": 2_000_000_000},
        });
        json!({"rateLimits": codex, "rateLimitsByLimitId": {"codex": codex}})
    }

    fn used(card: &StatusCard) -> Vec<i32> {
        card.limits
            .snapshots
            .iter()
            .filter_map(|snapshot| snapshot.primary.map(|window| window.used_percent))
            .collect()
    }

    fn notify(app: &mut CodexApp, message: serde_json::Value) {
        let message = serde_json::from_value(message).expect("a notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(message))),
        );
    }

    fn started(id: &str, name: Option<&str>, forked_from: Option<&str>) -> serde_json::Value {
        json!({"method": "thread/started", "params": {"thread": {
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": forked_from, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/repo",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
            "agentRole": null, "gitInfo": null, "name": name, "daybreakEnabled": null,
            "turns": []
        }}})
    }

    fn catalog(app: &mut CodexApp) {
        run(
            app,
            AppEv::Screen(ScreenEv::Status(status::Ev::Catalog(vec![
                status::CatalogModel {
                    model: "gpt-5.5".into(),
                    display_name: "GPT-5.5".into(),
                    default_effort: Some("medium".into()),
                },
            ]))),
        );
    }

    #[test]
    fn a_status_command_leaves_a_cell_in_the_active_thread_and_asks_for_the_limits() {
        let mut app = signed_in();
        let fx = status(&mut app);

        let [(request, card)] = cells(&app).try_into().expect("one cell");
        assert_eq!(request, Some(0));
        assert!(card.limits.refreshing);
        assert_eq!(card.limits.now, NOW);
        assert_eq!(reads(&fx).len(), 1);
    }

    #[test]
    fn the_card_holds_the_facts_of_the_session() {
        let mut app = signed_in();
        status(&mut app);

        let (_, card) = cells(&app).remove(0);
        assert_eq!(card.version, "0.0.0");
        assert_eq!(card.model, "gpt-5.5");
        assert_eq!(card.directory, "/repo");
        assert_eq!(card.home.as_deref(), Some("/home/u"));
        assert_eq!(card.provider.as_deref(), Some("openai"));
        assert_eq!(card.session_id.as_deref(), Some("thread-1"));
        assert_eq!(card.account, Some(chatgpt()));
        assert!(card.usage_link);
    }

    #[test]
    fn the_catalog_names_the_model_and_gives_its_default_reasoning() {
        let mut app = signed_in();
        catalog(&mut app);
        status(&mut app);

        let (_, card) = cells(&app).remove(0);
        assert_eq!(card.model, "GPT-5.5");
        let reasoning = card
            .reasoning
            .as_ref()
            .expect("the wire has reasoning settings");
        assert_eq!(reasoning.effort, "medium");
        assert_eq!(reasoning.summary.as_deref(), Some("auto"));
    }

    #[test]
    fn the_thread_s_own_effort_beats_the_config_and_the_catalog() {
        let mut app = signed_in();
        catalog(&mut app);
        notify(
            &mut app,
            json!({"method": "thread/settings/updated", "params": {
                "threadId": "thread-1",
                "threadSettings": {
                    "cwd": "/repo", "approvalPolicy": "on-request", "approvalsReviewer": "user",
                    "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
                    "model": "gpt-5.5", "modelProvider": "openai", "serviceTier": null,
                    "effort": "high", "summary": null,
                    "collaborationMode": {"mode": "default", "settings": {
                        "model": "gpt-5.5", "reasoning_effort": "high",
                        "developer_instructions": null}},
                    "personality": null,
                },
            }}),
        );
        status(&mut app);

        let (_, card) = cells(&app).remove(0);
        assert_eq!(card.reasoning.as_ref().expect("reasoning").effort, "high");
    }

    #[test]
    fn the_card_names_the_collaboration_mode_of_the_thread() {
        let mut app = signed_in();
        status(&mut app);

        let (_, card) = cells(&app).remove(0);
        assert_eq!(card.collaboration_mode.as_deref(), Some("Default"));
    }

    #[test]
    fn a_session_without_a_chatgpt_account_reads_nothing() {
        let mut app = ready();
        facts(&mut app);
        account(&mut app, Some(Account::ApiKey));
        let fx = status(&mut app);

        let [(request, card)] = cells(&app).try_into().expect("one cell");
        assert_eq!(request, None);
        assert!(!card.limits.refreshing);
        assert!(reads(&fx).is_empty());
    }

    #[test]
    fn a_provider_that_needs_no_openai_auth_reads_nothing() {
        let mut app = ready();
        facts(&mut app);
        let mut no_auth = app.screens().chat.status.as_ref().clone();
        no_auth.account_usage = false;
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(no_auth)))),
        );
        account(&mut app, Some(chatgpt()));
        let fx = status(&mut app);

        assert!(reads(&fx).is_empty());
        assert!(!cells(&app)[0].1.usage_link);
    }

    #[test]
    fn the_reply_finishes_the_cell_with_the_limits_it_read() {
        let mut app = signed_in();
        let fx = status(&mut app);
        let [read] = reads(&fx).try_into().expect("one read");
        answer(&mut app, &read, limits(8));

        let [(request, card)] = cells(&app).try_into().expect("one cell");
        assert_eq!(request, None);
        assert!(!card.limits.refreshing);
        assert_eq!(used(&card), vec![8]);
    }

    #[test]
    fn the_cached_limits_show_at_once_and_a_failed_read_keeps_them() {
        let mut app = signed_in();
        let first = reads(&status(&mut app)).remove(0);
        answer(&mut app, &first, limits(8));

        let fx = status(&mut app);
        let (_, card) = cells(&app).remove(1);
        assert_eq!(
            used(&card),
            vec![8],
            "the second card starts with the cache"
        );
        assert!(card.limits.refreshing);

        let [read] = reads(&fx).try_into().expect("one read");
        let (ticket, reply) = protocol::reply(&read, Err("boom".into())).expect("a reply");
        run(&mut app, AppEv::Reply(ticket, reply));
        let (request, card) = cells(&app).remove(1);
        assert_eq!(request, None);
        assert!(!card.limits.refreshing);
        assert_eq!(used(&card), vec![8]);
    }

    #[test]
    fn overlapping_refreshes_update_their_own_cell_only() {
        let mut app = signed_in();
        let first = reads(&status(&mut app)).remove(0);
        let second = reads(&status(&mut app)).remove(0);
        let waiting = |app: &CodexApp| {
            cells(app)
                .into_iter()
                .map(|(request, _)| request)
                .collect::<Vec<_>>()
        };
        assert_eq!(waiting(&app), vec![Some(0), Some(1)]);

        answer(&mut app, &first, json!({"rateLimits": {}}));
        assert_eq!(waiting(&app), vec![None, Some(1)]);

        answer(&mut app, &second, limits(8));
        assert_eq!(waiting(&app), vec![None, None]);
        let cards = cells(&app);
        assert_eq!(used(&cards[0].1), Vec::<i32>::new());
        assert_eq!(used(&cards[1].1), vec![8]);
    }

    #[test]
    fn a_rate_limit_notification_does_not_cancel_the_read_a_card_waits_for() {
        let mut app = signed_in();
        let read = reads(&status(&mut app)).remove(0);
        notify(
            &mut app,
            json!({"method": "account/rateLimits/updated", "params": {
                "rateLimits": {
                    "limitId": "codex",
                    "primary": {"usedPercent": 50, "windowDurationMins": 300, "resetsAt": 2_000_000_000},
                },
            }}),
        );
        answer(&mut app, &read, limits(8));

        let [(request, card)] = cells(&app).try_into().expect("one cell");
        assert_eq!(request, None);
        assert_eq!(used(&card), vec![8]);
    }

    #[test]
    fn a_reply_after_an_account_change_is_dropped_and_the_cell_ends_empty() {
        let mut app = signed_in();
        let read = reads(&status(&mut app)).remove(0);
        notify(
            &mut app,
            json!({"method": "account/updated", "params": {"authMode": "chatgpt", "planType": "pro"}}),
        );
        let [(request, card)] = cells(&app).try_into().expect("one cell");
        assert_eq!(request, None, "the change ends the wait");
        assert!(!card.limits.refreshing);

        answer(&mut app, &read, limits(8));
        let installed = app
            .session()
            .rate_snapshots
            .get(&BackendId::default())
            .map_or(0, |store| store.snapshots().count());
        assert_eq!(installed, 0, "the old account's answer is not installed");
        assert_eq!(used(&cells(&app)[0].1), Vec::<i32>::new());
    }

    #[test]
    fn the_instruction_sources_come_with_the_thread_start() {
        let mut app = CodexApp::new();
        let fx = run(&mut app, AppEv::Connected);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a thread starts on connect");
        let sources = vec![
            AgentsSource::Native("/repo/AGENTS.md".into()),
            AgentsSource::Foreign(r"C:\remote\AGENTS.md".into()),
        ];
        run(
            &mut app,
            AppEv::Reply(
                ticket,
                Reply::thread_started_with(ThreadId::new("thread-1"), sources.clone()),
            ),
        );
        facts(&mut app);
        status(&mut app);

        assert_eq!(cells(&app)[0].1.agents, sources);
        assert_eq!(
            cells(&app)[0].1.agents_summary(),
            vec!["AGENTS.md".to_string(), r"C:\remote\AGENTS.md".to_string()]
        );
    }

    #[test]
    fn the_thread_name_and_the_fork_come_from_the_thread() {
        let mut app = signed_in();
        notify(
            &mut app,
            started("thread-1", Some("Refactor"), Some("thread-0")),
        );
        status(&mut app);

        let (_, card) = cells(&app).remove(0);
        assert_eq!(card.thread_name(), Some("Refactor"));
        assert_eq!(card.forked_from(), Some("thread-0"));
    }

    #[test]
    fn the_cell_belongs_to_the_thread_it_ran_in() {
        let mut app = signed_in();
        status(&mut app);
        let shown = app.thread(&ThreadId::new("thread-1")).expect("the thread");
        let in_thread = shown
            .transcript()
            .iter()
            .filter(|(_, item)| matches!(item, Item::Status { .. }))
            .count();
        assert_eq!(in_thread, 1);
    }

    #[test]
    fn the_cell_is_the_last_status_the_copy_picker_offers() {
        let mut app = signed_in();
        status(&mut app);
        status(&mut app);
        let thread = app.active_thread();
        let card = thread.transcript().last_status().expect("a last status");
        assert_eq!(card.session_id.as_deref(), Some("thread-1"));
    }
}
