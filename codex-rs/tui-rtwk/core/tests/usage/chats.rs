//! The Chats report (oracle: `analytics/chat_panel_tests.rs`,
//! `consumer_refresh_tests.rs`, `chats_tests.rs`): which rows and metrics a
//! listing offers, how the cursor and the open row move, what the lines of a
//! report hold, and when the page asks for the listing.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::rc::Rc;

use codex_protocol::account::PlanType;
use codex_tui_core::screens::usage::chats::{
    Amount, ChatLine, ChatList, ChatStep, Metric, Pos as ChatPos, TaskLine, TaskList, Title,
    balance_text,
};
use codex_tui_core::screens::usage::{
    AccountReply, Ask, AskKind, Ev, Intent, Load, Model, NaiveDate, Section, settle, step,
};
use pretty_assertions::assert_eq;
use serde_json::json;

fn opened(plan: PlanType) -> (Model, Vec<Ask>) {
    let mut model = Model::default();
    let asks = step(&mut model, Intent::Open { view: None });
    let asks = settle(
        &mut model,
        Ev::Account {
            session: asks[0].session,
            stamp: asks[0].stamp,
            result: Ok(AccountReply {
                plan: Some(plan),
                label: None,
                today: NaiveDate::from_ymd_opt(2026, 9, 9).unwrap(),
                plan_enabled: false,
                model_names: Vec::new(),
                credit_groups: vec![0, 2],
                hour24: false,
            }),
        },
    );
    (model, asks)
}

fn chats(value: serde_json::Value) -> ChatList {
    serde_json::from_value(json!({ "rows": value })).unwrap()
}

fn tasks(value: serde_json::Value) -> TaskList {
    serde_json::from_value(json!({ "rows": value })).unwrap()
}

fn group(model: &str, credits: i64) -> serde_json::Value {
    json!({"model":model,"effort":"high","speed":"standard","credits_micros":credits})
}

/// The balance a row shows is the cent-rounded decimal, a tiny or huge one
/// as the backend wrote it (oracle: `consumer_chats_keep_missing_rows_and_
/// only_offer_known_metrics`, the table of raw balances).
#[test]
fn a_balance_is_rounded_to_cents_and_kept_where_it_cannot_be() {
    for (raw, expected) in [
        ("0E-10", "0.00"),
        ("0.0000000000", "0.00"),
        ("-0e+10", "0.00"),
        ("1e-1000", "1e-1000"),
        ("-1e-1000", "-1e-1000"),
        ("1e1000", "1e1000"),
        ("12.3", "12.30"),
        ("12.3400000000", "12.34"),
        ("12.346", "12.35"),
        ("2.675", "2.68"),
        ("-2.675", "-2.68"),
        ("1.005", "1.01"),
        ("1.0049999999", "1.00"),
        ("9.999", "10.00"),
        ("0012.3", "12.30"),
        ("1.23e2", "123.00"),
        ("1234e-2", "12.34"),
        ("9007199254740993", "9007199254740993"),
        ("999999999999999999", "999999999999999999"),
        ("141436", "141,436.00"),
        ("0.01", "0.01"),
        ("-0.000004", "-0.000004"),
        ("1e-7", "1e-7"),
    ] {
        assert_eq!(balance_text(raw), expected, "{raw}");
    }
}

/// Rows the backend gave nothing for stay in the list, last; only the
/// metrics some row reports are offered (oracle: `consumer_chats_keep_missing_
/// rows_and_only_offer_known_metrics`).
#[test]
fn consumer_rows_keep_the_missing_ones_last_and_offer_known_metrics() {
    let (mut model, _) = opened(PlanType::Plus);
    model.section = Section::Chats;
    model.tasks = Load::Ready(Rc::new(tasks(json!([
        {"title":"Private title from another account"},
        {"title":"Unverified backend row","task":{"status":"unavailable"}},
        {"title":"Refunded task","task":{"status":"partial","amounts":{"balance":"-0.000004"},
            "groups":[{"model":"gpt-5.5","effort":"high","speed":"fast","product":"codex",
                       "amounts":{"balance":"-0.000004"}}]}},
    ]))));
    assert_eq!(model.task_metrics(), vec![Metric::Balance]);
    let reported = model
        .task_rows()
        .iter()
        .map(|row| row.available().is_some())
        .collect::<Vec<_>>();
    assert_eq!(reported, vec![true, false, false]);
    // The rows without usage open nothing; the one with usage does.
    let pos = ChatPos {
        cursor: 1,
        ..ChatPos::default()
    };
    assert!(!model.chat_has_details(&pos));
    assert!(model.chat_has_details(&ChatPos::default()));
    let lines = model.task_view(&ChatPos::default(), true);
    let hidden = lines.iter().filter_map(|line| match line {
        TaskLine::Row { title, .. } => Some(title.clone()),
        _ => None,
    });
    assert_eq!(
        hidden.collect::<Vec<_>>(),
        vec![
            Title::Text("Refunded task".into()),
            Title::Unavailable,
            Title::Unavailable
        ]
    );
}

