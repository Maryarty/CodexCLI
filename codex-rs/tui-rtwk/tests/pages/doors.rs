// test root: pages
//! The doors of the W2 round-1 "needs first" task (doors-r1): one test per door, each the
//! smallest use a package makes of it. The features that use them have their own tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use rtwk::focus::KeyPress;
use rtwk_core::input::{KeyEvent, Mods};

use codex_tui_rtwk::keymap::press;

/// A press as the terminal reports it under a Cyrillic layout: the character typed, the key it
/// is on a US layout.
fn layout_press(typed: &str, base: char, mods: Mods) -> KeyPress {
    let event = KeyEvent::char(typed).with_mods(mods).with_base_char(base);
    KeyPress::from_event(&event).expect("a character key is a press")
}

#[test]
fn ctrl_c_is_the_key_on_a_us_layout_too() {
    assert!(press::is_ctrl_c(&layout_press("c", 'c', Mods::CTRL)));
    // a Cyrillic layout's Ctrl+с is Ctrl+C (oracle #35599)
    assert!(press::is_ctrl_c(&layout_press("с", 'c', Mods::CTRL)));
    assert!(!press::is_ctrl_c(&layout_press("с", 'c', Mods::NONE)));
    assert!(!press::is_ctrl_c(&layout_press("в", 'd', Mods::CTRL)));
}

mod presets_chord {
    use codex_tui_core::app::AppEv;

    use crate::common::start;

    const KEYS: &str = "n · new   s · from settings   e · edit   d · delete   esc · close";

    /// `Ctrl+X p`, the default of `open_presets` (oracle `leader_defaults`), opens the menu of
    /// `/presets` as the command does, over a draft too.
    #[test]
    fn ctrl_x_p_opens_the_presets_menu() {
        let mut h = start();
        h.wait_idle();
        h.inject([AppEv::Presets(Vec::new())]);
        h.t.type_text("draft");
        h.press("ctrl+x");
        h.press("p");
        h.wait_for("the menu", |s| s.contains(KEYS));
        assert!(h.t.screen().contains("draft"), "{}", h.t.screen());
    }

    /// A key that is not the chord's completion is the composer's.
    #[test]
    fn ctrl_x_then_another_key_leaves_the_menu_closed() {
        let mut h = start();
        h.wait_idle();
        h.press("ctrl+x");
        h.press("q");
        h.wait_idle();
        assert!(!h.t.screen().contains(KEYS), "{}", h.t.screen());
    }
}

/// A `Picker` on its own: a host that records the events the list emits.
mod picker {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rtwk::provider::{MultiProvider, Provider};
    use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
    use rtwk_test::{Options, Tester};

    use codex_tui_core::keymap::Keymap;
    use codex_tui_rtwk::widgets::picker::list::{Item, List, Role, Span};
    use codex_tui_rtwk::widgets::picker::{Event, Picker};

    #[derive(Clone)]
    struct Host {
        list: Rc<List>,
        seen: Rc<RefCell<Vec<Event>>>,
    }

