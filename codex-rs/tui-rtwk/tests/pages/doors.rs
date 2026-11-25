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
    fn a_newline_inside_a_burst_is_a_line_of_the_note() {
        let mut s = note();
        s.burst("foo\nbar");
        s.assert_contains("foo");
        s.assert_contains("bar");
        s.assert_contains("Tell us more");
        // the burst is over: an Enter sends
        s.wait(200);
        s.keys("enter");
        s.assert_absent("Tell us more");
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

/// F4 reaches the cells that disclose details (doors-r3, item 7): their open state is the
/// thread's controller (`ThreadUi::disclosed`), which a row says (`cells::disclosure::tile`).
mod disclosed_cells {
    use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
    use rtwk::style::Style;
    use rtwk::widgets::Text;
    use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
    use rtwk_test::{Options, Tester};

    use codex_tui_core::ids::{ItemId, TurnId};
    use codex_tui_core::threads::ItemKey;
    use codex_tui_rtwk::pages::session::cells::disclosure::{disclosing, expanded, tile};
    use codex_tui_rtwk::widgets::thread_ui::ThreadUi;

    /// A cell of the kind the transcript has (`ToolRowState`): its flag, its summary, its body
    /// when open, and the control.
    #[derive(Clone, PartialEq)]
    struct Cell;

    #[derive(Default)]
    struct CellState {
        open: bool,
    }

    impl StatefulWidget for Cell {
        type State = CellState;
        fn create_state(&self) -> CellState {
            CellState::default()
        }
    }

    impl_widget!(stateful eq Cell);

    impl State for CellState {
        type Widget = Cell;
        type Msg = bool;

        fn update(&mut self, open: bool, _: &Cell, _: &mut Cx<'_, Self>) -> Rebuild {
            self.open = open;
            Rebuild::Yes
        }

        fn build(&self, _: &Cell, cx: &mut Cx<'_, Self>) -> AnyWidget {
            let open = expanded(cx, self.open);
            let mut content = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::new("summary"));
            if open {
                content = content.child(Text::new("the body"));
            }
            let label = if open { "Hide details" } else { "Show details" };
            disclosing(cx, content.into(), label, Style::default(), open)
        }
    }

    fn key() -> ItemKey {
        ItemKey {
            turn: TurnId::new("turn-1"),
            item: ItemId::new("call-1"),
        }
    }

    fn mount(cell: impl Into<AnyWidget>) -> Tester {
        let mut t = Tester::with(Options::new().size(24, 6));
        t.pump_widget(cell);
        t
    }

    /// The controller opens the cell the way F4's `Enter` will, and the cell closes again when it
    /// is told so; the row's control is on the row below the cell's rows.
    #[test]
    fn the_controller_opens_and_closes_a_cell_under_a_tile() {
        let ui = ThreadUi::default();
        let mut t = mount(tile(&ui.disclosed(), key(), Cell.into()));
        assert!(t.screen().contains("Show details"), "{}", t.screen());
        assert!(!t.screen().contains("the body"));
        ui.disclosed().set(&key(), true);
        t.pump();
        assert!(t.screen().contains("the body"), "{}", t.screen());
        assert!(t.screen().contains("Hide details"));
        ui.disclosed().set(&key(), false);
        t.pump();
        assert!(!t.screen().contains("the body"), "{}", t.screen());
    }

    /// A tap on the control goes through the controller: it says the cell is open, and names the
    /// cell among those F4 may walk.
    #[test]
    fn a_tap_on_the_control_is_the_controllers() {
        let ui = ThreadUi::default();
        let mut t = mount(tile(&ui.disclosed(), key(), Cell.into()));
        assert!(ui.disclosed().known().contains(&key()));
        assert!(!ui.disclosed().is_open(&key()));
        // the control stands a quarter of the width in, on the row below the summary
        t.click(6, 1);
        t.pump();
        assert!(ui.disclosed().is_open(&key()), "{}", t.screen());
        assert!(t.screen().contains("the body"), "{}", t.screen());
        // the row of the control moved down with the body
        t.click(6, 2);
        t.pump();
        assert!(!ui.disclosed().is_open(&key()), "{}", t.screen());
    }

    /// The controller is the thread's: one per `ThreadUi`, shared by its clones, and a key it was
    /// not told of is closed.
    #[test]
    fn each_thread_has_its_own_controller() {
        let (ui, other) = (ThreadUi::default(), ThreadUi::default());
        let clone = ui.clone();
        assert!(ui.disclosed() == clone.disclosed());
        assert!(ui.disclosed() != other.disclosed());
        ui.disclosed().set(&key(), true);
        assert!(!other.disclosed().is_open(&key()));
        assert!(!ui.disclosed().toggle(&key()));
        assert!(ui.disclosed().toggle(&key()));
    }

    /// A cell outside a tile keeps its own flag, as before.
    #[test]
    fn a_cell_outside_a_tile_keeps_its_own_flag() {
        let mut t = mount(Cell);
        t.click(6, 1);
        t.pump();
        assert!(t.screen().contains("the body"), "{}", t.screen());
    }
}

