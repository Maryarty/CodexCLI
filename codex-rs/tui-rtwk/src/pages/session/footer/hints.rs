//! Which spelling of each passive hint the footer row holds (the oracle's
//! `chat_composer/passive_hints.rs`: `Hints::fit`, `desired_rows`,
//! `placement`). The widths come measured (`segmenter_of(cx)` in the widget);
//! nothing here cuts or measures text, it picks a spelling per label, widest
//! first: shorten or drop labels in the collapse order, and when a row still
//! overflows refill it in interaction priority order.
//!
//! Flutter writes this as a `LayoutBuilder` that tries `Row`s of decreasing
//! detail until one fits; the arithmetic is the pure part, unit-tested at the
//! oracle's widths.

/// The hints of one footer, in the reading order within each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Agents,
    Shortcuts,
    Action,
    SideActive,
    SideNav,
    AgentName,
    Shell,
    Vim,
    Plan,
    Goal,
    Ide,
    Context,
    Warning,
}

impl Kind {
    /// Shell, Vim and Plan: the compact input modes.
    fn is_input_mode(self) -> bool {
        matches!(self, Kind::Shell | Kind::Vim | Kind::Plan)
    }

    /// The left side (navigation and actions); the rest sit at the right.
    pub fn left(self) -> bool {
        matches!(
            self,
            Kind::Agents | Kind::Shortcuts | Kind::Action | Kind::SideActive | Kind::SideNav
        )
    }

    /// With two rows the navigation and the modes stay on the first, the
    /// actions and the other indicators go to the second.
    pub fn row(self, two_rows: bool) -> usize {
        usize::from(
            two_rows
                && !self.is_input_mode()
                && !matches!(self, Kind::Agents | Kind::Shortcuts | Kind::SideNav),
        )
    }
}

/// A hint with its spellings' widths in cells, the widest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub kind: Kind,
    pub widths: Vec<usize>,
}

/// The widths of a left group and a right group of one row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowWidth {
    pub left: usize,
    pub right: usize,
}

impl RowWidth {
    /// The row's cells: both groups and the cell between them.
    pub fn width(self) -> usize {
        self.left + self.right + usize::from(self.left > 0 && self.right > 0)
    }
}

/// A label that stayed, with the spelling it shows and the row it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub kind: Kind,
    pub form: usize,
    pub row: usize,
}

/// What the footer holds: the labels that stayed (in reading order) and the
/// widths of the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub rows: [RowWidth; 2],
    pub placed: Vec<Placed>,
}

/// `" · "` between two labels of a group.
const JOIN: usize = 3;

fn joined(first: usize, second: usize) -> usize {
    first + second + usize::from(first > 0 && second > 0) * JOIN
}

struct Slot<'a> {
    label: &'a Label,
    selected: Option<usize>,
}

impl Slot<'_> {
    fn width(&self) -> usize {
        self.selected
            .and_then(|form| self.label.widths.get(form))
            .copied()
            .unwrap_or(0)
    }
}

/// The rows the hints ask for at `width`: one when any spelling fits, a
/// second when a current action and the compact input modes each fit but not
/// together (`desired_rows`).
pub fn desired_rows(labels: &[Label], width: usize) -> u16 {
    let mut compact = RowWidth::default();
    let mut any_fits = false;
    for label in labels {
        let Some(&cells) = label.widths.last() else {
            continue;
        };
        any_fits |= cells <= width;
        let group = if label.kind.is_input_mode() {
            &mut compact.right
        } else if matches!(label.kind, Kind::Action | Kind::SideActive) {
            &mut compact.left
        } else {
            continue;
        };
        let next = joined(*group, cells);
        if next <= width {
            *group = next;
        }
    }
    u16::from(any_fits) + u16::from(compact.width() > width)
}

