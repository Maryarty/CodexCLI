//! The slash commands: the registry, the lookup, the popup's filter and the
//! dispatch decision, ported from codex-tui's tests. Expected values are the
//! oracle's (`slash_command.rs`, `bottom_pane/slash_commands.rs`,
//! `bottom_pane/command_popup.rs`, `utils/fuzzy-match`,
//! `chatwidget/tests/slash_commands.rs`); two tests read the oracle's source
//! itself (the enum order, the command strings).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_rtwk_core::commands::*;
use pretty_assertions::assert_eq;

// ---------------------------------------------------------------- oracle source

fn oracle(path: &str) -> String {
    let full = format!("{}/../../tui/src/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

/// `(variant, to_string, serializes)` of the oracle's enum, in order.
fn oracle_variants() -> Vec<(String, Option<String>, Vec<String>)> {
    let text = oracle("slash_command.rs");
    let body = text
        .split("pub enum SlashCommand {")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("the enum");
    let mut out = Vec::new();
    let (mut to_string, mut serialize) = (None, Vec::new());
    for line in body.lines().map(str::trim) {
        if let Some(attr) = line.strip_prefix("#[strum(") {
            for part in attr.trim_end_matches(")]").split(", ") {
                if let Some(v) = part.strip_prefix("to_string = ") {
                    to_string = Some(v.trim_matches('"').to_string());
                } else if let Some(v) = part.strip_prefix("serialize = ") {
                    serialize.push(v.trim_matches('"').to_string());
                }
            }
        } else if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
        } else if let Some(name) = line.strip_suffix(',') {
            out.push((
                name.to_string(),
                to_string.take(),
                std::mem::take(&mut serialize),
            ));
        }
    }
    out
}

fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

#[test]
fn the_registry_has_the_oracles_commands_in_the_oracles_order() {
    let oracle = oracle_variants();
    let ours: Vec<String> = SlashCommand::ALL.iter().map(|c| format!("{c:?}")).collect();
    let theirs: Vec<String> = oracle.iter().map(|(n, _, _)| n.clone()).collect();
    assert_eq!(ours, theirs);
}

#[test]
fn every_command_string_and_alias_is_the_oracles() {
    for ((variant, to_string, serialize), command) in
        oracle_variants().into_iter().zip(SlashCommand::ALL)
    {
        let name = to_string
            .clone()
            .or_else(|| serialize.first().cloned())
            .unwrap_or_else(|| kebab(&variant));
        assert_eq!(command.command(), name, "{variant}");
        assert_eq!(SlashCommand::parse(&name), Some(command), "{variant}");
        for alias in &serialize {
            assert_eq!(
                SlashCommand::parse(alias),
                Some(command),
                "{variant} {alias}"
            );
        }
    }
    assert_eq!(SlashCommand::parse("nope"), None);
}

// ------------------------------------------------------------- slash_command.rs

#[test]
fn stop_command_is_canonical_name() {
    assert_eq!(SlashCommand::Stop.command(), "stop");
}

#[test]
fn clean_alias_parses_to_stop_command() {
    assert_eq!(SlashCommand::parse("clean"), Some(SlashCommand::Stop));
}

#[test]
fn pet_alias_parses_to_pets_command() {
    assert_eq!(SlashCommand::Pets.command(), "pets");
    assert_eq!(SlashCommand::parse("pet"), Some(SlashCommand::Pets));
}

#[test]
fn certain_commands_are_available_during_task() {
    assert!(SlashCommand::Goal.available_during_task());
    assert!(SlashCommand::Ide.available_during_task());
    assert!(SlashCommand::Title.available_during_task());
    assert!(SlashCommand::Statusline.available_during_task());
    assert!(SlashCommand::Raw.available_during_task());
    assert!(SlashCommand::Raw.available_in_side_conversation());
    assert!(SlashCommand::Raw.supports_inline_args());
    assert!(SlashCommand::App.available_during_task());
}

#[test]
fn auto_review_command_is_approve() {
    assert_eq!(SlashCommand::AutoReview.command(), "approve");
    assert_eq!(
        SlashCommand::parse("approve"),
        Some(SlashCommand::AutoReview)
    );
}

// ---------------------------------------------------------------- fuzzy-match

#[test]
fn ascii_basic_indices() {
    assert_eq!(fuzzy_match("hello", "hl"), Some((vec![0, 2], -99)));
}

#[test]
fn unicode_dotted_i_istanbul_highlighting() {
    assert_eq!(fuzzy_match("İstanbul", "is"), Some((vec![0, 1], -99)));
}

#[test]
fn unicode_german_sharp_s_casefold() {
    assert!(fuzzy_match("straße", "strasse").is_none());
}

#[test]
fn prefer_contiguous_match_over_spread() {
    assert_eq!(fuzzy_match("abc", "abc").map(|m| m.1), Some(-100));
    assert_eq!(fuzzy_match("a-b-c", "abc").map(|m| m.1), Some(-98));
}

#[test]
fn start_of_string_bonus_applies() {
    assert_eq!(fuzzy_match("file_name", "file").map(|m| m.1), Some(-100));
    assert_eq!(fuzzy_match("my_file_name", "file").map(|m| m.1), Some(0));
}

#[test]
fn empty_needle_matches_with_max_score_and_no_indices() {
    assert_eq!(fuzzy_match("anything", ""), Some((vec![], i32::MAX)));
}

#[test]
fn case_insensitive_matching_basic() {
    assert_eq!(fuzzy_match("FooBar", "foO"), Some((vec![0, 1, 2], -100)));
}

#[test]
fn indices_are_deduped_for_multichar_lowercase_expansion() {
    assert_eq!(fuzzy_match("İ", "i\u{0307}"), Some((vec![0], -100)));
}

// -------------------------------------------------------------- prompt_args.rs

#[test]
fn parse_slash_name_splits_name_rest_and_offset() {
    // The doc of the oracle's `parse_slash_name`: `line[rest_offset..] == rest`.
    let line = "/review   fix it";
    let (name, rest, offset) = parse_slash_name(line).expect("a command line");
    assert_eq!((name, rest), ("review", "fix it"));
    assert_eq!(&line[offset..], rest);
    assert_eq!(parse_slash_name("/"), None);
    assert_eq!(parse_slash_name("/ x"), None);
    assert_eq!(parse_slash_name("plain"), None);
    assert_eq!(parse_slash_name("/clear"), Some(("clear", "", 6)));
}

// ------------------------------------------------------------ slash_commands.rs

fn all_enabled_flags() -> CommandFlags {
    CommandFlags {
        collaboration_modes_enabled: true,
        connectors_enabled: true,
        plugins_command_enabled: true,
        token_activity_command_enabled: true,
        service_tier_commands_enabled: true,
        daybreak_command_description: Some(""),
        goal_command_enabled: true,
        voice_command_enabled: true,
        worktrees_enabled: true,
        allow_elevate_sandbox: true,
        side_conversation_active: false,
    }
}

fn fast() -> ServiceTierCommand {
    ServiceTierCommand {
        id: "priority".to_string(),
        name: "fast".to_string(),
        description: "fastest inference".to_string(),
    }
}

#[test]
fn worktree_command_lookup_requires_feature() {
    assert_eq!(
        find_builtin_command("worktree", CommandFlags::default()),
        None
    );
    assert_eq!(
        find_builtin_command("worktree", all_enabled_flags()),
        Some(SlashCommand::Worktree)
    );
}

#[test]
fn debug_command_still_resolves_for_dispatch() {
    assert_eq!(
        find_builtin_command("debug-config", all_enabled_flags()),
        Some(SlashCommand::DebugConfig)
    );
}

#[test]
fn clear_command_resolves_for_dispatch() {
    assert_eq!(
        find_builtin_command("clear", all_enabled_flags()),
        Some(SlashCommand::Clear)
    );
}

#[test]
fn goal_command_allows_extra_os_for_dispatch() {
    assert_eq!(
        find_builtin_command("goooooooooooal", all_enabled_flags()),
        Some(SlashCommand::Goal)
    );
}

#[test]
fn stop_command_resolves_for_dispatch() {
    assert_eq!(
        find_builtin_command("stop", all_enabled_flags()),
        Some(SlashCommand::Stop)
    );
}

#[test]
fn clean_command_alias_resolves_for_dispatch() {
    assert_eq!(
        find_builtin_command("clean", all_enabled_flags()),
        Some(SlashCommand::Stop)
    );
}

#[test]
fn service_tier_commands_are_hidden_when_disabled() {
    let mut flags = all_enabled_flags();
    flags.service_tier_commands_enabled = false;
    assert_eq!(find_slash_command("fast", flags, &[fast()]), None);
}

#[test]
fn all_service_tiers_are_exposed_as_commands_after_model() {
    let slow = ServiceTierCommand {
        id: "batch".to_string(),
        name: "slow".to_string(),
        description: "slower inference with lower priority".to_string(),
    };
    let commands = vec![fast(), slow];
    let items = commands_for_input(all_enabled_flags(), &commands);
    let model_idx = items
        .iter()
        .position(|item| matches!(item, SlashCommandItem::Builtin(SlashCommand::Model)))
        .expect("model command should be visible");
    let inserted = items
        .into_iter()
        .skip(model_idx + 1)
        .take(commands.len())
        .collect::<Vec<_>>();
    let expected = commands
        .into_iter()
        .map(SlashCommandItem::ServiceTier)
        .collect::<Vec<_>>();
    assert_eq!(inserted, expected);
}

#[test]
fn goal_command_is_hidden_when_disabled() {
    let mut flags = all_enabled_flags();
    flags.goal_command_enabled = false;
    assert_eq!(find_builtin_command("goal", flags), None);
}

#[test]
fn voice_command_is_hidden_when_disabled() {
    let mut flags = all_enabled_flags();
    flags.voice_command_enabled = false;
    assert_eq!(find_builtin_command("voice", flags), None);
}

#[test]
fn usage_command_is_hidden_from_input_when_account_token_activity_is_disabled() {
    let mut flags = all_enabled_flags();
    flags.token_activity_command_enabled = false;
    assert_eq!(
        builtins_for_input(flags)
            .into_iter()
            .find(|(_, command)| *command == SlashCommand::Usage),
        None
    );
}

#[test]
fn usage_command_exact_lookup_still_resolves_when_account_token_activity_is_disabled() {
    let mut flags = all_enabled_flags();
    flags.token_activity_command_enabled = false;
    assert_eq!(
        find_builtin_command("usage", flags),
        Some(SlashCommand::Usage)
    );
}

#[test]
fn side_conversation_hides_commands_without_side_flag() {
    let commands = builtins_for_input(CommandFlags {
        side_conversation_active: true,
        ..all_enabled_flags()
    })
    .into_iter()
    .map(|(_, command)| command)
    .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec![
            SlashCommand::Ide,
            SlashCommand::Rename,
            SlashCommand::Agents,
            SlashCommand::Copy,
            SlashCommand::Export,
            SlashCommand::Raw,
            SlashCommand::Diff,
            SlashCommand::Mention,
            SlashCommand::Status,
            SlashCommand::Daemon,
            SlashCommand::Warnings,
            SlashCommand::Pwd,
            SlashCommand::Usage,
        ]
    );
}