/// The chat in a window of 4 to 6 rows (doors-r3, item 8). The bottom pane is the oracle's
/// (`bottom_pane_renderable`: the reader's gap, the composer's blank row over and under its line,
/// the footer) and the list takes what it leaves; where the window is shorter than the pane, the
/// gap is cut first and the footer stays.
mod small_windows {
    use crate::kit::{Owner, Screen};

    const TWO: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_1.jsonl");
    const PROMPT: &str = include_str!("../oracle/fixtures/transcript_view_5/one_prompt.jsonl");
    const FOOTER: &str = "for shortcuts";

    fn open(width: u16, height: u16, script: &'static str) -> Screen {
        Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(script)
            .open()
    }

    /// Six rows: the pane's five (gap, blank, composer, blank, footer) and one for the list.
    #[test]
    fn six_rows_leave_the_list_one_row_over_the_pane() {
        let s = open(32, 6, TWO);
        let rows = s.rows();
        assert_eq!(rows.len(), 6, "{}", s.text());
        assert!(!rows[0].is_empty(), "the list's row\n{}", s.text());
        assert!(rows[1].is_empty(), "the gap\n{}", s.text());
        assert!(rows[5].contains(FOOTER), "the footer\n{}", s.text());
    }

    /// Five rows are the pane's: the list has none, and nothing of it shows over the gap.
    #[test]
    fn five_rows_are_the_pane_and_no_list() {
        let s = open(32, 5, TWO);
        let rows = s.rows();
        assert_eq!(rows.len(), 5, "{}", s.text());
        assert!(rows[0].is_empty(), "the gap\n{}", s.text());
        assert!(rows[4].contains(FOOTER), "the footer\n{}", s.text());
        s.assert_absent("Worked for");
        s.assert_absent("bottom");
    }

    /// Four rows cut the gap, not the footer: the composer's blank row, its line, its blank row
    /// and the footer.
    #[test]
    fn four_rows_cut_the_gap_and_keep_the_footer() {
        let s = open(32, 4, TWO);
        let rows = s.rows();
        assert_eq!(rows.len(), 4, "{}", s.text());
        assert!(rows[3].contains(FOOTER), "the footer\n{}", s.text());
        assert!(rows[0].is_empty(), "the composer's blank row\n{}", s.text());
        s.assert_absent("Worked for");
    }

    /// The hint of the first Esc is a line of 34 cells: in a window of 32 columns it is cut at the
    /// right edge, after the indent, and the row does not overflow (the footer is `Paragraph`'s
    /// clipped line in the oracle).
    #[test]
    fn the_esc_hint_is_cut_at_the_edge_of_a_narrow_window() {
        let mut s = open(32, 8, PROMPT);
        s.keys("esc");
        let rows = s.rows();
        let last = rows.last().expect("rows");
        assert!(
            last.starts_with("  esc again to edit previous"),
            "{}",
            s.text()
        );
        assert!(last.chars().count() <= 32, "{last:?}");
    }
}

/// doors-r3 item 4: the usage page's host. `Open::dir("usage")` opens the page live (it asks the
/// fake backend, not a fixture), `usage_accounts` says what the lookups are answered with.
mod usage_host {
    use crate::kit::{Owner, Screen};

    const PLUS: &str = r#"{"plan":"plus","today":"2026-09-09"}"#;
    const ENTERPRISE: &str = r#"{"plan":"enterprise","today":"2026-09-09"}"#;

    /// A failed lookup is the host's message, whole, in the page's room; Shift+R asks again and
    /// the server's plan (the second answer) gives the page its heading and tabs
    /// (oracle `account_lookup_failure_recovers_on_refresh_with_the_server_plan`).
    #[test]
    fn a_failed_account_lookup_recovers_on_refresh_with_the_server_plan() {
        let mut s = Screen::build(Owner::RoutesLists)
            .size(100, 24)
            .dir("usage")
            .usage_accounts(&[Err("503 Service Unavailable"), Ok(ENTERPRISE)])
            .open();
        s.assert_contains("503 Service Unavailable");
        s.assert_absent("Credits");
        s.keys("R");
        s.wait_for("Credits");
        s.assert_absent("503 Service Unavailable");
        s.assert_contains("Usage · Enterprise");
    }

    /// An account that changes plan while the page is open: the refresh's answer is the new
    /// plan's tabs (a Plus account has Activity, no Credits; an Enterprise one the reverse).
    #[test]
    fn the_plan_changes_under_an_open_page() {
        let mut s = Screen::build(Owner::RoutesLists)
            .size(100, 24)
            .dir("usage")
            .usage_accounts(&[Ok(PLUS), Ok(ENTERPRISE)])
            .open();
        s.assert_contains("Activity");
        s.assert_absent("Credits");
        s.keys("R");
        s.wait_for("Credits");
        s.assert_absent("Activity");
    }
}

