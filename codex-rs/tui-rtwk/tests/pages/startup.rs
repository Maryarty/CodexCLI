//! Screen tests of the `startup` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/startup/`.

use crate::common::slot;

#[test]
fn startup_slot_is_registered_and_empty() {
    slot("startup");
}

// R-F5: the oracle runs the startup checks before it draws the chat, so its
// prompt is never preceded by a chat frame. A host that answers the facts
// has the app wait for them (`with_awaited_facts`).
mod gate {
    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::onboarding::{Ev, Facts, TrustAsk, TrustCancel, Welcome};
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

    /// The host's answer to the launch's ask (`Ask::Launch`, request 2:
    /// the first after the facts').
    fn launched(h: &Harness, prompts: Vec<startup::Prompt>) {
        inject(h, Ev::Launched {
            request: 2,
            prompts,
        });
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
        h.fake.inject(epoch, AppEv::Screen(ScreenEv::Onboarding(ev)));
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
        launched(&h, Vec::new());
        h.wait_for("the chat", |s| s.contains(CHAT));
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
        launched(&h, vec![hooks_due()]);
        h.wait_for("the hooks review", |s| s.contains("hooks"));
        assert_eq!(resume_page.count(&h.t), 0, "{}", h.t.screen());
        h.press("esc");
        h.wait_until("the resume picker", |h| resume_page.count(&h.t) == 1);
    }

    // run_ratatui_app: the hooks review is asked before the app draws; the
    // chat follows its answer.
    #[test]
    fn a_launch_prompt_is_drawn_without_a_chat_frame_before_it() {
        let mut h = awaiting();
        h.settle();
        answer(&h, Facts::default());
        launched(&h, vec![hooks_due()]);
        let mut seen = false;
        for _ in 0..200 {
            h.t.pump_for(ms(5));
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
            h.t.pump_for(ms(5));
            let screen = h.t.screen();
            assert!(!screen.contains(CHAT), "a chat frame:\n{screen}");
            if screen.contains("Folder access") {
                return;
            }
        }
        panic!("the prompt never came:\n{}", h.t.screen());
    }
}