#[test]
fn side_conversation_exact_lookup_still_resolves_hidden_commands_for_dispatch_error() {
    assert_eq!(
        find_builtin_command(
            "review",
            CommandFlags {
                side_conversation_active: true,
                ..all_enabled_flags()
            },
        ),
        Some(SlashCommand::Review)
    );
}

#[test]
fn side_conversation_exact_lookup_still_resolves_service_tier_commands_for_dispatch_error() {
    let flags = CommandFlags {
        side_conversation_active: true,
        ..all_enabled_flags()
    };
    assert_eq!(
        find_slash_command("fast", flags, &[fast()]),
        Some(SlashCommandItem::ServiceTier(fast()))
    );
}

#[test]
fn a_prefix_is_a_known_command_prefix() {
    // `has_slash_command_prefix`: a typed `/mo` is worth a popup.
    assert!(has_slash_command_prefix("mo", CommandFlags::default(), &[]));
    assert!(!has_slash_command_prefix(
        "zzzz",
        CommandFlags::default(),
        &[]
    ));
}

// ------------------------------------------------------------- command_popup.rs

fn popup(flags: CommandPopupFlags) -> CommandPopup {
    CommandPopup::new(flags, Vec::new())
}

fn names(popup: &CommandPopup) -> Vec<String> {
    popup
        .filtered_items()
        .into_iter()
        .map(|item| item.command().to_string())
        .collect()
}

