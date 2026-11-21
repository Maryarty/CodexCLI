//! The toolbar: the filter, status and sort controls with their values, the
//! focused control's value in the tab style. It has three spellings (full,
//! compact, and the focused control alone) and takes the first that fits the
//! width it is given, as Flutter lays out with a `LayoutBuilder`.
//! Oracle: `resume_picker.rs` `toolbar_for_width` and `toolbar_line`.

use rtwk::style::Span;
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::screens::resume::{Action, Control, Filter, Model, Sort, Status};

use super::look::Look;
use super::strings::Strings;

/// What the toolbar draws: the model's controls and the one with the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    action: Action,
    filter: Filter,
    status: Status,
    sort: Sort,
    has_cwd: bool,
    repo_available: bool,
    focus: Control,
}

impl Facts {
    pub fn of(model: &Model, focus: Control) -> Facts {
        Facts {
            action: model.action,
            filter: model.filter,
            status: model.status,
            sort: model.sort,
            has_cwd: model.scope.cwd.is_some(),
            repo_available: model.scope.repo_available(),
            focus,
        }
    }
}

struct Bar<'a> {
    look: &'a Look,
    strings: &'a Strings,
    facts: Facts,
}

impl Bar<'_> {
    fn label(&self, text: &str) -> Span {
        Span::styled(text.to_owned(), self.look.dim.clone())
    }

    /// One value of a control: ` text `, the active one styled, in the tab
    /// style while its control has the focus.
    fn value(&self, text: &str, active: bool, focused: bool) -> Span {
        let style = match (active, focused) {
            (true, true) => self.look.tab.clone(),
            (true, false) => self.look.current.clone(),
            (false, _) => self.look.dim.clone(),
        };
        Span::styled(self.strings.value(text), style)
    }

    fn filter_name(&self, filter: Filter) -> &str {
        match filter {
            Filter::Cwd => self.strings.filter_cwd,
            Filter::Repo => self.strings.filter_repo,
            Filter::All => self.strings.filter_all,
        }
    }

    fn filter(&self, compact: bool) -> Vec<Span> {
        let focused = self.facts.focus == Control::Filter;
        let facts = self.facts;
        if compact || !facts.has_cwd {
            return vec![
                self.label(self.strings.filter),
                self.value(self.filter_name(facts.filter), true, focused),
            ];
        }
        let mut line = vec![self.label(&format!("{} ", self.strings.filter))];
        let mut value = |filter: Filter| {
            line.push(self.value(self.filter_name(filter), facts.filter == filter, focused));
        };
        value(Filter::Cwd);
        if facts.repo_available {
            value(Filter::Repo);
        }
        value(Filter::All);
        line
    }

    fn status(&self, compact: bool) -> Vec<Span> {
        let focused = self.facts.focus == Control::Status;
        let (strings, facts) = (self.strings, self.facts);
        if compact {
            let name = match facts.status {
                Status::Active => strings.status_active,
                Status::Archived => strings.status_archived,
            };
            return vec![self.label(strings.status), self.value(name, true, focused)];
        }
        vec![
            self.label(&format!("{} ", strings.status)),
            self.value(
                strings.status_active,
                facts.status == Status::Active,
                focused,
            ),
            self.value(
                strings.status_archived,
                facts.status == Status::Archived,
                focused,
            ),
        ]
    }

    fn sort(&self, compact: bool) -> Vec<Span> {
        let focused = self.facts.focus == Control::Sort;
        let (strings, facts) = (self.strings, self.facts);
        if compact {
            let name = match facts.sort {
                Sort::Updated => strings.sort_updated,
                Sort::Created => strings.sort_created,
            };
            return vec![self.label(strings.sort), self.value(name, true, focused)];
        }
        vec![
            self.label(&format!("{} ", strings.sort)),
            self.value(strings.sort_updated, facts.sort == Sort::Updated, focused),
            self.value(strings.sort_created, facts.sort == Sort::Created, focused),
        ]
    }

    /// The whole strip (oracle: `toolbar_line`): the status control exists
    /// only when resuming.
    fn line(&self, compact: bool) -> Vec<Span> {
        let resume = self.facts.action == Action::Resume;
        let separator = if compact && resume { " " } else { "   " };
        let mut line = self.filter(compact);
        line.push(self.label(separator));
        if resume {
            line.extend(self.status(compact));
            line.push(self.label(separator));
        }
        line.extend(self.sort(compact));
        line
    }

    /// The control the arrows change, alone (oracle: the last resort of
    /// `toolbar_for_width`).
    fn focused(&self) -> Vec<Span> {
        match self.facts.focus {
            Control::Filter => self.filter(true),
            Control::Status => self.status(true),
            Control::Sort => self.sort(true),
        }
    }
}

/// The toolbar line of the picker's controls.
pub fn toolbar(look: &Look, strings: &'static Strings, facts: Facts) -> AnyWidget {
    let look = look.clone();
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let room = usize::try_from(constraints.max_width).unwrap_or(0);
        let bar = Bar {
            look: &look,
            strings,
            facts,
        };
        let width = |line: &[Span]| -> usize {
            line.iter()
                .map(|span| seg.width(&span.text))
                .sum()
        };
        let line = [bar.line(false), bar.line(true)]
            .into_iter()
            .find(|line| width(line) <= room)
            .unwrap_or_else(|| bar.focused());
        Text::rich(line)
            .wrap(TextWrap::None)
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis)
            .into()
    })
    .into()
}
