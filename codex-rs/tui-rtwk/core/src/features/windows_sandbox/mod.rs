//! The Windows sandbox prompts and the elevated setup as a feature slice (ADR-012; oracle
//! `windows_sandbox.rs`, `chatwidget/windows_sandbox_prompts.rs`, `app/platform_actions.rs`,
//! `/setup-default-sandbox`). Core keeps what IO and the surface need to agree on: the sandbox
//! configuration and managed requirements the host read, where the executors are, the setup that
//! is running (one at a time), and the prompt that is *due* (a one-shot the surface shows and says
//! it showed). The prompts' choices are data (`Page`, ADR-010: the words are the view's); the
//! degraded-sandbox label and the `/setup-default-sandbox` hint are facts (`State::degraded`,
//! `State::elevate_hint`) the permissions page reads. The setup itself is the server's
//! (`windowsSandbox/setupStart`; the end comes as a `windowsSandbox/setupCompleted` fact).
//! Words and sheets: `src/features/windows_sandbox/`.

use codex_app_server_protocol::{
    ClientRequest, ConfigReadResponse, ConfigRequirementsReadResponse, RequestId,
    WindowsSandboxImplementation, WindowsSandboxSetupMode, WindowsSandboxSetupStartParams,
};
use codex_protocol::config_types::WindowsSandboxLevel;

use super::{AppCx, PermissionSetup, PublicVerb, Scope};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::screens::select::permissions::{Label, Preset, Reviewer, Selection};
use crate::threads::Tone;
use crate::threads::queue::ModalToken;

mod seed;

/// The marker of the feature (`feature_list!`).
pub struct WindowsSandbox;

pub type Mode = WindowsSandboxSetupMode;

/// What `/setup-default-sandbox` names (`Feature::OPENS`, `Target::Feature`): no sheet is shown,
/// the view's `open` says the verb `Intent::Elevate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

/// Where the session's executors are (oracle `WindowsSandboxHost`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Host {
    Local,
    Mixed,
    Remote,
    #[default]
    Unknown,
}

/// The sandbox configuration and the managed requirements (oracle `WindowsSandboxConfig`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub mxc: bool,
    pub mode: Option<Mode>,
    /// `None`: the policy was not loaded; a loaded `None` list allows both modes.
    pub requirements: Option<Option<Vec<Mode>>>,
}

impl Config {
    /// `WindowsSandboxConfig::from_responses`.
    pub fn from_responses(
        config: &ConfigReadResponse,
        requirements: ConfigRequirementsReadResponse,
    ) -> Self {
        let configured = config
            .config
            .additional
            .get("windows")
            .and_then(|windows| windows.get("sandbox"));
        let mxc = configured.and_then(|value| serde_json::from_value(value.clone()).ok())
            == Some(WindowsSandboxImplementation::Mxc);
        let from_features = || {
            let features = config.config.additional.get("features")?;
            [
                ("elevated_windows_sandbox", Mode::Elevated),
                ("experimental_windows_sandbox", Mode::Unelevated),
                ("enable_experimental_windows_sandbox", Mode::Unelevated),
            ]
            .into_iter()
            .find_map(|(key, mode)| {
                (features.get(key).and_then(serde_json::Value::as_bool) == Some(true))
                    .then_some(mode)
            })
        };
        let mut state = Self {
            mxc,
            mode: configured
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .or_else(|| if mxc { None } else { from_features() }),
            requirements: Some(
                requirements
                    .requirements
                    .and_then(|requirements| requirements.allowed_windows_sandbox_implementations)
                    .map(|allowed| {
                        allowed
                            .into_iter()
                            .filter_map(|implementation| match implementation {
                                WindowsSandboxImplementation::Elevated => Some(Mode::Elevated),
                                WindowsSandboxImplementation::Unelevated => Some(Mode::Unelevated),
                                WindowsSandboxImplementation::Mxc => None,
                            })
                            .collect()
                    }),
            ),
        };
        if !state.mxc
            && let Some(Some(allowed)) = &state.requirements
            && !state.mode.is_some_and(|mode| allowed.contains(&mode))
        {
            // Managed requirements prefer elevated when the configured value is disallowed.
            state.mode = [Mode::Elevated, Mode::Unelevated]
                .into_iter()
                .find(|mode| allowed.contains(mode));
        }
        state
    }