#[test]
fn unavailable_side_conversation_command_is_shown_only_when_searched() {
    let mut popup = popup(CommandPopupFlags {
        side_conversation_active: true,
        ..CommandPopupFlags::default()
    });
    popup.on_composer_text_change("/".to_string());
    assert!(
        !popup
            .filtered_items()
            .contains(&CommandItem::Builtin(SlashCommand::Archive))
    );
    popup.on_composer_text_change("/arch".to_string());
    assert_eq!(
        popup.filtered_items(),
        vec![CommandItem::Builtin(SlashCommand::Archive)]
    );
    assert_eq!(popup.selected_item(), None);
    assert!(popup.rows()[0].unavailable);
    popup.on_composer_text_change("/a".to_string());
    popup.move_up();
    assert!(popup.selected_item().is_some());
}

#[test]
fn filter_includes_init_when_typing_prefix() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/in".to_string());
    assert!(names(&popup).iter().any(|n| n == "init"));
}

#[test]
fn selecting_init_by_exact_match() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/init".to_string());
    assert_eq!(
        popup.selected_item(),
        Some(CommandItem::Builtin(SlashCommand::Init))
    );
}

#[test]
fn model_is_first_suggestion_for_mo() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/mo".to_string());
    assert_eq!(names(&popup).first().map(String::as_str), Some("model"));
}

