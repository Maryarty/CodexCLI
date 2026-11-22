//! Cursor-neighborhood resolution for sigil-prefixed composer completions
//! (`bottom_pane/chat_composer/completion_target.rs`, carried): which `$` or
//! `@` token the caret is on, where it starts and ends, and whether its sigil
//! and name are still plain text (an atomic element is already bound and not
//! offered again). Pure: it reads the draft as a [`Doc`] (text, caret and
//! the atomic elements' ranges).

use std::ops::Range;

use super::codec::{is_common_env_var, is_mention_name_char};

/// What a token search reads of a draft: its text, the caret and the ranges
/// of its atomic elements, ordered by start (the oracle's `TextArea`).
#[derive(Clone, Copy, Debug)]
pub struct Doc<'a> {
    pub text: &'a str,
    pub cursor: usize,
    pub elements: &'a [Range<usize>],
}

impl<'a> Doc<'a> {
    fn text(&self) -> &'a str {
        self.text
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    /// The ordered element ranges that overlap `range` (one ending exactly
    /// at its start or starting exactly at its end does not).
    fn text_element_ranges_overlapping(
        &self,
        range: Range<usize>,
    ) -> impl Iterator<Item = &'a Range<usize>> {
        let first = self.elements.partition_point(|element| element.end <= range.start);
        self.elements[first..]
            .iter()
            .take_while(move |element| element.start < range.end)
    }

    fn element_id_for_exact_range(&self, range: Range<usize>) -> Option<usize> {
        self.elements.iter().position(|element| *element == range)
    }
}

/// Whether a plain-text `@` mention ends at `index` (`ends_plaintext_at_mention`).
pub fn ends_plaintext_at_mention(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| {
        byte.is_ascii_whitespace()
            || *byte == b'.'
                && bytes.get(index + 1).is_none_or(|next| {
                    next.is_ascii_whitespace()
                        || !next.is_ascii_alphanumeric() && *next != b'_' && *next != b'-'
                })
            || !matches!(*byte, b'.' | b'/' | b'\\')
                && !byte.is_ascii_alphanumeric()
                && *byte != b'_'
                && *byte != b'-'
    })
}

/// Whether a plain-text `$` mention ends at `index`.
pub fn ends_plaintext_at_dollar_mention(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| !is_mention_name_char(*byte))
}

/// Narrows one whitespace-delimited candidate to the editable segment around `anchor`.
///
/// Atomic elements form hard boundaries. If the anchor is inside an element, the raw candidate is
/// retained only when it is needed to identify that the candidate is already bound.
fn prefixed_candidate_range(
    doc: &Doc,
    range: Range<usize>,
    anchor: usize,
    prefix: char,
    allow_empty: bool,
) -> Option<(Range<usize>, String)> {
    let text = doc.text();
    let prefix_len = prefix.len_utf8();
    let raw_prefixed = text
        .get(range.clone())
        .filter(|token| token.starts_with(prefix))
        .map(|token| (range.clone(), token[prefix_len..].to_string()));
    let mut segment_start = range.start;
    let mut segment_end = range.end;
    let mut has_element_boundary = false;
    let anchor = anchor.clamp(range.start, range.end);

    for element in doc.text_element_ranges_overlapping(range) {
        has_element_boundary = true;
        if element.end <= anchor {
            segment_start = element.end;
            continue;
        }
        if anchor <= element.start {
            segment_end = element.start;
        } else {
            segment_start = anchor;
            segment_end = anchor;
        }
        break;
    }

    if !has_element_boundary {
        return raw_prefixed;
    }

    let segmented_prefixed = text
        .get(segment_start..segment_end)
        .and_then(|token| token.strip_prefix(prefix))
        .filter(|query| allow_empty || !query.is_empty())
        .map(|query| (segment_start..segment_end, query.to_string()));
    if segmented_prefixed.is_some()
        || raw_prefixed.as_ref().is_none_or(|(range, query)| {
            prefixed_token_range_is_editable(doc, prefix, range, query)
        })
    {
        segmented_prefixed
    } else {
        raw_prefixed
    }
}

