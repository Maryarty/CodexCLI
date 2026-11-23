//! The Daybreak setting a refused cybersecurity request is worded by (oracle `daybreak.rs`
//! `notice_for_setting`, `chatwidget/turn_runtime.rs` `cyber_policy_notice`): the account's
//! eligibility and the model catalog's access programs are app facts the host reads; the thread
//! says whether its own setting is on. Pending or failed discovery never implies access, so the
//! default fact is "not eligible, no catalog" and words as `Apply`.

use crate::screens::cells::Daybreak;

/// The cyber access programs one model advertises (`ModelAccessPrograms::cyber`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Programs {
    /// `CyberAccessProgram::Standard` is among them.
    pub standard: bool,
    /// `DaybreakBlue` or `DaybreakRed` is among them (`ModelAccessPrograms::daybreak`).
    pub daybreak: bool,
}

/// One entry of the model catalog, as far as Daybreak reads it: `programs` is `None` for a
/// model whose metadata says nothing, which is not the same as an empty list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAccess {
    pub model: String,
    pub programs: Option<Programs>,
}

/// What the host knows about the account and the catalog (`AppEv::Daybreak`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Access {
    /// The feature is on, the provider supports cyber access programs, and the account is a
    /// ChatGPT sign-in or an API key (oracle `daybreak_account_eligible`).
    pub eligible: bool,
    /// The model catalog (`model/list`).
    pub models: Vec<ModelAccess>,
}

/// The models whose Standard-only listing the oracle words as `Astra`.
const ASTRA: [&str; 2] = ["gpt-6-astra", "gpt-6-astra-wm"];

impl Access {
    /// Oracle `notice_for_setting` with `can_enable_daybreak = self.eligible`, for a thread on
    /// `model` whose own setting is `enabled`.
    pub fn notice(&self, model: Option<&str>, enabled: bool) -> Daybreak {
        if !self.eligible {
            return Daybreak::Apply;
        }
        if enabled {
            return Daybreak::Enabled;
        }
        let programs = model.and_then(|model| {
            self.models
                .iter()
                .find(|entry| entry.model == model)
                .and_then(|entry| entry.programs)
        });
        if model.is_some_and(|model| ASTRA.contains(&model))
            && programs.is_some_and(|programs| programs.standard && !programs.daybreak)
        {
            return Daybreak::Astra;
        }
        if programs.is_some_and(|programs| programs.daybreak) {
            return Daybreak::Disabled;
        }
        Daybreak::Apply
    }
}
