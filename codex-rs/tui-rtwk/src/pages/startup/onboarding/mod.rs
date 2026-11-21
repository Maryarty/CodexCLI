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

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, FlexFit, Flexible, MainAxisSize, SizedBox};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::Mods;
use rtwk::provider::{read, watch};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::onboarding::{
    self as core, ID, Intent, Model, NAME, TrustCancel, TrustDecision, UpdateChoice,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::app::theme::Tokens;
use crate::app::view::Hs;

mod signin;
pub mod strings;
mod trust;
mod update_prompt;
mod welcome;

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
        entry!("sign_in_env_key", "sign-in: OPENAI_API_KEY set"),
        entry!("api_key_entry", "sign-in: the API key entry"),
        entry!(
            "browser_long_url",
            "sign-in: ChatGPT in the browser, a long link"
        ),
        entry!("success_message", "sign-in: signed in with ChatGPT"),
        entry!("device_code", "sign-in: the device code shown"),
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
        }
    }

    /// The model's step this one is; none for a step the model does not run.
    fn model(self) -> Option<core::Stage> {
        match self {
            Step::Update => Some(core::Stage::Update),
            Step::SignIn => Some(core::Stage::SignIn),
            Step::Provider => Some(core::Stage::Provider),
            Step::Trust => Some(core::Stage::Trust),
            Step::Migration | Step::Import => None,
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
fn overview(model: &Model) -> bool {
    model.trust.as_ref().is_some_and(|t| {
        t.decision == Some(TrustDecision::Cancelled) && t.cancel == TrustCancel::AgentsOverview
    })
}

/// The startup page, opened at `step`.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupPage {
    pub step: Step,
}

impl StartupPage {
    /// The page as the app opens on it: at the first step due.
    pub fn launch() -> StartupPage {
        StartupPage { step: Step::Update }
    }
}

/// The page's step controller (R14: widget state only): the step it is on
/// (none once past the last) and the logo's replays.
#[derive(Debug)]
pub struct StartupPageState {
    at: Option<Step>,
    replay: u32,
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
        }
    }
}

impl_widget!(stateful eq StartupPage);

impl State for StartupPageState {
    type Widget = StartupPage;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &StartupPage, cx: &mut Cx<'_, Self>) {
        let Some(at) = self.at else {
            return;
        };
        let next = current(at, &watch::<ScreenStates, _>(cx).onboarding);
        self.at = next;
        if next.is_none() {
            let leave = cx.send(Msg::Leave);
            cx.emit(&leave, ());
        }
    }

    fn update(&mut self, msg: Msg, _: &StartupPage, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Leave => {
                let model = &watch::<ScreenStates, _>(cx).onboarding;
                let quit = quits(model);
                // The trust prompt cancelled at launch opens the agents
                // overview instead of the folder (lib.rs: `should_exit`).
                if overview(model) {
                    drop(Navigator::push_replacement::<(), _>(
                        cx,
                        crate::pages::agents::start_route(),
                    ));
                } else {
                    Navigator::maybe_pop(cx);
                }
                if quit {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.quit, ());
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
        let screens = watch::<ScreenStates, _>(cx);
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
            Some(Step::Trust) => match &model.trust {
                Some(ask) => trust::Trust { ask: ask.clone() }.into(),
                None => AnyWidget::default(),
            },
            // The provider step is the sign-in step's widget too, so the
            // menu's highlight outlives it (Esc there comes back to it).
            Some(step @ (Step::SignIn | Step::Provider)) => match &model.sign_in {
                Some(sign_in) => signin::SignInStep {
                    sign_in: sign_in.clone(),
                }
                .into(),
                None => Slot {
                    step: step.model().unwrap_or(core::Stage::SignIn),
                }
                .into(),
            },
            Some(Step::Import) => crate::pages::startup::import::Screen.into(),
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
        let faded = model
            .sign_in
            .as_ref()
            .is_some_and(codex_tui_core::screens::onboarding::auth::SignIn::is_text_entry);
        let page = LayoutBuilder::new(move |_, constraints| {
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
        });
        // The logo's replay key (oracle `keys::TOGGLE_ANIMATION`).
        let mut focus = Focus::new(page);
        if welcome_model.animations {
            for mods in [Mods::CTRL, Mods::CTRL | Mods::SHIFT] {
                focus = focus.bind(KeyPress::char('.').with_mods(mods), cx.send(Msg::Replay));
            }
        }
        focus.into()
    }
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
        Focus::new(SizedBox::new(None, Some(0), None))
            .autofocus()
            .bind(KeyPress::ENTER, cx.send(()))
            .into()
    }
}

/// The route of the startup page on `step`, pushed by the widget that opens
/// it (`Navigator::of(cx).push`).
pub fn route(step: Step) -> PageRoute<()> {
    PageRoute::new(move |_| StartupPage { step }.into())
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
