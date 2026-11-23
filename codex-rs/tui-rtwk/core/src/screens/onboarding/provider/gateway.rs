//! Gateway sign-in (`onboarding/gateway.rs`, `app/provider/editor`): the
//! custom gateways the app server has saved, to use one as the default
//! provider, or a form to save a new one. Creating a gateway returns to the
//! picker with it marked; only a gateway picked completes the sign-in.
//! The text of the form's fields, the field marked and the picker's cursor are the view's
//! (ADR-013: it passes the text with `Save`).

use super::{Effect, Setup, Settled};
use crate::screens::onboarding::Request;

/// The form's fields (the name, the base URL, the API key variable); Save
/// is the mark after them.
pub const FIELDS: usize = 3;

/// `rpc::UNCONFIRMED_CREATE`: a save whose reply was lost (the write may
/// have been committed).
pub const UNCONFIRMED_CREATE: &str =
    "Could not confirm the save. Check the provider list before adding it again.";

/// `GatewayStep::load`: the server fixes the default provider.
pub const POLICY: &str =
    "The default provider is set by configuration policy or a launch override.";

/// A custom gateway the server has saved (`ModelProviderSummary` of
/// `CustomGateway` settings, what the picker and the form use of it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub id: String,
    pub name: String,
    /// The variable of its authentication (`provider_fields`' `Variable` and
    /// `ExternalAuth`), what the form offers to complete.
    pub variable: Option<String>,
}

/// The server's providers that are custom gateways, and the default one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub gateways: Vec<Saved>,
    pub default_id: String,
}

impl Page {
    /// The row that creates a gateway: after the saved ones.
    pub fn create_row(&self) -> usize {
        self.gateways.len()
    }

    /// The variables of the saved gateways, sorted (`Draft::variables`).
    fn variables(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .gateways
            .iter()
            .filter_map(|saved| saved.variable.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The picker: the saved gateways, the row the list lands on when it is shown again after a
/// save or a refused selection (none: the first; a one-shot hint for the view's cursor) and the
/// note under it (a refused selection, an unconfirmed save).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picker {
    pub list: Page,
    pub lands_on: Option<usize>,
    pub notice: Option<String>,
}

/// What the form refused (`Draft::patch`); the view says it (no prose in
/// core), a server message is its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormError {
    /// "Enter a provider name."
    EnterName,
    /// "Enter a base URL."
    EnterUrl,
    /// "Enter an environment variable name such as TEAM_API_KEY."
    BadVariable,
    Server(String),
}

/// The form: the error, which stays until the next save replaces it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Form {
    pub error: Option<FormError>,
    variables: Vec<String>,
}

/// The text of the fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fields {
    pub name: String,
    pub url: String,
    pub variable: String,
}

/// `ModelProviderConfigWriteParams` of a new custom gateway: the text of
/// the fields that are not blank, trimmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    pub name: String,
    pub url: String,
    pub variable: Option<String>,
}