    pub fn is_enabled(&self) -> bool {
        self.mxc || self.mode.is_some()
    }

    pub fn level(&self) -> WindowsSandboxLevel {
        match self.mode {
            Some(Mode::Elevated) => WindowsSandboxLevel::Elevated,
            Some(Mode::Unelevated) => WindowsSandboxLevel::RestrictedToken,
            None => WindowsSandboxLevel::Disabled,
        }
    }

    pub fn allows(&self, mode: Mode) -> bool {
        match &self.requirements {
            Some(Some(allowed)) => allowed.contains(&mode),
            Some(None) => true,
            None => false,
        }
    }

    pub fn requires_elevated(&self) -> bool {
        self.mode == Some(Mode::Elevated) && matches!(self.requirements, Some(Some(_)))
    }
}

/// The two prompts (oracle `open_windows_sandbox_enable_prompt`, `..._fallback_prompt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompt {
    /// Set the sandbox up (the first time, or when a requirement asks for it).
    Enable,
    /// The elevated setup failed.
    Fallback,
}

/// A row of a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The default (admin) sandbox: `Begin(Elevated)`.
    Elevated,
    /// The non-admin sandbox: `Begin(Unelevated)`; the row needs an explicit confirmation.
    Legacy,
    /// Leaves Codex (the view's: the app's quit).
    Quit,
}

impl Choice {
    /// The setup the row asks for (`Quit` asks none: it is the view's).
    pub fn mode(self) -> Option<Mode> {
        match self {
            Choice::Elevated => Some(Mode::Elevated),
            Choice::Legacy => Some(Mode::Unelevated),
            Choice::Quit => None,
        }
    }

    /// The row is picked with a second press (`require_explicit_confirmation`).
    pub fn explicit_confirmation(self) -> bool {
        self == Choice::Legacy
    }
}

/// What a prompt shows (data; the words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub prompt: Prompt,
    /// Only the default sandbox may be used: the header says the organization requires it.
    pub required_by_organization: bool,
    pub choices: Vec<Choice>,
    /// Esc does not leave the prompt: it comes back until a choice is made.
    pub choice_required: bool,
}

/// What was chosen the prompt was for: the preset the sandbox is set up for and the profile
/// selection waiting on it (oracle `(ApprovalPreset, Option<PermissionProfileSelection>)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub mode: Mode,
    /// The built-in permission profile id the setup is for (`auto`).
    pub preset: String,
}

/// What the host tells the feature unasked (and a script seeds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// Where the sandbox stands, read at start and again on a thread switch.
    Environment {
        config: Config,
        host: Host,
        /// A local app server owns the setup (embedded or daemon).
        local_server: bool,
        /// The elevated setup is complete (`windowsSandbox/readiness` said ready).
        elevated_ready: bool,
    },
    /// `windowsSandbox/setupCompleted`.
    Completed {
        mode: Mode,
        success: bool,
        error: Option<String>,
    },
    /// The folder was trusted: the first prompt is due (`prompt_after_trust`).
    Trusted,
}

impl Fact {
    /// The sandbox as the server's config and requirements say it (`Config::from_responses`),
    /// for a local or a remote workspace.
    pub fn environment(
        config: &ConfigReadResponse,
        requirements: ConfigRequirementsReadResponse,
        remote_workspace: bool,
        elevated_ready: bool,
    ) -> Fact {
        Fact::Environment {
            config: Config::from_responses(config, requirements),
            host: if remote_workspace {
                Host::Remote
            } else {
                Host::Local
            },
            local_server: !remote_workspace,
            elevated_ready,
        }
    }

    /// `windowsSandbox/setupCompleted`.
    pub fn completed(
        done: &codex_app_server_protocol::WindowsSandboxSetupCompletedNotification,
    ) -> Fact {
        Fact::Completed {
            mode: done.mode,
            success: done.success,
            error: done.error.clone(),
        }
    }
}

/// The prompt that is due, asked of the surface once (`Scope::Thread` of the thread in use).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Prompt(Prompt),
    OwnedPrompt {
        prompt: Prompt,
        token: ModalToken<Selection>,
    },
}