    impl PartialEq for Host {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.list, &other.list)
        }
    }

    struct HostState;

    impl StatefulWidget for Host {
        type State = HostState;
        fn create_state(&self) -> HostState {
            HostState
        }
    }

    impl_widget!(stateful eq Host);

    impl State for HostState {
        type Widget = Host;
        type Msg = Event;

        fn build(&self, w: &Host, cx: &mut Cx<'_, Self>) -> AnyWidget {
            // the app provides its keymap above every widget (`app/view.rs`)
            MultiProvider::new()
                .with(Provider::value_shared(Rc::new(Keymap::defaults())))
                .child(Picker::new(w.list.clone(), cx.map(|event: Event| event)))
                .into()
        }

        fn update(&mut self, event: Event, w: &Host, _: &mut Cx<'_, Self>) -> Rebuild {
            w.seen.borrow_mut().push(event);
            Rebuild::No
        }
    }

    fn mount(list: List, width: u16) -> (Tester, Rc<RefCell<Vec<Event>>>) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut t = Tester::with(Options::new().size(width, 20));
        t.pump_widget(Host {
            list: Rc::new(list),
            seen: seen.clone(),
        });
        (t, seen)
    }

    fn row(name: &str, id: &str) -> Item {
        Item {
            id: id.to_owned(),
            name: name.to_owned(),
            ..Item::default()
        }
    }

    /// A shortcut on a sensitive row (`require_explicit_confirmation`) puts the cursor on it and
    /// the accept key picks it; a plain row is picked by its shortcut.
    #[test]
    fn a_shortcut_on_a_sensitive_row_waits_for_the_accept_key() {
        let mut sensitive = row("Restart now", "restart");
        sensitive.explicit_confirm = true;
        let list = List {
            title: Some("Restart?".to_owned()),
            items: vec![row("Later", "later"), sensitive],
            ..List::default()
        };
        let (mut t, seen) = mount(list, 40);
        t.press("2");
        t.pump();
        assert!(seen.borrow().is_empty(), "{:?}", seen.borrow());
        t.press("enter");
        t.pump();
        assert_eq!(*seen.borrow(), [Event::Choose("restart".to_owned())]);
        seen.borrow_mut().clear();
        t.press("1");
        t.pump();
        assert_eq!(*seen.borrow(), [Event::Choose("later".to_owned())]);
    }

    const URL: &str = "https://developers.openai.com/codex/windows";

    /// The footer note is styled runs, and a run with a link names the whole destination in
    /// every cell it wraps over (`windows_sandbox_help_links_keep_complete_wrapped_destinations`).
    #[test]
    fn a_linked_run_of_the_footer_note_names_its_destination_on_every_wrapped_cell() {
        let list = List {
            items: vec![row("Yes", "yes")],
            footer_note_spans: vec![
                Span::new("Learn more: ", Role::Dim),
                Span::new(URL, Role::Info).linked(URL),
            ],
            ..List::default()
        };
        let (t, _) = mount(list, 30);
        let mut spelled = String::new();
        for y in 0..20 {
            for x in 0..30 {
                let cell = t.cell(x, y);
                if let Some(link) = &cell.link {
                    assert_eq!(link, URL, "({x}, {y})");
                    if !cell.tail {
                        spelled.push_str(&cell.text);
                    }
                }
            }
        }
        assert_eq!(spelled, URL, "{}", t.screen());
        assert!(t.screen().contains("Learn more:"), "{}", t.screen());
    }

    /// A run of the picker's header carries its link the same way
    /// (`windows_sandbox_help_links_keep_complete_wrapped_destinations`: the URL is in the header).
    #[test]
    fn a_linked_run_of_the_header_names_its_destination_on_every_wrapped_cell() {
        let list = List {
            items: vec![row("Yes", "yes")],
            header: vec![vec![
                Span::new("Learn more: ", Role::Dim),
                Span::new(URL, Role::Info).linked(URL),
            ]],
            ..List::default()
        };
        let (t, _) = mount(list, 30);
        let mut spelled = String::new();
        for y in 0..20 {
            for x in 0..30 {
                let cell = t.cell(x, y);
                if let Some(link) = &cell.link {
                    assert_eq!(link, URL, "({x}, {y})");
                    if !cell.tail {
                        spelled.push_str(&cell.text);
                    }
                }
            }
        }
        assert_eq!(spelled, URL, "{}", t.screen());
    }

    /// `Role::Good` is the success colour of `Role::Success` without its italic.
    #[test]
    fn the_good_role_is_not_italic() {
        let list = List {
            items: vec![row("Yes", "yes")],
            header: vec![vec![
                Span::new("ok ", Role::Good),
                Span::new("ready", Role::Success),
            ]],
            ..List::default()
        };
        let (t, _) = mount(list, 40);
        let at = |text: &str| {
            let screen = t.screen();
            let (y, line) = screen
                .lines()
                .enumerate()
                .find(|(_, line)| line.contains(text))
                .unwrap_or_else(|| panic!("no {text}\n{screen}"));
            (
                line.find(text).map_or(0, |at| line[..at].chars().count()),
                y,
            )
        };
        let (gx, gy) = at("ok");
        let (sx, sy) = at("ready");
        let (good, success) = (t.cell(gx, gy), t.cell(sx, sy));
        assert_eq!(good.fg, success.fg);
        assert!(!good.attrs.intersects(rtwk::style::Attrs::ITALIC));
        assert!(success.attrs.intersects(rtwk::style::Attrs::ITALIC));
    }
}

mod kit_seeds {
    use crate::kit::{Owner, Screen};

