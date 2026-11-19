//! `UiApp`, the model of the rtwk app: the core model plus the state only the
//! UI has. `core` is `codex_tui_core::app::CodexApp`, plain data with one
//! writer (`apply_app`) and no rtwk; `ui` holds what the widgets keep across
//! view runs and `apply` never sees (R2, R14): the `Navigator`'s key, the
//! per-thread controllers, the language, the debug flag. The shell's `Link`
//! is the third part: the app's end of the effect runner.

use codex_tui_core::app::AppEv;
use codex_tui_core::app::CodexApp;
use codex_tui_core::app::Ingress;
use rtwk::GlobalKey;
use rtwk::app::AppCx;

pub mod debug;
pub mod run;
pub mod start_page;
pub mod theme;
pub mod view;

use crate::l10n::Lang;
use crate::services::shell;
use crate::sheets::presenter;
use crate::sheets::presenter::Presenter;
use crate::widgets::thread_ui::ThreadUiMap;

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
    /// The request presenter's model (ADR-002): the kept handle of the
    /// navigator's home and which request is open.
    pub presenter: Presenter,
    /// How many backend switches the composers followed.
    pub switches_seen: u32,
}

/// The model of the rtwk app.
pub struct UiApp {
    pub core: CodexApp,
    pub ui: UiState,
    pub(crate) link: shell::Link,
}

impl UiApp {
    pub fn new(backend: shell::Backend) -> Self {
        Self::over(CodexApp::new(), backend)
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
                presenter: Presenter::new(),
                switches_seen: 0,
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
        // The overview opens instead of a first thread (`codex agents`).
        self.core = self.core.with_start_thread_on_connect(!on);
        self
    }

    pub fn with_start_resume(mut self, on: bool) -> Self {
        self.ui.start_resume = on;
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
        let Some(fx) = self.core.ingest(ingress) else {
            cx.unchanged();
            return;
        };
        self.follow_core();
        self.ui.presenter.event();
        presenter::present(self, cx);
        // The connection did not start: the app ends, and the host reports
        // why (the oracle's `run_main_inner` returns the error).
        let ends = fx.ends_app();
        shell::run_effects(self, fx);
        if ends {
            cx.quit();
        }
    }

    /// Makes the controllers of the active thread if it has none.
    pub(crate) fn follow_core(&mut self) {
        if let Some(active) = self.core.session().active.clone() {
            self.ui.thread_ui.ensure(&active);
            // An empty conversation moved to another backend: its draft goes
            // with it (oracle: `NewBackendSession::CarryDraft`).
            let switches = self.core.session().switches;
            if switches != self.ui.switches_seen {
                self.ui.switches_seen = switches;
                if let Some(from) = self.core.session().switched_from.clone() {
                    let draft = self.ui.thread_ui.get(Some(&from)).composer.clone();
                    let text = draft.text().to_string();
                    if !text.is_empty() {
                        self.ui.thread_ui.get(Some(&active)).composer.reset(&text);
                        draft.clear();
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
