//! The notices the import leaves in the history (oracle:
//! `external_agent_config_migration_started_lines` and `_finished_lines` of
//! `flow.rs`): lines of segments with a semantic tone, which the chat's cells
//! turn into styled text. Pure functions of what core says the import did.

use codex_tui_core::screens::import::{Finished, ItemKind, Started};
use rtwk::l10n::NumSyms;

use super::labels;
use super::strings::Strings;

/// How a segment is drawn: the oracle's `dim()`, `cyan()`, `cyan().bold()`,
/// `green()` and `red()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dim,
    Cyan,
    CyanBold,
    Green,
    Red,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub tone: Tone,
    pub text: String,
}

/// One line of the history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line(pub Vec<Seg>);

impl Line {
    /// The line's text with no styles.
    pub fn text(&self) -> String {
        self.0.iter().map(|seg| seg.text.as_str()).collect()
    }
}

fn seg(tone: Tone, text: impl Into<String>) -> Seg {
    Seg {
        tone,
        text: text.into(),
    }
}

fn indented(indent: &str, tone: Tone, text: impl Into<String>) -> Line {
    Line(vec![seg(Tone::Plain, indent), seg(tone, text)])
}

/// The notice an import that started leaves; `remaining` items stay for
/// another `/import`.
pub fn started(strings: &Strings, done: &Started, num: &NumSyms) -> Vec<Line> {
    let mut summaries: Vec<(ItemKind, usize, Vec<&str>)> = Vec::new();
    for sel in &done.items {
        match summaries
            .iter_mut()
            .find(|(kind, _, _)| *kind == sel.kind())
        {
            Some((_, total, all)) => {
                *total += sel.count();
                all.extend(sel.names());
            }
            None => summaries.push((sel.kind(), sel.count(), sel.names())),
        }
    }
    let mut lines = vec![
        Line(vec![
            seg(Tone::Dim, strings.notice_bullet),
            seg(Tone::Cyan, strings.started),
            seg(Tone::Plain, strings.started_tail),
        ]),
        indented("  ", Tone::Dim, strings.apply_to_new_chats),
        indented("  ", Tone::CyanBold, strings.importing_title),
    ];
    for (kind, count, all) in summaries {
        let mut line = vec![
            seg(Tone::Plain, "    "),
            seg(Tone::Cyan, labels::kind(strings, kind)),
            seg(Tone::Plain, ": "),
            seg(Tone::Green, count.to_string()),
        ];
        if !all.is_empty() {
            let shown: Vec<&str> = all.iter().take(3).copied().collect();
            let mut summary = shown.join(", ");
            if all.len() > shown.len() {
                summary = strings.names_more(&summary, all.len() - shown.len(), num);
            }
            line.extend([seg(Tone::Dim, strings.dash), seg(Tone::Plain, summary)]);
        }
        lines.push(Line(line));
    }
    if done.remaining > 0 {
        lines.push(indented(
            "  ",
            Tone::Dim,
            strings.remaining(done.remaining, num),
        ));
    }
    lines
}

/// The notice an import that finished leaves.
pub fn finished(strings: &Strings, done: &Finished, num: &NumSyms) -> Vec<Line> {
    let tone_of = |failed: usize| if failed == 0 { Tone::Green } else { Tone::Red };
    let mut lines = vec![Line(vec![
        seg(Tone::Dim, strings.notice_bullet),
        seg(Tone::Plain, strings.finished),
        seg(Tone::Green, strings.imported(done.imported(), num)),
        seg(Tone::Plain, ", "),
        seg(tone_of(done.failed()), strings.failed(done.failed(), num)),
        seg(Tone::Plain, "."),
    ])];
    if !done.by_type.is_empty() {
        lines.push(indented("  ", Tone::CyanBold, strings.results_by_type));
        for result in &done.by_type {
            lines.push(Line(vec![
                seg(Tone::Plain, "    "),
                seg(Tone::Cyan, labels::kind(strings, result.kind)),
                seg(Tone::Plain, ": "),
                seg(Tone::Green, strings.imported(result.imported, num)),
                seg(Tone::Plain, ", "),
                seg(tone_of(result.failed), strings.failed(result.failed, num)),
            ]));
        }
    }
    lines.push(indented("  ", Tone::Dim, strings.run_again));
    lines
}
