#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/permissions <profile> ..` and the permission shortcut keys (oracle
//! `chatwidget/permission_discovery.rs`, `permissions_menu.rs`, `permission_shortcuts.rs`,
//! `slash_command_args.rs`): the arguments' tokens, the profile resolved against the catalog, the
//! pick that waits for a catalog still being read and keeps its profile through a retry, and the
//! cycle of built-in modes under managed requirements. The words are the view's.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::args::{self, ArgError, Spec};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::select::permission_args::{self, Args, Error, Inline, Refusal};
use codex_tui_core::screens::select::permission_shortcuts;
use codex_tui_core::screens::select::permissions::{
    self, Approval, Chosen, Current, Disabled, Discovery, Failure, Label, PermissionsMenu, Phase,
    Preset, Profile, Requirements, Reviewer, Selection,
};
use codex_tui_core::screens::select::{self, Intent as SelectIntent, Opening};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, chat};
use codex_tui_core::threads::{Item, NoteKey, Tone, UserMessage};
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::BTreeMap;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn start() -> CodexApp {
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

fn select_intent(app: &mut CodexApp, intent: SelectIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
    )
}

fn permissions_command(app: &mut CodexApp, args: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Permissions,
            intent: SessionIntent::Permissions {
                args: Some(args.to_owned()),
            },
        }))),
    )
}

/// The permission menus the screen asked for that no presenter has shown yet, oldest first.
fn asked_menus(app: &CodexApp) -> Vec<PermissionsMenu> {
    app.screens()
        .select
        .shows
        .pending()
        .filter_map(|show| match &show.opening {
            Opening::Permissions(menu) => Some(menu.clone()),
            _ => None,
        })
        .collect()
}

/// The presenter's receipt for every show waiting: the menus it shows.
fn presented(app: &mut CodexApp) -> Vec<PermissionsMenu> {
    let menus = asked_menus(app);
    let waiting: Vec<u64> = app
        .screens()
        .select
        .shows
        .pending()
        .map(|show| show.seq)
        .collect();
    for seq in waiting {
        select_intent(app, SelectIntent::Shown(seq));
    }
    menus
}

/// What a sheet on `menu` says to the model for the row `id` (a page change says nothing).
fn pick(app: &mut CodexApp, menu: &PermissionsMenu, id: &str) -> AppFx {
    let profiles = app.screens().select.profiles.clone();
    match menu
        .choose(&profiles, id)
        .expect("a row that can be chosen")
    {
        Chosen::Done(selection) => select_intent(app, SelectIntent::SetPermissions(selection)),
        Chosen::Retry => select_intent(app, SelectIntent::LoadProfiles),
        Chosen::Menu(_) => AppFx::default(),
    }
}

fn answer(app: &mut CodexApp, ticket: Ticket, reply: select::Reply) -> AppFx {
    run(
        app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Select(reply))),
    )
}

/// The select commands in `fx` (the `model/list` that warms the cyber catalog is left out).
fn asked(fx: &AppFx) -> Vec<(Ticket, select::Command)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                ticket,
                command: ScreenCommand::Select(command),
                ..
            } if !matches!(command, select::Command::Models { .. }) => {
                Some((*ticket, command.clone()))
            }
            _ => None,
        })
        .collect()
}

fn profile(id: &str) -> Profile {
    serde_json::from_value(json!({"id": id})).expect("profile")
}

fn catalog() -> Vec<Profile> {
    [
        ":read-only",
        ":workspace",
        ":danger-full-access",
        "server-only",
    ]
    .into_iter()
    .map(profile)
    .collect()
}

fn current() -> Current {
    Current {
        guardian: true,
        discover: true,
        approval: Some(Approval::OnRequest),
        reviewer: Reviewer::User,
        matches: Some(Preset::Auto),
        profile: Some(":workspace".into()),
        model: Some("gpt-5.5".into()),
        ..Current::default()
    }
}

