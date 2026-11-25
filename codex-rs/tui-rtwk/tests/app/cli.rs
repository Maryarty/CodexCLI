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
fn oss_and_local_provider_parse() {
    let plain = parse(&[]).expect("parsed");
    assert!(!plain.oss);
    assert_eq!(plain.oss_provider, None);
    let oss = parse(&["--oss"]).expect("parsed");
    assert!(oss.oss);
    assert_eq!(oss.oss_provider, None);
    for (args, want) in [
        (["--oss", "--local-provider", "ollama"].as_slice(), "ollama"),
        (
            ["--local-provider=lmstudio", "--oss"].as_slice(),
            "lmstudio",
        ),
    ] {
        let cli = parse(args).expect("parsed");
        assert_eq!(cli.oss_provider.as_deref(), Some(want), "{args:?}");
    }
}

#[test]
fn local_provider_needs_a_value() {
    assert!(parse(&["--local-provider"]).is_err());
}

#[test]
fn early_answer_skips_the_local_provider_value() {
    // The value of the flag is not a word of the command line: `help` after
    // it is a provider name, not the help subcommand.
    assert_eq!(answer(&["--local-provider", "help"]), None);
    assert!(answer(&["--local-provider", "ollama", "--help"]).is_some());
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
    Cli::early_answer(args.iter().map(ToString::to_string)).map(str::to_owned)
}

#[test]
fn version_prints_the_oracles_line() {
    let want = include_str!("../cli_help/version.txt");
    assert_eq!(answer(&["--version"]).as_deref(), Some(want));
    assert_eq!(answer(&["-V"]).as_deref(), Some(want));
}

#[test]
fn the_top_level_help_long_and_short() {
    let long = include_str!("../cli_help/top_long.txt");
    let short = include_str!("../cli_help/top_short.txt");
    assert_eq!(answer(&["--help"]).as_deref(), Some(long));
    assert_eq!(answer(&["help"]).as_deref(), Some(long));
    assert_eq!(answer(&["-h"]).as_deref(), Some(short));
}

#[test]
fn the_agents_help_long_and_short() {
    let long = include_str!("../cli_help/agents_long.txt");
    let short = include_str!("../cli_help/agents_short.txt");
    assert_eq!(answer(&["agents", "--help"]).as_deref(), Some(long));
    assert_eq!(answer(&["help", "agents"]).as_deref(), Some(long));
    assert_eq!(answer(&["agents", "-h"]).as_deref(), Some(short));
}

/// Help wins over the other flags that come before it, as in clap.
#[test]
fn help_is_answered_whatever_comes_before() {
    let long = include_str!("../cli_help/agents_long.txt");
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

/// Item 7: `-C` / `--cd` and `--no-alt-screen` are the flags of the oracle's
/// `codex agents` (`tests/cli_help/agents_long.txt`).
#[test]
fn the_cd_and_inline_flags_are_read() {
    for args in [
        ["agents", "-C", "/tmp/x"].as_slice(),
        ["agents", "--cd", "/tmp/x"].as_slice(),
        ["agents", "--cd=/tmp/x"].as_slice(),
    ] {
        let cli = parse(args).expect("parsed");
        assert_eq!(
            cli.cwd.as_deref(),
            Some(std::path::Path::new("/tmp/x")),
            "{args:?}"
        );
        assert!(cli.agents);
    }
    assert!(parse(&["--no-alt-screen"]).expect("parsed").no_alt_screen);
    assert!(!parse(&[]).expect("parsed").no_alt_screen);
    assert!(parse(&["-C"]).is_err());
}

/// `--no-daemon` and `--remote` are the connection flags (agents-daemon); the
/// two exclude each other with the oracle's words
/// (`startup_orchestration.rs:16-19`), the values reach the host.
#[test]
fn the_connection_flags_answer_as_the_oracle_does() {
    assert!(parse(&["agents", "--no-daemon"]).expect("parsed").no_daemon);
    let both = parse(&["--no-daemon", "--remote", "ws://h:1"]).unwrap_err();
    assert_eq!(
        both.to_string(),
        "--no-daemon cannot be used with --remote."
    );
    let remote = parse(&[
        "agents",
        "--remote=ws://h:1",
        "--remote-auth-token-env",
        "T",
    ])
    .expect("parsed");
    assert_eq!(remote.remote.as_deref(), Some("ws://h:1"));
    assert_eq!(remote.remote_auth_token_env.as_deref(), Some("T"));
    let spaced = parse(&["--remote", "unix://"]).expect("parsed");
    assert_eq!(spaced.remote.as_deref(), Some("unix://"));
    assert!(parse(&["--remote"]).is_err());
}

/// Flag values are not words of the command line: `--help` after `-C dir`
/// still answers, and `-C agents` is a directory named agents.
#[test]
fn a_directory_named_like_a_subcommand_is_a_value() {
    assert_eq!(
        Cli::early_answer(["-C".to_owned(), "agents".to_owned(), "-h".to_owned()]),
        Cli::early_answer(["-h".to_owned()])
    );
}

/// `-c key=value`, `--config key=value`, `--config=key=value`: collected raw,
/// in order (the oracle's `CliConfigOverrides`); a value is needed.
#[test]
fn config_overrides_are_collected_in_order() {
    let cli = parse(&[
        "-c",
        "analytics.enabled=false",
        "agents",
        "--config",
        "model=\"x\"",
        "--config=tui.animations=false",
    ])
    .expect("parsed");
    assert!(cli.agents);
    assert_eq!(
        cli.config_overrides,
        [
            "analytics.enabled=false",
            "model=\"x\"",
            "tui.animations=false"
        ]
    );
    assert!(parse(&["-c"]).is_err());
    assert!(parse(&["--config"]).is_err());
    assert_eq!(
        Cli::early_answer(["-c", "--help"].map(String::from)),
        None,
        "the value of -c is not a word of the command line"
    );
}

/// `cli/src/main.rs:1088-1096, 1136`: `codex agents` takes no initial prompt
/// or images, and two different `--remote` endpoints are a conflict; the
/// oracle's words.
#[test]
fn agents_refuses_a_prompt_images_and_conflicting_endpoints() {
    let prompt = "`codex agents` does not accept an initial prompt or images";
    assert_eq!(
        parse(&["agents", "fix the bug"]).unwrap_err().to_string(),
        prompt
    );
    assert_eq!(
        parse(&["agents", "-i", "a.png"]).unwrap_err().to_string(),
        prompt
    );
    assert_eq!(
        parse(&["agents", "--image=a.png"]).unwrap_err().to_string(),
        prompt
    );
    assert_eq!(
        parse(&["--remote", "ws://h:1", "agents", "--remote", "ws://h:2"])
            .unwrap_err()
            .to_string(),
        "`codex agents` received conflicting remote server endpoints"
    );
    // The same endpoint twice is no conflict; a prompt without `agents` is the
    // startup draft's initial prompt (f-startup-draft), as the oracle's `codex <prompt>`.
    assert!(parse(&["--remote", "ws://h:1", "agents", "--remote", "ws://h:1"]).is_ok());
    assert!(parse(&["fix the bug"]).is_ok());
}
