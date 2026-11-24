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
