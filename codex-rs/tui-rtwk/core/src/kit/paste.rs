//! A paste as the draft takes it: sanitized, quoted, or put in as a placeholder when it is large
//! (`chat_composer/paste_input.rs`). Pure text rules.

/// A paste longer than this many characters goes into the draft as a
/// placeholder (`chat_composer.rs`, `LARGE_PASTE_CHAR_THRESHOLD`).
pub const LARGE_PASTE_CHAR_THRESHOLD: usize = 1000;

/// A paste as the draft takes it (`apply_paste`): line breaks folded to
/// `\n`, then CSI sequences and control characters removed but for tabs
/// and newlines (`sanitize_user_text`). A lone ESC is dropped on its own;
/// a CSI sequence that never ends takes the rest of the text with it.
pub fn sanitize_paste(pasted: &str) -> String {
    let folded = pasted.replace("\r\n", "\n").replace('\r', "\n");
    let mut rest = folded.as_str();
    let mut out = String::with_capacity(folded.len());
    while let Some(at) =
        rest.find(|ch: char| ch == '\x1b' || ch.is_control() && !matches!(ch, '\n' | '\t'))
    {
        out.push_str(&rest[..at]);
        let mut chars = rest[at..].chars();
        let ch = chars.next().unwrap_or_default();
        let after = chars.as_str();
        match after.strip_prefix('[').filter(|_| ch == '\x1b') {
            // a CSI sequence ends at its final byte, `@` to `~`
            Some(sequence) => match sequence.find(|c: char| ('@'..='~').contains(&c)) {
                Some(end) => rest = &sequence[end + 1..],
                None => return out,
            },
            None => rest = after,
        }
    }
    out.push_str(rest);
    out
}

/// The size in characters of a paste the composer puts in as a placeholder,
/// or none for one it puts in as text (`handle_paste`).
pub fn large_paste(pasted: &str) -> Option<usize> {
    let chars = pasted.chars().count();
    (chars > LARGE_PASTE_CHAR_THRESHOLD).then_some(chars)
}

/// The blockquote rule of `apply_paste` (`chat_composer/paste_input.rs`):
/// a multiline paste on a line that begins with `> ` continues the quote
/// after every newline, a single trailing newline not counting. `current_line`
/// is the draft's line up to where the paste goes; `pasted` has its newlines
/// normalised. None when the paste is not quoted.
pub fn blockquote_paste(current_line: &str, pasted: &str) -> Option<String> {
    if !current_line.starts_with("> ") || !pasted.contains('\n') {
        return None;
    }
    let body = pasted
        .strip_suffix('\n')
        .filter(|body| !body.is_empty())
        .unwrap_or(pasted);
    Some(body.replace('\n', "\n> "))
}