    /// `Open::side()` is the live chat inside `/side`; Ctrl+C over its empty composer returns.
    #[test]
    fn the_side_seed_opens_a_side_conversation_that_ctrl_c_closes() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(100, 12)
            .live()
            .side()
            .open();
        s.assert_contains("to switch");
        assert!(!s.side_closed());
        s.keys("ctrl+c");
        s.assert_absent("to switch");
        assert!(s.side_closed());
    }
}

mod find_in_thread_ui {
    use codex_tui_rtwk::pages::session::transcript::find::{self, Mode};
    use codex_tui_rtwk::widgets::thread_ui::{Input, ThreadUi};

    use crate::kit::{Owner, Screen};

    const NEEDLES: &str = include_str!("../oracle/fixtures/transcript_view_4/find_jump.jsonl");

    /// Find is the thread's own (`ThreadUi::find`, no registry keyed by thread), it is
    /// `Input::Find` while it is open, and its bar stands in the bottom pane below the composer.
    #[test]
    fn find_is_held_by_the_thread_ui_and_says_its_input() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(NEEDLES)
            .open();
        let ui = s.thread_ui();
        assert_eq!(ui.input(), Input::Plain);
        assert_eq!(find::finder(&ui).mode(), Mode::Closed);
        s.keys("f3");
        assert_eq!(ui.input(), Input::Find);
        assert_eq!(find::finder(&ui).mode(), Mode::Editing);
        s.type_text("needle two");
        s.keys("enter");
        assert_eq!(find::finder(&ui).mode(), Mode::Reading);
        assert!(find::reading(&ui).is_some());
        assert_eq!(ui.input(), Input::Find);
        s.keys("esc");
        assert_eq!(ui.input(), Input::Plain);
        assert!(find::reading(&ui).is_none());
        // another thread's controllers have a finder of their own
        let other = ThreadUi::default();
        assert!(find::finder(&other) != find::finder(&ui));
        assert_eq!(other.input(), Input::Plain);
    }

    /// The bar is the last thing of the bottom pane: Find's rows are below the composer's prompt.
    #[test]
    fn the_find_bar_stands_below_the_composer() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(NEEDLES)
            .open();
        s.keys("f3");
        s.type_text("needle two");
        let rows = s.rows();
        let at = |needle: &str| rows.iter().position(|row| row.contains(needle));
        let prompt = at("Ask Codex to do anything").expect("the composer");
        let bar = at("Find: ").expect("the bar");
        assert!(bar > prompt, "{}", s.text());
    }
}

mod blocks {
    use codex_tui_core::app::facts::Rendering;
    use codex_tui_rtwk::pages::session::cells::citation::blocks_of;
    use codex_tui_rtwk::pages::session::cells::strings::EN;
    use codex_tui_rtwk::pages::session::transcript::markdown_text;
    use rtwk_rich::markdown::{MdBlock, MdInline};

    /// `blocks_of` is what the cell draws: a source with a heading and two paragraphs is three
    /// blocks, the heading with its `#` marker as drawn text (a text model over them adds none),
    /// and the paragraphs read as the source's.
    #[test]
    fn blocks_of_is_the_document_the_cell_draws() {
        let blocks = blocks_of(
            "# Title\n\npara one\n\npara two",
            false,
            None,
            &EN,
            Rendering::default(),
        );
        assert_eq!(blocks.len(), 3);
        let MdBlock::Heading { children, .. } = &*blocks[0] else {
            panic!("a heading, got {:?}", blocks[0]);
        };
        assert!(matches!(&children[0], MdInline::Text(marker) if &**marker == "# "));
        assert_eq!(
            markdown_text::text(&blocks[1..]).as_deref(),
            Some("para one\n\npara two")
        );
    }
}

mod kit_snapshots {
    use std::collections::BTreeSet;

    use crate::kit::{Owner, Screen, snap_frames};

    /// `NxM name` and `name (N columns)` head the frames of a snapshot the test glued of renders.
    #[test]
    fn a_frame_header_may_be_a_size_and_a_name_or_a_name_and_a_width() {
        assert_eq!(
            snap_frames("feedback_disclosure_short_panes"),
            [
                "40x12 initial",
                "40x12 scrolled",
                "40x12 restored",
                "80x8 initial",
                "80x8 scrolled",
                "80x8 restored"
            ]
        );
        assert_eq!(
            snap_frames("feedback_disclosures"),
            [
                "external (80 columns)",
                "external (40 columns)",
                "employee (80 columns)",
                "employee (40 columns)",
                "remote server (80 columns)"
            ]
        );
    }

