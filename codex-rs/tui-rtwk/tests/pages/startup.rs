//! Screen tests of the `startup` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/startup/`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::common::slot;
use codex_tui_core::app::Command;
use codex_tui_core::screens::onboarding::Ask;
use codex_tui_rtwk::services::backend::fake::FakeHandle;

fn startup_request(fake: &FakeHandle, read: impl Fn(&Ask) -> Option<u64>) -> Option<u64> {
    fake.commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Startup { ask, .. } => read(ask),
            _ => None,
        })
}

fn device(ask: &Ask) -> Option<u64> {
    match ask {
        Ask::GatewayDevice { request, .. } => Some(*request),
        _ => None,
    }
}

fn launch(ask: &Ask) -> Option<u64> {
    match ask {
        Ask::Launch { request } => Some(*request),
        _ => None,
    }
}

#[test]
fn startup_slot_is_registered_and_empty() {
    slot("startup");
}

// The host can expose an editable startup draft for an established local launch.
// A first sign-in or a start page still waits for the startup decision.
mod gate {
    use super::{device, launch, startup_request};
    use codex_tui_core::app::{AppEv, Command};
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};
    use codex_tui_core::screens::startup;

    use crate::common::{Harness, ms, start_app};

    const CHAT: &str = "? for shortcuts";

    fn awaiting() -> Harness {
        start_app(|app| app.with_awaited_facts(true))
    }

    fn answer(h: &Harness, facts: Facts) {
        let Some(epoch) = h.fake.epoch() else {
            panic!("the app is not attached to the scripted server");
        };
        h.fake.inject(
            epoch,
            AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { request: 1, facts })),
        );
    }

    fn wait_request(h: &mut Harness, read: impl Fn(&Ask) -> Option<u64>) -> u64 {
        h.wait_until("the startup ask", |h| {
            startup_request(&h.fake, &read).is_some()
        });
        startup_request(&h.fake, read).expect("the startup ask")
    }

    fn trust_saved(h: &mut Harness) {
        let request = wait_request(h, |ask| match ask {
            Ask::WriteTrust { request, .. } => Some(*request),
            _ => None,
        });
        inject(h, Ev::TrustSaved { request });
    }

    /// The host's answers to the asks that follow the facts: no gateway device code, then the
    /// prompts due for this launch.
    fn launched(h: &mut Harness, prompts: Vec<startup::Prompt>) {
        let request = wait_request(h, device);
        inject(
            h,
            Ev::GatewayDevice {
                request,
                code: None,
            },
        );
        let request = wait_request(h, launch);
        inject(h, Ev::Launched { request, prompts });
    }

    fn hooks_due() -> startup::Prompt {
        startup::Prompt::Hooks(startup::HooksPrompt {
            count: 2,
            trust: startup::Trust::Idle,
            updates: Vec::new(),
        })
    }

    fn inject(h: &Harness, ev: Ev) {
        let Some(epoch) = h.fake.epoch() else {
            panic!("the app is not attached to the scripted server");
        };
        h.fake
            .inject(epoch, AppEv::Screen(ScreenEv::Onboarding(ev)));
    }

    fn trust() -> Facts {
        Facts {
            welcome: Some(Welcome { animations: false }),
            trust: Some(TrustAsk {
                cwd: "/workspace/project".into(),
                target: "/workspace/project".into(),
                cancel: TrustCancel::AgentsOverview,
                ..TrustAsk::default()
            }),
            ..Facts::default()
        }
    }

    #[test]
    fn nothing_is_drawn_while_the_facts_are_awaited() {
        let mut h = awaiting();
        h.settle();
        let screen = h.t.screen();
        assert!(!screen.contains(CHAT), "the chat is up:\n{screen}");
    }

    #[test]
    fn the_chat_shows_when_the_facts_hold_nothing_due() {
        let mut h = awaiting();
        h.settle();
        answer(&h, Facts::default());
        // The launch's prompts are read before the first frame of the chat.
        for _ in 0..20 {
            h.t.pump_for(ms(5));
            assert!(!h.t.screen().contains(CHAT), "the chat before the launch");
        }
        launched(&mut h, Vec::new());
        h.wait_for("the chat", |s| s.contains(CHAT));
    }

    fn thread_starts(h: &Harness) -> usize {
        h.fake
            .commands()
            .iter()
            .filter(|c| matches!(c, Command::StartThread { .. }))
            .count()
    }

    fn turn_starts(h: &Harness) -> Vec<String> {
        h.fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::StartTurn { message, .. } => Some(message.text),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_provisional_draft_survives_trust_and_sends_once_when_ready() {
        let mut h = start_app(|app| app.with_awaited_facts(true).with_provisional_draft(true));
        h.wait_for("the draft", |s| s.contains("Ask Codex to do anything"));
        h.t.type_text("/new");
        h.press("enter");
        assert_eq!(thread_starts(&h), 0, "a startup command opened a thread");
        h.press("ctrl+c");
        h.t.type_text("before the startup checks");
        h.press("enter");
        let screen = h.t.screen();
        assert!(screen.contains("› before the startup checks"), "{screen}");
        assert!(screen.contains("Waiting for startup"), "{screen}");

        answer(&h, trust());
        h.wait_for("the trust prompt", |s| s.contains("trust"));
        assert!(!h.t.screen().contains("before the startup checks"));
        assert!(turn_starts(&h).is_empty());
        assert_eq!(thread_starts(&h), 0);
        h.press("enter");
        trust_saved(&mut h);
        h.settle();
        assert_eq!(thread_starts(&h), 0);
        launched(&mut h, Vec::new());
        h.wait_until("the confirmed turn", |h| !turn_starts(h).is_empty());
        h.settle();
        assert_eq!(thread_starts(&h), 1);
        assert_eq!(turn_starts(&h), ["before the startup checks"]);
    }

    #[test]
    fn the_provisional_draft_accepts_normal_input_after_launch() {
        let mut h = start_app(|app| app.with_awaited_facts(true).with_provisional_draft(true));
        h.wait_for("the draft", |s| s.contains("Ask Codex to do anything"));
        h.t.type_text("typed before the backend is ready");
        h.press("tab");
        assert!(h.t.screen().contains("› typed before the backend is ready"));
        assert!(turn_starts(&h).is_empty());
        answer(&h, Facts::default());
        launched(&mut h, Vec::new());
        h.wait_until("the fresh thread", |h| thread_starts(h) == 1);
        h.settle();
        h.t.type_text(" and kept");
        h.press("tab");
        h.wait_until("the turn", |h| !turn_starts(h).is_empty());
        assert_eq!(
            turn_starts(&h),
            ["typed before the backend is ready and kept"]
        );
    }

    // The oracle creates its session after the startup screens: the server
    // is not asked for a thread (its own sign-in would pre-empt the
    // device-code screen) until the launch is known.
    #[test]
    fn the_thread_starts_after_the_startup_screens() {
        let mut h = awaiting();
        h.settle();
        answer(&h, trust());
        h.wait_for("the trust prompt", |s| s.contains("trust"));
        assert_eq!(thread_starts(&h), 0, "{}", h.t.screen());
        h.press("enter");
        trust_saved(&mut h);
        h.settle();
        assert_eq!(thread_starts(&h), 0, "the prompts are still to be read");
        launched(&mut h, Vec::new());
        h.wait_for("the chat", |s| s.contains(CHAT));
        // The screen can be ahead of what the fake has recorded.
        h.wait_until("the thread start", |h| thread_starts(h) >= 1);
        assert_eq!(thread_starts(&h), 1);
    }

    // The same with nothing due: the thread starts when the launch is known.
    #[test]
    fn with_nothing_due_the_thread_starts_when_the_launch_is_known() {
        let mut h = awaiting();
        h.settle();
        answer(&h, Facts::default());
        h.settle();
        assert_eq!(thread_starts(&h), 0);
        launched(&mut h, Vec::new());
        h.wait_for("the chat", |s| s.contains(CHAT));
        // The screen can be ahead of what the fake has recorded.
        h.wait_until("the thread start", |h| thread_starts(h) >= 1);
        assert_eq!(thread_starts(&h), 1);
    }

    // R-F5 on a start page of the command line (`resume`): the resume
    // picker mounts only once the startup page popped, so the prompt goes
    // first and the picker is not drawn over it.
    #[test]
    fn a_start_page_waits_for_the_startup_page() {
        use codex_tui_rtwk::pages::resume;
        let resume_page = rtwk_test::find::by_type::<resume::Screen>();
        let mut h = start_app(|app| app.with_awaited_facts(true).with_start_resume(true));
        h.settle();
        answer(&h, Facts::default());
        launched(&mut h, vec![hooks_due()]);
        h.wait_for("the hooks review", |s| s.contains("hooks"));
        assert_eq!(resume_page.count(&h.t), 0, "{}", h.t.screen());
        h.press("esc");
        h.wait_until("the resume picker", |h| resume_page.count(&h.t) == 1);
    }

    // `SessionSelection::Resume` starts no session under the picker, and
    // `StartFresh` (the picker's cancel) starts one.
    #[test]
    fn the_resume_picker_starts_no_thread_until_it_is_cancelled() {
        use codex_tui_rtwk::pages::resume;
        let resume_page = rtwk_test::find::by_type::<resume::Screen>();
        let mut h = start_app(|app| app.with_awaited_facts(true).with_start_resume(true));
        h.settle();
        answer(&h, Facts::default());
        launched(&mut h, Vec::new());
        h.wait_until("the resume picker", |h| resume_page.count(&h.t) == 1);
        h.settle();
        assert_eq!(thread_starts(&h), 0, "{}", h.t.screen());
        h.press("esc");
        h.wait_until("the picker gone", |h| resume_page.count(&h.t) == 0);
        h.wait_until("the fresh thread", |h| thread_starts(h) == 1);
    }

    // onboarding_screen.rs `handle_key_event`: Ctrl-C on the sign-in menu
    // (auth in progress) exits the app; no chat frame comes before the exit.
    #[test]
    fn ctrl_c_on_the_sign_in_quits_without_a_chat_frame() {
        use codex_tui_core::screens::onboarding::auth::Offer;
        let mut h = crate::common::start_quittable(false);
        h.settle();
        answer(
            &h,
            Facts {
                sign_in: Some(Offer {
                    chatgpt: true,
                    api_key: true,
                    ..Offer::default()
                }),
                ..Facts::default()
            },
        );
        h.wait_for("the sign-in menu", |s| s.contains("Sign in with ChatGPT"));
        h.press("ctrl+c");
        for _ in 0..200 {
            let screen = h.t.screen();
            assert!(
                !screen.contains(CHAT),
                "a chat frame before the exit:\n{screen}"
            );
            if !h.t.is_running() {
                return;
            }
            h.t.pump_for(ms(5));
            std::thread::sleep(ms(2));
        }
        panic!("the app runs on:\n{}", h.t.screen());
    }

    // run_ratatui_app: the hooks review is asked before the app draws; the
    // chat follows its answer.
    #[test]
    fn a_launch_prompt_is_drawn_without_a_chat_frame_before_it() {
        let mut h = awaiting();
        h.settle();
        answer(&h, Facts::default());
        launched(&mut h, vec![hooks_due()]);
        let mut seen = false;
        for _ in 0..200 {
            // the injected event comes from the fake server's thread: wall
            // time, not only the fake clock
            h.t.pump_for(ms(5));
            std::thread::sleep(ms(2));
            let screen = h.t.screen();
            assert!(!screen.contains(CHAT), "a chat frame:\n{screen}");
            if screen.contains("hooks") || screen.contains("Hooks") {
                seen = true;
                break;
            }
        }
        assert!(seen, "the hooks review never came:\n{}", h.t.screen());
        h.press("esc");
        h.wait_for("the chat", |s| s.contains(CHAT));
    }

    #[test]
    fn a_due_step_is_drawn_without_a_chat_frame_before_it() {
        let mut h = awaiting();
        h.settle();
        answer(&h, trust());
        for _ in 0..200 {
            // the injected event comes from the fake server's thread: wall
            // time, not only the fake clock
            h.t.pump_for(ms(5));
            std::thread::sleep(ms(2));
            let screen = h.t.screen();
            assert!(!screen.contains(CHAT), "a chat frame:\n{screen}");
            if screen.contains("Folder access") {
                return;
            }
        }
        panic!("the prompt never came:\n{}", h.t.screen());
    }
}

