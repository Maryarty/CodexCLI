//! The command line: `codex-rtwk [--debug-screens] [--locale L] [-m MODEL]`.

#![allow(clippy::expect_used)]

use codex_tui_rtwk::Cli;

fn parse(args: &[&str]) -> anyhow::Result<Cli> {
    Cli::parse(args.iter().map(ToString::to_string))
}

#[test]
fn the_model_flag_in_its_three_forms() {
    for args in [
        ["-m", "gpt-x"].as_slice(),
        ["--model", "gpt-x"].as_slice(),
        ["--model=gpt-x"].as_slice(),
    ] {
        let cli = parse(args).expect("parsed");
        assert_eq!(cli.model.as_deref(), Some("gpt-x"), "{args:?}");
    }
    assert_eq!(parse(&[]).expect("parsed").model, None);
}

#[test]
fn the_model_flag_needs_a_value() {
    assert!(parse(&["-m"]).is_err());
    assert!(parse(&["--model"]).is_err());
}

#[test]
fn an_unknown_argument_is_an_error() {
    assert!(parse(&["--nope"]).is_err());
}

#[test]
fn flags_combine() {
    let cli = parse(&["--debug-screens", "--locale=ru", "-m", "m"]).expect("parsed");
    assert!(cli.debug_screens);
    assert!(cli.locale.is_some());
    assert_eq!(cli.model.as_deref(), Some("m"));
}

// `--help` and `--version` answer as the oracle's `codex` does (its output,
// captured from the shipped binary, 0.170.0, in `tests/cli_help/`): the
// top level and the `agents` subcommand, long and short.

fn answer(args: &[&str]) -> Option<String> {
    Cli::early_answer(args.iter().map(ToString::to_string))
}

#[test]
fn version_prints_the_oracles_line() {
    let want = include_str!("cli_help/version.txt");
    assert_eq!(answer(&["--version"]).as_deref(), Some(want));
    assert_eq!(answer(&["-V"]).as_deref(), Some(want));
}

#[test]
fn the_top_level_help_long_and_short() {
    let long = include_str!("cli_help/top_long.txt");
    let short = include_str!("cli_help/top_short.txt");
    assert_eq!(answer(&["--help"]).as_deref(), Some(long));
    assert_eq!(answer(&["help"]).as_deref(), Some(long));
    assert_eq!(answer(&["-h"]).as_deref(), Some(short));
}

#[test]
fn the_agents_help_long_and_short() {
    let long = include_str!("cli_help/agents_long.txt");
    let short = include_str!("cli_help/agents_short.txt");
    assert_eq!(answer(&["agents", "--help"]).as_deref(), Some(long));
    assert_eq!(answer(&["help", "agents"]).as_deref(), Some(long));
    assert_eq!(answer(&["agents", "-h"]).as_deref(), Some(short));
}

/// Help wins over the other flags that come before it, as in clap.
#[test]
fn help_is_answered_whatever_comes_before() {
    let long = include_str!("cli_help/agents_long.txt");
    assert_eq!(
        answer(&["--locale", "ru", "agents", "--help"]).as_deref(),
        Some(long)
    );
}

#[test]
fn a_plain_command_line_has_no_early_answer() {
    assert_eq!(answer(&[]), None);
    assert_eq!(answer(&["agents"]), None);
    assert_eq!(answer(&["-m", "x"]), None);
    assert_eq!(answer(&["--debug-screens"]), None);
}

/// `agents --version` is an error in the oracle (the subcommand has none),
/// not the version line.
#[test]
fn the_version_flag_of_a_subcommand_is_not_an_answer() {
    assert_eq!(answer(&["agents", "--version"]), None);
}