    /// A snapshot of two states of one screen, glued by a blank row with no header, is compared
    /// state by state (`Snap::state(height, index)`).
    #[test]
    fn two_headerless_states_are_compared_in_order() {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 4)
            .entry("feedback", "feedback_note_employee_80x4")
            .open();
        s.paste("A visible note.");
        s.snap("feedback_disclosure_four_rows").state(4, 0).assert();
        for _ in 0..16 {
            s.keys("pagedown");
        }
        s.snap("feedback_disclosure_four_rows").state(4, 1).assert();
    }

    /// The text of a link the screen wrapped over several rows is found whole, the rows' blanks
    /// and breaks not counted: the cells are the URL's own.
    #[test]
    fn a_link_is_located_over_the_rows_its_text_wraps_to() {
        let url = "https://example.test/forwarded/threads/10930?page=1&search=&filter=all&queue=customer_support_unprocessed&sort=latest_desc&forwardedScope=all";
        let s = Screen::build(Owner::ChatLive)
            .size(64, 24)
            .script(include_str!(
                "../oracle/fixtures/cells_5/user_history_cell_wraps_long_urls_inside_the_message_gutter.jsonl"
            ))
            .open();
        let cells = s
            .grid()
            .locate_cells(url)
            .expect("the url is on the screen");
        let rows: BTreeSet<usize> = cells.iter().map(|&(_, y)| y).collect();
        assert_eq!(rows.len(), 3, "{}", s.text());
        assert_eq!(cells.len(), url.len());
        // a text of one row is still found as it was
        let one = s
            .grid()
            .locate_cells("I just reprocessed")
            .expect("one row");
        assert_eq!(
            one.iter().map(|&(_, y)| y).collect::<BTreeSet<_>>().len(),
            1
        );
    }

    const APPS: &str = r#"{"step":"respond","method":"app/list","result":{"data":[{"id":"notion","name":"Notion","description":"Workspace docs","installUrl":"https://example.test/notion","isAccessible":true,"isEnabled":false,"pluginDisplayNames":[]}],"nextCursor":null}}"#;

    /// A request the table would answer at once is held in flight: the popup is loading until the
    /// test releases it, then the table's answer is the popup's rows.
    #[test]
    fn a_held_request_stays_in_flight_until_it_is_released() {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .live()
            .script(APPS)
            .open();
        let hold = s.hold_request("app/list");
        s.type_text("/apps");
        s.keys("enter");
        s.assert_contains("Loading installed and available apps...");
        s.assert_absent("Installed 1 of 1 available apps.");
        assert_eq!(s.requests_of("app/list"), 1);
        s.release(&hold);
        s.assert_contains("Installed 1 of 1 available apps.");
    }

    /// A `presets` step is what the host sends once the config is read: `/presets` opens its menu.
    #[test]
    fn a_presets_step_tells_the_app_the_saved_presets() {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(100, 20)
            .live()
            .open();
        s.feed(r#"{"step":"presets","slots":[{"slot":1,"input":"/status","label":"Status"}]}"#);
        s.type_text("/presets");
        s.keys("enter");
        s.assert_contains("n · new   s · from settings   e · edit   d · delete   esc · close");
    }
}

mod kit_states {
    use crate::kit::{Owner, Screen};

    /// `Screen::input_disabled()`: the composer takes no key and says why, as `set_input_enabled`
    /// leaves it, by the route the app has (the Windows sandbox setup running).
    #[test]
    fn the_input_disabled_seed_leaves_the_composer_off() {
        let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
        s.input_disabled();
        s.assert_contains("Input disabled until setup completes.");
        s.type_text("zq");
        s.assert_absent("› zq");
    }

    /// `Screen::attach_image_sized`: the attached file is a real PNG of the size asked.
    #[test]
    fn an_attached_image_has_the_size_the_test_gave_it() {
        let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
        let path = s.attach_image_sized("wide.png", 200, 50);
        s.assert_contains("[Image #1]");
        assert_eq!(image::image_dimensions(&path).unwrap(), (200, 50));
    }

    /// `Open::unavailable_model()`: the server's `thread/start` answer names an empty model, and
    /// the status card has none to name (`Model: loading`).
    #[test]
    fn the_unavailable_model_seed_leaves_the_thread_without_a_model() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .unavailable_model()
            .live()
            .open();
        s.type_text("/status");
        s.keys("enter").wait(300);
        s.assert_contains("Model:               loading");
    }

    /// `Open::composer_alone()` is the oracle's bare composer: the empty one of the
    /// `chat_composer` snapshots.
    #[test]
    fn the_composer_alone_seed_is_the_bare_composer() {
        let s = Screen::build(Owner::ChatLive)
            .size(100, 10)
            .composer_alone()
            .open();
        s.assert_snap("empty");
    }
}

