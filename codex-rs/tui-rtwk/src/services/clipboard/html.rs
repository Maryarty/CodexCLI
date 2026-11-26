//! The HTML flavour of a Markdown copy (oracle `clipboard_html.rs`): what a rich-text
//! application receives when the user pastes a copied response.
//!
//! Raw HTML is escaped, and destinations use the web-link policy: a destination that is not an
//! http(s) URL with a host stays visible as escaped text after its label. The original Markdown
//! remains the plain-text clipboard representation. Images render only their alt text so a paste
//! cannot trigger remote image requests.

use codex_tui_core::data::links::web_destination;
use pulldown_cmark::Event;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;

use crate::pages::session::cells::markdown::lift_markdown_fences;

/// `markdown` as HTML (oracle `render_markdown`): table-holding Markdown fences are unwrapped
/// first, as the transcript shows them.
pub fn render_markdown(markdown: &str) -> String {
    let normalized = lift_markdown_fences(markdown);
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let mut destinations = Vec::new();
    let events = Parser::new_ext(&normalized, options).filter_map(|event| {
        Some(match event {
            Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => return None,
            Event::Start(mut tag) => {
                if let Tag::Link { dest_url, .. } = &mut tag {
                    if let Some(destination) = web_destination(dest_url) {
                        *dest_url = destination.into();
                        destinations.push(None);
                    } else {
                        destinations.push(Some(dest_url.clone()));
                        return None;
                    }
                }
                Event::Start(tag)
            }
            Event::End(TagEnd::Link) => match destinations.pop().flatten() {
                Some(destination) => Event::Text(format!(" ({destination})").into()),
                None => Event::End(TagEnd::Link),
            },
            event => event,
        })
    });
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    html
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::render_markdown;

    #[test]
    fn rich_formatting_is_kept() {
        let markdown = "# Findings\n\nRead [the source](https://example.com/source?q=1&v=2).\n\n\
            - **Bold**, *italic*, ~~removed~~, and `inline <code>`\n\n\
            > A quoted finding.\n\n\
            ```rust\nfn main() { println!(\"<hello>\"); }\n```\n\n\
            | Item | Result |\n| --- | --- |\n| Copy | Formatted |\n";
        assert_eq!(
            render_markdown(markdown),
            "<h1>Findings</h1>\n\
             <p>Read <a href=\"https://example.com/source?q=1&amp;v=2\">the source</a>.</p>\n\
             <ul>\n\
             <li><strong>Bold</strong>, <em>italic</em>, <del>removed</del>, and <code>inline &lt;code&gt;</code></li>\n\
             </ul>\n\
             <blockquote>\n\
             <p>A quoted finding.</p>\n\
             </blockquote>\n\
             <pre><code class=\"language-rust\">fn main() { println!(\"&lt;hello&gt;\"); }\n\
             </code></pre>\n\
             <table><thead><tr><th>Item</th><th>Result</th></tr></thead><tbody>\n\
             <tr><td>Copy</td><td>Formatted</td></tr>\n\
             </tbody></table>\n"
        );
    }

    #[test]
    fn local_link_destinations_stay_visible() {
        let markdown = r#"[parser](/repo/src/parser.rs:42)

[**Windows parser**](C:\repo\src\parser.rs:42:7)

[`relative parser`](../src/parser.rs#L42-L48)

[file URL](file:///repo/src/parser.rs#L42)

[escaped path](/repo/a&amp;b/&lt;parser&gt;.rs:42)

[Unicode path](</repo/my project/解析.rs:42>)

[![diagram](https://example.com/diagram.png)](/repo/diagram.rs:7)

[web link](https://example.com/source) and [another file](src/lib.rs:9)
"#;
        assert_eq!(
            render_markdown(markdown),
            "<p>parser (/repo/src/parser.rs:42)</p>\n\
             <p><strong>Windows parser</strong> (C:\\repo\\src\\parser.rs:42:7)</p>\n\
             <p><code>relative parser</code> (../src/parser.rs#L42-L48)</p>\n\
             <p>file URL (file:///repo/src/parser.rs#L42)</p>\n\
             <p>escaped path (/repo/a&amp;b/&lt;parser&gt;.rs:42)</p>\n\
             <p>Unicode path (/repo/my project/解析.rs:42)</p>\n\
             <p>diagram (/repo/diagram.rs:7)</p>\n\
             <p><a href=\"https://example.com/source\">web link</a> and another file (src/lib.rs:9)</p>\n"
        );
    }

    #[test]
    fn images_are_inert() {
        assert_eq!(
            render_markdown(
                "![tracking pixel](https://example.com/pixel)\n\n\
                 [![**Preview** & details](https://example.com/image.png)](https://example.com/page)"
            ),
            "<p>tracking pixel</p>\n\
             <p><a href=\"https://example.com/page\"><strong>Preview</strong> &amp; details</a></p>\n"
        );
    }

    #[test]
    fn raw_html_is_escaped_and_unsafe_destinations_are_text() {
        assert_eq!(
            render_markdown("<script>alert(1)</script>\n\nHello <img src=x onerror=alert(1)>!"),
            "&lt;script&gt;alert(1)&lt;/script&gt;\n<p>Hello &lt;img src=x onerror=alert(1)&gt;!</p>\n"
        );
        for (destination, visible) in [
            ("javascript:alert(1)", "javascript:alert(1)"),
            ("JaVaScRiPt:alert(1)", "JaVaScRiPt:alert(1)"),
            ("jav&#x61;script:alert(1)", "javascript:alert(1)"),
            ("data:text/html,unsafe", "data:text/html,unsafe"),
            (
                "javascript:alert(&quot;&lt;&gt;&amp;&quot;)",
                "javascript:alert(\"&lt;&gt;&amp;\")",
            ),
        ] {
            assert_eq!(
                render_markdown(&format!("[link]({destination}) ![image]({destination})")),
                format!("<p>link ({visible}) image</p>\n")
            );
        }
    }

    #[test]
    fn a_table_fence_is_unwrapped() {
        let table = "| Item | Result |\n| --- | --- |\n| Copy | Formatted |\n";
        assert_eq!(
            render_markdown(&format!("```markdown\n{table}```")),
            render_markdown(table)
        );
    }
}
