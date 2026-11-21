//! `onboarding` (S01-S04, the startup page): the core half of the screen's
//! directory. The facts the startup page runs on (an update offered, the
//! sign-in due, the folder trust to ask) and what the user chose, changed
//! only by `apply`; the view half is `src/pages/startup/onboarding/` in the UI
//! crate. Oracle: `update_prompt.rs` (S01), `onboarding/onboarding_screen.rs`
//! (which steps show, in what order), `onboarding/welcome.rs`,
//! `onboarding/trust_directory.rs` and `onboarding/directory_trust.rs` (S04).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use serde::Deserialize;

use crate::app::AppFx;
use crate::app::Command as AppCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
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
    /// S02: the sign-in step is due (`show_login_screen`).
    pub sign_in: Option<Decided>,
    /// S03: the provider step is due.
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
    /// The last request number minted.
    minted: Request,
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
    /// Do not offer `version` again (`updates::dismiss_version`); not answered.
    DismissUpdate { version: String },
    /// Run the offered update once the app has ended (`UpdateAction`).
    RunUpdate,
}

/// The launch facts the host read, all at once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub update: Option<UpdateOffer>,
    pub welcome: Option<Welcome>,
    pub sign_in: bool,
    pub provider: bool,
    pub trust: Option<TrustAsk>,
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
            Stage::Update => self.update.as_ref().is_none_or(|u| u.choice.is_some()),
            Stage::SignIn => self.sign_in.is_none_or(|d| d.done),
            Stage::Provider => self.provider.is_none_or(|d| d.done),
            Stage::Trust => self.trust.as_ref().is_none_or(|t| {
                matches!(
                    t.decision,
                    Some(TrustDecision::Trusted | TrustDecision::Opened | TrustDecision::Cancelled)
                )
            }),
        }
    }

    /// Whether the app opens on the startup page: a step is not finished.
    pub fn due(&self) -> bool {
        self.steps().into_iter().any(|step| !self.done(step))
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
    SignInDue,
    ProviderDue,
    Trust(TrustAsk),
    /// The answer to `Ask::Facts { request }`.
    Facts { request: Request, facts: Facts },
    /// The trust write `request` is confirmed.
    TrustSaved { request: Request },
    /// The trust write `request` failed with this message.
    TrustFailed { request: Request, message: String },
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
    /// Quit from a step.
    Exit,
}

/// Asks the host for the launch facts once the first backend is up; a new
/// ask replaces one still awaited.
pub fn connected(model: &mut Model, fx: &mut AppFx) {
    if model.read {
        return;
    }
    let request = mint(model);
    model.asked = Some(request);
    ask(fx, Ask::Facts { request });
}

fn mint(model: &mut Model) -> Request {
    model.minted += 1;
    model.minted
}

fn ask(fx: &mut AppFx, ask: Ask) {
    fx.commands.push(AppCommand::Startup { ask });
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Facts { request, facts } => {
            if model.asked != Some(request) {
                return;
            }
            model.asked = None;
            model.read = true;
            model.update = facts.update;
            model.welcome = facts.welcome;
            model.sign_in = facts.sign_in.then(Decided::default);
            model.provider = facts.provider.then(Decided::default);
            model.trust = facts.trust;
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
        Ev::SignInDue => model.sign_in = Some(Decided::default()),
        Ev::ProviderDue => model.provider = Some(Decided::default()),
        Ev::Trust(ask) => model.trust = Some(ask),
        Ev::TrustSaved { request } => {
            if model.writing != Some(request) {
                return;
            }
            model.writing = None;
            if let Some(t) = model.trust.as_mut().filter(|t| t.decision == Some(TrustDecision::Saving)) {
                t.decision = Some(TrustDecision::Trusted);
            }
        }
        // onboarding_screen.rs `persist_selected_trust`: the step stays, with
        // the error, and no choice made.
        Ev::TrustFailed { request, message } => {
            if model.writing != Some(request) {
                return;
            }
            model.writing = None;
            if let Some(t) = model.trust.as_mut().filter(|t| t.decision == Some(TrustDecision::Saving)) {
                t.decision = None;
                t.error = Some(message);
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    match intent {
        // The first choice ends the prompt: a key still buffered for it (an
        // Enter after Esc) chooses nothing (onboarding_screen.rs:1064).
        Intent::ChooseUpdate(choice) => {
            if let Some(u) = model.update.as_mut().filter(|u| u.choice.is_none()) {
                u.choice = Some(choice);
                match choice {
                    UpdateChoice::UpdateNow => ask(fx, Ask::RunUpdate),
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
            if let Some(t) = model.trust.as_mut().filter(|t| t.decision != Some(TrustDecision::Saving)) {
                t.decision = Some(TrustDecision::Cancelled);
            }
        }
        Intent::SlotDone(Stage::SignIn) => {
            if let Some(d) = model.sign_in.as_mut() {
                d.done = true;
            }
        }
        Intent::SlotDone(Stage::Provider) => {
            if let Some(d) = model.provider.as_mut() {
                d.done = true;
            }
        }
        Intent::SlotDone(Stage::Update | Stage::Trust) => {}
        Intent::Exit => model.exit = true,
    }
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
    SignIn(()),
    Provider(()),
    Trust(SeedTrust),
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
        SeedEv::SignIn(()) => Ev::SignInDue,
        SeedEv::Provider(()) => Ev::ProviderDue,
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
        let m = model(vec![trust(false), Ev::SignInDue, update(), Ev::ProviderDue]);
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
        assert!(m.done(Stage::Update));
        assert_eq!(
            m.update.as_ref().and_then(|u| u.choice),
            Some(UpdateChoice::UpdateNow)
        );
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
        assert_eq!(
            t.error.as_deref(),
            Some("config/batchWrite failed")
        );
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
            sign_in: true,
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
            intent(&mut m, Intent::ChooseUpdate(UpdateChoice::SkipUntilNextVersion)),
            [Ask::DismissUpdate {
                version: "9.9.9".into()
            }]
        );
        let mut m = model(vec![update()]);
        assert_eq!(intent(&mut m, Intent::ChooseUpdate(UpdateChoice::Skip)), []);
        let mut m = model(vec![update()]);
        assert_eq!(
            intent(&mut m, Intent::ChooseUpdate(UpdateChoice::UpdateNow)),
            [Ask::RunUpdate]
        );
    }

    // A quit from a step ends the page and the app (`should_exit`).
    #[test]
    fn exit_ends_every_step() {
        let mut m = model(vec![update(), trust(false)]);
        intent(&mut m, Intent::Exit);
        assert!(m.exit);
        assert!(!m.due());
    }

    #[test]
    fn seeds_read_the_fixture_forms() {
        let ev = seed(serde_json::json!({"trust": {"cwd": "/a/b", "target": "/a", "cancel": "current_task"}}));
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
        assert_eq!(seed(serde_json::json!({"sign_in": null})), Ok(Ev::SignInDue));
    }
}