mod fake_turns {
    use codex_tui_core::app::Command;

    use crate::kit::{Owner, Screen};

    fn interrupted(s: &Screen) -> Option<String> {
        s.commands().into_iter().find_map(|command| match command {
            Command::InterruptTurn { turn, .. } => Some(turn.as_str().to_owned()),
            _ => None,
        })
    }

    /// `Open::fake_turn_ids()`: the turn the app starts is `fake-turn-1`, apart from a script's
    /// own `turn-1`; without it the fake keeps its `turn-1`.
    #[test]
    fn the_fake_mints_its_own_turn_ids_when_asked() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 12)
            .fake_turn_ids()
            .live()
            .open();
        s.type_text("hello");
        s.keys("enter").wait_for("esc to interrupt");
        s.keys("esc").wait(200);
        assert_eq!(interrupted(&s).as_deref(), Some("fake-turn-1"));

        let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
        s.type_text("hello");
        s.keys("enter").wait_for("esc to interrupt");
        s.keys("esc").wait(200);
        assert_eq!(interrupted(&s).as_deref(), Some("turn-1"));
    }
}

mod kit_doors_r1b {
    use crate::kit::{Owner, Screen};

    /// `Screen::stream_lost` / `stream_restored`: what the user sends while the stream is lost
    /// waits in the queue, and goes out when the stream is back.
    #[test]
    fn a_lost_stream_holds_a_submit_until_it_is_restored() {
        let mut s = Screen::build(Owner::ChatLive).size(100, 14).live().open();
        s.stream_lost("Cloud stream ended. Reopen the conversation to reconnect.");
        s.type_text("hello");
        s.keys("enter");
        s.assert_contains("Queued follow-up inputs");
        s.assert_absent("Working");
        s.stream_restored();
        s.assert_absent("Queued follow-up inputs");
        s.assert_contains("Working");
    }

    /// `Screen::hold_image_preparation` / `fail_image_preparation`: the turn's request is in
    /// flight and the composer free; the failure gives the message back whole, with the words
    /// of the host's preparation.
    #[test]
    fn a_failed_image_preparation_gives_the_message_back_to_the_composer() {
        let mut s = Screen::build(Owner::ChatLive).size(100, 14).live().open();
        s.hold_image_preparation();
        s.attach_image_sized("shot.png", 8, 8);
        s.type_text(" describe");
        s.keys("enter");
        s.assert_absent("› [Image #1] describe");
        s.fail_image_preparation("Failed to prepare image: shot.png: gone");
        s.assert_contains("Failed to prepare image: shot.png: gone");
        s.assert_contains("› [Image #1] describe");
    }

    /// A `[vim_search]` key in `Open::keymap(..)` is read by the chat composer: `s` starts the
    /// query (the letters typed after it are the query, not commands), Esc drops it.
    #[test]
    fn a_vim_search_key_of_the_kit_keymap_starts_the_query() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(40, 9)
            .live()
            .keymap(&[
                ("global.toggle_vim_mode", "f9"),
                ("vim_search.forward", "s"),
            ])
            .open();
        s.type_text("draft");
        s.keys("f9 g g s");
        s.type_text("raf");
        s.keys("esc");
        s.type_text("x");
        s.assert_contains("› raft");
    }

    /// `Screen::burst`: a paste a terminal without bracketed paste sends as keys is one paste for
    /// the composer: the newline is a line of the draft, not a submit.
    #[test]
    fn a_burst_of_keys_is_one_paste_with_its_newline() {
        let mut s = Screen::build(Owner::ChatLive).size(100, 14).live().open();
        s.burst("one\ntwo");
        s.assert_contains("› one");
        s.assert_contains("two");
        s.assert_absent("Working");
    }
}

mod copy_on_an_owned_transcript {
    use codex_tui_core::app::facts::TuiSettings;

    use crate::kit::{Owner, Screen};

    const RESPONSE: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_1.jsonl");

