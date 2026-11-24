//! The plan history (oracle: `analytics/plan_tests.rs`, `plan.rs`): what a
//! snapshot is made of, what it refuses with a safe text, and when the page
//! asks for it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_protocol::account::PlanType;
use codex_tui_core::screens::usage::plan::{
    self, Data, Dimension, Line, Note, PlanStep, Pos, Report,
};
use codex_tui_core::screens::usage::{
    AccountReply, Ask, AskKind, Ev, Intent, Load, Model, Note as LoadNote, Section, UsageChoice, View, settle, step,
};
use pretty_assertions::assert_eq;
use serde_json::json;

fn response() -> Data {
    serde_json::from_value(json!({
        "data_as_of":"2026-09-02T12:00:00Z", "coverage_start":"2026-09-01T00:00:00Z",
        "coverage_complete":false, "boundary_tolerance_seconds":60,
        "periods":[
            {"id":"five", "window_minutes":300,"plan_type":"plus",
             "starts_at":"2026-09-02T10:00:00Z","ends_at":"2026-09-02T15:00:00Z",
             "accounting_complete":false,"used_basis_points":12500,
             "breakdowns":[
                {"dimension":"future_dimension","rows":[{"key":"future","basis_points":500}]},
                {"dimension":"thread_source","rows":[{"key":"user","basis_points":12500}]}]},
            {"id":"weekly", "window_minutes":10080,"plan_type":"plus",
             "starts_at":"2026-09-01T00:00:00Z","ends_at":"2026-09-08T00:00:00Z",
             "accounting_complete":true,"used_basis_points":null,"breakdowns":null}
        ]
    }))
    .unwrap()
}

#[test]
fn a_snapshot_splits_into_the_two_windows_newest_first() {
    let mut data = response();
    let mut older = data.periods[0].clone();
    older.id = "older".into();
    older.starts_at = "2026-09-02T05:00:00Z".into();
    older.ends_at = "2026-09-02T10:00:00Z".into();
    data.periods.push(older);
    let report = Report::parse(data).unwrap().unwrap();
    let ids = |window: usize| {
        report.periods[window]
            .iter()
            .map(|period| period.id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(0), ["five", "older"]);
    assert_eq!(ids(1), ["weekly"]);
    // The period in progress at the snapshot is the one `*` marks.
    assert!(report.is_current(&report.periods[0][0]));
    assert!(!report.is_current(&report.periods[0][1]));
    // Unknown accounting stays unknown, an unknown dimension stays unlisted.
    assert_eq!(report.periods[1][0].used, None);
    assert!(!report.periods[0][0].complete);
    let dimensions: Vec<Dimension> = report.periods[0][0]
        .breakdowns
        .iter()
        .flatten()
        .map(|group| group.dimension)
        .collect();
    assert_eq!(dimensions, [Dimension::Unknown, Dimension::ThreadSource]);
}

#[test]
fn a_history_without_a_snapshot_time_is_no_report() {
    let mut missing = response();
    missing.data_as_of = None;
    assert!(Report::parse(missing).unwrap().is_none());
}

#[test]
fn each_invalid_history_names_what_was_wrong() {
    type Change = fn(&mut Data);
    let cases: [(Change, plan::Error); 6] = [
        (
            |history| history.periods[0].starts_at = "private wire value".into(),
            plan::Error::InvalidTimestamp,
        ),
        (
            |history| history.data_as_of = Some("private wire value".into()),
            plan::Error::InvalidTimestamp,
        ),
        (
            |history| history.periods[0].ends_at = history.periods[0].starts_at.clone(),
            plan::Error::InvalidPeriod,
        ),
        (
            |history| history.periods.push(history.periods[0].clone()),
            plan::Error::InvalidPeriod,
        ),
        (
            |history| history.periods[0].window_minutes = 1,
            plan::Error::UnsupportedWindow,
        ),
        (
            |history| history.periods[0].used_basis_points = Some(f64::INFINITY),
            plan::Error::InvalidPeriod,
        ),
    ];
    for (change, expected) in cases {
        let mut history = response();
        change(&mut history);
        assert_eq!(Report::parse(history).err(), Some(expected));
    }
}

#[test]
fn a_history_of_too_many_periods_is_refused() {
    let mut history = response();
    let period = history.periods[0].clone();
    history.periods = vec![period; 1_001];
    assert_eq!(
        Report::parse(history).err(),
        Some(plan::Error::TooManyPeriods)
    );
}

/// The plan's lines as the page draws them, over the default choice.
fn plan_view(model: &Model, pos: &Pos, zoomed: bool) -> codex_tui_core::screens::usage::plan::PlanView {
    let mut sel = UsageChoice::opening(None);
    sel.settled(model);
    View::new(model, &sel).plan_view(pos, zoomed)
}

fn consumer(plan_enabled: bool, plan: PlanType) -> (Model, Vec<Ask>) {
    let mut model = Model::default();
    let asks = step(&mut model, Intent::Open);
    let asks = settle(
        &mut model,
        Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Ok(AccountReply {
                plan: Some(plan),
                label: None,
                today: codex_tui_core::screens::usage::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap(),
                plan_enabled,
                model_names: Vec::new(),
                credit_groups: Vec::new(),
                hour24: true,
            }),
        },
    );
    (model, asks)
}

