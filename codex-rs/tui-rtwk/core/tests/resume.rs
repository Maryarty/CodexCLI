//! State tests of S18 (the resume picker): the listing events, the clock as a
//! model input (R11), and the picker's own cycle: pages with generation and
//! search ids (a stale reply is dropped), pagination, sources, archive,
//! restore, the previews and the thread hand-off to the app (ADR-003).
//! Oracle data: `resume_picker.rs` (`PickerState`, `PaginationState`,
//! `ThreadCatalog`) and its tests; the oracle keeps this state in
//! `PickerState`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::agents::{
    Command as AgentsCommand, Ev as AgentsEv, Reply as AgentsReply,
};
use codex_tui_core::screens::resume::{
    Action, ArchiveState, Command as ResumeCommand, Control, CwdFilter, Dir, Ev, Filter,
    Intent as ResumeIntent, Launch, Load, PageMode, Pick, Preview, Problem, Reply as ResumeReply,
    Row, Scope, Sort, Status, TranscriptState,
};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply};
use codex_tui_core::threads::ThreadAsk;
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);

fn play(steps: &[&str]) -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{}\n", steps.join("\n"))).expect("plays")
}

const LISTED: &str = r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[
{"id":"a","preview":"Fix resume picker timestamps","cloud":true,"created":1,"updated":2},
{"id":"b","preview":"Investigate lazy pagination cap","unread":true,"thread_name":"Pagination"}]}}}"#;

#[test]
fn a_listing_fills_the_rows_and_ends_the_loading() {
    let loading = r#"{"step":"screen","dir":"resume","ev":{"loading":null}}"#;
    let sandbox = play(&[loading]);
    assert_eq!(sandbox.app().screens().resume.load, Load::Loading);
    let one_line = LISTED.replace('\n', "");
    let sandbox = play(&[loading, &one_line]);
    let model = &sandbox.app().screens().resume;
    assert_eq!(model.load, Load::Idle);
    assert_eq!(model.rows.len(), 2);
    assert!(model.rows[0].cloud);
    assert!(model.rows[1].unread);
}

#[test]
fn a_thread_name_replaces_the_preview() {
    // Oracle: `Row::display_preview`.
    let one_line = LISTED.replace('\n', "");
    let sandbox = play(&[&one_line]);
    let rows = &sandbox.app().screens().resume.rows;
    assert_eq!(rows[0].display_preview(), "Fix resume picker timestamps");
    assert_eq!(rows[1].display_preview(), "Pagination");
}

#[test]
fn a_failure_keeps_its_message_and_the_next_listing_clears_it() {
    let failed = r#"{"step":"screen","dir":"resume","ev":{"failed":"Failed to read session metadata from /tmp/missing.jsonl"}}"#;
    let sandbox = play(&[failed]);
    assert_eq!(
        sandbox.app().screens().resume.load,
        Load::Failed("Failed to read session metadata from /tmp/missing.jsonl".into())
    );
    let listed = r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[]}}}"#;
    let sandbox = play(&[failed, listed]);
    assert_eq!(sandbox.app().screens().resume.load, Load::Idle);
}

#[test]
fn the_clock_is_a_model_input() {
    let sandbox = play(&[r#"{"step":"screen","dir":"resume","ev":{"clock":1800000000}}"#]);
    assert_eq!(sandbox.app().screens().resume.clock, 1_800_000_000);
}

/// The skeleton's seed form (three fake rollouts) still reads: the widened
/// `Row` defaults its new fields.
#[test]
fn the_skeleton_fake_rollout_form_still_seeds() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[{"id":"x","preview":"Hello","model_provider":"mock_provider","timestamp":"2025-01-02T12:00:00Z"}]}}}"#,
    ]);
    let row = &sandbox.app().screens().resume.rows[0];
    assert_eq!(row.model_provider, "mock_provider");
    assert_eq!(row.created, None);
}

// The picker's cycle, over the app (the effects it asks for are read).

fn row(id: &str, updated: i64) -> Row {
    Row {
        id: ThreadId::new(id),
        preview: format!("session {id}"),
        model_provider: String::new(),
        started: String::new(),
        thread_name: None,
        created: Some(updated),
        updated: Some(updated),
        cwd: None,
        branch: None,
        cloud: false,
        unread: false,
        backend: LOCAL,
        path: None,
    }
}

