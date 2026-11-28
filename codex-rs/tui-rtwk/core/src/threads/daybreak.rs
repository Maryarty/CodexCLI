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

/// What the host knows about the account, the providers and the catalog (`AppEv::Daybreak`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Access {
    /// The feature is on and the account is a ChatGPT sign-in or an API key (the account half of
    /// oracle `daybreak_account_eligible`; the provider half is `providers`).
    pub account: bool,
    /// The configured providers that support cyber access programs
    /// (`supports_cyber_access_programs`). A thread is eligible by the provider it runs on, which
    /// a `thread/started` or a `thread/settings/updated` names, not by the config's own.
    pub providers: Vec<String>,
    /// The model catalog (`model/list`).
    pub models: Vec<ModelAccess>,
}

/// What the `/daybreak` entry offers (oracle `daybreak_command_description`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// The thread's setting is on: turn it off.
    Disable,
    /// The catalog advertises Daybreak.
    Enable,
    /// No model of the catalog does.
    Learn,
    /// The catalog says nothing yet.
    Manage,
}

impl Access {
    /// Oracle `daybreak_account_eligible` for a thread that runs on `provider`: the account half
    /// and the provider's support. A thread whose provider is not known is not eligible.
    pub fn eligible(&self, provider: Option<&str>) -> bool {
        self.account
            && provider.is_some_and(|id| self.providers.iter().any(|supported| supported == id))
    }

    /// Oracle `daybreak_command_description`: the entry is listed when the thread's setting is
    /// on, or when the account is eligible for the thread's provider.
    pub fn row(&self, enabled: bool, provider: Option<&str>) -> Option<Row> {
        if enabled {
            Some(Row::Disable)
        } else if !self.eligible(provider) {
            None
        } else {
            Some(match self.availability() {
                Some(true) => Row::Enable,
                Some(false) => Row::Learn,
                None => Row::Manage,
            })
        }
    }

    /// Oracle `daybreak::available`: some model of the catalog advertises Daybreak.
    pub fn available(&self) -> bool {
        self.models
            .iter()
            .any(|entry| entry.programs.is_some_and(|programs| programs.daybreak))
    }

    /// Oracle `daybreak::availability`: missing or empty program lists do not establish that
    /// the account lacks access (`None`).
    pub fn availability(&self) -> Option<bool> {
        if self.available() {
            return Some(true);
        }
        if self.models.is_empty() {
            return None;
        }
        self.models
            .iter()
            .all(|entry| {
                entry
                    .programs
                    .is_some_and(|programs| programs.standard || programs.daybreak)
            })
            .then_some(false)
    }
}

/// The models whose Standard-only listing the oracle words as `Astra`.
const ASTRA: [&str; 2] = ["gpt-6-astra", "gpt-6-astra-wm"];

impl Access {
    /// Oracle `notice_for_setting` with `can_enable_daybreak` the account's eligibility for
    /// `provider`, for a thread on `model` whose own setting is `enabled`.
    pub fn notice(&self, provider: Option<&str>, model: Option<&str>, enabled: bool) -> Daybreak {
        if !self.eligible(provider) {
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