/// What a surface or a command says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/setup-default-sandbox`.
    Elevate,
    /// The prompt due was shown.
    Shown(Prompt),
    /// The setup is wanted for `mode` (a prompt's row, `Choice::mode`; the permissions page:
    /// `Ask for approval` with no sandbox).
    Begin(Mode),
    /// `Begin`, for a `/permissions` pick the sandbox was the missing piece of (oracle
    /// `open_windows_sandbox_enable_prompt(preset)`): when the setup ends well the pick is
    /// applied (`PublicVerb::PickPermissions`), when it fails or is refused nothing is. The
    /// fallback prompt after an elevated failure keeps the pick for its own choice.
    BeginThen {
        mode: Mode,
        then: Selection,
    },
    /// `/permissions` picked a preset the sandbox is the missing piece of
    /// (`State::pick_needs_setup`; oracle `OpenWindowsSandboxEnablePrompt { preset }`): the
    /// enable prompt is due and the pick waits on it. The prompt's own `Begin(mode)` then sets
    /// the sandbox up and the pick is applied when it ends well, as `BeginThen` does.
    Prompt(Selection),
    PromptOwned(OwnedPrompt),
    BeginOwned {
        token: ModalToken<Selection>,
        mode: Mode,
    },
    CancelOwnedSetup(ModalToken<Selection>),
}

/// The permission hold whose concrete selection stays with its app owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedPrompt(pub(super) ModalToken<Selection>);

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Start(Mode),
}

/// The asks of the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// `windowsSandbox/setupStart` for `cwd`.
    Start { mode: Mode, cwd: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Started(bool),
}

/// What the feature says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `That Windows sandbox option is disallowed by requirements.` (info)
    Disallowed,
    /// `Windows sandbox setup is still running. Restart Codex to retry.` (info)
    StillRunning,
    /// `Update the local app server to set up the Windows sandbox.`
    UpdateServer,
    /// `Windows sandbox setup failed.`
    Failed,
    /// `Windows sandbox setup did not start.`
    NotStarted,
    /// `Windows sandbox setup failed: {error}` (the non-admin setup).
    SetupFailed(String),
    /// The elevated setup is not for this machine's executors.
    Mixed,
}

/// The state: the sandbox's facts, the setup that runs and the prompt that is due.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    config: Option<Config>,
    host: Host,
    local_server: bool,
    elevated_ready: bool,
    pending: Option<Pending>,
    /// The setup request went out (`setup_started_at`).
    started: bool,
    trusted_prompt: bool,
    due: Option<Prompt>,
    /// The prompt the surface was asked to show for the due one (asked once).
    wished: Option<Prompt>,
    /// The last setup ended with this mode.
    last: Option<(Mode, bool)>,
    /// The permissions pick the setup is for (`Intent::BeginThen`).
    then: Option<Selection>,
    then_owner: Option<ModalToken<Selection>>,
}

impl State {
    fn config(&self) -> Config {
        self.config.clone().unwrap_or_default()
    }

    /// `windows_degraded_sandbox_enabled`: the non-admin sandbox on a local executor. The
    /// permissions page names its `Ask for approval` row `(non-admin sandbox)` then, and the
    /// `/setup-default-sandbox` command is listed.
    pub fn degraded(&self) -> bool {
        self.config().level() == WindowsSandboxLevel::RestrictedToken
            && self.local_server
            && self.host == Host::Local
    }

    /// `show_elevate_sandbox_hint`: the permissions page hints at `/setup-default-sandbox`.
    pub fn elevate_hint(&self) -> bool {
        self.degraded() && self.config().allows(Mode::Elevated)
    }

    /// `elevated_windows_sandbox_setup_required`.
    pub fn elevated_setup_required(&self) -> bool {
        self.config().requires_elevated() && (self.host != Host::Local || !self.elevated_ready)
    }

    /// The setup is not done yet and the first prompt would be shown (`maybe_prompt_windows_
    /// sandbox_enable`).
    pub fn setup_required(&self) -> bool {
        let config = self.config();
        !config.is_enabled() || self.elevated_setup_required()
    }

