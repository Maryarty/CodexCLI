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

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(project: &str, location: &str, credential: &str) -> Fields {
        Fields {
            project: project.into(),
            location: location.into(),
            credential_file: credential.into(),
        }
    }

    fn next(gcp: &mut Gcp, fields: Fields) -> Effect {
        gcp.apply(Intent::Next(fields), &mut || 7)
    }

    // gcp_tests.rs:16 gcp_menu_gating_and_setup_flow (the wizard's part)
    #[test]
    fn an_empty_project_is_refused_then_the_stages_follow() {
        let mut gcp = Gcp::default();
        assert_eq!(next(&mut gcp, fields("  ", "", "")), Effect::None);
        assert_eq!(gcp.error, Some(Error::EnterProject));
        assert_eq!(gcp.stage, Stage::Project);
        next(&mut gcp, fields("test-project", "", ""));
        assert_eq!((gcp.stage, gcp.error.clone()), (Stage::Location, None));
        next(&mut gcp, fields("test-project", "us-central1", ""));
        assert_eq!(gcp.stage, Stage::Credentials);
    }

    #[test]
    fn esc_goes_one_stage_back_and_leaves_from_the_first() {
        let mut gcp = Gcp {
            stage: Stage::Credentials,
            error: Some(Error::EnterProject),
        };
        assert_eq!(gcp.back(), Effect::None);
        assert_eq!((gcp.stage, gcp.error.clone()), (Stage::Location, None));
        gcp.back();
        assert_eq!(gcp.back(), Effect::Leave);
    }

    #[test]
    fn the_credential_stage_asks_for_the_setup_and_waits() {
        let mut gcp = Gcp {
            stage: Stage::Credentials,
            error: None,
        };
        let effect = next(&mut gcp, fields(" test-project ", "", " /adc.json "));
        assert_eq!(
            effect,
            Effect::Ask(
                7,
                Setup::Gcp(Params {
                    project_id: "test-project".into(),
                    location: None,
                    credential_file: Some("/adc.json".into()),
                })
            )
        );
        assert_eq!(gcp.stage, Stage::Configuring(7));
        // No second submission and no cancel while the server works.
        assert_eq!(next(&mut gcp, fields("p", "", "")), Effect::None);
        assert_eq!(gcp.back(), Effect::None);
        assert_eq!(gcp.stage, Stage::Configuring(7));
        assert!(!gcp.is_text_entry());
    }

    #[test]
    fn a_refusal_returns_to_the_credentials_and_a_stale_answer_is_dropped() {
        let mut gcp = Gcp {
            stage: Stage::Configuring(7),
            error: None,
        };
        assert!(!gcp.answer(6, Ok(())));
        assert_eq!(gcp.stage, Stage::Configuring(7));
        assert!(!gcp.answer(7, Err("must be absolute".into())));
        assert_eq!(gcp.stage, Stage::Credentials);
        assert_eq!(gcp.error, Some(Error::Server("must be absolute".into())));
        gcp.stage = Stage::Configuring(8);
        assert!(gcp.answer(8, Ok(())));
    }
}
