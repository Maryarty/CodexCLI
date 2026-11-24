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

    use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
    use rtwk_test::{Options, Tester};

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
            Picker::new(w.list.clone(), cx.map(|event: Event| event)).into()
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
        let mut s = Screen::build(Owner::ChatLive).size(100, 12).live().side().open();
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

    /// `blocks_of` is what the cell draws, and the text model reads it: a source with two
    /// paragraphs and a heading is three blocks, whose text is the source's.
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
        assert_eq!(
            markdown_text::text(&blocks).as_deref(),
            Some("# Title\n\npara one\n\npara two")
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
        let cells = s.grid().locate_cells(url).expect("the url is on the screen");
        let rows: BTreeSet<usize> = cells.iter().map(|&(_, y)| y).collect();
        assert_eq!(rows.len(), 3, "{}", s.text());
        assert_eq!(cells.len(), url.len());
        // a text of one row is still found as it was
        let one = s.grid().locate_cells("I just reprocessed").expect("one row");
        assert_eq!(one.iter().map(|&(_, y)| y).collect::<BTreeSet<_>>().len(), 1);
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
        let mut s = Screen::build(Owner::DialogsPane).size(100, 20).live().open();
        s.feed(
            r#"{"step":"presets","slots":[{"slot":1,"input":"/status","label":"Status"}]}"#,
        );
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

    /// `Open::unavailable_model()`: the thread's model is empty, the footer has none to name.
    #[test]
    fn the_unavailable_model_seed_leaves_the_thread_without_a_model() {
        let s = Screen::build(Owner::ChatLive)
            .size(80, 12)
            .unavailable_model()
            .live()
            .open();
        s.assert_contains("loading");
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
