//! The agent status history cell (oracle `app/agent_status_feed.rs`).

use codex_tui_core::features::subagents::{Activity, Entry, Note, Subagents};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::Attrs;
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use super::FeatureView;
use crate::app::theme::Tokens;
use crate::pages::session::strings::Strings as SessionStrings;
use crate::widgets::measure;

mod strings;
use strings::{COMMAND, NO_ACTIVITY, NO_AGENTS, NONE_RUNNING, TITLE, agent_title};
const INDENT: i32 = 4;
const LINES: usize = 3;

impl FeatureView for Subagents {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        match note {
            Note::NoAgents => NO_AGENTS.to_owned(),
            Note::Running(entries) => {
                let mut words = vec![COMMAND.to_owned(), TITLE.to_owned(), String::new()];
                if entries.is_empty() {
                    words.push(NONE_RUNNING.to_owned());
                }
                for entry in entries {
                    words.push(agent_title(entry));
                    let lines = entry
                        .recent
                        .iter()
                        .map(activity)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>();
                    if lines.is_empty() {
                        words.push(format!("    {NO_ACTIVITY}"));
                    } else {
                        words.extend(
                            lines
                                .iter()
                                .skip(lines.len().saturating_sub(LINES))
                                .map(|line| format!("    {line}")),
                        );
                    }
                    words.push(String::new());
                }
                if !entries.is_empty() {
                    words.pop();
                }
                words.join("\n")
            }
        }
    }

    fn cell(note: &Note) -> Option<AnyWidget> {
        if let Note::Running(entries) = note {
            Some(Feed(entries.clone()).into())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Feed(Vec<Entry>);

impl_widget!(stateless eq Feed);

impl StatelessWidget for Feed {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let entries = self.0.clone();
        LayoutBuilder::new(move |cx, limits| {
            let width = usize::try_from((limits.max_width - INDENT).max(1)).unwrap_or(1);
            let seg = segmenter_of(cx);
            let dim = tokens.dim();
            let italic = dim.clone().attrs(Attrs::ITALIC);
            let mut content = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::line(COMMAND).style(tokens.accent()))
                .child(Text::line(TITLE).style(rtwk::style::Style::new().attrs(Attrs::BOLD)))
                .child(Text::line(""));
            if entries.is_empty() {
                return content.child(Text::line(NONE_RUNNING).style(italic)).into();
            }
            for (index, entry) in entries.iter().enumerate() {
                if index != 0 {
                    content = content.child(Text::line(""));
                }
                content = content.child(Text::line(agent_title(entry)).style(tokens.accent()));
                let lines: Vec<_> = entry
                    .recent
                    .iter()
                    .map(activity)
                    .filter(|line| !line.is_empty())
                    .flat_map(|line| measure::rows(seg, &line, width))
                    .collect();
                if lines.is_empty() {
                    content = content.child(Padding::new(
                        EdgeInsets::only(INDENT, 0, 0, 0),
                        Text::line(NO_ACTIVITY).style(italic.clone()),
                    ));
                } else {
                    for row in lines.iter().skip(lines.len().saturating_sub(LINES)) {
                        content = content.child(Padding::new(
                            EdgeInsets::only(INDENT, 0, 0, 0),
                            Text::line(row.clone()).style(dim.clone()),
                        ));
                    }
                }
            }
            content.into()
        })
        .into()
    }
}

fn activity(activity: &Activity) -> String {
    let text = strings::activity(activity);
    let end = rtwk_core::unicode::grapheme_boundaries(&text)
        .nth(240)
        .map_or(text.len(), |(start, _)| start);
    text[..end].split_whitespace().collect::<Vec<_>>().join(" ")
}
