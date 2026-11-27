//! A fixture can show the IDE indicator without requesting context.

use super::{Fact, IdeReading};
use crate::screens::SeedError;

/// `{"kind":"ide_on","has_context":false}` shows the footer's indicator.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    IdeOn {
        #[serde(default)]
        has_context: bool,
    },
}

pub fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    let ev: SeedEv =
        serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))?;
    Ok(match ev {
        SeedEv::IdeOn { has_context } => Fact::IdeOn(IdeReading { has_context }),
    })
}
