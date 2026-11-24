//! What the copy picker offers besides the whole response (oracle `markdown.rs`
//! `extract_copy_targets`): each fenced or indented code block and each quote of the message's
//! Markdown source, in the order they stand in it. The source is cut, never the rendering, so
//! what a row copies is what the author wrote (a code block without its fence, a quote without
//! its `>` marks).

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use codex_tui_core::features::copy::strip_line_directives;

/// One thing of a response that is copied alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A code block: its language (the first word of the fence's info string) and its text.
    Code {
        language: Option<String>,
        content: String,
    },
    /// A block quote: its source lines without the `>` markers.
    Quote(String),
}

/// The copy targets of `source`.
pub fn extract(source: &str) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut quote: Option<(Range<usize>, usize, bool)> = None;
    let mut quote_depth = 0usize;
    let mut in_code_block = false;
    let mut code = None;

    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote) => {
                if quote_depth == 0 {
                    quote = Some((range, targets.len(), false));
                }
                quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote) => {
                quote_depth = quote_depth.saturating_sub(1);
                if quote_depth == 0 {
                    quote = None;
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(info) = kind {
                    let language = info
                        .split([',', ' ', '\t'])
                        .next()
                        .filter(|language| !language.is_empty())
                        .map(str::to_owned);
                    code = Some((language, String::new()));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if in_code_block {
                    if let Some((_, content)) = &mut code {
                        if text.starts_with('\n')
                            && range.start > 0
                            && source.as_bytes()[range.start - 1] == b'\r'
                        {
                            content.push('\r');
                        }
                        content.push_str(&text);
                    }
                } else if let Some((span, position, emitted)) = &mut quote
                    && !*emitted
                    && !strip_line_directives(&text).trim().is_empty()
                {
                    let content = source[span.clone()]
                        .split_inclusive('\n')
                        .map(|line| {
                            line.trim_start_matches(' ')
                                .strip_prefix('>')
                                .map_or(line, |line| line.strip_prefix(' ').unwrap_or(line))
                        })
                        .collect::<String>();
                    targets.insert(*position, Target::Quote(content));
                    *emitted = true;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some((language, content)) = code.take() {
                    targets.push(Target::Code { language, content });
                }
            }
            _ => {}
        }
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn code_blocks_and_quotes_come_in_source_order() {
        let source = "Overview\n\n```python\nprint('first')\n```\n\n> Important quote\n\n```sh\necho done\n```\n";
        assert_eq!(
            extract(source),
            vec![
                Target::Code {
                    language: Some("python".to_owned()),
                    content: "print('first')\n".to_owned(),
                },
                Target::Quote("Important quote\n".to_owned()),
                Target::Code {
                    language: Some("sh".to_owned()),
                    content: "echo done\n".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_fence_without_a_language_is_a_plain_block() {
        let targets = extract("```\nx\n```\n");
        assert_eq!(
            targets,
            vec![Target::Code {
                language: None,
                content: "x\n".to_owned(),
            }]
        );
    }

    #[test]
    fn a_quote_of_only_directives_is_no_target() {
        assert_eq!(extract("> ::git-stage{cwd=\"/r\"}\n"), Vec::new());
    }

    #[test]
    fn a_nested_quote_is_one_target_of_the_outer_source() {
        let targets = extract("> outer\n> > inner\n");
        assert_eq!(targets, vec![Target::Quote("outer\n> inner\n".to_owned())]);
    }
}