// `codex resume <id>` and `codex fork <id>`: the app reads the session, asks
// for its directory when it differs from this one (the cwd prompt), and opens
// it in place of a fresh thread (oracle: `run_ratatui_app`).
mod session {
    use super::{device, launch, startup_request};
    use std::sync::OnceLock;

    use codex_tui_core::app::AppEv;
    use codex_tui_core::rpc::ServerError;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::onboarding::session::Start;
    use codex_tui_core::screens::onboarding::{Ev, Facts};
    use codex_tui_core::screens::startup::Action;
    use codex_tui_core::screens::startup::cwd::{ResumeMode, Setting};
    use codex_tui_rtwk::app::UiApp;
    use codex_tui_rtwk::app::run::{app_view, localized, root};
    use codex_tui_rtwk::l10n::Lang;
    use codex_tui_rtwk::services::backend::fake::{FakeHandle, FakeServer};
    use codex_tui_rtwk::services::shell::Backend;
    use rtwk_test::{Options, Tester};
    use serde_json::{Value, json};

    use crate::common::ms;

    const CHAT: &str = "? for shortcuts";

    fn runtime() -> &'static tokio::runtime::Runtime {
        static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_time()
                .build()
                .expect("runtime")
        })
    }

    fn thread_json(id: &str, cwd: &str) -> Value {
        json!({
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null,
            "cwd": cwd, "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null,
            "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
            "daybreakEnabled": null, "turns": []
        })
    }

    fn opened(id: &str, cwd: &str) -> Value {
        json!({
            "thread": thread_json(id, cwd),
            "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
            "cwd": cwd, "instructionSources": [],
            "approvalPolicy": "on-request", "approvalsReviewer": "user",
            "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
        })
    }

    /// The server's side: the session `saved-1` recorded `/tmp/saved`; the
    /// resume and the fork answer in the directory they were asked for.
    fn server(request: &Value) -> Option<Result<Value, ServerError>> {
        let cwd = request["params"]["cwd"].as_str().unwrap_or("/tmp/saved");
        match request["method"].as_str()? {
            "thread/read" if request["params"]["threadId"] == "saved-1" => {
                Some(Ok(json!({"thread": thread_json("saved-1", "/tmp/saved")})))
            }
            "thread/read" => Some(Err(ServerError::from("no rollout found"))),
            "thread/resume" => Some(Ok(opened("saved-1", cwd))),
            "thread/fork" => Some(Ok(opened("copy-1", cwd))),
            _ => None,
        }
    }

    struct Run {
        t: Tester,
        fake: FakeHandle,
        _guard: tokio::runtime::EnterGuard<'static>,
    }

    fn start(action: Action) -> Run {
        let guard = rtwk_tokio::enter(runtime());
        let (backend, fake) = FakeServer::new();
        fake.respond(server);
        let app = UiApp::new(Backend::Fake(backend))
            .with_awaited_facts(true)
            .with_start_session(Start {
                action,
                id: "saved-1".into(),
            });
        let mut t = Tester::with(Options::new().size(80, 24));
        t.pump_widget(localized(root(app_view(app)), Lang::En));
        t.pump_for(ms(200));
        Run {
            t,
            fake,
            _guard: guard,
        }
    }

    impl Run {
        /// Frames on the fake clock, a real pause between them for the shell's
        /// thread, until `ready` holds: a real deadline far above a loaded
        /// machine, not a count of rounds (the backend answers in real time).
        #[allow(clippy::disallowed_methods)] // a real deadline, as `common::wait`
        fn until(&mut self, what: &str, mut ready: impl FnMut(&mut Run) -> bool) {
            let started = std::time::Instant::now();
            loop {
                self.t.pump_for(ms(5));
                std::thread::sleep(ms(2));
                if ready(self) {
                    return;
                }
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(30),
                    "{what} never came in 30 s:\n{}\ncommands: {:?}",
                    self.t.screen(),
                    self.fake.commands()
                );
            }
        }

        fn inject(&self, ev: Ev) {
            let epoch = self.fake.epoch().expect("attached to the scripted server");
            self.fake
                .inject(epoch, AppEv::Screen(ScreenEv::Onboarding(ev)));
        }

        /// The host's answers: the facts, no device code, no launch prompts.
        fn launch(&mut self, configured: Option<ResumeMode>) {
            self.until("the session read", |r| {
                r.fake
                    .requests()
                    .iter()
                    .any(|q| q["method"] == "thread/read")
            });
            self.inject(Ev::Facts {
                request: 1,
                facts: Facts {
                    resume: Setting {
                        current: "/work/now".into(),
                        configured,
                        ..Setting::default()
                    },
                    ..Facts::default()
                },
            });
            self.until("the device ask", |r| {
                startup_request(&r.fake, device).is_some()
            });
            self.inject(Ev::GatewayDevice {
                request: startup_request(&self.fake, device).expect("device ask"),
                code: None,
            });
            // The launch is asked once the session read is answered and the
            // gateway step is over, on the shell's thread: an answer sent
            // before the ask is dropped as stale.
            self.until("the launch ask", |r| {
                startup_request(&r.fake, launch).is_some()
            });
            self.inject(Ev::Launched {
                request: startup_request(&self.fake, launch).expect("launch ask"),
                prompts: Vec::new(),
            });
        }

        fn asked(&self, method: &str) -> Vec<Value> {
            self.fake
                .requests()
                .into_iter()
                .filter(|q| q["method"] == method)
                .collect()
        }
    }

    #[test]
    fn resume_by_id_asks_for_the_directory_and_opens_the_session_there() {
        let mut r = start(Action::Resume);
        r.launch(None);
        r.until("the cwd prompt", |r| {
            r.t.screen().contains("Working directory")
        });
        let screen = r.t.screen();
        assert!(
            screen.contains("Use session directory (/tmp/saved)")
                && screen.contains("Use current directory (/work/now)"),
            "{screen}"
        );
        assert!(
            r.asked("thread/resume").is_empty(),
            "nothing opens before the answer"
        );
        r.t.press("2");
        r.until("the resume", |r| !r.asked("thread/resume").is_empty());
        let resumes = r.asked("thread/resume");
        assert_eq!(resumes[0]["params"]["threadId"], "saved-1");
        assert_eq!(resumes[0]["params"]["cwd"], "/work/now");
        r.until("the chat", |r| r.t.screen().contains(CHAT));
        assert!(
            r.asked("thread/start").is_empty(),
            "{:?}",
            r.fake.requests()
        );
    }

    #[test]
    fn the_session_directory_is_the_default_choice() {
        let mut r = start(Action::Resume);
        r.launch(None);
        r.until("the cwd prompt", |r| {
            r.t.screen().contains("Working directory")
        });
        r.t.press("enter");
        r.until("the resume", |r| !r.asked("thread/resume").is_empty());
        assert_eq!(r.asked("thread/resume")[0]["params"]["cwd"], "/tmp/saved");
    }

    // A remembered mode (`tui.resume_cwd`) answers for the user.
    #[test]
    fn a_remembered_mode_opens_without_a_prompt() {
        let mut r = start(Action::Resume);
        r.launch(Some(ResumeMode::Current));
        r.until("the resume", |r| !r.asked("thread/resume").is_empty());
        assert_eq!(r.asked("thread/resume")[0]["params"]["cwd"], "/work/now");
        assert!(!r.t.screen().contains("Working directory"));
        r.until("the chat", |r| r.t.screen().contains(CHAT));
    }

    #[test]
    fn fork_by_id_opens_the_copy() {
        let mut r = start(Action::Fork);
        r.launch(Some(ResumeMode::Session));
        r.until("the fork", |r| !r.asked("thread/fork").is_empty());
        let forks = r.asked("thread/fork");
        assert_eq!(forks[0]["params"]["threadId"], "saved-1");
        assert_eq!(forks[0]["params"]["cwd"], "/tmp/saved");
        r.until("the chat", |r| r.t.screen().contains(CHAT));
        assert!(r.asked("thread/start").is_empty());
    }

    // Ctrl-C on the cwd prompt leaves: nothing is resumed.
    #[test]
    fn ctrl_c_on_the_cwd_prompt_opens_nothing() {
        let mut r = start(Action::Resume);
        r.launch(None);
        r.until("the cwd prompt", |r| {
            r.t.screen().contains("Working directory")
        });
        r.t.press("ctrl+c");
        for _ in 0..20 {
            r.t.pump_for(ms(5));
            std::thread::sleep(ms(2));
        }
        assert!(r.asked("thread/resume").is_empty() && r.asked("thread/start").is_empty());
    }
}

