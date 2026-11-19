//! The page header: title, filter strip, heading rows.

use super::list::grouping_name;
use super::*;

/// The three header rows (oracle: `render`): title and grouping, the tab
/// strip with the filter hint, the rule.
pub fn header(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    model: &Model,
    ui: &Ui,
    center: &Center,
    taps: &Taps,
) -> AnyWidget {
    let title = one(vec![
        span(strings.header, &look.bold),
        span(
            strings.group(
                grouping_name(strings, model.grouping),
                strings.grouping_key,
                num,
            ),
            &look.dim,
        ),
    ]);
    let labels: Rc<[String]> = strings
        .filters
        .iter()
        .zip(center.tab_counts)
        .map(|(label, count)| strings.filter_count(label, count, num))
        .collect();
    let strip = Tabs {
        labels,
        active: ui.filter,
        marks: (strings.tab_left, strings.tab_right),
        styles: (look.tab.clone(), look.dim.clone()),
        taps: taps.tabs.clone(),
    }
    .widget();
    let hint = (!ui.editing()).then(|| {
        one(vec![
            span(strings.filter_keys, &look.plain),
            span(format!(" {}", strings.filter_label), &look.muted),
        ])
    });
    let tabs = LayoutBuilder::new(move |_, constraints| {
        let mut row = Row::new().child(Expanded::new(1, strip.clone()));
        if let Some(hint) = hint.clone().filter(|_| constraints.max_width >= KEYS_FROM) {
            row = row.child(gap(2)).child(hint);
        }
        row.into()
    });
    let rule = Divider::new()
        .kind(BorderType::Single)
        .style(look.dim.clone())
        .merge(false);
    inset(stacked(vec![
        title,
        SizedBox::new(None, Some(1), Some(tabs.into())).into(),
        rule.into(),
    ]))
}

/// A heading of the list (oracle: `render_center_rows`, the `Group` arm);
/// `width` is the room the row has.
pub(super) fn heading(
    seg: Measure,
    look: &Look,
    strings: &Strings,
    model: &Model,
    label: &GroupLabel,
    count: &str,
    width: usize,
) -> AnyWidget {
    let text = match label {
        GroupLabel::Status(group) => match group {
            Group::NeedsYou => strings.needs_input,
            Group::Working => strings.working,
            Group::Ready => strings.ready,
            Group::Finished => strings.inactive,
        }
        .to_owned(),
        GroupLabel::Pinned => strings.pinned.to_owned(),
        GroupLabel::Model(name) => name.clone(),
        GroupLabel::Path(path) => path::fit_heading(
            seg,
            &display_directory(path, model.home.as_deref()),
            count,
            width,
        ),
    };
    one(vec![span(format!("{text}  {count}"), &look.dim)])
}

/// The label of the "Show more" row (oracle: `render_center_rows`).
pub(super) fn show_more<'a>(strings: &'a Strings, model: &Model) -> &'a str {
    if let Some(history) = &model.history {
        if history.loading {
            strings.searching_history
        } else if history.failed {
            strings.retry
        } else {
            strings.show_more_matches
        }
    } else if model.paging.loading {
        strings.loading_more
    } else if model.paging.failed {
        strings.retry
    } else {
        strings.show_more
    }
}
