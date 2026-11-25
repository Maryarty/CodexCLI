//! Rules that passed by name or by one spelling and now scan by meaning (tier A3 of the core
//! review: core-review-a F9, F10; core-review-b H1, H2, H3, K3; core-review-c R4). Today's
//! violations are counted in `holes_ratchets.txt` (the format of `adr013_ratchets.txt`:
//! `ratchet <rule> <key> <count>`, `exempt <rule> <key> <why>`): a count that rises is a new
//! violation, one that falls is written down, so the numbers only go down. R3 (the ADR-013
//! draw and gesture scans) lives in `adr013.rs` with its lines in `adr013_ratchets/`; H4 (the
//! hand-written wire) is `wire.rs` with `wire_budget.txt`.
//!
//! Every scan has a test that plants a violation and sees it caught.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::collections::BTreeSet;

use crate::adr013::{at, code_tokens, matching, report_in, starts};
use crate::layers::srcscan::{crate_dir, sources_of};

const RATCHETS: &str = "tests/rules/holes_ratchets.txt";

fn ratchet_file() -> String {
    std::fs::read_to_string(crate_dir().join(RATCHETS)).expect("read holes_ratchets.txt")
}

/// Fails with what the ratchet says of the violations `found` (one key each).
fn check(rule: &str, why: &str, found: &[String]) {
    let report = report_in(rule, found, &ratchet_file(), RATCHETS);
    assert!(
        report.is_empty(),
        "{rule} ({why}; the ratchet is {RATCHETS}): {report:#?}"
    );
}

type Files = [(String, String)];

fn file(name: &str, text: &str) -> (String, String) {
    (name.to_owned(), text.to_owned())
}

// ---- A lexer with the string literals kept (the scanner of adr013.rs blanks them) ----

#[derive(Debug, PartialEq)]
enum Tok {
    Id(String),
    P(char),
    Str(String),
}

/// `text` as tokens with their line (from 1): comments out, char literals out, string literals
/// whole (escapes dropped, so `\n` makes no word).
fn lex(text: &str) -> Vec<(Tok, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    line += usize::from(chars[i] == '\n');
                    i += 1;
                }
            }
        } else if c == '"' {
            let start = line;
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                    // a `\` before a newline folds the next line's indent away
                    if chars.get(i) == Some(&'\n') {
                        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
                            line += usize::from(chars[i] == '\n');
                            i += 1;
                        }
                    } else {
                        text.push(' ');
                        i += 1;
                    }
                    continue;
                }
                line += usize::from(chars[i] == '\n');
                text.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push((Tok::Str(text), start));
        } else if c == 'r' && (i == 0 || !word(chars[i - 1])) && raw_hashes(&chars, i + 1).is_some()
        {
            let hashes = raw_hashes(&chars, i + 1).expect("checked");
            let start = line;
            let mut j = i + 1 + hashes + 1;
            let mut text = String::new();
            while j < chars.len() {
                if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                line += usize::from(chars[j] == '\n');
                text.push(chars[j]);
                j += 1;
            }
            out.push((Tok::Str(text), start));
            i = j;
        } else if c == '\'' {
            if next == Some('\\') {
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
                i = j + 1;
            } else if chars.get(i + 2) == Some(&'\'') && next != Some('\'') {
                i += 3;
            } else {
                out.push((Tok::P('\''), line));
                i += 1;
            }
        } else if word(c) {
            let start = i;
            while i < chars.len() && word(chars[i]) {
                i += 1;
            }
            out.push((Tok::Id(chars[start..i].iter().collect()), line));
        } else {
            out.push((Tok::P(c), line));
            i += 1;
        }
    }
    out
}

fn raw_hashes(chars: &[char], at: usize) -> Option<usize> {
    let hashes = chars[at..].iter().take_while(|c| **c == '#').count();
    (chars.get(at + hashes) == Some(&'"')).then_some(hashes)
}