/// doors-r3 item 5: the kit's keymap, palette and joined-body doors, on the usage page.
mod kit_usage {
    use crate::kit::{self, Owner, Screen};

    const CANCEL_ON_QUESTION: &[(&str, &str)] = &[("list.cancel", "?")];
    const CANCEL_ON_SHIFT_QUESTION: &[(&str, &str)] = &[("list.cancel", "shift+?")];

    fn credits(keymap: &'static [(&'static str, &'static str)]) -> Screen {
        Screen::build(Owner::RoutesLists)
            .size(100, 32)
            .entry("usage", "credits_refund")
            .keymap(keymap)
            .open()
    }

    /// `Open::keymap(("list.cancel", "?"))`: the page's `?` is then the cancel, it hints no help,
    /// and the key closes the page, also the Shift spelling of it (oracle
    /// `configured_question_mark_keeps_its_action_and_hides_help_hint`).
    #[test]
    fn a_configured_question_mark_cancels_and_hides_its_help_hint() {
        for keymap in [CANCEL_ON_QUESTION, CANCEL_ON_SHIFT_QUESTION] {
            let mut s = credits(keymap);
            s.assert_contains("Product");
            s.assert_absent("? help");
            s.keys(keymap[0].1);
            s.assert_absent("Product");
        }
    }

    /// `key_release` / `key_repeat` reach a page's key bindings as the terminal's kitty events:
    /// the release of `q` is no press (the page stays), its repeat is one (the page closes)
    /// (oracle `horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat`).
    #[test]
    fn a_release_is_no_press_and_a_repeat_is_one() {
        let mut s = credits(&[]);
        s.key_release("q");
        s.assert_contains("Product");
        s.key_release("alt+q");
        s.assert_contains("Product");
        s.key_repeat("q");
        s.assert_absent("Product");
    }

    /// `kit::contrast` is the WCAG ratio, and `Grid::contrast_at` reads a plain cell (`Reset`)
    /// against the terminal colours the test gave: black text on a white terminal is 21.
    #[test]
    fn a_plain_cell_is_measured_against_the_terminal_the_test_gave() {
        assert!((kit::contrast((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 1e-9);
        assert!((kit::contrast((255, 255, 255), (255, 255, 255)) - 1.0).abs() < 1e-9);
        let s = Screen::build(Owner::RoutesLists)
            .size(100, 32)
            .entry("usage", "credits_refund")
            .terminal_colors(0x00_00_00, 0xff_ff_ff)
            .open();
        let grid = s.grid();
        let plain = (0..grid.height)
            .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let style = grid.style(x, y);
                !grid.cell(x, y).text.trim().is_empty()
                    && style.fg == "Reset"
                    && style.bg == "Reset"
            })
            .expect("a cell of plain text");
        let ratio = grid
            .contrast_at(plain.0, plain.1, kit::LIGHT)
            .expect("a measured cell");
        assert!(
            (ratio - 21.0).abs() < 1e-9,
            "the cell's contrast is {ratio}"
        );
        // every measured cell is listed under a ratio no cell can reach
        assert!(
            grid.low_contrast(kit::LIGHT, 21.5)
                .iter()
                .any(|(x, y, ..)| (*x, *y) == plain)
        );
    }

    /// The bodies a test joined with a blank row, of different heights (the oracle's `panel(..)`
    /// bodies of `analytics_dates_distinguish_zero_missing_and_exact_details`), come apart by
    /// their heights: the first three are 16 rows each.
    #[test]
    fn joined_bodies_of_different_heights_come_apart() {
        let bodies = kit::snap_states(
            "analytics_dates_distinguish_zero_missing_and_exact_details",
            &[16, 16, 16],
        );
        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[0].first().map(String::as_str), Some("13.1K credits"));
        assert!(
            bodies[0]
                .iter()
                .any(|row| row.contains("Aug 27 · No activity (0)"))
        );
        assert!(
            bodies[1]
                .iter()
                .any(|row| row.contains("12746.441206 credits"))
        );
        assert!(
            bodies[2]
                .iter()
                .any(|row| row.contains("Aug 29 · Not reported"))
        );
        assert!(bodies.iter().all(|body| body.len() == 16));
    }
}

/// doors-r3 item 1 and 2: the footer's `IDE context` indicator and its flash line, both raised by
/// facts a script (or the app) tells.
mod footer_facts {
    use crate::kit::{Owner, Screen};

