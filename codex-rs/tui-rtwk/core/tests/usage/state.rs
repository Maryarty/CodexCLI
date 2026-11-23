//! The state of the usage model (oracle: `analytics/account_layout_tests.rs`,
//! `sections.rs`, `controls.rs`): which sections an account kind shows, which
//! reports start and when, what a range, group or model change reloads, and
//! how a reply of another session or stamp is dropped. The asks are what the
//! host would be sent; the tests answer them as the host does.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_protocol::account::PlanType;
use codex_tui_core::screens::usage::models::{AccountAnalyticsHistory, AccountAnalyticsUnit};
use codex_tui_core::screens::usage::{
    AccountReply, Ask, AskKind, Control, Ev, GroupName, Intent, Load, LoadError, Model, Note,
    ReportReply, Section, Stamp, settle, step,
};
use pretty_assertions::assert_eq;

fn today() -> codex_tui_core::screens::usage::NaiveDate {
    codex_tui_core::screens::usage::NaiveDate::from_ymd_opt(2026, 9, 9).unwrap()
}

fn reply(plan: PlanType) -> AccountReply {
    AccountReply {
        plan: Some(plan),
        label: Some("example@example.com".into()),
        today: today(),
        plan_enabled: false,
        model_names: vec![("gpt-5".into(), "GPT-5".into())],
        credit_groups: vec![0, 2],
        hour24: false,
    }
}

fn history() -> AccountAnalyticsHistory {
    AccountAnalyticsHistory {
        unit: AccountAnalyticsUnit::Tokens,
        updated_at: None,
        data: Vec::new(),
    }
}

/// Opens the page and answers the account as the host would.
fn opened(plan: PlanType) -> (Model, Vec<Ask>) {
    let mut model = Model::default();
    let asks = step(&mut model, Intent::Open { view: None });
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].what, AskKind::Account);
    let asks = settle(
        &mut model,
        Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Ok(reply(plan)),
        },
    );
    (model, asks)
}

fn answer(model: &mut Model, ask: &Ask) {
    let (session, stamp) = (ask.session, ask.stamp);
    match ask.what {
        AskKind::Tasks => {
            settle(
                model,
                Ev::Tasks {
                    session,
                    stamp,
                    result: Ok(Some(Default::default())),
                },
            );
            return;
        }
        AskKind::Chats => {
            settle(
                model,
                Ev::Chats {
                    session,
                    stamp,
                    result: Ok(Some(Default::default())),
                },
            );
            return;
        }
        _ => {}
    }
    let AskKind::Report { section, .. } = ask.what else {
        panic!("{ask:?}")
    };
    settle(
        model,
        Ev::Report {
            session: ask.session,
            stamp: ask.stamp,
            section,
            result: Ok(ReportReply {
                history: Some(history()),
                token_models: None,
                attributed_usage: None,
            }),
        },
    );
}

fn sections_asked(asks: &[Ask]) -> Vec<Section> {
    asks.iter()
        .filter_map(|ask| match ask.what {
            AskKind::Report { section, .. } => Some(section),
            _ => None,
        })
        .collect()
}

#[test]
fn account_layouts_show_the_reports_of_each_kind() {
    for (plan, expected) in [
        (
            PlanType::Plus,
            vec![
                Section::Summary,
                Section::Usage,
                Section::Activity,
                Section::Plugins,
                Section::Skills,
                Section::Chats,
            ],
        ),
        (
            PlanType::Enterprise,
            vec![
                Section::Summary,
                Section::Credits,
                Section::Usage,
                Section::Plugins,
                Section::Skills,
            ],
        ),
        (
            PlanType::Business,
            vec![
                Section::Summary,
                Section::Credits,
                Section::Usage,
                Section::Plugins,
                Section::Skills,
                Section::Chats,
            ],
        ),
        (PlanType::Unknown, vec![Section::Summary]),
    ] {
        let (model, _) = opened(plan);
        assert_eq!(model.visible_sections(), &expected[..], "{plan:?}");
    }
    let (mut model, _) = opened(PlanType::Plus);
    model.plan_enabled = true;
    assert_eq!(model.visible_sections()[2], Section::Plan);
}

#[test]
fn the_usage_tab_is_the_business_one_per_account_kind() {
    // The words of the tabs live in the page's string table; the core only
    // says which account kind a tab is for.
    let (consumer, _) = opened(PlanType::Plus);
    let (business, _) = opened(PlanType::Business);
    assert!(!consumer.business());
    assert!(business.business());
}