/// The index of the token closing the bracket at `open`.
fn close_of(toks: &[(Tok, usize)], open: usize) -> usize {
    let mut depth = 0i32;
    for (i, (tok, _)) in toks.iter().enumerate().skip(open) {
        match tok {
            Tok::P('(' | '[' | '{') => depth += 1,
            Tok::P(')' | ']' | '}') => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    toks.len().saturating_sub(1)
}

fn is_id(toks: &[(Tok, usize)], i: usize, name: &str) -> bool {
    matches!(toks.get(i), Some((Tok::Id(id), _)) if id == name)
}

fn is_p(toks: &[(Tok, usize)], i: usize, c: char) -> bool {
    matches!(toks.get(i), Some((Tok::P(p), _)) if *p == c)
}

/// The code of a file without its `#[cfg(test)] mod` (at the end of the file, by the repo's habit).
fn without_tests(text: &str) -> &str {
    text.find("#[cfg(test)]\nmod ")
        .map_or(text, |at| &text[..at])
}

fn is_test_file(name: &str) -> bool {
    name.ends_with("/tests.rs") || name.contains("/tests/")
}

// ---- 1. b-H1 / F10: user-visible prose in core, by literal ----

/// The macros whose string arguments are shown or written out.
const TEXT_MACROS: &[&str] = &["format", "write", "writeln", "concat"];

/// Outside `{..}`, a word of >=3 letters: what the one-line `format!` rule of layers.rs reads.
fn has_words_outside_braces(literal: &str) -> bool {
    let mut outside = String::new();
    let mut depth = 0;
    for c in literal.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => outside.push(c),
            _ => {}
        }
    }
    outside
        .split(|c: char| !c.is_alphabetic())
        .any(|word| word.chars().count() >= 3)
}

/// At least three words (runs of >=2 letters) and a space: a sentence, not an id or a path.
fn is_sentence(literal: &str) -> bool {
    literal.contains(' ')
        && literal
            .split(|c: char| !c.is_alphabetic())
            .filter(|word| word.chars().count() >= 2)
            .count()
            >= 3
}

/// The lines of the string literals of `text` that are prose: an argument of `format!`/`write!`/
/// `writeln!`/`concat!` or of `.write_str(` with words in it (on one line or many), and a
/// sentence made owned by `.to_owned()`/`.into()`/`.to_string()`/`String::from(`.
fn prose_lines(text: &str) -> Vec<usize> {
    let toks = lex(without_tests(text));
    let mut lines = BTreeSet::new();
    for i in 0..toks.len() {
        let macro_call = matches!(&toks[i].0, Tok::Id(m) if TEXT_MACROS.contains(&m.as_str()))
            && is_p(&toks, i + 1, '!')
            && matches!(toks.get(i + 2), Some((Tok::P('(' | '[' | '{'), _)));
        let method = is_id(&toks, i, "write_str") && is_p(&toks, i + 1, '(');
        if macro_call || method {
            let open = if macro_call { i + 2 } else { i + 1 };
            let end = close_of(&toks, open);
            let mut depth = 0i32;
            for (tok, line) in &toks[open..end] {
                match tok {
                    Tok::P('(' | '[' | '{') => depth += 1,
                    Tok::P(')' | ']' | '}') => depth -= 1,
                    Tok::Str(literal) if depth == 1 && has_words_outside_braces(literal) => {
                        lines.insert(*line);
                    }
                    _ => {}
                }
            }
        }
        if let (Tok::Str(literal), line) = &toks[i] {
            let owned = is_p(&toks, i + 1, '.')
                && matches!(toks.get(i + 2), Some((Tok::Id(m), _)) if matches!(m.as_str(), "to_owned" | "into" | "to_string"))
                && is_p(&toks, i + 3, '(');
            let from = i >= 5
                && is_id(&toks, i - 5, "String")
                && is_id(&toks, i - 2, "from")
                && is_p(&toks, i - 1, '(');
            if (owned || from) && is_sentence(literal) {
                lines.insert(*line);
            }
        }
    }
    lines.into_iter().collect()
}

/// `file` of every prose literal of core (one key per site).
fn prose_sites(core: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in core {
        if is_test_file(name) {
            continue;
        }
        for _ in prose_lines(text) {
            found.push(name.clone());
        }
    }
    found
}

#[test]
fn core_prose_literals_are_ratcheted() {
    check(
        "holes_core_prose",
        "failures and messages are typed values in core; the words are the UI's string tables' (ADR-010)",
        &prose_sites(&sources_of("core/src")),
    );
}

