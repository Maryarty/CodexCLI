//! The Markdown of the transcript as the oracle draws it. rtwk's `MarkdownView` differs from
//! the oracle's renderer in a few places the oracle's tests pin; the parsed tree is rewritten
//! before the view shows it (rtwk-rich takes no element builders):
//!
//! - headings keep their source markers: `## Title` is drawn with its two hashes in the
//!   heading's own style (markdown_render.rs `start_heading`; G73 in rtwk-gaps.md);
//! - line breaks, local file links and references to them: `inlines.rs`;
//! - a web link's destination follows the host's terminal fact (`Markdown`); only a web
//!   address is a hyperlink (`http`, `https`: the oracle's `web_destination`).
//!
//! Flutter: `MarkdownBody(builders: {'h1': HeadingWithMarker(), 'a': LocalFileLink(cwd)})` under
//! a `Theme` that knows how links show. The view lays out, wraps and styles each rewritten
//! node as it does any text.

mod inlines;

use std::collections::BTreeSet;
use std::sync::Arc;

use rtwk::provider::Provider;
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::{
    Block, Definitions, LinkDestinations, MarkdownLinkTap, MarkdownStream, MarkdownTheme,
    MarkdownView, MdBlock, MdExtensions, MdInline, MdList, MdListItem,
};

use codex_tui_core::data::links::LinkDisplay;

/// The Markdown extensions the oracle reads beyond rtwk's default: with `math` (the
/// `tui.rendering.math` preference) TeX math in dollars (`$..$`, `$$..$$`) and in brackets
/// (`\(..\)`, `\[..\]`), the four forms of the oracle's `markdown_render/math.rs`. A formula
/// parses as `MdInline::Math` and the view draws it as Unicode (rtwk-math). Alerts and
/// footnotes stay off: the oracle reads neither.
pub fn extensions(math: bool) -> MdExtensions {
    if math {
        MdExtensions::MATH | MdExtensions::MATH_BRACKETS
    } else {
        MdExtensions::empty()
    }
}

/// A stream with no text yet that parses as the oracle reads Markdown (`extensions`).
pub fn empty_stream(math: bool) -> MarkdownStream {
    MarkdownStream::with_extensions(extensions(math))
}

/// A stream that starts with `text`, parsed as the oracle reads Markdown (`extensions`).
pub fn stream(text: &str, math: bool) -> MarkdownStream {
    let mut stream = empty_stream(math);
    stream.add(text);
    stream
}

/// A reader's optional activation handler for the links it draws.
#[derive(Clone, PartialEq)]
pub(crate) struct LinkTaps(pub Handler<MarkdownLinkTap>);

/// A `MarkdownView` of `source` as the oracle draws it, local file links shown as written.
pub fn view(source: &str) -> MarkdownView {
    view_in(source, None)
}

/// `view` with local file links shown relative to the session directory `cwd`.
pub fn view_in(source: &str, cwd: Option<&str>) -> MarkdownView {
    let mut stream = stream(source, true);
    let blocks = stream.blocks().to_vec();
    let definitions = stream.definitions().clone();
    MarkdownView::blocks(prepared(&blocks, &definitions, cwd), definitions)
}

/// `blocks` as the view takes them: rewritten as the oracle reads them (`inlines.rs`), each
/// heading with its marker.
pub fn prepared(blocks: &[Block], definitions: &Definitions, cwd: Option<&str>) -> Vec<Block> {
    with_heading_markers(&inlines::rewritten(blocks, definitions, cwd))
}

/// `view` in `theme`. A web link's destination is written after its label as the oracle
/// writes it for a terminal not known to show hyperlinks (`label (https://example.com)`),
/// and not at all for one that does: the label alone is then the hyperlink
/// (`LinkDestinations::Auto`). The host says which (`cells::Model::link_display`).
pub fn themed(view: MarkdownView, theme: MarkdownTheme) -> Markdown {
    Markdown { view, theme }
}

