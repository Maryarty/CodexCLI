//! File citations in an assistant message: where the directives stand in the
//! source and how each one reads. Pure text logic of the oracle's
//! markdown_render/inline_directives.rs (the offset scan, minus the parser
//! events: the view finds code, HTML and links itself) and
//! markdown_render/local_links.rs (the destination and its display).

use std::borrow::Cow;
use std::ops::Range;
use std::path::Path;

use super::directives::{QuoteEscaping, parse_assistant_directive_with_budget};

/// One `:codex-file-citation{path=…}` directive of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    /// The directive's bytes in the source, colons to closing brace.
    pub range: Range<usize>,
    /// The `path` attribute, never empty.
    pub path: String,
}

/// The citations of `input` in source order. A directive that is escaped
/// (an odd run of backslashes before it), follows a colon, or lies inside
/// another directive is not one; neither is one without a path. Where a
/// citation stands in code, HTML, a link or a reference definition is not
/// known here: those are the parser's, and the view puts the text back.
pub fn citations(input: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    if !input.contains("codex-file-citation") {
        return found;
    }
    let mut directive_end = 0;
    // Preferred and fallback allowances are shared across offsets, so fallback retries cannot
    // starve later preferred parses; both stay proportional to the input.
    let mut scan_budget = [input.len().saturating_mul(4); 2];
    for (start, _) in input.match_indices(':') {
        if start < directive_end || input[..start].ends_with(':') {
            continue;
        }
        let source = &input[start..];
        // Citations prefer literal quoting; other directives prefer escaped quotes.
        let escaping = if source
            .trim_start_matches(':')
            .starts_with("codex-file-citation{")
        {
            [QuoteEscaping::Literal, QuoteEscaping::Backslash]
        } else {
            [QuoteEscaping::Backslash, QuoteEscaping::Literal]
        };
        let Some(directive) =
            escaping
                .into_iter()
                .zip(&mut scan_budget)
                .find_map(|(escaping, remaining)| {
                    parse_assistant_directive_with_budget(source, escaping, remaining)
                })
        else {
            continue;
        };
        let end = start + directive.raw.len();
        directive_end = end;
        let escaped = input[..start]
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\\')
            .count()
            % 2
            != 0;
        if escaped || directive.name != "codex-file-citation" {
            continue;
        }
        if let Some(path) = directive
            .attributes
            .get("path")
            .filter(|path| !path.is_empty())
        {
            found.push(Citation {
                range: start..end,
                path: path.to_string(),
            });
        }
    }
    found
}

/// What a citation shows: its path as the oracle's local link renders it,
/// relative to the session `cwd` where it lies under it, separators as
/// forward slashes. A path that is not absolute is read against `cwd`.
pub fn label(path: &str, cwd: Option<&str>) -> String {
    let destination = destination(path, cwd);
    match target(&destination) {
        Some(target) => display(&target, cwd),
        None => destination,
    }
}

/// The link destination the oracle gives a citation: the path itself when it
/// looks local, else joined to `cwd` (`./` without one); `%`, `#` and `?`
/// are percent-encoded because citation paths are literal, and a trailing
/// `:line[:col]` has its colons encoded.
pub(crate) fn destination(path: &str, cwd: Option<&str>) -> String {
    let destination = if is_local_path_like(path) {
        path.to_owned()
    } else {
        cwd.map_or_else(
            || format!("./{path}"),
            |cwd| Path::new(cwd).join(path).to_string_lossy().into_owned(),
        )
    };
    let mut destination = destination
        .replace('%', "%25")
        .replace('#', "%23")
        .replace('?', "%3F");
    if let Some(suffix) = colon_location_suffix(&destination) {
        let start = destination.len() - suffix.len();
        destination.replace_range(start.., &suffix.replace(':', "%3A"));
    }
    destination
}

pub(crate) fn is_local_path_like(destination: &str) -> bool {
    destination.starts_with("file://")
        || destination.starts_with('/')
        || destination.starts_with("~/")
        || destination.starts_with("./")
        || destination.starts_with("../")
        || destination.starts_with("\\\\")
        || matches!(
            destination.as_bytes(),
            [drive, b':', separator, ..]
                if drive.is_ascii_alphabetic() && matches!(separator, b'/' | b'\\')
        )
}

