//! Google Cloud Vertex AI setup (`onboarding/gcp.rs`): the project, the
//! location and the credential file in turn, then the app server validates
//! and writes the configuration; the step stays in progress until it
//! answers. The fields' text is the view's (it passes it with `Next`).

use super::{Effect, Setup};
use crate::screens::onboarding::Request;

/// Where the setup stands (`GcpView`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Project,
    Location,
    Credentials,
    /// Validating and saving: no cancel, no second submission.
    Configuring(Request),
}

/// What went wrong; the view says it (no prose in core).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// "Enter a Google Cloud project ID."
    EnterProject,
    /// The server's refusal.
    Server(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gcp {
    pub stage: Stage,
    pub error: Option<Error>,
}

/// `account/gcp/setup` (`GcpSetupParams`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    pub project_id: String,
    pub location: Option<String>,
    pub credential_file: Option<String>,
}

/// The text of the three fields, as the view has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fields {
    pub project: String,
    pub location: String,
    pub credential_file: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Enter on the stage, with the fields' text.
    Next(Fields),
}

/// `ProviderInput::value`: the trimmed text, none when blank.
fn value(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

impl Gcp {
    pub fn is_text_entry(&self) -> bool {
        !matches!(self.stage, Stage::Configuring(_))
    }

    pub fn back(&mut self) -> Effect {
        match self.stage {
            Stage::Project => return Effect::Leave,
            Stage::Location => self.stage = Stage::Project,
            Stage::Credentials => self.stage = Stage::Location,
            Stage::Configuring(_) => return Effect::None,
        }
        self.error = None;
        Effect::None
    }

    pub fn apply(&mut self, intent: Intent, request: &mut dyn FnMut() -> Request) -> Effect {
        let Intent::Next(fields) = intent;
        match self.stage {
            Stage::Project if value(&fields.project).is_none() => {
                self.error = Some(Error::EnterProject);
                return Effect::None;
            }
            Stage::Project => self.stage = Stage::Location,
            Stage::Location => self.stage = Stage::Credentials,
            Stage::Credentials => {
                let request = request();
                self.stage = Stage::Configuring(request);
                self.error = None;
                return Effect::Ask(
                    request,
                    Setup::Gcp(Params {
                        project_id: value(&fields.project).unwrap_or_default(),
                        location: value(&fields.location),
                        credential_file: value(&fields.credential_file),
                    }),
                );
            }
            Stage::Configuring(_) => return Effect::None,
        }
        self.error = None;
        Effect::None
    }

    /// `spawn_provider_setup`: the answer of the setup being configured; a
    /// refusal goes back to the credential file with its message.
    pub fn answer(&mut self, request: Request, outcome: Result<(), String>) -> bool {
        if self.stage != Stage::Configuring(request) {
            return false;
        }
        match outcome {
            Ok(()) => true,
            Err(message) => {
                self.stage = Stage::Credentials;
                self.error = Some(Error::Server(message));
                false
            }
        }
    }
}
