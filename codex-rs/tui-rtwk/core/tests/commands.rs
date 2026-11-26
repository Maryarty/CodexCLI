//! The slash commands: the registry, the lookup, the popup's filter and the
//! dispatch decision, ported from codex-tui's tests. Expected values are the
//! oracle's (`slash_command.rs`, `bottom_pane/slash_commands.rs`,
//! `bottom_pane/command_popup.rs`, `utils/fuzzy-match`,
//! `chatwidget/tests/slash_commands.rs`); two tests read the oracle's source
//! itself (the enum order, the command strings).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::commands::*;
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

/// The command the oracle's popup selected on the first row after the text changed: the first
/// row that is not unavailable (the selection itself is the list widget's).
fn first_available(popup: &CommandPopup) -> Option<CommandItem> {
    popup
        .rows()
        .into_iter()
        .find(|row| !row.unavailable)
        .map(|row| row.item)
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
    assert_eq!(first_available(&popup), None);
    assert!(popup.rows()[0].unavailable);
    popup.on_composer_text_change("/a".to_string());
    assert!(first_available(&popup).is_some());
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
        first_available(&popup),
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
    assert_eq!(
        first_available(&popup),
        Some(CommandItem::ServiceTier(tier))
    );
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
        first_available(&popup),
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
        session(
            SlashCommand::Logout,
            SessionIntent::Feature {
                args: String::new()
            }
        )
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
            SessionIntent::Feature {
                args: "my chat".to_string()
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
        session(SlashCommand::Status, SessionIntent::Status)
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
    // a bare `/usage` with a login opens the usage menu (`features::usage_menu`)
    assert_eq!(
        dispatch("/usage", &cx()),
        open(
            Target::Feature(codex_tui_core::features::target_of(SlashCommand::Usage).unwrap()),
            SlashCommand::Usage,
            ""
        )
    );
}

#[test]
fn slash_mcp_requests_inventory_and_verbose_and_login() {
    assert_eq!(
        dispatch("/mcp", &cx()),
        open(
            Target::Feature(codex_tui_core::features::target_of(SlashCommand::Mcp).unwrap()),
            SlashCommand::Mcp,
            ""
        )
    );
    assert_eq!(
        dispatch("/mcp verbose", &cx()),
        session(
            SlashCommand::Mcp,
            SessionIntent::Feature {
                args: "verbose".to_string()
            }
        )
    );
    assert_eq!(
        dispatch("/mcp login github", &cx()),
        Outcome::Notice(Notice::NotAvailable(SlashCommand::Mcp))
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
            SessionIntent::Feature {
                args: "ship it".to_string()
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
        session(SlashCommand::Status, SessionIntent::Status)
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
    // The screens still routed by the session. Feature-owned routes come from their registry.
    for (line, target) in [
        ("/permissions", Target::Select),
        ("/review", Target::Select),
        ("/agents", Target::Agents),
        ("/subagents", Target::AgentPicker),
        ("/warnings", Target::Dialogs),
        ("/test-approval", Target::Approvals),
    ] {
        match dispatch(line, &cx()) {
            Outcome::Open(open) => assert_eq!(open.target, target, "{line}"),
            other => panic!("{line}: {other:?}"),
        }
    }
    for (command, target) in feature_openings() {
        assert_eq!(
            dispatch_command(command, "", &cx()),
            open(target, command, ""),
            "/{}",
            command.command(),
        );
    }
    // `Target::Pager` has no screen directory: the pager is the expanded mode of the
    // session transcript (chat-live), so the target opens nothing for now. `Target::Settings`
    // has none either since settings became a feature slice (`features/settings`, ADR-012).
    for target in [
        Target::Resume,
        Target::Agents,
        Target::Import,
        Target::Usage,
        Target::Select,
        Target::Dialogs,
        Target::Pets,
        Target::Voice,
        Target::Approvals,
        Target::Backtrack,
        Target::AgentPicker,
    ] {
        assert!(
            codex_tui_core::screens::dirs()
                .iter()
                .any(|d| d.name == target.dir()),
            "{target:?}"
        );
    }
}

#[test]
fn queued_feature_commands_keep_their_checks_and_argument_routes() {
    let signed_out = DispatchCx {
        has_backend_auth: false,
        ..cx()
    };
    let busy = DispatchCx {
        task_running: true,
        ..cx()
    };
    let review = DispatchCx {
        review_mode: true,
        ..cx()
    };
    let startup = DispatchCx {
        thread_started: false,
        ..cx()
    };
    for (line, cx, command, outcome) in [
        (
            "/usage invalid",
            signed_out,
            SlashCommand::Usage,
            Outcome::Notice(Notice::UsageNeedsLogin),
        ),
        (
            "/usage WEEK",
            cx(),
            SlashCommand::Usage,
            open(Target::Usage, SlashCommand::Usage, "weekly"),
        ),
        (
            "/resume my chat",
            busy,
            SlashCommand::Resume,
            Outcome::Notice(Notice::DisabledDuringTask(SlashCommand::Resume)),
        ),
        (
            "/resume my chat",
            review,
            SlashCommand::Resume,
            session(
                SlashCommand::Resume,
                SessionIntent::Feature {
                    args: "my chat".to_owned(),
                },
            ),
        ),
        (
            "/mcp LOGIN github",
            startup,
            SlashCommand::Mcp,
            Outcome::Notice(Notice::McpLoginNeedsSession),
        ),
        (
            "/keymap DEBUG",
            cx(),
            SlashCommand::Keymap,
            open(Target::Settings, SlashCommand::Keymap, "debug"),
        ),
    ] {
        assert_eq!(
            dispatch_queued(line, &cx, &[]),
            QueuedDispatch {
                outcome,
                command: Some(command),
            },
            "{line}",
        );
    }
    assert_eq!(
        dispatch_queued("/recap extra text", &startup, &[]),
        QueuedDispatch {
            outcome: Outcome::NotACommand,
            command: None,
        },
    );
}

#[test]
fn shared_gates_still_precede_slice_checks_and_bare_checks_leave_inline_args_alone() {
    let side_and_review = DispatchCx {
        side_conversation: true,
        review_mode: true,
        ..cx()
    };
    assert_eq!(
        dispatch_command(SlashCommand::Btw, "a side prompt", &side_and_review),
        Outcome::Notice(Notice::UnavailableInSide(SlashCommand::Btw)),
    );
    let busy_startup = DispatchCx {
        thread_started: false,
        task_running: true,
        ..cx()
    };
    assert_eq!(
        dispatch("/recap", &busy_startup),
        Outcome::Notice(Notice::DisabledDuringTask(SlashCommand::Recap)),
    );
    assert_eq!(
        dispatch("/model gpt-5", &busy_startup),
        session(
            SlashCommand::Model,
            SessionIntent::Feature {
                args: "gpt-5".to_owned(),
            },
        ),
    );
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

/// `slash_command.rs`, `available_during_task`: the oracle's exhaustive
/// match is read from its source, so a command the oracle adds (or moves)
/// fails here instead of defaulting to "available".
#[test]
fn available_during_task_equals_the_oracles_arms() {
    let text = oracle("slash_command.rs");
    let body = text
        .split("pub fn available_during_task(self)")
        .nth(1)
        .and_then(|rest| rest.split("fn is_visible").next())
        .expect("the function");
    let mut expected = std::collections::BTreeMap::new();
    let mut names: Vec<String> = Vec::new();
    for line in body.lines().map(str::trim) {
        let (arms, verdict) = match line.split_once("=>") {
            Some((arms, verdict)) => (arms, Some(verdict.trim().trim_end_matches(',') == "true")),
            None => (line, None),
        };
        names.extend(
            arms.split('|')
                .filter_map(|a| a.trim().strip_prefix("SlashCommand::"))
                .map(str::to_string),
        );
        if let Some(verdict) = verdict {
            for name in names.drain(..) {
                expected.insert(name, verdict);
            }
        }
    }
    assert_eq!(expected.len(), SlashCommand::ALL.len(), "{expected:?}");
    for command in SlashCommand::ALL {
        let name = format!("{command:?}");
        assert_eq!(
            Some(&command.available_during_task()),
            expected.get(&name),
            "{name}"
        );
    }
}

/// Architecture §1 (Amendment 2026-11-19b): only Startup, Session, Agents,
/// Resume and Usage are routes; the rest of the targets are modal sheets over
/// the page that opens them, or a mode of the session page.
#[test]
fn only_the_five_pages_are_routes() {
    let pages: Vec<_> = [
        Target::Resume,
        Target::Agents,
        Target::Import,
        Target::Usage,
    ]
    .into_iter()
    .collect();
    for target in &pages {
        assert_eq!(target.opens(), Opens::Page, "{target:?}");
    }
    for target in [
        Target::Select,
        Target::Settings,
        Target::Approvals,
        Target::Pets,
        Target::Voice,
        Target::AgentPicker,
    ] {
        assert_eq!(target.opens(), Opens::Sheet, "{target:?}");
    }
    assert_eq!(Target::Dialogs.opens(), Opens::Dialog);
    for target in [Target::Pager, Target::Backtrack] {
        assert_eq!(target.opens(), Opens::Mode, "{target:?}");
    }
}

// ---------------------------------------------------------------- notes
// What a command line answered with is an entry of the thread's transcript
// (oracle `add_info_message` / `add_error_message`, `new_info_event` /
// `new_error_event`), through `apply` (ADR-003), not a line of the screen.

mod notes {
    use std::rc::Rc;

    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::ids::{ThreadId, Ticket};
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::{ScreenIntent, chat};
    use codex_tui_core::threads::{Item, Note, NoteKey, ThreadModel, Tone};
    use pretty_assertions::assert_eq;

    use super::*;

    pub(super) const T: &str = "thread-1";

    pub(super) fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    /// The app with its thread started, as after `Connected` and the reply.
    pub(super) fn ready() -> CodexApp {
        let mut app = CodexApp::new();
        let fx = run(&mut app, AppEv::Connected);
        let ticket: Ticket = fx
            .commands()
            .iter()
            .find_map(|c| match c {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread");
        run(
            &mut app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
        );
        app
    }

    fn say(app: &mut CodexApp, thread: Option<&str>, notice: Notice) {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Note {
                thread: thread.map(ThreadId::new),
                note: Note::from(notice),
            }))),
        );
    }

    pub(super) fn notes_of(thread: &Rc<ThreadModel>) -> Vec<Note> {
        thread
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Note(note) => Some(note.clone()),
                _ => None,
            })
            .collect()
    }

    /// `add_info_message` for the two answers the oracle shows as
    /// information, `add_error_message` for every other.
    #[test]
    fn the_oracle_shows_two_answers_as_information() {
        for (notice, tone) in [
            (Notice::ModelStartup, Tone::Info),
            (Notice::ShellHelp, Tone::Info),
            (Notice::Unrecognized("x".into()), Tone::Error),
            (Notice::Usage(UsageKey::Raw), Tone::Error),
            (Notice::UsageNeedsLogin, Tone::Error),
            (Notice::DisabledDuringTask(SlashCommand::New), Tone::Error),
        ] {
            assert_eq!(Note::from(notice.clone()).tone, tone, "{notice:?}");
        }
    }

    #[test]
    fn a_command_answer_becomes_an_entry_of_the_thread_it_was_submitted_in() {
        let mut app = ready();
        say(&mut app, Some(T), Notice::Usage(UsageKey::Raw));
        say(&mut app, Some(T), Notice::ShellHelp);
        assert_eq!(
            notes_of(&app.active_thread()),
            [
                Note::from(NoteKey::Command(Notice::Usage(UsageKey::Raw))),
                Note::from(NoteKey::Command(Notice::ShellHelp)),
            ]
        );
    }

    /// With no thread yet the answer is the screen's (the oracle has the
    /// history above the live area for it; ours has no transcript to put it in).
    #[test]
    fn an_answer_with_no_thread_is_a_note_of_the_chat_screen() {
        let mut app = CodexApp::new();
        say(&mut app, None, Notice::ModelStartup);
        assert_eq!(
            app.screens().chat.notes,
            [Note::from(NoteKey::Command(Notice::ModelStartup))]
        );
        // a thread that does not exist is no thread
        say(&mut app, Some("thread-9"), Notice::ShellHelp);
        assert_eq!(app.screens().chat.notes.len(), 2);
    }

    /// `chat_composer.rs` 3432-3524: the composer keeps the line of an
    /// unrecognized command (not recalled), a command refused while a task
    /// runs, and a command that is validated by dispatch (`/fork`, `/review`);
    /// every other answer has taken the line, and recall holds it.
    #[test]
    fn the_composer_keeps_the_line_of_a_refused_command_as_the_oracle_does() {
        let keeps = |n: Notice| (n.keeps_draft(), n.is_recalled());
        assert_eq!(keeps(Notice::Unrecognized("x".into())), (true, false));
        assert_eq!(keeps(Notice::InputTooLarge { provided: 1 }), (true, false));
        assert_eq!(
            keeps(Notice::DisabledDuringTask(SlashCommand::New)),
            (true, true)
        );
        assert_eq!(
            keeps(Notice::DisabledDuringTask(SlashCommand::Review)),
            (true, true)
        );
        assert_eq!(
            keeps(Notice::DisabledDuringTask(SlashCommand::Cd)),
            (true, true)
        );
        // `/resume` is let through by the composer and blocked by dispatch,
        // after the composer cleared its text
        assert_eq!(
            keeps(Notice::DisabledDuringTask(SlashCommand::Resume)),
            (false, true)
        );
        assert_eq!(keeps(Notice::Usage(UsageKey::Raw)), (false, true));
        assert_eq!(keeps(Notice::UsageNeedsLogin), (false, true));
        assert_eq!(
            keeps(Notice::UnavailableInSide(SlashCommand::Copy)),
            (false, true)
        );
        assert_eq!(
            keeps(Notice::UnavailableInSide(SlashCommand::Review)),
            (true, true)
        );
    }
}