#[test]
fn account_pending_and_error_start_no_report() {
    let mut model = Model::default();
    let asks = step(&mut model, Intent::Open { view: None });
    assert!(model.account.is_loading());
    assert!(
        model
            .histories
            .iter()
            .all(|load| *load == Load::Unavailable)
    );
    let asks = settle(
        &mut model,
        Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Err(LoadError::Host(
                "Sign in again to view Analytics.".to_owned(),
            )),
        },
    );
    assert!(asks.is_empty());
    assert_eq!(
        model.account.note(),
        Some(Note::Error(LoadError::Host(
            "Sign in again to view Analytics.".to_owned()
        )))
    );
    assert!(model.visible_sections().is_empty());
    assert!(
        model
            .histories
            .iter()
            .all(|load| *load == Load::Unavailable)
    );
}

#[test]
fn an_account_starts_only_its_eligible_reports_once() {
    let (model, asks) = opened(PlanType::Plus);
    assert_eq!(asks[0].what, AskKind::Profile);
    assert_eq!(
        sections_asked(&asks),
        [
            Section::Usage,
            Section::Activity,
            Section::Plugins,
            Section::Skills
        ]
    );
    assert!(model.profile.is_loading());
    let (_, business) = opened(PlanType::Business);
    assert_eq!(
        sections_asked(&business),
        [
            Section::Credits,
            Section::Usage,
            Section::Plugins,
            Section::Skills
        ]
    );
    // The same account again changes nothing: no second start.
    let (mut model, _) = opened(PlanType::Plus);
    let session = model.session;
    let again = settle(
        &mut model,
        Ev::Account {
            session,
            stamp: Stamp(1),
            result: Ok(reply(PlanType::Plus)),
        },
    );
    assert!(again.is_empty());
}

#[test]
fn the_first_asks_carry_the_groups_and_ranges_of_the_oracle() {
    let (_, asks) = opened(PlanType::Plus);
    let usage = asks
        .iter()
        .find_map(|ask| match &ask.what {
            AskKind::Report {
                section: Section::Usage,
                days,
                group,
                model,
            } => Some((*days, *group, model.clone())),
            _ => None,
        })
        .unwrap();
    // 7 days, grouped by feature (the oracle's `new()` sets group 1).
    assert_eq!(usage, (7, 1, None));
    // Business tokens group by model (2) for a team-like plan, by token type
    // (6) for an enterprise workspace, when the stored group is not allowed.
    for (plan, expected) in [(PlanType::Team, 2), (PlanType::Business, 6)] {
        let (_, asks) = opened(plan);
        let group = asks
            .iter()
            .find_map(|ask| match &ask.what {
                AskKind::Report {
                    section: Section::Usage,
                    group,
                    ..
                } => Some(*group),
                _ => None,
            })
            .unwrap();
        assert_eq!(group, expected, "{plan:?}");
    }
}

#[test]
fn a_range_change_reloads_only_its_group_and_keeps_the_other_reports() {
    let (mut model, asks) = opened(PlanType::Plus);
    for ask in &asks[1..] {
        answer(&mut model, ask);
    }
    assert!(model.histories[Section::Usage as usize].ready().is_some());
    // On Usage: the usage range group; Activity and the tools keep theirs.
    step(&mut model, Intent::Select(Section::Usage));
    let asks = step(&mut model, Intent::Range);
    assert_eq!(sections_asked(&asks), [Section::Usage]);
    assert_eq!(model.ranges, [1, 0, 0]);
    assert!(model.histories[Section::Usage as usize].is_loading());
    assert!(
        model.histories[Section::Activity as usize]
            .ready()
            .is_some()
    );
    match &asks[0].what {
        AskKind::Report { days, .. } => assert_eq!(*days, 30),
        other => panic!("{other:?}"),
    }
    // Plugins and Skills share the tools group.
    step(&mut model, Intent::Select(Section::Plugins));
    let asks = step(&mut model, Intent::Range);
    assert_eq!(sections_asked(&asks), [Section::Plugins, Section::Skills]);
    assert_eq!(model.ranges, [1, 0, 1]);
    // The Overview has no range of its own.
    step(&mut model, Intent::Select(Section::Summary));
    assert!(step(&mut model, Intent::Range).is_empty());
    assert_eq!(model.ranges, [1, 0, 1]);
}

#[test]
fn the_group_cycle_follows_the_options_and_reloads_the_section() {
    let (mut model, asks) = opened(PlanType::Plus);
    for ask in &asks[1..] {
        answer(&mut model, ask);
    }
    step(&mut model, Intent::Select(Section::Usage));
    assert_eq!(model.group_options(), [1, 2, 0, 3]);
    let asks = step(&mut model, Intent::Group);
    assert_eq!(model.groups[Section::Usage as usize], 2);
    assert_eq!(sections_asked(&asks), [Section::Usage]);
    for expected in [0, 3, 1] {
        step(&mut model, Intent::Group);
        assert_eq!(model.groups[Section::Usage as usize], expected);
    }
    // Plugins have no groupings: no control, no ask.
    step(&mut model, Intent::Select(Section::Plugins));
    assert!(!model.control_available(Control::Group, false));
    assert!(step(&mut model, Intent::Group).is_empty());
}

