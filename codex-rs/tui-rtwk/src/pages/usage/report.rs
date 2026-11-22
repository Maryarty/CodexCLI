//! The body of a report tab other than the Overview (oracle: `panels.rs`,
//! `tool_panel.rs`, `task_panel.rs`, `plan_panel.rs`, `chart.rs`): what its
//! load says while it is not ready, or that the range holds no data. The
//! plots and the lists of a ready report are built in the steps that follow.

use rtwk::AnyWidget;
use rtwk::style::Style;

use codex_tui_core::screens::usage::{Load, Model, Section};

use super::strings::Strings;
use super::{line, nothing};

/// The report of `section`.
pub fn body(model: &Model, section: Section, strings: &Strings) -> AnyWidget {
    let load = &model.histories[section as usize];
    match load {
        Load::Ready(history) if history.data.is_empty() => {
            line(strings.empty_range, Style::default()).into()
        }
        Load::Ready(_) => nothing(),
        other => other
            .message()
            .map_or_else(nothing, |message| line(message, Style::default()).into()),
    }
}