/// What the commands that run at once do (oracle `slash_dispatch.rs`
/// `dispatch_command`): their effect on the thread, and the answer they leave
/// in its transcript.
mod effects {
    use codex_app_server_protocol::{RequestId, ServerNotification};
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::screens::{ScreenCommand, ScreenIntent, chat};
    use codex_tui_core::threads::{Note, NoteKey, Stub, Toggle, Tone, UserMessage};
    use pretty_assertions::assert_eq;

    use super::notes::{T, notes_of, ready, run};
    use super::*;

    /// `thread/started` of the thread, as the server describes it.
    pub(super) fn started(cwd: &str, path: Option<&str>, can_accept: Option<bool>) -> AppEv {
        started_with(cwd, path, can_accept, false)
    }

    pub(super) fn started_with(
        cwd: &str,
        path: Option<&str>,
        can_accept: Option<bool>,
        ephemeral: bool,
    ) -> AppEv {
        let notification: ServerNotification = serde_json::from_value(serde_json::json!({
            "method": "thread/started",
            "params": {"thread": {
                "id": T, "environments": null, "extra": null, "sessionId": T,
                "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
                "preview": "", "ephemeral": ephemeral, "section": null, "sectionEnteredAt": null,
                "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
                "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
                "recencyAt": 0, "status": {"type": "idle"}, "path": path, "cwd": cwd,
                "cliVersion": "0.0.0", "originator": null, "source": "cli",
                "canAcceptDirectInput": can_accept, "threadSource": null,
                "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
                "daybreakEnabled": null, "turns": []
            }}
        }))
        .expect("thread/started");
        AppEv::Backend(BackendEv::Notification(Box::new(notification)))
    }