/// Extracts a token prefixed with `prefix` under the cursor, if any.
///
/// The returned string does not include the prefix. Resolution considers the tokens on both sides
/// of same-line separator whitespace and preserves their ranges for cross-sigil arbitration. It
/// stops editable candidates at atomic text elements, and line breaks never provide affinity.
pub fn current_prefixed_token_range(
    doc: &Doc,
    prefix: char,
    allow_empty: bool,
) -> Option<(Range<usize>, String)> {
    current_prefixed_token_range_with_dollar_predicate(
        doc,
        prefix,
        allow_empty,
        dollar_query_is_completable,
    )
}

/// Extracts a prefixed token while using the provided dollar-query predicate for separator
/// arbitration.
pub fn current_prefixed_token_range_with_dollar_predicate(
    doc: &Doc,
    prefix: char,
    allow_empty: bool,
    dollar_query_is_completable: impl Fn(&str) -> bool,
) -> Option<(Range<usize>, String)> {
    let cursor_offset = doc.cursor();
    let text = doc.text();

    // Adjust the provided byte offset to the nearest valid char boundary at or before it.
    let mut safe_cursor = cursor_offset.min(text.len());
    if safe_cursor < text.len() && !text.is_char_boundary(safe_cursor) {
        safe_cursor = text
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= cursor_offset)
            .last()
            .unwrap_or(0);
    }

    let before_cursor = &text[..safe_cursor];
    let after_cursor = &text[safe_cursor..];
    let is_horizontal_whitespace = |c: char| {
        c.is_whitespace()
            && !matches!(
                c,
                '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
            )
    };

    let at_whitespace = after_cursor.chars().next().is_some_and(char::is_whitespace);
    let after_horizontal_whitespace = before_cursor
        .chars()
        .next_back()
        .is_some_and(is_horizontal_whitespace);
    let cursor_starts_token = after_horizontal_whitespace && !at_whitespace;
    let next_non_separator = after_cursor.chars().find(|c| !is_horizontal_whitespace(*c));
    let separator_precedes_token = next_non_separator.is_some_and(|c| !c.is_whitespace());
    let separator_precedes_completion = next_non_separator.is_some_and(|c| matches!(c, '$' | '@'));
    let at_separator = (at_whitespace || after_horizontal_whitespace) && separator_precedes_token;

    let end_left = if at_separator {
        before_cursor
            .trim_end_matches(is_horizontal_whitespace)
            .len()
    } else {
        let end_left_rel = after_cursor
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(after_cursor.len());
        safe_cursor + end_left_rel
    };
    let start_left = text[..end_left]
        .char_indices()
        .rfind(|(_, c)| c.is_whitespace())
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(0);

    let ws_len_right: usize = after_cursor
        .chars()
        .take_while(|c| is_horizontal_whitespace(*c))
        .map(char::len_utf8)
        .sum();
    let start_right = safe_cursor + ws_len_right;
    let end_right_rel = text[start_right..]
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(text.len() - start_right);
    let end_right = start_right + end_right_rel;
    let token_right = if start_right < end_right {
        Some(&text[start_right..end_right])
    } else {
        None
    };

    let left_prefixed = prefixed_candidate_range(
        doc,
        start_left..end_left,
        safe_cursor,
        prefix,
        allow_empty,
    );
    let right_prefixed = prefixed_candidate_range(
        doc,
        start_right..end_right,
        start_right,
        prefix,
        allow_empty,
    );

    if cursor_starts_token {
        let right_is_bound = token_right
            .and_then(|token| {
                let prefix = token.chars().next()?;
                matches!(prefix, '$' | '@').then_some((prefix, &token[prefix.len_utf8()..]))
            })
            .is_some_and(|(prefix, query)| {
                !prefixed_token_range_is_editable(
                    doc,
                    prefix,
                    &(start_right..end_right),
                    query,
                )
            });
        return if right_is_bound {
            left_prefixed
        } else {
            right_prefixed
        };
    }

    if allow_empty && after_cursor.starts_with(prefix) {
        let left_fragment = &text[start_left..safe_cursor];
        if let Some(left_token) = left_fragment.strip_prefix(prefix)
            && left_token
                .as_bytes()
                .iter()
                .all(|byte| is_mention_name_char(*byte))
        {
            let left_range = start_left..safe_cursor;
            let left_is_editable =
                prefixed_token_range_is_editable(doc, prefix, &left_range, left_token);
            if left_token.is_empty() || prefix == '$' && left_is_editable {
                return Some((left_range, left_token.to_string()));
            }
            if !left_is_editable {
                return left_prefixed.or(right_prefixed);
            }
        }
    }

    if at_separator {
        if after_horizontal_whitespace && !separator_precedes_completion {
            return right_prefixed;
        }
        if prefix == '$'
            && left_prefixed
                .as_ref()
                .is_some_and(|(_, query)| !dollar_query_is_completable(query))
            && right_prefixed
                .as_ref()
                .is_some_and(|(_, query)| dollar_query_is_completable(query))
        {
            return right_prefixed;
        }
        if prefix == '@'
            && right_prefixed.as_ref().is_some_and(|(range, query)| {
                prefixed_token_range_is_editable(doc, prefix, range, query)
            })
        {
            return right_prefixed;
        }
        if left_prefixed.as_ref().is_some_and(|(range, token)| {
            !prefixed_token_range_is_editable(doc, prefix, range, token)
        }) {
            return right_prefixed.or(left_prefixed);
        }
        if left_prefixed
            .as_ref()
            .is_some_and(|(_, token)| token.is_empty())
            && !allow_empty
        {
            return right_prefixed;
        }
        return left_prefixed.or(right_prefixed);
    }
    if after_cursor.starts_with(prefix) {
        let prefix_starts_token = before_cursor
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
        return if prefix_starts_token {
            right_prefixed.or(left_prefixed)
        } else {
            left_prefixed
        };
    }
    left_prefixed.or(right_prefixed)
}

