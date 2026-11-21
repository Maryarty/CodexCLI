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

pub mod bedrock;
pub mod gateway;
pub mod gcp;
pub mod wire;

/// A wizard's own state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wizard {
    Bedrock(bedrock::Bedrock),
    Gcp(gcp::Gcp),
    Gateway(gateway::Gateway),
}

/// What a wizard asks the host to run (`Login::Provider`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setup {
    /// `account/bedrock/discover`: the AWS credentials the server finds.
    BedrockDiscover,
    Bedrock(bedrock::Params),
    Gcp(gcp::Params),
    /// `model_provider/list`: the custom gateways the server has saved.
    GatewayList,
    /// `model_provider/config/write`: save a new custom gateway.
    GatewayWrite(gateway::Patch),
    /// `config/value/write` of `model_provider`: the gateway with this id
    /// becomes the default.
    GatewaySelect(String),
}

/// What the server answered a `Setup` with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The wizard's configuration was written.
    Written,
    /// The credentials found (`BedrockDiscover`).
    Discovered(bedrock::Discovery),
    /// The saved custom gateways (`GatewayList`).
    Gateways(gateway::Page),
    /// The gateway written (`GatewayWrite`).
    GatewaySaved(gateway::Saved),
    /// A create that may have been written (`GatewayWrite`, no reply).
    GatewayUnconfirmed,
    /// The default provider is the gateway (`GatewaySelect`).
    GatewaySelected,
}

/// What an answer made of the wizard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settled {
    /// It goes on (a stale answer, a refusal that put it back, a new view).
    Waiting,
    /// The provider is configured.
    Configured,
    /// The gateway with this id is selected.
    Selected(String),
    /// Another request is due.
    Ask(Request, Setup),
}

/// The answer to a `Setup`, or the message the server refused it with.
pub type Outcome = Result<Answer, String>;

/// A wizard's intents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A field's text changed (any key handled clears the error).
    Edited,
    Bedrock(bedrock::Intent),
    Gcp(gcp::Intent),
    Gateway(gateway::Intent),
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
    /// The wizard of a provider, and what it asks at once (the gateway
    /// reads the saved ones as it opens).
    pub fn of(provider: Provider, request: &mut dyn FnMut() -> Request) -> (Wizard, Effect) {
        match provider {
            Provider::Bedrock => (Wizard::Bedrock(bedrock::Bedrock::default()), Effect::None),
            Provider::Gcp => (Wizard::Gcp(gcp::Gcp::default()), Effect::None),
            Provider::Gateway => {
                let (gateway, effect) = gateway::Gateway::start(request);
                (Wizard::Gateway(gateway), effect)
            }
        }
    }

    /// A text field has the keys (`is_text_entry_active`).
    pub fn is_text_entry(&self) -> bool {
        match self {
            Wizard::Bedrock(bedrock) => bedrock.is_text_entry(),
            Wizard::Gcp(gcp) => gcp.is_text_entry(),
            Wizard::Gateway(gateway) => gateway.is_text_entry(),
        }
    }

    /// A request of the wizard is out and it waits for the answer: Esc
    /// does not leave it.
    pub fn waits(&self) -> bool {
        match self {
            Wizard::Bedrock(bedrock) => bedrock.waits(),
            Wizard::Gcp(gcp) => matches!(gcp.stage, gcp::Stage::Configuring(_)),
            Wizard::Gateway(gateway) => gateway.waits(),
        }
    }

    /// A key of the wizard clears its error (`handle_*_key_event` set the
    /// error to none after every key it handled).
    pub fn clear_error(&mut self) {
        match self {
            Wizard::Bedrock(bedrock) => bedrock.error = None,
            Wizard::Gcp(gcp) => gcp.error = None,
            // The form's error stays until the next save replaces it.
            Wizard::Gateway(_) => {}
        }
    }

    /// Esc: one stage back, or out of the wizard.
    pub fn back(&mut self) -> Effect {
        match self {
            Wizard::Bedrock(bedrock) => bedrock.back(),
            Wizard::Gcp(gcp) => gcp.back(),
            Wizard::Gateway(gateway) => gateway.back(),
        }
    }

    pub fn apply(&mut self, intent: Intent, request: &mut dyn FnMut() -> Request) -> Effect {
        match (self, intent) {
            (wizard, Intent::Edited) => {
                wizard.clear_error();
                Effect::None
            }
            (Wizard::Bedrock(bedrock), Intent::Bedrock(intent)) => bedrock.apply(intent, request),
            (Wizard::Gcp(gcp), Intent::Gcp(intent)) => gcp.apply(intent, request),
            (Wizard::Gateway(gateway), Intent::Gateway(intent)) => gateway.apply(intent, request),
            _ => Effect::None,
        }
    }

    /// The answer of the setup the wizard waits for: whether the provider is
    /// configured; a refusal puts the wizard back where it can retry.
    pub fn answer(
        &mut self,
        request: Request,
        outcome: Outcome,
        mint: &mut dyn FnMut() -> Request,
    ) -> Settled {
        let configured = |done: bool| if done { Settled::Configured } else { Settled::Waiting };
        match self {
            Wizard::Bedrock(bedrock) => configured(bedrock.answer(request, outcome)),
            Wizard::Gcp(gcp) => configured(gcp.answer(request, outcome.map(|_| ()))),
            Wizard::Gateway(gateway) => gateway.answer(request, outcome, mint),
        }
    }
}
