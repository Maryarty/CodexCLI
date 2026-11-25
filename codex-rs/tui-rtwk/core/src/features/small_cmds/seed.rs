//! The seed forms of the feature's facts, for the scripts' `fact` step (`SmallCmds::SEED`): what
//! the IDE answers, and `/ide` on with an IDE that answers.

use super::{Fact, IdeReading};
use crate::screens::SeedError;

/// `{"kind":"ide","has_context":true}` is an IDE that answers (`/ide` still off),
/// `{"kind":"ide_on","has_context":false}` is `/ide` on with it (the footer's `IDE context`
/// indicator, no note), `{"kind":"ide_unavailable","hint":"..","skip_hint":".."}` is none
/// answering.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Ide {
        #[serde(default)]
        has_context: bool,
    },
    IdeOn {
        #[serde(default)]
        has_context: bool,
    },
    IdeUnavailable {
        hint: String,
        #[serde(default)]
        skip_hint: String,
    },
}

pub fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    let ev: SeedEv =
        serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))?;
    Ok(match ev {
        SeedEv::Ide { has_context } => Fact::ide_context(has_context),
        SeedEv::IdeOn { has_context } => Fact::IdeOn(IdeReading { has_context }),
        SeedEv::IdeUnavailable { hint, skip_hint } => Fact::ide_unavailable(hint, skip_hint),
    })
}
