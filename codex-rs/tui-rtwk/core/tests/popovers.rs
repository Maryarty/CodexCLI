//! The composer's triggers and reverse history search (the `$`/`@` popups'
//! rows are `mentions`' own: `tests/mentions.rs`). History search ports `chat_composer/history_search.rs` behaviour
//! (substring, case-insensitive, newest first; the end of the matches keeps
//! the current one).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::kit::history::{
    WalkDirection, PromptWalk, HistorySearch, Lookup, SearchPreview, SearchStatus, Store,
};
use codex_tui_core::screens::composer::{Trigger, TriggerKind, trigger};
use codex_tui_core::screens::popovers::*;
use pretty_assertions::assert_eq;

fn t(kind: TriggerKind, start: usize, query: &str) -> Option<Trigger> {
    Some(Trigger {
        kind,
        start,
        query: query.to_string(),
    })
}

#[test]
fn slash_at_the_start_triggers_the_command_popup() {
    assert_eq!(trigger("/"), t(TriggerKind::Slash, 0, ""));
    assert_eq!(trigger("/mo"), t(TriggerKind::Slash, 0, "mo"));
}

#[test]
fn a_blank_after_the_command_name_closes_the_popup() {
    assert_eq!(trigger("/model "), None);
    assert_eq!(trigger("/model gpt"), None);
}

#[test]
fn a_slash_inside_the_text_is_not_a_command() {
    assert_eq!(trigger("hello /mo"), None);
}

#[test]
fn dollar_and_at_trigger_at_a_word_start() {
    assert_eq!(trigger("$sha"), t(TriggerKind::Skill, 0, "sha"));
    assert_eq!(trigger("see @src/a"), t(TriggerKind::Mention, 4, "src/a"));
    assert_eq!(trigger("a $"), t(TriggerKind::Skill, 2, ""));
}

#[test]
fn a_blank_after_the_mention_closes_the_popup() {
    assert_eq!(trigger("@parser "), None);
    assert_eq!(trigger("mail me@x"), None);
}

fn model_with_history() -> Model {
    let mut m = Model::default();
    *m.history = Store::seeded(vec![
        "cargo build".into(),
        "git status".into(),
        "cargo test".into(),
    ]);
    m
}

/// A search over the model's history with what the draft shows: the
/// preview a step put in, `None` for the draft as it was.
struct Walk {
    s: HistorySearch,
    history: PromptWalk,
    store: Store,
    fx: Vec<Lookup>,
    shown: Option<String>,
}

impl Walk {
    fn new() -> Self {
        Walk {
            s: HistorySearch::new(),
            history: PromptWalk::new(),
            store: (*model_with_history().history).clone(),
            fx: Vec::new(),
            shown: None,
        }
    }

    fn show(&mut self, preview: SearchPreview) {
        match preview {
            SearchPreview::Show(entry) => self.shown = Some(entry.text),
            SearchPreview::Original => self.shown = None,
            SearchPreview::Keep => {}
        }
    }

    fn set_query(&mut self, query: &str) {
        let preview = self
            .s
            .set_query(query.into(), &mut self.history, &self.store, &mut self.fx);
        self.show(preview);
    }

    fn step(&mut self, direction: WalkDirection) {
        let preview = self
            .s
            .step(direction, &mut self.history, &self.store, &mut self.fx);
        self.show(preview);
    }

    fn preview(&self) -> Option<&str> {
        self.shown.as_deref()
    }
}

#[test]
fn history_search_shows_the_newest_match_and_walks_older_and_newer() {
    let mut w = Walk::new();
    assert_eq!(w.s.status, SearchStatus::Idle);
    w.set_query("c");
    assert_eq!(w.preview(), Some("cargo test"));
    w.step(WalkDirection::Older);
    assert_eq!(w.preview(), Some("cargo build"));
    w.step(WalkDirection::Older);
    assert_eq!(w.preview(), Some("cargo build"), "the end stays");
    assert_eq!(w.s.status, SearchStatus::Match, "no flicker at the end");
    w.step(WalkDirection::Newer);
    assert_eq!(w.preview(), Some("cargo test"));
}

