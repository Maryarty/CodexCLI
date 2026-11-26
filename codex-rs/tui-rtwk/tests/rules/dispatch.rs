//! The slash dispatcher has a budget of zero feature-owned command cases (C1).

use std::collections::BTreeSet;

use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::owns_command;

use crate::adr013::{at, code_tokens, matching, starts};
use crate::layers::srcscan::crate_dir;

fn dispatch_tokens(source: &str) -> Vec<String> {
    let mut tokens = code_tokens(source);
    // The shared lexer separates raw identifier prefixes from their names.
    for i in (0..tokens.len().saturating_sub(2)).rev() {
        if starts(&tokens, i, &["r", "#"]) {
            tokens.drain(i..i + 2);
        }
    }
    tokens
}

fn owned_command_names() -> BTreeSet<String> {
    SlashCommand::ALL
        .into_iter()
        .filter(|&command| owns_command(command))
        .map(|command| format!("{command:?}"))
        .collect()
}

fn qualified_commands(tokens: &[String]) -> Vec<&str> {
    let mut found = Vec::new();
    for (i, _) in tokens.iter().enumerate() {
        if !starts(tokens, i, &["SlashCommand", ":", ":"]) {
            continue;
        }
        let variant = i + 3;
        if at(tokens, variant) == "{" {
            let imports = &tokens[variant + 1..matching(tokens, variant)];
            found.extend(
                imports
                    .split(|token| token == ",")
                    .filter_map(|import| import.first().map(String::as_str)),
            );
        } else {
            found.push(at(tokens, variant));
        }
    }
    found
}

fn aliases_command_type(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(i, token)| {
        starts(tokens, i, &["SlashCommand", "as"])
            || (starts(tokens, i, &["SlashCommand", ":", ":", "{"])
                && tokens[i + 4..matching(tokens, i + 3)]
                    .windows(2)
                    .any(|import| import[0] == "self" && import[1] == "as"))
            || (token == "type"
                && at(tokens, i + 2) == "="
                && tokens[i + 3..]
                    .iter()
                    .take_while(|token| *token != ";")
                    .last()
                    .is_some_and(|token| token == "SlashCommand"))
    })
}

fn owned_command_references(tokens: &[String], owned: &BTreeSet<String>) -> BTreeSet<String> {
    let mut found: BTreeSet<_> = qualified_commands(tokens)
        .into_iter()
        .filter(|command| owned.contains(*command))
        .map(str::to_owned)
        .collect();
    for (i, token) in tokens.iter().enumerate() {
        if token != "fn" || at(tokens, i + 1) == "(" {
            continue;
        }
        let Some(open) =
            (i + 2..tokens.len()).find(|&index| matches!(at(tokens, index), "{" | ";"))
        else {
            continue;
        };
        if tokens[open] == ";" {
            continue;
        }
        // A function can use unqualified variants after `use SlashCommand::*`.
        // Variants of other types such as `Target::Usage` are different names.
        for index in open + 1..matching(tokens, open) {
            if owned.contains(&tokens[index]) && !starts(tokens, index - 2, &[":", ":"]) {
                found.insert(tokens[index].clone());
            }
        }
    }
    found
}

#[test]
fn the_dispatcher_has_no_feature_owned_command_cases() {
    let path = crate_dir().join("core/src/commands/dispatch.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let tokens = dispatch_tokens(&source);
    assert!(
        !aliases_command_type(&tokens),
        "use the name SlashCommand in the dispatcher so the command ratchet can read its variants"
    );
    let found = owned_command_references(&tokens, &owned_command_names());
    assert!(
        found.is_empty(),
        "feature-owned commands in dispatch.rs: {found:?}"
    );
}

#[test]
fn the_dispatch_ratchet_distinguishes_command_variants_from_other_names() {
    let owned = owned_command_names();
    assert!(
        ["Usage", "Side", "Keymap", "Mcp", "Recap"]
            .into_iter()
            .all(|name| owned.contains(name))
    );
    let source = "
        use super::SlashCommand::Recap as Summary;
        use SlashCommand::{Usage as Analytics};
        enum Target { Mcp }
        fn dispatch(command: SlashCommand) {
            use SlashCommand::*;
            match command {
                Side | SlashCommand::Keymap => Target::Mcp,
                _ => Target::Mcp,
            }
            // SlashCommand::Mcp is allowed in a comment.
        }
    ";
    assert_eq!(
        owned_command_references(&dispatch_tokens(source), &owned),
        BTreeSet::from([
            "Keymap".into(),
            "Recap".into(),
            "Side".into(),
            "Usage".into()
        ]),
    );
    assert!(aliases_command_type(&code_tokens(
        "use super::SlashCommand as Command;"
    )));
    assert!(aliases_command_type(&dispatch_tokens(
        "use super::r#SlashCommand::{New, self as Command}; fn dispatch() { Command::Usage; }"
    )));
    assert!(aliases_command_type(&code_tokens(
        "type Cmd = crate::commands::SlashCommand;"
    )));
    assert!(!aliases_command_type(&code_tokens(
        "type Callback = fn(SlashCommand);"
    )));
    assert_eq!(
        owned_command_references(
            &dispatch_tokens("const CMD: SlashCommand = SlashCommand::r#Mcp;"),
            &owned
        ),
        BTreeSet::from(["Mcp".into()]),
    );
}
