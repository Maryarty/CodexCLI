//! Case-insensitive subsequence matcher: a copy of `codex-rs/utils/fuzzy-match`
//! (the core crate takes no new dependency for sixty lines).
//!
//! Returns the indices (character positions) of the matched characters in the
//! ORIGINAL `haystack` and a score where smaller is better. The match runs on
//! a lowercased copy and maps each lowered character back to its original
//! index, so the indices stay valid for highlighting even where lowercasing
//! expands a character (İ -> i̇).
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<(Vec<usize>, i32)> {
    if needle.is_empty() {
        return Some((Vec::new(), i32::MAX));
    }

    let mut lowered_chars: Vec<char> = Vec::new();
    let mut lowered_to_orig_char_idx: Vec<usize> = Vec::new();
    for (orig_idx, ch) in haystack.chars().enumerate() {
        for lc in ch.to_lowercase() {
            lowered_chars.push(lc);
            lowered_to_orig_char_idx.push(orig_idx);
        }
    }

    let lowered_needle: Vec<char> = needle.to_lowercase().chars().collect();

    let mut result_orig_indices: Vec<usize> = Vec::with_capacity(lowered_needle.len());
    let mut first_lower_pos: Option<usize> = None;
    let mut last_lower_pos: Option<usize> = None;
    let mut cur = 0usize;
    for &nc in &lowered_needle {
        let mut found_at: Option<usize> = None;
        while cur < lowered_chars.len() {
            if lowered_chars[cur] == nc {
                found_at = Some(cur);
                cur += 1;
                break;
            }
            cur += 1;
        }
        let pos = found_at?;
        result_orig_indices.push(lowered_to_orig_char_idx[pos]);
        first_lower_pos.get_or_insert(pos);
        last_lower_pos = Some(pos);
    }

    // Score from the lowered positions: the span between the first and last
    // hit minus the needle's length, and a prefix match is rewarded by 100.
    let first_lower_pos = first_lower_pos?;
    let last_lower_pos = last_lower_pos.unwrap_or(first_lower_pos);
    let window =
        (last_lower_pos as i32 - first_lower_pos as i32 + 1) - (lowered_needle.len() as i32);
    let mut score = window.max(0);
    if first_lower_pos == 0 {
        score -= 100;
    }

    result_orig_indices.sort_unstable();
    result_orig_indices.dedup();
    Some((result_orig_indices, score))
}
