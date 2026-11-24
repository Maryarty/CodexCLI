//! The `/ps` cell (oracle `history_cell/exec.rs` `UnifiedExecProcessesCell` under the typed
//! `/ps`): the background terminals the thread ran when the command was typed, one row each with
//! its last output lines under it. A row never wraps: a command is cut to 80 graphemes and to the
//! width, and a cut says ` [...]` (the oracle's own mark, not the theme's `…`), so the cell
//! reads the width it is given (`LayoutBuilder`) and picks the spelling of each row from it.
//!
//! Flutter: a `Column` of one-line `Text.rich` in a `LayoutBuilder`; the words are `strings.rs`'s.

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};
use rtwk_core::unicode::{TextSegmenter, grapheme_boundaries};

use codex_tui_core::features::processes::Note;

use super::strings::{
    CHUNK_FIRST, CHUNK_NEXT, COMMAND, CUT, HEADING, MAX_GRAPHEMES, MAX_PROCESSES, NEWLINE, NONE,
    PROCESS_MARK, more,
};
use crate::app::theme::Tokens;

/// What `/ps` left in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessesCell {
    pub note: Note,
}

impl_widget!(stateless eq ProcessesCell);

impl StatelessWidget for ProcessesCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let note = self.note.clone();
        LayoutBuilder::new(move |cx, room| {
            let seg = segmenter_of(cx);
            let width = usize::try_from(room.max_width).unwrap_or(0);
            rows(&note, &tokens, seg, width)
        })
        .into()
    }
}

fn blank() -> AnyWidget {
    SizedBox::new(None, Some(1), None).into()
}

fn row(spans: Vec<Span>) -> AnyWidget {
    Text::rich(spans)
        .max_lines(Some(1))
        .overflow(TextOverflow::Clip)
        .into()
}

/// The longest head of `text` that fits `budget` cells, and whether something is left.
fn head(seg: TextSegmenter, text: &str, budget: usize) -> (&str, bool) {
    let mut used = 0;
    for (start, end) in grapheme_boundaries(text) {
        used += seg.width(&text[start..end]);
        if used > budget {
            return (&text[..start], true);
        }
    }
    (text, false)
}

/// A row of `text` after `mark` in `width` cells: it ends in [`CUT`] when `cut` says it was
/// shortened already or the width does (oracle: the cut keeps room for the suffix only when the
/// room is wider than the suffix).
fn clipped(
    seg: TextSegmenter,
    mark: &'static str,
    text: &str,
    cut: bool,
    width: usize,
    ink: Style,
    faint: Style,
) -> Vec<Span> {
    let mark_width = seg.width(mark);
    if width <= mark_width {
        return vec![Span::styled(mark, faint)];
    }
    let budget = width - mark_width;
    let suffix = seg.width(CUT);
    let (kept, overflows) = head(seg, text, budget);
    if (cut || overflows) && budget > suffix {
        let (kept, _) = head(seg, text, budget - suffix);
        return vec![
            Span::styled(mark, faint.clone()),
            Span::styled(kept.to_owned(), ink),
            Span::styled(CUT, faint),
        ];
    }
    vec![
        Span::styled(mark, faint),
        Span::styled(kept.to_owned(), ink),
    ]
}

/// The first `MAX_GRAPHEMES` graphemes of a command, a newline as `↵`, and whether it was cut.
fn snippet(command: &str) -> (String, bool) {
    let mut out = String::new();
    let mut graphemes = grapheme_boundaries(command);
    for (start, end) in graphemes.by_ref().take(MAX_GRAPHEMES) {
        match &command[start..end] {
            "\n" | "\r\n" => out.push_str(NEWLINE),
            text => out.push_str(text),
        }
    }
    (out, graphemes.next().is_some())
}

fn rows(note: &Note, tokens: &Tokens, seg: TextSegmenter, width: usize) -> AnyWidget {
    let faint = tokens.dim().attrs(Attrs::DIM);
    let accent = tokens.accent_on(&Style::new());
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(row(vec![Span::styled(COMMAND, tokens.accent())]))
        .child(blank())
        .child(row(vec![Span::styled(HEADING, tokens.bold())]))
        .child(blank());
    if note.terminals.is_empty() {
        let italic = Style::new().attrs(Attrs::ITALIC);
        return column.child(row(vec![Span::styled(NONE, italic)])).into();
    }
    let mut shown = 0;
    for terminal in note.terminals.iter().take(MAX_PROCESSES) {
        let (command, cut) = snippet(&terminal.command);
        column = column.child(row(clipped(
            seg,
            PROCESS_MARK,
            &command,
            cut,
            width,
            accent.clone(),
            faint.clone(),
        )));
        for (index, chunk) in terminal.recent.iter().enumerate() {
            let mark = if index == 0 { CHUNK_FIRST } else { CHUNK_NEXT };
            column = column.child(row(clipped(
                seg,
                mark,
                chunk,
                false,
                width,
                faint.clone(),
                faint.clone(),
            )));
        }
        shown += 1;
    }
    let remaining = note.terminals.len().saturating_sub(shown);
    if remaining > 0 {
        let mark_width = seg.width(PROCESS_MARK);
        let spans = if width <= mark_width {
            vec![Span::styled(PROCESS_MARK, faint)]
        } else {
            let text = more(remaining);
            let (kept, _) = head(seg, &text, width - mark_width);
            vec![
                Span::styled(PROCESS_MARK, faint.clone()),
                Span::styled(kept.to_owned(), faint),
            ]
        };
        column = column.child(row(spans));
    }
    column.into()
}
