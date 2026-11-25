//! The strings of the bottom pane's activity slots: the Working row, the pending
//! preview and the background-terminal summary. `EN` is the oracle's text byte
//! for byte (`status_indicator_widget.rs`, `bottom_pane/pending_input_preview.rs`,
//! `bottom_pane/unified_exec_footer.rs`, `bottom_pane/hook_status.rs`).

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

use crate::l10n::Tables;
use crate::l10n::lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// `(0s • ` + the interrupt key + ` to interrupt)`; without a key `(0s)`.
    pub elapsed_open: &'static str,
    pub elapsed_sep: &'static str,
    pub to_interrupt: &'static str,
    pub elapsed_close: &'static str,
    /// Between the elapsed label, the background summary and the hook text.
    pub inline_sep: &'static str,
    /// `  └ `: the first row of the details, the hook text that did not fit.
    pub details_prefix: &'static str,
    /// What the compaction says under its header (`compaction.rs`).
    pub compaction_details: &'static str,
    /// `• ` before the hook text of a row with no turn status.
    pub bullet: &'static str,
    /// The pending preview: the section headers and what follows the key.
    pub steers_header: &'static str,
    pub steers_press: &'static str,
    pub steers_interrupt: &'static str,
    pub rejected_header: &'static str,
    pub queued_header: &'static str,
    pub edit_rest: &'static str,
    /// The questions that wait, under the preview: `  ? `, `N question(s)`, and what follows the key.
    pub question_mark: &'static str,
    pub questions_one: Msg<1>,
    pub questions_many: Msg<1>,
    pub answer_rest: &'static str,
    /// `  ↳ `, the hanging indent under it, and the clipped-rows mark.
    pub item_prefix: &'static str,
    pub item_hang: &'static str,
    pub ellipsis: &'static str,
    /// `N background terminal(s) running · /ps to view · /stop to close`.
    pub terminals_one: Msg<1>,
    pub terminals_many: Msg<1>,
    pub terminals_indent: &'static str,
    pub num: NumSyms,
}

impl Strings {
    /// `N question(s)` of the summary of the questions that wait (`question_summary`).
    pub fn questions(&self, count: usize) -> String {
        let table = if count == 1 {
            self.questions_one
        } else {
            self.questions_many
        };
        Fmt::new(table, [Arg::Text(&count.to_string())], &self.num).to_string()
    }

    /// The summary of `count` background terminals (`UnifiedExecFooter::summary_text`).
    pub fn terminals(&self, count: usize) -> String {
        let table = if count == 1 {
            self.terminals_one
        } else {
            self.terminals_many
        };
        Fmt::new(table, [Arg::Text(&count.to_string())], &self.num).to_string()
    }
}

pub const EN: Strings = Strings {
    elapsed_open: "(",
    elapsed_sep: " \u{2022} ",
    to_interrupt: " to interrupt)",
    elapsed_close: ")",
    inline_sep: " \u{b7} ",
    details_prefix: "  \u{2514} ",
    compaction_details: "Making room to continue.",
    bullet: "\u{2022} ",
    steers_header: "Messages to be submitted after next tool call",
    steers_press: " (press ",
    steers_interrupt: " to interrupt and send immediately)",
    rejected_header: "Messages to be submitted at end of turn",
    queued_header: "Queued follow-up inputs",
    edit_rest: " edit last queued message",
    question_mark: "  ? ",
    questions_one: msg!("{0} question"),
    questions_many: msg!("{0} questions"),
    answer_rest: " to answer",
    item_prefix: "  \u{21b3} ",
    item_hang: "    ",
    ellipsis: "\u{2026}",
    terminals_one: msg!("{0} background terminal running \u{b7} /ps to view \u{b7} /stop to close"),
    terminals_many: msg!(
        "{0} background terminals running \u{b7} /ps to view \u{b7} /stop to close"
    ),
    terminals_indent: "  ",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