#[test]
fn service_tier_command_uses_catalog_name_and_description() {
    let tier = ServiceTierCommand {
        id: "priority".to_string(),
        name: "fast".to_string(),
        description: "Fastest inference with increased plan usage".to_string(),
    };
    let mut popup = CommandPopup::new(
        CommandPopupFlags {
            service_tier_commands_enabled: true,
            ..CommandPopupFlags::default()
        },
        vec![tier.clone()],
    );
    popup.on_composer_text_change("/fa".to_string());
    assert_eq!(popup.selected_item(), Some(CommandItem::ServiceTier(tier)));
    assert_eq!(
        popup
            .rows()
            .first()
            .and_then(|row| row.description.as_deref()),
        Some("Fastest inference with increased plan usage")
    );
}

#[test]
fn command_popup_wrap_boundary_preserves_following_choice() {
    // The oracle's snapshot is of the drawing; the rows it draws are both tiers.
    let tier = |id: &str, name: &str, description: &str| ServiceTierCommand {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
    };
    let mut popup = CommandPopup::new(
        CommandPopupFlags {
            service_tier_commands_enabled: true,
            ..CommandPopupFlags::default()
        },
        vec![
            tier("priority", "tier-one", "Use faster inference"),
            tier("default", "tier-two", "Keep default speed"),
        ],
    );
    popup.on_composer_text_change("/tier".to_string());
    let rows = popup.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].item.command(), "tier-two");
    assert_eq!(rows[1].description.as_deref(), Some("Keep default speed"));
}

#[test]
fn filtered_commands_keep_presentation_order_for_prefix() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/m".to_string());
    assert_eq!(names(&popup), ["model", "memories", "mention", "mcp"]);
}

#[test]
fn prefix_filter_limits_matches_for_ac() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/ac".to_string());
    assert!(!names(&popup).iter().any(|cmd| cmd == "compact"));
}

#[test]
fn changing_filter_resets_selection_after_scrolling() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/".to_string());
    for _ in 0..MAX_POPUP_ROWS {
        popup.move_down();
    }
    assert!(popup.state.scroll_top > 0);
    popup.on_composer_text_change("/st".to_string());
    assert_eq!(
        popup.selected_item(),
        Some(CommandItem::Builtin(SlashCommand::Status))
    );
    assert_eq!(popup.state.scroll_top, 0);
}

#[test]
fn quit_hidden_in_empty_filter_but_shown_for_prefix() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/".to_string());
    assert!(
        !popup
            .filtered_items()
            .contains(&CommandItem::Builtin(SlashCommand::Quit))
    );
    popup.on_composer_text_change("/qu".to_string());
    assert!(
        popup
            .filtered_items()
            .contains(&CommandItem::Builtin(SlashCommand::Quit))
    );
}

#[test]
fn btw_hidden_in_empty_filter_but_shown_for_prefix() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/".to_string());
    assert!(
        !popup
            .filtered_items()
            .contains(&CommandItem::Builtin(SlashCommand::Btw))
    );
    popup.on_composer_text_change("/bt".to_string());
    assert!(
        popup
            .filtered_items()
            .contains(&CommandItem::Builtin(SlashCommand::Btw))
    );
}

#[test]
fn plan_command_hidden_when_collaboration_modes_disabled() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/".to_string());
    assert!(!names(&popup).iter().any(|cmd| cmd == "plan"));
}

#[test]
fn plan_command_visible_when_collaboration_modes_enabled() {
    let mut popup = popup(CommandPopupFlags {
        collaboration_modes_enabled: true,
        worktrees_enabled: true,
        ..CommandPopupFlags::default()
    });
    popup.on_composer_text_change("/plan".to_string());
    assert_eq!(
        popup.selected_item(),
        Some(CommandItem::Builtin(SlashCommand::Plan))
    );
}

#[test]
fn debug_commands_are_hidden_from_popup() {
    let popup = popup(CommandPopupFlags::default());
    assert!(!names(&popup).iter().any(|name| name.starts_with("debug")));
}

#[test]
fn app_and_voice_rows_of_the_popup_snapshots() {
    // command_popup_app (72 wide): `/app` then `/approve`; command_popup_voice: `/voice`.
    let mut app = popup(CommandPopupFlags::default());
    app.on_composer_text_change("/app".to_string());
    assert_eq!(names(&app), ["app", "approve"]);
    let mut voice = popup(CommandPopupFlags {
        voice_command_enabled: true,
        ..CommandPopupFlags::default()
    });
    voice.on_composer_text_change("/voice".to_string());
    assert_eq!(names(&voice), ["voice"]);
}

