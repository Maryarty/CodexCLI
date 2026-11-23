//! Local file links of a Markdown message: which destinations are local files and how
//! one reads after its label. Pure text logic carried from the oracle's
//! `markdown_render/local_links.rs` (`is_local_path_like_link`, `render_local_link_target`);
//! the destination and display helpers it shares with file citations live in
//! `threads/citations.rs`, which carried them first.

use crate::threads::citations::{
    colon_location_suffix, display as display_path, is_local_path_like as path_like, normalize,
    percent_decode, target,
};

/// Whether `destination` names a local file: `file://`, absolute, `~/`, `./`, `../`, UNC and
/// drive-letter paths.
pub fn is_local_path_like(destination: &str) -> bool {
    path_like(destination)
}

/// The text a local link shows after its label (oracle `render_local_link_target`): the path
/// relative to the session `cwd` where it lies under it, `~/` and relative paths as written,
/// percent escapes decoded, and a `#L74C3` fragment or a `:74:3` suffix as `:74:3`.
/// A `file://` URL that does not parse shows as written.
pub fn display(destination: &str, cwd: Option<&str>) -> String {
    match parse(destination) {
        Some((path, suffix)) => {
            let mut shown = display_path(&path, cwd);
            shown.push_str(&suffix.unwrap_or_default());
            shown
        }
        None => destination.to_owned(),
    }
}

/// `(path text, location suffix)`: the path never carries its `#L..` or `:line[:col]` suffix.
fn parse(destination: &str) -> Option<(String, Option<String>)> {
    if destination.starts_with("file://") {
        let (url, fragment) = match destination.split_once('#') {
            Some((url, fragment)) => (url, Some(fragment)),
            None => (destination, None),
        };
        let url = url.split_once('?').map_or(url, |(url, _)| url);
        let path = normalize(&target(url)?);
        return Some((path, fragment.and_then(hash_location)));
    }
    let mut path = destination;
    let mut suffix = None;
    // `#L..` wins when both forms are present, so `path#L10` is not a path ending in `:10`.
    if let Some((candidate, fragment)) = destination.rsplit_once('#')
        && let Some(location) = hash_location(fragment)
    {
        path = candidate;
        suffix = Some(location);
    }
    if suffix.is_none()
        && let Some(found) = colon_location_suffix(path)
    {
        path = &path[..path.len() - found.len()];
        suffix = Some(found.to_owned());
    }
    Some((normalize(&percent_decode(path)), suffix))
}

/// `L12`, `L12C3`, `L12C3-L14C9` as `:12`, `:12:3`, `:12:3-14:9`; `None` for any other
/// fragment, which stays part of the path text (carried from `codex_utils_string`).
fn hash_location(fragment: &str) -> Option<String> {
    let (start, end) = match fragment.split_once('-') {
        Some((start, end)) => (start, Some(end)),
        None => (fragment, None),
    };
    let mut shown = format!(":{}", point(start)?);
    if let Some(end) = end {
        shown.push('-');
        shown.push_str(&point(end)?);
    }
    Some(shown)
}

/// `L12` or `L12C3` as `12` or `12:3`.
fn point(text: &str) -> Option<String> {
    let rest = text.strip_prefix('L')?;
    let (line, column) = match rest.split_once('C') {
        Some((line, column)) => (line, Some(column)),
        None => (rest, None),
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(line) || column.is_some_and(|column| !digits(column)) {
        return None;
    }
    Some(match column {
        Some(column) => format!("{line}:{column}"),
        None => line.to_owned(),
    })
}
