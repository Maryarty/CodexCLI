//! `UiApp`, the model of the rtwk app: the core model plus the state only the
//! UI has. `core` is `codex_tui_core::app::CodexApp`, plain data with one
//! writer (`apply_app`) and no rtwk; `ui` holds what the widgets keep across
//! view runs and `apply` never sees (R2, R14): the `Navigator`'s key, the
//! per-thread controllers, the language, the debug flag. The shell's `Link`
//! is the third part: the app's end of the effect runner.

use codex_tui_core::app::AppEv;
use codex_tui_core::app::CodexApp;
use codex_tui_core::app::FirstThread;
use codex_tui_core::app::Ingress;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::kit::vim::VimMode;
use codex_tui_core::screens::chat::DesktopNotice;
use rtwk::GlobalKey;
use rtwk::app::AppCx;
use rtwk::editing::TextSelection;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod consent;
pub mod debug;
mod export_text;
pub mod presenters;
pub mod run;
pub mod start_page;
mod terminal_effects;
pub mod theme;
pub mod view;

use crate::l10n::Lang;
use crate::pages::session::transcript::copy::CopyLog;
use crate::services::shell;
use crate::sheets::presenter;
use crate::sheets::presenter::Presenter;
use crate::widgets::thread_ui::ThreadUiMap;

/// The text of a launch that failed, for the host to print once the terminal is restored
/// (the oracle's "Unsent draft"): written before the app ends, as the host's other slots.
pub type UnsentSlot = Arc<Mutex<Option<String>>>;

/// What only the UI keeps: never seen by `apply`, never in the core model.
pub struct UiState {
    /// Keeps the `Navigator`'s stack across view runs (ADR-002, G15).
    pub nav_key: GlobalKey,
    /// Per-thread UI controllers (R2, R14).
    pub thread_ui: ThreadUiMap,
    /// The language of the text, negotiated once at start (ADR-010).
    pub lang: Lang,
    /// Open the debug menu at start (`--debug-screens`).
    pub debug: bool,
    /// Open the agent command center over the chat at start (`codex-rtwk
    /// agents`).
    pub start_agents: bool,
    /// Open the resume picker over the chat at start (`codex-rtwk resume`).
    pub start_resume: bool,
    /// The first thread waits for the host's startup checks and prompts.
    pub awaits_facts: bool,
    /// The host expects a regular local launch: show the draft during the startup checks.
    pub(super) provisional_draft: bool,
    /// The request presenter's model (ADR-002): the kept handle of the
    /// navigator's home and which request is open.
    pub presenter: Presenter,
    /// The presenters of the features' wishes (ADR-013 Ж).
    pub presenters: presenters::Presenters,
    /// How many backend switches the composers followed.
    pub switches_seen: u32,
    /// The thread the composers followed last: a thread new to them starts with its register
    /// (`restore_kill_buffer_snapshot` on a new chat widget).
    pub followed: Option<ThreadId>,
    /// The copies the reader asked of the transcript and what came of them (the gap above the
    /// composer words it): the UI's follower of the answers core keeps.
    pub copies: CopyLog,
    pub(super) desktop_notice: Option<Rc<DesktopNotice>>,
    /// Where the unsent text goes when the launch fails (`UiApp::with_unsent_slot`).
    unsent: Option<UnsentSlot>,
    startup_failure: Option<Arc<Mutex<Option<String>>>>,
}

/// The model of the rtwk app.
pub struct UiApp {
    pub core: CodexApp,
    pub ui: UiState,
    pub(crate) link: shell::Link,
}

impl UiApp {
    pub fn new(backend: shell::Backend) -> Self {
        Self::over(
            CodexApp::new()
                .with_submission_namespace(codex_protocol::ThreadId::new().to_string())
                .with_transcript_text(export_text::renderer()),
            backend,
        )
    }

