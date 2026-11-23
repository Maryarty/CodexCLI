//! The lines of the review under the intro, top to bottom: the groups of the
//! summary and the entries of the checklist, told from the model's typed data.
//! Oracle: `external_agent_config_migration/render.rs`.

use codex_tui_core::screens::import::{Mark, Model};
use rtwk::l10n::NumSyms;

use super::View;
use super::labels;
use super::strings::Strings;

/// One line of the review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewRow {
    /// A group of the summary (`index` of the group).
    Group {
        index: usize,
        mark: Mark,
        label: String,
    },
    /// What an entry brings, under it.
    Detail(String),
    /// The objects the group imports (`None`: nothing is selected).
    Importing(Option<String>),
    /// The heading of the checklist: `None` the home, `Some` a project directory.
    Section(Option<String>),
    /// The blank row between two sections.
    Gap,
    /// An entry of the checklist (`index` of the item).
    Entry {
        index: usize,
        enabled: bool,
        label: &'static str,
    },
}

/// The lines of `view`.
pub fn of(
    view: View,
    model: &Model,
    checked: &[bool],
    strings: &Strings,
    num: &NumSyms,
) -> Vec<ReviewRow> {
    match view {
        View::Summary => summary(model, checked, strings, num),
        View::Customize => checklist(model, checked, strings, num),
    }
}

/// The index of the line of the cursor (an index of a group or an item).
pub fn cursor_row(rows: &[ReviewRow], at: Option<usize>) -> Option<usize> {
    let at = at?;
    rows.iter().position(|row| match row {
        ReviewRow::Group { index, .. } | ReviewRow::Entry { index, .. } => *index == at,
        _ => false,
    })
}

fn summary(model: &Model, checked: &[bool], strings: &Strings, num: &NumSyms) -> Vec<ReviewRow> {
    let mut rows = Vec::new();
    for (index, group) in model.groups.iter().enumerate() {
        let chosen = group
            .items
            .iter()
            .filter(|at| checked.get(**at).copied().unwrap_or(false))
            .filter_map(|at| model.items.get(*at));
        let summary = labels::count_summary(strings, chosen);
        rows.push(ReviewRow::Group {
            index,
            mark: Model::mark(group, checked),
            label: labels::group(strings, group, num),
        });
        rows.push(ReviewRow::Detail(
            labels::group_description(strings, group).to_owned(),
        ));
        rows.push(ReviewRow::Importing(
            (!summary.is_empty()).then_some(summary),
        ));
    }
    rows
}

fn checklist(model: &Model, checked: &[bool], strings: &Strings, num: &NumSyms) -> Vec<ReviewRow> {
    let mut rows = Vec::new();
    let mut scope: Option<Option<String>> = None;
    for (index, sel) in model.items.iter().enumerate() {
        let cwd = sel.cwd();
        if scope.as_ref() != Some(&cwd) {
            if scope.is_some() {
                rows.push(ReviewRow::Gap);
            }
            rows.push(ReviewRow::Section(cwd.clone()));
            scope = Some(cwd);
        }
        rows.push(ReviewRow::Entry {
            index,
            enabled: checked.get(index).copied().unwrap_or(false),
            label: labels::item(strings, sel.kind()),
        });
        rows.push(ReviewRow::Detail(labels::description(
            strings,
            sel,
            &model.places,
            num,
        )));
        if let Some(detail) = labels::detail(strings, sel, num) {
            rows.push(ReviewRow::Detail(detail));
        }
        rows.extend(
            labels::plugin_lines(strings, sel, num)
                .into_iter()
                .map(ReviewRow::Detail),
        );
    }
    rows
}