    fn command(app: &mut CodexApp, command: SlashCommand, intent: SessionIntent) -> AppFx {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
                thread: Some(ThreadId::new(T)),
                command,
                intent,
            }))),
        )
    }

    fn notes(app: &CodexApp) -> Vec<Note> {
        notes_of(&app.active_thread())
    }

    #[test]
    fn each_answer_has_the_tone_the_oracle_gives_it() {
        for (key, tone) in [
            (NoteKey::WorkingDirectory("/tmp".into()), Tone::Info),
            (NoteKey::Rollout(None), Tone::Info),
            (NoteKey::Rollout(Some("/tmp/r.jsonl".into())), Tone::Info),
            (NoteKey::DesktopOpened, Tone::Info),
            (NoteKey::StoppingTerminals, Tone::Info),
            (NoteKey::Toggled(Toggle::RawOutput, true), Tone::Info),
            (NoteKey::Stub(Stub::MemoryMaintenance), Tone::Error),
            (NoteKey::ParentOwned, Tone::Error),
        ] {
            assert_eq!(Note::from(key.clone()).tone, tone, "{key:?}");
        }
    }

    /// `clean_background_terminals`: the op, then the info line.
    #[test]
    fn stop_asks_the_thread_to_clean_its_terminals_and_says_so() {
        let mut app = ready();
        let fx = command(&mut app, SlashCommand::Stop, SessionIntent::Stop);
        assert!(
            fx.commands().iter().any(|c| matches!(
                c,
                Command::Screen {
                    thread,
                    command: ScreenCommand::Chat(chat::Command::CleanTerminals),
                    ..
                } if thread.as_str() == T
            )),
            "{:?}",
            fx.commands()
        );
        assert_eq!(notes(&app), [Note::from(NoteKey::StoppingTerminals)]);
    }

    /// The request is the app-server's `thread/backgroundTerminals/clean`.
    #[test]
    fn the_cleanup_is_the_servers_background_terminals_request() {
        let request = chat::wire(
            &chat::Command::CleanTerminals,
            &ThreadId::new(T),
            RequestId::Integer(7),
        );
        let json = serde_json::to_value(&request).expect("json");
        assert_eq!(json["method"], "thread/backgroundTerminals/clean");
        assert_eq!(json["params"]["threadId"], T);
    }

    /// `/review <text>` reviews the text as instructions; the popup's pick
    /// reviews its target. Both are `review/start`, inline on the thread.
    #[test]
    fn review_with_text_and_the_popups_pick_start_an_inline_review() {
        use codex_tui_core::threads::ReviewTarget;
        let mut app = ready();
        let fx = command(
            &mut app,
            SlashCommand::Review,
            SessionIntent::Review {
                args: Some("  the auth module ".into()),
            },
        );
        let sent: Vec<_> = fx
            .commands()
            .iter()
            .filter_map(|c| match c {
                Command::Screen {
                    thread,
                    command: ScreenCommand::Chat(chat::Command::Review(target)),
                    ..
                } if thread.as_str() == T => Some(target.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            sent,
            [ReviewTarget::Custom {
                instructions: "the auth module".into()
            }],
            "{:?}",
            fx.commands()
        );
        let target = ReviewTarget::BaseBranch {
            branch: "main".into(),
        };
        let fx = command(
            &mut app,
            SlashCommand::Review,
            SessionIntent::StartReview {
                target: target.clone(),
            },
        );
        assert!(
            fx.commands().iter().any(|c| matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Chat(chat::Command::Review(t)),
                    ..
                } if *t == target
            )),
            "{:?}",
            fx.commands()
        );
        let request = chat::wire(
            &chat::Command::Review(target),
            &ThreadId::new(T),
            RequestId::Integer(9),
        );
        let json = serde_json::to_value(&request).expect("json");
        assert_eq!(json["method"], "review/start");
        assert_eq!(json["params"]["threadId"], T);
        assert_eq!(json["params"]["target"]["type"], "baseBranch");
        assert_eq!(json["params"]["delivery"], "inline");
    }

    #[test]
    fn memory_maintenance_is_a_stub_error() {
        let mut app = ready();
        command(
            &mut app,
            SlashCommand::MemoryDrop,
            SessionIntent::MemoryMaintenance,
        );
        assert_eq!(
            notes(&app),
            [Note::from(NoteKey::Stub(Stub::MemoryMaintenance))]
        );
    }

    #[test]
    fn pwd_says_the_directory_the_thread_started_in() {
        let mut app = ready();
        run(&mut app, started("/tmp/project", None, Some(true)));
        command(&mut app, SlashCommand::Pwd, SessionIntent::Pwd);
        assert_eq!(
            notes(&app),
            [Note::from(NoteKey::WorkingDirectory("/tmp/project".into()))]
        );
    }

    #[test]
    fn rollout_says_the_path_or_that_the_server_has_not_named_it() {
        let mut app = ready();
        command(&mut app, SlashCommand::Rollout, SessionIntent::Rollout);
        run(
            &mut app,
            started("/tmp", Some("/tmp/codex-test-rollout.jsonl"), Some(true)),
        );
        command(&mut app, SlashCommand::Rollout, SessionIntent::Rollout);
        assert_eq!(
            notes(&app),
            [
                Note::from(NoteKey::Rollout(None)),
                Note::from(NoteKey::Rollout(Some(
                    "/tmp/codex-test-rollout.jsonl".into()
                ))),
            ]
        );
    }

    #[test]
    fn app_hands_the_thread_to_the_desktop_app_by_its_link() {
        let mut app = ready();
        let fx = command(&mut app, SlashCommand::App, SessionIntent::App);
        assert!(
            fx.commands().iter().any(|c| matches!(
                c,
                Command::OpenUrl { url, .. } if url == "codex://threads/thread-1"
            )),
            "{:?}",
            fx.commands()
        );
        assert_eq!(notes(&app), [Note::from(NoteKey::DesktopOpened)]);
    }

    #[test]
    fn raw_toggles_the_mode_and_says_which_it_is_now() {
        let mut app = ready();
        let raw = |app: &CodexApp| app.screens().chat.raw_output;
        command(&mut app, SlashCommand::Raw, SessionIntent::Raw { on: None });
        assert!(raw(&app));
        command(&mut app, SlashCommand::Raw, SessionIntent::Raw { on: None });
        assert!(!raw(&app));
        command(
            &mut app,
            SlashCommand::Raw,
            SessionIntent::Raw { on: Some(false) },
        );
        assert!(!raw(&app));
        command(
            &mut app,
            SlashCommand::Raw,
            SessionIntent::Raw { on: Some(true) },
        );
        assert!(raw(&app));
        assert_eq!(
            notes(&app),
            [true, false, false, true]
                .map(|on| Note::from(NoteKey::Toggled(Toggle::RawOutput, on)))
        );
    }

    /// `/init` sends the bundled prompt as a message of the thread.
    #[test]
    fn init_starts_a_turn_with_the_bundled_prompt() {
        let mut app = ready();
        let fx = command(&mut app, SlashCommand::Init, SessionIntent::Init);
        let sent: Vec<UserMessage> = fx
            .commands()
            .iter()
            .filter_map(|c| match c {
                Command::StartTurn { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect();
        let prompt = include_str!("../assets/prompt_for_init_command.md");
        assert_eq!(sent.len(), 1, "{:?}", fx.commands());
        assert_eq!(sent[0].text, prompt.trim());
        assert_eq!(
            prompt,
            include_str!("../../../tui/assets/prompt_for_init_command.md")
        );
    }

    /// `submit_op`: a thread its parent controls takes no turn and no
    /// compaction; it says so and starts nothing.
    #[test]
    fn a_parent_owned_thread_starts_no_turn_and_no_compaction() {
        let mut app = ready();
        run(&mut app, started("/tmp", None, Some(false)));
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Submit {
                thread: Some(ThreadId::new(T)),
                message: UserMessage::text("hello"),
            }),
        );
        assert!(
            !fx.commands()
                .iter()
                .any(|c| matches!(c, Command::StartTurn { .. })),
            "{:?}",
            fx.commands()
        );
        let fx = command(&mut app, SlashCommand::Compact, SessionIntent::Compact);
        assert!(
            !fx.commands()
                .iter()
                .any(|c| matches!(c, Command::Screen { .. })),
            "{:?}",
            fx.commands()
        );
        assert_eq!(
            notes(&app),
            [
                Note::from(NoteKey::ParentOwned),
                Note::from(NoteKey::ParentOwned)
            ]
        );
    }

    /// The server's word wins; without it a spawned sub-agent is parent-owned.
    #[test]
    fn a_thread_that_can_take_direct_input_starts_its_turn() {
        let mut app = ready();
        run(&mut app, started("/tmp", None, Some(true)));
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Submit {
                thread: Some(ThreadId::new(T)),
                message: UserMessage::text("hello"),
            }),
        );
        assert!(
            fx.commands()
                .iter()
                .any(|c| matches!(c, Command::StartTurn { .. })),
            "{:?}",
            fx.commands()
        );
        assert_eq!(notes(&app), []);
    }

    /// `parent_owned_command_is_allowed`, `handle_parent_owned_submission`.
    #[test]
    fn a_parent_owned_thread_takes_the_commands_that_do_not_reach_the_agent() {
        let flags = CommandFlags::default();
        for (line, takes) in [
            ("/status", true),
            ("/stop", true),
            ("/rollout", true),
            ("/diff", true),
            ("/side", true),
            ("/export a b", true),
            ("/title x", true),
            ("/statusline", true),
            ("/init", false),
            ("/review check this", false),
            ("/side inspect this", false),
            ("/diff now", false),
            ("/archive", false),
            ("/rename", false),
            ("/subagents parent", false),
            ("/pwd", false),
            // an alias is not the command's own name
            ("/cwd", false),
            ("/nonsense", false),
            ("!echo blocked", false),
            (" !echo blocked", false),
            ("keep this draft", false),
        ] {
            assert_eq!(parent_owned_takes(line, flags), takes, "{line}");
        }
    }
}