/// `/permissions` answered once: the catalog is held and the sheet closed.
fn with_catalog(requirements: Option<Requirements>) -> CodexApp {
    let mut app = start();
    let fx = select_intent(&mut app, SelectIntent::OpenPermissions(current()));
    let (ticket, _) = asked(&fx).remove(0);
    let fx = answer(&mut app, ticket, select::Reply::Requirements(requirements));
    let (ticket, _) = asked(&fx).remove(0);
    answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: catalog(),
            next: None,
        },
    );
    presented(&mut app);
    select_intent(&mut app, SelectIntent::AbandonPermissions);
    assert!(app.screens().select.profiles.found.is_some());
    app
}

fn discovery(requirements: Option<Requirements>) -> Discovery {
    Discovery::new(catalog(), requirements).expect("a catalog")
}

fn only_profile(id: &str, allowed: bool) -> Requirements {
    Requirements {
        profiles: Some(BTreeMap::from([(id.to_owned(), allowed)])),
        ..Requirements::default()
    }
}

/// The patch of the one `Settings` command in `fx`: the profile, the policy and the reviewer.
fn settings(fx: &AppFx) -> Option<(Option<String>, Option<Approval>, Option<Reviewer>)> {
    asked(fx)
        .into_iter()
        .find_map(|(_, command)| match command {
            select::Command::Settings { patch, .. } => Some((
                patch.permissions,
                patch.approval_policy,
                patch.approvals_reviewer,
            )),
            _ => None,
        })
}

