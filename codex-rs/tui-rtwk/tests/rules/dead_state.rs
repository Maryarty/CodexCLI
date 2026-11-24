//! No core state field is written for nobody (core-refactor plan tier A2, core-review-c D1, D2,
//! R7): a `pub` field of a screen's `Model`, a feature's `State` or a `ThreadPart` that nothing
//! outside tests reads is an outbox with no reader. It feeds no view and no host ask; only a
//! test looks at it, so the user sees nothing of what the domain "did". A verb that has to be
//! carried out is an ask (`AppFx`, `AppCx::effect`); a value nobody needs goes.
//!
//! The scan is by name and path, not by type: a read is `.field` (not a call, not a write:
//! `= ..`, `.push(..)`, `.insert(..)`, `.extend(..)`, `.entry(..)`, `.clear()`) in the module of the
//! state (`core/src/screens/<slot>/**` or `<slot>.rs`, `core/src/features/<slot>/**`) or in a file
//! that names the slot (`<slot>::`), and `.<slot>.field` anywhere else. Test code is `tests/`,
//! `core/tests/`, `core/src/fixtures/`, files named `tests.rs` and a `#[cfg(test)] mod` at column
//! 0. A name another struct also has makes the scan blind to a dead field (never wrong the other
//! way), which is why it ratchets: `adr013_ratchets/dead_state.txt` lists today's fields.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use crate::adr013::check;
use crate::layers::srcscan::sources_of;

type Files = [(String, String)];
type Owned = Vec<(String, String)>;

const RULE: &str = "dead_state_field_read_only_in_tests";

/// Calls on a field that change it: a write is no read.
const WRITERS: &[&str] = &[
    "push",
    "push_back",
    "push_front",
    "push_str",
    "insert",
    "extend",
    "entry",
    "clear",
];

fn is_test_file(name: &str) -> bool {
    name.starts_with("tests/")
        || name.starts_with("core/tests/")
        || name.starts_with("host/tests/")
        || name.starts_with("core/src/fixtures/")
        || name.ends_with("/tests.rs")
}