#[test]
fn the_prose_scan_bites() {
    let lines = |text: &str| prose_lines(text);
    // a multi-line format!, write!/writeln!, write_str and concat! are seen
    assert_eq!(
        lines(
            "fn f() {\n    let s = format!(\n        \"Could not open {}\",\n        x\n    );\n}\n"
        ),
        [3]
    );
    assert_eq!(lines("fn f() { write!(f, \"bad thing\")?; }"), [1]);
    assert_eq!(lines("fn f() { writeln!(\n f,\n \"bad thing\")?; }"), [3]);
    assert_eq!(lines("fn f() { f.write_str(\"bad thing\") }"), [1]);
    assert_eq!(lines("const A: &str = concat!(\"bad\", \" thing\");"), [1]);
    // a sentence made owned (F10)
    assert_eq!(
        lines("fn f() -> String { \"Couldn't continue this chat.\".to_owned() }"),
        [1]
    );
    assert_eq!(
        lines("fn f() { x.note(\"Couldn't continue this chat.\".into()) }"),
        [1]
    );
    assert_eq!(
        lines("fn f() { String::from(\"Couldn't continue this chat.\") }"),
        [1]
    );
    // not prose: a placeholder-only format, an id, a one-word owned literal, a comment, a
    // string in a test module, an argument of another call
    assert!(lines("fn f() { format!(\"{}:{}\", a, b); }").is_empty());
    assert!(lines("fn f() { let id = \"thread-name\".to_owned(); }").is_empty());
    assert!(lines("fn f() { // format!(\"Could not open\")\n }").is_empty());
    assert!(
        lines("fn f() {}\n#[cfg(test)]\nmod tests {\n fn t() { format!(\"Could not\"); }\n}\n")
            .is_empty()
    );
    assert!(lines("fn f() { format!(\"{}\", g(\"Could not open\")); }").is_empty());
    // `\n` is no word, a raw string is read
    assert!(lines("fn f() { format!(\"{}\\n{}\", a, b); }").is_empty());
    assert_eq!(lines("fn f() { format!(r#\"Could \"not\" open\"#); }"), [1]);
}

// ---- 2. b-H2: a slice reads only its own ThreadPart ----

/// The slices (the directories of `core/src/features/`) whose `ThreadPart` `ThreadView::parts()`
/// gives to any other slice, and `thread_parts`-style field reads that name another slice: ADR-012
/// "a slice does not read another slice's state". `file:slice` per read.
fn foreign_parts(files: &Files, slices: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        let mut seen = BTreeSet::new();
        let Some(rest) = name.strip_prefix("core/src/features/") else {
            continue;
        };
        let Some((own, _)) = rest.split_once('/') else {
            continue;
        };
        let tokens = code_tokens(text);
        for i in 0..tokens.len() {
            // `parts().x.<g>` through the handle, in the statement it opens
            let via_handle = starts(&tokens, i, &["parts", "(", ")"]);
            // `parts.<g>` / `states.<g>` (a closure's parameter, the hub's field)
            let direct = matches!(at(&tokens, i), "parts" | "states") && at(&tokens, i + 1) == ".";
            if !via_handle && !direct {
                continue;
            }
            let mut depth = 0i32;
            for k in i + 1..tokens.len().min(i + 60) {
                match tokens[k].as_str() {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth -= 1,
                    ";" if depth <= 0 => break,
                    "." => {
                        let g = at(&tokens, k + 1);
                        if g != own && slices.iter().any(|slice| slice == g) && seen.insert(k) {
                            found.push(format!("{name}:{g}"));
                        }
                    }
                    _ => {}
                }
                if depth < 0 {
                    break;
                }
            }
        }
    }
    found
}