fn notes(app: &CodexApp) -> Vec<(select::Note, Tone)> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Select(select) => Some((select.clone(), note.tone)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

// ---- the shared tokens (oracle slash_command_args_tests.rs)

#[test]
fn args_hold_positionals_values_switches_booleans_and_quotes() {
    let spec = Spec {
        values: &["--reviewer", "--speed"],
        switches: &["--clear"],
        booleans: &["colors"],
    };
    let parsed = args::parse(
        "'team review' item --reviewer user --speed=fast --clear --no-colors -- --literal",
        spec,
    )
    .expect("valid command arguments");
    assert_eq!(
        parsed.positionals,
        ["team review", "item", "--literal"].map(String::from)
    );
    assert_eq!(parsed.value("--reviewer"), Some("user"));
    assert_eq!(parsed.value("--speed"), Some("fast"));
    assert!(parsed.has_switch("--clear"));
    assert_eq!(parsed.boolean("colors"), Some(false));
}

#[test]
fn args_reject_the_named_arguments_that_do_not_fit() {
    let spec = Spec {
        values: &["--reviewer"],
        switches: &["--clear"],
        booleans: &["colors"],
    };
    for (line, expected) in [
        ("--reviewer", ArgError::NeedsValue("--reviewer".into())),
        ("--clear=yes", ArgError::TakesNoValue("--clear".into())),
        ("--clear --clear", ArgError::Repeated("--clear".into())),
        ("--colors --no-colors", ArgError::OnlyOne("colors".into())),
        ("--unknown", ArgError::Unknown("--unknown".into())),
        ("'unfinished", ArgError::Quoting),
        ("dangling\\", ArgError::Quoting),
    ] {
        assert_eq!(args::parse(line, spec).err(), Some(expected), "{line}");
    }
}

#[test]
fn args_split_like_the_shell() {
    let parsed = args::parse(r#"a "b c" 'd e' f\ g "h\"i" # comment"#, Spec::default()).unwrap();
    assert_eq!(
        parsed.positionals,
        ["a", "b c", "d e", "f g", "h\"i"].map(String::from)
    );
}

// ---- the arguments of /permissions

#[test]
fn the_arguments_name_a_profile_a_policy_and_a_reviewer() {
    assert_eq!(
        permission_args::parse("server-only --approval-policy=never --reviewer user"),
        Ok(Args {
            profile: "server-only".into(),
            approval: Some(Approval::Never),
            reviewer: Some(Reviewer::User),
        })
    );
    assert_eq!(
        permission_args::parse(":workspace --approval-policy untrusted --reviewer=auto_review"),
        Ok(Args {
            profile: ":workspace".into(),
            approval: Some(Approval::UnlessTrusted),
            reviewer: Some(Reviewer::AutoReview),
        })
    );
    // a granular policy is its JSON
    let granular = r#"{"granular":{"sandbox_approval":true,"rules":false,"skill_approval":false,"request_permissions":false,"mcp_elicitations":false}}"#;
    let parsed = permission_args::parse(&format!("p --approval-policy '{granular}'")).unwrap();
    assert!(matches!(parsed.approval, Some(Approval::Granular { .. })));
}

#[test]
fn bad_arguments_are_typed_errors() {
    for (line, error) in [
        ("", Error::Usage),
        ("--reviewer user", Error::Usage),
        ("a b", Error::Unexpected("b".into())),
        (
            "a --approval-policy sometimes",
            Error::Approval("sometimes".into()),
        ),
        ("a --reviewer robot", Error::Reviewer("robot".into())),
        (
            "a --reviewer",
            Error::Args(ArgError::NeedsValue("--reviewer".into())),
        ),
        ("a --nope", Error::Args(ArgError::Unknown("--nope".into()))),
    ] {
        assert_eq!(permission_args::parse(line), Err(error), "{line}");
    }
}

// ---- resolving a profile (oracle try_select_permission_profile)

fn args(profile: &str) -> Args {
    permission_args::parse(profile).expect("args")
}

fn mode(preset: Preset, reviewer: Reviewer) -> Label {
    Label::Mode { preset, reviewer }
}

#[test]
fn a_builtin_profile_takes_its_policy_and_reviewer() {
    let found = discovery(None);
    assert_eq!(
        permission_args::resolve(&args(":read-only"), &found, true),
        Ok(Selection::Profile {
            id: ":read-only".into(),
            approval: Some(Approval::OnRequest),
            reviewer: Some(Reviewer::User),
            label: mode(Preset::ReadOnly, Reviewer::User),
            unchanged: false,
        })
    );
    // the two named modes are `:workspace` with a reviewer
    for (name, reviewer) in [
        (":ask-for-approval", Reviewer::User),
        (":approve-for-me", Reviewer::AutoReview),
    ] {
        assert_eq!(
            permission_args::resolve(&args(name), &found, true),
            Ok(Selection::Profile {
                id: ":workspace".into(),
                approval: Some(Approval::OnRequest),
                reviewer: Some(reviewer),
                label: mode(Preset::Auto, reviewer),
                unchanged: false,
            }),
            "{name}"
        );
    }
}

#[test]
fn a_profile_of_the_server_carries_only_what_was_asked() {
    let found = discovery(None);
    assert_eq!(
        permission_args::resolve(&args("server-only"), &found, false),
        Ok(Selection::Profile {
            id: "server-only".into(),
            approval: None,
            reviewer: None,
            label: Label::Profile("server-only".into()),
            unchanged: false,
        })
    );
    assert_eq!(
        permission_args::resolve(
            &args("server-only --approval-policy never --reviewer user"),
            &found,
            false
        ),
        Ok(Selection::Profile {
            id: "server-only".into(),
            approval: Some(Approval::Never),
            reviewer: Some(Reviewer::User),
            label: Label::Profile("server-only".into()),
            unchanged: false,
        })
    );
}

#[test]
fn an_unknown_profile_is_refused() {
    assert_eq!(
        permission_args::resolve(&args("unknown-profile"), &discovery(None), true),
        Err(Refusal::Unknown("unknown-profile".into()))
    );
    // a mode name is looked up by the profile it stands for
    let bare = Discovery::new(vec![profile("server-only")], None).unwrap();
    assert_eq!(
        permission_args::resolve(&args(":ask-for-approval"), &bare, true),
        Err(Refusal::Unknown(":workspace".into()))
    );
}

#[test]
fn approve_for_me_needs_the_guardian_feature() {
    let found = discovery(None);
    for line in [":approve-for-me", ":workspace --reviewer auto_review"] {
        assert_eq!(
            permission_args::resolve(&args(line), &found, false),
            Err(Refusal::ApproveUnavailable),
            "{line}"
        );
    }
    assert!(
        permission_args::resolve(&args(":workspace --reviewer auto_review"), &found, true).is_ok()
    );
}

#[test]
fn requirements_disable_a_profile_a_policy_or_a_reviewer() {
    let barred = discovery(Some(only_profile(":read-only", false)));
    assert_eq!(
        permission_args::resolve(&args(":read-only"), &barred, true),
        Err(Refusal::Unavailable {
            profile: ":read-only".into(),
            why: Disabled::Requirements
        })
    );
    let by_policy = discovery(Some(Requirements {
        approvals: Some(vec![Approval::Never]),
        ..Requirements::default()
    }));
    assert!(matches!(
        permission_args::resolve(&args(":workspace"), &by_policy, true),
        Err(Refusal::Unavailable { .. })
    ));
    let by_reviewer = discovery(Some(Requirements {
        reviewers: Some(vec![Reviewer::AutoReview]),
        ..Requirements::default()
    }));
    assert!(matches!(
        permission_args::resolve(&args(":workspace"), &by_reviewer, true),
        Err(Refusal::Unavailable { .. })
    ));
    assert!(permission_args::resolve(&args(":approve-for-me"), &by_reviewer, true).is_ok());
}

// ---- the command through the app (oracle tests/permissions.rs)

#[test]
fn a_held_catalog_applies_the_profile_without_a_menu() {
    let mut app = with_catalog(None);
    let fx = permissions_command(&mut app, ":read-only");
    assert_eq!(asked_menus(&app), vec![], "no menu opens");
    assert_eq!(
        settings(&fx),
        Some((
            Some(":read-only".into()),
            Some(Approval::OnRequest),
            Some(Reviewer::User)
        ))
    );
    assert!(!app.screens().select.profiles.inline_pending());
}

#[test]
fn a_catalog_still_being_read_is_waited_for() {
    let mut app = start();
    let fx = permissions_command(&mut app, "server-only");
    let [menu] = asked_menus(&app).try_into().expect("one menu asked for");
    assert_eq!(
        menu.phase(&app.screens().select.profiles),
        Phase::Loading,
        "the wait shows meanwhile"
    );
    presented(&mut app);
    assert!(matches!(
        app.screens().select.profiles.inline,
        Some(Inline::Discovering { .. })
    ));
    let (ticket, command) = asked(&fx).remove(0);
    assert!(matches!(command, select::Command::Requirements { .. }));
    let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    let (ticket, _) = asked(&fx).remove(0);
    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: catalog(),
            next: None,
        },
    );
    assert_eq!(
        asked_menus(&app),
        vec![],
        "the pick takes the catalog: no menu opens for it"
    );
    assert_eq!(
        settings(&fx),
        Some((Some("server-only".into()), None, None))
    );
    assert!(!app.screens().select.profiles.inline_pending());
}

