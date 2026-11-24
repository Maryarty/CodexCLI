//! ADR-013, the core/UI boundary by the IO test (docs/rtwk-port/architecture.md "ADR-013", full text
//! docs/rtwk-port/research/arch-boundary/full/architecture.md §13): what lives in core is what IO
//! needs, everything drawn is UI state, and the mechanisms that carry the rest (`Interest`,
//! `Directive`, `SurfaceWish`, `SheetStack`) are made and read in one place each.
//!
//! Every rule is a scan with a synthetic bite (a planted violation the scan catches, a clean text it
//! spares). A rule over a mechanism that does not exist yet (`Interest`, `Directive`, `WishShown`,
//! `Sheet`) holds trivially today and bites the day the mechanism lands. A rule the code violates
//! today (the families of full §14 "Отступление") is a RATCHET: `adr013_ratchets.txt` lists today's
//! violations by key (a file, or `file:Type.field`) with a count, and a rule fails when a count
//! rises (a new violation: new code cannot add to the list) and when it falls (the saving is
//! written down so it cannot come back), as `hub_budget.txt` does for the hub.
//!
//! What a scan cannot decide, "is this value a request key or a cursor" (the IO test's questions
//! 2 against 5), is the review checklist of docs/rtwk-port/process.md §A5.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::layers::srcscan::{crate_dir, sources_of};

type Files = [(String, String)];

// ---- The ratchet file ----

const RATCHETS: &str =
    "tests/rules/adr013_ratchets.txt and tests/rules/adr013_ratchets/<family>.txt";

/// The ratchet lines: `adr013_ratchets.txt` (the format, the comments) and one file per
/// migration family in `adr013_ratchets/`, read as one text (families migrating in parallel
/// edit disjoint files).
fn ratchet_file() -> String {
    let dir = crate_dir().join("tests/rules");
    let mut text =
        std::fs::read_to_string(dir.join("adr013_ratchets.txt")).expect("read the ratchet file");
    // no family file left is no directory (git keeps none empty): every ratchet has run out
    let mut families: Vec<_> = std::fs::read_dir(dir.join("adr013_ratchets"))
        .into_iter()
        .flatten()
        .map(|entry| entry.expect("a ratchet file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    families.sort();
    for path in families {
        text.push('\n');
        text.push_str(&std::fs::read_to_string(&path).expect("read a family's ratchet file"));
    }
    text
}

/// What the violations `found` (one key each) say against the rule's lines of `file`:
/// `ratchet <rule> <key> <count>` is today's count, `exempt <rule> <key>` a named exception.
fn report(rule: &str, found: &[String], file: &str) -> Vec<String> {
    let mut budget: BTreeMap<String, usize> = BTreeMap::new();
    let mut exempt: BTreeSet<String> = BTreeSet::new();
    for line in file.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["ratchet", name, key, count] if *name == rule => {
                budget.insert((*key).to_owned(), count.parse().expect("a count"));
            }
            ["exempt", name, key, ..] if *name == rule => {
                exempt.insert((*key).to_owned());
            }
            _ => {}
        }
    }
    let mut now: BTreeMap<&str, usize> = BTreeMap::new();
    for key in found.iter().filter(|key| !exempt.contains(*key)) {
        *now.entry(key.as_str()).or_default() += 1;
    }
    let keys: BTreeSet<&str> = now
        .keys()
        .copied()
        .chain(budget.keys().map(String::as_str))
        .collect();
    let mut out = Vec::new();
    for key in keys {
        let current = now.get(key).copied().unwrap_or(0);
        let allowed = budget.get(key).copied().unwrap_or(0);
        match current.cmp(&allowed) {
            std::cmp::Ordering::Greater => out.push(format!(
                "{key}: {current} found, {allowed} allowed: a new violation (the list only shrinks)"
            )),
            std::cmp::Ordering::Less => out.push(format!(
                "{key}: {current} found, {allowed} allowed: lower its line in {RATCHETS} to {current} (delete it at 0)"
            )),
            std::cmp::Ordering::Equal => {}
        }
    }
    out
}

/// Fails with what `report` says and why the rule is there.
fn check(rule: &str, why: &str, found: &[String]) {
    let report = report(rule, found, &ratchet_file());
    assert!(
        report.is_empty(),
        "{rule} (ADR-013: {why}; the ratchet is {RATCHETS}): {report:#?}"
    );
}

#[test]
fn the_ratchet_report_bites() {
    let file = "# a comment\nratchet r a.rs 2\nratchet r gone.rs 1\nratchet other b.rs 5\nexempt r ok.rs why\n";
    let key = |text: &str| text.to_owned();
    // equal: quiet; the exempt file is not counted; the other rule's lines are not read
    assert!(report("r", &[key("a.rs"), key("a.rs"), key("ok.rs")], file).len() == 1);
    // one line is `gone.rs`, which no longer has its violation
    let quiet = report("r", &[key("a.rs"), key("a.rs"), key("gone.rs")], file);
    assert!(quiet.is_empty(), "{quiet:?}");
    // a new file, and a count that rose
    let rose = report(
        "r",
        &[
            key("a.rs"),
            key("a.rs"),
            key("a.rs"),
            key("gone.rs"),
            key("new.rs"),
        ],
        file,
    );
    assert_eq!(rose.len(), 2, "{rose:?}");
    assert!(rose.iter().all(|line| line.contains("new violation")));
    // a count that fell must be written down
    let fell = report("r", &[key("gone.rs"), key("a.rs")], file);
    assert_eq!(fell.len(), 1, "{fell:?}");
    assert!(fell[0].contains("lower its line"), "{fell:?}");
}

/// The file has only lines it understands, of rules this file checks, and keys that name files
/// the tree has: a typo in the list would otherwise hide a violation or exempt nothing.
#[test]
fn the_ratchet_file_is_well_formed() {
    const RULES: &[&str] = &[
        "adr013_domain_types_hold_no_draw_state",
        "adr013_kit_is_not_app_state",
        "adr013_feature_trait_has_no_view_types",
        "adr013_core_intents_name_no_gesture",
        "adr013_widgets_name_no_screen_half",
        "adr013_no_ticket_in_ui",
        "adr013_interest_made_in_one_place",
        "adr013_directives_read_in_one_place",
        "adr013_wishes_shown_by_the_presenter",
        "adr013_sheets_declare_alive",
        "adr013_no_whole_state_watch",
        "r2_thread_subtree_is_keyed",
        "adr002_screen_and_sheet_widgets_are_eq",
        "adr006_keycode_only_in_keymap",
        "adr013_kit_names_no_screen",
        "adr013_data_names_only_ids",
        "adr013_store_written_only_by_apply",
    ];
    let mut bad = Vec::new();
    for line in ratchet_file().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let (rule, key) = match words.as_slice() {
            ["ratchet", rule, key, count] if count.parse::<usize>().is_ok_and(|n| n > 0) => {
                (*rule, *key)
            }
            ["exempt", rule, key, _why, ..] => (*rule, *key),
            _ => {
                bad.push(format!("not a line of the list: {line}"));
                continue;
            }
        };
        if !RULES.contains(&rule) {
            bad.push(format!("unknown rule `{rule}`: {line}"));
        }
        let path = key.split(':').next().unwrap_or(key);
        if path.contains('/') && !crate_dir().join(path).is_file() {
            bad.push(format!("no such file `{path}`: {line}"));
        }
    }
    assert!(bad.is_empty(), "{RATCHETS}: {bad:#?}");
}

// ---- A small scanner of Rust items: comments and literals out, tokens, struct and enum fields ----