/// The gate: only a consumer account with the feature on shows the Plan and
/// asks for it (oracle: `plan_gate_prevents_requests...`).
#[test]
fn the_plan_is_asked_only_where_it_is_shown() {
    let asks_plan = |asks: &[Ask]| asks.iter().any(|ask| ask.what == AskKind::Plan);
    let (model, asks) = consumer(true, PlanType::Plus);
    assert!(model.visible_sections().contains(&Section::Plan));
    assert!(asks_plan(&asks));
    assert!(matches!(model.plan, Load::Loading(_)));
    let (model, asks) = consumer(false, PlanType::Plus);
    assert!(!model.visible_sections().contains(&Section::Plan));
    assert!(!asks_plan(&asks));
    assert!(matches!(model.plan, Load::Unavailable));
    let (model, asks) = consumer(true, PlanType::Business);
    assert!(!model.visible_sections().contains(&Section::Plan));
    assert!(!asks_plan(&asks));
    assert!(matches!(model.plan, Load::Unavailable));
}

#[test]
fn the_reply_settles_the_load_of_its_stamp_only() {
    let (mut model, asks) = consumer(true, PlanType::Plus);
    let ask = asks.iter().find(|ask| ask.what == AskKind::Plan).unwrap();
    // A reply under another stamp, or of an invalid history, does not show.
    settle(
        &mut model,
        Ev::Plan {
            session: ask.session,
            stamp: codex_tui_core::screens::usage::Stamp(ask.stamp.0 + 100),
            result: Ok(Some(response())),
        },
    );
    assert!(matches!(model.plan, Load::Loading(_)));
    let mut invalid = response();
    invalid.periods[0].window_minutes = 1;
    let mut other = model.clone();
    settle(
        &mut other,
        Ev::Plan {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(invalid)),
        },
    );
    assert_eq!(
        other.plan.note(),
        Some(LoadNote::Error(plan::Error::UnsupportedWindow.into()))
    );
    settle(
        &mut model,
        Ev::Plan {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(response())),
        },
    );
    let report = model.plan.ready().expect("ready");
    assert_eq!(report.periods[0].len() + report.periods[1].len(), 2);
    // No snapshot time: nothing to show, not an error.
    let (mut model, asks) = consumer(true, PlanType::Plus);
    let ask = asks.iter().find(|ask| ask.what == AskKind::Plan).unwrap();
    let mut empty = response();
    empty.data_as_of = None;
    settle(
        &mut model,
        Ev::Plan {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(empty)),
        },
    );
    assert!(matches!(model.plan, Load::Unavailable));
}

