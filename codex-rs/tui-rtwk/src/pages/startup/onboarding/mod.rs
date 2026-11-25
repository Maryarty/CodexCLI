//! `onboarding` (S01-S04): the startup page (architecture §1, amendment
//! 2026-11-19b). One page whose body is the step its controller is on: the
//! update prompt, sign-in and provider under the welcome header, then
//! trust; model migration and import are steps of it too. Oracle:
//! `onboarding/onboarding_screen.rs`, `update_prompt.rs`, `welcome.rs`,
//! `onboarding/trust_directory.rs`.
//!
//! In Flutter: a `Stepper`-like page whose `State` is the controller: the
//! step it is on, moved on as the model (core's
//! `codex_tui_core::screens::onboarding`, watched) marks a step done; past
//! the last one the page pops to the route under it (the session page),
//! quitting the app where the oracle exits. One widget per step; a slot
//! step (sign-in, provider: the `onboarding` task's) says it is done with
//! `Intent::SlotDone`.

use rtwk::focus::Focus;
use rtwk::foundation::ValueListenableBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, FlexFit, Flexible, MainAxisSize, SizedBox};
use rtwk::navigation::{Navigator, PageRoute, PopInvoked};
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::import::Phase as ImportPhase;
use codex_tui_core::screens::onboarding::{
    self as core, ID, Intent, Model, NAME, TrustCancel, TrustDecision, UpdateChoice,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;

mod device;
mod signin;
pub mod strings;
mod trust;
mod update_prompt;
mod welcome;

/// The GovCloud guidance's view (S03): the launch's startup prompt
/// `Prompt::GovCloud` draws it.
pub use signin::provider::gov_cloud::GovCloudGuidance;

/// What the debug menu lists for this directory: the startup page on each
/// state of its oracle tests.
pub fn debug_entries() -> Vec<DebugEntry> {
    macro_rules! entry {
        ($name:literal, $description:literal) => {
            DebugEntry {
                name: $name,
                description: $description,
                script: include_str!(concat!("fixtures/", $name, ".jsonl")),
                open: EntryOpen::Route(|_: &Seeded| StartupPage::launch().into()),
            }
        };
    }
    vec![
        entry!("update_prompt", "the update prompt (npm)"),
        entry!(
            "update_prompt_windows",
            "the update prompt, the Windows command"
        ),
        entry!("welcome", "the welcome header over the sign-in slot"),
        entry!(
            "flow_update_trust",
            "the update prompt, then the trust step"
        ),
        entry!(
            "trust_git_repo",
            "trust: a Git repository, the sandbox hint"
        ),
        entry!("trust_keep_current", "trust: keep the current directory"),
        entry!(
            "trust_remote_subdirectory",
            "trust: a subdirectory of a remote Git root"
        ),
        entry!("trust_restricted", "trust: a restricted folder"),
        entry!(
            "trust_existing_task",
            "trust: an existing task in a restricted folder"
        ),
        entry!("trust_error", "trust: the trust write failed"),
        entry!(
            "trust_restricted_long",
            "trust: a long restricted path at 40 cells"
        ),
        entry!(
            "trust_long_checkout",
            "trust: a checkout path too long for its rows"
        ),
        entry!(
            "trust_long_root",
            "trust: a subdirectory of a long repository root"
        ),
        entry!("sign_in_menu", "sign-in: ChatGPT and an API key"),
        entry!("sign_in_forced_chatgpt", "sign-in: ChatGPT login forced"),
        entry!("sign_in_bedrock", "sign-in: the Bedrock wizard offered"),
        entry!(
            "sign_in_cloud",
            "sign-in: the Bedrock and Google Cloud wizards offered"
        ),
        entry!("sign_in_gateway", "sign-in: every provider wizard offered"),
        entry!(
            "gateway_oauth_starting",
            "sign-in: the gateway's own sign-in starting"
        ),
        entry!(
            "gateway_oauth_browser",
            "sign-in: the gateway's own sign-in in the browser"
        ),
        entry!(
            "gateway_oauth_failed",
            "sign-in: the gateway's own sign-in failed"
        ),
        entry!("sign_in_env_key", "sign-in: OPENAI_API_KEY set"),
        entry!(
            "bedrock_gov_cloud",
            "the GovCloud guidance at launch (startup prompt)"
        ),
        entry!("api_key_entry", "sign-in: the API key entry"),
        entry!(
            "browser_long_url",
            "sign-in: ChatGPT in the browser, a long link"
        ),
        entry!("success_message", "sign-in: signed in with ChatGPT"),
        entry!("device_code", "sign-in: the device code shown"),
        entry!("gateway_device_code", "the gateway's device code at launch"),
        entry!(
            "device_code_requesting",
            "sign-in: the device code requested"
        ),
    ]
}

/// The steps of the startup page, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    Update,
    SignIn,
    Provider,
    Trust,
    Migration,
    Import,
    /// The gateway's device code (`run_gateway_device_login`), shown once
    /// the steps above are over until its attempt ends.
    Device,
    /// The launch's startup prompts (S05), asked one after the other once
    /// the steps above are over: the page shows the open one.
    Prompts,
    /// The folder trust of a `/cd`, asked over the chat after the launch (not a step of the
    /// launch: the model's `consent` holds it).
    Consent,
}