fn page(ids: &[(&str, i64)], next: Option<&str>) -> ResumeReply {
    ResumeReply::Page {
        rows: ids.iter().map(|(id, at)| row(id, *at)).collect(),
        next: next.map(str::to_owned),
        unread: Vec::new(),
    }
}

fn send(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn intent(app: &mut CodexApp, intent: ResumeIntent) -> AppFx {
    send(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Resume(intent))),
    )
}

fn event(app: &mut CodexApp, ev: Ev) {
    send(app, AppEv::Screen(ScreenEv::Resume(ev)));
}

fn event_agents(app: &mut CodexApp, ev: AgentsEv) {
    send(app, AppEv::Screen(ScreenEv::Agents(ev)));
}

fn answer(app: &mut CodexApp, ticket: Ticket, reply: ResumeReply) -> AppFx {
    send(
        app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Resume(reply))),
    )
}

/// The reply of a thread's history (the turns of a preview or a transcript).
fn turns(app: &mut CodexApp, ticket: Ticket) -> AppFx {
    let reply = AgentsReply::History {
        turns: Vec::new(),
        next: None,
    };
    send(
        app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Agents(reply))),
    )
}

/// The thread-turns RPCs `fx` asks for: the ticket and the thread.
fn turns_asked(fx: &AppFx) -> Vec<(Ticket, String)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(AgentsCommand::Thread(ThreadAsk::History { thread, .. })),
                ..
            } => Some((*ticket, thread.clone())),
            _ => None,
        })
        .collect()
}

fn fail(app: &mut CodexApp, ticket: Ticket, message: &str) -> AppFx {
    send(
        app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote(message.into()))),
    )
}

/// The picker's RPCs `fx` asks for, in order.
fn asked(fx: &AppFx) -> Vec<(Ticket, ResumeCommand)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Resume(command),
                ..
            } => Some((*ticket, command.clone())),
            _ => None,
        })
        .collect()
}

fn only(fx: &AppFx) -> (Ticket, ResumeCommand) {
    let found = asked(fx);
    assert_eq!(found.len(), 1, "one picker RPC in {:?}", fx.commands());
    found.into_iter().next().unwrap()
}

fn cursor_of(command: &ResumeCommand) -> Option<String> {
    match command {
        ResumeCommand::List { cursor, .. } => cursor.clone(),
        other => panic!("not a list: {other:?}"),
    }
}

fn opened() -> (CodexApp, Ticket) {
    let mut app = CodexApp::new();
    let fx = intent(&mut app, ResumeIntent::Open);
    let (ticket, command) = only(&fx);
    assert_eq!(cursor_of(&command), None, "the first page");
    (app, ticket)
}

/// An opened picker whose first page (ids with descending ages) is in.
fn listed(ids: &[&str], next: Option<&str>) -> CodexApp {
    let (mut app, ticket) = opened();
    let rows: Vec<(&str, i64)> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, 1000 - index as i64))
        .collect();
    answer(&mut app, ticket, page(&rows, next));
    app
}

fn names(app: &CodexApp) -> Vec<String> {
    app.screens()
        .resume
        .visible()
        .iter()
        .map(|row| row.id.to_string())
        .collect()
}

#[test]
fn opening_lists_the_first_page_of_every_source() {
    let mut app = CodexApp::new();
    event_agents(&mut app, AgentsEv::CloudBackend(CLOUD));
    let fx = intent(&mut app, ResumeIntent::Open);
    let found = asked(&fx);
    assert_eq!(found.len(), 2, "one first page per connected backend");
    let mut backends: Vec<_> = found.iter().map(|(_, command)| command.backend()).collect();
    backends.sort();
    assert_eq!(backends, vec![LOCAL, CLOUD]);
    assert_eq!(app.screens().resume.load, Load::Loading);
}

#[test]
fn a_page_adds_its_rows_sorted_and_without_duplicates() {
    let (mut app, ticket) = opened();
    answer(&mut app, ticket, page(&[("a", 10), ("b", 30)], Some("c1")));
    assert_eq!(names(&app), vec!["b", "a"], "newest first");
    assert_eq!(app.screens().resume.load, Load::Idle);
    // A later page repeating a row does not add it twice.
    let fx = intent(&mut app, ResumeIntent::LoadMore);
    let (ticket, command) = only(&fx);
    assert_eq!(cursor_of(&command).as_deref(), Some("c1"));
    answer(&mut app, ticket, page(&[("b", 30), ("c", 20)], None));
    assert_eq!(names(&app), vec!["b", "c", "a"]);
}

