//! The oracle's Markdown headings keep their source markers: `## Title` is
//! drawn with its two hashes in the heading's own style (markdown_render.rs
//! `start_heading`). rtwk's `MarkdownView` draws a heading as its styled text
//! only (G73 in rtwk-gaps.md).
//!
//! Flutter: `MarkdownBody(builders: {'h1': HeadingWithMarker(), ..})`. rtwk-rich
//! takes no element builders, so the parsed blocks are handed over with each
//! heading's marker as the first text of its content (`MarkdownView::blocks`):
//! the view lays out, wraps and styles it as it does any heading text.

use std::sync::Arc;

use rtwk_rich::markdown::{
    Block, ConfiguredMarkdownView, LinkDestinations, MarkdownStream, MarkdownTheme, MarkdownView,
    MdBlock, MdInline, MdList, MdListItem,
};

/// A `MarkdownView` of `source` whose headings carry their source markers.
pub fn view(source: &str) -> MarkdownView {
    let mut stream = MarkdownStream::new(source);
    let blocks = with_heading_markers(stream.blocks());
    MarkdownView::blocks(blocks, stream.definitions().clone())
}

/// `view` in `theme`, a link's destination written after its label as the
/// oracle writes it: `label (https://example.com)`. Flutter: the
/// `MarkdownBody`'s `styleSheet` and an `onTapLink` that shows the target; rtwk
/// writes the destination itself (`LinkDestinations::Auto`).
pub fn themed(view: MarkdownView, theme: MarkdownTheme) -> ConfiguredMarkdownView {
    view.theme(theme).link_destinations(LinkDestinations::Auto)
}

/// `blocks` with the marker (`#` to `######`, and a space) at the start of
/// every heading, in lists and quotes too.
pub fn with_heading_markers(blocks: &[Block]) -> Vec<Block> {
    blocks.iter().map(marked).collect()
}

fn marked(block: &Block) -> Block {
    match &**block {
        MdBlock::Heading { level, children } => {
            let marker = format!("{} ", "#".repeat(usize::from(*level)));
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