/// `text` as code only: comments and the insides of string and char literals are blanked (newlines
/// kept), so a scan sees what the compiler sees.
fn cleaned(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
        } else if c == '"' {
            out.push('"');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < chars.len() {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            out.push('"');
            i += 1;
        } else if c == 'r'
            && (i == 0
                || !is_ident(chars[i - 1])
                || (chars[i - 1] == 'b' && (i < 2 || !is_ident(chars[i - 2]))))
            && raw_string_hashes(&chars, i + 1).is_some()
        {
            let hashes = raw_string_hashes(&chars, i + 1).expect("checked");
            let mut j = i + 1 + hashes + 1;
            out.push('"');
            while j < chars.len() {
                if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                out.push(blank(chars[j]));
                j += 1;
            }
            out.push('"');
            i = j;
        } else if c == '\'' {
            if next == Some('\\') {
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
                out.push(' ');
                i = j + 1;
            } else if chars.get(i + 2) == Some(&'\'') && next != Some('\'') {
                out.push(' ');
                i += 3;
            } else {
                out.push('\'');
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `#`s of a raw string's opening if `chars[at..]` is `#*"`.
fn raw_string_hashes(chars: &[char], at: usize) -> Option<usize> {
    let hashes = chars[at..].iter().take_while(|c| **c == '#').count();
    (chars.get(at + hashes) == Some(&'"')).then_some(hashes)
}

/// The code of `text` as tokens: identifiers (and numbers) whole, everything else one character.
fn code_tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = cleaned(text).chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
        } else if chars[i].is_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else {
            out.push(chars[i].to_string());
            i += 1;
        }
    }
    out
}

fn is_ident(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
}

fn at(tokens: &[String], i: usize) -> &str {
    tokens.get(i).map_or("", String::as_str)
}

/// The tokens from `i` start with the words `pattern`.
fn starts(tokens: &[String], i: usize, pattern: &[&str]) -> bool {
    pattern
        .iter()
        .enumerate()
        .all(|(k, word)| at(tokens, i + k) == *word)
}