/// The sort metric moves the rows; a percentage not reported is last
/// (oracle: `compare` over the metric).
#[test]
fn the_sort_metric_orders_the_rows_and_a_missing_value_goes_last() {
    let (mut model, _) = opened(PlanType::Plus);
    model.section = Section::Chats;
    model.tasks = Load::Ready(Rc::new(tasks(json!([
        {"title":"low","task":{"status":"available","amounts":{"weekly_percent":1.0,"balance":"9"}}},
        {"title":"none","task":{"status":"available","amounts":{"balance":"5"}}},
        {"title":"high","task":{"status":"available","amounts":{"weekly_percent":50.5,"balance":"1"}}},
    ]))));
    assert_eq!(model.task_metrics(), vec![Metric::Weekly, Metric::Balance]);
    let order = |model: &Model| {
        model
            .task_rows()
            .iter()
            .map(|row| row.available().and_then(|a| a.shown(model.task_metric())))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        order(&model),
        vec![Some("50.5%".into()), Some("1.0%".into()), None]
    );
    step(&mut model, Intent::TaskMetric);
    assert_eq!(model.task_metric(), Metric::Balance);
    assert_eq!(
        order(&model),
        vec![
            Some("9.00".into()),
            Some("5.00".into()),
            Some("1.00".into())
        ]
    );
}

/// Moving the cursor closes the open row; Esc closes the open row first and
/// the page after; a row with nothing to open stays shut (oracle: `handle_key`
/// for the chats, `unavailable_chats_do_not_open_invisible_details`).
#[test]
fn the_cursor_and_the_open_row_move_together() {
    let mut pos = ChatPos::default();
    assert!(!pos.step(ChatStep::Toggle, 5, false));
    assert_eq!(pos.detail, None);
    assert!(!pos.step(ChatStep::Toggle, 5, true));
    assert_eq!(pos.detail, Some(0));
    assert!(!pos.step(ChatStep::Down, 5, true));
    assert_eq!((pos.cursor, pos.detail), (1, None));
    pos.step(ChatStep::Bottom, 5, true);
    assert_eq!(pos.cursor, 4);
    pos.step(ChatStep::Down, 5, true);
    assert_eq!(pos.cursor, 4);
    pos.step(ChatStep::Toggle, 5, true);
    assert!(!pos.step(ChatStep::Collapse, 5, true));
    assert_eq!(pos.detail, None);
    pos.step(ChatStep::Toggle, 5, true);
    assert!(!pos.step(ChatStep::Back, 5, true));
    assert!(pos.step(ChatStep::Back, 5, true));
    // A listing that shrank keeps the cursor on its last row.
    pos.cursor = 9;
    assert_eq!(pos.at(3), 2);
    pos.step(ChatStep::Top, 0, false);
    assert_eq!(pos.cursor, 0);
}

/// A tap picks the row, the next tap opens it, the one after closes it.
#[test]
fn a_tap_picks_then_opens_then_closes() {
    let mut pos = ChatPos::default();
    pos.pick(2, 5, true);
    assert_eq!((pos.cursor, pos.detail), (2, None));
    pos.pick(2, 5, true);
    assert_eq!(pos.detail, Some(2));
    pos.pick(2, 5, true);
    assert_eq!(pos.detail, None);
    pos.pick(9, 5, true);
    assert_eq!(pos.cursor, 2);
}

fn business() -> Model {
    let (mut model, _) = opened(PlanType::EnterpriseCbpUsageBased);
    model.section = Section::Chats;
    model
}