    /// The setup is running: the composer is disabled and `Setting up sandbox...` is the status
    /// (`show_windows_sandbox_setup_status`).
    pub fn setting_up(&self) -> bool {
        self.pending.is_some() && self.started
    }

    /// A setup waits for its end (a thread switch is held back, `windows_sandbox_blocks_thread_
    /// switch`).
    pub fn blocks_thread_switch(&self) -> bool {
        self.local_setup() && (self.pending.is_some() || self.started)
    }

    fn local_setup(&self) -> bool {
        self.local_server && self.host == Host::Local
    }

    /// The prompt that is due, if any: the surface shows it and says `Intent::Shown`.
    pub fn due(&self) -> Option<Prompt> {
        self.due
    }

    /// How the last setup ended.
    pub fn last(&self) -> Option<(Mode, bool)> {
        self.last
    }

    /// A `/permissions` pick is the sandbox's to hold until the sandbox is set up (oracle
    /// `approval_preset_actions`): the Auto preset on a Windows host that is `Mixed` or
    /// `Unknown`, or a local one with no sandbox when the approvals go to the user; a remote
    /// host owns its pick. Not Windows (the environment was never read), nothing is held.
    pub fn pick_needs_setup(&self, selection: &Selection) -> bool {
        // the legacy list picks the preset, the catalog (every thread's list) the profile of its
        // mode row: both are the Auto preset (`profile_selection`)
        let (Selection::Preset {
            preset: Preset::Auto,
            reviewer,
        }
        | Selection::Profile {
            label:
                Label::Mode {
                    preset: Preset::Auto,
                    reviewer,
                },
            ..
        }) = selection
        else {
            return false;
        };
        let Some(config) = &self.config else {
            return false;
        };
        match self.host {
            Host::Mixed | Host::Unknown => true,
            Host::Remote => false,
            Host::Local => *reviewer == Reviewer::User && !config.is_enabled(),
        }
    }

    /// The permissions pick that is applied when the setup ends well.
    pub fn then(&self) -> Option<&Selection> {
        self.then.as_ref()
    }

    /// The setup that runs.
    pub fn pending(&self) -> Option<&Pending> {
        self.pending.as_ref()
    }

