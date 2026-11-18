//! The pty scenarios: `codex` (the oracle) and `codex-rtwk` in a pseudo-terminal
//! against the same mocked model, compared at named checkpoints
//! (ADR-007 §2, research/test-infra.md §D3). The scenario tests are opt-in
//! (`#[ignore]`); `tests/scenarios/README.md` has the command. The unit tests
//! of the harness itself (`unit.rs`) run with the normal suite.
#![allow(clippy::disallowed_methods)]

#[path = "../support/mod.rs"]
mod support;

mod unit;

use std::path::Path;

use support::backend;
use support::run::{Outcome, Side, SideRun, report_dir, run_both, run_side};
use support::script::{Scenario, parse};

fn load(name: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/scenarios")
        .join(format!("{name}.jsonl"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The oracle alone: it reaches every checkpoint of the script and exits
/// cleanly. Runs wherever the oracle binary is; a scenario's script is valid
/// when this passes.
fn oracle_alone(name: &str) -> SideRun {
    let scn = load(name);
    let exe = backend::binary("oracle").unwrap_or_else(|e| panic!("{e}"));
    let run = run_side(&scn, Side::Oracle, &exe).unwrap_or_else(|e| panic!("{e}"));
    assert!(run.notes.is_empty(), "oracle notes: {:#?}\nscreens in {}", run.notes, report_dir(name).display());
    assert!(run.ended_early.is_none(), "the oracle ended early: {:?}", run.ended_early);
    run
}

/// Both binaries, compared at every checkpoint. Gaps the script names are
/// printed and pass, unless `SCENARIO_STRICT=1` (the gate of a slice that
/// claims parity).
fn parity(name: &str) {
    let scn = load(name);
    match run_both(&scn).unwrap_or_else(|e| panic!("{e}")) {
        Outcome::Skipped(why) => eprintln!("SKIPPED {name}: {why}"),
        Outcome::Compared(report) => {
            for gap in report.expected_gaps() {
                eprintln!("EXPECTED GAP {name} {gap}");
            }
            let strict = std::env::var_os("SCENARIO_STRICT").is_some_and(|v| v != "0");
            let failures = report.failures(strict);
            assert!(
                failures.is_empty(),
                "{name}: oracle and codex-rtwk differ:\n{}\nboth screens in {}",
                failures.join("\n"),
                report_dir(name).display()
            );
        }
    }
}

#[test]
#[ignore = "pty scenario: run with `~/.cache/rtwk/acargo nextest run --run-ignored only --test scenarios`"]
fn text_100_oracle() {
    let run = oracle_alone("text-100");
    assert_eq!(run.exit_code, Some(0));
}

#[test]
#[ignore = "pty scenario: run with `~/.cache/rtwk/acargo nextest run --run-ignored only --test scenarios`"]
fn text_100_parity() {
    parity("text-100");
}
