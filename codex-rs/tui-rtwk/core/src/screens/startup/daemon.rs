//! The daemon compatibility check of a launch, as its pure decisions (oracle `daemon_startup.rs`
//! `compatibility_warning` and `daemon_recovery.rs`): which of the feature settings this session
//! asked for the shared background server does not have. The words of the failure and of the
//! recovery menu are the host's (`host/src/daemon.rs`). The host reads the three requests (`config/read` with layers,
//! `configRequirements/read`, `experimentalFeature/list`), maps feature keys and stages to the
//! plain values used here, and asks these functions what is left.

use std::collections::BTreeMap;

/// One feature the server reports (`experimentalFeature/list`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reported {
    pub name: String,
    /// The stage is Removed: the feature is inert and never mismatches.
    pub removed: bool,
    pub enabled: bool,
}

/// What the server lacks of the settings asked: the first (by key) names the reason, all go
/// into `restart_features` (what a restart would set).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmet {
    pub name: String,
    pub enabled: bool,
    pub restart_features: Vec<(String, bool)>,
}

/// Drops the settings a legacy managed config layer sets: they override the command line
/// (`layers` is `config/read`'s `layers`; a layer the server disabled does not count).
pub fn drop_managed_layers(requested: &mut BTreeMap<String, bool>, layers: &[serde_json::Value]) {
    for layer in layers.iter().filter(|layer| {
        layer["disabledReason"].is_null()
            && matches!(
                layer["name"]["type"].as_str(),
                Some("legacyManagedConfigTomlFromFile" | "legacyManagedConfigTomlFromMdm")
            )
    }) {
        requested.retain(|name, _| layer["config"]["features"].get(name).is_none());
    }
}

/// Drops the settings the managed requirements fix (`featureRequirements`' names).
pub fn drop_required<'a>(
    requested: &mut BTreeMap<String, bool>,
    required: impl IntoIterator<Item = &'a String>,
) {
    let required: Vec<&String> = required.into_iter().collect();
    requested.retain(|name, _| !required.contains(&name));
}

/// What of `requested` the server's `features` do not have: a removed feature is inert and an
/// unreported one is taken as disabled.
pub fn unmet(mut requested: BTreeMap<String, bool>, features: &[Reported]) -> Result<(), Unmet> {
    requested.retain(
        |name, enabled| match features.iter().find(|feature| feature.name == *name) {
            Some(feature) => !feature.removed && feature.enabled != *enabled,
            None => *enabled,
        },
    );
    let Some((name, enabled)) = requested.first_key_value() else {
        return Ok(());
    };
    Err(Unmet {
        name: name.clone(),
        enabled: *enabled,
        restart_features: requested.into_iter().collect(),
    })
}