/// The open chat's groups by credits, the zero ones hidden and counted until
/// `a` shows them (oracle: `chats_breakdown_ranks_nonzero_groups_and_reflows`).
#[test]
fn an_open_chat_ranks_its_groups_and_hides_the_zero_ones() {
    let mut model = business();
    model.chats = Load::Ready(Rc::new(chats(json!([
        {"title":"Q3","estimate":{"credits_micros":19_350_041_551_i64,"groups":[
            group("Zero", 0), group("Small", 215_610_920), group("Top", 19_134_430_635), group("Refund", -4)]}},
        {"title":"Gone"},
    ]))));
    let mut pos = ChatPos {
        cursor: 0,
        detail: Some(0),
        zero_groups: false,
    };
    let models = |lines: &[ChatLine]| {
        lines
            .iter()
            .filter_map(|line| match line {
                ChatLine::Group { model, .. } => model.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let lines = model.chat_view(&pos);
    assert_eq!(models(&lines), ["Top", "Small", "Refund"]);
    assert!(lines.contains(&ChatLine::Zeros {
        count: 1,
        shown: false
    }));
    assert!(lines.contains(&ChatLine::Missing(1)));
    pos.zero_groups = true;
    let lines = model.chat_view(&pos);
    assert_eq!(models(&lines), ["Top", "Small", "Zero", "Refund"]);
    assert!(lines.contains(&ChatLine::Zeros {
        count: 1,
        shown: true
    }));
    // No row opens an estimate that is not there.
    assert!(!model.chat_has_details(&ChatPos {
        cursor: 1,
        ..ChatPos::default()
    }));
}

/// The dollar column is there once some chat has one; a chat without an
/// estimate keeps a dash and the title of the safe text.
#[test]
fn the_dollar_column_follows_the_estimates() {
    let mut model = business();
    model.chats = Load::Ready(Rc::new(chats(json!([
        {"title":"A","estimate":{"credits_micros":1_000_000,"usd_micros":2_500_000}},
        {"title":"B"},
    ]))));
    let lines = model.chat_view(&ChatPos::default());
    assert!(lines.contains(&ChatLine::Head { usd: true }));
    let amounts = lines
        .iter()
        .filter_map(|line| match line {
            ChatLine::Row { amount, title, .. } => Some((title.clone(), amount.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        amounts,
        vec![
            (
                Title::Text("A".into()),
                Amount::Both {
                    usd: Some("$2.50".into()),
                    credits: Some("1.00".into())
                }
            ),
            (
                Title::Unavailable,
                Amount::Both {
                    usd: None,
                    credits: None
                }
            ),
        ]
    );
}

/// The card keeps the first five rows and says when an estimate is missing
/// (oracle: the dashboard's Chats card).
#[test]
fn the_business_card_keeps_five_rows() {
    let mut model = business();
    model.chats = Load::Ready(Rc::new(chats(json!(
        (0..8)
            .map(|i| json!({"title":format!("Chat {i}"),"estimate":{"credits_micros":i*1_000_000}}))
            .chain([json!({"title":"none"})])
            .collect::<Vec<_>>()
    ))));
    let lines = model.chat_card();
    let rows = lines
        .iter()
        .filter(|line| matches!(line, ChatLine::CardRow { .. }))
        .count();
    assert_eq!(rows, 5);
    assert_eq!(lines.last(), Some(&ChatLine::CardMissing));
}

/// Opening the page asks for the listing of the account's kind, and a reply
/// of an older stamp or session is dropped (oracle: `load_chats`,
/// `load_tasks`).
#[test]
fn the_page_asks_for_the_listing_of_its_account_kind() {
    let (model, asks) = opened(PlanType::EnterpriseCbpUsageBased);
    assert!(
        asks.iter().any(|ask| ask.what == AskKind::Chats),
        "{asks:?}"
    );
    assert!(!asks.iter().any(|ask| ask.what == AskKind::Tasks));
    let (mut model_c, asks) = opened(PlanType::Plus);
    assert!(
        asks.iter().any(|ask| ask.what == AskKind::Tasks),
        "{asks:?}"
    );
    assert!(!asks.iter().any(|ask| ask.what == AskKind::Chats));
    let ask = asks.iter().find(|ask| ask.what == AskKind::Tasks).unwrap();
    // A stale stamp does not settle the load.
    settle(
        &mut model_c,
        Ev::Tasks {
            session: ask.session,
            stamp: codex_tui_core::screens::usage::Stamp::default(),
            result: Ok(Some(TaskList::default())),
        },
    );
    assert!(model_c.tasks.ready().is_none());
    settle(
        &mut model_c,
        Ev::Tasks {
            session: ask.session,
            stamp: ask.stamp,
            result: Ok(Some(TaskList::default())),
        },
    );
    assert!(model_c.tasks.ready().is_some());
    drop(model);
}