/// The index of the token that closes the `(`, `[` or `{` at `open`.
fn matching(tokens: &[String], open: usize) -> usize {
    let mut depth = 0i32;
    for (i, token) in tokens.iter().enumerate().skip(open) {
        match token.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    tokens.len().saturating_sub(1)
}

/// The index after the `<...>` that opens at `open` (an arrow `->` is not a closer).
fn skip_angles(tokens: &[String], open: usize) -> usize {
    let mut depth = 0i32;
    let mut i = open;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "<" => depth += 1,
            ">" if i > 0 && tokens[i - 1] != "-" && tokens[i - 1] != "=" => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    tokens.len()
}

/// `tokens` cut at the commas outside every bracket.
fn split_commas(tokens: &[String]) -> Vec<&[String]> {
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, token) in tokens.iter().enumerate() {
        match token.as_str() {
            "(" | "[" | "{" | "<" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            ">" if i > 0 && tokens[i - 1] != "-" && tokens[i - 1] != "=" => depth -= 1,
            "," if depth == 0 => {
                pieces.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(&tokens[start..]);
    pieces
}

/// A field or variant piece without its attributes and visibility.
fn strip_attrs_and_visibility(mut piece: &[String]) -> &[String] {
    loop {
        if starts(piece, 0, &["#", "["]) {
            piece = &piece[matching(piece, 1) + 1..];
        } else if at(piece, 0) == "pub" {
            piece = &piece[1..];
            if at(piece, 0) == "(" {
                piece = &piece[matching(piece, 0) + 1..];
            }
        } else {
            return piece;
        }
    }
}

#[derive(Clone, Debug)]
struct Field {
    name: String,
    /// The identifiers of the field's type: `Option<Vec<Row>>` is `Option`, `Vec`, `Row`.
    ty: Vec<String>,
}

#[derive(Clone, Debug)]
struct Variant {
    /// Empty for a struct's one body.
    name: String,
    fields: Vec<Field>,
}

#[derive(Clone, Debug)]
struct Def {
    file: String,
    name: String,
    is_enum: bool,
    variants: Vec<Variant>,
}

fn named_fields(body: &[String]) -> Vec<Field> {
    split_commas(body)
        .into_iter()
        .filter_map(|piece| {
            let piece = strip_attrs_and_visibility(piece);
            (is_ident(at(piece, 0)) && at(piece, 1) == ":").then(|| Field {
                name: piece[0].clone(),
                ty: piece[2..].iter().filter(|t| is_ident(t)).cloned().collect(),
            })
        })
        .collect()
}

fn tuple_fields(body: &[String]) -> Vec<Field> {
    split_commas(body)
        .into_iter()
        .map(strip_attrs_and_visibility)
        .filter(|piece| !piece.is_empty())
        .enumerate()
        .map(|(n, piece)| Field {
            name: n.to_string(),
            ty: piece.iter().filter(|t| is_ident(t)).cloned().collect(),
        })
        .collect()
}

fn enum_variants(body: &[String]) -> Vec<Variant> {
    split_commas(body)
        .into_iter()
        .filter_map(|piece| {
            let piece = strip_attrs_and_visibility(piece);
            if !is_ident(at(piece, 0)) {
                return None;
            }
            let fields = match at(piece, 1) {
                "{" => named_fields(&piece[2..matching(piece, 1)]),
                "(" => tuple_fields(&piece[2..matching(piece, 1)]),
                _ => Vec::new(),
            };
            Some(Variant {
                name: piece[0].clone(),
                fields,
            })
        })
        .collect()
}

/// The text before its test module (`#[cfg(test)] mod`, at column 0): types of the tests are no
/// domain.
fn without_tests(text: &str) -> &str {
    text.split_once("\n#[cfg(test)]\nmod ")
        .map_or(text, |(code, _)| code)
}

/// Every `struct` and `enum` of the files with its fields (macro-generated ones come out with the
/// fields their text has, which for `screen_list!` and `feature_dirs!` is none).
fn defs_of(files: &Files) -> Vec<Def> {
    let mut defs = Vec::new();
    for (file, text) in files {
        let tokens = code_tokens(without_tests(text));
        let mut i = 0;
        while i + 2 < tokens.len() {
            let keyword = tokens[i].as_str();
            let after_path = i > 0 && matches!(tokens[i - 1].as_str(), "." | ":" | "$");
            if matches!(keyword, "struct" | "enum") && is_ident(&tokens[i + 1]) && !after_path {
                let name = tokens[i + 1].clone();
                let mut j = i + 2;
                if at(&tokens, j) == "<" {
                    j = skip_angles(&tokens, j);
                }
                while j < tokens.len() && !matches!(tokens[j].as_str(), "{" | "(" | ";") {
                    j += 1;
                }
                let is_enum = keyword == "enum";
                let variants = match at(&tokens, j) {
                    "{" => {
                        let end = matching(&tokens, j);
                        let body = &tokens[j + 1..end];
                        i = end;
                        if is_enum {
                            enum_variants(body)
                        } else {
                            vec![Variant {
                                name: String::new(),
                                fields: named_fields(body),
                            }]
                        }
                    }
                    "(" if !is_enum => {
                        let end = matching(&tokens, j);
                        let fields = tuple_fields(&tokens[j + 1..end]);
                        i = end;
                        vec![Variant {
                            name: String::new(),
                            fields,
                        }]
                    }
                    _ => Vec::new(),
                };
                defs.push(Def {
                    file: file.clone(),
                    name,
                    is_enum,
                    variants,
                });
            }
            i += 1;
        }
    }
    defs
}

// ---- 1. adr013_domain_types_hold_no_draw_state ----

/// What a drawn value goes by (full §13): a field named by one of these words, whole or as a part
/// of its snake-case name (`selected_row`, `is_open`), is the cursor, the scroll, the tab... of a
/// view. The old rule's words (`help`, `confirm`, `searching`, `draft`) stay.
const DRAW_WORDS: &[&str] = &[
    "cursor",
    "selected",
    "scroll",
    "focus",
    "highlight",
    "hover",
    "tab",
    "page",
    "stack",
    "kept",
    "expanded",
    "density",
    "menu",
    "shown",
    "open",
];

/// What the old `core_screen_models_hold_no_ui_state_fields` also denied of a screen's `Model`, by
/// exact name. Elsewhere in the domain these names are not drawn state: a `draft` rides in a
/// directive (the text the domain gives back to the composer) and `help` is a card's content.
const OLD_MODEL_WORDS: &[&str] = &["help", "confirm", "searching", "draft"];

/// Position-like words that a server's answer or a request uses as well (`cursor` of a page,
/// `selected` marketplaces, `page` token): of a field whose type is only text they are domain
/// (the IO test's question 2), of an index or a row they are the view's.
const POSITION_WORDS: &[&str] = &["cursor", "selected", "page"];

/// A part that makes a name a request's own (the cursor a server answered with, the page a list
/// was asked for): the IO test's question 2, domain.
const REQUEST_NAMES: &[&str] = &["next_cursor", "cursor_next", "page_size", "page_token"];

/// Only text: a token, a name, a list of names.
fn is_text_type(ty: &[String]) -> bool {
    ty.iter().all(|t| {
        matches!(
            t.as_str(),
            "Option" | "Vec" | "String" | "str" | "Rc" | "Arc" | "Box"
        )
    })
}

fn is_draw_name(def: &Def, field: &Field) -> bool {
    let name = field.name.as_str();
    if REQUEST_NAMES.contains(&name) {
        return false;
    }
    if def.name == "Model"
        && def.file.starts_with("core/src/screens/")
        && OLD_MODEL_WORDS.contains(&name)
    {
        return true;
    }
    name.split('_').any(|part| {
        DRAW_WORDS.contains(&part) && !(POSITION_WORDS.contains(&part) && is_text_type(&field.ty))
    })
}

/// Types of the UI toolkit: a field of one of them is a view's, whatever its name.
fn is_draw_type(ident: &str) -> bool {
    ident.ends_with("Controller") || matches!(ident, "ScrollState" | "FocusNode" | "GlobalKey")
}

/// The roots of the domain: the app, a thread's model, and the macro-generated `ScreenStates`
/// (each directory's `Model`), `ThreadSlots` (its `ThreadPart`), `FeatureStates` (each feature's
/// `State`) and `FeatureThreadParts`: their fields are not in the text, their types are named here.
fn is_domain_root(def: &Def) -> bool {
    matches!(def.name.as_str(), "CodexApp" | "ThreadModel")
        || (def.file.starts_with("core/src/screens/")
            && matches!(def.name.as_str(), "Model" | "ThreadPart"))
        || (def.file.starts_with("core/src/features/")
            && matches!(def.name.as_str(), "State" | "ThreadPart"))
}

/// The defs a root reaches through its fields' types, resolved by name (a name two files share
/// makes both reachable: more than the truth, never less).
fn reachable(defs: &[Def]) -> Vec<&Def> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, def) in defs.iter().enumerate() {
        by_name.entry(def.name.as_str()).or_default().push(i);
    }
    let mut seen = vec![false; defs.len()];
    let mut queue: Vec<usize> = Vec::new();
    for (i, def) in defs.iter().enumerate() {
        if is_domain_root(def) {
            seen[i] = true;
            queue.push(i);
        }
    }
    while let Some(i) = queue.pop() {
        for field in defs[i].variants.iter().flat_map(|v| &v.fields) {
            for ident in &field.ty {
                for &j in by_name.get(ident.as_str()).into_iter().flatten() {
                    if !seen[j] {
                        seen[j] = true;
                        queue.push(j);
                    }
                }
            }
        }
    }
    defs.iter()
        .zip(seen)
        .filter_map(|(def, seen)| seen.then_some(def))
        .collect()
}

/// The domain files: core's sources but the fixtures (scripts and the sandbox are test machinery).
fn domain_defs(core: &Files) -> Vec<Def> {
    let domain: Vec<(String, String)> = core
        .iter()
        .filter(|(name, _)| !name.starts_with("core/src/fixtures/"))
        .cloned()
        .collect();
    defs_of(&domain)
}

/// `file:Type.field` (`file:Type::Variant.field`) of every field a type reachable from the domain
/// roots has with a drawing name or type.
fn draw_fields(core: &Files) -> Vec<String> {
    let defs = domain_defs(core);
    let mut found = Vec::new();
    for def in reachable(&defs) {
        for variant in &def.variants {
            let owner = if variant.name.is_empty() {
                def.name.clone()
            } else {
                format!("{}::{}", def.name, variant.name)
            };
            for field in &variant.fields {
                if is_draw_name(def, field) || field.ty.iter().any(|t| is_draw_type(t)) {
                    found.push(format!("{}:{owner}.{}", def.file, field.name));
                }
            }
        }
    }
    found.sort();
    found
}

/// What the old `core_screen_models_hold_no_ui_state_fields` held for the screens' `Model`s, now
/// for every type the domain reaches (`CodexApp`, `ThreadModel`, the screens' and the features'
/// states). Core keeps the IO's values (full §1.3): a cursor, a selection, a scroll, a tab, a sheet's
/// page stack, `expanded`, `density`, `open` are the widget's `State`, a rtwk controller or
/// `ThreadUi`. The families of full §14 (select::Menu, settings, dialogs, resume, pets) still hold
/// them and are the ratchet.
#[test]
fn adr013_domain_types_hold_no_draw_state() {
    let core = sources_of("core/src");
    assert!(
        reachable(&domain_defs(&core)).len() > 20,
        "the reachability scan found no domain: it reads `struct`s of core/src"
    );
    check(
        "adr013_domain_types_hold_no_draw_state",
        "a drawn value is UI state: widget State, a rtwk controller or ThreadUi, never core",
        &draw_fields(&core),
    );
}

#[test]
fn the_draw_state_scan_bites() {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let files = vec![
        file(
            "core/src/app.rs",
            "pub struct CodexApp { pub screens: Rc<ScreenStates>, pub threads: ThreadSet, pub rows: Vec<Row> }\n",
        ),
        file(
            "core/src/screens/foo.rs",
            "pub struct Model {\n    /// the first row on screen\n    pub scroll_top: usize,\n    pub cursor: usize,\n    pub next_cursor: Option<String>,\n    pub cursor_token: Option<String>,\n    pub selected: Vec<String>,\n    pub rows: Vec<Row>,\n    pub pane: Pane,\n    pub is_open: bool,\n    pub query: String,\n    pub draft: String,\n    pub ctl: ListController,\n}\npub struct Pane { #[allow(x)] pub(crate) expanded: bool, pub name: String, pub draft: String }\npub enum Step { Pick { highlight: usize, id: u8 }, Done(bool) }\npub struct Unreached { pub cursor: usize }\n",
        ),
        file(
            "core/src/threads.rs",
            "pub struct ThreadModel { pub step: Step }\n#[cfg(test)]\nmod tests { struct Fixture { cursor: usize } }\n",
        ),
        file(
            "core/src/screens/other.rs",
            "pub struct Row { pub selected: bool }\n",
        ),
        // a comment and a string do not hold a field
        file(
            "core/src/screens/quiet.rs",
            "// pub struct Model { pub cursor: usize }\nconst DOC: &str = \"pub struct Model { pub open: bool }\";\n",
        ),
    ];
    assert_eq!(
        draw_fields(&files),
        [
            "core/src/screens/foo.rs:Model.cursor",
            "core/src/screens/foo.rs:Model.ctl",
            "core/src/screens/foo.rs:Model.draft",
            "core/src/screens/foo.rs:Model.is_open",
            "core/src/screens/foo.rs:Model.scroll_top",
            "core/src/screens/foo.rs:Pane.expanded",
            "core/src/screens/foo.rs:Step::Pick.highlight",
            "core/src/screens/other.rs:Row.selected",
        ]
        .map(str::to_owned)
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );
}

// ---- 2. adr013_kit_is_not_app_state ----

const DOMAIN_ROOTS: &[&str] = &["CodexApp", "ThreadModel", "ScreenStates", "FeatureStates"];

/// `core::kit` is pure logic without app state (vim, paste burst, history, mention codec, command
/// filter, backtrack machine): its types belong to whoever holds them, the UI, never to a field of a
/// domain type, and its files name no root of the domain.
fn kit_leaks(core: &Files) -> Vec<String> {
    let mut found = Vec::new();
    let kit: Vec<(String, String)> = core
        .iter()
        .filter(|(name, _)| name.starts_with("core/src/kit/"))
        .cloned()
        .collect();
    for (file, text) in &kit {
        let tokens = code_tokens(without_tests(text));
        for root in DOMAIN_ROOTS {
            if tokens.iter().any(|t| t == root) {
                found.push(format!("{file}:names:{root}"));
            }
        }
    }
    let kit_types: BTreeSet<String> = defs_of(&kit).into_iter().map(|def| def.name).collect();
    for def in reachable(&domain_defs(core)) {
        if def.file.starts_with("core/src/kit/") {
            continue;
        }
        for variant in &def.variants {
            for field in &variant.fields {
                if let Some(ty) = field.ty.iter().find(|t| kit_types.contains(*t)) {
                    found.push(format!(
                        "{}:{}.{}:holds:{ty}",
                        def.file, def.name, field.name
                    ));
                }
            }
        }
    }
    found.sort();
    found
}

#[test]
fn adr013_kit_is_not_app_state() {
    check(
        "adr013_kit_is_not_app_state",
        "core::kit is logic without state: the UI holds its types, the domain never does",
        &kit_leaks(&sources_of("core/src")),
    );
}

/// A kit file names a `crate::` path of `kit`, `data` or `ids` only (an allowlist, like
/// `data_names_others`): the kit depends on std, protocol types, `core::data` and `core::ids`, never on a
/// screen, a feature, the commands or the threads that depend on it (full §2, layer table). A bare
/// `screens::`/`features::` (a re-export through an alias) is the same hit.
fn kit_names_screens(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| name.starts_with("core/src/kit/"),
        |tokens, i| {
            (starts(tokens, i, &["crate", ":", ":"])
                && !matches!(at(tokens, i + 3), "kit" | "data" | "ids"))
                || (matches!(at(tokens, i), "screens" | "features")
                    && starts(tokens, i + 1, &[":", ":"])
                    && !matches!(at(tokens, i.wrapping_sub(1)), "." | ":"))
        },
    )
}