#[test]
fn a_retry_after_a_failed_read_keeps_the_profile() {
    let mut app = start();
    let fx = permissions_command(&mut app, "server-only");
    let (ticket, _) = asked(&fx).remove(0);
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::error(RpcError::Remote("disconnected".into())),
        ),
    );
    let [menu] = presented(&mut app).try_into().expect("one menu asked for");
    let profiles = &app.screens().select.profiles;
    assert_eq!(menu.phase(profiles), Phase::Failed);
    assert!(matches!(profiles.failure, Some(Failure::Other(_))));
    let fx = pick(&mut app, &menu, permissions::RETRY);
    assert_eq!(
        menu.phase(&app.screens().select.profiles),
        Phase::Loading,
        "the same menu waits again"
    );
    let (ticket, command) = asked(&fx).remove(0);
    assert!(matches!(command, select::Command::Requirements { .. }));
    let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    let (ticket, _) = asked(&fx).remove(0);
    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: catalog(),
            next: None,
        },
    );
    assert_eq!(
        settings(&fx),
        Some((Some("server-only".into()), None, None)),
        "the retry applied the profile the command named"
    );
}

#[test]
fn a_refused_profile_is_said_and_holds_the_queue() {
    let mut app = with_catalog(Some(only_profile(":read-only", false)));
    // a message waits behind a running turn
    run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text("go"),
        }),
    );
    let intent = chat::ThreadIntent::Queue(UserMessage::text("follow-up").into());
    run(
        &mut app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: codex_tui_core::screens::ScreenThreadIntent::Chat(intent),
        }),
    );
    assert!(!app.active_thread().queue_held());
    let fx = permissions_command(&mut app, ":read-only");
    assert_eq!(settings(&fx), None);
    assert_eq!(
        notes(&app),
        vec![(
            select::Note::PermissionRefused(Refusal::Unavailable {
                profile: ":read-only".into(),
                why: Disabled::Requirements,
            }),
            Tone::Error
        )]
    );
    assert!(!app.screens().select.hold_queue, "the app took the flag");
    assert!(
        app.active_thread().queue_held(),
        "the follow-up stays unsent"
    );
}