#[test]
fn a_reply_of_an_older_listing_is_dropped() {
    // Oracle: a reset listing invalidates every pending page (`listing_token`,
    // `request_token`). Changing the sort restarts the listing.
    let (mut app, first) = opened();
    let fx = intent(&mut app, ResumeIntent::Change(Control::Sort, Dir::Next));
    let (second, command) = only(&fx);
    assert!(matches!(
        command,
        ResumeCommand::List {
            sort: Sort::Created,
            ..
        }
    ));
    assert_eq!(app.screens().resume.sort, Sort::Created);

    let stale = answer(&mut app, first, page(&[("old", 5)], Some("more")));
    assert!(asked(&stale).is_empty(), "a stale page starts nothing");
    assert!(app.screens().resume.rows.is_empty(), "and adds nothing");
    assert_eq!(app.screens().resume.load, Load::Loading);

    answer(&mut app, second, page(&[("new", 6)], None));
    assert_eq!(names(&app), vec!["new"]);
}

#[test]
fn a_repeated_reply_is_taken_once() {
    let (mut app, ticket) = opened();
    answer(&mut app, ticket, page(&[("a", 1)], None));
    answer(&mut app, ticket, page(&[("b", 2)], None));
    assert_eq!(names(&app), vec!["a"]);
}

#[test]
fn a_search_reads_pages_until_a_row_matches() {
    // Oracle: `continue_search_if_needed`.
    let mut app = listed(&["alpha", "beta"], Some("c1"));
    let fx = intent(&mut app, ResumeIntent::Query("needle".into()));
    let (ticket, command) = only(&fx);
    assert_eq!(cursor_of(&command).as_deref(), Some("c1"));
    assert!(app.screens().resume.listing.search.is_some());

    let fx = answer(&mut app, ticket, page(&[("gamma", 3)], Some("c2")));
    let (ticket, command) = only(&fx);
    assert_eq!(
        cursor_of(&command).as_deref(),
        Some("c2"),
        "still nothing found"
    );

    let mut found = row("delta", 2);
    found.preview = "the needle".into();
    let fx = answer(
        &mut app,
        ticket,
        ResumeReply::Page {
            rows: vec![found],
            next: Some("c3".into()),
            unread: Vec::new(),
        },
    );
    assert!(asked(&fx).is_empty(), "a match ends the search");
    assert_eq!(app.screens().resume.listing.search, None);
    assert_eq!(names(&app), vec!["delta"]);
}

#[test]
fn a_search_ends_when_the_sources_end() {
    let mut app = listed(&["alpha"], Some("c1"));
    let fx = intent(&mut app, ResumeIntent::Query("zzz".into()));
    let (ticket, _) = only(&fx);
    let fx = answer(&mut app, ticket, page(&[("beta", 2)], None));
    assert!(asked(&fx).is_empty());
    assert_eq!(app.screens().resume.listing.search, None);
    assert!(names(&app).is_empty());
}

#[test]
fn a_page_of_an_older_search_brings_its_rows_and_the_current_search_goes_on() {
    // The race: the user types again while the page of the first search is in
    // flight. The old page's rows are kept; what is read next serves the new
    // search, and the page does not stall it.
    let mut app = listed(&["alpha"], Some("c1"));
    let fx = intent(&mut app, ResumeIntent::Query("one".into()));
    let (old_ticket, _) = only(&fx);
    let first_token = app.screens().resume.listing.search.expect("search one");
    let fx = intent(&mut app, ResumeIntent::Query("two".into()));
    assert!(
        asked(&fx).is_empty(),
        "a page is in flight: no second request"
    );
    let second_token = app.screens().resume.listing.search.expect("search two");
    assert_ne!(first_token, second_token, "each search has its own token");

    let fx = answer(&mut app, old_ticket, page(&[("beta", 2)], Some("c2")));
    assert_eq!(
        app.screens().resume.rows.len(),
        2,
        "the old page's rows are in"
    );
    let (ticket, command) = only(&fx);
    assert_eq!(cursor_of(&command).as_deref(), Some("c2"));

    let mut found = row("gamma", 1);
    found.preview = "about two".into();
    answer(
        &mut app,
        ticket,
        ResumeReply::Page {
            rows: vec![found],
            next: None,
            unread: Vec::new(),
        },
    );
    assert_eq!(names(&app), vec!["gamma"]);
}

