//! How a web link of a message is written (plain data the markdown widget and the cells model share).

/// How a web link's destination is written in a message (oracle `markdown_render/web_links.rs`
/// `WebLinkDisplay`): after the label, or not at all because the terminal shows the label as a
/// hyperlink. The oracle reads this once from the terminal (identity, multiplexer, `TERM`, a
/// stdout that is no terminal); the host does the same and sends it as `Ev::LinkDisplay`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkDisplay {
    /// `label (https://example.com)`: the unknown terminal's, so a label never hides the only
    /// usable URL.
    #[default]
    WithDestination,
    /// The styled label alone, a hyperlink to its destination.
    LabelOnly,
}