    /// The configuration read (none: not read, or not Windows).
    pub fn sandbox(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    /// What a prompt shows (`open_windows_sandbox_*_prompt`).
    pub fn page(&self, prompt: Prompt) -> Page {
        let config = self.config();
        let unelevated = config.allows(Mode::Unelevated);
        let mut choices = Vec::new();
        if config.allows(Mode::Elevated) {
            choices.push(Choice::Elevated);
        }
        if unelevated {
            choices.push(Choice::Legacy);
        }
        choices.push(Choice::Quit);
        Page {
            prompt,
            required_by_organization: !unelevated,
            choices,
            choice_required: !unelevated || self.elevated_setup_required(),
        }
    }

    fn environment(
        &mut self,
        config: Config,
        host: Host,
        local_server: bool,
        ready: bool,
        cx: &mut AppCx<'_>,
    ) {
        self.config = Some(config);
        self.host = host;
        self.local_server = local_server;
        self.elevated_ready = ready;
        if self.local_setup() && std::mem::take(&mut self.trusted_prompt) {
            self.maybe_prompt(cx);
        }
    }

    fn maybe_prompt(&mut self, cx: &mut AppCx<'_>) {
        if self.setup_required() {
            // A startup prompt starts its own setup.
            self.cancel_then(cx);
            self.due = Some(Prompt::Enable);
        }
    }

    /// The prompt that is due is the surface's to show, once.
    fn wish_due(&mut self, cx: &mut AppCx<'_>) {
        if let Some(prompt) = self.due
            && self.wished != Some(prompt)
        {
            if let Some(token) = &self.then_owner {
                cx.wish(
                    token.thread().clone(),
                    Wish::OwnedPrompt {
                        prompt,
                        token: token.clone(),
                    },
                );
            } else {
                let thread = cx.view().active().cloned().unwrap_or_default();
                cx.wish(Scope::Thread(thread), Wish::Prompt(prompt));
            }
            self.wished = Some(prompt);
        }
    }

    pub fn owns_prompt(&self, token: &ModalToken<Selection>) -> bool {
        self.then_owner.as_ref() == Some(token)
    }

    fn finish_then(&mut self, success: bool, cx: &mut AppCx<'_>) {
        let selection = self.then.take().filter(|_| success);
        if let Some(token) = self.then_owner.take() {
            cx.verb(PublicVerb::PermissionSetup(PermissionSetup::Finished {
                token,
                success,
            }));
        } else if let Some(selection) = selection {
            cx.verb(PublicVerb::PickPermissions(selection));
        }
    }

    fn cancel_then(&mut self, cx: &mut AppCx<'_>) {
        self.finish_then(false, cx);
    }

    pub(super) fn clear_owned(&mut self, token: &ModalToken<Selection>) {
        if self.owns_prompt(token) {
            self.then_owner = None;
            self.then = None;
            self.due = None;
            self.wished = None;
        }
    }

    pub(super) fn start_owned(
        &mut self,
        token: &ModalToken<Selection>,
        mode: Mode,
        cx: &mut AppCx<'_>,
    ) {
        if self.owns_prompt(token) {
            self.begin(mode, None, cx);
            if self.pending.is_none() {
                self.cancel_then(cx);
            }
        }
    }

    fn begin(&mut self, mode: Mode, then: Option<Selection>, cx: &mut AppCx<'_>) {
        if self.pending.is_some() {
            if !self.started {
                cx.note(None, Note::StillRunning);
            }
            return;
        }
        if self.host == Host::Mixed {
            cx.note(None, Note::Mixed);
            return;
        }
        if !self.config().allows(mode) {
            cx.note(None, Note::Disallowed);
            return;
        }
        let owner = self.then_owner.as_ref().map(|token| token.thread().clone());
        let cwd = cx
            .view()
            .working_directory(owner.as_ref().or_else(|| cx.view().active()));
        self.pending = Some(Pending {
            mode,
            preset: "auto".to_owned(),
        });
        self.started = true;
        self.due = None;
        self.wished = None;
        if then.is_some() {
            self.then = then;
        }
        cx.ask(
            owner.as_ref(),
            Wanted::Start { mode, cwd },
            Ctx::Start(mode),
        );
    }

    fn ended(&mut self, mode: Mode, success: bool, error: Option<String>, cx: &mut AppCx<'_>) {
        // Another setup's end (a second client's) is not this one's.
        let Some(pending) = self.pending.clone() else {
            return;
        };
        if pending.mode != mode {
            return;
        }
        self.last = Some((mode, success));
        if success {
            let config = self.config.get_or_insert_with(Config::default);
            config.mode = Some(mode);
            self.elevated_ready |= mode == Mode::Elevated;
            self.pending = None;
            self.started = false;
            self.finish_then(true, cx);
        } else if mode == Mode::Elevated {
            // The elevated setup failed: the fallback prompt, the setup stays the pending one's
            // (the choice retries it).
            self.pending = None;
            self.started = false;
            self.due = Some(Prompt::Fallback);
        } else {
            self.pending = None;
            self.started = false;
            self.cancel_then(cx);
            cx.note(
                None,
                Note::SetupFailed(error.unwrap_or_else(|| "unknown error".to_owned())),
            );
        }
    }
}

feature! {
    impl Feature for WindowsSandbox {
        const NAME: &'static str = "windows_sandbox";
        const SEED: Option<fn(serde_json::Value) -> Result<Fact, SeedError>> = Some(seed::seed);
        const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::ElevateSandbox, Sheet));
        type State = State;
        type Target = Sheet;
        type Intent = Intent;
        type Command = Wanted;
        type Reply = Reply;
        type Ctx = Ctx;
        type Note = Note;
        type Fact = Fact;
        type Wish = Wish;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    // Listed only while the degraded sandbox is on; a direct dispatch outside it is
                    // nothing (oracle: "guard anyway").
                    Intent::Elevate => {
                        if state.degraded() {
                            state.cancel_then(cx);
                            state.begin(Mode::Elevated, None, cx);
                        }
                    }
                    Intent::Prompt(then) => {
                        state.cancel_then(cx);
                        state.then = Some(then);
                        state.due = Some(Prompt::Enable);
                        state.wished = None;
                        state.wish_due(cx);
                    }
                    Intent::PromptOwned(OwnedPrompt(token)) => {
                        if state.pending.is_some() {
                            cx.verb(PublicVerb::PermissionSetup(PermissionSetup::Finished { token, success: false }));
                        } else {
                            state.cancel_then(cx);
                            state.then_owner = Some(token);
                            state.due = Some(Prompt::Enable);
                            state.wished = None;
                            state.wish_due(cx);
                        }
                    }
                    Intent::BeginOwned { token, mode } if state.owns_prompt(&token) => {
                        cx.verb(PublicVerb::PermissionSetup(PermissionSetup::Begin { token, mode }));
                    }
                    Intent::CancelOwnedSetup(token) if state.owns_prompt(&token) => state.cancel_then(cx),
                    Intent::Begin(mode) if state.then_owner.is_none() => state.begin(mode, None, cx),
                    Intent::BeginThen { mode, then } if state.then_owner.is_none() => state.begin(mode, Some(then), cx),
                    Intent::BeginOwned { .. } | Intent::CancelOwnedSetup(_) | Intent::Begin(_) | Intent::BeginThen { .. } => {},
                    Intent::Shown(prompt) => {
                        if state.due == Some(prompt) {
                            state.due = None;
                            state.wished = None;
                        }
                    }
                }
            }

            fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
                let Ctx::Start(_) = ctx;
                match result {
                    Ok(Reply::Started(true)) => {}
                    // Transport lost or deserialization failed: the setup may still end; wait for it.
                    Err(RpcError::Malformed(_)) => cx.note(None, Note::Failed),
                    Err(RpcError::Remote(message)) if message.contains("-32601") => {
                        state.pending = None;
                        state.started = false;
                        state.cancel_then(cx);
                        cx.note(None, Note::UpdateServer);
                    }
                    Err(_) => {
                        state.pending = None;
                        state.started = false;
                        state.cancel_then(cx);
                        cx.note(None, Note::Failed);
                    }
                    Ok(Reply::Started(false)) => {
                        state.pending = None;
                        state.started = false;
                        state.cancel_then(cx);
                        cx.note(None, Note::NotStarted);
                    }
                }
            }

            fn on_fact(state: &mut State, fact: Fact, cx: &mut AppCx<'_>) {
                match fact {
                    Fact::Environment {
                        config,
                        host,
                        local_server,
                        elevated_ready,
                    } => state.environment(config, host, local_server, elevated_ready, cx),
                    Fact::Completed {
                        mode,
                        success,
                        error,
                    } => state.ended(mode, success, error, cx),
                    Fact::Trusted => {
                        state.trusted_prompt = true;
                        if state.config.is_some() && state.local_setup() {
                            state.trusted_prompt = false;
                            state.maybe_prompt(cx);
                        }
                    }
                }
                // The prompt that fell due is the surface's to show, once.
                state.wish_due(cx);
            }

            fn on_shown(state: &mut State, thread: &ThreadId, cx: &mut AppCx<'_>) {
                if state.then_owner.as_ref().is_some_and(|token| token.thread() != thread) {
                    state.cancel_then(cx);
                }
            }

            fn note_tone(note: &Note) -> Tone {
                match note {
                    Note::Disallowed | Note::StillRunning | Note::Mixed => Tone::Info,
                    _ => Tone::Error,
                }
            }

            fn wire(command: &Wanted, _thread: &ThreadId, id: RequestId) -> ClientRequest {
                let Wanted::Start { mode, cwd } = command;
                // The directory is an absolute path (the server's type); a relative one is left out.
                let params = serde_json::from_value(serde_json::json!({ "mode": mode, "cwd": cwd }))
                    .unwrap_or(WindowsSandboxSetupStartParams {
                        mode: *mode,
                        cwd: None,
                    });
                ClientRequest::WindowsSandboxSetupStart {
                    request_id: id,
                    params,
                }
            }

            fn decode(command: &Wanted, result: serde_json::Value) -> Result<Reply, RpcError> {
                let Wanted::Start { .. } = command;
                serde_json::from_value::<codex_app_server_protocol::WindowsSandboxSetupStartResponse>(
                    result,
                )
                .map(|response| Reply::Started(response.started))
                .map_err(|error| RpcError::Malformed(error.to_string()))
            }
        }
    }
}