    fn after_copy(owned: bool) -> Screen {
        let settings = TuiSettings {
            transcript_owned: owned,
            ..TuiSettings::default()
        };
        let mut s = Screen::build(Owner::ChatLive)
            .size(100, 24)
            .script(RESPONSE)
            .tui(settings)
            .open();
        s.type_text("/copy").keys("enter");
        s
    }

    /// `show_copy_picker`: where the chat owns its transcript `/copy` begins copy mode over it
    /// (the presenter asks the feature: `FeatureView::in_place`); the picker is the other mode's way.
    #[test]
    fn slash_copy_begins_copy_mode_only_where_the_transcript_is_owned() {
        let owned = after_copy(true);
        owned.assert_contains("enter copy · esc close");
        owned.assert_absent("Whole response");
        let terminal = after_copy(false);
        terminal.assert_contains("Whole response");
        terminal.assert_absent("enter copy · esc close");
    }

    /// The copy the page sends after Enter is the piece as the target copies it (`shape`): the
    /// whole response is its Markdown source, not the text the area selected.
    #[test]
    fn enter_on_the_whole_response_copies_its_markdown_source() {
        use codex_tui_core::app::Command;
        use codex_tui_core::app::effects::{Ask, CopyFormat};

        let settings = TuiSettings {
            transcript_owned: true,
            ..TuiSettings::default()
        };
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 40)
            .script(include_str!(
                "../oracle/fixtures/transcript_view_2/copy_nav.jsonl"
            ))
            .tui(settings)
            .open();
        s.type_text("/copy").keys("enter");
        s.assert_contains("Copy whole response");
        s.keys("enter");
        s.wait(50);
        let sent: Vec<(String, CopyFormat)> = s
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Effect {
                    ask: Ask::Copy { text, format, .. },
                    ..
                } => Some((text, format)),
                _ => None,
            })
            .collect();
        assert_eq!(sent.len(), 1, "{}", s.text());
        assert_eq!(sent[0].1, CopyFormat::Markdown);
        assert!(sent[0].0.contains("Latest **plan**"), "{:?}", sent[0].0);
    }
}

/// A side conversation asks for a follow-up: the empty composer of the thread says so
/// (`side_placeholder_text`), and the thread beside it keeps the usual words.
mod side_placeholder {
    use crate::kit::{Owner, Screen};

    const SIDE: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"screen","dir":"chat","ev":{"flags":{"collaboration_modes_enabled":true,"side_conversation_active":true}}}
"#;

    #[test]
    fn a_side_conversation_asks_for_a_follow_up() {
        let s = Screen::build(Owner::UiPane)
            .size(80, 24)
            .script(SIDE)
            .open();
        s.assert_contains("Ask a follow-up question");
        s.assert_absent("Ask Codex to do anything");
    }

    #[test]
    fn the_usual_thread_keeps_its_usual_placeholder() {
        let s = Screen::build(Owner::UiPane)
            .size(80, 24)
            .script(r#"{"step":"seed","seed":"session","thread":"thread-1"}"#)
            .open();
        s.assert_contains("Ask Codex to do anything");
        s.assert_absent("Ask a follow-up question");
    }
}

/// `Open::draft`: the chat's composer starts with a text a script cannot type, the caret at its
/// end (`set_text_content` of the oracle's tests).
mod kit_draft_seed {
    use crate::kit::{Owner, Screen};

    #[test]
    fn the_draft_is_in_the_composer_before_the_first_key() {
        let s = Screen::build(Owner::ChatLive)
            .size(80, 14)
            .live()
            .draft("first\nsecond")
            .open();
        s.assert_contains("› first");
        s.assert_contains("second");
        s.assert_absent("Ask Codex to do anything");
    }

    #[test]
    fn what_is_typed_after_it_goes_to_its_end() {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 14)
            .live()
            .draft("note")
            .open();
        s.type_text(" more");
        s.assert_contains("› note more");
    }
}

/// `Attachments.diagnostics`: the host's connectivity diagnostics, a script's `attachments`
/// seed, are listed by the consent step (`feedback_consent_params`): the file for any category,
/// the list for every one but a good result.
mod feedback_connectivity {
    use crate::kit::{Owner, Screen};

    const SCRIPT: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"screen","dir":"dialogs","ev":{"kind":"attachments","can_save":true,"diagnostics":[{"headline":"Proxy environment variables are set and may affect connectivity.","details":["HTTPS_PROXY = hello"]}]}}