#[test]
fn history_search_is_case_insensitive_and_reports_no_match() {
    let mut w = Walk::new();
    w.set_query("GIT");
    assert_eq!(w.preview(), Some("git status"));
    w.set_query("zzz");
    assert_eq!(w.s.status, SearchStatus::NoMatch);
    assert_eq!(w.preview(), None);
    w.set_query("");
    assert_eq!(w.s.status, SearchStatus::Idle);
}

// ------------------------------------------------- the search's own state: the composer holds it

#[test]
fn a_query_built_key_by_key_matches_as_one_typed_at_once() {
    let mut w = Walk::new();
    let mut query = String::new();
    for typed in ["g", "it"] {
        query.push_str(typed);
        w.set_query(&query);
    }
    assert_eq!(w.s.query, "git");
    assert_eq!(w.s.status, SearchStatus::Match);
    assert_eq!(w.preview(), Some("git status"));
}

#[test]
fn shortening_or_emptying_the_query_restarts_from_the_newest() {
    let mut w = Walk::new();
    w.set_query("gitx");
    assert_eq!(w.s.status, SearchStatus::NoMatch);
    w.set_query("git");
    assert_eq!(w.s.status, SearchStatus::Match);
    w.set_query("");
    assert_eq!(w.s.status, SearchStatus::Idle);
    assert_eq!(w.preview(), None);
}

/// `history_search.rs:213-230`: Enter accepts only a match, which is the
/// composer's decision on `status`: no preview otherwise.
#[test]
fn only_a_match_has_a_preview() {
    let mut w = Walk::new();
    assert_eq!((w.s.status, w.preview()), (SearchStatus::Idle, None));
    w.set_query("zzz");
    assert_eq!((w.s.status, w.preview()), (SearchStatus::NoMatch, None));
    w.set_query("cargo");
    assert_eq!(
        (w.s.status, w.preview()),
        (SearchStatus::Match, Some("cargo test"))
    );
}

/// `apply_history_search_result`: a read that is out shows `searching` and
/// keeps the preview; a failed read returns to the draft saying nothing.
#[test]
fn a_pending_read_keeps_the_preview_and_an_unavailable_one_restores_the_draft() {
    use codex_tui_core::kit::history::SearchResult;
    let mut w = Walk::new();
    w.set_query("cargo");
    assert_eq!(w.preview(), Some("cargo test"));
    let preview = w.s.apply(SearchResult::Pending);
    w.show(preview);
    assert_eq!(
        (w.s.status, w.preview()),
        (SearchStatus::Searching, Some("cargo test"))
    );
    let preview = w.s.apply(SearchResult::Unavailable);
    w.show(preview);
    assert_eq!((w.s.status, w.preview()), (SearchStatus::Idle, None));
}

/// Ctrl+V: the host answers with a file, and a composer takes the first
/// answer after the one it counted when it asked (an answer that came
/// before is another's).
#[test]
fn a_clipboard_image_answers_only_the_asks_before_it() {
    use codex_tui_core::screens::composer::clipboard::Pasted;
    use std::path::Path;
    let mut pasted = Pasted::default();
    assert_eq!(pasted.after(0), None);
    pasted.on_image("/tmp/a.png".into());
    assert_eq!(pasted.after(0), Some(Path::new("/tmp/a.png")));
    assert_eq!(
        pasted.after(1),
        None,
        "asked after the answer: waits for the next"
    );
    pasted.on_image("/tmp/b.png".into());
    assert_eq!(pasted.after(1), Some(Path::new("/tmp/b.png")));
    assert_eq!(
        (pasted.answers(), pasted.latest()),
        (2, Some(Path::new("/tmp/b.png")))
    );
}

#[test]
fn the_image_pasted_seed_is_an_event() {
    let ev = seed(serde_json::json!({"image_pasted": "/tmp/a.png"})).expect("seed");
    assert_eq!(ev, Ev::ImagePasted("/tmp/a.png".into()));
}