#[test]
fn clearing_the_query_ends_the_search() {
    let mut app = listed(&["alpha"], Some("c1"));
    intent(&mut app, ResumeIntent::Query("zzz".into()));
    assert!(app.screens().resume.listing.search.is_some());
    intent(&mut app, ResumeIntent::ClearQuery);
    assert_eq!(app.screens().resume.listing.search, None);
    assert_eq!(app.screens().resume.query, "");
}

#[test]
fn a_paste_is_folded_into_the_query() {
    // Oracle: `normalize_pasted_search_query`.
    let mut app = listed(&["alpha"], None);
    intent(&mut app, ResumeIntent::Paste("  two\n words ".into()));
    assert_eq!(app.screens().resume.query, "two words");
    intent(&mut app, ResumeIntent::Paste("more".into()));
    assert_eq!(app.screens().resume.query, "two words more");
}

#[test]
fn reaching_for_more_asks_for_the_next_page_one_at_a_time() {
    // Oracle: `load_more_if_needed`. When the list reaches (the cursor near
    // its end, room left, the End key) is the page's rule; core reads a page
    // for the reach, once, and none when the sources ended.
    let mut app = listed(&["a", "b"], Some("c1"));
    let fx = intent(&mut app, ResumeIntent::LoadMore);
    let (ticket, command) = only(&fx);
    assert_eq!(cursor_of(&command).as_deref(), Some("c1"));
    assert!(
        asked(&intent(&mut app, ResumeIntent::LoadMore)).is_empty(),
        "one page at a time"
    );
    answer(&mut app, ticket, page(&[("c", 1)], None));
    assert!(
        asked(&intent(&mut app, ResumeIntent::LoadMore)).is_empty(),
        "the list ended"
    );
}

#[test]
fn dismissing_the_problem_clears_the_line_under_the_list() {
    let mut app = listed(&["a"], None);
    event(&mut app, Ev::Problem(Problem::NoTranscript));
    assert_eq!(
        app.screens().resume.inline_error,
        Some(Problem::NoTranscript)
    );
    let fx = intent(&mut app, ResumeIntent::DismissProblem);
    assert!(asked(&fx).is_empty());
    assert_eq!(app.screens().resume.inline_error, None);
}

#[test]
fn sources_page_on_their_own_cursors_and_one_failing_does_not_stop_the_other() {
    // Oracle: `SourceError` ("Could not load one backend...").
    let mut app = CodexApp::new();
    event_agents(&mut app, AgentsEv::CloudBackend(CLOUD));
    let fx = intent(&mut app, ResumeIntent::Open);
    let found = asked(&fx);
    let local = found.iter().find(|(_, c)| c.backend() == LOCAL).unwrap().0;
    let cloud = found.iter().find(|(_, c)| c.backend() == CLOUD).unwrap().0;
    answer(&mut app, local, page(&[("l1", 5)], Some("local-next")));
    fail(&mut app, cloud, "cloud is down");
    let model = &app.screens().resume;
    assert_eq!(model.load, Load::Idle);
    assert_eq!(model.inline_error, Some(Problem::OneBackend));
    assert_eq!(names(&app), vec!["l1"]);
    // The next page goes to the live source only.
    let fx = intent(&mut app, ResumeIntent::LoadMore);
    let (_, command) = only(&fx);
    assert_eq!(command.backend(), LOCAL);
    assert_eq!(cursor_of(&command).as_deref(), Some("local-next"));
}

#[test]
fn a_failed_first_page_shows_on_the_search_line() {
    let (mut app, ticket) = opened();
    fail(&mut app, ticket, "boom");
    assert_eq!(app.screens().resume.load, Load::Failed("boom".into()));
}

#[test]
fn a_failed_later_page_is_an_inline_error_and_the_rows_stay() {
    let mut app = listed(&["a"], Some("c1"));
    let fx = intent(&mut app, ResumeIntent::LoadMore);
    let (ticket, _) = only(&fx);
    fail(&mut app, ticket, "boom");
    assert_eq!(app.screens().resume.inline_error, Some(Problem::MoreFailed));
    assert_eq!(names(&app), vec!["a"]);
    assert!(
        asked(&intent(&mut app, ResumeIntent::LoadMore)).is_empty(),
        "no retry loop"
    );
}