#[test]
fn unknown_profiles_and_bad_arguments_are_errors_in_the_history() {
    let mut app = with_catalog(None);
    permissions_command(&mut app, "unknown-profile");
    permissions_command(&mut app, "a b");
    assert_eq!(
        notes(&app),
        vec![
            (
                select::Note::PermissionRefused(Refusal::Unknown("unknown-profile".into())),
                Tone::Error
            ),
            (
                select::Note::PermissionArgs(Error::Unexpected("b".into())),
                Tone::Error
            ),
        ]
    );
}

#[test]
fn leaving_the_loading_menu_gives_the_pick_up() {
    let mut app = start();
    let fx = permissions_command(&mut app, "server-only");
    let (ticket, _) = asked(&fx).remove(0);
    presented(&mut app);
    select_intent(&mut app, SelectIntent::AbandonPermissions);
    assert!(!app.screens().select.profiles.inline_pending());
    // the late answer only fills the cache
    let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    let (ticket, _) = asked(&fx).remove(0);
    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: catalog(),
            next: None,
        },
    );
    assert_eq!(settings(&fx), None);
    assert!(app.screens().select.profiles.found.is_some());
    assert_eq!(asked_menus(&app), vec![]);
}

#[test]
fn a_failed_read_keeps_the_pick_waiting_until_the_sheet_is_left() {
    let mut app = start();
    let fx = permissions_command(&mut app, "server-only");
    let (ticket, _) = asked(&fx).remove(0);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("down".into()))),
    );
    // the failure is a fact the sheet shows with its Retry row; nothing is said in the history
    assert_eq!(
        app.screens().select.profiles.failure,
        Some(Failure::Other("down".into()))
    );
    assert!(app.screens().select.profiles.inline_pending());
    assert!(app.screens().select.profiles.inline_waits());
    assert_eq!(notes(&app), vec![]);
    select_intent(&mut app, SelectIntent::AbandonPermissions);
    assert!(!app.screens().select.profiles.inline_pending());
}

#[test]
fn full_access_by_argument_asks_first_and_declining_gives_the_pick_up() {
    let mut app = with_catalog(None);
    let fx = permissions_command(&mut app, ":danger-full-access");
    assert_eq!(settings(&fx), None, "full access is confirmed first");
    let [menu] = presented(&mut app).try_into().expect("one menu asked for");
    assert!(matches!(menu, PermissionsMenu::FullAccess { .. }));
    // the pick waits on the confirmation, no longer on the catalog: the sheet of the wait is gone
    assert!(app.screens().select.profiles.inline_pending());
    assert!(!app.screens().select.profiles.inline_waits());
    let fx = pick(&mut app, &menu, permissions::CONTINUE);
    assert_eq!(
        settings(&fx),
        Some((
            Some(":danger-full-access".into()),
            Some(Approval::Never),
            Some(Reviewer::User)
        ))
    );
    assert!(!app.screens().select.profiles.inline_pending());
}

#[test]
fn escaping_the_full_access_confirmation_gives_the_pick_up() {
    let mut app = with_catalog(None);
    permissions_command(&mut app, ":danger-full-access");
    presented(&mut app);
    select_intent(&mut app, SelectIntent::AbandonPermissions);
    assert!(!app.screens().select.profiles.inline_pending());
}

// ---- the shortcut cycle (oracle permission_shortcuts_tests.rs)

