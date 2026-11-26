//! `onboarding` (S01-S04, the startup page): the core half of the screen's
//! directory. The facts the startup page runs on (an update offered, the
//! sign-in due, the folder trust to ask) and what the user chose, changed
//! only by `apply`; the view half is `src/pages/startup/onboarding/` in the UI
//! crate. Oracle: `update_prompt.rs` (S01), `onboarding/onboarding_screen.rs`
//! (which steps show, in what order), `onboarding/welcome.rs`,
//! `onboarding/trust_directory.rs` and `onboarding/directory_trust.rs` (S04);
//! the sign-in step (S02) is `auth`.

pub mod auth;
pub mod consent;
pub mod device;
pub mod gateway;
pub mod session;

use codex_app_server_protocol::AuthMode;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GatewayOAuthStatus;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use serde::Deserialize;

use crate::app::AppFx;
use crate::app::Command as AppCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::startup;
use crate::threads::ThreadFx;

pub const NAME: &str = "onboarding";
pub const ID: &str = "S01-S04";
pub const KIND: Kind = Kind::Route;

/// The steps of the startup page, in the order they run
/// (`run_update_prompt_if_needed` before `OnboardingScreen::new`'s steps).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stage {
    /// S01: a newer version is available.
    Update,
    /// S02: sign in (the `onboarding` task's slot).
    SignIn,
    /// S03: provider setup (the `onboarding` task's slot).
    Provider,
    /// S04: trust the working folder.
    Trust,
}

/// The app-scoped model state of the startup page, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// S01: the update offered (`updates::get_upgrade_version_for_popup`).
    pub update: Option<UpdateOffer>,
    /// The welcome header: shown while the user is signed out
    /// (`WelcomeWidget::get_step_state`).
    pub welcome: Option<Welcome>,
    /// S02: the sign-in step, due (`show_login_screen`).
    pub sign_in: Option<auth::SignIn>,
    /// S03: the provider setup chosen at sign-in, until it is configured.
    pub provider: Option<Decided>,
    /// S04: the folder to trust (`show_trust_screen`).
    pub trust: Option<TrustAsk>,
    /// The user asked to leave the app from a step (quit, Ctrl+C).
    pub exit: bool,
    /// The launch facts were read (`Ev::Facts`): they are asked once.
    read: bool,
    /// The facts ask awaited: an answer under another number is stale.
    asked: Option<Request>,
    /// The trust write awaited: an answer under another number is stale.
    writing: Option<Request>,
    /// The update run awaited: the host stores the update before the page
    /// leaves.
    running: Option<Request>,
    /// The host stored the update to run.
    update_queued: bool,
    /// The last request number minted.
    minted: Request,
    /// The launch's startup prompts asked for (`Ask::Launch`), awaited.
    launching: Option<Request>,
    /// The launch's startup prompts are known (`Ev::Launched`).
    launched: bool,
    /// The provider panel was asked by the host before the config exists
    /// (`Ev::OssPick`): the page stays until the launch facts come, as the
    /// steps they bring are asked after the panel.
    holding: bool,
    /// The startup prompts due at launch not asked yet, in order (S05:
    /// the hooks review, the model migration).
    pub prompts: Vec<startup::Prompt>,
    /// The selected gateway's own sign-in, run after the steps
    /// (`authenticate_gateway`).
    gateway: gateway::Phase,
    /// The config was reloaded after the steps (asked once; asked again with
    /// a new connection).
    reloaded: bool,
    /// The gateway's device code, asked for once the steps are over and
    /// before the launch's prompts (`device_login::start_if_needed`).
    device: device::Phase,
    /// The session `codex resume <id>` / `codex fork <id>` names.
    session: session::Phase,
    /// What the host read for the session's working directory (`Facts`).
    resume: startup::cwd::Setting,
    /// The folder trust asked over the chat (`/cd`), apart from the launch's.
    consent: consent::Consent,
}

/// The number of an ask of this page to the host (`Ask`): its answer carries
/// it back, and one the page no longer waits for is dropped.
pub type Request = u64;

/// What the startup page asks the host for (the root `Command::Startup`):
/// IO the host runs, answered with this page's events under the ask's number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The launch facts (`Ev::Facts`): the update offered, the sign-in due,
    /// the folder trust to ask, the welcome's animation.
    Facts { request: Request },
    /// Save `target` as a trusted project (`config/batchWrite`), answered
    /// with `Ev::TrustSaved` or `Ev::TrustFailed`.
    WriteTrust { request: Request, target: String },
    /// Read folder trust before a session action, answered with
    /// `Ev::Consent` or `Ev::ConsentFailed` under `request`.
    CheckConsent {
        request: Request,
        target: consent::Target,
        cancel: TrustCancel,
    },
    /// Do not offer `version` again (`updates::dismiss_version`); not answered.
    DismissUpdate { version: String },
    /// Run the offered update once the app has ended (`UpdateAction`),
    /// answered with `Ev::UpdateQueued`: the page leaves only then, so the
    /// host has the update stored before the app ends.
    RunUpdate { request: Request },
    /// Start a login (`account/login/start`), answered with
    /// `Ev::Auth(AuthResult { request, .. })`; ChatGPT's opens the browser
    /// on a local server.
    Login {
        request: Request,
        login: auth::Login,
    },
    /// Cancel a login started (`account/login/cancel`); not answered.
    CancelLogin { login_id: String },
    /// Show the ChatGPT login's `url` in the browser: asked only for the
    /// attempt the page still awaits (a stale or cancelled one never opens
    /// it), and the host skips it for a remote workspace
    /// (`show_chatgpt_login_in_browser`); not answered.
    OpenBrowser { url: String },
    /// Reload the session's config now that the folder is trusted (the
    /// project's config and hooks apply); the host skips it for a remote
    /// workspace (`consent.directory_trust_persisted && !uses_remote_workspace`).
    /// Its answer, if any, is an event of the app.
    ReloadConfig,
    /// The startup prompts due now that the steps are over (S05; the hooks
    /// of the trusted folder, the model migration), answered with
    /// `Ev::Launched`. The host reads them after a reload asked before it.
    Launch { request: Request },
    /// Remember the working directory choice of the cwd prompt
    /// (`tui.resume_cwd`), answered with the startup screen's
    /// `Ev::CwdSaved` under `request`.
    SaveResumeCwd {
        request: u64,
        mode: startup::cwd::ResumeMode,
    },
    /// Trust the reviewed hooks (`write_hook_trusts`), answered with the
    /// startup screen's `Ev::HooksTrusted` under `request`.
    TrustHooks {
        request: u64,
        updates: Vec<startup::HookTrust>,
    },
    /// Save the answer to the model migration (`record_model_migration_seen`;
    /// accepted, also the model and its effort: `PersistModelSelection`);
    /// not answered, a failure is only logged.
    SaveMigration {
        from: String,
        to: String,
        accepted: bool,
        effort: Option<String>,
    },
    /// The GovCloud guidance was acknowledged: not shown again
    /// (`notice.hide_gov_cloud_guidance`); not answered, a failed write is
    /// only logged.
    AcknowledgeGovCloud,
    /// The provider panel (`--oss`, `oss_selection.rs`) was answered, before the config exists:
    /// the launch goes on with the provider, which the config then keeps when it was picked
    /// here; None (Ctrl-C) ends the launch with `OSS provider selection was cancelled by user`.
    /// The launch task answers it itself (nothing is asked of the host's startup worker).
    OssChoice {
        provider: Option<super::startup::Provider>,
    },
    /// The background-server recovery menu was answered (`daemon_recovery.rs`): Cancel (also
    /// Ctrl-C) ends the launch with the compatibility error, Run without starts the embedded
    /// server with a warning, Restart restarts the managed server with the settings asked, checks
    /// once more and ends the launch with the error when it still differs. The mismatch itself
    /// is the host's (it read it); not answered, the launch goes on with the server it ends up
    /// with, or ends.
    DaemonChoice {
        choice: super::startup::DaemonChoice,
    },
    /// The gateway `provider_id` was selected: read the configuration again,
    /// check it selects that gateway and start the embedded server with it
    /// (`lib.rs`' `selected_gateway`), answered with `Ev::GatewayStarted`. A
    /// configuration that cannot be read, or selects another provider, ends
    /// the launch with its message.
    SelectGateway {
        request: Request,
        provider_id: String,
    },
    /// Read the account and the provider on the server of the selected
    /// gateway (`authenticate_gateway`), answered with `Ev::GatewayLogins`.
    GatewayLogins {
        request: Request,
        provider_id: String,
    },
    /// Sign in to the gateway (`start_gateway_oauth`: read the status,
    /// log in unless the device code is the way), answered with
    /// `Ev::GatewayOAuth`.
    GatewayOAuth {
        request: Request,
        provider_id: String,
    },
    /// Cancel the gateway sign-in on the way out
    /// (`GatewayOAuthCancel`); not answered.
    CancelGatewayOAuth,
    /// Start the gateway's device code if the session's gateway needs a
    /// sign-in and a device code is the way (`device_login::start_if_needed`),
    /// answered with `Ev::GatewayDevice`. A read that fails ends the launch
    /// with its message.
    GatewayDevice {
        request: Request,
        /// The provider of the thread a `resume <id>` launch reads first.
        thread: Option<session::Resumed>,
    },
    /// End the launch with `message` (the oracle returns the error from
    /// `run_ratatui_app`); not answered.
    FailLaunch { message: String },
    /// Refresh the update cache over the network (`check_for_update`), asked
    /// once after the facts that found it stale; not answered, a failure is
    /// only logged.
    RefreshUpdateCache,
}

