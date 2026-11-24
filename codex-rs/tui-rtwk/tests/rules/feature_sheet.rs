//! `FeatureSheet` (fw-ui-reg-2): the second half of a feature's view, `args` and `open`, exists
//! only for the slices a slash command opens a sheet of (`Feature::OPENS` of core). A slice
//! without a sheet writes no stub (`type Args = Never<Self>`, `.absurd()`): `FeatureTarget` of it
//! is uninhabited, so `sheet_list!` has no line for it and no match an arm.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// the lists that `build.rs` writes from the files: the rules read the same text (fw-hubs-c5)
use crate::layers::registry;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(crate_dir().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `src/features/mod.rs` as the compiler sees it: the hub, then the `pub mod` lines and
/// `sheet_list!` that `build.rs` writes from core's slices.
fn features_mod() -> String {
    let slices = registry::core_features(&crate_dir().join("core/src/features"))
        .unwrap_or_else(|e| panic!("core features: {e}"));
    let generated = registry::ui_features_text(&crate_dir().join("src/features"), &slices)
        .unwrap_or_else(|e| panic!("ui features: {e}"));
    registry::with_generated(&read("src/features/mod.rs"), &generated)
}

fn dirs_of(rel: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(crate_dir().join(rel))
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

/// The slices of core with `const OPENS: ... = Some(..)` (a default `None` is no declaration).
fn core_opens() -> BTreeSet<String> {
    dirs_of("core/src/features")
        .into_iter()
        .filter(|dir| {
            let text = read(&format!("core/src/features/{dir}/mod.rs"));
            text.find("const OPENS:").is_some_and(|at| {
                let item: String = text[at..]
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                item.contains("= Some(")
            })
        })
        .collect()
}

/// The lines `name => Variant;` of `macro_rules! sheet_list` in `src/features/mod.rs`.
fn sheet_list(text: &str) -> BTreeSet<String> {
    let start = text
        .find("macro_rules! sheet_list {")
        .expect("`macro_rules! sheet_list` in src/features/mod.rs");
    let body = &text[start..];
    let end = body.find("\n}\n").expect("the end of sheet_list");
    body[..end]
        .lines()
        .filter_map(|line| line.trim().strip_suffix(';'))
        .filter_map(|line| line.split_once(" => "))
        .map(|(name, _)| name.trim().to_owned())
        .collect()
}

/// The slices of the UI crate with an `impl FeatureSheet for`.
fn sheet_impls() -> BTreeSet<String> {
    dirs_of("src/features")
        .into_iter()
        .filter(|dir| {
            read(&format!("src/features/{dir}/mod.rs")).contains("\nimpl FeatureSheet for ")
        })
        .collect()
}

fn differences(
    opens: &BTreeSet<String>,
    listed: &BTreeSet<String>,
    impls: &BTreeSet<String>,
) -> Vec<String> {
    let mut fails = Vec::new();
    for name in opens.difference(listed) {
        fails.push(format!(
            "{name}: core declares `Feature::OPENS` but `sheet_list!` has no line for it"
        ));
    }
    for name in listed.difference(opens) {
        fails.push(format!(
            "{name}: in `sheet_list!` but core declares no `Feature::OPENS` for it"
        ));
    }
    for name in opens.difference(impls) {
        fails.push(format!(
            "{name}: `Feature::OPENS` without an `impl FeatureSheet` in src/features/{name}/"
        ));
    }
    for name in impls.difference(opens) {
        fails.push(format!("{name}: an `impl FeatureSheet` for a slice that declares no `Feature::OPENS` (a stub: delete it)"));
    }
    fails
}

#[test]
fn feature_sheet_only_with_opens() {
    let text = features_mod();
    let mut fails = differences(&core_opens(), &sheet_list(&text), &sheet_impls());
    // the stubs are gone, and the view trait has no sheet items of its own
    let view = text
        .split("pub trait FeatureView")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("`pub trait FeatureView` in src/features/mod.rs");
    for needle in ["type Args", "fn args(", "fn open<"] {
        if view.contains(needle) {
            fails.push(format!(
                "`FeatureView` has `{needle}`: it belongs to `FeatureSheet`"
            ));
        }
    }
    for dir in dirs_of("src/features") {
        let text = read(&format!("src/features/{dir}/mod.rs"));
        for (n, line) in text.lines().enumerate() {
            if line.contains("type Args = Never<")
                || line.contains("args.absurd()")
                || line.contains("target.absurd()")
            {
                fails.push(format!(
                    "src/features/{dir}/mod.rs:{}: a sheet stub for a slice without a sheet",
                    n + 1
                ));
            }
        }
    }
    assert!(
        fails.is_empty(),
        "FeatureSheet only with OPENS:\n{}",
        fails.join("\n")
    );
}

#[test]
fn the_feature_sheet_check_bites() {
    let set = |names: &[&str]| {
        names
            .iter()
            .map(|n| (*n).to_owned())
            .collect::<BTreeSet<_>>()
    };
    assert!(differences(&set(&["diff"]), &set(&["diff"]), &set(&["diff"])).is_empty());
    assert_eq!(
        differences(&set(&["diff", "export"]), &set(&["diff"]), &set(&["diff"])).len(),
        2
    );
    assert_eq!(
        differences(&set(&["diff"]), &set(&["diff"]), &set(&["diff", "goal"])).len(),
        1
    );
    assert_eq!(
        differences(&set(&["diff"]), &set(&["diff", "goal"]), &set(&["diff"])).len(),
        1
    );
    let real = sheet_list(&features_mod());
    assert!(real.contains("export"), "{real:?}");
}
