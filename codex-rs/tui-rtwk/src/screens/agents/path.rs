//! G37 (`docs/rtwk-port/rtwk-gaps.md`): the one hand text function of the
//! command center. No `TextOverflow` mode cuts a path by its segments (first
//! ones and the last two kept, the middle ones behind a `…`), which is what
//! the oracle's group heading does; the cut is held to the oracle's four
//! tests. Oracle: `text_formatting.rs` `center_truncate_path`.

use rtwk_core::unicode::TextOverflow;
use rtwk_core::unicode::TextSegmenter;
use rtwk_core::unicode::fit_text;

/// The cells `text` takes on the screen.
fn columns(seg: TextSegmenter, text: &str) -> usize {
    seg.width(text)
}

/// The end of `original` that fits `allowed` cells behind a `…`.
fn front_truncate(seg: TextSegmenter, original: &str, allowed: usize) -> String {
    if allowed == 0 {
        return String::new();
    }
    if columns(seg, original) <= allowed {
        return original.to_owned();
    }
    if allowed == 1 {
        return "\u{2026}".to_owned();
    }
    let room = isize::try_from(allowed).unwrap_or(isize::MAX);
    fit_text(
        &seg,
        original,
        room,
        TextOverflow::EllipsisStart,
        "\u{2026}",
    )
    .into_owned()
}

/// One segment of a path while it is cut to fit.
struct Piece<'a> {
    original: &'a str,
    text: String,
    cuttable: bool,
    tail: bool,
}

fn assemble(leading: bool, pieces: &[Piece<'_>]) -> String {
    let mut path = String::new();
    if leading {
        path.push('/');
    }
    for piece in pieces {
        if !path.is_empty() && !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(&piece.text);
    }
    path
}

/// Cuts `pieces` until the path fits `max` cells: the segment that may be
/// cut, tail ones first, loses its start behind a `…`.
fn fit_pieces(
    seg: TextSegmenter,
    leading: bool,
    pieces: &mut [Piece<'_>],
    max: usize,
    cut: bool,
    count: usize,
) -> Option<String> {
    loop {
        let candidate = assemble(leading, pieces);
        let width = columns(seg, &candidate);
        if width <= max {
            return Some(candidate);
        }
        if !cut {
            return None;
        }
        let order: Vec<usize> = (0..pieces.len())
            .rev()
            .filter(|at| pieces[*at].cuttable && pieces[*at].tail)
            .chain(
                (0..pieces.len())
                    .rev()
                    .filter(|at| pieces[*at].cuttable && !pieces[*at].tail),
            )
            .collect();
        let mut changed = false;
        for at in order {
            if columns(seg, pieces[at].original) <= max && count > 2 {
                continue;
            }
            let others = width.saturating_sub(columns(seg, &pieces[at].text));
            let allowed = max.saturating_sub(others).max(1);
            let text = front_truncate(seg, pieces[at].original, allowed);
            if text != pieces[at].text {
                pieces[at].text = text;
                changed = true;
                break;
            }
        }
        if !changed {
            return None;
        }
    }
}

/// A path cut to `max` cells with a `…` in place of its middle segments,
/// its first segments and its last two kept as the room allows (oracle:
/// `text_formatting.rs` `center_truncate_path`, held to its four tests).
pub fn center_truncate_path(seg: TextSegmenter, path: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if columns(seg, path) <= max {
        return path.to_owned();
    }
    let leading = path.starts_with('/');
    let mut segments: Vec<&str> = path.split('/').collect();
    if leading && segments.first().is_some_and(|first| first.is_empty()) {
        segments.remove(0);
    }
    if path.ends_with('/') && segments.last().is_some_and(|last| last.is_empty()) {
        segments.pop();
    }
    let count = segments.len();
    if count == 0 {
        return if leading && max >= 1 {
            "/".to_owned()
        } else {
            "\u{2026}".to_owned()
        };
    }
    let suffix = if count > 1 { count.min(3) - 1 } else { 0 };
    let mut combos: Vec<(usize, usize)> = (1..=count)
        .flat_map(|left| {
            let least = usize::from(left != count);
            (least..=count - left).map(move |right| (left, right))
        })
        .collect();
    let by_room = |a: &(usize, usize), b: &(usize, usize)| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then((b.0 + b.1).cmp(&(a.0 + a.1)))
    };
    combos.sort_by(by_room);
    let (preferred, rest): (Vec<_>, Vec<_>) =
        combos.into_iter().partition(|(_, right)| *right >= suffix);
    for (left, right) in preferred.into_iter().chain(rest) {
        let mut pieces: Vec<Piece<'_>> = segments[..left]
            .iter()
            .map(|seg| Piece {
                original: seg,
                text: (*seg).to_owned(),
                cuttable: true,
                tail: false,
            })
            .collect();
        let ellipsis = left + right < count;
        if ellipsis {
            pieces.push(Piece {
                original: "\u{2026}",
                text: "\u{2026}".to_owned(),
                cuttable: false,
                tail: false,
            });
        }
        pieces.extend(segments[count - right..].iter().map(|seg| Piece {
            original: seg,
            text: (*seg).to_owned(),
            cuttable: true,
            tail: true,
        }));
        if let Some(fit) = fit_pieces(seg, leading, &mut pieces, max, ellipsis || count <= 2, count) {
            return fit;
        }
    }
    front_truncate(seg, path, max)
}

/// A group heading's path cut to what a row of `width` cells leaves behind
/// the `count` and its two blanks, at most 64 cells (oracle: `heading`).
pub fn fit_heading(seg: TextSegmenter, path: &str, count: &str, width: usize) -> String {
    let room = width.saturating_sub(columns(seg, count) + 2).min(64);
    center_truncate_path(seg, path, room)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_truncate_doesnt_truncate_short_path() {
        assert_eq!(
            center_truncate_path(TextSegmenter::default(), "/Users/codex/Public", 40),
            "/Users/codex/Public"
        );
    }

    #[test]
    fn test_center_truncate_truncates_long_path() {
        assert_eq!(
            center_truncate_path(TextSegmenter::default(), "~/hello/the/fox/is/very/fast", 24),
            "~/hello/the/\u{2026}/very/fast"
        );
    }

    #[test]
    fn test_center_truncate_truncates_long_windows_path() {
        let path = "C:/Users/codex/Projects/super/long/windows/path/file.txt";
        assert_eq!(
            center_truncate_path(TextSegmenter::default(), path, 36),
            "C:/Users/codex/\u{2026}/path/file.txt"
        );
    }

    #[test]
    fn test_center_truncate_handles_long_segment() {
        assert_eq!(
            center_truncate_path(TextSegmenter::default(), "~/supercalifragilisticexpialidocious", 18),
            "~/\u{2026}cexpialidocious"
        );
    }
}