/// A refresh or a close drops the history with the other account-owned loads.
#[test]
fn a_refresh_and_a_close_drop_the_history() {
    let (mut model, asks) = consumer(true, PlanType::Plus);
    let ask = asks.iter().find(|ask| ask.what == AskKind::Plan).unwrap();
    settle(
        &mut model,
        Ev::Plan {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(response())),
        },
    );
    assert!(model.plan.ready().is_some());
    step(&mut model, Intent::Refresh);
    assert!(matches!(model.plan, Load::Unavailable));
    step(&mut model, Intent::Leave);
    assert!(matches!(model.plan, Load::Unavailable));
}

fn ready(history: Data) -> Model {
    let (mut model, asks) = consumer(true, PlanType::Plus);
    let ask = asks.iter().find(|ask| ask.what == AskKind::Plan).unwrap();
    settle(
        &mut model,
        Ev::Plan {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(history)),
        },
    );
    model
}

fn several() -> Data {
    let mut data = response();
    let template = data.periods[0].clone();
    for index in 0..3 {
        let mut older = template.clone();
        older.id = format!("older-{index}");
        older.starts_at = format!("2026-09-02T0{}:00:00Z", 4 - index);
        older.ends_at = format!("2026-09-02T0{}:00:00Z", 9 - index);
        data.periods.push(older);
    }
    data
}

/// Up, down, home and end move the row of the table in front, the left and
/// right arrows put the other table in front, Enter opens the row and
/// moving off it closes it (oracle: `plan_action`).
#[test]
fn the_keys_move_in_the_table_in_front_and_open_its_row() {
    let model = ready(several());
    let report = model.plan.ready().map(|report| &**report);
    let mut pos = Pos::default();
    assert!(!pos.step(PlanStep::Down, report));
    assert!(!pos.step(PlanStep::Toggle, report));
    assert_eq!((pos.cursor, pos.expanded), ([1, 0], [true, false]));
    pos.step(PlanStep::Down, report);
    assert_eq!((pos.cursor, pos.expanded), ([2, 0], [false, false]));
    pos.step(PlanStep::Bottom, report);
    pos.step(PlanStep::Down, report);
    assert_eq!(pos.cursor, [3, 0], "stops at the last period");
    pos.step(PlanStep::Top, report);
    assert_eq!(pos.cursor, [0, 0]);
    pos.step(PlanStep::Up, report);
    assert_eq!(pos.cursor, [0, 0]);
    // The other table keeps its own row and its own open details.
    pos.step(PlanStep::Toggle, report);
    pos.step(PlanStep::Switch, report);
    assert_eq!(pos.window, 1);
    pos.step(PlanStep::Down, report);
    pos.step(PlanStep::Toggle, report);
    assert_eq!((pos.cursor, pos.expanded), ([0, 0], [true, true]));
}

/// Esc closes the open details first and the page when none is open.
#[test]
fn back_closes_the_details_before_the_page() {
    let model = ready(several());
    let report = model.plan.ready().map(|report| &**report);
    let mut pos = Pos::default();
    pos.step(PlanStep::Toggle, report);
    assert!(!pos.step(PlanStep::Back, report));
    assert_eq!(pos.expanded, [false, false]);
    assert!(pos.step(PlanStep::Back, report));
}

/// A snapshot of another time starts the rows over, the table in front stays
/// (oracle: `State::poll`, the last assertion of
/// `period_navigation_preserves_unknowns_and_resets_on_new_snapshot`).
#[test]
fn a_new_snapshot_starts_the_rows_over() {
    let first = ready(several());
    let mut pos = Pos::default();
    pos.step(PlanStep::Down, first.plan.ready().map(|report| &**report));
    pos.step(PlanStep::Toggle, first.plan.ready().map(|report| &**report));
    pos.step(PlanStep::Switch, first.plan.ready().map(|report| &**report));
    let mut next = several();
    next.data_as_of = Some("2026-09-02T13:00:00Z".into());
    let next = ready(next);
    let report = next.plan.ready().map(|report| &**report);
    // The view reads the rows as reset, and the next key makes it so.
    assert!(
        plan_view(&next, &pos, true).tables[0]
            .iter()
            .any(|line| matches!(
                line,
                Line::Period {
                    index: 0,
                    selected: true,
                    ..
                }
            ))
    );
    pos.step(PlanStep::Up, report);
    assert_eq!(
        (pos.window, pos.cursor, pos.expanded),
        (1, [0, 0], [false, false])
    );
}