#[test]
fn the_popup_matches_only_the_first_token_of_the_first_line() {
    let mut popup = popup(CommandPopupFlags::default());
    popup.on_composer_text_change("/init some text\nsecond".to_string());
    assert_eq!(names(&popup), ["init"]);
    popup.on_composer_text_change("no slash".to_string());
    assert!(
        popup.filtered_items().len() > 10,
        "no filter: the full list"
    );
}

// -------------------------------------------------- queue (slash_dispatch.rs:1317)

#[test]
fn queued_commands_continue_or_stop_the_queue_as_the_oracle_says() {
    use SlashCommand::*;
    for cmd in [
        Status,
        Pwd,
        Usage,
        Ps,
        Stop,
        Mcp,
        Copy,
        Raw,
        Vim,
        Diff,
        App,
        Rename,
        Voice,
        Recap,
        TestApproval,
    ] {
        assert_eq!(
            queued_command_drain_result(cmd, true, true),
            QueueDrain::Continue,
            "{cmd:?}"
        );
    }
    for cmd in [
        Model, Resume, New, Review, Agents, Skills, Import, Theme, Pets, Quit, Exit,
    ] {
        assert_eq!(
            queued_command_drain_result(cmd, true, true),
            QueueDrain::Stop,
            "{cmd:?}"
        );
    }
    assert_eq!(
        queued_command_drain_result(Cd, true, false),
        QueueDrain::Stop
    );
    assert_eq!(
        queued_command_drain_result(Cd, false, false),
        QueueDrain::Continue
    );
    assert_eq!(
        queued_command_drain_result(Worktree, false, true),
        QueueDrain::Stop
    );
    assert_eq!(
        queued_command_drain_result(Worktree, false, false),
        QueueDrain::Continue
    );
}

// ------------------------------------------------- chatwidget/tests/slash_commands.rs

fn cx() -> DispatchCx {
    DispatchCx {
        flags: CommandFlags {
            voice_command_enabled: true,
            goal_command_enabled: true,
            collaboration_modes_enabled: true,
            token_activity_command_enabled: true,
            ..CommandFlags::default()
        },
        thread_started: true,
        has_backend_auth: true,
        ..DispatchCx::default()
    }
}

fn open(target: Target, command: SlashCommand, args: &str) -> Outcome {
    Outcome::Open(Open {
        target,
        command,
        args: args.to_string(),
    })
}

fn session(command: SlashCommand, intent: SessionIntent) -> Outcome {
    Outcome::Session { command, intent }
}

#[test]
fn slash_exit_aliases_request_exit() {
    for line in ["/quit", "/exit"] {
        assert_eq!(dispatch(line, &cx()), Outcome::Quit, "{line}");
    }
}

#[test]
fn slash_logout_requests_app_server_logout() {
    assert_eq!(
        dispatch("/logout", &cx()),
        session(SlashCommand::Logout, SessionIntent::Logout)
    );
}

#[test]
fn unrecognized_slash_command_is_not_added_to_local_recall() {
    assert_eq!(
        dispatch("/does-not-exist", &cx()),
        Outcome::Notice(Notice::Unrecognized("does-not-exist".to_string()))
    );
}

#[test]
fn text_that_is_not_a_command_is_submitted() {
    for line in ["/", "/usr/bin/env ls", " /clear", "hello /clear", "/ clear"] {
        assert_eq!(dispatch(line, &cx()), Outcome::NotACommand, "{line:?}");
    }
}

#[test]
fn slash_resume_opens_picker_while_mcp_startup_is_running() {
    assert_eq!(
        dispatch("/resume", &cx()),
        open(Target::Resume, SlashCommand::Resume, "")
    );
}

#[test]
fn slash_resume_with_arg_requests_named_session() {
    assert_eq!(
        dispatch("/resume  my chat ", &cx()),
        session(
            SlashCommand::Resume,
            SessionIntent::ResumeNamed {
                name: "my chat".to_string()
            }
        )
    );
}

#[test]
fn slash_import_opens_claude_code_import_picker() {
    assert_eq!(
        dispatch("/import", &cx()),
        open(Target::Import, SlashCommand::Import, "")
    );
}

#[test]
fn slash_memories_opens_memory_menu() {
    assert_eq!(
        dispatch("/memories", &cx()),
        open(Target::Settings, SlashCommand::Memories, "")
    );
}

#[test]
fn slash_pets_opens_picker() {
    assert_eq!(
        dispatch("/pets", &cx()),
        open(Target::Pets, SlashCommand::Pets, "")
    );
    assert_eq!(
        dispatch("/pet", &cx()),
        open(Target::Pets, SlashCommand::Pets, "")
    );
}

#[test]
fn slash_archive_and_delete_open_the_confirmation_dialog() {
    assert_eq!(
        dispatch("/archive", &cx()),
        open(Target::Select, SlashCommand::Archive, "")
    );
    assert_eq!(
        dispatch("/delete", &cx()),
        open(Target::Select, SlashCommand::Delete, "")
    );
}