fn feature_slices() -> Vec<String> {
    let mut slices: Vec<String> = std::fs::read_dir(crate_dir().join("core/src/features"))
        .expect("features dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    slices.sort();
    slices
}

#[test]
fn a_slice_reads_only_its_own_thread_part() {
    let slices = feature_slices();
    assert!(slices.contains(&"goal".to_owned()) && slices.contains(&"small_cmds".to_owned()));
    let found = foreign_parts(&sources_of("core/src"), &slices);
    assert!(
        found.is_empty(),
        "a slice reads another slice's ThreadPart through `ThreadView::parts()` (ADR-012): {found:#?}"
    );
}

#[test]
fn the_foreign_parts_scan_bites() {
    let slices: Vec<String> = ["goal", "small_cmds"].map(str::to_owned).into();
    let own = file(
        "core/src/features/small_cmds/mod.rs",
        "pub fn d(t: ThreadView<'_>) { t.parts().and_then(|parts| parts.small_cmds.saved()); }",
    );
    assert!(foreign_parts(&[own], &slices).is_empty());
    for body in [
        "pub fn d(t: ThreadView<'_>) { t.parts().and_then(|parts| parts.goal.saved()); }",
        "pub fn d(t: ThreadView<'_>) { let p = t.parts().map(|p| p.goal.clone()); }",
        "pub fn d(states: &mut FeatureStates) { let _ = states.goal.x; }",
    ] {
        let found = foreign_parts(
            &[file("core/src/features/small_cmds/mod.rs", body)],
            &slices,
        );
        assert_eq!(
            found,
            ["core/src/features/small_cmds/mod.rs:goal"],
            "{body}"
        );
    }
    // the hub and the framework name every slice: only a slice's directory is scanned
    let hub = file(
        "core/src/features/mod.rs",
        "fn f(states: &mut S) { states.goal.x(); }",
    );
    assert!(foreign_parts(&[hub], &slices).is_empty());
}

// ---- 3. b-H3: thread RPCs by an allow list ----

const CORE_THREAD: &[&str] = &["core/src/threads.rs", "core/src/protocol/mod.rs"];
/// Where a slice builds its own, besides the thread's own files (`Feature::wire`, `rpc_list!`);
/// the probe of `rpc_list.rs` and the framework's test double are not slices.
const SLICE: &[&str] = &[
    "core/src/threads.rs",
    "core/src/protocol/mod.rs",
    "core/src/features/",
    "core/src/protocol/rpc_list.rs",
];
const CATALOG: &[&str] = &["core/src/protocol/", "core/src/screens/"];
const READ_ONLY: &[&str] = &["core/src/", "host/src/"];

/// Every thread RPC the code builds (`ClientRequest::X`, its `XParams`, an `rpc_list!` line's
/// `=> X(`) by name with the files that may build it. A name that is not here is unclassified:
/// a new RPC is a decision made in this table (lifecycle: the thread's own files, a feature's:
/// its slice, a catalogue or read-only call: where listed).
const THREAD_RPCS: &[(&str, &[&str])] = &[
    // the lifecycle of the thread (ADR-003): `threads.rs` and the app's two commands
    ("ThreadResume", CORE_THREAD),
    ("ThreadFork", CORE_THREAD),
    ("ThreadStart", CORE_THREAD),
    ("ThreadRead", CORE_THREAD),
    ("ThreadTurnsList", CORE_THREAD),
    ("ThreadUnsubscribe", CORE_THREAD),
    ("TurnStart", CORE_THREAD),
    ("TurnInterrupt", CORE_THREAD),
    ("TurnSteer", CORE_THREAD),
    ("ReviewStart", CORE_THREAD),
    ("ThreadBackgroundTerminalsClean", CORE_THREAD),
    ("ThreadPrepareApprovedPlan", CORE_THREAD),
    (
        "ThreadSettingsUpdate",
        &[
            "core/src/threads.rs",
            "core/src/protocol/mod.rs",
            "core/src/threads/goal.rs",
        ],
    ),
    // a feature's, in its slice
    ("ThreadCompactStart", SLICE),
    ("ThreadSetName", SLICE),
    ("ThreadShellCommand", SLICE),
    ("ThreadRevert", SLICE),
    ("ThreadGoalGet", SLICE),
    ("ThreadGoalSet", SLICE),
    ("ThreadGoalClear", SLICE),
    ("ThreadMetadataUpdate", SLICE),
    // the approvals screen answers a guardian denial
    (
        "ThreadApproveGuardianDeniedAction",
        &["core/src/screens/approvals.rs"],
    ),
    // the catalogue of threads (resume, agents): archive, delete, move between sections
    ("ThreadArchive", CATALOG),
    ("ThreadUnarchive", CATALOG),
    ("ThreadDelete", CATALOG),
    ("ThreadSectionMove", CATALOG),
    // read-only lists
    ("ThreadList", READ_ONLY),
    ("ThreadLoadedList", READ_ONLY),
    ("ThreadSearch", READ_ONLY),
];

fn is_thread_rpc_name(name: &str) -> bool {
    ["Thread", "Turn", "Review"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(char::is_uppercase))
    })
}

