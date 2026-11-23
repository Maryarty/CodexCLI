//! Binding a history entry's mentions back to its text (`chat_composer.rs`
//! `find_next_mention_token_range`, `bind_mentions_from_snapshot`): an entry
//! keeps `$skill` and `@plugin` as bindings, the draft that recalls it makes
//! each one an atomic element again, at the next place its token stands as
//! a word of its own.

use std::ops::Range;

use super::codec::is_mention_name_char;
use super::target::{ends_plaintext_at_dollar_mention, ends_plaintext_at_mention};

/// The range of the next `token` (sigil and name) in `text` from `from` on
/// that is a mention of its own: an `@` not inside an email address or a
/// longer path, a `$` not inside a longer name.
pub fn find_next_mention_token_range(text: &str, token: &str, from: usize) -> Option<Range<usize>> {
    if token.is_empty() || from >= text.len() {
        return None;
    }
    let bytes = text.as_bytes();
    let token_bytes = token.as_bytes();
    let sigil = *token_bytes.first()?;
    let mut index = from;
    while index < bytes.len() {
        if bytes[index] != sigil {
            index += 1;
            continue;
        }
        let end = index.saturating_add(token_bytes.len());
        if end > bytes.len() {
            return None;
        }
        if &bytes[index..end] != token_bytes {
            index += 1;
            continue;
        }
        let starts = if sigil == b'@' {
            starts_plaintext_at_mention(text, index)
        } else {
            true
        };
        let ends = if sigil == b'@' {
            ends_plaintext_at_mention(bytes, end)
        } else {
            ends_plaintext_at_dollar_mention(bytes, end)
        };
        if starts && ends {
            return Some(index..end);
        }
        index = end;
    }
    None
}

fn starts_plaintext_at_mention(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    text.get(..index)
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|ch| ch.is_whitespace() || !(ch.is_ascii() && is_mention_name_char(ch as u8)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_found_as_a_word_of_its_own() {
        assert_eq!(
            find_next_mention_token_range("use $sample now", "$sample", 0),
            Some(4..11)
        );
        assert_eq!(
            find_next_mention_token_range("$sample", "$sample", 0),
            Some(0..7)
        );
        // a longer name is another mention
        assert_eq!(
            find_next_mention_token_range("$sample-x", "$sample", 0),
            None
        );
    }

    #[test]
    fn an_at_token_is_no_email_address_or_path() {
        assert_eq!(
            find_next_mention_token_range("a@sample.com", "@sample", 0),
            None
        );
        assert_eq!(
            find_next_mention_token_range("@sample/pkg", "@sample", 0),
            None
        );
        assert_eq!(
            find_next_mention_token_range("see @sample.", "@sample", 0),
            Some(4..11)
        );
    }

    #[test]
    fn the_search_starts_where_the_last_binding_ended() {
        let text = "$a $a";
        assert_eq!(find_next_mention_token_range(text, "$a", 0), Some(0..2));
        assert_eq!(find_next_mention_token_range(text, "$a", 2), Some(3..5));
        assert_eq!(find_next_mention_token_range(text, "$a", 5), None);
    }
}