impl Step {
    /// The position of the step in the page's sequence.
    pub fn index(self) -> usize {
        match self {
            Step::Update => 0,
            Step::SignIn => 1,
            Step::Provider => 2,
            Step::Trust => 3,
            Step::Migration => 4,
            Step::Import => 5,
            Step::Device => 6,
            Step::Prompts => 7,
            Step::Consent => 8,
        }
    }

    /// The model's step this one is; none for a step the model does not run.
    fn model(self) -> Option<core::Stage> {
        match self {
            Step::Update => Some(core::Stage::Update),
            Step::SignIn => Some(core::Stage::SignIn),
            Step::Provider => Some(core::Stage::Provider),
            Step::Trust => Some(core::Stage::Trust),
            Step::Migration | Step::Import | Step::Device | Step::Prompts | Step::Consent => None,
        }
    }

    fn of(step: core::Stage) -> Step {
        match step {
            core::Stage::Update => Step::Update,
            core::Stage::SignIn => Step::SignIn,
            core::Stage::Provider => Step::Provider,
            core::Stage::Trust => Step::Trust,
        }
    }
}

/// The step the controller is on: `at` if it is not done, else the first
/// step after it that is not; none past the last. A step the model does not
/// run stays until the page is left.
fn current(at: Step, model: &Model) -> Option<Step> {
    let Some(step) = at.model() else {
        return Some(at);
    };
    let steps = model.steps();
    let from = steps.iter().position(|s| *s == step).unwrap_or(0);
    steps[from..]
        .iter()
        .copied()
        .find(|s| !model.done(*s))
        .map(Step::of)
}

/// Whether leaving the page quits the app (the oracle exits: a quit, an
/// update to run, a trust prompt cancelled by quitting).
fn quits(model: &Model) -> bool {
    model.exit
        || model
            .update
            .as_ref()
            .is_some_and(|u| u.choice == Some(UpdateChoice::UpdateNow))
        || model.trust.as_ref().is_some_and(|t| {
            t.decision == Some(TrustDecision::Cancelled) && t.cancel == TrustCancel::Quit
        })
}

/// Whether leaving the page opens the agents overview: the trust prompt was
/// cancelled with that as its cancel.
pub(crate) fn overview(model: &Model) -> bool {
    model.trust.as_ref().is_some_and(|t| {
        t.decision == Some(TrustDecision::Cancelled) && t.cancel == TrustCancel::AgentsOverview
    })
}

/// How the startup page tells the home it is gone (Flutter: the `onDone` of
/// a route): the home waits for it to mount the chat.
#[derive(Clone, Debug, PartialEq)]
pub struct Left {
    /// The page popped back to the home.
    pub page: Handler<()>,
    /// The page was replaced by the command center (the trust prompt
    /// cancelled at launch) and the center popped: it is the page the home
    /// waited on from then on (`PopScope.onPopInvokedWithResult`).
    pub center: Handler<PopInvoked>,
}

/// The startup page, opened at `step`.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupPage {
    pub step: Step,
    /// Told once the page is gone: the home mounts the chat then.
    pub left: Option<Left>,
}