/// The launch facts the host read, all at once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub update: Option<UpdateOffer>,
    pub welcome: Option<Welcome>,
    /// The sign-in due, with what it offers (the provider setups among it).
    pub sign_in: Option<auth::Offer>,
    pub trust: Option<TrustAsk>,
    /// The update cache is missing or stale: the page asks the host to
    /// refresh it (`Ask::RefreshUpdateCache`), for the next run.
    pub refresh_update: bool,
    /// What the working directory of a resumed or forked session is decided
    /// by (the config's `tui.resume_cwd`, `--cd`, the workspace).
    pub resume: startup::cwd::Setting,
}

/// A slot step's state: due, or done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decided {
    pub done: bool,
}

/// S01: the versions and the command that updates (`UpdateAction::command_str`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateOffer {
    pub current: String,
    pub latest: String,
    pub command: String,
    pub choice: Option<UpdateChoice>,
}

/// The three answers of the update prompt (`UpdateSelection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UpdateChoice {
    UpdateNow,
    Skip,
    SkipUntilNextVersion,
}

/// The welcome header's options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Welcome {
    /// `tui.animations && tui.effects.welcome`.
    pub animations: bool,
}

/// What the cancel choice of the trust step does (`TrustCancelAction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrustCancel {
    #[default]
    Quit,
    AgentsOverview,
    CurrentTask,
}

/// S04: the folder trust asked for (`TrustDirectoryWidget`'s data).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrustAsk {
    pub cwd: String,
    /// The repository root trust applies to (the cwd outside a repository).
    pub target: String,
    /// The folder is saved as untrusted: it opens restricted.
    pub restricted: bool,
    /// A task saved while the folder was trusted reopens in it.
    pub existing_task: bool,
    pub cancel: TrustCancel,
    /// Windows: continuing creates the sandbox.
    pub sandbox_hint: bool,
    /// The last trust write failed: the server's message (the view says
    /// for which folder).
    pub error: Option<String>,
    pub decision: Option<TrustDecision>,
}

/// What became of the trust step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrustDecision {
    /// Trust asked for: the write is out, the step waits for it (R9).
    Saving,
    /// The write is confirmed.
    Trusted,
    /// A restricted folder opened as it is: nothing is written.
    Opened,
    /// The cancel choice.
    Cancelled,
}

impl Model {
    /// The steps due, in order: what the startup page runs.
    pub fn steps(&self) -> Vec<Stage> {
        let mut steps = Vec::new();
        if self.update.is_some() {
            steps.push(Stage::Update);
        }
        if self.sign_in.is_some() {
            steps.push(Stage::SignIn);
        }
        if self.provider.is_some() {
            steps.push(Stage::Provider);
        }
        if self.trust.is_some() {
            steps.push(Stage::Trust);
        }
        steps
    }

    /// Whether `step` is finished.
    pub fn done(&self, step: Stage) -> bool {
        if self.exit {
            return true;
        }
        match step {
            // "Update now" is done once the host holds the update to run.
            Stage::Update => self.update.as_ref().is_none_or(|u| match u.choice {
                None => false,
                Some(UpdateChoice::UpdateNow) => self.update_queued,
                Some(UpdateChoice::Skip | UpdateChoice::SkipUntilNextVersion) => true,
            }),
            // A selected gateway still has to sign in on its own server.
            Stage::SignIn => self
                .sign_in
                .as_ref()
                .is_none_or(|s| s.done() && !self.gateway.active()),
            Stage::Provider => self.provider.is_none_or(|d| d.done),
            Stage::Trust => self.trust.as_ref().is_none_or(|t| {
                matches!(
                    t.decision,
                    Some(TrustDecision::Trusted | TrustDecision::Opened | TrustDecision::Cancelled)
                )
            }),
        }
    }

    /// Where the selected gateway's own sign-in stands (it runs after the
    /// steps).
    pub fn gateway(&self) -> &gateway::Phase {
        &self.gateway
    }

    /// Whether the app opens on the startup page: a step is not finished,
    /// or a startup prompt is due.
    pub fn due(&self) -> bool {
        self.holding
            || self.steps().into_iter().any(|step| !self.done(step))
            || !self.prompts.is_empty()
            || self.device_code().is_some()
    }

    /// The gateway's one-time code on screen (the launch waits for that
    /// attempt to end); none once the app is leaving.
    pub fn device_code(&self) -> Option<&auth::DeviceCode> {
        if self.ends() {
            return None;
        }
        self.device.code()
    }

    /// Whether the launch ends with the steps: a quit, an update to run, the
    /// trust prompt cancelled (the oracle exits or opens the overview, and
    /// asks no startup prompt).
    pub fn ends(&self) -> bool {
        self.exit
            || self
                .update
                .as_ref()
                .is_some_and(|u| u.choice == Some(UpdateChoice::UpdateNow))
            || self
                .trust
                .as_ref()
                .is_some_and(|t| t.decision == Some(TrustDecision::Cancelled))
    }

    /// Whether the launch's startup prompts are known (`Ev::Launched`).
    pub fn launched(&self) -> bool {
        self.launched
    }

    /// Whether the steps are over and the launch's startup prompts are
    /// asked of the host but not known yet: the page waits for them.
    pub fn awaits_launch(&self) -> bool {
        self.read
            && !self.launched
            && !self.ends()
            && self.steps().into_iter().all(|s| self.done(s))
    }

    /// Whether the host asked the provider panel before the launch facts
    /// (`--oss`, both or no local server running) and they have not come:
    /// the page holds open, past the answer.
    pub fn holding(&self) -> bool {
        self.holding
    }

    /// Whether the launch is decided: the facts were read, or a step is due
    /// without them (a fixture). Until then the app does not know whether
    /// the chat or the startup page comes first.
    pub fn settled(&self) -> bool {
        self.due() || (self.read && (self.launched || self.ends()))
    }

