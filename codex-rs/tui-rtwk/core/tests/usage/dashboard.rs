//! The dashboard's grid and the lead of a card (oracle: `analytics/dashboard.rs`,
//! `dashboard_cards_align_and_keep_stable_summary_heights`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::usage::dashboard::{Geometry, Lead};
use codex_tui_core::screens::usage::{Model, Section, UsageChoice, View};
use pretty_assertions::assert_eq;

fn account(plan: &str) -> (Model, UsageChoice) {
    let script = format!(
        r#"{{"step":"seed","seed":"session","thread":"thread-1"}}
{{"step":"screen","dir":"usage","ev":{{"account":{{"plan":"{plan}","today":"2026-09-02"}}}}}}
"#
    );
    let model = Sandbox::play(&script)
        .expect("plays")
        .app()
        .screens()
        .usage
        .clone();
    let mut sel = UsageChoice::opening(None);
    sel.settled(&model);
    (model, sel)
}

/// The oracle's 144 by 64 screen: the body is 142 by 57 once the page has
/// taken its inset, the heading, tabs, rule, controls and hints.
#[test]
fn six_cards_sit_two_to_a_row_at_eighteen_rows() {
    let geometry = Geometry::new(142, 57, 6);
    assert_eq!(
        (
            geometry.columns,
            geometry.card_width,
            geometry.rows,
            geometry.card_height,
            geometry.inner_width(),
            geometry.inner_height()
        ),
        (2, 69, 3, 18, 65, 14)
    );
}

#[test]
fn a_narrow_body_stacks_the_cards_and_keeps_sixteen_rows() {
    let geometry = Geometry::new(70, 41, 6);
    assert_eq!(
        (geometry.columns, geometry.card_width, geometry.rows),
        (1, 70, 6)
    );
    assert_eq!(geometry.card_height, 16);
    // The least room still gives every card a row of content.
    let tiny = Geometry::new(1, 1, 6);
    assert_eq!((tiny.card_width, tiny.inner_width()), (1, 1));
}

#[test]
fn the_height_is_clamped_and_never_depends_on_the_cards_content() {
    assert_eq!(Geometry::new(142, 200, 2).card_height, 24);
    assert_eq!(Geometry::new(142, 10, 6).card_height, 16);
    // No card at all still has a row.
    assert_eq!(Geometry::new(142, 57, 0).rows, 1);
}

#[test]
fn a_card_belongs_to_a_row_and_a_row_to_its_cards() {
    let geometry = Geometry::new(142, 57, 5);
    assert_eq!(
        (geometry.row_of(0), geometry.row_of(3), geometry.row_of(4)),
        (0, 1, 2)
    );
    assert_eq!(geometry.cards_of(1), 2..4);
    // The last row holds what is left.
    assert_eq!(geometry.cards_of(2), 4..5);
    let narrow = Geometry::new(70, 41, 5);
    assert_eq!((narrow.row_of(4), narrow.cards_of(4)), (4, 4..5));
}

#[test]
fn the_lead_names_the_range_the_model_and_the_grouping() {
    let (model, sel) = account("plus");
    let consumer = (&model, &sel);
    let lead = |(model, sel): (&Model, &UsageChoice), section| View::new(model, sel).card_lead(section);
    assert_eq!(
        lead(consumer, Section::Usage),
        Lead {
            range: Some(false),
            model: None,
            group: Some(sel.groups[Section::Usage as usize])
        }
    );
    assert_eq!(
        lead(consumer, Section::Plugins),
        Lead {
            range: Some(false),
            model: None,
            group: None
        }
    );
    for section in [Section::Summary, Section::Chats] {
        assert_eq!(
            lead(consumer, section),
            Lead {
                range: None,
                model: None,
                group: None
            }
        );
    }
    assert_eq!(lead(consumer, Section::Plan).range, None);
    assert!(lead(consumer, Section::Plan).group.is_some());
}

#[test]
fn a_business_token_card_carries_the_model_filter() {
    let (model, mut sel) = account("enterprise_cbp_usage_based");
    let lead = View::new(&model, &sel).card_lead(Section::Usage);
    assert_eq!((lead.model.clone(), lead.range), (Some(None), Some(false)));
    sel.token_model = Some("gpt-5".into());
    let business = View::new(&model, &sel);
    assert_eq!(
        business.card_lead(Section::Usage).model,
        Some(Some("gpt-5".to_owned()))
    );
    // Only the token report has a model.
    assert_eq!(business.card_lead(Section::Activity).model, None);
}

#[test]
fn the_long_range_shows_on_the_cards_of_its_group() {
    let (model, mut sel) = account("plus");
    let group = model.range_group(Section::Plugins) as usize;
    sel.ranges[group] = 1;
    let view = View::new(&model, &sel);
    assert_eq!(view.card_lead(Section::Plugins).range, Some(true));
    assert_eq!(view.card_lead(Section::Skills).range, Some(true));
    assert_eq!(view.card_lead(Section::Activity).range, Some(false));
}

#[test]
fn the_plot_keeps_one_row_in_the_least_card_and_grows_with_the_tallest() {
    // 16 rows of card: 12 of content, 13 go to the rest of the report.
    assert_eq!(Geometry::new(142, 10, 6).plot_rows(), 1);
    // 24 rows of card: 20 of content.
    assert_eq!(Geometry::new(142, 200, 2).plot_rows(), 7);
    // The oracle's 18-row cards (the snapshot's one-row plot).
    assert_eq!(Geometry::new(142, 57, 6).plot_rows(), 1);
}