#[test]
fn slash_pwd_and_cwd_alias_display_current_working_directory() {
    for line in ["/pwd", "/cwd"] {
        assert_eq!(
            dispatch(line, &cx()),
            session(SlashCommand::Pwd, SessionIntent::Pwd),
            "{line}"
        );
    }
    assert_eq!(
        dispatch("/pwd now", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Pwd))
    );
}

#[test]
fn slash_clear_requests_ui_clear_when_idle() {
    assert_eq!(
        dispatch("/clear", &cx()),
        session(SlashCommand::Clear, SessionIntent::Clear { name: None })
    );
}

#[test]
fn slash_new_and_clear_with_name_request_a_named_session() {
    assert_eq!(
        dispatch("/new  Add User ", &cx()),
        session(
            SlashCommand::New,
            SessionIntent::New {
                name: Some("Add User".to_string())
            }
        )
    );
    assert_eq!(
        dispatch("/clear Add User", &cx()),
        session(
            SlashCommand::Clear,
            SessionIntent::Clear {
                name: Some("Add User".to_string())
            }
        )
    );
}

#[test]
fn slash_fork_requests_current_fork_and_named_fork() {
    assert_eq!(
        dispatch("/fork", &cx()),
        session(SlashCommand::Fork, SessionIntent::Fork { name: None })
    );
    assert_eq!(
        dispatch("/fork   Add User  ", &cx()),
        session(
            SlashCommand::Fork,
            SessionIntent::Fork {
                name: Some("Add User".to_string())
            }
        )
    );
}

#[test]
fn slash_clear_is_disabled_while_task_running() {
    let busy = DispatchCx {
        task_running: true,
        ..cx()
    };
    assert_eq!(
        dispatch("/clear", &busy),
        Outcome::Notice(Notice::DisabledDuringTask(SlashCommand::Clear))
    );
    // Commands that may run during a task still do.
    assert_eq!(
        dispatch("/status", &busy),
        open(Target::Status, SlashCommand::Status, "")
    );
}

#[test]
fn slash_stop_submits_background_terminal_cleanup() {
    assert_eq!(
        dispatch("/stop", &cx()),
        session(SlashCommand::Stop, SessionIntent::Stop)
    );
    assert_eq!(
        dispatch("/clean", &cx()),
        session(SlashCommand::Stop, SessionIntent::Stop)
    );
}

#[test]
fn slash_rollout_displays_current_path() {
    assert_eq!(
        dispatch("/rollout", &cx()),
        session(SlashCommand::Rollout, SessionIntent::Rollout)
    );
}

#[test]
fn slash_app_without_a_thread_shows_the_starting_error() {
    let starting = DispatchCx {
        thread_started: false,
        ..cx()
    };
    assert_eq!(
        dispatch("/app", &starting),
        Outcome::Notice(Notice::SessionStarting(SlashCommand::App))
    );
    assert_eq!(
        dispatch("/app", &cx()),
        session(SlashCommand::App, SessionIntent::App)
    );
}

#[test]
fn slash_memory_maintenance_reports_stubbed_feature() {
    for line in ["/debug-m-drop", "/debug-m-update"] {
        assert!(matches!(
            dispatch(line, &cx()),
            Outcome::Session {
                intent: SessionIntent::MemoryMaintenance,
                ..
            }
        ));
    }
}

#[test]
fn signed_out_usage_command_reports_chatgpt_login_requirement() {
    let signed_out = DispatchCx {
        has_backend_auth: false,
        ..cx()
    };
    for line in ["/usage", "/usage weekly"] {
        assert_eq!(
            dispatch(line, &signed_out),
            Outcome::Notice(Notice::UsageNeedsLogin),
            "{line}"
        );
    }
}

#[test]
fn usage_command_with_invalid_view_reports_usage_snapshot() {
    assert_eq!(
        dispatch("/usage monthly", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Usage))
    );
}

#[test]
fn usage_views_open_the_requested_summary_mode() {
    for (arg, view) in [
        ("daily", "daily"),
        ("weekly", "weekly"),
        ("cumulative", "cumulative"),
    ] {
        assert_eq!(
            dispatch(&format!("/usage {arg}"), &cx()),
            open(Target::Usage, SlashCommand::Usage, view)
        );
    }
    assert_eq!(
        dispatch("/usage", &cx()),
        open(Target::Usage, SlashCommand::Usage, "")
    );
}