impl StartupPage {
    /// The page as the app opens on it: at the first step due.
    pub fn launch() -> StartupPage {
        StartupPage {
            step: Step::Update,
            left: None,
        }
    }
}

/// The page's step controller (R14: widget state only): the step it is on
/// (none once past the last) and the logo's replays.
#[derive(Debug)]
pub struct StartupPageState {
    at: Option<Step>,
    replay: u32,
    /// The import flow's request count when the page opened: the import step
    /// is over once the flow is done at a later one.
    import_from: u64,
    /// The startup prompts' answers when the page opened: an exit among the
    /// later ones quits the app.
    answers_from: usize,
    /// A startup prompt was asked and is not on screen yet.
    asking: bool,
    /// Told by the gateway wizard whether its form's mark rests on Save (the logo then keeps
    /// its colour).
    resting: signin::Resting,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    Leave,
    Replay,
}

impl StatefulWidget for StartupPage {
    type State = StartupPageState;

    fn create_state(&self) -> StartupPageState {
        StartupPageState {
            at: Some(self.step),
            replay: 0,
            import_from: 0,
            answers_from: 0,
            asking: false,
            resting: signin::Resting::default(),
        }
    }
}

impl_widget!(stateful eq StartupPage);

impl State for StartupPageState {
    type Widget = StartupPage;
    type Msg = Msg;

    fn init_state(&mut self, _: &StartupPage, cx: &mut Cx<'_, Self>) {
        let screens = read::<ScreenStates, _>(cx);
        self.import_from = screens.import.generation();
        self.answers_from = screens.startup.answers.len();
    }

    fn did_change_dependencies(&mut self, _: &StartupPage, cx: &mut Cx<'_, Self>) {
        let Some(at) = self.at else {
            return;
        };
        let screens = slice(cx);
        let next = match at {
            // the import step (`/import`) is run by the import flow: the page
            // goes on, past its last step, when the flow it opened is done
            Step::Import => {
                let done = screens.import_phase == ImportPhase::Done
                    && screens.import_generation > self.import_from;
                (!done).then_some(Step::Import)
            }
            // the question of a `/cd` is asked while the model holds it open
            Step::Consent => core::consent::open(&screens.onboarding).map(|_| Step::Consent),
            Step::Device | Step::Prompts => self.after_steps(&screens, cx),
            _ => match current(at, &screens.onboarding) {
                Some(step) => Some(step),
                // Past the steps: the gateway's device code, then the
                // launch's startup prompts.
                None => self.after_steps(&screens, cx),
            },
        };
        self.at = next;
        if next.is_none() {
            let leave = cx.send(Msg::Leave);
            cx.emit(&leave, ());
        }
    }