/// The thread RPC names a file's tokens build: `ClientRequest::X`, `XParams` and `=> X(` in an
/// `rpc_list!`.
fn thread_rpc_names(text: &str) -> Vec<String> {
    let tokens = code_tokens(text);
    let mut names = Vec::new();
    for i in 0..tokens.len() {
        let token = tokens[i].as_str();
        if starts(&tokens, i, &["ClientRequest", ":", ":"])
            && is_thread_rpc_name(at(&tokens, i + 3))
        {
            names.push(at(&tokens, i + 3).to_owned());
        } else if let Some(name) = token.strip_suffix("Params")
            && is_thread_rpc_name(name)
        {
            names.push(name.to_owned());
        } else if token == "rpc_list" && starts(&tokens, i + 1, &["!", "{"]) {
            let end = matching(&tokens, i + 2);
            for k in i + 3..end {
                if tokens[k] == "="
                    && at(&tokens, k + 1) == ">"
                    && is_thread_rpc_name(at(&tokens, k + 2))
                    && at(&tokens, k + 3) == "("
                {
                    names.push(at(&tokens, k + 2).to_owned());
                }
            }
        }
    }
    names
}

/// `file:Name` of every thread RPC built outside its home, or not classified at all.
fn thread_rpc_sites(files: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if is_test_file(name) {
            continue;
        }
        // once per file and name: a request and its params are one site
        let names: BTreeSet<String> = thread_rpc_names(without_tests(text)).into_iter().collect();
        for rpc in names {
            let home = THREAD_RPCS
                .iter()
                .find(|(known, _)| *known == rpc)
                .is_some_and(|(_, homes)| homes.iter().any(|home| name.starts_with(home)));
            if !home {
                found.push(format!("{name}:{rpc}"));
            }
        }
    }
    found
}

#[test]
fn thread_rpcs_are_classified_and_built_at_home() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host/src"));
    check(
        "holes_thread_rpcs",
        "a thread RPC is built where THREAD_RPCS says (ADR-003: the lifecycle in ThreadFx, a feature's in its slice); a new one is a line of that table",
        &thread_rpc_sites(&files),
    );
}

#[test]
fn the_thread_rpc_allow_list_bites() {
    let one = |name: &str, text: &str| thread_rpc_sites(&[file(name, text)]);
    // at home: quiet
    assert!(one("core/src/threads.rs", "ClientRequest::TurnStart {").is_empty());
    assert!(
        one(
            "core/src/features/goal/rpcs.rs",
            "ClientRequest::ThreadSetName {"
        )
        .is_empty()
    );
    // outside its home
    assert_eq!(
        one(
            "core/src/features/side/mod.rs",
            "ClientRequest::TurnStart {"
        ),
        ["core/src/features/side/mod.rs:TurnStart"]
    );
    assert_eq!(
        one("src/pages/x.rs", "let p = ThreadResumeParams {"),
        ["src/pages/x.rs:ThreadResume"]
    );
    // an unclassified RPC is refused wherever it is
    assert_eq!(
        one(
            "core/src/features/side/mod.rs",
            "ClientRequest::ThreadInjectItems {"
        ),
        ["core/src/features/side/mod.rs:ThreadInjectItems"]
    );
    // an `rpc_list!` line without the prefix is seen: a lifecycle RPC in a slice, an unknown one
    assert_eq!(
        one(
            "core/src/features/x/rpcs.rs",
            "crate::rpc_list! {\n    pub enum C => R;\n    A, Self::A => ThreadResume(p) -> R2, |_| R::Ack;\n    B, Self::B => ThreadInjectItems(p) -> R2, |_| R::Ack;\n    C, Self::C => ThreadSetName(p) -> R2, |_| R::Ack;\n}\n"
        ),
        [
            "core/src/features/x/rpcs.rs:ThreadInjectItems",
            "core/src/features/x/rpcs.rs:ThreadResume"
        ]
    );
    // a match arm that merely returns a function of that name is not an `rpc_list!` line
    assert!(one("core/src/screens/x.rs", "fn f() { A => ThreadResume(1) }").is_empty());
    // a test module is not scanned
    assert!(
        one(
            "core/src/screens/x.rs",
            "#[cfg(test)]\nmod tests { ClientRequest::TurnStart }"
        )
        .is_empty()
    );
}