#[test]
fn slash_mcp_requests_inventory_and_verbose_and_login() {
    assert_eq!(
        dispatch("/mcp", &cx()),
        session(SlashCommand::Mcp, SessionIntent::Mcp { args: None })
    );
    assert_eq!(
        dispatch("/mcp verbose", &cx()),
        session(
            SlashCommand::Mcp,
            SessionIntent::Mcp {
                args: Some("verbose".to_string())
            }
        )
    );
    assert_eq!(
        dispatch("/mcp login github", &cx()),
        session(
            SlashCommand::Mcp,
            SessionIntent::Mcp {
                args: Some("login github".to_string())
            }
        )
    );
}

#[test]
fn slash_mcp_invalid_args_show_usage() {
    assert_eq!(
        dispatch("/mcp full", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Mcp))
    );
    assert_eq!(
        dispatch("/mcp login", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Mcp))
    );
}

#[test]
fn slash_keymap_opens_the_picker_debug_opens_the_inspector_invalid_shows_usage() {
    assert_eq!(
        dispatch("/keymap", &cx()),
        open(Target::Settings, SlashCommand::Keymap, "")
    );
    assert_eq!(
        dispatch("/keymap debug", &cx()),
        open(Target::Settings, SlashCommand::Keymap, "debug")
    );
    assert_eq!(
        dispatch("/keymap nope", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Keymap))
    );
}

#[test]
fn raw_slash_command_toggles_and_accepts_on_off_args() {
    assert_eq!(
        dispatch("/raw", &cx()),
        session(SlashCommand::Raw, SessionIntent::Raw { on: None })
    );
    assert_eq!(
        dispatch("/raw on", &cx()),
        session(SlashCommand::Raw, SessionIntent::Raw { on: Some(true) })
    );
    assert_eq!(
        dispatch("/raw OFF", &cx()),
        session(SlashCommand::Raw, SessionIntent::Raw { on: Some(false) })
    );
}

#[test]
fn raw_slash_command_reports_usage_for_invalid_arg() {
    assert_eq!(
        dispatch("/raw maybe", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Raw))
    );
}

#[test]
fn goal_slash_command_with_extra_os_emits_set_goal_event() {
    assert_eq!(
        dispatch("/gooooal ship it", &cx()),
        session(
            SlashCommand::Goal,
            SessionIntent::Goal {
                args: Some("ship it".to_string())
            }
        )
    );
}

#[test]
fn voice_arguments() {
    assert_eq!(
        dispatch("/voice", &cx()),
        session(
            SlashCommand::Voice,
            SessionIntent::VoiceControl(VoiceControl::Toggle)
        )
    );
    assert_eq!(
        dispatch("/voice Mute", &cx()),
        session(
            SlashCommand::Voice,
            SessionIntent::VoiceControl(VoiceControl::Mute)
        )
    );
    assert_eq!(
        dispatch("/voice settings", &cx()),
        open(Target::Voice, SlashCommand::Voice, "settings")
    );
    assert_eq!(
        dispatch("/voice loud", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Voice))
    );
}

#[test]
fn side_conversation_and_review_gates() {
    let side = DispatchCx {
        side_conversation: true,
        flags: CommandFlags {
            side_conversation_active: true,
            ..cx().flags
        },
        ..cx()
    };
    assert_eq!(
        dispatch("/review", &side),
        Outcome::Notice(Notice::UnavailableInSide(SlashCommand::Review))
    );
    assert_eq!(
        dispatch("/status", &side),
        open(Target::Status, SlashCommand::Status, "")
    );
    let review = DispatchCx {
        review_mode: true,
        ..cx()
    };
    assert_eq!(
        dispatch("/side", &review),
        Outcome::Notice(Notice::UnavailableInReview(SlashCommand::Side))
    );
}

#[test]
fn mention_inserts_an_at_sign() {
    assert_eq!(dispatch("/mention", &cx()), Outcome::Insert("@"));
}

#[test]
fn every_screen_opening_command_names_its_target() {
    // The trigger table of the parity pack, as data: command -> screen dir.
    let table = [
        ("/model", Target::Select),
        ("/permissions", Target::Select),
        ("/review", Target::Select),
        ("/skills", Target::Settings),
        ("/config", Target::Settings),
        ("/theme", Target::Settings),
        ("/title", Target::Settings),
        ("/statusline", Target::Settings),
        ("/experimental", Target::Settings),
        ("/agents", Target::Agents),
        ("/subagents", Target::Agents),
        ("/diff", Target::Pager),
        ("/status", Target::Status),
        ("/usage", Target::Usage),
        ("/warnings", Target::Dialogs),
        ("/feedback", Target::Dialogs),
        ("/rename", Target::Dialogs),
        ("/test-approval", Target::Approvals),
    ];
    for (line, target) in table {
        match dispatch(line, &cx()) {
            Outcome::Open(open) => assert_eq!(open.target, target, "{line}"),
            other => panic!("{line}: {other:?}"),
        }
    }
    for target in [
        Target::Resume,
        Target::Agents,
        Target::Import,
        Target::Usage,
        Target::Pager,
        Target::Status,
        Target::Select,
        Target::Settings,
        Target::Dialogs,
        Target::Pets,
        Target::Voice,
        Target::Approvals,
        Target::Backtrack,
    ] {
        assert!(
            codex_tui_rtwk_core::screens::dirs()
                .iter()
                .any(|d| d.name == target.dir()),
            "{target:?}"
        );
    }
}