    /// The write the trust step waits for: the folder to save as trusted.
    pub fn trust_write(&self) -> Option<&str> {
        self.trust
            .as_ref()
            .filter(|t| t.decision == Some(TrustDecision::Saving))
            .map(|t| t.target.as_str())
    }
}

/// The events of this screen; a variant of the root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The startup facts, read by the host at launch.
    Update {
        current: String,
        latest: String,
        command: String,
    },
    Welcome(Welcome),
    /// The sign-in is due, offering this.
    SignIn(auth::Offer),
    /// A sign-in state the oracle's tests start from (fixtures).
    SignInState(auth::State),
    Trust(TrustAsk),
    /// The answer to `Ask::Login`.
    Auth(auth::AuthResult),
    /// `account/login/completed`.
    LoginCompleted {
        login_id: Option<String>,
        success: bool,
        error: Option<String>,
    },
    /// `account/updated`: the account is a ChatGPT one.
    AccountUpdated {
        chatgpt_account: bool,
    },
    /// The host found both or none of the local servers running (`--oss`) and asks the provider
    /// before the config is built: the panel opens first among the startup prompts and the
    /// page stays until the facts.
    OssPick {
        lmstudio: super::startup::Status,
        ollama: super::startup::Status,
    },
    /// The answer to `Ask::Facts { request }`.
    Facts {
        request: Request,
        facts: Facts,
    },
    /// The trust write `request` is confirmed.
    TrustSaved {
        request: Request,
    },
    /// The answer to `Ask::RunUpdate { request }`: the update is stored.
    UpdateQueued {
        request: Request,
    },
    /// The answer to `Ask::CheckConsent { request, .. }`: the question the
    /// folder needs, none when it needs none.
    Consent {
        request: Request,
        ask: Option<TrustAsk>,
    },
    /// The host also read a cwd that the session action still needs.
    ConsentAt {
        request: Request,
        ask: Option<TrustAsk>,
        checked: consent::Checked,
    },
    /// The folder's trust of `Ask::CheckConsent { request, .. }` could not be
    /// read.
    ConsentFailed {
        request: Request,
        message: String,
    },
    /// The trust write `request` failed with this message.
    TrustFailed {
        request: Request,
        message: String,
    },
    /// The answer to `Ask::Launch { request }`: the startup prompts due.
    Launched {
        request: Request,
        prompts: Vec<startup::Prompt>,
    },
    /// The answer to `Ask::SelectGateway { request, .. }`: the server runs
    /// with the selected gateway.
    GatewayStarted {
        request: Request,
    },
    /// The answer to `Ask::GatewayLogins { request, .. }`.
    GatewayLogins {
        request: Request,
        logins: gateway::Logins,
    },
    /// The answer to `Ask::GatewayOAuth { request, .. }`: signed in, or the
    /// message the sign-in failed with.
    GatewayOAuth {
        request: Request,
        result: Result<(), String>,
    },
    /// `account/gatewayOAuth/changed`: the page to finish the sign-in in.
    GatewayOAuthChanged {
        provider_id: String,
        auth_url: Option<String>,
    },
    /// The answer to `Ask::GatewayDevice { request }`: the code to show, or
    /// none when the launch goes on without a prompt.
    GatewayDevice {
        request: Request,
        code: Option<auth::DeviceCode>,
    },
    /// The gateway's code on screen, the state the oracle's snapshot test
    /// starts from (fixtures).
    GatewayDeviceState(auth::DeviceCode),
    /// `account/gatewayOAuth/changed` for an attempt (`login_id`) that
    /// ended.
    GatewayDeviceChanged {
        login_id: String,
        change: device::Change,
    },
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    ChooseUpdate(UpdateChoice),
    /// The first choice of the trust step (trust, or open restricted).
    Trust,
    /// The cancel choice of the trust step.
    CancelTrust,
    /// A slot step finished.
    SlotDone(Stage),
    /// The sign-in step's.
    SignIn(auth::Intent),
    /// Enter on the gateway's failed sign-in: try again.
    RetryGatewayOAuth,
    /// Quit from a step.
    Exit,
    /// The first startup prompt due was put on screen (the page asked it
    /// of the startup screen): it leaves the queue.
    PromptAsked,
    /// The page the command line opened instead of a thread (`codex
    /// resume`'s picker) was left. The app decides: a fresh thread starts
    /// when none is open or on its way (oracle: `SessionSelection::StartFresh`);
    /// the model keeps nothing of it.
    StartPageLeft,
}

/// A server notification the sign-in reads (`handle_app_server_notification`):
/// a login completed, the account changed.
pub fn observe(model: &mut Model, notification: &ServerNotification, fx: &mut AppFx) {
    let ev = match notification {
        ServerNotification::AccountLoginCompleted(done) => Ev::LoginCompleted {
            login_id: done.login_id.clone(),
            success: done.success,
            error: done.error.clone(),
        },
        // `on_account_updated`: `AuthMode::has_chatgpt_account`.
        ServerNotification::AccountUpdated(update) => Ev::AccountUpdated {
            chatgpt_account: matches!(
                update.auth_mode,
                Some(
                    AuthMode::Chatgpt | AuthMode::ChatgptAuthTokens | AuthMode::PersonalAccessToken
                )
            ),
        },
        ServerNotification::GatewayOAuthChanged(change) => {
            // The device login's attempt that ended
            // (`handle_event_during_device_login`).
            if let Some(login_id) = change.login_id.clone() {
                let ended = match change.status {
                    GatewayOAuthStatus::Succeeded => Some(device::Change::Succeeded),
                    GatewayOAuthStatus::Failed | GatewayOAuthStatus::NotReady => {
                        Some(device::Change::Failed(
                            change
                                .error
                                .clone()
                                .unwrap_or_else(|| "Gateway sign-in failed".into()),
                        ))
                    }
                    GatewayOAuthStatus::Started => None,
                };
                if let Some(change) = ended {
                    apply(model, Ev::GatewayDeviceChanged { login_id, change }, fx);
                }
            }
            Ev::GatewayOAuthChanged {
                provider_id: change.provider_id.clone(),
                auth_url: change.auth_url.clone(),
            }
        }
        _ => return,
    };
    apply(model, ev, fx);
}

/// Asks the host for the launch facts once the first backend is up; a new
/// ask replaces one still awaited.
pub fn connected(model: &mut Model, fx: &mut AppFx) {
    if model.read {
        // A launch ask lost with the connection is asked again.
        follow(model, fx);
        return;
    }
    let request = mint(model);
    model.asked = Some(request);
    ask(fx, Ask::Facts { request });
}

/// The connection was lost: a trust write still awaited will not be
/// answered, so the step is open again (`CancelTrust` is not held back by
/// `Saving`) and a late answer to that write is dropped. The same for the
/// update to run: its ack (`UpdateQueued`) is lost with the connection, so
/// the prompt is open again instead of waiting for it for good.
pub fn disconnected(model: &mut Model) {
    model.holding = false;
    model.launching = None;
    model.reloaded = false;
    session::lost(model);
    gateway::lost(model);
    device::lost(model);
    consent::abandon(model);
    if model.writing.take().is_some()
        && let Some(t) = model
            .trust
            .as_mut()
            .filter(|t| t.decision == Some(TrustDecision::Saving))
    {
        t.decision = None;
    }
    if model.running.take().is_some()
        && let Some(u) = model.update.as_mut()
    {
        u.choice = None;
    }
}

fn mint(model: &mut Model) -> Request {
    model.minted += 1;
    model.minted
}

fn ask(fx: &mut AppFx, ask: Ask) {
    fx.commands.push(AppCommand::Startup { ask });
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    apply_ev(model, ev, fx);
    follow(model, fx);
}

