//! The replay helper of the screen tests of the route screens: reads a
//! codex-tui insta `.snap` (the oracle, `../tui/src/**/snapshots/`) and
//! compares rows of the screen under test with it. The expected rows are the
//! oracle's text; nothing here is ours to edit when a screen changes.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::PathBuf;

use rtwk_test::Tester;

/// The oracle snapshot called `name`, as a path relative to `codex-rs/tui/src`:
/// `name` is the snapshot's whole file stem
/// (`codex_tui__chatwidget__tests__approval_modal_exec`) or its test name, the
/// tail after the last `__` the file name has before it (`approval_modal_exec`,
/// `transcript_overlay_paginated_history_states`). Panics when no file or
/// several files have it.
pub fn find(name: &str) -> String {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "snap") {
                out.push(path);
            }
        }
    }
    static ALL: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();
    let root = oracle_path("");
    let all = ALL.get_or_init(|| {
        let mut out = Vec::new();
        walk(&root, &mut out);
        out
    });
    let stem = name.strip_suffix(".snap").unwrap_or(name);
    let tail = format!("__{stem}");
    let found: Vec<&PathBuf> = all
        .iter()
        .filter(|p| {
            let file = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            file == stem || file.ends_with(&tail)
        })
        .collect();
    match found.as_slice() {
        [one] => one
            .strip_prefix(&root)
            .unwrap_or(one)
            .to_string_lossy()
            .into_owned(),
        [] => panic!("no oracle snapshot named `{name}` under codex-rs/tui/src"),
        many => panic!(
            "oracle snapshot `{name}` is ambiguous; use the whole file stem. Candidates: {:?}",
            many.iter().map(|p| p.file_stem()).collect::<Vec<_>>()
        ),
    }
}

fn oracle_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tui/src")
        .join(rel)
}

/// The body of an insta snapshot: the lines after the front matter.
fn body(rel: &str) -> Vec<String> {
    let text = std::fs::read_to_string(oracle_path(rel))
        .unwrap_or_else(|err| panic!("oracle snapshot {rel}: {err}"));
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "{rel}: front matter");
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
    }
    lines.map(str::to_string).collect()
}

/// A `Debug`-quoted row (`"text   "`) as the text it holds. The oracle also
/// writes rows as `"text" Hidden by multi-width symbols: [(3, " ")]` and
/// `"text", // hidden by multi-width symbols: [(3, " ")]`: the note is cut.
fn unquote(row: &str) -> String {
    parse_row(row).0
}

/// A row and the columns the oracle's note says wide glyphs hid.
pub fn parse_row(row: &str) -> (String, Vec<usize>) {
    let raw = (row.to_string(), Vec::new());
    let Some(inner) = row.strip_prefix('"') else {
        return raw;
    };
    let mut text = String::new();
    let mut chars = inner.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                let rest = inner[at + 1..].trim_start_matches(',').trim();
                let note = rest
                    .trim_start_matches("//")
                    .trim()
                    .to_ascii_lowercase()
                    .starts_with("hidden by multi-width symbols");
                if !rest.is_empty() && !note {
                    // TestBackend's rows are not escaped: a quote inside the text is the text.
                    text.push('"');
                    continue;
                }
                return (
                    text,
                    if note {
                        hidden_columns(rest)
                    } else {
                        Vec::new()
                    },
                );
            }
            '\\' => match chars.next() {
                Some((_, 'n')) => text.push('\n'),
                Some((_, 't')) => text.push('\t'),
                Some((_, 'u')) => {
                    let mut hex = String::new();
                    for (_, h) in chars.by_ref() {
                        if h == '}' {
                            break;
                        }
                        if h != '{' {
                            hex.push(h);
                        }
                    }
                    match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        Some(ch) => text.push(ch),
                        None => return raw,
                    }
                }
                Some((_, other)) => text.push(other),
                None => return raw,
            },
            c => text.push(c),
        }
    }
    raw
}

