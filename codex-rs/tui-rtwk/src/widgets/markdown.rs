//! The Markdown of the transcript as the oracle draws it. rtwk's `MarkdownView` differs from
//! the oracle's renderer in a few places the oracle's tests pin; the parsed tree is rewritten
//! before the view shows it (rtwk-rich takes no element builders):
//!
//! - headings keep their source markers: `## Title` is drawn with its two hashes in the
//!   heading's own style (markdown_render.rs `start_heading`; G73 in rtwk-gaps.md);
//! - line breaks, local file links and references to them: `inlines.rs`;
//! - TeX math (`$..$`, `$$..$$`, `\(..\)`, `\[..\]`) parses as a formula and the view draws it as
//!   Unicode (rtwk-math), unless the user's `tui.rendering.math` is off: every parse here takes
//!   the preference (`math`), and a widget that parses where it is built reads it
//!   (`math_preference`); `math.rs` is the text of a formula for the readers that do not mount
//!   the view;
//! - a web link's destination follows the host's terminal fact (`Markdown`); only a web
//!   address is a hyperlink (`http`, `https`: the oracle's `web_destination`).
//!
//! Flutter: `MarkdownBody(builders: {'h1': HeadingWithMarker(), 'a': LocalFileLink(cwd)})` under
//! a `Theme` that knows how links show. The view lays out, wraps and styles each rewritten
//! node as it does any text.

mod inlines;
pub(crate) mod math;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::Arc;

use rtwk::provider::Provider;
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};
use rtwk_rich::markdown::{
    Block, Definitions, LinkDestinations, MarkdownLinkTap, MarkdownStream, MarkdownTheme,
    MarkdownView, MdBlock, MdExtensions, MdInline, MdList, MdListItem,
};

use codex_tui_core::app::facts::{Rendering, TuiSettings};
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

/// Whether the user's `tui.rendering.math` draws formulas as Unicode, read where `cx` is built so
/// that a change of the setting redraws the widget. A tree with no app above it (a widget on its
/// own) has the config's default.
///
/// Flutter: `context.select<TuiSettings, bool>((s) => s.rendering.math)`.
pub fn math_preference<S>(cx: &mut Cx<'_, S>) -> bool {
    Provider::<TuiSettings>::maybe_of(cx, true).map_or_else(
        || Rendering::default().math,
        |settings| settings.rendering.math,
    )
}

/// A `MarkdownView` of `source` as the oracle draws it, local file links shown as written;
/// formulas are drawn as the preference `math` says.
pub fn view(source: &str, math: bool) -> MarkdownView {
    view_in(source, None, math)
}

/// `view` with local file links shown relative to the session directory `cwd`.
pub fn view_in(source: &str, cwd: Option<&str>, math: bool) -> MarkdownView {
    let mut stream = stream(source, math);
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
    Markdown {
        content: Content::View(view),
        theme,
    }
}

/// `themed` over the Markdown `text`, parsed where the widget is built: the user's
/// `tui.rendering.math` is read there (`math_preference`), so a caller with no `Cx` of its own
/// (a cell, a prompt, a question) draws formulas as the preference says.
pub fn source(text: impl Into<String>, theme: MarkdownTheme) -> Markdown {
    Markdown {
        content: Content::Source(text.into()),
        theme,
    }
}

/// A [`MarkdownView`] in a theme, its link display from the app's terminal fact.
#[derive(Clone)]
pub struct Markdown {
    content: Content,
    theme: MarkdownTheme,
}

/// What a [`Markdown`] draws: a view parsed by the caller, or the text to parse at build.
#[derive(Clone)]
enum Content {
    View(MarkdownView),
    Source(String),
}

/// What a build of a [`Markdown`] over source text parsed: the same text under the same
/// preference parses to the same view, so a rebuild of a mounted widget (its parent rebuilt, a
/// theme or a link fact changed) takes it again instead of parsing a long prompt once more.
struct TextParse {
    text: String,
    math: bool,
    view: MarkdownView,
}

#[doc(hidden)]
#[derive(Default)]
pub struct MarkdownState {
    parsed: RefCell<Option<TextParse>>,
}

impl MarkdownState {
    /// The view of `text` under the math preference `math`, parsed once for as long as both stay.
    fn view_of(&self, text: &str, math: bool) -> MarkdownView {
        let mut parsed = self.parsed.borrow_mut();
        if let Some(last) = parsed.as_ref()
            && last.math == math
            && last.text == text
        {
            return last.view.clone();
        }
        let view = view(text, math);
        *parsed = Some(TextParse {
            text: text.to_string(),
            math,
            view: view.clone(),
        });
        view
    }
}