    const IDE_ON: &str =
        r#"{"step":"fact","feature":"small_cmds","fact":{"kind":"ide_on","has_context":false}}"#;
    const FLASH: &str = r#"{"step":"screen","dir":"chat","ev":{"facts":{"flash":{"text":"Copied to clipboard","millis":3000}}}}"#;

    fn chat() -> Screen {
        Screen::build(Owner::ChatLive).size(80, 14).live().open()
    }

    /// `/ide` on (the `ide_on` seed of `small_cmds`) is `IDE context` in the footer, no note in
    /// the transcript; the state is the feature's (a config reload does not wipe it).
    #[test]
    fn the_ide_context_indicator_follows_the_small_commands_state() {
        let mut s = chat();
        s.assert_absent("IDE context");
        s.feed(IDE_ON);
        s.assert_contains("IDE context");
        s.assert_absent("IDE context on");
    }

    /// `facts::Ev::Flash`: the line takes the footer's left side for its `millis` and then goes;
    /// the same line told again while it is on is another showing, its time starts anew.
    #[test]
    fn a_flash_stays_for_its_time_and_a_new_showing_restarts_it() {
        let mut s = chat();
        s.assert_absent("Copied to clipboard");
        s.feed(FLASH);
        s.assert_contains("Copied to clipboard");
        s.wait(2_000);
        s.assert_contains("Copied to clipboard");
        s.feed(FLASH);
        s.wait(2_000);
        s.assert_contains("Copied to clipboard");
        s.wait(1_500);
        s.assert_absent("Copied to clipboard");
        s.assert_contains("for shortcuts");
    }
}

/// doors-r3 item 3: a snapshot the test joined with `join("\n---\n")` comes apart at its bare
/// rules, not at the front matter's.
mod snapshot_cuts {
    use crate::kit;

    #[test]
    fn a_joined_snapshot_is_cut_at_its_bare_rules() {
        let states = kit::snap_cuts("rate_limit_reset_popup_states");
        assert_eq!(states.len(), 10);
        assert_eq!(
            states[0].first().map(String::as_str),
            Some("  Usage limit resets")
        );
        assert!(
            states[0]
                .iter()
                .any(|row| row.contains("Checking your available resets"))
        );
        assert!(
            states[1]
                .iter()
                .any(|row| row.contains("2 usage limit resets available."))
        );
        assert!(
            states
                .iter()
                .all(|state| !state.iter().any(|row| row == "---"))
        );
    }
}

/// doors-r3 item 6: Find's scan is the widget's, reachable for the select mode's `n` / `N`.
mod finder_search {
    use std::borrow::Cow;

    use codex_tui_rtwk::widgets::finder::{self, Direction, Rows};

    struct Lines(&'static [&'static str]);

    impl Rows for Lines {
        fn count(&self) -> usize {
            self.0.len()
        }

        fn text(&self, row: usize) -> Cow<'_, str> {
            Cow::Borrowed(self.0[row])
        }
    }

    const ROWS: Lines = Lines(&["one Needle", "two", "NEEDLE needle", "four"]);

    #[test]
    fn the_query_is_folded_and_the_scan_goes_on_from_a_place_without_wrapping() {
        assert_eq!(finder::fold("NeedLE İ"), "needle i\u{307}");
        let query = finder::fold("Needle");
        let hit = |from, direction| finder::search(&ROWS, &query, from, direction);
        let newer = hit((0, 0), Direction::Newer).expect("in the first row");
        assert_eq!((newer.row, newer.range), (0, 4..10));
        // on from that match: the next row that has one, then its second match
        let next = hit((0, 10), Direction::Newer).expect("in the third row");
        assert_eq!((next.row, next.range), (2, 0..6));
        let next = hit((2, 6), Direction::Newer).expect("its second match");
        assert_eq!((next.row, next.range), (2, 7..13));
        assert!(hit((2, 13), Direction::Newer).is_none(), "never wraps");
        let older = hit((2, 7), Direction::Older).expect("before the byte");
        assert_eq!((older.row, older.range), (2, 0..6));
        let older = hit((2, 0), Direction::Older).expect("in an older row");
        assert_eq!(older.row, 0);
        assert!(hit((0, 4), Direction::Older).is_none(), "never wraps");
        assert!(finder::search(&ROWS, "", (0, 0), Direction::Newer).is_none());
    }
}

/// doors-r3 item 9: the command popup stays open (its rows kept) across Find.
mod command_popup_across_find {
    use crate::kit::{Owner, Screen};

    /// The popup of `/mod` is not painted while Find's bar has the keys, and is back when Find
    /// closes (the oracle's composer keeps `ActivePopup::Command` under the footer).
    #[test]
    fn the_popup_is_kept_while_find_is_open() {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.type_text("/mod");
        s.assert_contains("/model");
        s.keys("f3");
        s.assert_absent("/model");
        s.keys("esc");
        s.assert_contains("/model");
    }
}
