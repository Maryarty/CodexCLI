//! The startup/import host asks use the app's request registrar. The two generations for an
//! opened consent question or import flow belong to the screens.

use std::fs;

const ONBOARDING: &str = "core/src/screens/onboarding.rs";
const GATEWAY: &str = "core/src/screens/onboarding/gateway.rs";
const STARTUP: &str = "core/src/screens/startup.rs";
const IMPORT: &str = "core/src/screens/import/mod.rs";

fn violations(path: &str, source: &str) -> Vec<&'static str> {
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    let retired: &[&str] = match path {
        ONBOARDING | GATEWAY => &["minted:", "minted+=", ".minted"],
        STARTUP => &["next_request"],
        IMPORT => &[
            "request:self.generation",
            "request==model.generation",
            "request!=model.generation",
            "self.generation+=1",
        ],
        _ => &[],
    };
    retired
        .iter()
        .copied()
        .chain([
            ".commands.push(AppCommand::Startup",
            ".commands.push(crate::app::Command::Import",
        ])
        .filter(|pattern| compact.contains(pattern))
        .collect()
}

#[test]
fn startup_and_import_do_not_reintroduce_screen_owned_request_ids() {
    let bad: Vec<_> = [ONBOARDING, GATEWAY, STARTUP, IMPORT]
        .iter()
        .flat_map(|path| {
            let source = fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));
            violations(path, &source)
                .into_iter()
                .map(|pattern| (*path, pattern))
        })
        .collect();
    assert!(bad.is_empty(), "retired request bookkeeping: {bad:?}");
}

#[test]
fn the_ratchet_catches_the_previous_shapes_and_spares_flow_generations() {
    assert_eq!(violations(ONBOARDING, "model.minted += 1;").len(), 2);
    assert_eq!(
        violations(GATEWAY, "let mut minted = model.minted;").len(),
        1
    );
    assert_eq!(violations(STARTUP, "model.next_request += 1;").len(), 1);
    assert_eq!(
        violations(
            IMPORT,
            "request: self.generation; fx.commands\n.push(crate::app::Command::Import);"
        )
        .len(),
        2
    );
    assert!(
        violations(
            IMPORT,
            "let generation = self.generation + 1; self.review_id += 1;"
        )
        .is_empty()
    );
}
