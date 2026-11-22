//! The background terminals line (the oracle's `bottom_pane/unified_exec_footer.rs`):
//! how many unified-exec processes still run, dim, indented two columns, a
//! hint to `/ps` and `/stop` them; clipped at the pane's edge, never wrapped.
//! The thread model keeps the terminals (`ThreadModel::terminals`); the line
//! is a selector over their count.

use rtwk::layout::SizedBox;
use rtwk::provider::select;
use rtwk::text::{Text, TextOverflow};
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::ThreadModel;

use super::strings;

/// The line, nothing while no terminal runs.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::pages::session) struct TerminalsLine;

impl_widget!(stateless eq TerminalsLine);

impl StatelessWidget for TerminalsLine {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let count = select::<ThreadModel, usize, _>(cx, |thread| thread.terminals().len());
        if count == 0 {
            return SizedBox::shrink().into();
        }
        let strings = strings::of(cx);
        Text::line(format!(
            "{}{}",
            strings.terminals_indent,
            strings.terminals(count)
        ))
        .overflow(TextOverflow::Clip)
        .style(super::dim())
        .into()
    }
}
