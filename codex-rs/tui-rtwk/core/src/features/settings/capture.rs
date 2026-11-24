//! A shortcut captured in `/keymap` (oracle `keymap_setup.rs` `keymap_with_edit`,
//! `app/event_dispatch.rs` `apply_keymap_capture`): what the captured key does to the
//! action's bindings, and whether the keymap that follows still resolves (conflicts and the
//! reserved keys are the resolver's: `Keymap::resolve`). IO-free: the config it starts from is
//! the `KeymapSpec` the host read; the write is the feature's (`Edit::KeymapBound`).

use super::keymap;
use crate::keymap::{Keymap, KeymapSpec};

/// What the captured key is for (`KeymapEditIntent`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The key is the only binding.
    ReplaceAll,
    /// The key is one more binding.
    AddAlternate,
    /// The key takes the place of `old_key`, which must still be bound.
    ReplaceOne { old_key: String },
}

/// What the edit says of itself (`message`s of the oracle).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// `Remapped `{context}.{action}` to `{key}`.`
    Remapped,
    /// `Added `{key}` to `{context}.{action}`.`
    Added,
    /// `Replaced `{old_key}` with `{key}` for `{context}.{action}`.`
    Replaced { old_key: String },
    /// `No change: `{context}.{action}` already uses `{key}`.`
    Unchanged,
}

/// Why the capture cannot be applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// `Unknown keymap action `{context}.{action}`. Reopen /keymap and choose an action.`
    UnknownAction,
    /// `` `{context}.{action}` no longer uses `{old_key}`. Reopen /keymap and choose a binding
    /// again.``
    NoLongerUses { old_key: String },
    /// The keymap with the key does not resolve: the resolver's words (the conflict sheet).
    Conflict(String),
}

/// What the capture comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The action is bound to `keys` (the config's specs, in order) and `spec` is the config
    /// after the edit.
    Updated {
        spec: KeymapSpec,
        keys: Vec<String>,
        said: Said,
    },
    /// The action already has what the key would give it.
    Unchanged,
}

fn dedup(keys: Vec<String>) -> Vec<String> {
    keys.into_iter().fold(Vec::new(), |mut kept, key| {
        if !kept.contains(&key) {
            kept.push(key);
        }
        kept
    })
}

/// The bindings the action has now, as the config writes them: what the user set, else what
/// the keymap gives it.
fn active(spec: &KeymapSpec, keymap: &Keymap, context: &str, action: &str) -> Option<Vec<String>> {
    if let Some(keys) = spec.get(context, action) {
        return Some(dedup(keys.to_vec()));
    }
    keymap::active_specs(keymap, context, action)
}

/// Applies the key captured for `context.action` to the config `spec`.
pub fn captured(
    spec: &KeymapSpec,
    context: &str,
    action: &str,
    key: &str,
    intent: &Intent,
) -> Result<Outcome, Failure> {
    let keymap = Keymap::resolve(spec).unwrap_or_else(|_| Keymap::defaults());
    let current = active(spec, &keymap, context, action).ok_or(Failure::UnknownAction)?;
    let (next, said) = match intent {
        Intent::ReplaceAll => (vec![key.to_owned()], Said::Remapped),
        Intent::AddAlternate => {
            if current.iter().any(|bound| bound == key) {
                return Ok(Outcome::Unchanged);
            }
            let mut keys = current.clone();
            keys.push(key.to_owned());
            (keys, Said::Added)
        }
        Intent::ReplaceOne { old_key } => {
            if !current.iter().any(|bound| bound == old_key) {
                return Err(Failure::NoLongerUses {
                    old_key: old_key.clone(),
                });
            }
            let keys = current
                .iter()
                .map(|bound| if bound == old_key { key } else { bound })
                .map(str::to_owned)
                .collect();
            (
                dedup(keys),
                Said::Replaced {
                    old_key: old_key.clone(),
                },
            )
        }
    };
    if next == current {
        return Ok(Outcome::Unchanged);
    }
    let refs: Vec<&str> = next.iter().map(String::as_str).collect();
    let edited = spec.clone().with(context, action, &refs);
    Keymap::resolve(&edited).map_err(|error| Failure::Conflict(error.message()))?;
    Ok(Outcome::Updated {
        spec: edited,
        keys: next,
        said,
    })
}