/// `/cd`: `request_working_directory_change` (chatwidget/working_directory.rs)
/// and `AppEvent::ChangeWorkingDirectory` (app/event_dispatch.rs). The session
/// goes on in the directory: a fork of the conversation when it has history,
/// a fresh thread when it has none.
mod directory {
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent};
    use codex_tui_core::commands::resolve_directory;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::{
        ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, ScreenThreadIntent, agents, chat,
    };
    use codex_tui_core::threads::{CdRefusal, Note, NoteKey, ThreadAsk, UserMessage};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::effects::{started, started_with};
    use super::notes::{T, notes_of, ready, run};
    use super::*;

    fn notified(app: &mut CodexApp, notification: serde_json::Value) {
        let notification: ServerNotification =
            serde_json::from_value(notification).expect("notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        );
    }

    fn turn_started(app: &mut CodexApp) {
        notified(
            app,
            json!({"method": "turn/started", "params": {"threadId": T, "turn": {
                "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
                "error": null}}}),
        );
    }

    /// One whole turn: the agent said something and the turn ended.
    fn a_turn_with_history(app: &mut CodexApp) {
        turn_started(app);
        notified(
            app,
            json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
                "completedAtMs": 0, "startedAtMs": 0,
                "item": {"type": "agentMessage", "id": "a1", "text": "done", "phase": null}}}),
        );
        notified(
            app,
            json!({"method": "turn/completed", "params": {"threadId": T, "turn": {
                "id": "turn-1", "items": [], "itemsView": "notLoaded", "status": "completed",
                "error": null}}}),
        );
    }

    fn cd(app: &mut CodexApp, thread: Option<&str>, path: &str) -> AppFx {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
                thread: thread.map(ThreadId::new),
                command: SlashCommand::Cd,
                intent: SessionIntent::Cd {
                    path: path.to_string(),
                },
            }))),
        )
    }

    fn asks(fx: &AppFx) -> Vec<ThreadAsk> {
        fx.commands()
            .iter()
            .filter_map(|c| match c {
                Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                    ..
                } => Some(ask.clone()),
                _ => None,
            })
            .collect()
    }

    fn refusal(why: CdRefusal) -> Vec<Note> {
        vec![Note::from(NoteKey::CdRefused(why))]
    }

    fn queue(app: &mut CodexApp) {
        run(
            app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new(T),
                intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(
                    UserMessage::text("q").into(),
                )),
            }),
        );
    }

    fn submit(app: &mut CodexApp) {
        run(
            app,
            AppEv::Intent(Intent::Submit {
                thread: Some(ThreadId::new(T)),
                message: UserMessage::text("s"),
            }),
        );
    }

    /// The oracle's table of `can_change_working_directory`.
    #[test]
    fn the_directory_is_not_changed_unless_the_session_is_idle_and_the_primary_one() {
        type State = (&'static str, fn(&mut CodexApp));
        let states: [State; 7] = [
            ("pending", submit),
            ("active", turn_started),
            ("queued", |app| {
                turn_started(app);
                queue(app);
            }),
            ("steer", |app| {
                turn_started(app);
                submit(app);
            }),
            ("side", |app| {
                run(
                    app,
                    AppEv::Screen(ScreenEv::Chat(chat::Ev::Flags(CommandFlags {
                        side_conversation_active: true,
                        ..CommandFlags::default()
                    }))),
                );
            }),
            ("owned", |app| {
                run(app, started("/tmp", None, Some(false)));
            }),
            ("ephemeral", |app| {
                run(app, started_with("/tmp", None, Some(true), true));
            }),
        ];
        for (state, setup) in states {
            let mut app = ready();
            setup(&mut app);
            let fx = cd(&mut app, Some(T), "/tmp");
            assert!(asks(&fx).is_empty(), "{state}: {:?}", fx.commands());
            assert!(
                !fx.commands()
                    .iter()
                    .any(|c| matches!(c, Command::StartThread { .. })),
                "{state}: {:?}",
                fx.commands()
            );
            assert_eq!(
                notes_of(&app.active_thread())
                    .into_iter()
                    .filter(|note| matches!(note.key, NoteKey::CdRefused(_)))
                    .collect::<Vec<_>>(),
                refusal(CdRefusal::NotIdle),
                "{state}"
            );
        }
    }

    #[test]
    fn cd_before_the_session_starts_says_so() {
        let mut app = CodexApp::new();
        run(&mut app, AppEv::Connected);
        let fx = cd(&mut app, None, "/tmp");
        assert!(asks(&fx).is_empty(), "{:?}", fx.commands());
        assert_eq!(
            app.screens().chat.notes,
            refusal(CdRefusal::NotStarted),
            "no thread yet: the answer waits above the composer"
        );
    }

    #[test]
    fn a_conversation_with_history_is_forked_into_the_directory() {
        let mut app = ready();
        run(
            &mut app,
            started("/tmp/project", Some("/tmp/r.jsonl"), Some(true)),
        );
        a_turn_with_history(&mut app);
        let fx = cd(&mut app, Some(T), "/tmp");
        assert_eq!(
            asks(&fx),
            [ThreadAsk::ForkAt {
                thread: T.to_string(),
                cwd: Some("/tmp".to_string()),
                temporary: None,
            }]
        );
        assert_eq!(notes_of(&app.active_thread()), []);
    }

    #[test]
    fn a_conversation_without_history_starts_afresh_in_the_directory() {
        let mut app = ready();
        run(
            &mut app,
            started("/tmp/project", Some("/tmp/r.jsonl"), Some(true)),
        );
        let fx = cd(&mut app, Some(T), "sub");
        assert_eq!(
            asks(&fx),
            [ThreadAsk::Start {
                cwd: Some("/tmp/project/sub".to_string())
            }]
        );
    }

    #[test]
    fn history_the_server_has_not_saved_cannot_be_carried_over() {
        let mut app = ready();
        run(&mut app, started("/tmp/project", None, Some(true)));
        a_turn_with_history(&mut app);
        let fx = cd(&mut app, Some(T), "/tmp");
        assert!(asks(&fx).is_empty(), "{:?}", fx.commands());
        assert_eq!(
            notes_of(&app.active_thread()),
            refusal(CdRefusal::HistoryNotSaved)
        );
    }

    /// The new conversation takes the screen, as `/new` and `/fork` do.
    #[test]
    fn the_session_goes_on_in_the_conversation_the_server_started() {
        for with_history in [true, false] {
            let mut app = ready();
            run(
                &mut app,
                started("/tmp/project", Some("/tmp/r.jsonl"), Some(true)),
            );
            if with_history {
                a_turn_with_history(&mut app);
            }
            let fx = cd(&mut app, Some(T), "/tmp");
            let ticket = fx
                .commands()
                .iter()
                .find_map(|c| match c {
                    Command::Screen { ticket, .. } => Some(*ticket),
                    _ => None,
                })
                .expect("the ask");
            let thread = serde_json::from_value(json!({
                "id": "moved-1", "environments": null, "extra": null, "sessionId": "moved-1",
                "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
                "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
                "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
                "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
                "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp",
                "cliVersion": "0.0.0", "originator": null, "source": "cli",
                "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
                "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
                "turns": []
            }))
            .expect("a thread");
            let reply = if with_history {
                agents::Reply::Forked(Box::new(thread))
            } else {
                agents::Reply::Started(Box::new(thread))
            };
            run(
                &mut app,
                AppEv::Reply(ticket, Reply::screen(ScreenReply::Agents(reply))),
            );
            assert_eq!(
                app.session().active,
                Some(ThreadId::new("moved-1")),
                "with_history={with_history}"
            );
        }
    }

    /// `AbsolutePathBuf::resolve_path_against_base`.
    #[test]
    fn the_directory_is_resolved_against_the_threads_own() {
        let home = Some("/home/me");
        for (requested, expected) in [
            ("/tmp", "/tmp"),
            ("~", "/home/me"),
            ("~/work", "/home/me/work"),
            ("sub/dir", "/tmp/project/sub/dir"),
            ("./sub", "/tmp/project/sub"),
            ("..", "/tmp"),
            ("../other", "/tmp/other"),
            ("/a/b/../c/./d", "/a/c/d"),
            ("/../..", "/"),
            ("", "/tmp/project"),
        ] {
            assert_eq!(
                resolve_directory(requested, "/tmp/project", home),
                expected,
                "{requested:?}"
            );
        }
        // without a home the tilde is an ordinary name
        assert_eq!(
            resolve_directory("~", "/tmp/project", None),
            "/tmp/project/~"
        );
    }
}