/// The columns of `[(14, " "), (16, " ")]` in a hidden-by-multi-width note.
fn hidden_columns(note: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut rest = note;
    while let Some(at) = rest.find('(') {
        rest = &rest[at + 1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse() {
            out.push(n);
        }
    }
    out
}

fn trimmed(rows: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut rows: Vec<String> = rows.into_iter().map(|r| r.trim_end().to_string()).collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

/// The rows of the oracle snapshot `rel` (relative to `codex-rs/tui/src`),
/// unquoted, trailing blanks trimmed.
pub fn rows(rel: &str) -> Vec<String> {
    trimmed(body(rel).iter().map(|r| unquote(r)))
}

/// The rows of the frame `name` of a snapshot of several (see `frames`).
pub fn section(rel: &str, name: &str) -> Vec<String> {
    frame(rel, name).rows
}

/// One stretch of a style listing of a `Buffer` dump: from cell (x, y) on,
/// every cell has this style (ratatui writes a run where the style changes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleRun {
    pub x: usize,
    pub y: usize,
    pub fg: String,
    pub bg: String,
    pub underline: String,
    pub modifier: String,
}

/// One screen of an oracle snapshot: the whole file, or one of the several
/// the test joined into it.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// The frame's header: `loading` of `--- loading ---` (or of
    /// `---- loading ----`), `124 columns` of `124 columns:`, `width=36,
    /// cursor=0`, the label before a `Buffer {` dump. Empty for a snapshot of
    /// one frame. The same header again is `name#2`, `name#3`.
    pub name: String,
    /// The rows, trailing blank rows cut, trailing blanks of each cut.
    pub rows: Vec<String>,
    /// Per row: the columns the oracle's note says wide glyphs hid.
    pub hidden: Vec<Vec<usize>>,
    /// The `Cursor: Some((x, y))` line: `Some(Some(..))` shown, `Some(None)`
    /// hidden, `None` the snapshot says nothing of the cursor.
    pub cursor: Option<Option<(usize, usize)>>,
    /// The style listing of a `Buffer` dump.
    pub styles: Option<Vec<StyleRun>>,
    /// The dump's `area` (width, height): the cells its rows and styles cover.
    pub area: Option<(usize, usize)>,
}

fn header(
    line: &str,
    prev: Option<&String>,
    next: Option<&String>,
) -> Option<(String, Option<String>)> {
    for bars in ["---- ", "--- "] {
        let tail = bars.trim_end();
        if let Some(name) = line
            .strip_prefix(bars)
            .and_then(|r| r.strip_suffix(&format!(" {tail}")))
            && !name.is_empty()
        {
            return Some((name.to_string(), None));
        }
    }
    if let Some((n, rest)) = line.split_once(" columns:")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        let rest = rest.trim();
        return Some((
            format!("{n} columns"),
            (!rest.is_empty()).then(|| rest.to_string()),
        ));
    }
    if line.starts_with("width=") || line.starts_with("height=") {
        return Some((line.trim_end_matches(':').to_string(), None));
    }
    // `80x24`, `48×6:`: the size of the terminal a frame was drawn at, as the
    // test wrote it (the colon stays when the test wrote one).
    if let Some((w, h)) = line.trim_end_matches(':').split_once(['x', '×'])
        && !w.is_empty()
        && !h.is_empty()
        && w.chars().all(|c| c.is_ascii_digit())
        && h.chars().all(|c| c.is_ascii_digit())
    {
        return Some((line.to_string(), None));
    }
    // `40x12 initial`, `80x8 scrolled:`: the size a frame was drawn at and what the test did
    // before it, first in the snapshot or under a blank row, over the rows of that render.
    if let Some((size, what)) = line.trim_end_matches(':').split_once(' ')
        && let Some((w, h)) = size.split_once(['x', '×'])
        && !w.is_empty()
        && !h.is_empty()
        && w.chars().all(|c| c.is_ascii_digit())
        && h.chars().all(|c| c.is_ascii_digit())
        && !what.is_empty()
        && what.chars().all(|c| c.is_ascii_lowercase() || c == ' ')
        && prev.is_none_or(|p| p.trim().is_empty())
        && next.is_some()
    {
        return Some((line.to_string(), None));
    }
    // `external (80 columns)`, `empty (48 columns):`: who was drawn and at which width, first in
    // the snapshot or under a blank row, over the rows of that render.
    if let Some((who, tail)) = line.trim_end_matches(':').split_once(" (")
        && let Some(n) = tail.strip_suffix(" columns)")
        && !who.is_empty()
        && who.chars().all(|c| c.is_ascii_lowercase() || c == ' ')
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
        && prev.is_none_or(|p| p.trim().is_empty())
        && next.is_some()
    {
        return Some((line.to_string(), None));
    }
    // `(255, 255, 255): Buffer {`: a label and the dump's first line on one row.
    if let Some(name) = line.strip_suffix(": Buffer {")
        && !name.is_empty()
        && !line.starts_with(' ')
    {
        return Some((name.to_string(), Some("Buffer {".to_string())));
    }
    // `width 20:`: the label of a frame drawn at that width.
    if let Some(n) = line
        .strip_prefix("width ")
        .and_then(|r| r.strip_suffix(':'))
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
        && next.is_some()
    {
        return Some((line.to_string(), None));
    }
    // `before scroll:`: a bare label of lowercase words, first in the
    // snapshot or under a blank row, over the rows of a render.
    if let Some(name) = line.strip_suffix(':')
        && !name.is_empty()
        && name.chars().all(|c| c.is_ascii_lowercase() || c == ' ')
        && prev.is_none_or(|p| p.trim().is_empty())
        && next.is_some()
    {
        return Some((line.to_string(), None));
    }
    // `provider_error_local_80`: the test's own name for a render, a snake_case
    // word of two parts or more with no colon, first in the snapshot or under a
    // blank row, over the rows of that render.
    if line.starts_with(|c: char| c.is_ascii_lowercase())
        && line.contains('_')
        && line
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && prev.is_none_or(|p| p.trim().is_empty())
        && next.is_some()
    {
        return Some((line.to_string(), None));
    }
    if next.is_some_and(|n| n.trim_end() == "Buffer {")
        && !line.starts_with(' ')
        && line != "Buffer {"
    {
        return Some((line.to_string(), None));
    }
    None
}

