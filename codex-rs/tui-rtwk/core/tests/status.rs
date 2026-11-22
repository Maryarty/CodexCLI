#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S19: the card a script's `screen` step shows holds the raw
//! values; the blended total is the oracle's (`TokenUsage::blended_total`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::status::{Approval, Profile};
use pretty_assertions::assert_eq;

const CARD: &str = r#"{"step":"screen","dir":"status","ev":{"kind":"card","version":"0.0.0","model":"gpt-5.1-codex-max","reasoning_effort":"high","reasoning_summary":"detailed","directory":"/w","profile":"workspace","approval":"on_request","tokens":{"input":1200,"cached_input":200,"output":900,"total":2250},"context":null,"limits":[]}}"#;

#[test]
fn the_card_keeps_the_values_it_was_shown_with() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    let card = sandbox.app().screens().status.card.clone().expect("a card");
    assert_eq!(card.model, "gpt-5.1-codex-max");
    assert_eq!(card.profile, Profile::Workspace);
    assert_eq!(card.approval, Approval::OnRequest);
}

#[test]
fn the_blended_total_leaves_out_cached_input() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    let card = sandbox.app().screens().status.card.clone().expect("a card");
    assert_eq!(card.tokens.blended_total(), 1_900);
}

#[test]
fn a_card_missing_a_field_is_a_bad_seed() {
    let err = Sandbox::play(r#"{"step":"screen","dir":"status","ev":{"kind":"card"}}"#)
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
    fn a_status_card_script_step_seeds_the_cell_and_not_the_dialog_card() {
        let script = r#"{"step":"screen","dir":"status","ev":{"kind":"status_card","version":"0.0.0","model":"m","directory":"/w","permissions":{"sandbox":"read_only","approval":"never"},"tokens":{"input":1,"cached_input":0,"output":2,"total":3}}}"#;
        let sandbox = Play::play(script).expect("plays");
        let status = &sandbox.app().screens().status;
        let cell = status.cell.clone().expect("a cell");
        assert_eq!(cell.model, "m");
        assert_eq!(cell.permissions.approval, Approval::Never);
        assert_eq!(cell.tokens.blended_total(), 3);
        assert!(status.card.is_none());
    }

    /// The built-in read-only label with the usual approval.
    fn read_only(network: bool) -> PermissionsLabel {
        PermissionsLabel::ReadOnly {
            network,
            approval: ApprovalLabel::Ask,
        }
    }
}