    fn update(&mut self, msg: Msg, w: &StartupPage, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Leave => {
                let screens = read::<ScreenStates, _>(cx);
                let model = &screens.onboarding;
                let answers = screens.startup.answers.get(self.answers_from..);
                // The question of a `/cd` only leaves: the launch's own outcome (a quit, the
                // command center) is not its.
                if w.step == Step::Consent {
                    Navigator::maybe_pop(cx);
                    return Rebuild::No;
                }
                let quit = quits(model) || answers.is_some_and(|a| a.iter().any(|a| exits(*a)));
                // A quit leaves from the page: no frame of what lies under it.
                if quit {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.quit, ());
                    return Rebuild::No;
                }
                // The trust prompt cancelled at launch opens the agents
                // overview instead of the folder (lib.rs: `should_exit`): the
                // center is the page the home waits on now.
                if overview(model) {
                    let center = match &w.left {
                        Some(left) => crate::pages::agents::start_route_then(left.center.clone()),
                        None => crate::pages::agents::start_route(),
                    };
                    drop(Navigator::push_replacement::<(), _>(cx, center));
                } else {
                    Navigator::maybe_pop(cx);
                    if let Some(left) = &w.left {
                        cx.emit(&left.page, ());
                    }
                }
                Rebuild::No
            }
            Msg::Replay => {
                self.replay = self.replay.wrapping_add(1);
                Rebuild::Yes
            }
        }
    }

    fn build(&self, _: &StartupPage, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let screens = slice(cx);
        let model = &screens.onboarding;
        let strings = strings::of(cx);
        let bold = Tokens::of(cx).bold();
        let body: AnyWidget = match self.at {
            None => return AnyWidget::default(),
            Some(Step::Update) => match &model.update {
                // The update prompt is a screen of its own, no header.
                Some(offer) => {
                    return update_prompt::UpdatePrompt {
                        offer: offer.clone(),
                    }
                    .into();
                }
                None => AnyWidget::default(),
            },
            Some(Step::Consent) => match core::consent::open(model) {
                Some(ask) => trust::Trust { ask: ask.clone() }.into(),
                None => AnyWidget::default(),
            },
            Some(Step::Trust) => match &model.trust {
                Some(ask) => trust::Trust { ask: ask.clone() }.into(),
                None => AnyWidget::default(),
            },
            // The provider step is the sign-in step's widget too, so the
            // menu's highlight outlives it (Esc there comes back to it).
            Some(step @ (Step::SignIn | Step::Provider)) => match &model.sign_in {
                Some(sign_in) => signin::SignInStep {
                    sign_in: sign_in.clone(),
                    resting: self.resting.clone(),
                }
                .into(),
                None => Slot {
                    step: step.model().unwrap_or(core::Stage::SignIn),
                }
                .into(),
            },
            // The device code is a screen of its own, no welcome header.
            Some(Step::Device) => match model.device_code() {
                Some(code) => {
                    return device::GatewayDevice { code: code.clone() }.into();
                }
                None => AnyWidget::default(),
            },
            Some(Step::Import) => crate::pages::startup::import::Screen.into(),
            Some(Step::Prompts) => crate::pages::startup::prompts::Panel { embedded: true }.into(),
            Some(Step::Migration) => Screen.into(),
        };
        // The welcome heads the sign-in screen only: the trust prompt at
        // launch is a screen of its own (`check_directory_trust`).
        let welcome = model
            .welcome
            .filter(|_| matches!(self.at, Some(Step::SignIn | Step::Provider)));
        let Some(welcome_model) = welcome else {
            return body;
        };
        let replay = self.replay;
        // The logo goes while a login's link or code is shown
        // (`should_suppress_animations`: `Presentation::Hidden`).
        let frozen = model
            .sign_in
            .as_ref()
            .is_some_and(codex_tui_core::screens::onboarding::auth::SignIn::freezes_animations);
        // A text entry open (`text_entry_context().active`) fades it
        // (`Presentation::Faded`).
        // (not while the gateway form's mark rests on Save: the wizard says so).
        let typing = model
            .sign_in
            .as_ref()
            .is_some_and(codex_tui_core::screens::onboarding::auth::SignIn::is_text_entry);
        let page =
            ValueListenableBuilder::new(self.resting.0.clone(), move |_, resting: &bool, _| {
                let faded = typing && !*resting;
                let (bold, body) = (bold.clone(), body.clone());
                LayoutBuilder::new(move |_, constraints| {
                    let logo = welcome::shows_logo(
                        welcome_model.animations && !frozen,
                        constraints.max_width,
                        constraints.max_height,
                    );
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(welcome::header(strings, bold.clone(), logo, replay, faded))
                        .child(Flexible {
                            fit: FlexFit::Loose,
                            ..Flexible::new(1, body.clone())
                        })
                        .into()
                })
                .into()
            });
        // The logo's replay key (oracle `keys::TOGGLE_ANIMATION`).
        let mut focus = Focus::new(page);
        if welcome_model.animations {
            focus = bind_fixed(focus, fixed::REPLAY_LOGO, cx.send(Msg::Replay));
        }
        focus.into()
    }
}

impl StartupPageState {
    /// Past the steps (`run_ratatui_app`): the gateway's device code while
    /// it is shown, then the launch's startup prompts.
    fn after_steps(&mut self, screens: &Slice, cx: &mut Cx<'_, Self>) -> Option<Step> {
        if screens.onboarding.device_code().is_some() {
            return Some(Step::Device);
        }
        self.prompts(screens, cx)
    }