// ---- 5. b-K3: ActionId::new only in the tables ----

const ACTION_TABLES: &[&str] = &[
    "core/src/keymap/catalogue.rs",
    "core/src/keymap/actions.rs",
    "core/src/keymap/fixed.rs",
];

/// `file` per `ActionId::new(` outside the tables: a typo in a name written inline compiles, and a
/// reserved-key or overlap rule then matches nothing.
fn inline_action_ids(files: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if ACTION_TABLES.contains(&name.as_str()) {
            continue;
        }
        let tokens = code_tokens(text);
        for i in 0..tokens.len() {
            if starts(&tokens, i, &["ActionId", ":", ":", "new", "("]) {
                found.push(name.clone());
            }
        }
    }
    found
}

#[test]
fn action_ids_are_made_in_the_tables_only() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host/src"));
    check(
        "holes_action_id_new",
        "use the `actions::` constants: a name typed inline is a typo that compiles",
        &inline_action_ids(&files),
    );
}

#[test]
fn the_action_id_scan_bites() {
    let files = [
        file("core/src/keymap/actions.rs", "ActionId::new(C, \"a\")"),
        file(
            "core/src/keymap/resolve.rs",
            "let x = ActionId::new(C, \"a\");\n// ActionId::new(C, \"b\")\nlet y = ActionId::new(\n C, \"c\");",
        ),
        file("src/keymap/mod.rs", "ActionId :: new (C, \"a\")"),
    ];
    assert_eq!(
        inline_action_ids(&files),
        [
            "core/src/keymap/resolve.rs",
            "core/src/keymap/resolve.rs",
            "src/keymap/mod.rs"
        ]
    );
}

// ---- 7. c-R4: ids are newtypes, not strings (the widening of r13) ----

/// Fields (and parameters) named for a thread, a turn or a parent that are text.
const ID_NAMES: &[&str] = &["thread", "turn", "parent"];

/// `file:name` of every `thread: String` (also `Option<String>`) in core's sources, and of every
/// `name: BTreeSet<String>` / `HashSet<String>` / `BTreeMap<String, _>` / `HashMap<String, _>`
/// field of a screen (the sets and maps of thread ids the agents screen keeps): an id of the
/// wrong kind compiles.
fn string_ids(core: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in core {
        if name.starts_with("core/src/fixtures/") || is_test_file(name) {
            continue;
        }
        let in_screen = name.starts_with("core/src/screens/");
        let tokens = code_tokens(without_tests(text));
        for i in 1..tokens.len() {
            let field = tokens[i].as_str();
            if !field.starts_with(|c: char| c.is_alphabetic() || c == '_') {
                continue;
            }
            if at(&tokens, i + 1) != ":" || at(&tokens, i + 2) == ":" || tokens[i - 1] == ":" {
                continue;
            }
            if matches!(tokens[i - 1].as_str(), "let" | "mut") {
                continue;
            }
            // skip a wrapper in front of the type: Option<, Rc<, std::collections::
            let mut k = i + 2;
            while matches!(
                at(&tokens, k),
                "Option" | "Rc" | "Arc" | "Box" | "std" | "collections" | "<"
            ) || (at(&tokens, k) == ":" && k > i + 2)
            {
                k += 1;
            }
            let ty = at(&tokens, k);
            let is_string =
                ty == "String" && matches!(at(&tokens, k + 1), "," | ">" | "}" | ")" | "=");
            let is_set = matches!(ty, "BTreeSet" | "HashSet")
                && at(&tokens, k + 1) == "<"
                && at(&tokens, k + 2) == "String";
            let is_map = matches!(ty, "BTreeMap" | "HashMap")
                && at(&tokens, k + 1) == "<"
                && at(&tokens, k + 2) == "String"
                && at(&tokens, k + 3) == ",";
            if (is_string && ID_NAMES.contains(&field)) || (in_screen && (is_set || is_map)) {
                found.push(format!("{name}:{field}"));
            }
        }
    }
    found
}

#[test]
fn ids_are_not_strings() {
    check(
        "holes_string_ids",
        "R13: a thread, a turn or a parent is a `ThreadId` / `TurnId`, never text; sets and maps of thread ids are keyed by the newtype",
        &string_ids(&sources_of("core/src")),
    );
}

