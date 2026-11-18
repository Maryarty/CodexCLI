//! The command line: `codex-rtwk [--debug-screens] [--locale L] [-m MODEL]`.

#![allow(clippy::expect_used)]

use codex_tui_rtwk::Cli;

fn parse(args: &[&str]) -> anyhow::Result<Cli> {
    Cli::parse(args.iter().map(|arg| arg.to_string()))
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
