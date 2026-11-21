//! S03, the provider setup wizards of the sign-in's "another way": what each
//! has collected and where it stands, changed only by `apply` (through the
//! sign-in's `Intent` and the host's answer to its `Login::Provider` ask).
//! Oracle: `onboarding/gcp.rs`, `bedrock.rs`, `gateway.rs`,
//! `provider_setup.rs` (the shared fields).
//!
//! What the view keeps (widget state, R14): the text typed in each field
//! (`TextEditingController`s: the oracle's `ProviderInput`), which a wizard's
//! intents carry when they need it. A wizard is left by Esc on its first
//! stage; its answer to a request is dropped when it no longer waits for it
//! (the sign-in's `awaiting` ask).

use crate::screens::onboarding::Request;
use crate::screens::onboarding::auth::Provider;

pub mod gcp;

/// A wizard's own state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wizard {
    Gcp(gcp::Gcp),
}

/// What a wizard asks the host to run (`Login::Provider`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setup {
    Gcp(gcp::Params),
}

/// The answer to a `Setup`: the wizard's configuration was written, or the
/// message the server refused it with.
pub type Outcome = Result<(), String>;

/// A wizard's intents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A field's text changed (any key handled clears the error).
    Edited,
    Gcp(gcp::Intent),
}

/// What an intent of a wizard asks of the sign-in step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing more.
    None,
    /// Esc on the first stage: back to the sign-in menu.
    Leave,
    /// Run `setup`, answered under the request.
    Ask(Request, Setup),
}

impl Wizard {
    /// The wizard of a provider; none for one not built yet.
    pub fn of(provider: Provider) -> Option<Wizard> {
        match provider {
            Provider::Gcp => Some(Wizard::Gcp(gcp::Gcp::default())),
            Provider::Bedrock | Provider::Gateway => None,
        }
    }

    /// A text field has the keys (`is_text_entry_active`).
    pub fn is_text_entry(&self) -> bool {
        match self {
            Wizard::Gcp(gcp) => gcp.is_text_entry(),
        }
    }

    /// A key of the wizard clears its error (`handle_*_key_event` set the
    /// error to none after every key it handled).
    pub fn clear_error(&mut self) {
        match self {
            Wizard::Gcp(gcp) => gcp.error = None,
        }
    }

    /// Esc: one stage back, or out of the wizard.
    pub fn back(&mut self) -> Effect {
        match self {
            Wizard::Gcp(gcp) => gcp.back(),
        }
    }

    pub fn apply(&mut self, intent: Intent, request: &mut dyn FnMut() -> Request) -> Effect {
        match (self, intent) {
            (wizard, Intent::Edited) => {
                wizard.clear_error();
                Effect::None
            }
            (Wizard::Gcp(gcp), Intent::Gcp(intent)) => gcp.apply(intent, request),
        }
    }

    /// The answer of the setup the wizard waits for: whether the provider is
    /// configured; a refusal puts the wizard back where it can retry.
    pub fn answer(&mut self, request: Request, outcome: Outcome) -> bool {
        match self {
            Wizard::Gcp(gcp) => gcp.answer(request, outcome),
        }
    }
}