/// Returns whether a token candidate's sigil and mention name are editable plaintext.
///
/// A candidate is bound when either its entire range is atomic or its mention-name prefix is
/// atomic and the remaining text begins with a terminator for that sigil.
pub fn prefixed_token_range_is_editable(
    doc: &Doc,
    prefix: char,
    range: &Range<usize>,
    token: &str,
) -> bool {
    if doc.element_id_for_exact_range(range.clone()).is_some() {
        return false;
    }

    let name_len = token
        .as_bytes()
        .iter()
        .take_while(|byte| is_mention_name_char(**byte))
        .count();
    let mention_end = range.start + prefix.len_utf8() + name_len;
    let ends_bound_mention = if prefix == '@' {
        ends_plaintext_at_mention(doc.text().as_bytes(), mention_end)
    } else {
        ends_plaintext_at_dollar_mention(doc.text().as_bytes(), mention_end)
    };
    !(name_len > 0
        && mention_end < range.end
        && ends_bound_mention
        && doc
            .element_id_for_exact_range(range.start..mention_end)
            .is_some())
}

/// Rejects shell-like dollar syntax while preserving lowercase and plugin-qualified skill queries.
///
/// Uppercase common environment-variable spellings and tokens beginning with a positional or
/// special shell parameter are excluded. The check uses the complete colon-qualified name so a
/// skill such as `home` or `home:search` is not mistaken for `$HOME`.
pub fn dollar_query_is_completable(query: &str) -> bool {
    matches!(dollar_query_kind(query), DollarQueryKind::Completable)
}

/// Classifies text after `$` for arbitration between shell syntax and mention completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DollarQueryKind {
    Completable,
    ShellVariable,
    DefiniteShellParameter,
    /// A digit-leading query containing a non-digit, or a `-` query with a suffix.
    AmbiguousShellParameter,
    Invalid,
}