// The command line: `resume <ID>` and `fork <ID>` name the session the first
// thread opens; `resume` alone is the picker.
mod command_line {
    use codex_tui_core::screens::onboarding::session::Start;
    use codex_tui_core::screens::startup::Action;
    use codex_tui_rtwk::Cli;

    fn parse(words: &[&str]) -> anyhow::Result<Cli> {
        Cli::parse(words.iter().map(|w| (*w).to_owned()))
    }

    #[test]
    fn an_id_after_resume_or_fork_names_the_session() {
        for (word, action) in [("resume", Action::Resume), ("fork", Action::Fork)] {
            let cli = parse(&[word, "0192-abc"]).expect("parses");
            assert_eq!(
                cli.session,
                Some(Start {
                    action,
                    id: "0192-abc".into()
                })
            );
            assert!(!cli.resume);
        }
    }

    #[test]
    fn the_id_may_come_after_the_flags() {
        let cli = parse(&["resume", "--cd", "/work", "0192-abc", "-m", "gpt-5"]).expect("parses");
        assert_eq!(cli.session.map(|s| s.id), Some("0192-abc".to_owned()));
        assert_eq!(cli.cwd.as_deref(), Some(std::path::Path::new("/work")));
        assert_eq!(cli.model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn resume_alone_is_the_picker_and_fork_needs_an_id() {
        let cli = parse(&["resume"]).expect("parses");
        assert!(cli.resume && cli.session.is_none());
        let refused = parse(&["fork"]).expect_err("a fork has no picker here");
        assert!(
            refused.to_string().contains("needs a session ID"),
            "{refused}"
        );
    }
}