/// What follows a change: the selected gateway's sign-in once the steps are
/// over, then the launch.
fn follow(model: &mut Model, fx: &mut AppFx) {
    gateway::follow(model, fx);
    follow_launch(model, fx);
}

fn apply_ev(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    match ev {
        Ev::Launched { request, prompts } => {
            if model.launching != Some(request) {
                return;
            }
            model.launching = None;
            model.launched = true;
            // The cwd prompt of a session named on the command line is
            // already first.
            model.prompts.extend(prompts);
        }
        Ev::OssPick { lmstudio, ollama } => {
            model.holding = true;
            model.prompts.insert(
                0,
                startup::Prompt::Oss(startup::OssPrompt { lmstudio, ollama }),
            );
        }
        Ev::GatewayStarted { request } => gateway::started(model, request, fx),
        Ev::GatewayLogins { request, logins } => gateway::logins(model, request, logins, fx),
        Ev::GatewayOAuth { request, result } => gateway::oauth(model, request, result),
        Ev::GatewayDevice { request, code } => device::answered(model, request, code, fx),
        Ev::GatewayDeviceState(code) => model.device = device::Phase::Waiting(code),
        Ev::GatewayDeviceChanged { login_id, change } => {
            device::changed(model, login_id, change, fx);
        }
        Ev::GatewayOAuthChanged {
            provider_id,
            auth_url,
        } => gateway::changed(model, &provider_id, auth_url, fx),
        Ev::Facts { request, facts } => {
            if model.asked != Some(request) {
                return;
            }
            model.asked = None;
            model.read = true;
            model.holding = false;
            model.update = facts.update;
            model.welcome = facts.welcome;
            model.sign_in = facts.sign_in.map(auth::SignIn::new);
            model.provider = None;
            model.trust = facts.trust;
            model.resume = facts.resume;
            if facts.refresh_update {
                ask(fx, Ask::RefreshUpdateCache);
            }
        }
        Ev::Update {
            current,
            latest,
            command,
        } => {
            model.update = Some(UpdateOffer {
                current,
                latest,
                command,
                choice: None,
            });
        }
        Ev::Welcome(welcome) => model.welcome = Some(welcome),
        Ev::SignIn(offer) => model.sign_in = Some(auth::SignIn::new(offer)),
        Ev::SignInState(state) => {
            if let Some(sign_in) = model.sign_in.as_mut() {
                sign_in.state = state;
            }
            follow_provider(model);
        }
        Ev::Auth(result) => {
            if let Some(sign_in) = model.sign_in.as_mut() {
                let mut asks = Vec::new();
                // The browser opens for the login the page took in, not for
                // an answer it dropped.
                let browser = match &result.outcome {
                    auth::Outcome::Browser { login_id, .. } => Some(login_id.clone()),
                    _ => None,
                };
                let mut minted = model.minted;
                let mut mint = || {
                    minted += 1;
                    minted
                };
                auth::apply_result(sign_in, result, &mut mint, &mut asks);
                model.minted = minted;
                if let (Some(id), auth::State::Browser { login_id, url }) =
                    (browser, &sign_in.state)
                    && *login_id == id
                {
                    asks.push(Ask::OpenBrowser { url: url.clone() });
                }
                asks.into_iter().for_each(|a| ask(fx, a));
            }
            // The wizard's answer ends the provider step (a gateway
            // selected, a provider configured): the page is not asked to.
            follow_provider(model);
        }
        Ev::LoginCompleted {
            login_id,
            success,
            error,
        } => {
            if let Some(sign_in) = model.sign_in.as_mut() {
                auth::login_completed(sign_in, login_id.as_deref(), success, error);
            }
        }
        Ev::AccountUpdated { chatgpt_account } => {
            if let Some(sign_in) = model.sign_in.as_mut() {
                sign_in.chatgpt_account = chatgpt_account;
            }
        }
        Ev::Trust(ask) => model.trust = Some(ask),
        Ev::UpdateQueued { request } => {
            if model.running == Some(request) {
                model.running = None;
                model.update_queued = true;
            }
        }
        Ev::Consent { request, ask } => consent::answered(model, request, ask, None),
        Ev::ConsentAt {
            request,
            ask,
            checked,
        } => consent::answered(model, request, ask, Some(checked)),
        Ev::ConsentFailed { request, message } => consent::failed(model, request, message),
        Ev::TrustSaved { request } => {
            if consent::saved(model, request) {
                return;
            }
            if model.writing != Some(request) {
                return;
            }
            model.writing = None;
            if let Some(t) = model
                .trust
                .as_mut()
                .filter(|t| t.decision == Some(TrustDecision::Saving))
            {
                // The folder is trusted: the project's config applies (the
                // reload is asked with the launch's prompts).
                t.decision = Some(TrustDecision::Trusted);
            }
        }
        // onboarding_screen.rs `persist_selected_trust`: the step stays, with
        // the error, and no choice made.
        Ev::TrustFailed { request, message } => {
            if consent::save_failed(model, request, message.clone()) {
                return;
            }
            if model.writing != Some(request) {
                return;
            }
            model.writing = None;
            if let Some(t) = model
                .trust
                .as_mut()
                .filter(|t| t.decision == Some(TrustDecision::Saving))
            {
                t.decision = None;
                t.error = Some(message);
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    apply_intent_inner(model, intent, fx);
    follow(model, fx);
}

fn apply_intent_inner(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    match intent {
        Intent::StartPageLeft => {}
        Intent::PromptAsked => {
            if !model.prompts.is_empty() {
                model.prompts.remove(0);
            }
        }
        // The first choice ends the prompt: a key still buffered for it (an
        // Enter after Esc) chooses nothing (onboarding_screen.rs:1064).
        Intent::ChooseUpdate(choice) => {
            if let Some(u) = model.update.as_mut().filter(|u| u.choice.is_none()) {
                u.choice = Some(choice);
                match choice {
                    UpdateChoice::UpdateNow => {
                        let request = mint(model);
                        model.running = Some(request);
                        ask(fx, Ask::RunUpdate { request });
                    }
                    UpdateChoice::Skip => {}
                    UpdateChoice::SkipUntilNextVersion => ask(
                        fx,
                        Ask::DismissUpdate {
                            version: u.latest.clone(),
                        },
                    ),
                }
            }
        }
        Intent::Trust => {
            if consent::trust(model, fx) {
                return;
            }
            let Some(t) = model.trust.as_mut().filter(|t| t.decision.is_none()) else {
                return;
            };
            t.error = None;
            // A restricted folder opens as it is; trust is written and
            // waited for (R9).
            if t.restricted {
                t.decision = Some(TrustDecision::Opened);
                return;
            }
            t.decision = Some(TrustDecision::Saving);
            let target = t.target.clone();
            let request = mint(model);
            model.writing = Some(request);
            ask(fx, Ask::WriteTrust { request, target });
        }
        Intent::CancelTrust => {
            if consent::cancel(model) {
                return;
            }
            if let Some(t) = model
                .trust
                .as_mut()
                .filter(|t| t.decision != Some(TrustDecision::Saving))
            {
                t.decision = Some(TrustDecision::Cancelled);
            }
        }
        Intent::SignIn(intent) => {
            let Some(sign_in) = model.sign_in.as_mut() else {
                return;
            };
            let mut minted = model.minted;
            let mut mint = || {
                minted += 1;
                minted
            };
            let mut asks = Vec::new();
            auth::apply_intent(sign_in, intent, &mut mint, &mut asks);
            model.minted = minted;
            asks.into_iter().for_each(|a| ask(fx, a));
            follow_provider(model);
        }
        Intent::SlotDone(Stage::Provider) => {
            if let Some(d) = model.provider.as_mut() {
                d.done = true;
            }
        }
        Intent::SlotDone(Stage::SignIn | Stage::Update | Stage::Trust) => {}
        Intent::RetryGatewayOAuth => gateway::retry(model, fx),
        Intent::Exit => {
            gateway::exit(model, fx);
            device::exit(model, fx);
            // `cancel_auth_if_active`: a login started is cancelled on the
            // way out.
            if let Some(sign_in) = model.sign_in.as_mut().filter(|s| !s.done()) {
                let mut asks = Vec::new();
                auth::apply_intent(sign_in, auth::Intent::Cancel, &mut || 0, &mut asks);
                asks.into_iter().for_each(|a| ask(fx, a));
            }
            model.exit = true;
        }
    }
}

/// The steps are over (the facts read, every step done, the launch going
/// on): the config is reloaded where the oracle reloads it (after a sign-in
/// screen, `lib.rs` "reloaded_config"; after the trust was saved,
/// `consent.directory_trust_persisted`), then the startup prompts due are
/// asked for, once.
fn follow_launch(model: &mut Model, fx: &mut AppFx) {
    if !model.read
        || model.launched
        || model.launching.is_some()
        || model.ends()
        || model.steps().into_iter().any(|s| !model.done(s))
    {
        return;
    }
    if !model.reloaded {
        model.reloaded = true;
        let trusted = model
            .trust
            .as_ref()
            .is_some_and(|t| t.decision == Some(TrustDecision::Trusted));
        if trusted || model.sign_in.is_some() {
            ask(fx, Ask::ReloadConfig);
        }
    }
    // The gateway's device code comes before the prompts
    // (`device_login::start_if_needed` before the models are read).
    if !device::follow(model, fx) {
        return;
    }
    // A session named on the command line is read and its directory decided
    // before the prompts (`lookup_session_target_with_app_server`).
    if matches!(session::follow(model, fx), session::Step::Wait) {
        return;
    }
    let request = mint(model);
    model.launching = Some(request);
    ask(fx, Ask::Launch { request });
}

/// The provider step runs while the sign-in has a provider chosen: it is
/// due from the choice and goes when the sign-in is back on its menu.
fn follow_provider(model: &mut Model) {
    let chosen = model
        .sign_in
        .as_ref()
        .is_some_and(|s| matches!(s.state, auth::State::Provider(_)));
    model.provider = if chosen {
        Some(model.provider.unwrap_or_default())
    } else {
        None
    };
}

/// Per-thread state of this screen: none (the page keeps no thread state,
/// ADR-003).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: none per thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// The serde form of a script's `screen` step for this directory.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum SeedEv {
    Update {
        current: String,
        latest: String,
        command: String,
    },
    Welcome {
        animations: bool,
    },
    SignIn(Option<SeedSignIn>),
    SignInState(SeedSignInState),
    /// The gateway's device code shown.
    GatewayDeviceState {
        url: String,
        code: String,
    },
    Trust(SeedTrust),
}

/// The sign-in's offer; none offers ChatGPT and an API key, no provider.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedSignIn {
    #[serde(default = "yes")]
    chatgpt: bool,
    #[serde(default = "yes")]
    api_key: bool,
    #[serde(default)]
    bedrock: bool,
    #[serde(default)]
    gcp: bool,
    #[serde(default)]
    gateway: bool,
    #[serde(default)]
    env_api_key: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum SeedSignInState {
    Browser {
        login_id: String,
        url: String,
    },
    DeviceCode {
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        code: Option<String>,
    },
    SuccessMessage(()),
    ApiKey {
        #[serde(default)]
        prefill: Option<String>,
    },
    GatewayOauth {
        provider_id: String,
        #[serde(default)]
        url: Option<String>,
    },
    GatewayOauthFailed {
        provider_id: String,
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedTrust {
    cwd: String,
    target: Option<String>,
    #[serde(default)]
    restricted: bool,
    #[serde(default)]
    existing_task: bool,
    #[serde(default)]
    cancel: Option<String>,
    #[serde(default)]
    sandbox_hint: bool,
    #[serde(default)]
    error: Option<String>,
}

/// A script's `screen` step for this directory: the event `value` stands for
/// (`fixtures::script`).
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let bad = |err: String| crate::screens::SeedError::Bad(err);
    let seed: SeedEv = serde_json::from_value(value).map_err(|err| bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Update {
            current,
            latest,
            command,
        } => Ev::Update {
            current,
            latest,
            command,
        },
        SeedEv::Welcome { animations } => Ev::Welcome(Welcome { animations }),
        SeedEv::SignIn(offer) => Ev::SignIn(match offer {
            None => auth::Offer {
                chatgpt: true,
                api_key: true,
                ..auth::Offer::default()
            },
            Some(o) => auth::Offer {
                chatgpt: o.chatgpt,
                api_key: o.api_key,
                providers: auth::Providers {
                    bedrock: o.bedrock,
                    gcp: o.gcp,
                    gateway: o.gateway,
                },
                env_api_key: o.env_api_key,
            },
        }),
        SeedEv::SignInState(state) => Ev::SignInState(match state {
            SeedSignInState::Browser { login_id, url } => auth::State::Browser { login_id, url },
            SeedSignInState::DeviceCode { url, code } => auth::State::DeviceCode {
                request: 0,
                code: url.zip(code).map(|(url, code)| auth::DeviceCode {
                    login_id: "login-1".into(),
                    url,
                    code,
                }),
            },
            SeedSignInState::SuccessMessage(()) => auth::State::SuccessMessage,
            SeedSignInState::ApiKey { prefill } => auth::State::ApiKey { prefill },
            SeedSignInState::GatewayOauth { provider_id, url } => {
                auth::State::GatewayOAuth { provider_id, url }
            }
            SeedSignInState::GatewayOauthFailed {
                provider_id,
                message,
            } => auth::State::GatewayOAuthFailed {
                provider_id,
                message,
            },
        }),
        SeedEv::GatewayDeviceState { url, code } => Ev::GatewayDeviceState(auth::DeviceCode {
            login_id: "login-1".into(),
            url,
            code,
        }),
        SeedEv::Trust(t) => Ev::Trust(TrustAsk {
            target: t.target.unwrap_or_else(|| t.cwd.clone()),
            cwd: t.cwd,
            restricted: t.restricted,
            existing_task: t.existing_task,
            cancel: match t.cancel.as_deref() {
                None | Some("quit") => TrustCancel::Quit,
                Some("agents_overview") => TrustCancel::AgentsOverview,
                Some("current_task") => TrustCancel::CurrentTask,
                Some(other) => return Err(bad(other.to_owned())),
            },
            sandbox_hint: t.sandbox_hint,
            error: t.error,
            decision: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::startup::{OssPrompt, Prompt, Status};
    use super::*;

    fn trust(restricted: bool) -> Ev {
        Ev::Trust(TrustAsk {
            cwd: "/workspace/project".into(),
            target: "/workspace/project".into(),
            restricted,
            ..TrustAsk::default()
        })
    }

    fn update() -> Ev {
        Ev::Update {
            current: "0.0.0".into(),
            latest: "9.9.9".into(),
            command: "npm install -g @openai/codex".into(),
        }
    }

    fn model(evs: Vec<Ev>) -> Model {
        let mut model = Model::default();
        for ev in evs {
            apply(&mut model, ev, &mut AppFx::default());
        }
        model
    }

    fn intent(model: &mut Model, intent: Intent) -> Vec<Ask> {
        let mut fx = AppFx::default();
        apply_intent(model, intent, &mut fx);
        asks(fx)
    }

    fn asks(fx: AppFx) -> Vec<Ask> {
        fx.into_commands()
            .into_iter()
            .filter_map(|command| match command {
                AppCommand::Startup { ask } => Some(ask),
                _ => None,
            })
            .collect()
    }

    fn ev(model: &mut Model, ev: Ev) {
        apply(model, ev, &mut AppFx::default());
    }

    // The order of `run_update_prompt_if_needed` then `OnboardingScreen::new`:
    // update, sign-in, provider, trust; nothing due, no page.
    #[test]
    fn steps_run_in_the_oracle_order() {
        assert!(!Model::default().due());
        let m = model(vec![
            trust(false),
            Ev::SignIn(auth::Offer {
                api_key: true,
                providers: auth::Providers {
                    gcp: true,
                    ..auth::Providers::default()
                },
                ..auth::Offer::default()
            }),
            update(),
        ]);
        assert_eq!(m.steps(), [Stage::Update, Stage::SignIn, Stage::Trust]);
        let mut m = m;
        intent(
            &mut m,
            Intent::SignIn(auth::Intent::UseMethod(auth::Method::Provider(
                auth::Provider::Gcp,
            ))),
        );
        assert_eq!(
            m.steps(),
            [Stage::Update, Stage::SignIn, Stage::Provider, Stage::Trust]
        );
        assert!(m.due());
    }

    // update_prompt.rs:305 update_prompt_confirm_selects_update
    #[test]
    fn update_prompt_confirm_selects_update() {
        let mut m = model(vec![update()]);
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::UpdateNow));
        assert_eq!(
            m.update.as_ref().and_then(|u| u.choice),
            Some(UpdateChoice::UpdateNow)
        );
        // Done once the host holds the update to run.
        answered(&mut m, Ev::UpdateQueued { request: 1 });
        assert!(m.done(Stage::Update));
    }

    // update_prompt.rs:313 update_prompt_dismiss_option_leaves_prompt_in_normal_state
    #[test]
    fn update_prompt_dismiss_option_leaves_prompt_in_normal_state() {
        let mut m = model(vec![update()]);
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::Skip));
        assert_eq!(
            m.update.as_ref().and_then(|u| u.choice),
            Some(UpdateChoice::Skip)
        );
        assert!(!m.due());
    }

    // update_prompt.rs:322 update_prompt_dont_remind_selects_dismissal
    #[test]
    fn update_prompt_dont_remind_selects_dismissal() {
        let mut m = model(vec![update()]);
        intent(
            &mut m,
            Intent::ChooseUpdate(UpdateChoice::SkipUntilNextVersion),
        );
        assert_eq!(
            m.update.as_ref().and_then(|u| u.choice),
            Some(UpdateChoice::SkipUntilNextVersion)
        );
    }

    // update_prompt.rs:332 update_prompt_ctrl_c_skips_update (the view sends
    // Skip for Ctrl+C, Ctrl+D and Esc)
    #[test]
    fn update_prompt_ctrl_c_skips_update() {
        let mut m = model(vec![update(), trust(false)]);
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::Skip));
        assert!(m.done(Stage::Update));
        assert!(!m.done(Stage::Trust));
        assert!(!m.exit);
    }

    // onboarding_screen.rs:1064 buffered_enter_is_discarded_when_trust_step_becomes_active:
    // the Enter that follows Esc in one burst reaches the prompt, not the
    // trust step, and changes nothing there.
    #[test]
    fn buffered_enter_is_discarded_when_trust_step_becomes_active() {
        let mut m = model(vec![update(), trust(false)]);
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::Skip));
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::UpdateNow));
        assert_eq!(
            m.update.as_ref().and_then(|u| u.choice),
            Some(UpdateChoice::Skip)
        );
        assert_eq!(m.trust_write(), None);
        assert!(!m.done(Stage::Trust));
    }

    // onboarding_screen.rs:1130 restricted_acceptance_and_cancellation_do_not_persist_trust
    #[test]
    fn restricted_acceptance_and_cancellation_do_not_persist_trust() {
        let mut m = model(vec![trust(true)]);
        intent(&mut m, Intent::Trust);
        assert_eq!(
            m.trust.as_ref().and_then(|t| t.decision),
            Some(TrustDecision::Opened)
        );
        assert_eq!(m.trust_write(), None);
        assert!(m.done(Stage::Trust));
        let mut m = model(vec![trust(false)]);
        intent(&mut m, Intent::CancelTrust);
        assert_eq!(m.trust_write(), None);
        assert!(m.done(Stage::Trust));
    }

    // onboarding_screen.rs:1162 trust_persistence_failure_keeps_trust_step_in_progress
    #[test]
    fn trust_persistence_failure_keeps_trust_step_in_progress() {
        let mut m = model(vec![trust(false)]);
        let write = intent(&mut m, Intent::Trust);
        assert_eq!(
            write,
            [Ask::WriteTrust {
                request: 1,
                target: "/workspace/project".into()
            }]
        );
        assert_eq!(m.trust_write(), Some("/workspace/project"));
        assert!(!m.done(Stage::Trust));
        ev(
            &mut m,
            Ev::TrustFailed {
                request: 1,
                message: "config/batchWrite failed".into(),
            },
        );
        let t = m.trust.as_ref().expect("trust");
        assert_eq!(t.decision, None);
        assert_eq!(t.error.as_deref(), Some("config/batchWrite failed"));
        assert!(!m.done(Stage::Trust));
        intent(&mut m, Intent::Trust);
        assert_eq!(m.trust.as_ref().and_then(|t| t.error.clone()), None);
        // The first write's answer is stale now: the second one is awaited.
        ev(&mut m, Ev::TrustSaved { request: 1 });
        assert!(!m.done(Stage::Trust));
        ev(&mut m, Ev::TrustSaved { request: 2 });
        assert!(m.done(Stage::Trust));
        assert!(!m.due());
    }

    // The facts are asked once the first backend is up; only the answer to
    // the ask awaited is applied, and a read model asks no more.
    #[test]
    fn launch_facts_are_asked_once_and_stale_answers_dropped() {
        let mut m = Model::default();
        let mut fx = AppFx::default();
        connected(&mut m, &mut fx);
        connected(&mut m, &mut fx);
        assert_eq!(
            asks(fx),
            [Ask::Facts { request: 1 }, Ask::Facts { request: 2 }]
        );
        let facts = Facts {
            sign_in: Some(auth::Offer::default()),
            welcome: Some(Welcome { animations: true }),
            ..Facts::default()
        };
        ev(
            &mut m,
            Ev::Facts {
                request: 1,
                facts: facts.clone(),
            },
        );
        assert!(!m.due(), "the replaced ask's answer is dropped");
        ev(&mut m, Ev::Facts { request: 2, facts });
        assert_eq!(m.steps(), [Stage::SignIn]);
        assert!(m.due());
        let mut fx = AppFx::default();
        connected(&mut m, &mut fx);
        assert_eq!(asks(fx), []);
    }

    // update_prompt.rs:96: "not now" persists nothing, "don't remind me"
    // dismisses the version, "update now" runs it after the app ends.
    #[test]
    fn update_choices_ask_the_host() {
        let mut m = model(vec![update()]);
        assert_eq!(
            intent(
                &mut m,
                Intent::ChooseUpdate(UpdateChoice::SkipUntilNextVersion)
            ),
            [Ask::DismissUpdate {
                version: "9.9.9".into()
            }]
        );
        let mut m = model(vec![update()]);
        assert_eq!(intent(&mut m, Intent::ChooseUpdate(UpdateChoice::Skip)), []);
        let mut m = model(vec![update()]);
        assert_eq!(
            intent(&mut m, Intent::ChooseUpdate(UpdateChoice::UpdateNow)),
            [Ask::RunUpdate { request: 1 }]
        );
    }

    // The update to run is stored by the host before the page leaves (and
    // the app ends): the prompt is not done until the host says it is queued,
    // and only for the run it asked for.
    #[test]
    fn the_update_prompt_waits_for_the_update_to_be_queued() {
        let mut m = model(vec![update()]);
        intent(&mut m, Intent::ChooseUpdate(UpdateChoice::UpdateNow));
        assert!(!m.done(Stage::Update));
        assert!(m.due());
        assert_eq!(answered(&mut m, Ev::UpdateQueued { request: 9 }), []);
        assert!(!m.done(Stage::Update), "a stale answer");
        answered(&mut m, Ev::UpdateQueued { request: 1 });
        assert!(m.done(Stage::Update));
        assert!(!m.due());
    }

    // Skipping needs no host: the prompt is done at once.
    #[test]
    fn skipping_the_update_is_done_at_once() {
        for choice in [UpdateChoice::Skip, UpdateChoice::SkipUntilNextVersion] {
            let mut m = model(vec![update()]);
            intent(&mut m, Intent::ChooseUpdate(choice));
            assert!(m.done(Stage::Update), "{choice:?}");
        }
    }

    fn answered(model: &mut Model, ev: Ev) -> Vec<Ask> {
        let mut fx = AppFx::default();
        apply(model, ev, &mut fx);
        asks(fx)
    }

    // `show_chatgpt_login_in_browser`: the browser shows the login URL of the
    // attempt taken in, and only that one.
    #[test]
    fn the_browser_opens_for_the_login_the_page_took_in() {
        let mut m = model(vec![Ev::SignIn(auth::Offer {
            chatgpt: true,
            ..auth::Offer::default()
        })]);
        let started = intent(
            &mut m,
            Intent::SignIn(auth::Intent::UseMethod(auth::Method::ChatGpt)),
        );
        let [Ask::Login { request, .. }] = started[..] else {
            panic!("a login asked: {started:?}");
        };
        let browser = |request| {
            Ev::Auth(auth::AuthResult {
                request,
                outcome: auth::Outcome::Browser {
                    login_id: "login-1".into(),
                    url: "https://auth.example".into(),
                },
            })
        };
        assert_eq!(
            answered(&mut m, browser(request + 1)),
            [Ask::CancelLogin {
                login_id: "login-1".into()
            }],
            "an answer to an ask not awaited opens no browser; its stale login is cancelled (O1)"
        );
        assert_eq!(
            answered(&mut m, browser(request)),
            [Ask::OpenBrowser {
                url: "https://auth.example".into()
            }]
        );
    }

    #[test]
    fn an_answer_that_is_no_browser_login_opens_no_browser() {
        let mut m = model(vec![Ev::SignIn(auth::Offer {
            chatgpt: true,
            ..auth::Offer::default()
        })]);
        let started = intent(
            &mut m,
            Intent::SignIn(auth::Intent::UseMethod(auth::Method::ChatGpt)),
        );
        let [Ask::Login { request, .. }] = started[..] else {
            panic!("a login asked: {started:?}");
        };
        assert_eq!(
            answered(
                &mut m,
                Ev::Auth(auth::AuthResult {
                    request,
                    outcome: auth::Outcome::Failed(auth::Error::Server("down".into())),
                })
            ),
            []
        );
    }

    // lib.rs: `consent.directory_trust_persisted && !uses_remote_workspace`
    // reloads the config: only a trust that was written and confirmed asks.
    #[test]
    fn a_confirmed_trust_write_asks_for_the_config_reload() {
        let (mut m, _) = read(Facts {
            trust: Some(TrustAsk {
                cwd: "/workspace/project".into(),
                target: "/workspace/project".into(),
                ..TrustAsk::default()
            }),
            ..Facts::default()
        });
        intent(&mut m, Intent::Trust);
        // The reload comes first: the launch's prompts are read from the
        // reloaded config.
        assert_eq!(
            answered(&mut m, Ev::TrustSaved { request: 2 }),
            [
                Ask::ReloadConfig,
                Ask::GatewayDevice {
                    request: 3,
                    thread: None
                }
            ]
        );
        // A repeated or stale answer asks nothing more.
        assert_eq!(answered(&mut m, Ev::TrustSaved { request: 2 }), []);
    }

    /// A model that read `facts` (the ask of the first connection), and
    /// what the answer asked.
    fn read(facts: Facts) -> (Model, Vec<Ask>) {
        let mut m = Model::default();
        connected(&mut m, &mut AppFx::default());
        let asked = answered(&mut m, Ev::Facts { request: 1, facts });
        (m, asked)
    }

    fn hooks_due() -> startup::Prompt {
        startup::Prompt::Hooks(startup::HooksPrompt {
            count: 1,
            trust: startup::Trust::Idle,
            updates: Vec::new(),
        })
    }

    /// The host found no gateway sign-in to show (`Ask::GatewayDevice`,
    /// request `request`): the launch's prompts are asked next.
    fn no_device_code(m: &mut Model, request: Request) -> Vec<Ask> {
        answered(
            m,
            Ev::GatewayDevice {
                request,
                code: None,
            },
        )
    }

    // run_ratatui_app: with no step due the app asks for the gateway's device
    // code (`device_login::start_if_needed`), then reads the launch's prompts
    // (the hooks review, the model migration) before the first frame; the
    // page waits for them and is not settled until they are known.
    #[test]
    fn a_launch_with_no_step_due_asks_for_its_startup_prompts() {
        let (mut m, asked) = read(Facts::default());
        assert_eq!(
            asked,
            [Ask::GatewayDevice {
                request: 2,
                thread: None
            }]
        );
        assert!(m.awaits_launch() && !m.settled() && !m.due());
        assert_eq!(no_device_code(&mut m, 2), [Ask::Launch { request: 3 }]);
        assert!(m.awaits_launch() && !m.settled() && !m.due());
        ev(
            &mut m,
            Ev::Launched {
                request: 3,
                prompts: Vec::new(),
            },
        );
        assert!(!m.awaits_launch() && m.settled() && !m.due());
    }

    #[test]
    fn launched_prompts_keep_the_page_due_until_asked() {
        let (mut m, _) = read(Facts::default());
        no_device_code(&mut m, 2);
        ev(
            &mut m,
            Ev::Launched {
                request: 3,
                prompts: vec![hooks_due()],
            },
        );
        assert!(m.due());
        assert_eq!(intent(&mut m, Intent::PromptAsked), []);
        assert!(m.prompts.is_empty() && !m.due() && m.settled());
    }

    fn oss_pick() -> Ev {
        Ev::OssPick {
            lmstudio: Status::NotRunning,
            ollama: Status::NotRunning,
        }
    }

    // `--oss` with both or no local server: the host asks before the config exists. The panel is
    // the first prompt and the page is due without any fact.
    #[test]
    fn oss_pick_opens_the_page_with_the_panel_first() {
        let mut m = Model::default();
        assert!(!m.due() && !m.holding());
        ev(&mut m, oss_pick());
        assert!(m.due() && m.settled() && m.holding());
        assert_eq!(
            m.prompts,
            [Prompt::Oss(OssPrompt {
                lmstudio: Status::NotRunning,
                ollama: Status::NotRunning,
            })]
        );
        // The prompts of the launch come after it.
        ev(
            &mut m,
            Ev::Launched {
                request: 1,
                prompts: Vec::new(),
            },
        );
        assert!(matches!(m.prompts.first(), Some(Prompt::Oss(_))));
    }

    // The panel answered and the queue empty, the page stays: the steps the facts bring come
    // next.
    #[test]
    fn the_page_is_held_from_the_pick_until_the_facts() {
        let mut m = Model::default();
        ev(&mut m, oss_pick());
        assert_eq!(intent(&mut m, Intent::PromptAsked), []);
        assert!(m.prompts.is_empty() && m.due() && m.holding());
        let mut fx = AppFx::default();
        connected(&mut m, &mut fx);
        assert_eq!(asks(fx), [Ask::Facts { request: 1 }]);
        assert!(m.holding());
        ev(
            &mut m,
            Ev::Facts {
                request: 1,
                facts: Facts::default(),
            },
        );
        assert!(!m.holding() && !m.due());
    }

    #[test]
    fn a_lost_connection_releases_the_hold() {
        let mut m = Model::default();
        ev(&mut m, oss_pick());
        disconnected(&mut m);
        assert!(!m.holding());
    }

    // A launch ask lost with the connection is asked again; the answer to
    // the lost one is dropped.
    #[test]
    fn a_launch_ask_lost_with_the_connection_is_asked_again() {
        let (mut m, _) = read(Facts::default());
        no_device_code(&mut m, 2);
        disconnected(&mut m);
        let mut fx = AppFx::default();
        connected(&mut m, &mut fx);
        assert_eq!(asks(fx), [Ask::Launch { request: 4 }]);
        ev(
            &mut m,
            Ev::Launched {
                request: 3,
                prompts: vec![hooks_due()],
            },
        );
        assert!(!m.launched() && m.prompts.is_empty());
        ev(
            &mut m,
            Ev::Launched {
                request: 4,
                prompts: Vec::new(),
            },
        );
        assert!(m.launched());
    }

    // The launch's prompts are read once the steps are over, after the
    // sign-in's reload; a launch that ends with its steps reads none.
    #[test]
    fn the_launch_waits_for_the_steps_and_skips_a_launch_that_ends() {
        let (m, asked) = read(Facts {
            sign_in: Some(auth::Offer::default()),
            ..Facts::default()
        });
        assert_eq!(asked, []);
        assert!(!m.awaits_launch());
        let (mut m2, _) = read(Facts {
            trust: Some(TrustAsk {
                cwd: "/w".into(),
                target: "/w".into(),
                ..TrustAsk::default()
            }),
            ..Facts::default()
        });
        assert_eq!(intent(&mut m2, Intent::CancelTrust), []);
        assert!(m2.settled() && !m2.awaits_launch());
    }

    #[test]
    fn a_trust_that_was_not_written_asks_no_config_reload() {
        let mut m = model(vec![trust(false)]);
        intent(&mut m, Intent::Trust);
        let failed = Ev::TrustFailed {
            request: 1,
            message: "no".into(),
        };
        assert_eq!(answered(&mut m, failed), []);
        assert_eq!(answered(&mut m, Ev::TrustSaved { request: 1 }), []);
        let mut m = model(vec![trust(true)]);
        assert_eq!(intent(&mut m, Intent::Trust), []);
        let mut m = model(vec![trust(false)]);
        assert_eq!(intent(&mut m, Intent::CancelTrust), []);
    }

    // updates.rs `get_upgrade_version`: a missing or stale cache is
    // refreshed in the background, once, when the facts arrive.
    #[test]
    fn facts_that_found_the_update_cache_stale_ask_for_a_refresh() {
        let mut m = Model::default();
        let mut fx = AppFx::default();
        connected(&mut m, &mut fx);
        let facts = |refresh_update| Ev::Facts {
            request: 1,
            facts: Facts {
                refresh_update,
                ..Facts::default()
            },
        };
        // (No step is due: the launch's prompts are asked after it.)
        assert_eq!(
            answered(&mut m, facts(true)),
            [
                Ask::RefreshUpdateCache,
                Ask::GatewayDevice {
                    request: 2,
                    thread: None
                }
            ]
        );
        // The answer was taken: a repeat asks nothing more.
        assert_eq!(answered(&mut m, facts(true)), []);
        let mut m = Model::default();
        connected(&mut m, &mut AppFx::default());
        assert_eq!(
            answered(&mut m, facts(false)),
            [Ask::GatewayDevice {
                request: 2,
                thread: None
            }]
        );
    }

    // A quit from a step ends the page and the app (`should_exit`).
    // onboarding_screen.rs `handle_key_event`: a quit while a login runs
    // cancels it on the way out (`cancel_auth_if_active`).
    #[test]
    fn exit_cancels_the_login_in_progress() {
        let mut m = model(vec![Ev::SignIn(auth::Offer {
            chatgpt: true,
            ..auth::Offer::default()
        })]);
        let started = intent(
            &mut m,
            Intent::SignIn(auth::Intent::UseMethod(auth::Method::ChatGpt)),
        );
        let [Ask::Login { request, .. }] = started[..] else {
            panic!("a login asked: {started:?}");
        };
        ev(
            &mut m,
            Ev::Auth(auth::AuthResult {
                request,
                outcome: auth::Outcome::Browser {
                    login_id: "login-1".into(),
                    url: "https://auth.example".into(),
                },
            }),
        );
        assert_eq!(
            intent(&mut m, Intent::Exit),
            [Ask::CancelLogin {
                login_id: "login-1".into(),
            }]
        );
        assert!(m.exit);
    }

    #[test]
    fn exit_ends_every_step() {
        let mut m = model(vec![update(), trust(false)]);
        intent(&mut m, Intent::Exit);
        assert!(m.exit);
        assert!(!m.due());
    }

    #[test]
    fn seeds_read_the_fixture_forms() {
        let ev = seed(
            serde_json::json!({"trust": {"cwd": "/a/b", "target": "/a", "cancel": "current_task"}}),
        );
        assert_eq!(
            ev,
            Ok(Ev::Trust(TrustAsk {
                cwd: "/a/b".into(),
                target: "/a".into(),
                cancel: TrustCancel::CurrentTask,
                ..TrustAsk::default()
            }))
        );
        assert_eq!(
            seed(serde_json::json!({"welcome": {"animations": true}})),
            Ok(Ev::Welcome(Welcome { animations: true }))
        );
        assert_eq!(
            seed(serde_json::json!({"sign_in": null})),
            Ok(Ev::SignIn(auth::Offer {
                chatgpt: true,
                api_key: true,
                ..auth::Offer::default()
            }))
        );
    }

    #[test]
    fn a_seed_shows_the_gateways_device_code() {
        let Ok(seeded) = seed(serde_json::json!({"gateway_device_state": {
            "url": "https://login.example.com/device",
            "code": "ABCD-EFGH",
        }})) else {
            panic!("the seed was refused");
        };
        let mut m = Model::default();
        ev(&mut m, seeded);
        let code = m.device_code().expect("a code");
        assert_eq!(code.code, "ABCD-EFGH");
        assert!(m.due(), "the page opens for it");
    }

    #[test]
    fn seeds_read_the_gateway_sign_in_states() {
        assert_eq!(
            seed(
                serde_json::json!({"sign_in_state": {"gateway_oauth": {"provider_id": "office"}}})
            ),
            Ok(Ev::SignInState(auth::State::GatewayOAuth {
                provider_id: "office".into(),
                url: None,
            }))
        );
        assert_eq!(
            seed(serde_json::json!({"sign_in_state": {"gateway_oauth": {
                "provider_id": "office",
                "url": "https://office.example.test/authorize",
            }}})),
            Ok(Ev::SignInState(auth::State::GatewayOAuth {
                provider_id: "office".into(),
                url: Some("https://office.example.test/authorize".into()),
            }))
        );
        assert_eq!(
            seed(
                serde_json::json!({"sign_in_state": {"gateway_oauth_failed": {
                    "provider_id": "office",
                    "message": "Gateway sign-in expired.",
                }}})
            ),
            Ok(Ev::SignInState(auth::State::GatewayOAuthFailed {
                provider_id: "office".into(),
                message: "Gateway sign-in expired.".into(),
            }))
        );
    }
}