/// The frames of the snapshot `rel`, in order. A snapshot the test made of
/// several screens marks each with a header line (`--- x ---`,
/// `---- x ----`, `N columns:`, `width=N..`, `height=N`, `80x24`, `40x12 initial`,
/// `external (80 columns)`, a bare label such as `before scroll:`, a label above a `Buffer {`
/// dump); one without is a single frame with an empty name.
pub fn frames(rel: &str) -> Vec<Frame> {
    frames_with(rel, &header)
}

/// State `index` (from 0) of a snapshot the test glued of states of one screen with a blank row
/// between and no header (`format!("{initial}\n\n{scrolled}")`): `height` rows each, the states
/// `height + 1` lines apart. Panics when the snapshot has no such state.
pub fn state(rel: &str, height: usize, index: usize) -> Frame {
    let body = body(rel);
    let start = index * (height + 1);
    assert!(
        start < body.len(),
        "{rel}: no state {index} of {height} rows; the snapshot has {} lines",
        body.len()
    );
    let end = (start + height).min(body.len());
    if let Some(sep) = body.get(end)
        && !sep.trim().is_empty()
    {
        panic!("{rel}: state {index} of {height} rows is not followed by a blank row: `{sep}`");
    }
    parse_frame(format!("state {index}"), &body[start..end])
}

/// State `index` (from 0) of a snapshot the test glued of states of different heights with a
/// blank row between and no header (`panel(..)` bodies joined by `"\n\n"`): `heights[i]` rows
/// for state `i`. A body's own blank rows are part of its height. Panics when the snapshot has
/// no such state, or a state is not followed by a blank row.
pub fn states(rel: &str, heights: &[usize], index: usize) -> Frame {
    let body = body(rel);
    let start: usize = heights[..index].iter().map(|height| height + 1).sum();
    let height = heights[index];
    assert!(
        start < body.len(),
        "{rel}: no state {index} of {heights:?} rows; the snapshot has {} lines",
        body.len()
    );
    let end = (start + height).min(body.len());
    if index + 1 < heights.len()
        && let Some(sep) = body.get(end)
        && !sep.trim().is_empty()
    {
        panic!("{rel}: state {index} of {heights:?} rows is not followed by a blank row: `{sep}`");
    }
    parse_frame(format!("state {index}"), &body[start..end])
}

/// The states of a snapshot the test joined of renders of one screen with `join("\n---\n")` and
/// no header (`states.join("\n---\n")`): the lines between the bare `---` rules, named
/// `state 0`, `state 1`... in the order the test wrote them. A bare `---` row of a screen's own
/// (a markdown rule) is not told from the test's: ask for it only of a snapshot made so.
pub fn cut(rel: &str) -> Vec<Frame> {
    body(rel)
        .split(|line| line == "---")
        .enumerate()
        .map(|(index, lines)| parse_frame(format!("state {index}"), lines))
        .collect()
}

/// State `index` (from 0) of a snapshot cut at its bare rules (`cut`). Panics when there is none.
pub fn cut_state(rel: &str, index: usize) -> Frame {
    let all = cut(rel);
    let count = all.len();
    all.into_iter().nth(index).unwrap_or_else(|| {
        panic!("{rel}: no state {index} of {count} between its bare `---` rules")
    })
}

/// What a header reader answers for a line: the frame's name and the row the header line also is.
type Header = Option<(String, Option<String>)>;

/// A header reader: the line, the line before it and the line after it.
type HeaderOf<'a> = dyn Fn(&str, Option<&String>, Option<&String>) -> Header + 'a;