    /// The app over a core model made elsewhere (a seeded one, for the
    /// debug menu and the fixtures' tests).
    pub fn over(core: CodexApp, backend: shell::Backend) -> Self {
        Self {
            core,
            ui: UiState {
                nav_key: GlobalKey::with_label("navigator"),
                thread_ui: ThreadUiMap::default(),
                lang: Lang::default(),
                debug: false,
                start_agents: false,
                start_resume: false,
                awaits_facts: false,
                provisional_draft: false,
                presenter: Presenter::new(),
                presenters: presenters::Presenters::default(),
                switches_seen: 0,
                followed: None,
                copies: CopyLog::default(),
                desktop_notice: None,
                unsent: None,
                startup_failure: None,
            },
            link: shell::Link::new(backend),
        }
    }

    pub fn with_debug_screens(mut self, on: bool) -> Self {
        self.ui.debug = on;
        self
    }

    pub fn with_start_agents(mut self, on: bool) -> Self {
        self.ui.start_agents = on;
        self.with_synced_first_thread()
    }

    pub fn with_start_resume(mut self, on: bool) -> Self {
        self.ui.start_resume = on;
        self.with_synced_first_thread()
    }

    /// `codex resume <id>` / `codex fork <id>`: the first thread is that
    /// session (read after the connection is up, opened after the startup
    /// steps and the cwd prompt).
    pub fn with_start_session(
        mut self,
        start: codex_tui_core::screens::onboarding::session::Start,
    ) -> Self {
        self.core = self.core.with_start_session(start);
        self
    }

    /// The prompt of the command line (`codex "prompt"`): sent when the first thread starts.
    pub fn with_initial_prompt(mut self, message: codex_tui_core::threads::UserMessage) -> Self {
        self.core = self.core.with_initial_prompt(message);
        self
    }

    /// The host prints what the user wrote and the launch never took when the app ends because
    /// the server did not start (`restore_terminal_before_fatal_exit`): `slot` gets it first.
    pub fn with_unsent_slot(mut self, slot: UnsentSlot) -> Self {
        self.ui.unsent = Some(slot);
        self
    }

    /// Reports an accepted launch failure after the terminal has been restored.
    pub fn with_startup_failure_slot(mut self, slot: Arc<Mutex<Option<String>>>) -> Self {
        self.ui.startup_failure = Some(slot);
        self
    }

    /// When the first thread starts follows from the command line: a page
    /// (`codex agents`, `codex resume`) opens instead of one, a host that
    /// answers the startup facts has it wait for the startup steps, any
    /// other starts it on connect.
    fn with_synced_first_thread(mut self) -> Self {
        let when = if self.ui.start_agents || self.ui.start_resume {
            FirstThread::Never
        } else if self.ui.awaits_facts {
            FirstThread::AfterLaunch
        } else {
            FirstThread::OnConnect
        };
        self.core = self.core.with_first_thread(when);
        self
    }

    /// The host answers the startup facts (`Ask::Facts`): the home waits for
    /// them (`StartupGate`) and the first thread starts after the startup
    /// screens (`Ev::Launched`), not on connect.
    pub fn with_awaited_facts(mut self, on: bool) -> Self {
        self.ui.awaits_facts = on;
        self.with_synced_first_thread()
    }

    /// Let a regular local launch edit a draft while the host checks whether a prompt is due.
    pub fn with_provisional_draft(mut self, on: bool) -> Self {
        self.ui.provisional_draft = on;
        self
    }

    /// The host answers the folder trust of `/cd`: a folder whose trust is undecided asks the
    /// question over the chat before the session starts there.
    pub fn with_folder_consent(mut self, on: bool) -> Self {
        self.core = self.core.with_folder_consent(on);
        self
    }

    /// The shell reads the time from `clock` (the default is the wall clock): a test hands it a
    /// `shell::FixedClock` and says the time itself.
    pub fn with_clock(mut self, clock: std::sync::Arc<dyn shell::Clock>) -> Self {
        self.link = self.link.with_clock(clock);
        self
    }

    pub fn with_lang(mut self, lang: Lang) -> Self {
        self.ui.lang = lang;
        self
    }