#[test]
fn a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop() {
    for line in [
        "/compact",
        "/init",
        "/recap",
        "/plan fix it",
        "/side",
        "/vim",
    ] {
        assert!(
            matches!(dispatch(line, &cx()), Outcome::Session { .. }),
            "{line}"
        );
    }
}

// ------------------------------------------------------- chat-nav-fix (G1)

/// `slash_dispatch.rs:163-165`: `Resume | Cd` while a turn runs or is about to
/// start, with or without arguments.
#[test]
fn slash_resume_and_cd_are_disabled_while_a_task_runs() {
    let busy = DispatchCx {
        task_running: true,
        ..cx()
    };
    for (line, command) in [
        ("/resume", SlashCommand::Resume),
        ("/resume my chat", SlashCommand::Resume),
        ("/cd", SlashCommand::Cd),
        ("/cd /tmp", SlashCommand::Cd),
    ] {
        assert_eq!(
            dispatch(line, &busy),
            Outcome::Notice(Notice::DisabledDuringTask(command)),
            "{line}"
        );
    }
    // Idle, the picker opens.
    assert_eq!(
        dispatch("/resume", &cx()),
        open(Target::Resume, SlashCommand::Resume, "")
    );
}

/// `analytics/activity_chart_tests.rs`: `token_activity_view_aliases_parse`.
#[test]
fn token_activity_view_aliases_parse() {
    assert_eq!(TokenActivityView::parse(""), Some(TokenActivityView::Daily));
    assert_eq!(
        TokenActivityView::parse("day"),
        Some(TokenActivityView::Daily)
    );
    assert_eq!(
        TokenActivityView::parse("week"),
        Some(TokenActivityView::Weekly)
    );
    assert_eq!(
        TokenActivityView::parse("cumulative"),
        Some(TokenActivityView::Cumulative)
    );
    assert_eq!(TokenActivityView::parse("year"), None);
    // `parse` trims and lowercases (`activity_chart.rs:37-43`).
    assert_eq!(
        TokenActivityView::parse("  Weekly "),
        Some(TokenActivityView::Weekly)
    );
    assert_eq!(
        TokenActivityView::parse("DAILY"),
        Some(TokenActivityView::Daily)
    );
}

#[test]
fn usage_accepts_the_oracles_aliases() {
    for (line, view) in [
        ("/usage week", "weekly"),
        ("/usage Weekly", "weekly"),
        ("/usage WEEK", "weekly"),
        ("/usage day", "daily"),
        ("/usage Daily", "daily"),
        ("/usage cumulative", "cumulative"),
    ] {
        assert_eq!(
            dispatch(line, &cx()),
            open(Target::Usage, SlashCommand::Usage, view),
            "{line}"
        );
    }
    assert_eq!(
        dispatch("/usage year", &cx()),
        Outcome::Notice(Notice::Usage(UsageKey::Usage))
    );
}

/// `model_popups.rs:19-24`: `/model` before the thread exists.
#[test]
fn slash_model_before_startup_shows_the_oracles_info_line() {
    let starting = DispatchCx {
        thread_started: false,
        ..cx()
    };
    assert_eq!(
        dispatch("/model", &starting),
        Outcome::Notice(Notice::ModelStartup)
    );
    assert_eq!(
        dispatch("/model", &cx()),
        open(Target::Select, SlashCommand::Model, "")
    );
}

/// `slash_dispatch.rs:557-559`: a bare `/cd` dispatches `/cd ~`.
#[test]
fn a_bare_cd_goes_home() {
    assert_eq!(
        dispatch("/cd", &cx()),
        session(
            SlashCommand::Cd,
            SessionIntent::Cd {
                path: "~".to_string()
            }
        )
    );
}

/// `add_info_message` vs `add_error_message`: `/model` before startup is the
/// only info line among the notices.
#[test]
fn only_the_model_startup_line_is_an_info_line() {
    assert!(Notice::ModelStartup.is_info());
    assert!(!Notice::UsageNeedsLogin.is_info());
    assert!(!Notice::Unrecognized("x".into()).is_info());
    assert!(!Notice::DisabledDuringTask(SlashCommand::Clear).is_info());
    assert!(!Notice::Usage(UsageKey::Pwd).is_info());
}