fn db_first_app() -> CodexApp {
    let mut app = CodexApp::new();
    event(
        &mut app,
        Ev::Launched {
            action: Action::Resume,
            launch: Launch::Startup,
            db_first: true,
        },
    );
    app
}

#[test]
fn the_first_empty_indexed_page_falls_back_to_the_stores_listing() {
    // Oracle: `should_restart_from_store`.
    let mut app = db_first_app();
    let fx = intent(&mut app, ResumeIntent::Open);
    let (ticket, command) = only(&fx);
    assert!(matches!(
        command,
        ResumeCommand::List {
            mode: PageMode::StateDbOnly,
            ..
        }
    ));
    let fx = answer(&mut app, ticket, page(&[], None));
    let (ticket, command) = only(&fx);
    assert!(matches!(
        command,
        ResumeCommand::List {
            mode: PageMode::StoreDefault,
            ..
        }
    ));
    answer(&mut app, ticket, page(&[("a", 1)], None));
    assert_eq!(names(&app), vec!["a"]);
}

#[test]
fn an_empty_later_page_of_the_index_ends_the_list() {
    let mut app = db_first_app();
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::Open));
    answer(&mut app, ticket, page(&[("a", 1)], Some("c1")));
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::LoadMore));
    let fx = answer(&mut app, ticket, page(&[], None));
    assert!(
        asked(&fx).is_empty(),
        "no fallback after the index had rows"
    );
    assert_eq!(names(&app), vec!["a"]);
}

#[test]
fn the_toolbar_changes_restart_the_listing_with_what_they_ask_for() {
    let mut app = CodexApp::new();
    event(
        &mut app,
        Ev::Scope(Scope {
            cwd: Some("/work/app".into()),
            repo_cwds: vec!["/work/app".into(), "/work/app-wt".into()],
        }),
    );
    assert_eq!(app.screens().resume.filter, Filter::Cwd);
    let (_, command) = only(&intent(&mut app, ResumeIntent::Open));
    assert!(matches!(
        command,
        ResumeCommand::List { cwd: Some(CwdFilter::One(ref cwd)), .. } if cwd == "/work/app"
    ));
    let generation = app.screens().resume.listing.generation;
    // Right on the filter: the repository's checkouts.
    let (_, command) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Filter, Dir::Next),
    ));
    assert_eq!(app.screens().resume.filter, Filter::Repo);
    assert!(matches!(
        command,
        ResumeCommand::List { cwd: Some(CwdFilter::Many(ref dirs)), .. } if dirs.len() == 2
    ));
    assert!(app.screens().resume.listing.generation > generation);
    // Tab to the status: archived sessions.
    let (_, command) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Status, Dir::Next),
    ));
    assert_eq!(app.screens().resume.status, Status::Archived);
    assert!(matches!(
        command,
        ResumeCommand::List { archived: true, .. }
    ));
}

/// A scope with a working directory and one linked checkout: the three
/// filters are on offer.
fn scoped_app() -> CodexApp {
    let mut app = CodexApp::new();
    event(
        &mut app,
        Ev::Scope(Scope {
            cwd: Some("/work/app".into()),
            repo_cwds: vec!["/work/app".into(), "/work/app-wt".into()],
        }),
    );
    app
}

#[test]
fn a_picked_value_is_set_and_restarts_the_listing() {
    // A tap names the value (Flutter's `SegmentedButton.onSelectionChanged`),
    // where an arrow steps to the neighbour: the listing restarts either way.
    let mut app = scoped_app();
    let (first, _) = only(&intent(&mut app, ResumeIntent::Open));
    let generation = app.screens().resume.listing.generation;
    let (second, command) = only(&intent(
        &mut app,
        ResumeIntent::Pick(Pick::Sort(Sort::Created)),
    ));
    assert_eq!(app.screens().resume.sort, Sort::Created);
    assert!(matches!(
        command,
        ResumeCommand::List {
            sort: Sort::Created,
            ..
        }
    ));
    assert!(app.screens().resume.listing.generation > generation);
    // The page of the listing it replaced is dropped.
    answer(&mut app, first, page(&[("old", 5)], None));
    assert!(app.screens().resume.rows.is_empty());
    answer(&mut app, second, page(&[("new", 6)], None));
    // (The rows kept, not the visible ones: the directory filter is on.)
    assert_eq!(app.screens().resume.rows.len(), 1);
    let (_, command) = only(&intent(
        &mut app,
        ResumeIntent::Pick(Pick::Status(Status::Archived)),
    ));
    assert_eq!(app.screens().resume.status, Status::Archived);
    assert!(matches!(
        command,
        ResumeCommand::List { archived: true, .. }
    ));
    let (_, command) = only(&intent(
        &mut app,
        ResumeIntent::Pick(Pick::Filter(Filter::Repo)),
    ));
    assert_eq!(app.screens().resume.filter, Filter::Repo);
    assert!(matches!(
        command,
        ResumeCommand::List {
            cwd: Some(CwdFilter::Many(_)),
            ..
        }
    ));
    // A pick jumps over the values between (the arrows step through them).
    let (_, command) = only(&intent(
        &mut app,
        ResumeIntent::Pick(Pick::Filter(Filter::All)),
    ));
    assert_eq!(app.screens().resume.filter, Filter::All);
    assert!(matches!(command, ResumeCommand::List { cwd: None, .. }));
}