impl StatefulWidget for Markdown {
    type State = MarkdownState;

    fn create_state(&self) -> MarkdownState {
        MarkdownState::default()
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
        let parsed = match &widget.content {
            Content::View(parsed) => parsed.clone(),
            Content::Source(text) => self.view_of(text, math_preference(cx)),
        };
        let markdown = parsed
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
    use codex_tui_core::app::facts::{Rendering, TuiSettings};
    use rtwk::provider::Provider;
    use rtwk_rich::markdown::MarkdownTheme;

    use super::{MarkdownState, source, view};

    fn rows_of(widget: impl Into<rtwk::AnyWidget>) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 12));
        t.pump_widget(widget);
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }

    fn rows(text: &str) -> Vec<String> {
        rows_of(view(text, true))
    }

    /// `text` as a `source` widget draws it under the user's `tui.rendering.math`.
    fn rows_with_math(text: &str, math: bool) -> Vec<String> {
        let settings = TuiSettings {
            rendering: Rendering {
                math,
                ..Rendering::default()
            },
            ..TuiSettings::default()
        };
        rows_of(Provider::value(settings).child(source(text, MarkdownTheme::default())))
    }

    /// The preference decides where the widget is built: a formula is Unicode with it on (or no
    /// app above) and stays its source with it off, in the parse the view takes and in the
    /// widget that parses at build.
    #[test]
    fn the_math_preference_decides_what_a_formula_draws_as() {
        assert_eq!(rows_with_math("see $\\alpha$", true), ["see α"]);
        assert_eq!(rows_with_math("see $\\alpha$", false), ["see $\\alpha$"]);
        assert_eq!(
            rows_of(source("see $\\alpha$", MarkdownTheme::default())),
            ["see α"]
        );
        assert_eq!(rows_of(view("see $\\alpha$", true)), ["see α"]);
        assert_eq!(rows_of(view("see $\\alpha$", false)), ["see $\\alpha$"]);
    }

    /// A rebuild of a mounted widget takes the parse of the same text under the same preference
    /// (the blocks are the same allocation); a new text or a new preference parses again.
    #[test]
    fn a_rebuild_takes_the_parse_of_the_same_text_and_a_change_parses_again() {
        use std::rc::Rc;

        let state = MarkdownState::default();
        let blocks = |view: rtwk_rich::markdown::MarkdownView| view.blocks.expect("parsed blocks");
        let first = blocks(state.view_of("see $\\alpha$", true));
        let again = blocks(state.view_of("see $\\alpha$", true));
        assert!(Rc::ptr_eq(&first, &again));
        let other_text = blocks(state.view_of("see $\\beta$", true));
        assert!(!Rc::ptr_eq(&first, &other_text));
        let other_math = blocks(state.view_of("see $\\beta$", false));
        assert!(!Rc::ptr_eq(&other_text, &other_math));
        let kept = blocks(state.view_of("see $\\beta$", false));
        assert!(Rc::ptr_eq(&other_math, &kept));
    }

    /// A widget replaced by one of the same type over another text draws the other text, and a
    /// change of the preference under it draws the formula the other way.
    #[test]
    fn a_mounted_source_widget_follows_its_text_and_the_preference() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 12));
        let settings = |math: bool| TuiSettings {
            rendering: Rendering {
                math,
                ..Rendering::default()
            },
            ..TuiSettings::default()
        };
        let drawn = |t: &rtwk_test::Tester| -> Vec<String> {
            t.screen()
                .lines()
                .map(|line| line.trim_end().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        };
        let mount = |t: &mut rtwk_test::Tester, text: &str, math: bool| {
            t.pump_widget(
                Provider::value(settings(math)).child(source(text, MarkdownTheme::default())),
            );
        };
        mount(&mut t, "see $\\alpha$", true);
        assert_eq!(drawn(&t), ["see α"]);
        mount(&mut t, "see $\\beta$", true);
        assert_eq!(drawn(&t), ["see β"]);
        mount(&mut t, "see $\\beta$", false);
        assert_eq!(drawn(&t), ["see $\\beta$"]);
    }

    /// A local link whose label is only a formula has text in it: the oracle reads the formula as
    /// the text it was written as, so the label shows (as the formula) before the path.
    #[test]
    fn a_local_link_labelled_by_a_formula_shows_the_label_and_the_path() {
        assert_eq!(
            rows("[$\\alpha$](/repo/src/lib.rs)"),
            ["α (/repo/src/lib.rs)"]
        );
        assert_eq!(rows("[ ](/repo/src/lib.rs)"), ["/repo/src/lib.rs"]);
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