#[test]
fn adr013_kit_names_no_screen() {
    check(
        "adr013_kit_names_no_screen",
        "core::kit names no module of core but kit, data and ids: the values it reads are core::data, so the screens use the kit and not the other way round",
        &kit_names_screens(&sources_of("core/src")),
    );
}

/// `core::data` holds plain values: it names no module of core but itself and `ids`.
fn data_names_others(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| name == "core/src/data.rs" || name.starts_with("core/src/data/"),
        |tokens, i| {
            starts(tokens, i, &["crate", ":", ":"]) && !matches!(at(tokens, i + 3), "data" | "ids")
        },
    )
}

#[test]
fn adr013_data_names_only_ids() {
    check(
        "adr013_data_names_only_ids",
        "core::data is plain domain values shared by kit, screens and widgets: it depends on std, protocol and ids only",
        &data_names_others(&sources_of("core/src")),
    );
}

/// The history `Store` is written by `apply` only (`popovers::Ev::History*`): its writers are `pub`
/// for `core/tests`, so the UI and the host must never call them nor build a store of their own
/// (`Shared<Store>` is the app's; the UI reads it through `&Store`).
fn store_written_in_ui(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| name.starts_with("src/") || name.starts_with("host/src/"),
        |tokens, i| {
            (at(tokens, i) == "."
                && matches!(
                    at(tokens, i + 1),
                    "set_metadata" | "on_entry" | "on_batch" | "on_batch_error"
                )
                && at(tokens, i + 2) == "(")
                || (at(tokens, i) == "Store"
                    && starts(tokens, i + 1, &[":", ":"])
                    && matches!(at(tokens, i + 3), "new" | "seeded"))
        },
    )
}

#[test]
fn adr013_store_written_only_by_apply() {
    let mut files = sources_of("src");
    files.extend(sources_of("host/src"));
    check(
        "adr013_store_written_only_by_apply",
        "the history Store changes only through apply (popovers::Ev::History*): its pub writers are for core/tests, the UI and the host read it through &Store",
        &store_written_in_ui(&files),
    );
}

#[test]
fn the_store_scan_bites() {
    let files = vec![
        (
            "src/widgets/composer_draft.rs".to_owned(),
            "store.on_entry(1, 0, None);\nlet s = Store::seeded(v);\nlet t = history::Store::new();\nstore.set_metadata(1, 2);\n// store.on_batch(\nlet ok = Store::default();\nfn on_entry() {}\n".to_owned(),
        ),
        (
            "core/src/screens/popovers.rs".to_owned(),
            "model.history.on_entry(1, 0, t);\n".to_owned(),
        ),
    ];
    assert_eq!(
        store_written_in_ui(&files),
        [
            "src/widgets/composer_draft.rs",
            "src/widgets/composer_draft.rs",
            "src/widgets/composer_draft.rs",
            "src/widgets/composer_draft.rs"
        ]
    );
}

#[test]
fn the_kit_and_data_scans_bite() {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let files = vec![
        file(
            "core/src/kit/mentions/popup.rs",
            "use crate::screens::composer::Row;\npub use crate::features::x::Y;\n// screens::z\nlet n = a.screens::b;\nuse crate::commands::fuzzy_match;\nuse crate::kit::fuzzy::f;\nuse crate::ids::ThreadId;\nuse screens::q;\n",
        ),
        file("core/src/kit/vim.rs", "use crate::data::mention::Row;\n"),
        file(
            "core/src/data/mention.rs",
            "use crate::ids::ThreadId;\nuse crate::kit::codec::f;\nuse crate::data::task::T;\n",
        ),
        file(
            "core/src/data.rs",
            "pub mod files;\nuse crate::threads::X;\n",
        ),
    ];
    assert_eq!(
        kit_names_screens(&files),
        [
            "core/src/kit/mentions/popup.rs",
            "core/src/kit/mentions/popup.rs",
            "core/src/kit/mentions/popup.rs",
            "core/src/kit/mentions/popup.rs"
        ]
    );
    assert_eq!(
        data_names_others(&files),
        ["core/src/data/mention.rs", "core/src/data.rs"]
    );
}

#[test]
fn the_kit_scan_bites() {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let files = vec![
        file(
            "core/src/kit/vim.rs",
            "pub struct Vim { pub mode: u8 }\nfn f(app: &CodexApp) {}\n",
        ),
        file(
            "core/src/kit/mod.rs",
            "// ThreadModel in a comment\npub mod vim;\n",
        ),
        file(
            "core/src/app.rs",
            "pub struct CodexApp { pub vim: Vim, pub n: u8 }\n",
        ),
    ];
    assert_eq!(
        kit_leaks(&files),
        [
            "core/src/app.rs:CodexApp.vim:holds:Vim",
            "core/src/kit/vim.rs:names:CodexApp"
        ]
    );
    // no kit folder: nothing to leak
    assert!(kit_leaks(&files[2..]).is_empty());
}

// ---- 3. adr013_feature_trait_has_no_view_types ----

/// The lines of the block that a line of `text` opens (one `header` accepts) up to the `}` at column 0.
fn block(text: &str, header: impl Fn(&str) -> bool) -> Vec<&str> {
    text.lines()
        .skip_while(|line| !header(line))
        .skip(1)
        .take_while(|line| *line != "}")
        .collect()
}