#[test]
fn an_unattributed_consumer_falls_back_to_the_product_grouping() {
    let (mut model, asks) = opened(PlanType::Plus);
    let usage = asks
        .iter()
        .find(|ask| {
            matches!(
                ask.what,
                AskKind::Report {
                    section: Section::Usage,
                    ..
                }
            )
        })
        .unwrap();
    settle(
        &mut model,
        Ev::Report {
            session: usage.session,
            stamp: usage.stamp,
            section: Section::Usage,
            result: Ok(ReportReply {
                history: Some(history()),
                token_models: None,
                attributed_usage: Some(false),
            }),
        },
    );
    assert_eq!(model.groups[Section::Usage as usize], 0);
    step(&mut model, Intent::Select(Section::Usage));
    assert_eq!(model.group_options(), [0, 2]);
    assert_eq!(model.group_name(Section::Usage, 0), GroupName::Product);
}

#[test]
fn the_business_model_control_cycles_the_models_and_filters_the_tokens() {
    let (mut model, asks) = opened(PlanType::Business);
    let usage = asks
        .iter()
        .find(|ask| {
            matches!(
                ask.what,
                AskKind::Report {
                    section: Section::Usage,
                    ..
                }
            )
        })
        .unwrap();
    settle(
        &mut model,
        Ev::Report {
            session: usage.session,
            stamp: usage.stamp,
            section: Section::Usage,
            result: Ok(ReportReply {
                history: Some(history()),
                token_models: Some(vec!["gpt-5".into(), "gpt-5-mini".into()]),
                attributed_usage: None,
            }),
        },
    );
    step(&mut model, Intent::Select(Section::Usage));
    assert!(model.control_available(Control::Model, false));
    let mut seen = Vec::new();
    for _ in 0..3 {
        let asks = step(&mut model, Intent::Model);
        let AskKind::Report { model: filter, .. } = &asks[0].what else {
            panic!("{asks:?}")
        };
        seen.push(filter.clone());
    }
    assert_eq!(
        seen,
        [
            Some("gpt-5".to_string()),
            Some("gpt-5-mini".to_string()),
            None
        ]
    );
    // A consumer has no model control.
    let (mut consumer, _) = opened(PlanType::Plus);
    step(&mut consumer, Intent::Select(Section::Usage));
    assert!(!consumer.control_available(Control::Model, false));
    assert!(step(&mut consumer, Intent::Model).is_empty());
}

#[test]
fn a_stale_or_foreign_reply_settles_nothing() {
    let (mut model, asks) = opened(PlanType::Plus);
    let usage = asks
        .iter()
        .find(|ask| {
            matches!(
                ask.what,
                AskKind::Report {
                    section: Section::Usage,
                    ..
                }
            )
        })
        .unwrap()
        .clone();
    // A range change replaces the load: the first answer is stale.
    step(&mut model, Intent::Select(Section::Usage));
    let newer = step(&mut model, Intent::Range).remove(0);
    answer(&mut model, &usage);
    assert!(model.histories[Section::Usage as usize].is_loading());
    answer(&mut model, &newer);
    assert!(model.histories[Section::Usage as usize].ready().is_some());
    // A reply of the session before a refresh is dropped too.
    let before = step(&mut model, Intent::Range).remove(0);
    step(&mut model, Intent::Refresh);
    answer(&mut model, &before);
    assert_eq!(model.histories[Section::Usage as usize], Load::Unavailable);
    assert!(model.account.is_loading());
}

#[test]
fn a_refresh_starts_a_session_and_a_close_drops_every_load() {
    let (mut model, asks) = opened(PlanType::Plus);
    for ask in &asks[1..] {
        answer(&mut model, ask);
    }
    step(&mut model, Intent::Select(Section::Skills));
    let session = model.session;
    let asks = step(&mut model, Intent::Refresh);
    assert_eq!((model.session, asks.len()), (session + 1, 1));
    assert_eq!(model.section, Section::Skills);
    assert!(
        model
            .histories
            .iter()
            .all(|load| *load == Load::Unavailable)
    );
    assert!(model.profile == Load::Unavailable);
    step(&mut model, Intent::Leave);
    assert!(!model.live);
    assert!(model.account == Load::Unavailable);
    // An answer arriving after the close finds no load to settle.
    settle(
        &mut model,
        Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Ok(reply(PlanType::Plus)),
        },
    );
    assert!(model.account == Load::Unavailable);
}

