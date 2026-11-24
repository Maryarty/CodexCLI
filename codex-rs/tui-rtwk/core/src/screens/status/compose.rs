//! The card of a `/status` command, made from the facts the app holds at that
//! moment (oracle `StatusHistoryCell::new`, `status_controls.rs`
//! `add_status_output`). Pure: the caller (`app/session.rs`) reads the models
//! it owns and passes the facts; the clock is a fact too.

use super::card::{Account, Permissions, Reasoning, Remote, SandboxKind, StatusCard};
use super::helpers::AgentsSource;
use super::limits::Limits;
use super::{Approval, Context, Tokens};

/// What the host reads of the loaded config for the card (the part of
/// `StatusFacts` the status line has no use for).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct CardFacts {
    /// The working directory, as the config has it.
    pub cwd: String,
    /// The id of the active permission profile (`:workspace`, a user's own).
    pub profile: Option<String>,
    pub sandbox: SandboxKind,
    /// The sandbox lets the network through.
    pub network: bool,
    /// The profile is the disabled one (no sandbox at all).
    pub disabled: bool,
    /// Workspace roots other than the directory.
    pub extra_roots: Vec<String>,
    pub approval: Approval,
    /// The model's wire protocol has reasoning settings (`WireApi::Responses`).
    pub reasoning: bool,
    /// `model_reasoning_summary`; none reads `auto`.
    pub reasoning_summary: Option<String>,
    /// The configured provider uses the first-party auth path: only then is
    /// the account's workspace shown.
    pub first_party_provider: bool,
    /// `model_context_window`, the window before any usage is known.
    pub context_window: Option<i64>,
}

impl Default for CardFacts {
    fn default() -> Self {
        Self {
            cwd: String::new(),
            profile: None,
            sandbox: SandboxKind::Workspace,
            network: false,
            disabled: false,
            extra_roots: Vec::new(),
            approval: Approval::OnRequest,
            reasoning: true,
            reasoning_summary: None,
            first_party_provider: true,
            context_window: None,
        }
    }
}

/// One model of the server's catalog (oracle `ModelPreset`): its name as shown
/// and the reasoning it runs with when nothing says otherwise.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct CatalogModel {
    pub model: String,
    pub display_name: String,
    #[serde(default)]
    pub default_effort: Option<String>,
}

/// The facts a card is made of. Borrowed from the models that own them.
pub struct Inputs<'a> {
    pub version: &'a str,
    pub config: &'a CardFacts,
    /// The model of the session, none before the thread said.
    pub model: Option<&'a str>,
    /// The reasoning effort the thread runs with, else the config's.
    pub effort: Option<&'a str>,
    pub provider: Option<&'a str>,
    /// The provider needs OpenAI auth: the card links the usage page.
    pub requires_openai_auth: bool,
    pub auto_review: bool,
    pub home: Option<&'a str>,
    pub catalog: &'a [CatalogModel],
    pub account: Option<&'a Account>,
    pub remote: Option<&'a Remote>,
    /// The thread the card is made in.
    pub thread: ThreadFacts<'a>,
    pub limits: Limits,
}

/// The thread's own facts.
pub struct ThreadFacts<'a> {
    pub id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub cwd: Option<&'a str>,
    pub forked_from: Option<&'a str>,
    /// The collaboration mode the thread runs in, when the card names it
    /// (oracle `collaboration_mode_label`).
    pub collaboration_mode: Option<&'a str>,
    pub agents: &'a [AgentsSource],
    pub tokens: Tokens,
    pub context: Option<Context>,
}

/// Shown for the model before the thread names one (oracle
/// `DEFAULT_MODEL_DISPLAY_NAME`).
const DEFAULT_MODEL_DISPLAY_NAME: &str = "loading";

/// The provider id of the built-in first-party provider.
const OPENAI_PROVIDER_ID: &str = "openai";

impl StatusCard {
    /// The card of a `/status` that runs now.
    pub fn compose(inputs: Inputs<'_>) -> Self {
        let Inputs {
            version,
            config,
            model,
            effort,
            provider,
            requires_openai_auth,
            auto_review,
            home,
            catalog,
            account,
            remote,
            thread,
            limits,
        } = inputs;
        let model_id = model.filter(|model| !model.is_empty());
        let display = match model_id {
            None => DEFAULT_MODEL_DISPLAY_NAME.to_string(),
            Some(model) => catalog
                .iter()
                .find(|preset| preset.model == model)
                .map_or_else(|| model.to_string(), |preset| preset.display_name.clone()),
        };
        let default_effort = model_id.and_then(|model| {
            catalog
                .iter()
                .find(|preset| preset.model == model)
                .and_then(|preset| preset.default_effort.as_deref())
        });
        let reasoning = config.reasoning.then(|| Reasoning {
            effort: effort
                .or(default_effort)
                .map_or_else(|| "none".to_string(), str::to_string),
            summary: remote
                .is_none()
                .then(|| config.reasoning_summary.as_deref().unwrap_or("auto").to_string()),
        });
        let provider = provider
            .filter(|id| !id.trim().is_empty())
            .map(str::to_string);
        // A connected server owns the provider definitions; only the built-in
        // id is certain.
        let workspace_matches_provider = if remote.is_some() {
            provider
                .as_deref()
                .is_none_or(|provider| provider == OPENAI_PROVIDER_ID)
        } else {
            config.first_party_provider
        };
        let account = account.cloned().map(|account| match account {
            Account::ChatGpt {
                email,
                plan,
                workspace,
            } => Account::ChatGpt {
                email,
                plan,
                workspace: workspace.filter(|_| workspace_matches_provider),
            },
            other => other,
        });
        let context = thread.context.or_else(|| {
            config.context_window.map(|window| Context {
                window,
                used: 0,
                percent_left: 100,
            })
        });
        Self {
            version: version.to_string(),
            remote: remote.cloned(),
            model: display,
            model_id: model_id.map(str::to_string),
            reasoning,
            // The provider is shown once a thread exists.
            provider: thread.id.and(provider),
            directory: thread
                .cwd
                .filter(|cwd| !cwd.is_empty())
                .unwrap_or(&config.cwd)
                .to_string(),
            home: home.map(str::to_string),
            permissions: Permissions {
                profile: config.profile.clone(),
                sandbox: config.sandbox,
                network: config.network,
                disabled: config.disabled,
                approval: config.approval,
                auto_review,
                extra_roots: config.extra_roots.clone(),
            },
            agents: thread.agents.to_vec(),
            account,
            thread_name: thread.name.map(str::to_string),
            session_id: thread.id.map(str::to_string),
            forked_from: thread.forked_from.map(str::to_string),
            collaboration_mode: thread.collaboration_mode.map(str::to_string),
            usage_link: requires_openai_auth,
            tokens: thread.tokens,
            context,
            limits,
            hour24: false,
            number: 0,
        }
    }
}
