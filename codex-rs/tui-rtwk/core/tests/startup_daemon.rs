#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The daemon compatibility decisions (oracle `daemon_startup_tests.rs`
//! `daemon_feature_compatibility_respects_required_and_optional_attachment`): the ten scenarios
//! run over the pure stages the host calls in the oracle's order (managed layers, requirements,
//! then the server's feature list). The `-c` flags a scenario passes arrive normalised, as the
//! host's `requested_features` hands them over (its own test covers the flags). The port's
//! daemon is a required attachment (`allow_embedded_fallback` false), so each scenario is a
//! mismatch or none; the unreachable one cannot happen (the host is connected already).

use std::collections::BTreeMap;

use codex_tui_core::screens::startup::daemon::{
    Reported, Unmet, drop_managed_layers, drop_required, unmet,
};
use serde_json::json;

/// The server's features of the oracle's table (`:112-120`); `api_key_model_discovery` is
/// enabled unless the scenario is "explicit enable".
fn features(scenario: &str) -> Vec<Reported> {
    let reported = |name: &str, enabled: bool, removed: bool| Reported {
        name: name.to_owned(),
        removed,
        enabled,
    };
    vec![
        reported(
            "api_key_model_discovery",
            scenario != "explicit enable",
            false,
        ),
        reported("code_mode_host", false, false),
        reported("instant_interrupt", true, false),
        reported("remote_compaction_v2", false, false),
        reported("shell_tool", false, true),
    ]
}

fn flags(pairs: &[(&str, bool)]) -> BTreeMap<String, bool> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

/// The stages in the host's order, the early exits included.
fn check(scenario: &str, requested: BTreeMap<String, bool>) -> Result<(), Unmet> {
    let mut requested = requested;
    let layers = [json!({
        "name": {"type": "legacyManagedConfigTomlFromFile"},
        "config": if scenario == "legacy managed" {
            json!({"features": {"api_key_model_discovery": true}})
        } else {
            json!({})
        },
    })];
    drop_managed_layers(&mut requested, &layers);
    if scenario == "managed" {
        let required = flags(&[("api_key_model_discovery", true)]);
        drop_required(&mut requested, required.keys());
    }
    if requested.is_empty() {
        return Ok(());
    }
    unmet(requested, &features(scenario))
}

#[test]
fn daemon_feature_compatibility_respects_required_and_optional_attachment() {
    let disabled = flags(&[("api_key_model_discovery", false)]);
    for (scenario, requested, reason) in [
        (
            "matching",
            // `remote_compaction_v2` (a removed feature) never reaches here: the host drops it;
            // `shell_tool` is reported Removed (inert) and `zz_missing=false` is the default.
            flags(&[
                ("instant_interrupt", true),
                ("shell_tool", true),
                ("zz_missing", false),
            ]),
            None,
        ),
        (
            "explicit enable",
            flags(&[("api_key_model_discovery", true)]),
            Some(("api_key_model_discovery", true)),
        ),
        (
            "conflicting client",
            disabled.clone(),
            Some(("api_key_model_discovery", false)),
        ),
        (
            "old requirements server",
            disabled.clone(),
            Some(("api_key_model_discovery", false)),
        ),
        (
            "missing enabled",
            flags(&[("zz_missing", true)]),
            Some(("zz_missing", true)),
        ),
        ("managed", disabled.clone(), None),
        ("legacy managed", disabled, None),
    ] {
        let result = check(scenario, requested);
        match reason {
            None => assert_eq!(result, Ok(()), "{scenario}"),
            Some((name, enabled)) => {
                let error = result.expect_err(scenario);
                assert_eq!(
                    (error.name.as_str(), error.enabled),
                    (name, enabled),
                    "{scenario}"
                );
            }
        }
    }
}

#[test]
fn mixed_flags_restart_with_what_the_server_lacks() {
    // `features.zz_overridden=true` is replaced by the table after it; the table's
    // `instant_interrupt=true` matches the server and `code_mode_host=true` (the last word)
    // does not (oracle `mixed flags`, `:218-226`).
    let requested = flags(&[
        ("api_key_model_discovery", false),
        ("code_mode_host", true),
        ("instant_interrupt", true),
    ]);
    let error = check("mixed flags", requested).expect_err("mismatch");
    assert_eq!(
        (error.name.as_str(), error.enabled),
        ("api_key_model_discovery", false)
    );
    assert_eq!(
        error.restart_features,
        [
            ("api_key_model_discovery".to_owned(), false),
            ("code_mode_host".to_owned(), true)
        ]
    );
}