#[test]
fn an_identity_change_reopens_the_session() {
    let (mut model, _) = opened(PlanType::Plus);
    let session = model.session;
    let asks = settle(&mut model, Ev::Invalidated { session });
    assert_eq!((model.session, asks.len()), (session + 1, 1));
    // An invalidation of a session already left changes nothing.
    assert!(settle(&mut model, Ev::Invalidated { session }).is_empty());
    assert_eq!(model.session, session + 1);
}

#[test]
fn the_slash_command_picks_the_overview_chart() {
    let mut model = Model::default();
    step(
        &mut model,
        Intent::Open {
            view: Some(codex_tui_core::screens::usage::TokenActivityView::Cumulative),
        },
    );
    assert_eq!(model.section, Section::Summary);
    assert_eq!(model.groups[Section::Summary as usize], 2);
}

#[test]
fn selecting_a_hidden_section_changes_nothing() {
    let (mut model, _) = opened(PlanType::Plus);
    step(&mut model, Intent::Select(Section::Credits));
    assert_eq!(model.section, Section::Summary);
    step(&mut model, Intent::Select(Section::Chats));
    assert_eq!(model.section, Section::Chats);
}

#[test]
fn the_date_range_of_a_section_ends_on_the_session_day() {
    let (mut model, _) = opened(PlanType::Plus);
    let range = model.section_date_range(Section::Usage);
    assert_eq!(range.start().to_string(), "2026-09-03");
    assert_eq!(range.end().to_string(), "2026-09-09");
    step(&mut model, Intent::Select(Section::Usage));
    step(&mut model, Intent::Range);
    assert_eq!(
        model.section_date_range(Section::Usage).start().to_string(),
        "2026-08-11"
    );
    assert_eq!(model.model_name("gpt-5"), "GPT-5");
    assert_eq!(model.model_name("gpt-6"), "gpt-6");
}

#[test]
fn the_task_metric_cycles_in_the_model() {
    use codex_tui_core::screens::usage::chats::{
        Amounts, Decimal, Metric, Status, Task, TaskList, TaskRow,
    };
    let (mut model, _) = opened(PlanType::Plus);
    // The metrics offered are the ones some row reports: the balance, then
    // the percentages the rows have.
    model.section = Section::Chats;
    model.tasks = codex_tui_core::screens::usage::Load::Ready(std::rc::Rc::new(TaskList {
        rows: vec![TaskRow {
            title: "A task".into(),
            task: Some(Task {
                status: Status::Available,
                amounts: Amounts {
                    weekly_percent: Some(1.0),
                    five_hour_percent: Some(2.0),
                    balance: Some(Decimal::new("3")),
                },
                groups: Vec::new(),
            }),
        }],
        ..TaskList::default()
    }));
    assert_eq!(model.task_metrics(), Metric::ALL.to_vec());
    step(&mut model, Intent::TaskMetric);
    step(&mut model, Intent::TaskMetric);
    assert_eq!(model.chat_metric, 2);
    step(&mut model, Intent::TaskMetric);
    assert_eq!(model.chat_metric, 0);
}

/// The account's clock preference settles with the account (oracle:
/// `ClockFormat`, read once when the page opens): the report times and the
/// dashboard's hour column follow it.
#[test]
fn the_account_settles_the_clock_format() {
    let mut model = Model::default();
    assert!(!model.hour24);
    for hour24 in [true, false] {
        let asks = step(&mut model, Intent::Open { view: None });
        settle(
            &mut model,
            Ev::Account {
                session: asks[0].session,
                stamp: asks[0].stamp,
                result: Ok(AccountReply {
                    hour24,
                    ..reply(PlanType::Plus)
                }),
            },
        );
        assert_eq!(model.hour24, hour24);
    }
}

#[test]
fn the_asks_go_to_the_host_as_usage_commands_and_the_answers_come_back_through_the_app() {
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent as AppIntent, apply_app};
    use codex_tui_core::screens::{ScreenEv, ScreenIntent};

    let mut app = CodexApp::new();
    app.attach();
    let mut fx = AppFx::default();
    apply_app(
        &mut app,
        AppEv::Intent(AppIntent::Screen(ScreenIntent::Usage(Intent::Open {
            view: None,
        }))),
        &mut fx,
    );
    let asks = fx
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Usage { ask, .. } => Some(ask.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(asks.len(), 1, "the account first: {:?}", fx.commands());
    assert_eq!(asks[0].what, AskKind::Account);

    // The account's answer, through the app, starts the reports.
    let mut fx = AppFx::default();
    apply_app(
        &mut app,
        AppEv::Screen(ScreenEv::Usage(Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Ok(reply(PlanType::Plus)),
        })),
        &mut fx,
    );
    let later = fx
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::Usage { .. }))
        .count();
    assert!(later > 1, "the reports are asked: {:?}", fx.commands());
}