/// The code and the test code of the files: comment lines blanked, a trailing `#[cfg(test)] mod`
/// split off into the tests.
fn split(files: &Files) -> (Owned, Owned) {
    let mut code = Vec::new();
    let mut tests = Vec::new();
    for (name, text) in files {
        let blanked = |text: &str| {
            text.lines()
                .map(|line| {
                    if line.trim_start().starts_with("//") {
                        ""
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        if is_test_file(name) {
            tests.push((name.clone(), blanked(text)));
            continue;
        }
        match text.split_once("\n#[cfg(test)]\nmod ") {
            Some((before, after)) => {
                code.push((name.clone(), blanked(before)));
                tests.push((name.clone(), blanked(after)));
            }
            None => code.push((name.clone(), blanked(text))),
        }
    }
    (code, tests)
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// How many times `text` reads `.field` (`.slot.field` when `qualified`): not a call, not a write.
fn reads(text: &str, field: &str, slot: &str, qualified: bool) -> usize {
    let mut count = 0;
    let mut from = 0;
    while let Some(found) = text[from..].find(field) {
        let at = from + found;
        from = at + field.len();
        let before = text[..at].trim_end();
        let after = &text[at + field.len()..];
        if !before.ends_with('.')
            || before.ends_with("..")
            || after.starts_with(is_ident_char)
            || text[..at].chars().next_back().is_some_and(is_ident_char)
        {
            continue;
        }
        if qualified {
            let owner = before[..before.len() - 1].trim_end();
            let Some(head) = owner.strip_suffix(slot) else {
                continue;
            };
            if !head.trim_end().ends_with('.') {
                continue;
            }
        }
        let rest = after.trim_start();
        if rest.starts_with('(') || written(rest) {
            continue;
        }
        count += 1;
    }
    count
}

/// `rest` (what follows a field's name) assigns it or calls a writer on it.
fn written(rest: &str) -> bool {
    if let Some(after) = rest.strip_prefix('=') {
        return !after.starts_with(['=', '>']);
    }
    let Some(call) = rest.strip_prefix('.') else {
        return false;
    };
    let call = call.trim_start();
    WRITERS.iter().any(|writer| {
        call.strip_prefix(writer)
            .is_some_and(|after| after.trim_start().starts_with('('))
    })
}

/// The state structs of the domain with their `pub` fields: `(file, struct, slot, kind, field)`.
fn state_fields(files: &Files) -> Vec<(String, String, String, String, String)> {
    let mut out = Vec::new();
    for (file, text) in files {
        let Some(rest) = file.strip_prefix("core/src/") else {
            continue;
        };
        let mut parts = rest.split('/');
        let (Some(kind), Some(entry)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !matches!(kind, "screens" | "features") {
            continue;
        }
        let slot = entry.strip_suffix(".rs").unwrap_or(entry);
        let owned: &[&str] = if kind == "screens" {
            &["Model", "ThreadPart"]
        } else {
            &["State", "ThreadPart"]
        };
        for name in owned {
            let head = format!("pub struct {name} {{");
            let Some(start) = text.find(&head) else {
                continue;
            };
            let body = &text[start + head.len()..];
            let mut depth = 0i32;
            let mut at_top = String::new();
            for c in body.chars() {
                match c {
                    '{' | '(' | '[' | '<' => depth += 1,
                    '}' if depth == 0 => break,
                    '}' | ')' | ']' | '>' => depth -= 1,
                    _ => {}
                }
                if depth == 0 || c == '\n' {
                    at_top.push(c);
                }
            }
            for line in at_top.lines() {
                let line = line.trim();
                if let Some(field) = line
                    .strip_prefix("pub ")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(field, _)| field.trim())
                    .filter(|field| field.chars().all(is_ident_char) && !field.is_empty())
                {
                    out.push((
                        file.clone(),
                        (*name).to_owned(),
                        slot.to_owned(),
                        kind.to_owned(),
                        field.to_owned(),
                    ));
                }
            }
        }
    }
    out
}

/// The `file:Type.field` of every state field that tests read and the code does not.
fn dead_fields(defs: &Files, code: &Files, tests: &Files) -> Vec<String> {
    let reads_in = |files: &Files, kind: &str, slot: &str, field: &str, defining: &str| {
        files
            .iter()
            .map(|(name, text)| {
                let module = format!("core/src/{kind}/{slot}/");
                let near = name.starts_with(&module)
                    || name == defining
                    || text.contains(&format!("{slot}::"));
                reads(text, field, slot, !near)
            })
            .sum::<usize>()
    };
    state_fields(defs)
        .into_iter()
        .filter(|(file, _, slot, kind, field)| {
            reads_in(code, kind, slot, field, file) == 0
                && reads_in(tests, kind, slot, field, file) > 0
        })
        .map(|(file, name, _, _, field)| format!("{file}:{name}.{field}"))
        .collect()
}

#[test]
fn dead_state_field_read_only_in_tests() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host/src"));
    files.extend(sources_of("core/tests"));
    files.extend(sources_of("tests"));
    let (code, tests) = split(&files);
    let defs: Vec<_> = files
        .iter()
        .filter(|(name, _)| name.starts_with("core/src/"))
        .cloned()
        .collect();
    check(
        RULE,
        "a core state field is written for a reader: a view, a host ask or the app; one only a test reads is an outbox nobody drains, and what the domain meant to do goes through an ask",
        &dead_fields(&defs, &code, &tests),
    );
}

#[test]
fn the_dead_state_scan_bites() {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let defs = vec![file(
        "core/src/screens/pets.rs",
        "pub struct Model {\n    /// doc\n    pub catalog: Option<Vec<String>>,\n    pub chosen: Vec<String>,\n    pub used: Vec<String>,\n    pub written: u8,\n}\n",
    )];
    let code = vec![
        file(
            "core/src/screens/pets.rs",
            "fn apply(model: &mut Model) {\n    model.chosen.push(x);\n    model.written = 1;\n    let n = model.used.len();\n    model.catalog.take();\n}\n#[cfg(test)]\nmod tests { fn t(m: Model) { m.catalog.clone(); } }\n",
        ),
        file(
            "src/sheets/pets.rs",
            "fn view(s: &S) { s.pets.catalog.clone(); }\n",
        ),
    ];
    let tests = vec![
        file(
            "core/tests/pets.rs",
            "fn t(s: S) { s.screens().pets.chosen.clone(); s.pets.written; s.pets.used; }\n",
        ),
        file("core/src/screens/pets.rs", "m.catalog.clone();"),
    ];
    // `chosen` is pushed to by the code and read by a test; `written` is assigned; `used` is
    // read by the code (`.len()`); `catalog` is read by a view
    assert_eq!(
        dead_fields(&defs, &code, &tests),
        [
            "core/src/screens/pets.rs:Model.chosen",
            "core/src/screens/pets.rs:Model.written"
        ]
    );
    // a call of the same name is no field, a comparison is a read
    assert_eq!(reads("a.pets.chosen()", "chosen", "pets", true), 0);
    assert_eq!(reads("a.chosen == b", "chosen", "pets", false), 1);
    assert_eq!(reads("a.chosen = b", "chosen", "pets", false), 0);
    assert_eq!(reads("x.other.chosen", "chosen", "pets", true), 0);
    assert_eq!(reads("x.pets\n    .chosen", "chosen", "pets", true), 1);
}