#[test]
fn picking_the_value_in_force_does_nothing() {
    let mut app = scoped_app();
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::Open));
    let generation = app.screens().resume.listing.generation;
    for pick in [
        Pick::Filter(Filter::Cwd),
        Pick::Status(Status::Active),
        Pick::Sort(Sort::Updated),
    ] {
        let fx = intent(&mut app, ResumeIntent::Pick(pick));
        assert!(asked(&fx).is_empty(), "{pick:?} asks nothing");
    }
    assert_eq!(app.screens().resume.listing.generation, generation);
    // The listing already asked for still counts.
    answer(&mut app, ticket, page(&[("a", 1)], None));
    assert_eq!(app.screens().resume.rows.len(), 1);
}

#[test]
fn a_filter_the_picker_does_not_offer_is_refused() {
    // No repository checkouts: the repository filter is not offered; no
    // working directory: neither is the directory filter.
    let mut app = CodexApp::new();
    event(
        &mut app,
        Ev::Scope(Scope {
            cwd: Some("/work/app".into()),
            repo_cwds: Vec::new(),
        }),
    );
    intent(&mut app, ResumeIntent::Open);
    let fx = intent(&mut app, ResumeIntent::Pick(Pick::Filter(Filter::Repo)));
    assert!(asked(&fx).is_empty());
    assert_eq!(app.screens().resume.filter, Filter::Cwd);
    let mut app = CodexApp::new();
    intent(&mut app, ResumeIntent::Open);
    assert_eq!(app.screens().resume.filter, Filter::All);
    let fx = intent(&mut app, ResumeIntent::Pick(Pick::Filter(Filter::Cwd)));
    assert!(asked(&fx).is_empty());
    assert_eq!(app.screens().resume.filter, Filter::All);
}

#[test]
fn the_repository_filter_shows_the_rows_of_linked_checkouts() {
    // Oracle: `linked_worktree_sessions_follow_directory_scope`: a session of a linked checkout
    // is hidden by the directory filter and shown by the repository filter, which asks again.
    let mut app = CodexApp::new();
    event(
        &mut app,
        Ev::Scope(Scope {
            cwd: Some("/repo/primary".into()),
            repo_cwds: vec!["/repo/primary".into(), "/repo/linked".into()],
        }),
    );
    let linked = || {
        let mut linked = row("linked", 10);
        linked.cwd = Some("/repo/linked".into());
        ResumeReply::Page {
            rows: vec![linked],
            next: None,
            unread: Vec::new(),
        }
    };
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::Open));
    answer(&mut app, ticket, linked());
    assert!(names(&app).is_empty(), "the directory filter hides it");
    let (ticket, _) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Filter, Dir::Next),
    ));
    answer(&mut app, ticket, linked());
    assert_eq!(app.screens().resume.filter, Filter::Repo);
    assert_eq!(names(&app), vec!["linked"]);
}

// Preview and transcript.

#[test]
fn opening_a_rows_details_reads_its_turns_once_as_the_threads_own_rpc() {
    let mut app = listed(&["a"], None);
    let id = ThreadId::new("a");
    let fx = intent(&mut app, ResumeIntent::LoadPreview(id.clone()));
    assert!(asked(&fx).is_empty(), "not a picker RPC: the thread's own");
    let found = turns_asked(&fx);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, "a");
    let key = (LOCAL, id.clone());
    assert_eq!(
        app.screens().resume.previews.get(&key),
        Some(&Preview::Loading)
    );
    turns(&mut app, found[0].0);
    assert_eq!(
        app.screens().resume.previews.get(&key),
        Some(&Preview::Loaded(Vec::new()))
    );
    // Opened again (the page collapsed and expanded the row): the cached
    // preview is used.
    let fx = intent(&mut app, ResumeIntent::LoadPreview(id));
    assert!(turns_asked(&fx).is_empty());
}

