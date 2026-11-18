//! Case-insensitive subsequence matcher (`codex-rs/utils/fuzzy-match`).

/// The indices (character positions) of the matched characters in the
/// original `haystack` and a score where smaller is better.
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<(Vec<usize>, i32)> {
    let _ = (haystack, needle);
    todo!()
}
