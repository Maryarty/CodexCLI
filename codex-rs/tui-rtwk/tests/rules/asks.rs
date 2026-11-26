//! The startup/import host asks use the app's request registrar. The two generations for an
//! opened consent question or import flow belong to the screens.

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
    let bad: Vec<_> = [
        (
            ONBOARDING,
            include_str!("../../core/src/screens/onboarding.rs"),
        ),
        (
            GATEWAY,
            include_str!("../../core/src/screens/onboarding/gateway.rs"),
        ),
        (STARTUP, include_str!("../../core/src/screens/startup.rs")),
        (IMPORT, include_str!("../../core/src/screens/import/mod.rs")),
    ]
    .into_iter()
    .flat_map(|(path, source)| {
        violations(path, source)
            .into_iter()
            .map(move |pattern| (path, pattern))
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

const RETIRED_HOST_CHANNELS: &[&str] = &[
    "BindWorktree",
    "CreateWorktree",
    "Directories",
    "Import",
    "OpenUrl",
    "Popovers",
    "ReconnectIdleThread",
    "SaveGrouping",
    "Startup",
    "Usage",
    "VerifyUser",
];

fn legacy_host_channels(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.trim_start().split_once([' ', '(', '{', ',']))
        .map(|(name, _)| name)
        .filter(|name| RETIRED_HOST_CHANNELS.contains(name))
        .collect()
}

#[test]
fn host_only_work_uses_the_effect_channel() {
    let app = include_str!("../../core/src/app.rs");
    let Some((_, command)) = app.split_once("pub enum Command {") else {
        panic!("Command");
    };
    let Some((body, _)) = command.split_once("impl Command") else {
        panic!("Command body");
    };
    let retired = legacy_host_channels(body);
    assert!(
        retired.is_empty(),
        "retired host channels on Command: {retired:?}"
    );

    let protocol = include_str!("../../core/src/protocol/mod.rs");
    let Some((requests, _)) = protocol.split_once("pub fn reply_from_server") else {
        panic!("command and reply routers");
    };
    let compact: String = requests.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.replace("=>{None}", "=>None,");
    let no_request =
        "Command::RejectRequest{..}|Command::ResolveRequest{..}|Command::Effect{..}=>None,";
    assert_eq!(compact.matches(no_request).count(), 2);
    assert_eq!(compact.matches("=>None").count(), 2);
}

#[test]
fn the_host_channel_ratchet_bites_on_legacy_commands() {
    assert_eq!(
        legacy_host_channels(
            "    Startup { ask: Ask },
    Usage(Ask),"
        ),
        ["Startup", "Usage"]
    );
    assert!(legacy_host_channels("    Effect { request: u64, ask: Ask },").is_empty());
}