/// The names of the items at the block's own depth (four spaces in) that `banned` lists: the
/// associated types and the functions, as `Owner::name`.
fn banned_items(owner: &str, block: &[&str], types: &[&str], fns: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for line in block {
        let Some(item) = line.strip_prefix("    ") else {
            continue;
        };
        if item.starts_with(' ') {
            continue;
        }
        let item = item
            .strip_prefix("pub(crate) ")
            .or_else(|| item.strip_prefix("pub "))
            .unwrap_or(item);
        let name = |rest: &str| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        };
        if let Some(rest) = item.strip_prefix("type ")
            && types.contains(&name(rest).as_str())
        {
            found.push(format!("{owner}::{}", name(rest)));
        }
        if let Some(rest) = item.strip_prefix("fn ")
            && fns.contains(&name(rest).as_str())
        {
            found.push(format!("{owner}::{}", name(rest)));
        }
    }
    found
}

/// The view forms a feature declares in core (`Menu`, `Pick`, `Prompt`, `choose`, `picked`,
/// `prompt_submitted`), the contexts that open them (`AppCx::open`, `AppCx::prompt`) and the view
/// hooks that draw them (`FeatureView::menu_list`, `prompt_words`).
fn feature_view_forms(feature: &str, cx: &str, view: &str) -> Vec<String> {
    let is_trait = |name: &'static str| {
        move |line: &str| {
            let line = line.trim_start();
            let line = line.strip_prefix("pub ").unwrap_or(line);
            line.strip_prefix(&format!("trait {name}"))
                .is_some_and(|rest| rest.starts_with([':', ' ', '<', '{']))
        }
    };
    let mut found = banned_items(
        "Feature",
        &block(feature, is_trait("Feature")),
        &["Menu", "Pick", "Prompt"],
        &["choose", "picked", "prompt_submitted"],
    );
    found.extend(banned_items(
        "AppCx",
        &block(cx, |line| {
            line.starts_with("impl") && line.contains("AppCx") && !line.contains(" for ")
        }),
        &[],
        &["open", "prompt"],
    ));
    found.extend(banned_items(
        "FeatureView",
        &block(view, is_trait("FeatureView")),
        &[],
        &["menu_list", "prompt_words"],
    ));
    found
}

#[test]
fn adr013_feature_trait_has_no_view_types() {
    let read = |rel: &str| {
        std::fs::read_to_string(crate_dir().join(rel)).unwrap_or_else(|_| panic!("read {rel}"))
    };
    let found = feature_view_forms(
        &read("core/src/features/mod.rs"),
        &read("core/src/features/cx.rs"),
        &read("src/features/mod.rs"),
    );
    check(
        "adr013_feature_trait_has_no_view_types",
        "a feature's visible half is src/features/<f>/ (open + its own sheet), not a Menu/Pick/Prompt of core",
        &found,
    );
}

#[test]
fn the_feature_view_scan_bites() {
    let feature = "pub trait Feature: 'static {\n    type State: Clone;\n    type Menu: Clone;\n    type Pick;\n    type Prompt;\n    type Promptly;\n    fn intent(state: &mut Self::State) {}\n    fn choose(menu: &Self::Menu) {}\n    fn picked(state: &mut Self::State) {}\n    fn prompt_submitted(\n        state: &mut Self::State,\n    ) {}\n    fn on_fact() {\n        fn picked() {}\n    }\n}\n\npub trait FeatureView: Feature {\n    fn menu_list() {}\n}\n";
    let cx = "impl<'a> AppCx<'a> {\n    pub fn ask() {}\n    pub fn open(&mut self) {}\n    pub fn prompt(&mut self) {}\n}\n\nimpl From<u8> for AppCx<'_> {\n    fn open() {}\n}\n";
    let view = "pub trait FeatureView: Feature {\n    fn note_words() {}\n    fn menu_list() {}\n    fn prompt_words() {}\n}\n";
    assert_eq!(
        feature_view_forms(feature, cx, view),
        [
            "Feature::Menu",
            "Feature::Pick",
            "Feature::Prompt",
            "Feature::choose",
            "Feature::picked",
            "Feature::prompt_submitted",
            "AppCx::open",
            "AppCx::prompt",
            "FeatureView::menu_list",
            "FeatureView::prompt_words",
        ]
    );
    let clean = "pub trait Feature: 'static {\n    type State;\n    fn intent() {}\n}\n";
    assert!(
        feature_view_forms(
            clean,
            "impl<'a> AppCx<'a> {\n    pub fn ask() {}\n}\n",
            clean
        )
        .is_empty()
    );
}

// ---- 4. adr013_core_intents_name_no_gesture ----

/// Variants that name a gesture, not a verb of the domain (full §13): the widget decides what a tap, a
/// key or a wheel means and sends the verb.
const GESTURES: &[&str] = &[
    "Choose",
    "Secondary",
    "Toggle",
    "Reorder",
    "Close",
    "Back",
    "Tab",
    "Hot",
    "Highlight",
    "Cursor",
    "Expand",
    "ToggleDensity",
    "Activate",
    "Preview",
    "Move",
    "Room",
    "End",
];

/// `file:Enum::Variant` of every variant of an enum of core named `*Intent` whose name is a gesture.
fn gesture_intents(core: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for def in domain_defs(core) {
        if def.is_enum && def.name.ends_with("Intent") {
            for variant in &def.variants {
                if GESTURES.contains(&variant.name.as_str()) {
                    found.push(format!("{}:{}::{}", def.file, def.name, variant.name));
                }
            }
        }
    }
    found.sort();
    found
}

#[test]
fn adr013_core_intents_name_no_gesture() {
    check(
        "adr013_core_intents_name_no_gesture",
        "an intent is a verb of the domain (Submit, Search{query}), never a gesture the widget decided (Choose, Toggle, Cursor...)",
        &gesture_intents(&sources_of("core/src")),
    );
}

#[test]
fn the_gesture_scan_bites() {
    let files = vec![(
        "core/src/screens/foo.rs".to_owned(),
        "pub enum Intent {\n    Submit(String),\n    Choose(usize),\n    #[allow(x)]\n    Toggle { id: u8 },\n    Close,\n    // Back\n    Ended,\n}\nenum PageIntent { Tab(u8), Search { query: String } }\nenum Outcome { Close, Back }\n".to_owned(),
    )];
    assert_eq!(
        gesture_intents(&files),
        [
            "core/src/screens/foo.rs:Intent::Choose",
            "core/src/screens/foo.rs:Intent::Close",
            "core/src/screens/foo.rs:Intent::Toggle",
            "core/src/screens/foo.rs:PageIntent::Tab",
        ]
    );
}

// ---- 5-9. What the UI may name: the screen half, tickets, interest, directives, wishes ----

/// The tokens of every file `keep` selects, with the file: the scans below run per token.
fn token_files(files: &Files, keep: impl Fn(&str) -> bool) -> Vec<(&str, Vec<String>)> {
    files
        .iter()
        .filter(|(name, _)| keep(name))
        .map(|(name, text)| (name.as_str(), code_tokens(text)))
        .collect()
}

/// One key (the file) per place a file's tokens satisfy `hit` at.
fn hits_by_file(
    files: &Files,
    keep: impl Fn(&str) -> bool,
    hit: impl Fn(&[String], usize) -> bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for (name, tokens) in token_files(files, keep) {
        for i in 0..tokens.len() {
            if hit(&tokens, i) {
                found.push(name.to_owned());
            }
        }
    }
    found
}

fn in_ui_sources(name: &str) -> bool {
    name.starts_with("src/")
}

/// The widgets' file never names a path through `screens::` or `features::`: neither core's (the
/// domain of a screen or a feature) nor the UI crate's own (a screen's half): a shared brick is
/// independent of the screens it serves (`Picker` takes its list as props).
fn screen_half_named(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| name.starts_with("src/widgets/"),
        |tokens, i| {
            matches!(at(tokens, i), "screens" | "features")
                && starts(tokens, i + 1, &[":", ":"])
                && at(tokens, i.wrapping_sub(1)) != "."
        },
    )
}