    /// The ingress router: core drops what an older connection sent (R1), the
    /// rest is applied, the UI's controllers follow the model, and the effects
    /// go to the shell.
    pub fn ingest(&mut self, ingress: Ingress, cx: &mut AppCx<'_, '_, UiApp>) {
        let failure = match ingress.event() {
            Some(AppEv::StartFailed(message)) => Some(message.clone()),
            _ => None,
        };
        let Some(mut fx) = self.core.ingest(ingress) else {
            cx.unchanged();
            return;
        };
        if let (Some(message), Some(slot)) = (failure, &self.ui.startup_failure)
            && let Ok(mut slot) = slot.lock()
        {
            *slot = Some(message);
        }
        if let Some(notice) = fx.take_desktop_notice() {
            self.ui.desktop_notice = Some(Rc::new(notice));
        }
        self.follow_core();
        self.ui.presenter.event();
        presenter::present(self, cx);
        presenters::present(self, cx);
        // The connection did not start: the app ends, and the host reports
        // why (the oracle's `run_main_inner` returns the error).
        let ends = fx.ends_app();
        if ends {
            self.keep_unsent();
        }
        shell::run_effects(self, fx);
        if ends {
            cx.quit();
        }
    }

    /// The unsent text of the blank thread goes into the host's slot before the app ends: the
    /// queue the first thread never took, else the composer with the keys the paste detector
    /// still holds.
    fn keep_unsent(&self) {
        let Some(slot) = &self.ui.unsent else { return };
        let blank = self.ui.thread_ui.get(None);
        let text = self
            .core
            .unsent_startup_text(&crate::pages::session::composer::burst::unsent(&blank));
        if let Ok(mut held) = slot.lock() {
            *held = text;
        }
    }

    /// Makes the controllers of the active thread if it has none.
    pub(crate) fn follow_core(&mut self) {
        self.ui.copies.take_in(self.core.latest_effect());
        if let Some(active) = self.core.session().active.clone() {
            let vim_default = self.core.tui().vim_mode_default;
            // a thread with no controllers yet is the blank ones' twin
            let known = !Rc::ptr_eq(
                &self.ui.thread_ui.get(Some(&active)),
                &self.ui.thread_ui.get(None),
            );
            self.ui.thread_ui.ensure(&active, vim_default);
            // a new chat widget takes the kill ring and the register of the one before it
            let from = self.ui.followed.replace(active.clone());
            if !known && let Some(from) = from.filter(|from| *from != active) {
                let held = self.ui.thread_ui.get(Some(&from)).draft.take_register();
                self.ui
                    .thread_ui
                    .get(Some(&active))
                    .draft
                    .restore_register(held);
            }
            // An empty conversation moved to another backend: its draft goes
            // with it (oracle: `NewBackendSession::CarryDraft`).
            let switches = self.core.session().switches;
            if switches != self.ui.switches_seen {
                self.ui.switches_seen = switches;
                if let Some(from) = self.core.session().switched_from.clone() {
                    let (from, to) = (
                        self.ui.thread_ui.get(Some(&from)),
                        self.ui.thread_ui.get(Some(&active)),
                    );
                    let text = from.composer.text().to_string();
                    if !text.is_empty() {
                        let caret = from.composer.selection().extent_offset;
                        to.composer.reset(&text);
                        to.composer
                            .set_selection(TextSelection::collapsed(caret.min(text.len())));
                        from.composer.clear();
                    }
                    // A Cloud destination continues the draft's Normal mode
                    // (`restore_cloud_switch_draft`): a Vim Escape the user made while Cloud
                    // connected is not undone by the new draft's start in Insert.
                    let agents = &self.core.screens().agents;
                    let normal = from
                        .draft
                        .with_vim(|vim| vim.is_enabled() && vim.mode() == VimMode::Normal);
                    if normal && agents.is_cloud(agents.displayed) {
                        to.draft.set_vim_enabled(true);
                    }
                }
            }
        }
    }

    /// Puts an event into the ingress stream under the current epoch.
    pub fn send(&self, ev: AppEv) {
        shell::send(self, Ingress::new(self.core.epoch(), ev));
    }
}