#[test]
fn a_failed_preview_is_marked_failed() {
    let mut app = listed(&["a"], None);
    let id = ThreadId::new("a");
    let fx = intent(&mut app, ResumeIntent::LoadPreview(id.clone()));
    fail(&mut app, turns_asked(&fx)[0].0, "no such thread");
    assert_eq!(
        app.screens().resume.previews.get(&(LOCAL, id)),
        Some(&Preview::Failed)
    );
}

#[test]
fn a_late_preview_of_an_archived_row_is_ignored() {
    // Oracle: "Archiving removes the owned cache entry; ignore its late preview."
    let mut app = listed(&["a", "b"], None);
    let preview = turns_asked(&intent(
        &mut app,
        ResumeIntent::LoadPreview(ThreadId::new("a")),
    ))[0]
        .0;
    let (archive, _) = only(&intent(&mut app, ResumeIntent::Archive(ThreadId::new("a"))));
    answer(&mut app, archive, ResumeReply::Archived);
    turns(&mut app, preview);
    assert!(app.screens().resume.previews.is_empty());
}

#[test]
fn a_transcript_is_asked_for_and_marked_loaded() {
    let mut app = listed(&["a"], None);
    let id = ThreadId::new("a");
    let fx = intent(&mut app, ResumeIntent::OpenTranscript(id));
    let ticket = turns_asked(&fx)[0].0;
    let transcript = app.screens().resume.transcript.clone().expect("loading");
    assert_eq!(transcript.state, TranscriptState::Loading);
    turns(&mut app, ticket);
    let transcript = app.screens().resume.transcript.clone().expect("loaded");
    assert_eq!(transcript.state, TranscriptState::Loaded { turns: 0 });
}

#[test]
fn esc_cancels_a_transcript_that_is_loading_and_its_late_reply_is_dropped() {
    let mut app = listed(&["a"], None);
    let fx = intent(&mut app, ResumeIntent::OpenTranscript(ThreadId::new("a")));
    let ticket = turns_asked(&fx)[0].0;
    intent(&mut app, ResumeIntent::CancelTranscript);
    assert_eq!(app.screens().resume.transcript, None);
    turns(&mut app, ticket);
    assert_eq!(app.screens().resume.transcript, None);
}

// Archive and restore.

#[test]
fn archiving_a_row_removes_it_and_a_page_in_flight_cannot_bring_it_back() {
    // Oracle: the archived session's seen key stays as a tombstone.
    let mut app = listed(&["a", "b"], Some("c1"));
    let (page_ticket, _) = only(&intent(&mut app, ResumeIntent::LoadMore));
    let (archive, command) = only(&intent(&mut app, ResumeIntent::Archive(ThreadId::new("a"))));
    assert!(matches!(command, ResumeCommand::Archive { .. }));
    assert!(matches!(
        app.screens().resume.archive,
        ArchiveState::Pending(_)
    ));
    answer(&mut app, archive, ResumeReply::Archived);
    assert_eq!(names(&app), vec!["b"]);
    assert_eq!(app.screens().resume.archive, ArchiveState::Idle);
    answer(&mut app, page_ticket, page(&[("a", 1000), ("c", 5)], None));
    assert_eq!(names(&app), vec!["b", "c"], "a stays archived");
}

#[test]
fn one_archive_runs_at_a_time() {
    let mut app = listed(&["a", "b"], None);
    only(&intent(&mut app, ResumeIntent::Archive(ThreadId::new("a"))));
    let fx = intent(&mut app, ResumeIntent::Archive(ThreadId::new("b")));
    assert!(asked(&fx).is_empty());
}

#[test]
fn a_failed_archive_says_so_and_keeps_the_row() {
    let mut app = listed(&["a"], None);
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::Archive(ThreadId::new("a"))));
    fail(&mut app, ticket, "disk full");
    assert_eq!(
        app.screens().resume.inline_error,
        Some(Problem::ArchiveFailed("disk full".into()))
    );
    assert_eq!(names(&app), vec!["a"]);
    assert_eq!(app.screens().resume.archive, ArchiveState::Idle);
}