#[test]
fn adr013_widgets_name_no_screen_half() {
    check(
        "adr013_widgets_name_no_screen_half",
        "a widget of src/widgets/ takes props, never a type of core::screens or core::features: the picker is independent of its screens",
        &screen_half_named(&sources_of("src")),
    );
}

#[test]
fn the_screen_half_scan_bites() {
    let files = vec![
        (
            "src/widgets/x.rs".to_owned(),
            "use codex_tui_core::screens::select::list::Row;\nuse codex_tui_core::{features::Goal, ids::ThreadId};\n// use crate::screens::y;\nlet n = screens.len();\n".to_owned(),
        ),
        (
            "src/pages/y.rs".to_owned(),
            "use codex_tui_core::screens::select::list::Row;\n".to_owned(),
        ),
    ];
    assert_eq!(
        screen_half_named(&files),
        ["src/widgets/x.rs", "src/widgets/x.rs"]
    );
}

/// The ticket of a request is core's (full §1.4.5): the UI matches an answer by the domain key of its
/// operation. The banner is the exception, where the ticket is the id of the entity (`exempt` lines).
fn tickets_in_ui(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| {
            ["src/pages/", "src/sheets/", "src/features/", "src/widgets/"]
                .iter()
                .any(|root| name.starts_with(root))
        },
        |tokens, i| at(tokens, i) == "Ticket",
    )
}

#[test]
fn adr013_no_ticket_in_ui() {
    check(
        "adr013_no_ticket_in_ui",
        "the UI matches answers by the operation's domain key (OpKey), never by a Ticket",
        &tickets_in_ui(&sources_of("src")),
    );
}

#[test]
fn the_ticket_scan_bites() {
    let files = vec![
        (
            "src/sheets/plugins/mod.rs".to_owned(),
            "use codex_tui_core::ids::Ticket;\nstruct S { waiting: Option<Ticket> }\nstruct T(BannerTicket);\n// Ticket\n".to_owned(),
        ),
        (
            "src/app/view.rs".to_owned(),
            "fn f(t: Ticket) {}\n".to_owned(),
        ),
    ];
    assert_eq!(
        tickets_in_ui(&files),
        ["src/sheets/plugins/mod.rs", "src/sheets/plugins/mod.rs"]
    );
}

/// An `Intent::Interest` (a surface wants data: open, close, dispose) is made by the one `Interest`
/// widget (full §4.7), so the counter in core can never be left up by a surface that forgot to close.
fn interest_made_elsewhere(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| in_ui_sources(name) && name != "src/widgets/interest.rs",
        |tokens, i| {
            starts(tokens, i, &["Intent", ":", ":", "Interest"]) || at(tokens, i) == "InterestStep"
        },
    )
}

#[test]
fn adr013_interest_made_in_one_place() {
    check(
        "adr013_interest_made_in_one_place",
        "Intent::Interest and InterestStep are made only by the Interest widget (src/widgets/interest.rs), so a counter is never left up",
        &interest_made_elsewhere(&sources_of("src")),
    );
}

#[test]
fn the_interest_scan_bites() {
    let files = vec![
        (
            "src/sheets/plugins/mod.rs".to_owned(),
            "cx.send(Intent::Interest(Interest::Open(k)));\n// Intent::Interest\nlet s = InterestStep::Open;\n".to_owned(),
        ),
        (
            "src/widgets/interest.rs".to_owned(),
            "cx.send(Intent::Interest(Interest::Close(k)));\n".to_owned(),
        ),
    ];
    assert_eq!(
        interest_made_elsewhere(&files),
        ["src/sheets/plugins/mod.rs", "src/sheets/plugins/mod.rs"]
    );
}

/// A `Directive` (the domain tells a controller what to do: restore a draft, put text) is read by the
/// one listener (full §4.6): `Directive<..>`, `.restore()` (the thread's pending restore, no
/// arguments) and `submit_seq` stand only in `src/widgets/directives.rs`, and so does the number a
/// one-shot is counted by (`prompt.seq`, `Restore::seq`): a page or a sheet that read one would be a
/// second listener, carrying the one-shot out twice or losing it to a rebuild. The markdown
/// `Directive` of the citations is not generic, `restore(&text)` of the citations and the search
/// preview is another verb, and the `seq` of the presenters' wishes and of the composer's elements
/// are other counters (outside pages and sheets).
fn directives_read_elsewhere(files: &Files) -> Vec<String> {
    let mut found = hits_by_file(
        files,
        |name| in_ui_sources(name) && name != "src/widgets/directives.rs",
        |tokens, i| {
            (at(tokens, i) == "Directive" && at(tokens, i + 1) == "<")
                || (starts(tokens, i, &["restore", "(", ")"])
                    && at(tokens, i.wrapping_sub(1)) == ".")
                || at(tokens, i) == "submit_seq"
        },
    );
    found.extend(hits_by_file(
        files,
        |name| name.starts_with("src/pages/") || name.starts_with("src/sheets/"),
        |tokens, i| at(tokens, i) == "seq" && matches!(at(tokens, i.wrapping_sub(1)), "." | ":"),
    ));
    found
}

#[test]
fn adr013_directives_read_in_one_place() {
    check(
        "adr013_directives_read_in_one_place",
        "a Directive is read only by the listener (src/widgets/directives.rs), by its seq",
        &directives_read_elsewhere(&sources_of("src")),
    );
}

#[test]
fn the_directive_scan_bites() {
    let files = vec![
        (
            "src/pages/session/session_page.rs".to_owned(),
            "let seq = thread.restore().map(Restore::seq);\nlet d: Option<Directive<Restore>> = None;\nfn restore() {}\nrestore(x);\nself.restore(c, &snapshot);\nlet t = directives.restore(&title);\nenum Directive { Citation }\n".to_owned(),
        ),
        (
            "src/pages/session/live/plan_mode.rs".to_owned(),
            "let seq = thread.screen_slots().chat.plan.prompt().map(|prompt| prompt.seq);\nlet n = thread.submit_seq();\nlet seq = 3; // prompt.seq\n".to_owned(),
        ),
        (
            "src/widgets/directives.rs".to_owned(),
            "fn r(d: Directive<Restore>) { d.restore() }\nlet n = prompt.seq + thread.submit_seq();\n".to_owned(),
        ),
        (
            "src/app/presenters/mod.rs".to_owned(),
            "let fresh = wish.seq > taken;\n".to_owned(),
        ),
        (
            "src/widgets/composer_draft.rs".to_owned(),
            "elements.sort_by_key(|e| e.seq);\n".to_owned(),
        ),
    ];
    // `.restore(` and `Directive<` and the `seq` of `Restore::seq`, in a page (three); `.seq` and
    // `submit_seq` in a page (two more); the declaration `fn restore(`, the citations' enum, a local
    // named `seq`, a comment, the listener itself and the other counters' `seq` are not
    assert_eq!(
        directives_read_elsewhere(&files),
        [
            "src/pages/session/session_page.rs",
            "src/pages/session/session_page.rs",
            "src/pages/session/live/plan_mode.rs",
            "src/pages/session/session_page.rs",
            "src/pages/session/live/plan_mode.rs",
        ]
    );
}

/// `WishShown` (the UI tells the domain a wished surface is on screen) is sent by the presenters only
/// (full §4.4): a page that sent it would show a surface the presenter did not.
fn wish_shown_elsewhere(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| in_ui_sources(name) && !name.starts_with("src/app/presenters/"),
        |tokens, i| at(tokens, i) == "WishShown",
    )
}

#[test]
fn adr013_wishes_shown_by_the_presenter() {
    check(
        "adr013_wishes_shown_by_the_presenter",
        "the domain's wish to show a surface is answered (WishShown) only by the presenter (src/app/presenters)",
        &wish_shown_elsewhere(&sources_of("src")),
    );
}

