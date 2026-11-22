//! What the chat shows of an assistant message while it streams (oracle
//! `streaming/prose_preview.rs`): the lines the newline committed in full,
//! and of the unfinished last line only what cannot change meaning once more
//! text arrives. A link's destination, a follow-up directive's prompt and a
//! tail that may turn out to be table, quote, indented or fenced stay out
//! until the line is complete or the message is.
//!
//! The oracle keeps this as scanner state fed by each chunk; here it is a
//! function of the text, read as if the tail had arrived one character at a
//! time (the state the oracle's own tests walk through): the message holds
//! no second writer and a chunk that carries structure and text at once
//! shows what the same text arriving in pieces shows. Departure, written in
//! the pack: the oracle's 8 KiB window over a very long tail (`…` above the
//! last 8192 bytes) is not carried; the whole safe part of the tail shows.

/// The part of `text`, an assistant message still streaming, that the chat
/// shows, as a byte length.
pub fn visible_len(text: &str) -> usize {
    let committed = text.rfind('\n').map_or(0, |at| at + 1);
    committed + Tail::default().rich_safe_len(&text[committed..])
}

/// `text` as shown while it streams.
pub fn visible(text: &str) -> &str {
    &text[..visible_len(text)]
}

/// The scanner of one append-only unfinished line, reset when a newline
/// commits it.
#[derive(Default)]
struct Tail {
    scanned_len: usize,
    safe_len: usize,
    rich_safe_len: usize,
    has_pipe: bool,
    link: LinkPreview,
}

impl Tail {
    /// The shown length of `tail` after its characters arrived one by one.
    fn rich_safe_len(mut self, tail: &str) -> usize {
        for (index, character) in tail.char_indices() {
            self.update(&tail[..index + character.len_utf8()]);
        }
        self.shown()
    }

    fn shown(&self) -> usize {
        self.rich_safe_len
    }

    fn update(&mut self, source: &str) {
        // Only scan newly arrived bytes, including on very long single-line responses.
        self.has_pipe |= source[self.scanned_len..].contains('|');
        // Indented and quoted lines may belong to nested code blocks. Keep their
        // existing newline holdback instead of guessing at the missing block context.
        let advances = !(self.has_pipe
            || source.starts_with([' ', '\t', '>'])
            || source.starts_with("```")
            || source.starts_with("~~~")
            || matches!(source, "`" | "``" | "~" | "~~"));
        // Keep tentative fence delimiters in the scanner, but stop collecting ranges once
        // block structure freezes this tail until newline commitment.
        if advances || source.len() <= 2 {
            self.link.scan(source, self.scanned_len);
        }
        if advances {
            self.safe_len = source.len();
            self.rich_safe_len = source.len();
        }
        self.scanned_len = source.len();
        // Keep the structural bound independent of the rich-only link holdback.
        if let Some((start, _)) = self.link.destination {
            self.rich_safe_len = self.rich_safe_len.min(start);
        }
        if let Some(start) = self.link.followup_start {
            self.rich_safe_len = self.rich_safe_len.min(start);
        }
        if let Some(start) = self.link.code_followup_start
            && self.link.pending_ticks != self.link.code_ticks
        {
            self.rich_safe_len = self.rich_safe_len.min(start);
        }
    }
}

/// Scans append-only source without buffering or rescanning a growing destination.
/// Keep the label visible; the opening `(` and everything after it wait for closure.
#[derive(Default)]
struct LinkPreview {
    brackets: usize,
    closed_label: bool,
    destination: Option<(usize, usize)>,
    delimiter: Option<u8>,
    whitespace: bool,
    escaped: bool,
    code_ticks: usize,
    pending_ticks: usize,
    code_followup_start: Option<usize>,
    followup_start: Option<usize>,
    followup_brackets: usize,
    followup_attributes: Option<AttributeState>,
}

enum AttributeState {
    Key,
    ValueStart,
    Unquoted,
    Quoted(u8),
}