#[test]
fn the_current_session_is_not_archived_from_the_picker() {
    let mut app = listed(&["current", "other"], None);
    event(
        &mut app,
        Ev::Launched {
            action: Action::Resume,
            launch: Launch::Session {
                current: ThreadId::new("current"),
            },
            db_first: false,
        },
    );
    let fx = intent(&mut app, ResumeIntent::Archive(ThreadId::new("current")));
    assert!(asked(&fx).is_empty());
    assert_eq!(
        app.screens().resume.inline_error,
        Some(Problem::CurrentSession)
    );
    only(&intent(
        &mut app,
        ResumeIntent::Archive(ThreadId::new("other")),
    ));
}

#[test]
fn archiving_is_not_offered_in_the_archived_list() {
    let mut app = listed(&["a"], None);
    let (ticket, _) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Status, Dir::Next),
    ));
    answer(&mut app, ticket, page(&[("a", 1)], None));
    assert_eq!(app.screens().resume.status, Status::Archived);
    assert!(asked(&intent(&mut app, ResumeIntent::Archive(ThreadId::new("a")))).is_empty());
}

// The hand-off to the app (ADR-003).

fn resumed_thread(fx: &AppFx) -> Option<String> {
    fx.commands().iter().find_map(|command| match command {
        Command::Screen {
            command: ScreenCommand::Agents(AgentsCommand::Thread(ThreadAsk::Resume { thread })),
            ..
        } => Some(thread.clone()),
        _ => None,
    })
}

#[test]
fn enter_on_a_row_asks_the_app_to_open_that_thread() {
    // The picker never resumes: the app's `open_thread` does (OpenThread).
    let mut app = listed(&["a", "b"], None);
    let fx = intent(&mut app, ResumeIntent::Resume(ThreadId::new("b")));
    assert_eq!(resumed_thread(&fx).as_deref(), Some("b"));
    assert_eq!(app.screens().resume.resuming, None, "taken by the app");
}

#[test]
fn a_row_that_is_not_listed_opens_nothing_and_says_so() {
    let mut app = listed(&["a"], None);
    let fx = intent(&mut app, ResumeIntent::Resume(ThreadId::new("ghost")));
    assert_eq!(resumed_thread(&fx), None);
    assert_eq!(
        app.screens().resume.inline_error,
        Some(Problem::NoMetadata(None))
    );
}

#[test]
fn enter_in_the_archived_list_restores_the_session_and_then_opens_it() {
    let mut app = listed(&["a"], None);
    let (ticket, _) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Status, Dir::Next),
    ));
    answer(&mut app, ticket, page(&[("a", 1)], None));

    let fx = intent(&mut app, ResumeIntent::Resume(ThreadId::new("a")));
    let (ticket, command) = only(&fx);
    assert!(matches!(command, ResumeCommand::Unarchive { .. }));
    assert_eq!(resumed_thread(&fx), None, "not before the restore");
    assert!(matches!(
        app.screens().resume.archive,
        ArchiveState::Restoring(_)
    ));
    let fx = answer(
        &mut app,
        ticket,
        ResumeReply::Unarchived {
            thread: ThreadId::new("a"),
            cwd: Some("/work".into()),
        },
    );
    assert_eq!(resumed_thread(&fx).as_deref(), Some("a"));
    assert_eq!(app.screens().resume.archive, ArchiveState::Idle);
}

#[test]
fn a_failed_restore_says_so() {
    let mut app = listed(&["a"], None);
    let (ticket, _) = only(&intent(
        &mut app,
        ResumeIntent::Change(Control::Status, Dir::Next),
    ));
    answer(&mut app, ticket, page(&[("a", 1)], None));
    let (ticket, _) = only(&intent(&mut app, ResumeIntent::Resume(ThreadId::new("a"))));
    let fx = fail(&mut app, ticket, "gone");
    assert_eq!(resumed_thread(&fx), None);
    assert_eq!(
        app.screens().resume.inline_error,
        Some(Problem::RestoreFailed("gone".into()))
    );
}

#[test]
fn a_lost_connection_fails_the_pages_it_owed() {
    // The reconnect drops the tickets of the lost backend; its page fails so
    // the picker does not wait for a reply that never comes.
    let (mut app, _ticket) = opened();
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Reconnecting)),
    );
    assert_eq!(
        app.screens().resume.load,
        Load::Failed("connection lost".into())
    );
}