fn chosen(current: &Current, found: &Discovery, forward: bool) -> Option<(String, Reviewer)> {
    permission_shortcuts::next(current, found, forward).map(|selection| match selection {
        Selection::Profile { id, reviewer, .. } => (id, reviewer.expect("a reviewer")),
        Selection::Preset { .. } => panic!("a catalog mode"),
    })
}

#[test]
fn the_shortcuts_cycle_the_builtin_modes() {
    let found = discovery(None);
    let user = Current {
        profile: Some(":workspace".into()),
        reviewer: Reviewer::User,
        ..current()
    };
    let auto = Current {
        reviewer: Reviewer::AutoReview,
        ..user.clone()
    };
    let read_only = Current {
        profile: Some(":read-only".into()),
        matches: Some(Preset::ReadOnly),
        ..user.clone()
    };
    let cycle = |current: &Current, forward| chosen(current, &found, forward);
    assert_eq!(
        cycle(&user, true),
        Some((":workspace".into(), Reviewer::AutoReview))
    );
    assert_eq!(
        cycle(&auto, true),
        Some((":read-only".into(), Reviewer::User))
    );
    assert_eq!(
        cycle(&read_only, true),
        Some((":workspace".into(), Reviewer::User))
    );
    assert_eq!(
        cycle(&read_only, false),
        Some((":workspace".into(), Reviewer::AutoReview))
    );
}

#[test]
fn the_shortcuts_skip_what_the_requirements_forbid() {
    let in_auto = Current {
        reviewer: Reviewer::AutoReview,
        ..current()
    };
    let by_reviewers = discovery(Some(Requirements {
        reviewers: Some(vec![Reviewer::AutoReview]),
        ..Requirements::default()
    }));
    let by_model = discovery(Some(Requirements {
        auto_required: Some(vec!["gpt-5.5".into()]),
        ..Requirements::default()
    }));
    let by_profiles = discovery(Some(Requirements {
        profiles: Some(BTreeMap::from([
            (":workspace".to_owned(), false),
            (":read-only".to_owned(), false),
        ])),
        ..Requirements::default()
    }));
    for (found, current) in [
        (&by_reviewers, &in_auto),
        (&by_model, &in_auto),
        (&by_profiles, &current()),
    ] {
        assert_eq!(chosen(current, found, true), None);
    }
}

#[test]
fn the_required_models_match_a_provider_alias_only() {
    let models = ["protected-model".to_owned()];
    for (model, protected) in [
        ("protected-model", true),
        ("protected-model-preview", false),
        ("openai-codex/protected-model-preview", false),
        ("provider_1/protected-model", true),
        ("protected-modelish", false),
        ("/protected-model", false),
        ("bad.provider/protected-model", false),
        ("provider/nested/protected-model", false),
    ] {
        assert_eq!(
            permission_shortcuts::auto_review_required(&models, model),
            protected,
            "{model}"
        );
    }
}

#[test]
fn a_shortcut_picks_once_and_waits_for_its_answer() {
    let mut app = with_catalog(None);
    let key = |app: &mut CodexApp| {
        select_intent(
            app,
            SelectIntent::PermissionShortcut {
                forward: true,
                current: current(),
            },
        )
    };
    let fx = key(&mut app);
    assert_eq!(
        settings(&fx),
        Some((
            Some(":workspace".into()),
            Some(Approval::OnRequest),
            Some(Reviewer::AutoReview)
        ))
    );
    assert_eq!(asked(&key(&mut app)), vec![], "the second press is ignored");
    let (ticket, _) = asked(&fx).remove(0);
    answer(&mut app, ticket, select::Reply::Updated);
    assert!(!app.screens().select.profiles.shortcut);
    assert_eq!(
        asked(&key(&mut app)).len(),
        1,
        "after the answer the key works again"
    );
}

#[test]
fn a_shortcut_without_another_mode_says_so() {
    let mut app = with_catalog(Some(only_profile(":workspace", false)));
    // only Read Only is left, and the thread is in it
    let in_read_only = Current {
        profile: Some(":read-only".into()),
        matches: Some(Preset::ReadOnly),
        ..current()
    };
    let fx = select_intent(
        &mut app,
        SelectIntent::PermissionShortcut {
            forward: true,
            current: in_read_only,
        },
    );
    assert_eq!(asked(&fx), vec![]);
    assert_eq!(
        notes(&app),
        vec![(select::Note::NoOtherPermissionModes, Tone::Info)]
    );
}

