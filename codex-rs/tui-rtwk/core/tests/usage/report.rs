//! What a chart report decides before it has words (oracle: `analytics/chart.rs`
//! `history_lines` and the height rule of `render.rs`), on the Credits fixture
//! of `analytics_maximized_refunds_leave_room_for_both_bands`.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::usage::report::{
    Body, Head, Label, Left, Message, ReportView, Right, Summary, Unit,
};
use codex_tui_core::screens::usage::{GroupName, Model, Section, UsageChoice, View};
use pretty_assertions::assert_eq;

fn days(refund: bool) -> String {
    let weights = [35, 36, 17, 12];
    let names = ["Desktop app", "CLI", "IDE extension", "Web"];
    [0, 41, 57, 85, 75, 109, 64]
        .iter()
        .enumerate()
        .map(|(index, total)| {
            let mut shares: Vec<i32> = weights.iter().map(|w| total * w / 100).collect();
            shares[3] = total - shares[..3].iter().sum::<i32>();
            let (total, values) = if refund && index == 6 {
                (
                    "-0.000004".to_owned(),
                    r#"[{"key":"CLI","label":"CLI","value":-0.000004}]"#.to_owned(),
                )
            } else {
                let values = names
                    .iter()
                    .zip(&shares)
                    .map(|(name, value)| {
                        format!(r#"{{"key":"{name}","label":"{name}","value":{value}.0}}"#)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                (format!("{total}.0"), format!("[{values}]"))
            };
            let date = [
                "2026-08-27",
                "2026-08-28",
                "2026-08-29",
                "2026-08-30",
                "2026-08-31",
                "2026-09-01",
                "2026-09-02",
            ][index];
            format!(r#"{{"date":"{date}","total":{total},"values":{values}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn model(refund: bool) -> (Model, UsageChoice) {
    let data = days(refund);
    let script = format!(
        r#"{{"step":"seed","seed":"session","thread":"thread-1"}}
{{"step":"screen","dir":"usage","ev":{{"account":{{"plan":"enterprise_cbp_usage_based","today":"2026-09-02"}},"section":"credits","histories":[{{"section":"credits","load":{{"ready":{{"unit":"credits","updated_at":null,"data":[{data}]}}}}}}]}}}}
"#
    );
    let model = Sandbox::play(&script)
        .expect("plays")
        .app()
        .screens()
        .usage
        .clone();
    let sel = UsageChoice::seeded(model.seeded.as_ref().expect("seeded"), &model);
    (model, sel)
}

fn label(date: codex_tui_core::screens::usage::NaiveDate, month: bool) -> String {
    let (_, m, d) = date.ymd();
    let name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
    ][m as usize - 1];
    if month {
        format!("{name} {d}")
    } else {
        d.to_string()
    }
}

fn view(cursor: usize, viewport: usize) -> ReportView {
    ReportView {
        cursor,
        detail: None,
        zoomed: true,
        viewport,
    }
}

#[test]
fn the_head_the_summary_and_the_columns_of_the_focused_credits_report() {
    let (model, sel) = model(true);
    let model = View::new(&model, &sel);
    let report = model
        .focused(Section::Credits, view(6, 25), 96, &label)
        .expect("ready");
    assert_eq!(
        report.head,
        Head::Total {
            amount: "367.00".into(),
            unit: Unit::Credits,
            reported: false
        }
    );
    assert_eq!(
        report.table.summary,
        Summary::Amount {
            amount: "-0.000004".into(),
            unit: Unit::Credits
        }
    );
    assert_eq!(
        report.table.header,
        Some((Left::Group(GroupName::Product), Right::Credits))
    );
    assert_eq!(report.table.message, None);
    let names: Vec<_> = report
        .table
        .rows
        .iter()
        .map(|row| row.label.clone())
        .collect();
    assert_eq!(names[0], Label::Name("CLI".into()));
    assert!(matches!(report.body, Body::Plot(ref plot) if plot.bands == 2));
}

#[test]
fn a_day_without_numbers_says_so_and_the_report_keeps_its_rows() {
    let (model, sel) = model(false);
    let model = View::new(&model, &sel);
    let first = model
        .focused(Section::Credits, view(0, 25), 96, &label)
        .expect("ready");
    assert_eq!(first.table.summary, Summary::NoActivity);
    // The counted reports replace the breakdown by a message; the credits one does not.
    assert_eq!(first.table.message, None);
    let last = model
        .focused(Section::Credits, view(6, 25), 96, &label)
        .expect("ready");
    assert_eq!(first.line_count(), last.line_count());
    assert!(!model.day_has_details(Section::Credits, 0));
    assert!(model.day_has_details(Section::Credits, 6));
    assert_eq!(Message::NotReported, Message::NotReported);
}

#[test]
fn the_plot_needs_fifteen_rows_and_fifty_cells_and_the_table_sits_beside_it_from_140() {
    let (model, sel) = model(false);
    let model = View::new(&model, &sel);
    let short = model
        .focused(Section::Credits, view(6, 14), 96, &label)
        .expect("ready");
    assert!(
        matches!(short.body, Body::Empty(_) | Body::Hidden),
        "{:?}",
        short.body
    );
    let narrow = model
        .focused(Section::Credits, view(6, 30), 49, &label)
        .expect("ready");
    assert!(matches!(narrow.body, Body::Hidden | Body::Empty(_)));
    let wide = model
        .focused(Section::Credits, view(6, 30), 140, &label)
        .expect("ready");
    assert!(wide.side_by_side);
    assert_eq!((wide.plot_width, wide.table_width), (92, 44));
    let normal = model
        .focused(Section::Credits, view(6, 30), 139, &label)
        .expect("ready");
    assert!(!normal.side_by_side);
    assert_eq!(normal.table_width, 56);
}