impl LinkPreview {
    fn scan(&mut self, source: &str, start: usize) {
        for (index, byte) in source.bytes().enumerate().skip(start) {
            if self.followup_start.is_some()
                && self.closed_label
                && self.brackets == self.followup_brackets
                && byte != b'{'
            {
                self.followup_start = None;
            }
            if let Some(state) = self.followup_attributes.as_mut() {
                if let AttributeState::Quoted(quote) = state {
                    if byte == *quote && !self.escaped {
                        *state = AttributeState::Key;
                    }
                    // Match the directive parser: only a backslash before the quote escapes it.
                    self.escaped = byte == b'\\';
                } else if byte == b'}' {
                    self.followup_start = None;
                    self.followup_attributes = None;
                } else {
                    *state = match (&*state, byte) {
                        (AttributeState::Key, b'=') => AttributeState::ValueStart,
                        (AttributeState::ValueStart, b'\'' | b'"') => AttributeState::Quoted(byte),
                        (AttributeState::ValueStart, b' ' | b'\t') => AttributeState::ValueStart,
                        (AttributeState::ValueStart, _) => AttributeState::Unquoted,
                        (AttributeState::Unquoted, b' ' | b'\t') => AttributeState::Key,
                        (AttributeState::Unquoted, _) => AttributeState::Unquoted,
                        (AttributeState::Key, _) | (AttributeState::Quoted(_), _) => {
                            AttributeState::Key
                        }
                    };
                }
                continue;
            }
            if self.destination.is_none() {
                if byte == b'`' && (self.code_ticks > 0 || !self.escaped) {
                    self.pending_ticks += 1;
                    self.closed_label = false;
                    continue;
                }
                if self.pending_ticks > 0 {
                    if self.code_ticks == 0 {
                        self.code_ticks = self.pending_ticks;
                    } else if self.code_ticks == self.pending_ticks {
                        self.code_ticks = 0;
                        self.code_followup_start = None;
                    }
                    self.pending_ticks = 0;
                }
                if self.code_ticks > 0 {
                    // Until the code span closes, its opener may still be a literal unmatched
                    // backtick. Withhold directive-like text rather than reveal a live prompt.
                    if byte == b'['
                        && self.code_followup_start.is_none()
                        && let Some(prefix) = source[..index].strip_suffix(":codex-followup")
                    {
                        self.code_followup_start = Some(prefix.len());
                    }
                    continue;
                }
            }
            if self.escaped {
                self.escaped = false;
                self.closed_label = false;
                self.whitespace = false;
                continue;
            }
            if byte == b'\\' {
                self.escaped = true;
                self.closed_label = false;
                continue;
            }
            if let Some((destination_start, depth)) = self.destination.as_mut() {
                if let Some(delimiter) = self.delimiter {
                    if byte == delimiter {
                        self.delimiter = None;
                    }
                } else if byte == b'<'
                    && *depth == 1
                    && (index == *destination_start + 1 || self.whitespace)
                {
                    self.delimiter = Some(b'>');
                } else if matches!(byte, b'\'' | b'"') && self.whitespace && *depth == 1 {
                    self.delimiter = Some(byte);
                } else if byte == b'(' {
                    *depth += 1;
                } else if byte == b')' {
                    *depth -= 1;
                    if *depth == 0 {
                        self.destination = None;
                    }
                }
                self.whitespace = byte.is_ascii_whitespace();
            } else {
                match byte {
                    b'[' => {
                        if self.followup_start.is_none()
                            && let Some(prefix) = source[..index].strip_suffix(":codex-followup")
                            && !prefix.ends_with(':')
                            && prefix
                                .bytes()
                                .rev()
                                .take_while(|byte| *byte == b'\\')
                                .count()
                                % 2
                                == 0
                        {
                            self.followup_start = Some(prefix.len());
                            self.followup_brackets = self.brackets;
                        }
                        self.brackets += 1;
                    }
                    b']' if self.brackets > 0 => {
                        self.brackets -= 1;
                        self.closed_label = true;
                        continue;
                    }
                    b'(' if self.closed_label => {
                        self.destination = Some((index, 1));
                        self.whitespace = false;
                    }
                    b'{' if self.closed_label
                        && self.followup_start.is_some()
                        && self.brackets == self.followup_brackets =>
                    {
                        self.followup_attributes = Some(AttributeState::Key);
                    }
                    _ => {}
                }
                self.closed_label = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `visible` shows after `text` arrived piece by piece.
    fn after(pieces: &[&str]) -> String {
        let mut text = String::new();
        for piece in pieces {
            text.push_str(piece);
        }
        visible(&text).to_string()
    }

    /// `opener` arrives at once after `head` (as the oracle's own tests push it),
    /// then every character of `tail`: the shown text stays `head`.
    fn stays(head: &str, opener: &str, tail: &str) {
        let mut text = format!("{head}{opener}");
        assert_eq!(visible(&text), head, "after {text:?}");
        for character in tail.chars() {
            text.push(character);
            assert_eq!(visible(&text), head, "after {text:?}");
        }
    }

    #[test]
    fn a_complete_line_shows_in_full_and_the_tail_of_prose_shows_as_it_grows() {
        assert_eq!(after(&["one\ntwo"]), "one\ntwo");
        assert_eq!(after(&["one\n"]), "one\n");
        assert_eq!(after(&["a plain line"]), "a plain line");
    }

    #[test]
    fn an_unfinished_link_destination_waits_for_its_closing_parenthesis() {
        // oracle unfinished_link_destinations_do_not_expand_the_preview
        assert_eq!(
            after(&["See [documentation](https://example.com/unfinished"]),
            "See [documentation]"
        );
        assert_eq!(
            after(&["See [documentation](https://example.com/a)"]),
            "See [documentation](https://example.com/a)"
        );
        // ... and flushes with the newline.
        assert_eq!(
            after(&["See [documentation](https://example.com/a/very/long/path\n"]),
            "See [documentation](https://example.com/a/very/long/path\n"
        );
    }

    #[test]
    fn a_destination_stays_hidden_through_nesting_escapes_titles_and_angle_brackets() {
        // oracle unfinished_link_destinations_do_not_expand_the_preview, every character split.
        for destination in [
            "https://example.com/a/very/long/path/to/the/guide",
            "https://example.com/a_(nested_(path))",
            r"https://example.com/escaped\)parenthesis",
            "<https://example.com/unbalanced)parenthesis>",
            " <https://example.com/unbalanced)parenthesis>",
            "https://example.com/guide \"a title with ) parentheses\"",
            "https://example.com/guide (x < y)",
        ] {
            let mut text = String::from("See [documentation]");
            for character in format!("({destination}").chars() {
                text.push(character);
                assert_eq!(
                    visible(&text),
                    "See [documentation]",
                    "{destination}: {text:?}"
                );
            }
            text.push_str(") and continue.");
            assert_eq!(visible(&text), text, "{destination}");
        }
        // A destination far past any window stays held.
        let long = format!(
            "See [documentation](https://example.com/{}more-path",
            "界".repeat(3000)
        );
        assert_eq!(visible(&long), "See [documentation]");
    }

    #[test]
    fn escapes_code_and_closed_links_show_as_written() {
        // oracle link_preview_respects_code_escapes_and_raw_mode (the rich half).
        for source in [
            r"See \[label](https://example.com/unfinished",
            r"See [label\](https://example.com/unfinished",
            "See `[label](https://example.com/unfinished`",
            "See ``[la`bel](https://example.com/unfinished``",
            "See [label](https://example.com/complete) and prose",
            r"See \:codex-followup[label]{prompt=unfinished",
            "See `:codex-followup[label]{prompt=unfinished`",
        ] {
            assert_eq!(visible(source), source, "{source}");
        }
    }

    #[test]
    fn code_before_an_unfinished_link_shows_and_a_pipe_does_not_release_the_hold() {
        // oracle link_preview_respects_code_escapes_and_raw_mode (the code line) and
        // link_holdback_stays_rich_only_when_a_pipe_freezes_the_preview.
        assert_eq!(
            visible("See `[code](literal` then [label](https://example.com/unfinished"),
            "See `[code](literal` then [label]"
        );
        assert_eq!(visible("See [label](https://example.com"), "See [label]");
        assert_eq!(
            visible("See [label](https://example.com|suffix)"),
            "See [label]"
        );
    }

    #[test]
    fn a_follow_up_directive_stays_hidden_until_it_closes() {
        // oracle followup_preview_hides_prompts_until_complete
        stays(
            "Before ",
            ":codex-followup[",
            r#"Inspect `items[0]`]{prompt="Use \\"}\" and `ticks`""#,
        );
        let done =
            r#"Before :codex-followup[Inspect `items[0]`]{prompt="Use \\"}\" and `ticks`"} after."#;
        assert_eq!(visible(done), done);
        // The oracle's second walk: a long prompt, then it closes.
        let long = format!(
            "Before :codex-followup[Long]{{prompt=\"{}more prompt",
            "界".repeat(3000)
        );
        assert_eq!(visible(&long), "Before ");
    }

    #[test]
    fn follow_up_attribute_quotes_open_only_at_value_boundaries() {
        // oracle followup_preview_only_opens_quotes_at_value_boundaries
        for attributes in [
            "prompt=don't",
            "prompt=foo='bar",
            "prompt=don't other = \"quote } here\"",
            "prompt = 'quote } here'other=don't",
        ] {
            stays(
                "Before ",
                ":codex-followup[",
                &["Open]{", attributes].concat(),
            );
            let done = ["Before :codex-followup[Open]{", attributes, "} after."].concat();
            assert_eq!(visible(&done), done, "{attributes}");
        }
    }

    #[test]
    fn surrounding_unfinished_markdown_keeps_its_text_and_hides_the_prompt() {
        // oracle followup_preview_handles_surrounding_unfinished_markdown
        stays(
            "[unfinished ",
            ":codex-followup[",
            "Inspect]{prompt=\"secret\"",
        );
        for source in [
            ":codex-followup[not a directive] continuing text",
            ":codex-followup[not a directive] `code`",
        ] {
            assert_eq!(visible(source), source, "{source}");
        }
    }

    #[test]
    fn an_unmatched_code_opener_does_not_expose_a_prompt() {
        // oracle unmatched_code_openers_do_not_expose_followup_prompts
        assert_eq!(
            visible("Type ` then :codex-followup[Open]{prompt=secret}"),
            "Type ` then "
        );
        assert_eq!(
            visible("Type `x` then :codex-followup[Open]{prompt=secret}"),
            "Type `x` then :codex-followup[Open]{prompt=secret}"
        );
    }

    #[test]
    fn a_tail_that_may_be_a_table_a_quote_or_code_waits_for_its_newline() {
        assert_eq!(after(&["| a | b |"]), "");
        assert_eq!(after(&["text | more"]), "text ");
        assert_eq!(after(&["    indented"]), "");
        assert_eq!(after(&["> quote"]), "");
        assert_eq!(after(&["```rust"]), "");
        assert_eq!(after(&["`"]), "");
        assert_eq!(after(&["`c"]), "`c");
        assert_eq!(after(&["done\n| a | b |"]), "done\n");
        assert_eq!(after(&["| a | b |\n"]), "| a | b |\n");
    }
}