#[test]
fn the_wish_scan_bites() {
    let files = vec![
        (
            "src/pages/session/open.rs".to_owned(),
            "cx.send(Intent::WishShown(id));\n".to_owned(),
        ),
        (
            "src/app/presenters/auth.rs".to_owned(),
            "cx.send(Intent::WishShown(id));\n".to_owned(),
        ),
    ];
    assert_eq!(wish_shown_elsewhere(&files), ["src/pages/session/open.rs"]);
}

// ---- 10. adr013_sheets_declare_alive ----

/// `file:Type` of every `impl Sheet for Type` of the UI whose body has no `alive`: a sheet says when it
/// lives (the request is still there, the scope is current, the thread has not changed: full §6.4), so
/// nothing stays open over what it was opened for.
fn sheets_without_alive(files: &Files) -> Vec<String> {
    let mut found = Vec::new();
    for (name, tokens) in token_files(files, in_ui_sources) {
        for (i, token) in tokens.iter().enumerate() {
            if token != "impl" {
                continue;
            }
            let Some(open) = (i..tokens.len()).find(|&j| matches!(at(&tokens, j), "{" | ";"))
            else {
                continue;
            };
            let header = &tokens[i..open];
            let Some(at_sheet) =
                (0..header.len()).find(|&k| header[k] == "Sheet" && at(header, k + 1) == "for")
            else {
                continue;
            };
            if at(&tokens, open) != "{" {
                continue;
            }
            let body = &tokens[open..=matching(&tokens, open)];
            if !body.iter().any(|t| t == "alive") {
                found.push(format!("{name}:{}", at(header, at_sheet + 2)));
            }
        }
    }
    found
}

#[test]
fn adr013_sheets_declare_alive() {
    check(
        "adr013_sheets_declare_alive",
        "a sheet declares when it lives (`alive`): the request is still there, the scope current, the thread unchanged",
        &sheets_without_alive(&sources_of("src")),
    );
}

#[test]
fn the_alive_scan_bites() {
    let files = vec![(
        "src/features/plugins/sheet.rs".to_owned(),
        "impl Sheet for PluginsSheet {\n    fn pages(&self) {}\n}\nimpl<'a> Sheet for Kept<'a> {\n    fn alive(&self) -> bool { true }\n}\nimpl Other for Plain {\n    fn pages(&self) {}\n}\nimpl Sheet for Loose;\n".to_owned(),
    )];
    assert_eq!(
        sheets_without_alive(&files),
        ["src/features/plugins/sheet.rs:PluginsSheet"]
    );
}

// ---- 11. adr013_no_whole_state_watch ----

/// A page that `watch`es the whole `ScreenStates` or `FeatureStates` wakes on every change of any
/// screen (full §11): it `select`s the slice it draws.
fn whole_state_watches(files: &Files) -> Vec<String> {
    hits_by_file(files, in_ui_sources, |tokens, i| {
        starts(tokens, i, &["watch", ":", ":", "<"])
            && matches!(at(tokens, i + 4), "ScreenStates" | "FeatureStates")
    })
}

#[test]
fn adr013_no_whole_state_watch() {
    check(
        "adr013_no_whole_state_watch",
        "a view selects the slice it draws; watching the whole ScreenStates or FeatureStates wakes it on every change",
        &whole_state_watches(&sources_of("src")),
    );
}

#[test]
fn the_whole_watch_scan_bites() {
    let files = vec![(
        "src/pages/resume/mod.rs".to_owned(),
        "let a = watch::<ScreenStates, _>(cx);\nlet b = watch::<FeatureStates, _>(cx);\nlet c = select::<ScreenStates, u8, _>(cx, f);\nlet d = read::<ScreenStates, _>(cx);\nlet e = watch::<ThreadModel, _>(cx);\n// watch::<ScreenStates, _>(cx)\n".to_owned(),
    )];
    assert_eq!(
        whole_state_watches(&files),
        ["src/pages/resume/mod.rs", "src/pages/resume/mod.rs"]
    );
}

// ---- 12. r2_thread_subtree_is_keyed ----

const KEY_CALLS: &[&[&str]] = &[
    &["with_key", "("],
    &["Key", ":", ":", "value", "("],
    &["Key", ":", ":", "from", "("],
    &["ValueKey", ":", ":", "new", "("],
];

/// A key built from a `ThreadId` makes a per-thread subtree: it is made by `thread_scope`, the one
/// helper (R2), so switching the thread remounts exactly the thread's subtree and no page keys by a
/// thread by its own rule.
fn thread_keys_elsewhere(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| in_ui_sources(name) && name != "src/widgets/thread_scope.rs",
        |tokens, i| {
            KEY_CALLS.iter().any(|call| {
                if !starts(tokens, i, call) || at(tokens, i.wrapping_sub(1)) == "fn" {
                    return false;
                }
                let open = i + call.len() - 1;
                let args = &tokens[open..=matching(tokens, open)];
                args.iter()
                    .any(|t| matches!(t.as_str(), "ThreadId" | "thread_id"))
                    || (0..args.len()).any(|k| starts(args, k, &["thread", ".", "id", "("]))
            })
        },
    )
}

#[test]
fn r2_thread_subtree_is_keyed() {
    check(
        "r2_thread_subtree_is_keyed",
        "a per-thread subtree is keyed by ThreadId only through the one `thread_scope` helper (R2)",
        &thread_keys_elsewhere(&sources_of("src")),
    );
}

#[test]
fn the_thread_key_scan_bites() {
    let files = vec![
        (
            "src/pages/session/mod.rs".to_owned(),
            "AnyWidget::from(page).with_key(Key::value(thread_id.clone()));\nrow.with_key(Key::value(\n    (thread.id().clone(), 1),\n));\nfoo.with_key(ThreadId::new(a));\n".to_owned(),
        ),
        (
            "src/pages/session/ok.rs".to_owned(),
            "fn with_key(thread_id: u8) {}\nx.with_key(Key::value((thread.read_only(), 1)));\nx.with_key(page);\nx.with_key(Key::value(ticket));\n".to_owned(),
        ),
        (
            "src/widgets/thread_scope.rs".to_owned(),
            "x.with_key(Key::value(thread_id.clone()));\n".to_owned(),
        ),
    ];
    // `with_key(Key::value(thread_id..))` is two calls with the id inside: both are found, as is the
    // `thread.id()` key and `ThreadId`; the declaration, the read-only key and the thread_scope file are not
    assert_eq!(
        thread_keys_elsewhere(&files),
        [
            "src/pages/session/mod.rs",
            "src/pages/session/mod.rs",
            "src/pages/session/mod.rs",
            "src/pages/session/mod.rs",
            "src/pages/session/mod.rs",
        ]
    );
}

// ---- 13. adr002_screen_and_sheet_widgets_are_eq ----

/// `file:Widget` of every `impl_widget!(stateful X)` of a page, a sheet or a feature without `eq`: a
/// widget that cannot say it is unchanged rebuilds its subtree on every parent build (ADR-002).
fn stateful_without_eq(files: &Files) -> Vec<String> {
    let mut found = Vec::new();
    let keep = |name: &str| {
        ["src/pages/", "src/sheets/", "src/features/"]
            .iter()
            .any(|root| name.starts_with(root))
    };
    for (name, tokens) in token_files(files, keep) {
        for i in 0..tokens.len() {
            if starts(&tokens, i, &["impl_widget", "!", "("]) && at(&tokens, i + 3) == "stateful" {
                let call = &tokens[i + 2..=matching(&tokens, i + 2)];
                if at(call, 2) != "eq" {
                    found.push(format!("{name}:{}", at(call, 2)));
                }
            }
        }
    }
    found
}

#[test]
fn adr002_screen_and_sheet_widgets_are_eq() {
    check(
        "adr002_screen_and_sheet_widgets_are_eq",
        "a page's, sheet's or feature's stateful widget is `eq` (impl_widget!(stateful eq X)), so a parent build leaves it alone",
        &stateful_without_eq(&sources_of("src")),
    );
}