fn value(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

impl Form {
    fn new(page: &Page) -> Form {
        Form {
            error: None,
            variables: page.variables(),
        }
    }

    /// The saved variable `text` is the start of (`ProviderForm::suggestion`;
    /// the variable's field with the cursor at its end, which the view
    /// checks).
    pub fn suggestion(&self, text: &str) -> Option<&str> {
        if text.is_empty() {
            return None;
        }
        self.variables
            .iter()
            .map(String::as_str)
            .find(|name| *name != text && name.starts_with(text))
    }

    /// `Draft::patch` of a new gateway.
    fn patch(fields: &Fields) -> Result<Patch, FormError> {
        let Some(name) = value(&fields.name) else {
            return Err(FormError::EnterName);
        };
        let Some(url) = value(&fields.url) else {
            return Err(FormError::EnterUrl);
        };
        let variable = value(&fields.variable);
        if let Some(key) = &variable
            && !(key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
        {
            return Err(FormError::BadVariable);
        }
        Ok(Patch {
            name,
            url,
            variable,
        })
    }
}

/// Where the sign-in through a gateway stands (`GatewayStep`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Reading the saved gateways; `notice` goes under the list that follows.
    Loading {
        request: Request,
        notice: Option<String>,
    },
    /// The server's refusal: the message and "Back".
    LoadError(String),
    Picking(Picker),
    Editing {
        picker: Picker,
        form: Form,
    },
    /// Saving the new gateway: no cancel, no second submission.
    Saving {
        picker: Picker,
        form: Form,
        request: Request,
    },
    /// Making the gateway at `index` the default.
    Selecting {
        picker: Picker,
        index: usize,
        request: Request,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gateway {
    pub stage: Stage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The saved gateway at this index of the picker's list is to be the default.
    Use(usize),
    /// The row that creates a gateway: the form opens.
    Create,
    /// Save the form, with the text of the fields.
    Save(Fields),
}

impl Gateway {
    /// `start_gateway_picker`: the saved gateways are asked for at once.
    pub fn start(request: &mut dyn FnMut() -> Request) -> (Gateway, Effect) {
        let request = request();
        (
            Gateway {
                stage: Stage::Loading {
                    request,
                    notice: None,
                },
            },
            Effect::Ask(request, Setup::GatewayList),
        )
    }

    /// The form is up: its fields take the keys (`is_text_entry_active`; on Save the view
    /// says it does not).
    pub fn is_text_entry(&self) -> bool {
        matches!(&self.stage, Stage::Editing { .. })
    }

    /// A request is out and the wizard waits for it.
    pub fn waits(&self) -> bool {
        matches!(
            self.stage,
            Stage::Loading { .. } | Stage::Saving { .. } | Stage::Selecting { .. }
        )
    }

    /// Esc: the picker from the form, out of the wizard from the rest; not
    /// while a save or a selection is out.
    pub fn back(&mut self) -> Effect {
        match &self.stage {
            Stage::Loading { .. } | Stage::LoadError(_) | Stage::Picking(_) => Effect::Leave,
            Stage::Editing { picker, .. } => {
                self.stage = Stage::Picking(picker.clone());
                Effect::None
            }
            Stage::Saving { .. } | Stage::Selecting { .. } => Effect::None,
        }
    }

    pub fn apply(&mut self, intent: Intent, request: &mut dyn FnMut() -> Request) -> Effect {
        match (&mut self.stage, intent) {
            (Stage::Picking(picker), Intent::Use(index)) => {
                let picker = picker.clone();
                let Some(saved) = picker.list.gateways.get(index) else {
                    return Effect::None;
                };
                let id = saved.id.clone();
                let request = request();
                self.stage = Stage::Selecting {
                    picker,
                    index,
                    request,
                };
                Effect::Ask(request, Setup::GatewaySelect(id))
            }
            (Stage::Picking(picker), Intent::Create) => {
                let mut picker = picker.clone();
                // Esc on the form lands the list on the row that opened it.
                picker.lands_on = Some(picker.list.create_row());
                let form = Form::new(&picker.list);
                self.stage = Stage::Editing { picker, form };
                Effect::None
            }
            (Stage::Editing { picker, form }, Intent::Save(fields)) => match Form::patch(&fields) {
                Ok(patch) => {
                    let request = request();
                    self.stage = Stage::Saving {
                        picker: picker.clone(),
                        form: form.clone(),
                        request,
                    };
                    Effect::Ask(request, Setup::GatewayWrite(patch))
                }
                Err(error) => {
                    form.error = Some(error);
                    Effect::None
                }
            },
            _ => Effect::None,
        }
    }

    /// The answer of the request the wizard waits for; a stale one is
    /// dropped.
    pub fn answer(
        &mut self,
        request: Request,
        outcome: Result<super::Answer, String>,
        mint: &mut dyn FnMut() -> Request,
    ) -> Settled {
        use super::Answer;
        match (&mut self.stage, outcome) {
            (
                Stage::Loading {
                    request: waiting,
                    notice,
                },
                outcome,
            ) if *waiting == request => {
                match outcome {
                    Ok(Answer::Gateways(page)) => {
                        self.stage = Stage::Picking(Picker {
                            list: page,
                            lands_on: None,
                            notice: notice.take(),
                        });
                    }
                    Ok(_) => {}
                    Err(message) => self.stage = Stage::LoadError(message),
                }
                Settled::Waiting
            }
            (
                Stage::Saving {
                    picker,
                    form,
                    request: waiting,
                },
                outcome,
            ) if *waiting == request => match outcome {
                Ok(Answer::GatewaySaved(saved)) => {
                    let mut picker = picker.clone();
                    picker.lands_on = Some(picker.list.gateways.len());
                    picker.notice = None;
                    picker.list.gateways.push(saved);
                    self.stage = Stage::Picking(picker);
                    Settled::Waiting
                }
                // A lost reply can follow a committed write: read the list
                // again and say so.
                Ok(Answer::GatewayUnconfirmed) => {
                    let request = mint();
                    self.stage = Stage::Loading {
                        request,
                        notice: Some(UNCONFIRMED_CREATE.to_owned()),
                    };
                    Settled::Ask(request, Setup::GatewayList)
                }
                Ok(_) => Settled::Waiting,
                Err(message) => {
                    let mut form = form.clone();
                    form.error = Some(FormError::Server(message));
                    self.stage = Stage::Editing {
                        picker: picker.clone(),
                        form,
                    };
                    Settled::Waiting
                }
            },
            (
                Stage::Selecting {
                    picker,
                    index,
                    request: waiting,
                },
                outcome,
            ) if *waiting == request => match outcome {
                Ok(Answer::GatewaySelected) => {
                    let id = picker
                        .list
                        .gateways
                        .get(*index)
                        .map(|saved| saved.id.clone())
                        .unwrap_or_default();
                    Settled::Selected(id)
                }
                Ok(_) => Settled::Waiting,
                Err(message) => {
                    let mut picker = picker.clone();
                    picker.lands_on = Some(*index);
                    picker.notice = Some(message);
                    self.stage = Stage::Picking(picker);
                    Settled::Waiting
                }
            },
            _ => Settled::Waiting,
        }
    }
}