    /// The launch's startup prompts step (oracle `run_ratatui_app`: the
    /// hooks review, then the model migration as the app starts): on while a
    /// prompt is open or due, or while the host reads which are due; it asks
    /// the first one due of the startup screen. None once they are answered
    /// (or for a launch that ends with its steps, or a fixture that read no
    /// facts).
    fn prompts(&mut self, screens: &Slice, cx: &mut Cx<'_, Self>) -> Option<Step> {
        let model = &screens.onboarding;
        if screens.prompt_asked {
            self.asking = false;
            return Some(Step::Prompts);
        }
        if self.asking || model.awaits_launch() {
            return Some(Step::Prompts);
        }
        // The recovery menu was cancelled: the host ends the launch with the error it read.
        if screens.daemon_ended {
            return Some(Step::Prompts);
        }
        let first = model.prompts.first()?.clone();
        self.asking = true;
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Startup(codex_tui_core::screens::startup::Intent::Ask(first)),
        );
        cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::PromptAsked));
        Some(Step::Prompts)
    }
}

/// What the page reads of the screen states: the onboarding model, whether a startup prompt is
/// asked, and the import flow's phase and generation (a `select` of these, not the whole of
/// `ScreenStates`).
#[derive(Clone, PartialEq)]
struct Slice {
    onboarding: core::Model,
    prompt_asked: bool,
    /// The recovery menu was cancelled (the app ends on the host's error).
    daemon_ended: bool,
    import_phase: ImportPhase,
    import_generation: u64,
}

fn slice(cx: &mut Cx<'_, StartupPageState>) -> Slice {
    select::<ScreenStates, Slice, _>(cx, |states| Slice {
        onboarding: states.onboarding.clone(),
        prompt_asked: states.startup.asked.is_some(),
        daemon_ended: states.startup.answers.iter().any(|a| {
            matches!(
                a,
                codex_tui_core::screens::startup::Answer::Daemon(
                    codex_tui_core::screens::startup::DaemonChoice::Cancel
                ) | codex_tui_core::screens::startup::Answer::DaemonQuit
            )
        }),
        import_phase: states.import.phase,
        import_generation: states.import.generation(),
    })
}

/// Whether a startup prompt's answer ends the app (Ctrl-C on a prompt, the
/// migration's exit, the unarchive prompt's quit). The recovery menu's Cancel and Ctrl-C end
/// it too, but with the host's error text (`Ask::DaemonChoice`): the page waits for it.
fn exits(answer: codex_tui_core::screens::startup::Answer) -> bool {
    use codex_tui_core::screens::startup::{Answer, MigrationOutcome, UnarchiveChoice};
    matches!(
        answer,
        Answer::CwdExit
            | Answer::OssCancelled
            | Answer::Migration(MigrationOutcome::Exit)
            | Answer::Unarchive(UnarchiveChoice::Quit)
    )
}

/// A step another task fills (sign-in, provider: task `onboarding`): it
/// draws nothing yet, and Enter marks it done.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub step: core::Stage,
}

/// Widget state of a slot step: none.
pub struct SlotState;

impl StatefulWidget for Slot {
    type State = SlotState;

    fn create_state(&self) -> SlotState {
        SlotState
    }
}

impl_widget!(stateful eq Slot);

impl State for SlotState {
    type Widget = Slot;
    type Msg = ();

    fn update(&mut self, (): (), w: &Slot, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Onboarding(Intent::SlotDone(w.step)),
        );
        Rebuild::No
    }

    fn build(&self, _: &Slot, cx: &mut Cx<'_, Self>) -> AnyWidget {
        bind_fixed(
            Focus::new(SizedBox::new(None, Some(0), None)).autofocus(),
            fixed::ENTER,
            cx.send(()),
        )
        .into()
    }
}

/// The route of the startup page on `step`, pushed by the widget that opens
/// it (`Navigator::of(cx).push`).
pub fn route(step: Step) -> PageRoute<()> {
    PageRoute::new(move |_| StartupPage { step, left: None }.into())
}

/// The page at `step`, telling `left` once it is gone.
pub fn route_then(step: Step, left: Left) -> PageRoute<()> {
    PageRoute::new(move |_| {
        StartupPage {
            step,
            left: Some(left.clone()),
        }
        .into()
    })
}

/// Opens the startup page on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| StartupPage::launch().into())
}

/// The empty form of the model-migration step (not built yet).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: none (R14).
#[derive(Debug)]
pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let shared = crate::l10n::strings::of(cx);
        Text::new(shared.empty_slot(NAME, ID).to_string()).into()
    }
}