/// Conservatively classifies a dollar query before the mention catalog is consulted.
///
/// Numeric-only and exact special-parameter forms remain definite shell syntax. A digit-leading
/// query containing a non-digit, or a `-` query with a suffix, is ambiguous because loaded
/// mentions may legally use those spellings.
pub fn dollar_query_kind(query: &str) -> DollarQueryKind {
    let name_end = query
        .as_bytes()
        .iter()
        .take_while(|byte| is_mention_name_char(**byte) || **byte == b':')
        .count();
    let name = &query[..name_end];
    let is_shell_var =
        name.bytes().all(|byte| !byte.is_ascii_lowercase()) && is_common_env_var(name);
    let is_shell_parameter = name
        .as_bytes()
        .first()
        .is_some_and(|byte| *byte == b'-' || byte.is_ascii_digit());
    let is_numeric_parameter = !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit());
    if query.is_empty() {
        DollarQueryKind::Completable
    } else if name_end == 0 {
        DollarQueryKind::Invalid
    } else if is_shell_var {
        DollarQueryKind::ShellVariable
    } else if is_numeric_parameter || matches!(name, "-" | "_") {
        DollarQueryKind::DefiniteShellParameter
    } else if is_shell_parameter {
        DollarQueryKind::AmbiguousShellParameter
    } else {
        DollarQueryKind::Completable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// The oracle's `TextArea` as far as these tests drive it.
    #[derive(Default)]
    struct Fixture {
        text: String,
        cursor: usize,
        elements: Vec<Range<usize>>,
    }

    impl Fixture {
        fn insert_str(&mut self, text: &str) {
            self.text.push_str(text);
            self.cursor = self.text.len();
        }

        fn add_element_range(&mut self, range: Range<usize>) {
            self.elements.push(range);
            self.elements.sort_by_key(|element| element.start);
        }

        fn set_cursor(&mut self, cursor: usize) {
            self.cursor = cursor.min(self.text.len());
        }

        fn doc(&self) -> Doc<'_> {
            Doc {
                text: &self.text,
                cursor: self.cursor,
                elements: &self.elements,
            }
        }
    }


    #[test]
    fn current_prefixed_token_affinity_does_not_cross_line_break() {
        for (text, prefix, allow_empty) in [
            ("@file\n  continue", '@', false),
            ("continue  \n@file", '@', false),
            ("$skill\n  continue", '$', true),
            ("continue  \n$skill", '$', true),
        ] {
            let mut textarea = Fixture::default();
            textarea.insert_str(text);
            textarea.set_cursor(text.find("  ").expect("indentation present") + 1);

            assert_eq!(
                current_prefixed_token_range(&textarea.doc(), prefix, allow_empty),
                None
            );
        }
    }

    #[test]
    fn current_prefixed_token_affinity_does_not_cross_separator_before_plain_text() {
        for (text, prefix, allow_empty) in [("@old  word", '@', false), ("$old  word", '$', true)] {
            let mut textarea = Fixture::default();
            textarea.insert_str(text);
            textarea.set_cursor(text.find("  ").expect("separator present") + 1);

            assert_eq!(
                current_prefixed_token_range(&textarea.doc(), prefix, allow_empty),
                None
            );
        }
    }

    #[test]
    fn current_prefixed_token_prefers_right_after_adjacent_target() {
        let bound = "@bound";
        let left = "@bound@old";
        let text = "@bound@old  @new";
        let right_start = left.len() + 2;
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.add_element_range(0..bound.len());
        textarea.set_cursor(left.len());

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '@', /*allow_empty*/ false),
            Some((right_start..text.len(), "new".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_prefers_right_at_token_start() {
        let text = "$old $new";
        let right_start = "$old ".len();
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.set_cursor(right_start);

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
            Some((right_start..text.len(), "new".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_falls_back_from_bound_at_token_on_right() {
        let text = "@left  @bound";
        let bound_start = text.find("@bound").expect("bound mention");
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.add_element_range(bound_start..text.len());
        textarea.set_cursor("@left ".len());

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '@', /*allow_empty*/ true),
            Some((0.."@left".len(), "left".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_falls_back_from_bound_dollar_suffix_on_right() {
        for suffix in ["/path", ".config"] {
            let text = ["$left  $bound", suffix].concat();
            let bound_start = text.find("$bound").expect("bound skill mention");
            let bound_end = bound_start + "$bound".len();
            let mut textarea = Fixture::default();
            textarea.insert_str(&text);
            textarea.add_element_range(bound_start..bound_end);
            textarea.set_cursor("$left  ".len());

            assert_eq!(
                current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
                Some((0.."$left".len(), "left".to_string()))
            );
        }
    }

    #[test]
    fn current_prefixed_token_targets_later_segment_between_bound_elements() {
        let text = "$bound1$one$bound2$two$bound3";
        let bound2_start = text.find("$bound2").expect("second bound mention");
        let two_start = text.find("$two").expect("later plaintext target");
        let bound3_start = text.find("$bound3").expect("third bound mention");
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.add_element_range(0.."$bound1".len());
        textarea.add_element_range(bound2_start..two_start);
        textarea.add_element_range(bound3_start..text.len());
        textarea.set_cursor(two_start + "$t".len());

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
            Some((two_start..bound3_start, "two".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_bounds_right_candidate_before_bound_element() {
        let text = "continue @other@bound";
        let other_start = text.find("@other").expect("editable mention");
        let bound_start = text.find("@bound").expect("bound mention");
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.add_element_range(bound_start..text.len());
        textarea.set_cursor(other_start);

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '@', /*allow_empty*/ false),
            Some((other_start..bound_start, "other".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_stops_before_overlapping_element() {
        let editable = "$other";
        let placeholder = "[Pasted Content 1001 chars]";
        let text = format!("{editable}{placeholder}");
        let mut textarea = Fixture::default();
        textarea.insert_str(&text);
        textarea.add_element_range(editable.len()..text.len());
        textarea.set_cursor(editable.len());

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
            Some((0..editable.len(), "other".to_string()))
        );
    }

    #[test]
    fn current_prefixed_token_stops_at_trailing_space_before_line_break() {
        let text = "$figma \nnext";
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.set_cursor("$figma ".len());

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
            None
        );
    }

    #[test]
    fn current_prefixed_token_keeps_nested_dollar_prefix_in_same_token() {
        let text = "$HOME/$USER";
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.set_cursor(text.find("$USER").expect("nested prefix present"));

        assert_eq!(
            current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true),
            Some((0..text.len(), "HOME/$USER".to_string()))
        );
    }

    #[test]
    fn typed_qualified_suffix_preserves_bound_plain_mention() {
        // Qualified skill bindings and history persistence are intentionally deferred. Typing a
        // suffix after a bound plain mention must not reinterpret it as an editable qualified one.
        let bound = "$google-calendar";
        let text = "$google-calendar:availability";
        let mut textarea = Fixture::default();
        textarea.insert_str(text);
        textarea.add_element_range(0..bound.len());
        textarea.set_cursor(text.len());

        let (range, token) = current_prefixed_token_range(&textarea.doc(), '$', /*allow_empty*/ true)
            .expect("dollar token should be present");
        assert_eq!((range.clone(), token.as_str()), (0..text.len(), &text[1..]));
        assert!(!prefixed_token_range_is_editable(
            &textarea.doc(),
            '$',
            &range,
            &token,
        ));
    }

    #[test]
    fn dollar_query_classifies_shell_and_skill_syntax() {
        assert!(!dollar_query_is_completable("{CODEX_HOME}/config.toml"));
        assert!(!dollar_query_is_completable("(pwd)"));
        assert!(dollar_query_is_completable("home:search"));
        assert!(dollar_query_is_completable("home"));
        assert!(!dollar_query_is_completable("HOME"));
        for query in ["0", "1", "12", "-", "_"] {
            assert!(!dollar_query_is_completable(query));
            assert_eq!(
                dollar_query_kind(query),
                DollarQueryKind::DefiniteShellParameter
            );
        }
        for query in ["1_suffix", "1foo", "-x"] {
            assert!(!dollar_query_is_completable(query));
            assert_eq!(
                dollar_query_kind(query),
                DollarQueryKind::AmbiguousShellParameter
            );
        }
    }
    
}