/// The frames of a snapshot whose headers are the lines `labels` name: a snapshot the test glued
/// of several renders under labels of its own (`Compact` / `Full`, `Full height:` / `Clipped:`,
/// `Resume 118x30`, `empty (48 columns):`, `Before account change:`), which no rule of `frames`
/// can tell from a row of a screen. A label is matched as the line's text, a trailing colon of
/// either one aside; the frame is named by the label as given, without the colon. A `---` line
/// above a label is the test's `join("\n---\n")`. Only the labels are headers.
pub fn frames_labelled(rel: &str, labels: &[&str]) -> Vec<Frame> {
    let bare = |l: &str| l.trim_end().trim_end_matches(':').to_string();
    let labels: Vec<(String, String)> = labels.iter().map(|l| (bare(l), bare(l))).collect();
    frames_with(rel, &move |line, _, _| {
        let at = bare(line);
        labels
            .iter()
            .find(|(label, _)| *label == at)
            .map(|(_, name)| (name.clone(), None))
    })
}

/// The frame of a snapshot glued under `labels` (`frames_labelled`) called `name`.
pub fn frame_labelled(rel: &str, labels: &[&str], name: &str) -> Frame {
    pick(rel, frames_labelled(rel, labels), name)
}

fn frames_with(rel: &str, header_of: &HeaderOf<'_>) -> Vec<Frame> {
    let body = body(rel);
    let mut chunks: Vec<(String, Vec<String>)> = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, line) in body.iter().enumerate() {
        // `---` alone between two frames is the test's `join("\n---\n")`.
        if line == "---"
            && body
                .get(i + 1)
                .is_some_and(|next| header_of(next, Some(line), body.get(i + 2)).is_some())
        {
            continue;
        }
        if let Some((name, inline)) = header_of(
            line,
            i.checked_sub(1).and_then(|j| body.get(j)),
            body.get(i + 1),
        ) {
            let n = seen.entry(name.clone()).or_insert(0);
            *n += 1;
            let name = if *n > 1 { format!("{name}#{n}") } else { name };
            chunks.push((name, inline.into_iter().collect()));
        } else if let Some((_, lines)) = chunks.last_mut() {
            lines.push(line.clone());
        } else {
            chunks.push((String::new(), vec![line.clone()]));
        }
    }
    if chunks.is_empty() {
        chunks.push((String::new(), Vec::new()));
    }
    chunks
        .into_iter()
        .map(|(name, lines)| parse_frame(name, &lines))
        .collect()
}

/// The frame of the snapshot called `name` (`frames`); the only frame of a
/// snapshot of one when `name` is empty.
pub fn frame(rel: &str, name: &str) -> Frame {
    pick(rel, frames(rel), name)
}

/// A frame's name as a test may write it: the colon of a bare label (`before scroll:`) is the
/// label's punctuation, in front of a `#2` as well.
fn same_name(frame: &str, wanted: &str) -> bool {
    let bare = |n: &str| match n.split_once('#') {
        Some((head, tail)) => format!("{}#{tail}", head.trim_end_matches(':')),
        None => n.trim_end_matches(':').to_string(),
    };
    frame == wanted || bare(frame) == bare(wanted)
}

fn pick(rel: &str, all: Vec<Frame>, name: &str) -> Frame {
    if name.is_empty() {
        assert!(
            all.len() == 1,
            "{rel} has several frames: {:?}; name one",
            all.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );
        return all.into_iter().next().unwrap_or_default();
    }
    let names: Vec<String> = all.iter().map(|f| f.name.clone()).collect();
    all.into_iter()
        .find(|f| same_name(&f.name, name))
        .unwrap_or_else(|| panic!("{rel}: no frame `{name}`; its frames: {names:?}"))
}

fn parse_frame(name: String, lines: &[String]) -> Frame {
    let mut frame = Frame {
        name,
        ..Frame::default()
    };
    if let Some(at) = lines.iter().position(|l| l.trim_end() == "Buffer {") {
        parse_buffer(&mut frame, &lines[at + 1..]);
        return frame;
    }
    let mut lines: Vec<&String> = lines.iter().collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if let Some(last) = lines.last()
        && let Some(cursor) = parse_cursor(last)
    {
        frame.cursor = Some(cursor);
        lines.pop();
    }
    // `cursor=Some((x, y))` above the rows: a test that formats the layout's cursor into its frame
    if let Some(first) = lines.first()
        && let Some(cursor) = first
            .strip_prefix("cursor=")
            .and_then(|rest| parse_cursor(&format!("Cursor: {rest}")))
    {
        frame.cursor = Some(cursor);
        lines.remove(0);
    }
    let mut rows = Vec::new();
    let mut hidden = Vec::new();
    for line in lines {
        let (text, cols) = parse_row(line);
        rows.push(text);
        hidden.push(cols);
    }
    while rows.last().is_some_and(|r| r.trim_end().is_empty()) {
        rows.pop();
        hidden.pop();
    }
    frame.rows = rows.into_iter().map(|r| r.trim_end().to_string()).collect();
    frame.hidden = hidden;
    frame
}