#[test]
fn a_shortcut_without_a_catalog_asks_for_it_once() {
    let mut app = start();
    let key = |app: &mut CodexApp| {
        select_intent(
            app,
            SelectIntent::PermissionShortcut {
                forward: true,
                current: current(),
            },
        )
    };
    let fx = key(&mut app);
    assert_eq!(asked(&fx).len(), 1, "the catalog is requested");
    assert_eq!(asked_menus(&app).len(), 1, "its sheet shows the wait");
    select_intent(&mut app, SelectIntent::AbandonPermissions);
    assert_eq!(asked(&key(&mut app)), vec![], "the pending read is reused");
}

#[test]
fn permission_catalog_survives_foreign_loss_and_retires_with_its_physical_owner() {
    use codex_tui_core::app::Ingress;
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::ScreenThreadIntent;

    let mut app = start();
    let foreign = BackendId(1);
    let foreign_epoch = app.attach_backend(foreign);
    let cycle = |app: &mut CodexApp| {
        run(
            app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new(T),
                intent: ScreenThreadIntent::ApplyAdjacentPermissions { forward: true },
            }),
        )
    };
    let before = cycle(&mut app);
    let (old_requirements, _) = asked(&before).into_iter().next().expect("requirements");
    app.ingest(Ingress::of(foreign, foreign_epoch, AppEv::Disconnected));
    assert!(app.screens().select.profiles.pending.is_some());
    assert!(asked(&cycle(&mut app)).is_empty());

    let page = answer(
        &mut app,
        old_requirements,
        select::Reply::Requirements(None),
    );
    let (old_page, _) = asked(&page).into_iter().next().expect("profile page");
    answer(
        &mut app,
        old_page,
        select::Reply::Profiles {
            profiles: catalog(),
            next: None,
        },
    );
    assert!(app.screens().select.profiles.found.is_some());
    app.ingest(Ingress::of(foreign, foreign_epoch, AppEv::Disconnected));
    assert!(app.screens().select.profiles.found.is_some());

    let owner = BackendId::default();
    let epoch = app.epoch_of(owner);
    app.ingest(Ingress::transport_replaced(owner, epoch, epoch.next()))
        .expect("new transport");
    assert!(app.screens().select.profiles.found.is_none());
    let reads = select_intent(
        &mut app,
        SelectIntent::OpenPermissions(Current {
            discover: true,
            ..current()
        }),
    );
    let (new_read, request) = asked(&reads)
        .into_iter()
        .next()
        .expect("a fresh catalog read");
    assert!(matches!(request, select::Command::Requirements { .. }));
    assert_ne!(new_read, old_requirements);

    for (ticket, response) in [
        (old_requirements, select::Reply::Requirements(None)),
        (
            old_page,
            select::Reply::Profiles {
                profiles: catalog(),
                next: None,
            },
        ),
    ] {
        assert!(
            app.ingest(Ingress::transport_reply(
                owner,
                epoch,
                epoch,
                ticket,
                Reply::screen(ScreenReply::Select(response))
            ))
            .is_none()
        );
    }
    assert!(app.screens().select.profiles.found.is_none());
    assert!(app.screens().select.profiles.pending.is_some());
}

#[test]
fn permission_requirements_from_a_retired_generation_cannot_open_a_page() {
    let mut app = start();
    let open = |app: &mut CodexApp| {
        select_intent(
            app,
            SelectIntent::OpenPermissions(Current {
                discover: true,
                ..current()
            }),
        )
    };
    let old = asked(&open(&mut app)).remove(0).0;
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(select::Ev::ScopeChanged)),
    );
    let fresh = asked(&open(&mut app)).remove(0).0;
    assert!(asked(&answer(&mut app, old, select::Reply::Requirements(None))).is_empty());
    assert!(matches!(
        &asked(&answer(&mut app, fresh, select::Reply::Requirements(None)))[0].1,
        select::Command::Profiles { .. }
    ));
}
