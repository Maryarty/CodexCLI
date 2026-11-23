//! The seed forms of the feature's facts, for the scripts' `fact` step (`Settings::SEED`).

use std::collections::BTreeMap;

use super::catalog::{
    CatalogueItem, ConfigPanel, Experimental, Facts, Feature, FeaturesStatus, KeymapPicker,
    KeymapTab, Memories, MemoryFacts, Row, Setup, SetupItem, SetupKind, ThemePicker, setup,
};
use super::hooks::{Hook, HookIssue, Hooks};
use super::{Fact, Offer};
use crate::screens::SeedError;

/// The seed forms of the feature's facts (`kind`): `config_panel` with its
/// `rows`; `theme_picker` with `themes` and the `current` one;
/// `status_line` with `use_theme_colors`, the `selected` ids in order, the
/// `catalogue` and the runtime `values`; `title` the same without the colour
/// preference; `keymap` with its `tabs`; `memories` with its two switches;
/// `facts` with `memories`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    ConfigPanel {
        rows: Vec<Row>,
    },
    ThemePicker {
        themes: Vec<String>,
        #[serde(default)]
        custom: Vec<String>,
        #[serde(default)]
        current: Option<String>,
        #[serde(default)]
        themes_dir: Option<String>,
    },
    StatusLine {
        use_theme_colors: bool,
        selected: Vec<String>,
        catalogue: Vec<CatalogueItem>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Title {
        selected: Vec<String>,
        catalogue: Vec<CatalogueItem>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Keymap {
        tabs: Vec<KeymapTab>,
    },
    Skills {
        items: Vec<SkillSeed>,
    },
    Experimental {
        features: Vec<Feature>,
        #[serde(default)]
        loading: bool,
        /// Keys whose write did not confirm.
        #[serde(default)]
        unconfirmed: Vec<String>,
        /// What the failed write answered.
        #[serde(default)]
        failed: Option<String>,
        #[serde(default)]
        generation: u64,
    },
    FeaturesDiscovered {
        #[serde(default)]
        generation: u64,
        #[serde(default)]
        features: Option<Vec<Feature>>,
    },
    Memories {
        use_memories: bool,
        generate_memories: bool,
    },
    Hooks {
        hooks: Vec<Hook>,
        #[serde(default)]
        warnings: Vec<String>,
        #[serde(default)]
        errors: Vec<HookIssue>,
    },
    /// The config's values the dialogs open from (`Fact::Facts`).
    Facts {
        #[serde(default)]
        memories: Option<MemoryFacts>,
    },
}

/// One skill of the toggle (`SkillsToggleItem`).
#[derive(Clone, Debug, serde::Deserialize)]
struct SkillSeed {
    /// The skill's own name (`skill_name`).
    id: String,
    /// Its display name.
    name: String,
    description: String,
    enabled: bool,
}

/// A script's `fact` step for the feature: the fact `value` makes.
pub fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    let seed: SeedEv =
        serde_json::from_value(value).map_err(|err| SeedError::Bad(err.to_string()))?;
    Ok(Fact::Offered(match seed {
        SeedEv::FeaturesDiscovered {
            generation,
            features,
        } => {
            return Ok(Fact::FeaturesDiscovered {
                generation,
                features,
            });
        }
        SeedEv::Facts { memories } => {
            return Ok(Fact::Facts(Facts {
                memories,
                ..Facts::default()
            }));
        }
        SeedEv::Hooks {
            hooks,
            warnings,
            errors,
        } => Offer::Hooks(Hooks::new(hooks, warnings, errors)),
        SeedEv::Memories {
            use_memories,
            generate_memories,
        } => Offer::Memories(Memories {
            use_memories,
            generate_memories,
        }),
        SeedEv::Experimental {
            features,
            loading,
            unconfirmed,
            failed,
            generation,
        } => {
            let mut dialog = Experimental::new(features, loading, generation);
            if let Some(error) = failed {
                dialog.status = FeaturesStatus::Message(error);
            }
            // A write that failed: the features show what the user flipped them to, the config
            // has the opposite (the sheet's draft starts with these keys flipped).
            if !unconfirmed.is_empty() {
                for feature in &mut dialog.features {
                    if unconfirmed.contains(&feature.key) {
                        feature.enabled = !feature.enabled;
                    }
                }
                dialog.unconfirmed = unconfirmed;
            }
            Offer::Experimental(dialog)
        }
        SeedEv::ConfigPanel { rows } => Offer::Config(ConfigPanel::new(rows)),
        SeedEv::ThemePicker {
            themes,
            custom,
            current,
            themes_dir,
        } => Offer::Theme(ThemePicker {
            themes,
            custom,
            original: current,
            themes_dir,
        }),
        SeedEv::StatusLine {
            use_theme_colors,
            selected,
            catalogue,
            values,
        } => Offer::Setup(setup(
            SetupKind::StatusLine,
            Some(use_theme_colors),
            &selected,
            catalogue,
            values,
        )),
        SeedEv::Title {
            selected,
            catalogue,
            values,
        } => Offer::Setup(setup(SetupKind::Title, None, &selected, catalogue, values)),
        SeedEv::Skills { items } => Offer::Setup(Setup {
            kind: SetupKind::Skills,
            items: items
                .into_iter()
                .map(|skill| SetupItem {
                    id: skill.id,
                    label: skill.name,
                    description: skill.description,
                    enabled: skill.enabled,
                    orderable: false,
                    section_break_after: false,
                    placeholder: String::new(),
                })
                .collect(),
            values: BTreeMap::new(),
        }),
        SeedEv::Keymap { tabs } => {
            if tabs.is_empty() {
                return Err(SeedError::Bad("no tabs".into()));
            }
            Offer::Keymap(KeymapPicker { tabs })
        }
    }))
}