/// A trailing `:line`, `:line:col` or range (`:1-2`, `:1:2-3:4`, en dash too).
pub(crate) fn colon_location_suffix(text: &str) -> Option<&str> {
    // Walk back over the tail made of digits, `:`, `-` and `–`, then take the longest
    // suffix of it that has the shape `:\d+(:\d+)?([-–]\d+(:\d+)?)?`.
    let mut start = text.len();
    while start > 0 {
        let previous = text[..start].chars().next_back()?;
        if previous.is_ascii_digit() || matches!(previous, ':' | '-' | '–') {
            start -= previous.len_utf8();
        } else {
            break;
        }
    }
    text[start..]
        .match_indices(':')
        .map(|(index, _)| &text[start + index..])
        .find(|candidate| is_location(candidate))
}

fn is_location(text: &str) -> bool {
    fn digits(text: &str) -> Option<&str> {
        let end = text.bytes().take_while(u8::is_ascii_digit).count();
        (end > 0).then(|| &text[end..])
    }
    fn position(text: &str) -> Option<&str> {
        let rest = digits(text)?;
        match rest.strip_prefix(':') {
            Some(after) => Some(digits(after).unwrap_or(rest)),
            None => Some(rest),
        }
    }
    let Some(rest) = text.strip_prefix(':').and_then(position) else {
        return false;
    };
    rest.is_empty()
        || rest
            .strip_prefix(['-', '–'])
            .and_then(position)
            .is_some_and(str::is_empty)
}

/// The path text a destination stands for: a `file://` URL's path (`None`
/// where the URL does not parse), else the destination percent-decoded.
pub(crate) fn target(destination: &str) -> Option<String> {
    let Some(rest) = destination.strip_prefix("file://") else {
        return Some(percent_decode(destination).into_owned());
    };
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    if authority.contains(['[', ']', ' ']) || path.is_empty() {
        return None;
    }
    let path = percent_decode(path);
    let path = match path.as_bytes() {
        // `/C:/repo` names a Windows drive.
        [b'/', drive, b':', b'/' | b'\\', ..] if drive.is_ascii_alphabetic() => &path[1..],
        _ => &path[..],
    };
    Some(
        if authority.is_empty() || authority.eq_ignore_ascii_case("localhost") {
            path.to_owned()
        } else {
            format!("//{authority}{path}")
        },
    )
}

pub(crate) fn percent_decode(text: &str) -> Cow<'_, str> {
    if !text.contains('%') {
        return Cow::Borrowed(text);
    }
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let pair = bytes.get(index + 1..index + 3).and_then(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            u8::try_from(high * 16 + low).ok()
        });
        match (bytes[index], pair) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                index += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    match String::from_utf8(decoded) {
        Ok(decoded) => Cow::Owned(decoded),
        Err(_) => Cow::Borrowed(text),
    }
}

/// Forward slashes only; `\\server\share` becomes `//server/share`.
pub(crate) fn normalize(path: &str) -> String {
    match path.strip_prefix("\\\\") {
        Some(rest) => format!("//{}", rest.replace('\\', "/").trim_start_matches('/')),
        None => path.replace('\\', "/"),
    }
}

fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
        || matches!(path.as_bytes(), [drive, b':', b'/', ..] if drive.is_ascii_alphabetic())
}

/// The path without trailing separators, roots (`/`, `//`, `C:/`) kept.
fn trim_separator(path: &str) -> &str {
    if path == "/" || path == "//" {
        return path;
    }
    if matches!(path.as_bytes(), [drive, b':', b'/'] if drive.is_ascii_alphabetic()) {
        return path;
    }
    path.trim_end_matches('/')
}

/// What stays of `path` below `cwd`; `None` where it is not strictly under it.
fn strip_cwd<'a>(path: &'a str, cwd: &str) -> Option<&'a str> {
    let path = trim_separator(path);
    let cwd = trim_separator(cwd);
    if path == cwd {
        return None;
    }
    if cwd == "/" || cwd == "//" {
        return path.strip_prefix('/');
    }
    path.strip_prefix(cwd)
        .and_then(|rest| rest.strip_prefix('/'))
}

pub(crate) fn display(path: &str, cwd: Option<&str>) -> String {
    let path = normalize(path);
    if !is_absolute(&path) {
        return path;
    }
    if let Some(cwd) = cwd
        && let Some(stripped) = strip_cwd(&path, &normalize(cwd))
    {
        return stripped.to_owned();
    }
    path
}
