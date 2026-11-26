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

/// The destination a click can open in the browser, as in the oracle's `web_destination`.
pub fn web_destination(destination: &str) -> Option<String> {
    const MAX_DESTINATION_BYTES: usize = 8 * 1024;
    if destination.len() > MAX_DESTINATION_BYTES {
        return None;
    }
    let clean: String = destination.chars().filter(|ch| !ch.is_control()).collect();
    let url = url::Url::parse(&clean).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(clean)
}

#[cfg(test)]
mod tests {
    use super::web_destination;

    #[test]
    fn click_destination_requires_a_web_scheme_and_host() {
        let url = "https://example.org/a?b=1&c=2";
        assert_eq!(web_destination(url).as_deref(), Some(url));
        assert_eq!(
            web_destination("http://example.org/a\u{0007}b").as_deref(),
            Some("http://example.org/ab"),
        );
        for invalid in [
            "file:///tmp/a",
            "mailto:a@example.org",
            "javascript:1",
            "https://",
            "/tmp/a",
        ] {
            assert_eq!(web_destination(invalid), None, "{invalid}");
        }
        let oversized = ["https://example.org/", &"a".repeat(8192)].concat();
        assert_eq!(web_destination(&oversized), None);
    }
}
