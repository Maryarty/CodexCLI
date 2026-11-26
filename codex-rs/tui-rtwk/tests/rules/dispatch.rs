//! The slash dispatcher has a budget of zero feature-owned command cases (C1).

use std::collections::{BTreeMap, BTreeSet};

use crate::adr013::{at, code_tokens, matching, starts};
use crate::layers::srcscan::{crate_dir, sources_of};

fn qualified_commands(tokens: &[String]) -> impl Iterator<Item = &str> {
    tokens.windows(4).filter_map(|words| {
        (words[0] == "SlashCommand" && words[1] == ":" && words[2] == ":")
            .then_some(words[3].as_str())
    })
}

fn claims(source: &str) -> BTreeSet<String> {
    let tokens = code_tokens(source);
    let mut claims = BTreeSet::new();
    for (i, token) in tokens.iter().enumerate() {
        if token == "const"
            && matches!(at(&tokens, i + 1), "RUNS" | "OPENS" | "COMMAND_RULES")
            && at(&tokens, i + 2) == ":"
        {
            let declaration = &tokens[i..];
            let end = declaration
                .iter()
                .position(|token| token == ";")
                .unwrap_or(declaration.len());
            claims.extend(qualified_commands(&declaration[..end]).map(str::to_owned));
        }
    }
    claims
}

fn owned_command_references(source: &str, owned: &BTreeSet<String>) -> BTreeSet<String> {
    let tokens = code_tokens(source);
    let mut found: BTreeSet<_> = qualified_commands(&tokens)
        .filter(|command| owned.contains(*command))
        .map(str::to_owned)
        .collect();
    for (i, token) in tokens.iter().enumerate() {
        if token != "fn" || at(&tokens, i + 1) == "(" {
            continue;
        }
        let Some(open) =
            (i + 2..tokens.len()).find(|&index| matches!(at(&tokens, index), "{" | ";"))
        else {
            continue;
        };
        if tokens[open] == ";" {
            continue;
        }
        // A function can use unqualified variants after `use SlashCommand::*`.
        // Variants of other types such as `Target::Usage` are different names.
        for index in open + 1..matching(&tokens, open) {
            if owned.contains(&tokens[index]) && !starts(&tokens, index - 2, &[":", ":"]) {
                found.insert(tokens[index].clone());
            }
        }
    }
    found
}

#[test]
fn the_dispatcher_has_no_feature_owned_command_cases() {
    let mut owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, source) in sources_of("core/src/features") {
        let Some((feature, _)) = path
            .strip_prefix("core/src/features/")
            .and_then(|path| path.split_once('/'))
        else {
            continue;
        };
        for command in claims(&source) {
            owners
                .entry(command)
                .or_default()
                .insert(feature.to_owned());
        }
    }
    let conflicts: Vec<_> = owners
        .iter()
        .filter(|(_, features)| features.len() > 1)
        .collect();
    assert!(
        conflicts.is_empty(),
        "commands claimed by more than one feature: {conflicts:?}"
    );
    let path = crate_dir().join("core/src/commands/dispatch.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let found = owned_command_references(&source, &owners.into_keys().collect());
    assert!(
        found.is_empty(),
        "feature-owned commands in dispatch.rs: {found:?}"
    );
}

#[test]
fn the_dispatch_ratchet_distinguishes_command_variants_from_other_names() {
    let owned = claims(
        "const OPENS: Option<_> = Some((SlashCommand::Usage, Sheet));
         const RUNS: &[_] = &[SlashCommand::Side];
         const COMMAND_RULES: &[_] = &[CommandRule::new(SlashCommand::Keymap)];
         // const RUNS: &[_] = &[SlashCommand::Resume];",
    );
    assert_eq!(
        owned,
        BTreeSet::from(["Keymap".into(), "Side".into(), "Usage".into()])
    );
    let source = "
        enum Target { Usage }
        fn dispatch(command: SlashCommand) {
            use SlashCommand::*;
            match command {
                Side | SlashCommand::Keymap => Target::Usage,
                _ => Target::Side,
            }
            // Side and SlashCommand::Usage are allowed in comments.
        }
    ";
    assert_eq!(
        owned_command_references(source, &owned),
        BTreeSet::from(["Keymap".into(), "Side".into()]),
    );
}