#[test]
fn the_eq_scan_bites() {
    let files = vec![
        (
            "src/sheets/plugins/mod.rs".to_owned(),
            "impl_widget!(stateful PluginsSheet);\nimpl_widget!(stateful eq Fine);\nimpl_widget!(stateless Cell);\nimpl_widget!(\n    stateful\n    Spread\n);\n".to_owned(),
        ),
        (
            "src/widgets/markdown.rs".to_owned(),
            "impl_widget!(stateful Markdown);\n".to_owned(),
        ),
    ];
    assert_eq!(
        stateful_without_eq(&files),
        [
            "src/sheets/plugins/mod.rs:PluginsSheet",
            "src/sheets/plugins/mod.rs:Spread"
        ]
    );
}

// ---- 14. adr006_keycode_only_in_keymap ----

/// A `KeyCode::` or a keymap `Key::` variant in a page, a sheet or the app is a key the user cannot rebind (ADR-006): keys are
/// read through `Shortcuts` and `Actions` of the resolved keymap, `KeyCode` stays in `src/keymap/` and
/// in the primitives of `src/widgets/` (the picker, the key guard, the page keys).
fn keycode_outside_keymap(files: &Files) -> Vec<String> {
    hits_by_file(
        files,
        |name| {
            in_ui_sources(name)
                && !name.starts_with("src/keymap/")
                && !name.starts_with("src/widgets/")
        },
        |tokens, i| {
            starts(tokens, i, &["KeyCode", ":", ":"])
                // the keymap's own `Key` (`Key::Tab`, `Key::Char('r')`): a stroke spelled in a page is
                // a key the user cannot rebind either, a hardwired one is a named action of
                // `core::keymap::fixed` (`Key::value`, a widget key, is another type)
                || (starts(tokens, i, &["Key", ":", ":"])
                    && matches!(
                        at(tokens, i + 3),
                        "Char"
                            | "F"
                            | "Enter"
                            | "Tab"
                            | "BackTab"
                            | "Backspace"
                            | "Esc"
                            | "Delete"
                            | "Insert"
                            | "Up"
                            | "Down"
                            | "Left"
                            | "Right"
                            | "Home"
                            | "End"
                            | "PageUp"
                            | "PageDown"
                    )
                    && at(tokens, i.wrapping_sub(1)) != ":")
                // the copy of `KeyCode` the pages used to name keys by (`Fixed::Enter`,
                // `key.fixed()`): a hardwired key is a named action of `core::keymap::fixed`
                || starts(tokens, i, &["Fixed", ":", ":"])
                || starts(tokens, i, &[".", "fixed", "(", ")"])
                // a press written out (`KeyPress::char('h')`, `KeyPress::ESC`) and a key read from the
                // text it typed (`key.text() == Some("?")`): a hardwired key is a named action of
                // `core::keymap::fixed`, whose presses `src/keymap` builds
                || starts(tokens, i, &["KeyPress", ":", ":"])
                || starts(tokens, i, &[".", "text", "(", ")", "=", "=", "Some"])
                // a char bound (`focus.bind('k', ..)`): `cleaned` blanks a char literal to a space, so
                // `.bind(` followed at once by `,` is the shape of a char as the key
                || starts(tokens, i, &[".", "bind", "(", ","])
                // a character compared to the press's (`key.ch == Some('/')`, `!=` too) and the
                // modifiers read by hand (`key.mods == Mods::NONE`, `.mods.contains(Mods::CTRL)`):
                // a named fixed action (`pressed_fixed`) and the helpers of `src/keymap`
                // (`unmodified`, `types_text`, ..) say it
                || starts(tokens, i, &[".", "ch", "=", "=", "Some", "("])
                || starts(tokens, i, &[".", "ch", "!", "=", "Some", "("])
                || starts(tokens, i, &["mods", "=", "=", "Mods", ":", ":"])
                || starts(tokens, i, &["mods", "!", "=", "Mods", ":", ":"])
                || starts(tokens, i, &["mods", ".", "contains", "(", "Mods", ":", ":"])
                || starts(tokens, i, &["mods", ".", "0"])
        },
    )
}

#[test]
fn adr006_keycode_only_in_keymap() {
    check(
        "adr006_keycode_only_in_keymap",
        "keys are read through the resolved keymap (Shortcuts/Actions), so the user can rebind them; KeyCode stays in src/keymap and the widgets' primitives",
        &keycode_outside_keymap(&sources_of("src")),
    );
}

#[test]
fn the_keycode_scan_bites() {
    let files = vec![
        (
            "src/pages/agents/dialogs.rs".to_owned(),
            "match key.code { KeyCode::Enter => a(), KeyCode::Esc => b() }\n// KeyCode::Tab\nif key.fixed() == Fixed::Up {}\nlet give = Give::Fixed;\nlet h = KeyPress::char('h');\nfocus.bind(KeyPress::ESC, x);\nif key.text() == Some(\"?\") {}\nlet t = composer.text();\nlet n = key.text().is_empty();\nlet s = Stroke::new(Key::Char('r'), m);\nlet w = x.with_key(Key::value(1));\n// Key::Tab\nfocus.bind('k', y);\nfocus.bind(digit, y);\nfocus.bind(\"k\", y);\n// .bind('q', z)\nif key.ch == Some('/') {}\nif key.ch != Some('!') {}\nlet p = key.mods == Mods::NONE;\nlet q = key.mods != Mods::SHIFT;\nlet r = key.mods.contains(Mods::CTRL);\nlet t = key.mods.0 & 1;\nlet same = digit.ch == key.ch && key.mods == digit.mods;\n// key.mods == Mods::NONE\n"
                .to_owned(),
        ),
        (
            "src/keymap/mod.rs".to_owned(),
            "KeyCode::Enter\n".to_owned(),
        ),
        (
            "src/widgets/picker/mod.rs".to_owned(),
            "KeyCode::Up\n".to_owned(),
        ),
    ];
    assert_eq!(
        keycode_outside_keymap(&files),
        [
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs",
            "src/pages/agents/dialogs.rs"
        ]
    );
}

// ---- The scanner itself ----

#[test]
fn the_item_scanner_reads_what_rust_writes() {
    let text = "/// docs `struct Fake { x: u8 }`\n#[derive(Debug)]\npub struct A<T: Fn(u8) -> u8 = u8> where T: Clone {\n    #[allow(dead_code)]\n    pub(crate) one: Option<Vec<(B, C)>>,\n    two: [D; 3],\n    pub f: fn(E) -> F,\n}\npub struct Tuple(pub G, pub(crate) H);\nstruct Unit;\nenum E {\n    #[doc = \"a { b\"]\n    P,\n    Q(I, J),\n    R { k: K, l: Vec<L> },\n    S = 4,\n}\nlet c = '{'; let d = '\\''; let r = r#\"struct Raw { z: Z }\"#; let life: &'a str;\n";
    let defs = defs_of(&[("x.rs".to_owned(), text.to_owned())]);
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["A", "Tuple", "Unit", "E"]);
    let fields = |def: &Def| -> Vec<(String, Vec<String>)> {
        def.variants
            .iter()
            .flat_map(|v| v.fields.iter().map(|f| (f.name.clone(), f.ty.clone())))
            .collect()
    };
    let a = fields(&defs[0]);
    assert_eq!(
        a.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["one", "two", "f"]
    );
    assert_eq!(a[0].1, ["Option", "Vec", "B", "C"]);
    assert_eq!(fields(&defs[1]).len(), 2);
    assert!(defs[2].variants.is_empty());
    let variants: Vec<(&str, usize)> = defs[3]
        .variants
        .iter()
        .map(|v| (v.name.as_str(), v.fields.len()))
        .collect();
    assert_eq!(variants, [("P", 0), ("Q", 2), ("R", 2), ("S", 0)]);
}