/// `/archive` and `/delete`: the confirmation the oracle shows
/// (`chatwidget/slash_dispatch.rs`) and what the answer Yes does
/// (`archive_current_thread`, `delete_current_thread` of
/// `app/event_dispatch.rs`).
mod removal {
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::agents::Action;
    use codex_tui_core::screens::select::{Confirm, Intent as Select, Opening};
    use codex_tui_core::screens::{
        ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, agents, chat,
    };
    use codex_tui_core::threads::{Note, NoteKey, RemoveNote};
    use pretty_assertions::assert_eq;

    use super::notes::{T, notes_of, ready, run};
    use super::*;

    fn select(app: &mut CodexApp, intent: Select) -> AppFx {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
        )
    }

    /// The confirmation is asked for and `row` is picked: Yes is a verb of the sheet, No closes
    /// it and says nothing (`ConfirmSheet`).
    fn pick(app: &mut CodexApp, confirm: Confirm, row: usize) -> AppFx {
        let asked = select(app, Select::Confirm(confirm));
        if row == 1 {
            select(app, Select::Confirmed(confirm))
        } else {
            asked
        }
    }

    /// The sheet the model asked for last, if it is still waiting to be shown.
    fn asked(app: &CodexApp) -> Option<Opening> {
        app.screens()
            .select
            .shows
            .pending()
            .last()
            .map(|show| show.opening.clone())
    }

    fn removals(fx: &AppFx) -> Vec<agents::Command> {
        fx.commands()
            .iter()
            .filter_map(|command| match command {
                Command::Screen {
                    command:
                        ScreenCommand::Agents(
                            command @ (agents::Command::Archive { .. }
                            | agents::Command::Delete { .. }),
                        ),
                    ..
                } => Some(command.clone()),
                _ => None,
            })
            .collect()
    }

    fn ticket_of(fx: &AppFx) -> codex_tui_core::ids::Ticket {
        fx.commands()
            .iter()
            .find_map(|command| match command {
                Command::Screen { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("the request")
    }

    fn removal(action: Action, note: RemoveNote) -> Vec<Note> {
        vec![Note::from(NoteKey::Remove(action, note))]
    }

    fn turn_started(app: &mut CodexApp) {
        let notification: codex_app_server_protocol::ServerNotification =
            serde_json::from_value(serde_json::json!({
                "method": "turn/started",
                "params": {"threadId": T, "turn": {
                    "id": "turn-1", "items": [], "itemsView": "full",
                    "status": "inProgress", "error": null}}}))
            .expect("notification");
        run(
            app,
            AppEv::Backend(codex_tui_core::app::BackendEv::Notification(Box::new(
                notification,
            ))),
        );
    }

    fn typed(app: &mut CodexApp, thread: Option<&str>, command: SlashCommand) {
        let intent = match command {
            SlashCommand::Archive => SessionIntent::Archive,
            _ => SessionIntent::Delete,
        };
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
                thread: thread.map(ThreadId::new),
                command,
                intent,
            }))),
        );
    }

    /// `SlashCommand::Archive` and `Delete` of `dispatch_command`: the
    /// confirmation opens whatever the session is; the archive of a running
    /// turn says it stops it.
    #[test]
    fn the_commands_open_the_confirmation_the_oracle_shows() {
        let mut app = ready();
        typed(&mut app, Some(T), SlashCommand::Archive);
        assert_eq!(
            asked(&app),
            Some(Opening::Confirm(Confirm::Archive { running: false }))
        );
        turn_started(&mut app);
        typed(&mut app, Some(T), SlashCommand::Archive);
        assert_eq!(
            asked(&app),
            Some(Opening::Confirm(Confirm::Archive { running: true }))
        );
        typed(&mut app, Some(T), SlashCommand::Delete);
        assert_eq!(asked(&app), Some(Opening::Confirm(Confirm::Delete)));
        // before any thread: the question still opens, the refusal is Yes's
        let mut app = CodexApp::new();
        typed(&mut app, None, SlashCommand::Archive);
        assert_eq!(
            asked(&app),
            Some(Opening::Confirm(Confirm::Archive { running: false }))
        );
    }

    #[test]
    fn yes_asks_the_server_to_archive_or_delete_the_thread() {
        for (confirm, expected) in [
            (
                Confirm::Archive { running: false },
                agents::Command::Archive {
                    thread: T.to_string(),
                },
            ),
            (
                Confirm::Archive { running: true },
                agents::Command::Archive {
                    thread: T.to_string(),
                },
            ),
            (
                Confirm::Delete,
                agents::Command::Delete {
                    thread: T.to_string(),
                },
            ),
        ] {
            let mut app = ready();
            let fx = pick(&mut app, confirm, 1);
            assert_eq!(removals(&fx), [expected], "{confirm:?}");
            assert_eq!(app.session().active, Some(ThreadId::new(T)), "not yet gone");
        }
    }

    #[test]
    fn no_leaves_the_session_as_it_is() {
        for confirm in [Confirm::Archive { running: true }, Confirm::Delete] {
            let mut app = ready();
            let fx = pick(&mut app, confirm, 0);
            assert_eq!(removals(&fx), [], "{confirm:?}");
            assert_eq!(notes_of(&app.active_thread()), [], "{confirm:?}");
        }
    }

    #[test]
    fn a_pick_of_the_confirmation_is_not_left_for_later() {
        let mut app = ready();
        pick(&mut app, Confirm::Delete, 1);
        assert_eq!(app.screens().select.picked, []);
    }

    #[test]
    fn without_a_thread_the_answer_says_it_must_start_first() {
        for (confirm, action) in [
            (Confirm::Archive { running: false }, Action::Archive),
            (Confirm::Delete, Action::Delete),
        ] {
            let mut app = CodexApp::new();
            run(&mut app, AppEv::Connected);
            // the thread of the connection is never answered: none is active
            assert_eq!(app.session().active, None);
            let fx = pick(&mut app, confirm, 1);
            assert_eq!(removals(&fx), [], "{confirm:?}");
            assert_eq!(
                app.screens().chat.notes,
                removal(action, RemoveNote::NotStarted),
                "{confirm:?}"
            );
        }
    }

    #[test]
    fn a_side_conversation_is_not_removed() {
        for (confirm, action) in [
            (Confirm::Archive { running: false }, Action::Archive),
            (Confirm::Delete, Action::Delete),
        ] {
            let mut app = ready();
            run(
                &mut app,
                AppEv::Screen(ScreenEv::Chat(chat::Ev::Flags(CommandFlags {
                    side_conversation_active: true,
                    ..CommandFlags::default()
                }))),
            );
            let fx = pick(&mut app, confirm, 1);
            assert_eq!(removals(&fx), [], "{confirm:?}");
            assert_eq!(
                notes_of(&app.active_thread()),
                removal(action, RemoveNote::SideConversation),
                "{confirm:?}"
            );
        }
    }

    /// `return_to_agents_after_removal`: the chat forgets the thread and the
    /// command center takes its place.
    #[test]
    fn the_server_removing_the_thread_takes_the_chat_back_to_the_command_center() {
        let mut app = ready();
        let before = app.session().removed;
        let fx = pick(&mut app, Confirm::Archive { running: false }, 1);
        run(
            &mut app,
            AppEv::Reply(
                ticket_of(&fx),
                Reply::screen(ScreenReply::Agents(agents::Reply::Done)),
            ),
        );
        assert_eq!(app.session().active, None);
        assert_eq!(app.thread(&ThreadId::new(T)).map(|_| ()), None);
        assert_eq!(app.session().removed, before + 1);
    }

    #[test]
    fn a_refusal_of_the_server_is_said_in_the_chat() {
        for (confirm, action) in [
            (Confirm::Archive { running: false }, Action::Archive),
            (Confirm::Delete, Action::Delete),
        ] {
            let mut app = ready();
            let before = app.session().removed;
            let fx = pick(&mut app, confirm, 1);
            run(
                &mut app,
                AppEv::Reply(ticket_of(&fx), Reply::failed("the server said no")),
            );
            assert_eq!(
                notes_of(&app.active_thread()),
                removal(action, RemoveNote::Failed),
                "{confirm:?}"
            );
            assert_eq!(app.session().active, Some(ThreadId::new(T)), "{confirm:?}");
            assert_eq!(app.session().removed, before, "{confirm:?}");
        }
    }

    #[test]
    fn a_second_yes_while_the_first_is_unanswered_asks_once() {
        let mut app = ready();
        let first = pick(&mut app, Confirm::Delete, 1);
        let second = pick(&mut app, Confirm::Delete, 1);
        assert_eq!(removals(&first).len(), 1);
        assert_eq!(removals(&second), []);
    }
}