/// `Cursor: Some((31, 4))` or `Cursor: None`.
fn parse_cursor(line: &str) -> Option<Option<(usize, usize)>> {
    let rest = line.trim().strip_prefix("Cursor: ")?;
    if rest == "None" {
        return Some(None);
    }
    let nums = rest.strip_prefix("Some((")?.strip_suffix("))")?;
    let (x, y) = nums.split_once(", ")?;
    Some(Some((x.parse().ok()?, y.parse().ok()?)))
}

/// The lines after `Buffer {`: `area`, `content` (quoted rows) and `styles`.
fn parse_buffer(frame: &mut Frame, lines: &[String]) {
    #[derive(PartialEq)]
    enum In {
        Top,
        Content,
        Styles,
        Done,
    }
    let mut state = In::Top;
    let mut rows = Vec::new();
    let mut hidden = Vec::new();
    let mut styles = Vec::new();
    for line in lines {
        let t = line.trim();
        match state {
            In::Top => {
                if let Some(area) = t.strip_prefix("area: Rect {") {
                    let num = |key: &str| {
                        area.split(key)
                            .nth(1)
                            .and_then(|r| r.trim_start_matches(' ').split([',', ' ']).next())
                            .and_then(|n| n.parse::<usize>().ok())
                    };
                    if let (Some(w), Some(h)) = (num("width:"), num("height:")) {
                        frame.area = Some((w, h));
                    }
                } else if t == "content: [" {
                    state = In::Content;
                } else if t == "styles: [" {
                    state = In::Styles;
                }
            }
            In::Content => {
                if t == "]," {
                    state = In::Top;
                } else {
                    let (text, cols) = parse_row(t);
                    rows.push(text);
                    hidden.push(cols);
                }
            }
            In::Styles => {
                if t == "]" {
                    state = In::Done;
                } else if let Some(run) = parse_run(t) {
                    styles.push(run);
                }
            }
            In::Done => {
                if let Some(cursor) = parse_cursor(t) {
                    frame.cursor = Some(cursor);
                }
            }
        }
    }
    while rows
        .last()
        .is_some_and(|r: &String| r.trim_end().is_empty())
    {
        rows.pop();
        hidden.pop();
    }
    frame.rows = rows.into_iter().map(|r| r.trim_end().to_string()).collect();
    frame.hidden = hidden;
    frame.styles = Some(styles);
}

fn parse_run(line: &str) -> Option<StyleRun> {
    // `x: 2, y: 0, fg: Rgb(1, 2, 3), bg: Reset, underline: Reset, modifier: BOLD | DIM,`
    let line = line.trim().trim_end_matches(',');
    let field = |key: &str, next: Option<&str>| -> Option<String> {
        let at = line.find(key)? + key.len();
        let rest = &line[at..];
        let end = match next {
            Some(n) => rest.find(n)?,
            None => rest.len(),
        };
        Some(rest[..end].trim().trim_end_matches(',').to_string())
    };
    Some(StyleRun {
        x: field("x: ", Some(", y: "))?.parse().ok()?,
        y: field("y: ", Some(", fg: "))?.parse().ok()?,
        fg: field("fg: ", Some(", bg: "))?,
        bg: field("bg: ", Some(", underline: "))?,
        underline: field("underline: ", Some(", modifier: "))?,
        modifier: field("modifier: ", None)?,
    })
}

/// The screen's rows, trailing blanks trimmed.
pub fn screen(t: &Tester) -> Vec<String> {
    trimmed(t.lines())
}

/// Compares whole screens: the rows equal, line by line.
pub fn assert_same(label: &str, actual: &[String], expected: &[String]) {
    pretty_assertions::assert_eq!(
        expected.join("\n"),
        actual.join("\n"),
        "{label}: ours (right) differs from the oracle (left)"
    );
}

/// Compares `expected` with the screen rows from `at` on (a snapshot that is a
/// fragment of the oracle's screen).
pub fn assert_fragment(label: &str, actual: &[String], at: usize, expected: &[String]) {
    let got: Vec<String> = actual
        .iter()
        .skip(at)
        .take(expected.len())
        .cloned()
        .collect();
    assert_same(label, &trimmed(got), &trimmed(expected.iter().cloned()));
}