fn placement(slots: &[Slot<'_>], two: bool) -> [RowWidth; 2] {
    let mut rows = [RowWidth::default(); 2];
    for slot in slots {
        let row = &mut rows[slot.label.kind.row(two)];
        let group = if slot.label.kind.left() {
            &mut row.left
        } else {
            &mut row.right
        };
        *group = joined(*group, slot.width());
    }
    rows
}

/// The spellings that fit `width` (`Hints::fit`): the collapse order first
/// (Shortcuts drops its label, Plan, Warning, IDE and Action take their short
/// spelling, Warning its count, Agents drops), then, for a row that still
/// overflows, the refill in interaction priority order.
pub fn fit(labels: &[Label], width: usize, two: bool) -> Layout {
    let mut slots: Vec<Slot<'_>> = labels
        .iter()
        .map(|label| Slot {
            label,
            selected: Some(0),
        })
        .collect();
    let mut rows = placement(&slots, two);
    for (kind, form) in [
        (Kind::Shortcuts, None),
        (Kind::Plan, Some(1)),
        (Kind::Warning, Some(1)),
        (Kind::Ide, Some(1)),
        (Kind::Action, Some(1)),
        (Kind::Warning, Some(2)),
        (Kind::Agents, None),
    ] {
        if rows[kind.row(two)].width() <= width {
            continue;
        }
        let Some(slot) = slots.iter_mut().find(|slot| slot.label.kind == kind) else {
            continue;
        };
        if let Some(index) = form
            && slot
                .label
                .widths
                .get(index)
                .is_none_or(|&cells| cells >= slot.width())
        {
            continue;
        }
        slot.selected = form;
        rows = placement(&slots, two);
    }
    let overflow = rows.map(|row| row.width() > width);
    if overflow.contains(&true) {
        // Refill only the overflowing rows in interaction priority order.
        for slot in &mut slots {
            if overflow[slot.label.kind.row(two)] {
                slot.selected = None;
            }
        }
        for kind in [
            Kind::Action,
            Kind::SideActive,
            Kind::Shell,
            Kind::Vim,
            Kind::Plan,
            Kind::Goal,
            Kind::Ide,
            Kind::Warning,
            Kind::Context,
            Kind::SideNav,
            Kind::AgentName,
            Kind::Agents,
            Kind::Shortcuts,
        ] {
            if !overflow[kind.row(two)] {
                continue;
            }
            if let Some(at) = slots.iter().position(|slot| slot.label.kind == kind) {
                slots[at].selected = Some(slots[at].label.widths.len() - 1);
                if placement(&slots, two)[kind.row(two)].width() > width {
                    slots[at].selected = None;
                }
            }
        }
        rows = placement(&slots, two);
    }
    let mut placed: Vec<Placed> = slots
        .iter()
        .filter_map(|slot| {
            slot.selected.map(|form| Placed {
                kind: slot.label.kind,
                form,
                row: slot.label.kind.row(two),
            })
        })
        .collect();
    placed.sort_by_key(|item| item.kind);
    Layout { rows, placed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(kind: Kind, widths: &[usize]) -> Label {
        Label {
            kind,
            widths: widths.to_vec(),
        }
    }

    fn kinds(layout: &Layout) -> Vec<Kind> {
        layout.placed.iter().map(|item| item.kind).collect()
    }

    /// `? for shortcuts` (15) and `100% context left` (17): the shortcuts
    /// when only they fit, the meter alone when it does, both when both do
    /// (oversized_goal_yields_to_an_eligible_whole_hint_with_status_disabled).
    #[test]
    fn the_hint_that_fits_whole_stays_and_the_other_leaves() {
        let labels = [label(Kind::Shortcuts, &[15]), label(Kind::Context, &[17])];
        assert_eq!(kinds(&fit(&labels, 15, false)), [Kind::Shortcuts]);
        assert_eq!(kinds(&fit(&labels, 17, false)), [Kind::Context]);
        assert_eq!(
            kinds(&fit(&labels, 33, false)),
            [Kind::Shortcuts, Kind::Context]
        );
    }

    /// Plan keeps its cycle hint while it fits, then takes the short
    /// spelling before the shortcuts go.
    #[test]
    fn plan_shortens_before_anything_else_leaves() {
        let labels = [label(Kind::Shortcuts, &[15]), label(Kind::Plan, &[25, 9])];
        let wide = fit(&labels, 60, false);
        assert_eq!(wide.placed[1].form, 0);
        // shortcuts go first (the collapse order starts with them)
        let narrow = fit(&labels, 30, false);
        assert_eq!(kinds(&narrow), [Kind::Plan]);
        assert_eq!(narrow.placed[0].form, 0);
        let tight = fit(&labels, 20, false);
        assert_eq!(kinds(&tight), [Kind::Plan]);
        assert_eq!(tight.placed[0].form, 1);
    }

    /// An action and an input mode that fit apart but not together ask for
    /// a second row (`desired_rows`); the navigation stays on the first.
    #[test]
    fn two_rows_only_when_the_action_and_the_mode_do_not_fit_together() {
        let labels = [label(Kind::Action, &[20, 12]), label(Kind::Plan, &[25, 9])];
        assert_eq!(desired_rows(&labels, 80), 1);
        assert_eq!(desired_rows(&labels, 20), 2);
        assert_eq!(desired_rows(&[], 80), 0);
        assert_eq!(desired_rows(&labels, 5), 0);
    }

    /// With two rows the mode stays on the first and the action goes down
    /// (`active_side_action_keeps_plan_expanded_on_the_other_row`).
    #[test]
    fn the_mode_stays_above_and_the_action_goes_below() {
        let labels = [label(Kind::SideActive, &[58]), label(Kind::Plan, &[25, 9])];
        let layout = fit(&labels, 60, true);
        let plan = layout.placed.iter().find(|p| p.kind == Kind::Plan).unwrap();
        let side = layout
            .placed
            .iter()
            .find(|p| p.kind == Kind::SideActive)
            .unwrap();
        assert_eq!((plan.row, plan.form), (0, 0));
        assert_eq!(side.row, 1);
    }

    /// A label whose shortest spelling does not fit leaves; the others stay.
    #[test]
    fn a_label_that_cannot_fit_leaves_the_rest() {
        let labels = [
            label(Kind::Agents, &[15]),
            label(Kind::Shortcuts, &[15]),
            label(Kind::Warning, &[24, 9, 3]),
        ];
        let layout = fit(&labels, 14, false);
        assert!(layout.rows[0].width() <= 14);
        assert_eq!(kinds(&layout), [Kind::Warning]);
        assert_eq!(layout.placed[0].form, 2);
    }
}