"#;

    fn consent(digit: &str) -> Screen {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 34)
            .script(SCRIPT)
            .open();
        s.type_text("/feedback");
        s.keys("enter");
        s.keys(digit);
        s.assert_contains("Include logs and session history?");
        s
    }

    #[test]
    fn a_bug_report_lists_the_proxy_diagnostics_under_its_files() {
        let s = consent("1");
        s.assert_contains("• codex-connectivity-diagnostics.txt");
        s.assert_contains("Connectivity diagnostics");
        s.assert_contains("- Proxy environment variables are set and may affect connectivity.");
        s.assert_contains("- HTTPS_PROXY = hello");
    }

    #[test]
    fn a_good_result_names_the_file_and_leaves_the_list_out() {
        let s = consent("3");
        s.assert_contains("• codex-connectivity-diagnostics.txt");
        s.assert_absent("Connectivity diagnostics");
        s.assert_absent("HTTPS_PROXY");
    }

    #[test]
    fn without_diagnostics_neither_the_file_nor_the_list_shows() {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 34)
            .script(r#"{"step":"seed","seed":"session","thread":"thread-1"}"#)
            .open();
        s.type_text("/feedback");
        s.keys("enter");
        s.keys("1");
        s.assert_contains("Include logs and session history?");
        s.assert_absent("codex-connectivity-diagnostics.txt");
        s.assert_absent("Connectivity diagnostics");
    }
}

/// `Screen::burst` reaches any focused widget, the feedback note's text area too: the characters
/// of a paste arrive. A newline inside the burst is the text area's to keep (it has no paste burst
/// detector yet: a widget need of the dialogs package), so that test waits.
mod burst_into_the_feedback_note {
    use crate::kit::{Owner, Screen};

    fn note() -> Screen {
        Screen::build(Owner::DialogsPane)
            .size(60, 11)
            .entry("feedback", "feedback_note_bug_no_logs")
            .open()
    }

    #[test]
    fn the_characters_of_a_burst_reach_the_note() {
        let mut s = note();
        s.burst("pasted text");
        s.assert_contains("pasted text");
        s.assert_contains("Tell us more");
    }

    #[test]
    #[ignore = "NEED (widget): the note's TextArea has no paste-burst, a fast Enter submits (oracle feedback_paste_burst_preserves_multiline_note_until_submit)"]
    fn a_newline_inside_a_burst_is_a_line_of_the_note() {
        let mut s = note();
        s.burst("foo\nbar");
        s.assert_contains("foo");
        s.assert_contains("bar");
        s.assert_contains("Tell us more");
    }
}

/// The panel's status-line and title rows open the setups the way `/statusline` and `/title`
/// do, with the live values of the chat (`dialog_of`, `open`'s `live`), not the debug menu's.
mod config_panel_opens_the_live_setups {
    use crate::kit::{Owner, Screen};

    const SCRIPT: &str = concat!(
        include_str!("../oracle/fixtures/chat_status_3/preview_live.jsonl"),
        "\n",
        // the turn is over: `/config` is not a command of a working thread
        r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"summary","status":"completed","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#,
        "\n",
        r#"{"step":"fact","feature":"settings","fact":{"kind":"config_panel","rows":[{"id":"theme","section":"appearance","label":"Theme","description":"Choose a syntax color palette with a live preview","keywords":"colors dark light palette","value":{"choice":"Automatic"}},{"id":"status_line_display","section":"appearance","label":"Status line display","description":"Choose and reorder the information below the composer","keywords":"statusline footer model branch context","value":"configure"},{"id":"terminal_title_display","section":"appearance","label":"Terminal title display","description":"Choose and reorder the information in your terminal tab or window title","keywords":"title tab window name","value":"configure"}]}}"#,
        "\n"
    );

    fn panel() -> Screen {
        let mut s = Screen::build(Owner::UiPane)
            .size(100, 28)
            .script(SCRIPT)
            .open();
        s.type_text("/config");
        s.keys("enter");
        s.assert_contains("Configuration");
        s
    }

    #[test]
    fn the_status_line_row_previews_the_chats_live_values() {
        let mut s = panel();
        s.keys("down enter");
        s.assert_absent("Configuration");
        s.assert_contains("preview-live-root · feature/live-preview-branch · Live preview thread");
    }

    #[test]
    fn the_title_row_previews_the_chats_live_values() {
        let mut s = panel();
        s.keys("down down enter");
        s.assert_contains(
            "preview-live-root | Live preview thread | feature/live-preview-branch | Tasks 2/5",
        );
    }
}