/// A [`MarkdownView`] in a theme, its link display from the app's terminal fact.
#[derive(Clone)]
pub struct Markdown {
    view: MarkdownView,
    theme: MarkdownTheme,
}

#[doc(hidden)]
pub struct MarkdownState;

impl StatefulWidget for Markdown {
    type State = MarkdownState;

    fn create_state(&self) -> MarkdownState {
        MarkdownState
    }
}

impl_widget!(stateful Markdown);

impl State for MarkdownState {
    type Widget = Markdown;
    type Msg = ();

    fn build(&self, widget: &Markdown, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // Read as the oracle reads it, once for the run (a `LazyLock`): the host's fact comes at
        // startup, and a message drawn before it is drawn again as its parent rebuilds. A tree
        // with no app above it (a widget on its own) has the oracle's default.
        let destinations = match crate::app::theme::Tokens::link_display(cx) {
            LinkDisplay::WithDestination => LinkDestinations::Suffix,
            LinkDisplay::LabelOnly => LinkDestinations::Auto,
        };
        // Only a web address is a hyperlink, as the oracle's `web_destination` has it: a
        // `mailto:` link shows its destination and is no hyperlink.
        let schemes = BTreeSet::from(["http".to_string(), "https".to_string()]);
        let markdown = widget
            .view
            .clone()
            .theme(widget.theme.clone())
            .hyperlink_schemes(schemes)
            .link_destinations(destinations);
        if let Some(taps) = Provider::<LinkTaps>::maybe_of(cx, true) {
            markdown.on_link_tap(taps.0.clone()).into()
        } else {
            markdown.into()
        }
    }
}

/// The source marker of a heading of level 1 to 6 (markdown_render.rs
/// `start_heading`): the hashes and a space.
const MARKERS: [&str; 6] = ["# ", "## ", "### ", "#### ", "##### ", "###### "];

/// `blocks` with the marker (`#` to `######`, and a space) at the start of
/// every heading, in lists and quotes too.
fn with_heading_markers(blocks: &[Block]) -> Vec<Block> {
    blocks.iter().map(marked).collect()
}

fn marked(block: &Block) -> Block {
    match &**block {
        MdBlock::Heading { level, children } => {
            let marker = MARKERS[usize::from(*level).clamp(1, MARKERS.len()) - 1];
            let mut content = Vec::with_capacity(children.len() + 1);
            content.push(MdInline::Text(Arc::from(marker)));
            content.extend(children.iter().cloned());
            Arc::new(MdBlock::Heading {
                level: *level,
                children: content,
            })
        }
        MdBlock::BlockQuote(children) => {
            Arc::new(MdBlock::BlockQuote(with_heading_markers(children)))
        }
        MdBlock::List(list) => Arc::new(MdBlock::List(MdList {
            items: list
                .items
                .iter()
                .map(|item| {
                    Arc::new(MdListItem {
                        children: with_heading_markers(&item.children),
                        checked: item.checked,
                    })
                })
                .collect(),
            ..list.clone()
        })),
        _ => Arc::clone(block),
    }
}

#[cfg(test)]
mod tests {
    use super::view;

    fn rows(source: &str) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 12));
        t.pump_widget(view(source));
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }

    #[test]
    fn a_heading_keeps_its_hashes_at_every_level() {
        let drawn = rows("# One\n\n###### Six");
        assert_eq!(drawn, ["# One", "###### Six"]);
    }

    #[test]
    fn headings_in_a_list_item_and_a_quote_keep_their_hashes() {
        let drawn = rows("- ## Item head\n\n> ### Quoted head");
        assert!(
            drawn.iter().any(|row| row.ends_with("## Item head")),
            "{drawn:?}"
        );
        assert!(
            drawn.iter().any(|row| row.ends_with("### Quoted head")),
            "{drawn:?}"
        );
    }

    #[test]
    fn a_heading_wraps_with_its_marker_in_the_first_row() {
        let drawn = rows("## a heading long enough to wrap");
        assert!(drawn[0].starts_with("## "), "{drawn:?}");
        assert!(drawn.len() > 1, "{drawn:?}");
    }
}
