//! The links of a spoken answer to the workspace files it names: the answer mounts an
//! `Interest` for each path it reads, the host's finds come back as `file:` URLs the answer
//! draws as links.

use std::collections::BTreeMap;

use rtwk::provider::select;
use rtwk::style::{Attrs, Span};
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::spoken_links::{Candidate, Interest, Scan, SpokenLinks};
use codex_tui_core::features::{Feature, FeatureStates};

use super::FeatureView;
use crate::pages::session::strings::Strings;
use crate::widgets::interest::Interest as Watch;

impl FeatureView for SpokenLinks {
    fn note_words(note: &<SpokenLinks as Feature>::Note, _: &Strings) -> String {
        note.absurd()
    }
}

/// `lines` of a spoken answer (`cwd` its session's directory) with the words that read as paths
/// of the workspace underlined and linked once the host finds the file there, handed to `build`;
/// the widget it makes is watched for as long as it is on the screen, so the host is asked about
/// each path once.
pub(crate) fn link<S>(
    cx: &mut Cx<'_, S>,
    cwd: Option<&str>,
    lines: Vec<Vec<Span>>,
    build: impl FnOnce(Vec<Vec<Span>>) -> AnyWidget,
) -> AnyWidget {
    let mut scan = Scan::new();
    let found: Vec<Vec<Candidate>> = lines
        .iter()
        .map(|line| {
            if scan.exhausted() || cwd.is_none() {
                return Vec::new();
            }
            scan.line(
                &line
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>(),
            )
        })
        .collect();
    let mut interests: Vec<Interest> = cwd
        .iter()
        .flat_map(|cwd| {
            found.iter().flatten().map(|candidate| Interest {
                cwd: (*cwd).to_owned(),
                path: candidate.path.clone(),
            })
        })
        .collect();
    interests.sort();
    interests.dedup();
    let answers: BTreeMap<String, String> = interests
        .iter()
        .zip(urls(cx, &interests))
        .filter_map(|(interest, url)| Some((interest.path.clone(), url?)))
        .collect();
    let lines = lines
        .into_iter()
        .zip(found)
        .map(|(line, found)| linked(line, &found, &answers))
        .collect();
    watch(interests, build(lines))
}

/// What the host found for each of `interests` so far: the URL of the file, or none.
fn urls<S>(cx: &mut Cx<'_, S>, interests: &[Interest]) -> Vec<Option<String>> {
    let interests = interests.to_vec();
    select::<FeatureStates, Vec<Option<String>>, _>(cx, move |features| {
        interests
            .iter()
            .map(|interest| features.spoken_links.url(interest).map(str::to_owned))
            .collect()
    })
}

/// `child`, for as long as it is on the screen, wants `interests` looked up.
fn watch(interests: Vec<Interest>, child: AnyWidget) -> AnyWidget {
    interests
        .into_iter()
        .rev()
        .fold(child, |child, interest| Watch::new(interest, child).into())
}

/// `line` with each candidate the host found a file for underlined and linked to it, unless a
/// link is there already.
fn linked(line: Vec<Span>, found: &[Candidate], answers: &BTreeMap<String, String>) -> Vec<Span> {
    let mut spans = line;
    for candidate in found {
        let Some(url) = answers.get(&candidate.path) else {
            continue;
        };
        let mut offset = 0;
        let overlaps = spans.iter().any(|span| {
            let start = offset;
            offset += span.text.len();
            span.style.link.is_some()
                && start < candidate.bytes.end
                && candidate.bytes.start < offset
        });
        if overlaps {
            continue;
        }
        let mut cut = Vec::with_capacity(spans.len() + 2);
        let mut offset = 0;
        for span in spans {
            let (start, end) = (offset, offset + span.text.len());
            offset = end;
            let from = candidate.bytes.start.clamp(start, end) - start;
            let to = candidate.bytes.end.clamp(start, end) - start;
            if from >= to {
                cut.push(span);
                continue;
            }
            let style = span.style.clone();
            let text = span.text.to_string();
            if from > 0 {
                cut.push(Span::styled(text[..from].to_owned(), style.clone()));
            }
            cut.push(Span::styled(
                text[from..to].to_owned(),
                style.clone().attrs(Attrs::UNDERLINE).link(url.as_str()),
            ));
            if to < text.len() {
                cut.push(Span::styled(text[to..].to_owned(), style));
            }
        }
        spans = cut;
    }
    spans
}