#[test]
fn the_string_id_scan_bites() {
    let files = [file(
        "core/src/screens/agents/mod.rs",
        "pub struct Model {\n    pub unread: BTreeSet<String>,\n    pub overlay: BTreeMap<String, Overlay>,\n    pub pinned: std::collections::HashSet<String>,\n    pub ok: BTreeSet<ThreadId>,\n    pub by_ref: &'a BTreeSet<String>,\n}\nenum Intent { Open { thread: String }, Mark { thread: ThreadId, turn: Option<String> } }\nfn f(parent: String, other: String) { let thread: String = x; }\n",
    )];
    assert_eq!(
        string_ids(&files),
        [
            "core/src/screens/agents/mod.rs:unread",
            "core/src/screens/agents/mod.rs:overlay",
            "core/src/screens/agents/mod.rs:pinned",
            "core/src/screens/agents/mod.rs:thread",
            "core/src/screens/agents/mod.rs:turn",
            "core/src/screens/agents/mod.rs:parent",
        ]
    );
    // a set of strings outside the screens is not read as ids
    let elsewhere = [file(
        "core/src/kit/x.rs",
        "struct S { seen: BTreeSet<String>, thread: String }",
    )];
    assert_eq!(string_ids(&elsewhere), ["core/src/kit/x.rs:thread"]);
}

// ---- 8. F9: fixture-only events ----

/// The events live `apply` has only because a fixture script needs them (core-review-a F9):
/// `(name, the module's directory or file, the enum's own files)`. Nothing the app receives from
/// the server makes them; they become `AppEv::Seed` (the debug menu's one door). Each is named
/// here, so the sites that name one are counted and a new one fails.
const FIXTURE_ONLY: &[(&str, &str, &[&str])] = &[
    (
        "agents::Ev::Current",
        "agents",
        &["core/src/screens/agents/", "core/src/protocol/agents.rs"],
    ),
    ("chat::Ev::Pending", "chat", &["core/src/screens/chat.rs"]),
    (
        "approvals::Ev::Presented",
        "approvals",
        &["core/src/screens/approvals.rs"],
    ),
    (
        "status::Ev::Card",
        "status",
        &["core/src/screens/status.rs"],
    ),
];

/// `file:event` per token run naming a fixture-only event: qualified (`agents::Ev::Current`) or,
/// in the files of the enum itself, `Ev::Current`. The apply arm and the script decoder are the
/// sites today; a live producer would be one more.
fn fixture_event_sites(core: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in core {
        if name.starts_with("core/src/fixtures/") || is_test_file(name) {
            continue;
        }
        let tokens = code_tokens(without_tests(text));
        for (event, module, own) in FIXTURE_ONLY {
            let variant = event.rsplit("::").next().unwrap_or_default();
            let local = own.iter().any(|prefix| name.starts_with(prefix));
            for i in 0..tokens.len() {
                let qualified = i >= 3
                    && at(&tokens, i - 3) == *module
                    && at(&tokens, i - 2) == ":"
                    && at(&tokens, i - 1) == ":";
                if starts(&tokens, i, &["Ev", ":", ":", variant]) && (local || qualified) {
                    found.push(format!("{name}:{event}"));
                }
            }
        }
    }
    found
}

#[test]
fn fixture_only_events_are_named() {
    check(
        "holes_fixture_only_events",
        "a script cannot build a state the app cannot reach: an event only a fixture sends is `AppEv::Seed` (F9); these four are named until they move",
        &fixture_event_sites(&sources_of("core/src")),
    );
}

#[test]
fn the_fixture_event_scan_bites() {
    let files = [
        file(
            "core/src/app.rs",
            "ScreenEv::Agents(agents::Ev::Current(id)) => {}",
        ),
        file(
            "core/src/screens/chat.rs",
            "Ev::Pending { thread, pending } => {}\nOther::Pending(1);",
        ),
        file("core/src/screens/other.rs", "Ev::Pending { .. }"),
        file("core/src/fixtures/script.rs", "agents::Ev::Current(x)"),
    ];
    assert_eq!(
        fixture_event_sites(&files),
        [
            "core/src/app.rs:agents::Ev::Current",
            "core/src/screens/chat.rs:chat::Ev::Pending"
        ]
    );
}