/// What the `@` and `$` popups list from the server (oracle `refresh_startup_skills`,
/// `fetch_plugin_mentions`, `on_connectors_loaded`, `mention_items`): the skills and installed
/// plugins of the thread in use, asked once when its directory is known, and the installed apps
/// of the `/apps` directory, as one catalog the popovers model holds.
mod live {
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::app::{
        AppEv, AppFx, BackendEv, CodexApp, Command, apply_app, apply_thread,
    };
    use codex_tui_core::ids::{ThreadId, Ticket};
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::popovers::{self, MentionKind};
    use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenThreadEv, chat};
    use codex_tui_core::threads::ThreadEv;
    use pretty_assertions::assert_eq;
    use serde_json::{Value, json};

    const T: &str = "thread-1";

    fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        app.assert_consistent();
        fx
    }

    fn ready() -> CodexApp {
        let mut app = CodexApp::new();
        let fx = run(&mut app, AppEv::Connected);
        let ticket: Ticket = fx
            .commands()
            .iter()
            .find_map(|c| match c {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread");
        run(
            &mut app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
        );
        app
    }

    /// The server says where the thread works; the next event of the app asks for the lists.
    fn in_dir(app: &mut CodexApp, cwd: &str) -> AppFx {
        apply_thread(
            app,
            &ThreadId::new(T),
            ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Metadata {
                name: None,
                forked_from: None,
                cwd: cwd.to_owned(),
                path: None,
                parent_owned: false,
                ephemeral: false,
                model: None,
                effort: None,
                provider: "openai".to_owned(),
                daybreak: None,
                resumed: false,
                fresh: false,
            })),
            &mut AppFx::default(),
        );
        poke(app)
    }

    /// An event that changes nothing the popups hold.
    fn poke(app: &mut CodexApp) -> AppFx {
        run(
            app,
            AppEv::Screen(ScreenEv::Popovers(popovers::Ev::Tasks(Vec::new()))),
        )
    }

    /// The popovers' RPCs in `fx`, with the commands that carry them.
    fn asked(fx: &AppFx) -> Vec<(&Command, popovers::Command)> {
        fx.commands()
            .iter()
            .filter_map(|command| match command {
                Command::Screen {
                    command: ScreenCommand::Popovers(asked),
                    ..
                } => Some((command, asked.clone())),
                _ => None,
            })
            .collect()
    }

    fn answer(app: &mut CodexApp, command: &Command, result: Value) -> AppFx {
        let (ticket, reply) =
            codex_tui_core::protocol::reply(command, Ok(result)).expect("a command with a reply");
        run(app, AppEv::Reply(ticket, reply))
    }

    fn skills(names: &[&str]) -> Value {
        let skills: Vec<_> = names
            .iter()
            .map(|name| {
                json!({
                    "name": name,
                    "description": format!("{name} skill"),
                    "path": format!("/skills/{name}/SKILL.md"),
                    "scope": "user",
                    "enabled": true,
                })
            })
            .collect();
        json!({"data": [{"cwd": "/work", "skills": skills, "errors": []}]})
    }

    fn plugins() -> Value {
        json!({"marketplaces": [{
            "name": "debug", "path": null, "interface": null,
            "plugins": [{
                "id": "calendar@debug", "name": "calendar",
                "source": {"type": "local", "path": "/plugins/calendar"},
                "installed": true, "enabled": true,
                "installPolicy": "AVAILABLE", "authPolicy": "ON_USE",
                "interface": {"displayName": "Calendar", "capabilities": [], "screenshots": [], "screenshotUrls": []},
            }],
        }]})
    }

    fn names(app: &CodexApp) -> Vec<(MentionKind, String)> {
        app.screens()
            .popovers
            .mentions
            .iter()
            .map(|mention| (mention.kind, mention.name.clone()))
            .collect()
    }

    #[test]
    fn the_skills_and_plugins_are_asked_for_once_the_thread_has_its_directory() {
        let mut app = ready();
        assert_eq!(asked(&poke(&mut app)).len(), 0, "no directory yet");
        let fx = in_dir(&mut app, "/work");
        let asks: Vec<_> = asked(&fx).into_iter().map(|(_, ask)| ask).collect();
        assert_eq!(
            asks,
            vec![
                popovers::Command::Skills {
                    cwd: "/work".into()
                },
                popovers::Command::Plugins {
                    cwd: "/work".into()
                },
            ]
        );
        assert_eq!(asked(&poke(&mut app)).len(), 0, "once per thread");
    }

    #[test]
    fn the_answers_make_the_catalog_skills_then_plugins() {
        let mut app = ready();
        let fx = in_dir(&mut app, "/work");
        let asks = asked(&fx);
        assert_eq!(names(&app), vec![], "nothing listed before the answers");
        answer(&mut app, asks[1].0, plugins());
        assert_eq!(names(&app), vec![(MentionKind::Plugin, "Calendar".into())]);
        answer(&mut app, asks[0].0, skills(&["figma", "docs"]));
        assert_eq!(
            names(&app),
            vec![
                (MentionKind::Skill, "figma".into()),
                (MentionKind::Skill, "docs".into()),
                (MentionKind::Plugin, "Calendar".into()),
            ]
        );
    }

    #[test]
    fn a_failed_list_leaves_the_popups_as_they_were() {
        let mut app = ready();
        let fx = in_dir(&mut app, "/work");
        let asks = asked(&fx);
        answer(&mut app, asks[0].0, skills(&["figma"]));
        let (ticket, reply) = codex_tui_core::protocol::reply(asks[1].0, Err("403".to_string()))
            .expect("a command with a reply");
        run(&mut app, AppEv::Reply(ticket, reply));
        assert_eq!(names(&app), vec![(MentionKind::Skill, "figma".into())]);
    }

    fn apps_changed() -> AppEv {
        let note: ServerNotification = serde_json::from_value(json!({
            "method": "app/list/updated",
            "params": {"data": [{"id": "notion", "name": "Notion", "isAccessible": true,
                                 "isEnabled": true, "pluginDisplayNames": []}]},
        }))
        .expect("the directory notification");
        AppEv::Backend(BackendEv::Notification(Box::new(note)))
    }

    #[test]
    fn the_installed_apps_of_the_directory_join_the_catalog_and_leave_with_it() {
        let mut app = ready();
        let fx = in_dir(&mut app, "/work");
        answer(&mut app, asked(&fx)[0].0, skills(&["figma"]));
        // the notification makes the app read the directory again
        let fx = run(&mut app, apps_changed());
        let read = fx
            .commands()
            .iter()
            .find(|command| {
                matches!(
                    command,
                    Command::Screen {
                        command: ScreenCommand::Select(_),
                        ..
                    }
                )
            })
            .expect("app/list");
        answer(
            &mut app,
            read,
            json!({"data": [
                {"id": "notion", "name": "Notion", "isAccessible": true, "isEnabled": true,
                 "pluginDisplayNames": []},
                {"id": "linear", "name": "Linear", "isAccessible": false, "isEnabled": true,
                 "pluginDisplayNames": []},
            ]}),
        );
        assert_eq!(
            names(&app),
            vec![
                (MentionKind::Skill, "figma".into()),
                (MentionKind::App, "Notion".into()),
            ]
        );
        // an account change forgets the directory, and the apps with it
        run(
            &mut app,
            AppEv::Backend(BackendEv::Notification(Box::new(
                serde_json::from_value(json!({
                    "method": "account/updated",
                    "params": {"authMode": null, "planType": null},
                }))
                .expect("the account notification"),
            ))),
        );
        assert_eq!(names(&app), vec![(MentionKind::Skill, "figma".into())]);
    }

    #[test]
    fn what_a_script_seeded_stays_until_the_server_lists_something() {
        let mut app = ready();
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Popovers(popovers::Ev::Mentions(vec![
                popovers::Mention::new(MentionKind::Skill, "Seeded", "", "seeded"),
            ]))),
        );
        in_dir(&mut app, "/work");
        poke(&mut app);
        assert_eq!(names(&app), vec![(MentionKind::Skill, "Seeded".into())]);
    }
}