/// A click picks the row; a click on the row under the cursor of the table
/// in front opens it, another closes it.
#[test]
fn a_click_picks_a_row_and_a_second_opens_it() {
    let model = ready(several());
    let report = model.plan.ready().map(|report| &**report);
    let mut pos = Pos::default();
    pos.pick(0, 2, report);
    assert_eq!(
        (pos.window, pos.cursor, pos.expanded),
        (0, [2, 0], [false, false])
    );
    pos.pick(0, 2, report);
    assert_eq!(pos.expanded, [true, false]);
    pos.pick(1, 0, report);
    assert_eq!(
        (pos.window, pos.cursor[1], pos.expanded),
        (1, 0, [true, false])
    );
    pos.pick(1, 0, report);
    assert_eq!(pos.expanded, [true, true]);
    pos.pick(0, 1, report);
    assert_eq!(
        (pos.window, pos.cursor[0], pos.expanded),
        (0, 1, [false, true])
    );
}

/// The card shows the first period of each table; the report all of them
/// and the details of the open one, split by what the group says.
#[test]
fn the_card_shows_one_period_and_the_report_all_with_the_open_details() {
    let model = ready(several());
    let mut pos = Pos::default();
    pos.step(PlanStep::Toggle, model.plan.ready().map(|report| &**report));
    let periods = |view: &plan::PlanView, window: usize| {
        view.tables[window]
            .iter()
            .filter(|line| matches!(line, Line::Period { .. }))
            .count()
    };
    let card = plan_view(&model, &pos, false);
    assert_eq!((periods(&card, 0), periods(&card, 1)), (1, 1));
    assert!(
        !card.tables[0].contains(&Line::Incomplete),
        "no details on a card"
    );
    let report = plan_view(&model, &pos, true);
    assert_eq!((periods(&report, 0), periods(&report, 1)), (4, 1));
    // Feature (thread source) is the Plan's group at first; the key is named.
    assert!(report.tables[0].contains(&Line::Incomplete));
    assert!(report.tables[0].contains(&Line::Share {
        label: "Tasks".into(),
        percent: "125".into()
    }));
    // The weekly period has no breakdown; open, it says so.
    pos.step(PlanStep::Switch, model.plan.ready().map(|report| &**report));
    pos.step(PlanStep::Toggle, model.plan.ready().map(|report| &**report));
    assert!(plan_view(&model, &pos, true).tables[1].contains(&Line::NoBreakdown));
}

/// The notes under the tables: the snapshot time and the coverage, the gaps
/// and the approximation; the turn-start group's own note.
#[test]
fn the_notes_say_what_the_numbers_are() {
    let model = ready(response());
    let notes = plan_view(&model, &Pos::default(), true).notes;
    assert_eq!(
        notes,
        [
            Note::Gaps,
            Note::AsOf(1_788_350_400),
            Note::Since(1_788_220_800),
            Note::Approximate
        ]
    );
    let mut sel = UsageChoice::opening(None);
    sel.settled(&model);
    sel.groups[Section::Plan as usize] = 3;
    assert_eq!(
        View::new(&model, &sel).plan_view(&Pos::default(), true).notes[0],
        Note::Turn
    );
    // Nothing is said of a history that is not shown.
    let empty = Model::default();
    assert_eq!(plan_view(&empty, &Pos::default(), true).notes, []);
    assert_eq!(
        plan_view(&empty, &Pos::default(), true).tables[0].last(),
        Some(&Line::Unavailable)
    );
}

/// Side by side from 100 cells, each table (width - 4) / 2 wide.
#[test]
fn the_tables_stand_side_by_side_from_a_hundred_cells() {
    assert!(!plan::side_by_side(99));
    assert!(plan::side_by_side(100));
    assert_eq!((plan::table_width(99), plan::table_width(100)), (99, 48));
    assert_eq!(plan::table_width(101), 48);
}
