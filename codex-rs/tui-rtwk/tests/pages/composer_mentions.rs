// test root: pages
//! The mention popups (`$` skills, `@` files, plugins, skills and tasks): the keys that move
//! through them, pick, close and switch their tab, what a pick leaves in the draft and in
//! history, and the pointer on the popup widget (a tap on a row picks it, a tap on a tab
//! switches it). Oracle: `chat_composer.rs` `handle_key_event_with_skill_popup` and
//! `handle_key_event_with_mentions_v2_popup`; the mouse is ADR-011's.
//!
//! The page tests drive the whole composer through keys; the popup's rows are drawn above it
//! by `MentionPopup`, which the page places (a Need of `session_page.rs`), so the tests that
//! need the rows on the page are marked `pending(mount)`. The widget tests mount the popup
//! alone, on a draft whose session the test syncs, as the composer does.

use crate::kit::{Owner, Screen};

fn skills() -> Screen {
    Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "../oracle/fixtures/composer_popups/mentions_state.jsonl"
        ))
        .open()
}

fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›'))
        .unwrap_or(usize::MAX)
}

#[test]
fn tab_puts_the_highlighted_skill_in_as_one_element_and_a_space() {
    let mut s = skills();
    s.type_text("$ru");
    s.keys("tab");
    s.assert_row(prompt(&s), "› $rustdoc");
    // the space after it: the next word stands apart
    s.type_text("x");
    s.assert_row(prompt(&s), "› $rustdoc x");
}

#[test]
fn down_moves_the_highlight_and_wraps_round_the_ends() {
    let mut s = skills();
    s.type_text("$ru").keys("down enter");
    s.assert_row_starts_with(prompt(&s), "› $rusty");
    // Ctrl+N and Ctrl+P are Down and Up
    let mut s = skills();
    s.type_text("$ru").keys("ctrl+n ctrl+n enter");
    s.assert_row_starts_with(prompt(&s), "› $rustdoc");
    let mut s = skills();
    s.type_text("$ru").keys("up enter");
    s.assert_row_starts_with(prompt(&s), "› $rusty");
}

#[test]
fn escape_closes_the_popup_for_this_token_until_it_changes() {
    // closed, Enter is the draft's: it goes out as typed, nothing is put in
    let mut s = skills();
    s.type_text("$ru").keys("esc enter");
    s.wait_for("$ru");
    assert_eq!(s.turns().len(), 1);
    // a changed token opens it again, and Enter picks
    let mut s = skills();
    s.type_text("$ru").keys("esc");
    s.type_text("sty").keys("enter");
    s.assert_row(prompt(&s), "› $rusty");
    assert!(s.turns().is_empty());
}

#[test]
fn enter_with_no_row_in_the_skill_popup_is_the_popups_and_sends_nothing() {
    let mut s = skills();
    s.type_text("$zzz").keys("enter");
    s.assert_row(prompt(&s), "› $zzz");
    assert!(s.turns().is_empty(), "nothing was sent");
}

#[test]
fn enter_with_no_row_in_the_at_popup_is_a_submit() {
    let mut s = skills();
    s.type_text("@zzzz").keys("enter");
    s.wait_for("@zzzz");
    assert_eq!(s.turns().len(), 1, "the draft went out");
}

#[test]
fn left_and_right_move_the_at_popups_tab_and_leave_the_caret() {
    let mut s = skills();
    s.type_text("@ru").keys("left right left");
    s.type_text("X");
    // the caret stayed at the end of the token
    s.assert_row(prompt(&s), "› @ruX");
}

#[test]
fn left_and_right_are_the_carets_in_the_skill_popup() {
    let mut s = skills();
    s.type_text("$ru").keys("left");
    s.type_text("X");
    s.assert_row(prompt(&s), "› $rXu");
}

#[test]
fn a_picked_skill_is_an_element_again_from_history() {
    let mut s = skills();
    s.type_text("$ru").keys("tab");
    s.type_text("hi").keys("enter");
    s.wait_for("hi");
    assert_eq!(s.turns().len(), 1);
    s.keys("up");
    s.assert_row(prompt(&s), "› $rustdoc hi");
    // an element goes whole: the skill is deleted by one key, its space stays
    s.keys("home delete");
    s.assert_row(prompt(&s), "›  hi");
}

mod popup {
    //! The popup widget on its own: a draft whose session the test syncs, the popup mounted
    //! over it, and the pointer on its rows and its tabs.

    use std::cell::RefCell;
    use std::rc::Rc;

    use codex_tui_core::data::files::{FileMatch, FileMatches, MatchType};
    use codex_tui_core::data::mention::{Mention, MentionKind};
    use codex_tui_core::data::usage::MentionUsage;
    use codex_tui_core::kit::mentions::search_mode::SearchMode;
    use codex_tui_core::kit::mentions::session::{Active, Sources};
    use codex_tui_core::kit::mentions::target::Doc;
    use codex_tui_rtwk::pages::session::popovers::mention::MentionPopup;
    use codex_tui_rtwk::widgets::thread_ui::ThreadUi;

    fn plugin(n: usize) -> Mention {
        Mention::new(
            MentionKind::Plugin,
            &format!("Plugin {n:02}"),
            "Tools and workflows",
            &format!("plugin-{n}"),
        )
    }

    fn skill(name: &str) -> Mention {
        Mention::new(MentionKind::Skill, name, "A skill", name)
    }

    /// A draft that holds `text` with the caret at its end and the popup of it synced, over
    /// `mentions` and `files`.
    fn open(text: &str, mentions: &[Mention], files: &[&str]) -> Rc<ThreadUi> {
        let ui = Rc::new(ThreadUi::default());
        let usage = MentionUsage::default();
        let matches = FileMatches {
            query: text.trim_start_matches('@').to_string(),
            matches: files
                .iter()
                .map(|path| FileMatch {
                    path: (*path).into(),
                    match_type: MatchType::File,
                    score: 50,
                    indices: None,
                })
                .collect(),
        };
        let sources = Sources {
            mentions,
            tasks: &[],
            usage: &usage,
            file_matches: &matches,
        };
        let doc = Doc {
            text,
            cursor: text.len(),
            elements: &[],
        };
        ui.draft.mentions().sync(&doc, &sources);
        ui
    }

    fn mount(ui: &Rc<ThreadUi>, width: u16, height: u16) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, height));
        t.pump_widget(MentionPopup::new(ui.clone()));
        t
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    fn at(t: &rtwk_test::Tester, text: &str) -> (u16, u16) {
        let rows = rows(t);
        let y = rows
            .iter()
            .position(|row| row.contains(text))
            .unwrap_or_else(|| panic!("`{text}` is not on the screen:\n{}", rows.join("\n")));
        let x = rows[y]
            .find(text)
            .map_or(0, |at| rows[y][..at].chars().count());
        (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0))
    }

    /// The row numbers the popup's pick ran with: what a click asks the composer for.
    fn chosen(ui: &Rc<ThreadUi>) -> Rc<RefCell<Vec<usize>>> {
        let picked = Rc::new(RefCell::new(Vec::new()));
        let sink = picked.clone();
        ui.draft
            .mentions()
            .on_choose(Rc::new(move |at| sink.borrow_mut().push(at)));
        picked
    }

    #[test]
    fn the_unified_popup_has_its_title_tabs_query_rows_and_hint() {
        let ui = open("@pl", &[plugin(0), plugin(1)], &[]);
        let t = mount(&ui, 80, 24);
        let shown = rows(&t);
        let text = shown.join("\n");
        for part in [
            "Mentions",
            "Files, directories, plugins, skills and tasks",
            "All Results",
            "Filesystem Only",
            "Plugins",
            "Plugin 00",
            "Plugin 01",
            "enter/tab",
            "esc",
        ] {
            assert!(text.contains(part), "`{part}` is missing:\n{text}");
        }
    }

    #[test]
    fn a_click_on_a_row_asks_the_composer_to_pick_it() {
        let ui = open("@pl", &[plugin(0), plugin(1), plugin(2)], &[]);
        let picked = chosen(&ui);
        let mut t = mount(&ui, 80, 24);
        let (x, y) = at(&t, "Plugin 01");
        t.click(x, y);
        t.pump();
        assert_eq!(*picked.borrow(), vec![1]);
    }

    #[test]
    fn a_click_on_a_dollar_row_asks_the_composer_to_pick_it() {
        let ui = open("$ru", &[skill("rustdoc"), skill("rusty")], &[]);
        let picked = chosen(&ui);
        let mut t = mount(&ui, 80, 24);
        let (x, y) = at(&t, "rusty");
        t.click(x, y);
        t.pump();
        assert_eq!(*picked.borrow(), vec![1]);
        let text = rows(&t).join("\n");
        assert!(text.contains("enter") && text.contains("close"), "{text}");
    }

    #[test]
    fn a_tap_on_a_tab_switches_the_search_mode() {
        let ui = open("@pl", &[plugin(0)], &["src/plan.rs"]);
        let mode = |ui: &Rc<ThreadUi>| {
            ui.draft.mentions().read(|session| match session.active() {
                Active::At(popup) => popup.search_mode(),
                _ => panic!("the `@` popup is up"),
            })
        };
        assert_eq!(mode(&ui), SearchMode::ALL[0]);
        let mut t = mount(&ui, 80, 24);
        let (x, y) = at(&t, "Plugins");
        t.click(x, y);
        t.pump();
        assert_eq!(mode(&ui), SearchMode::ALL[2]);
        let (x, y) = at(&t, "Filesystem Only");
        t.click(x, y);
        t.pump();
        assert_eq!(mode(&ui), SearchMode::ALL[1]);
    }

    #[test]
    fn a_popup_that_is_not_up_draws_nothing() {
        let ui = Rc::new(ThreadUi::default());
        let t = mount(&ui, 80, 24);
        assert!(rows(&t).iter().all(String::is_empty));
    }
}

fn side() -> Screen {
    Screen::build(Owner::ChatNavPane)
        .size(120, 24)
        .script(include_str!(
            "../oracle/fixtures/chat_commands_3/side.jsonl"
        ))
        .open()
}

/// The rows from the composer's prompt to the screen's end.
fn composer_rows(s: &Screen) -> Vec<String> {
    s.rows()[prompt(s)..].to_vec()
}

/// The oracle's `mention_menu_reserves_height_above_composer_and_owns_cursor_over_interactive_footer`
/// and `mention_scopes_overlay_history_without_moving_composer_or_footer`: the popup of any tab
/// sits above the composer, which keeps its rows, its caret and its footer, at every size.
#[test]
fn mention_menu_keeps_composer_rows_and_cursor_across_tabs() {
    for (width, height) in [
        (80, 32),
        (80, 12),
        (80, 7),
        (80, 6),
        (80, 5),
        (40, 32),
        (40, 12),
        (40, 7),
        (40, 6),
        (40, 5),
    ] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(width, height)
            .script(include_str!(
                "../oracle/fixtures/composer_popups/mentions_state.jsonl"
            ))
            .open();
        s.type_text("@");
        let at = prompt(&s);
        let rows = composer_rows(&s);
        s.assert_cursor(3, at);
        for key in ["right", "right", "left", "left", "left"] {
            s.keys(key);
            assert_eq!(prompt(&s), at, "{width}x{height} after {key}");
            assert_eq!(composer_rows(&s), rows, "{width}x{height} after {key}");
            s.assert_cursor(3, at);
        }
        // Esc closes the popup: the composer stays where it was
        s.keys("esc");
        assert_eq!(prompt(&s), at, "{width}x{height} after esc");
        assert_eq!(composer_rows(&s), rows, "{width}x{height} after esc");
        s.assert_cursor(3, at);
    }
}

/// `/skills` `List skills` puts `$` in the composer (`insert_str("$")`) and its popup opens; Esc
/// closes the popup and leaves the `$`; leaving the menu with Esc inserts nothing.
#[test]
fn list_skills_puts_a_dollar_in_the_draft_and_opens_the_skill_popup() {
    let mut s = skills();
    s.type_text("/skills").keys("enter");
    s.assert_contains("List skills");
    s.keys("enter");
    s.assert_row(prompt(&s), "› $");
    s.assert_contains("rustdoc");
    s.keys("esc");
    s.assert_row(prompt(&s), "› $");
    s.assert_absent("rustdoc");

    let mut s = skills();
    s.type_text("/skills").keys("enter");
    s.keys("esc");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_absent("rustdoc");
}

/// A command a side conversation cannot run is listed after a search with its reason, and is no
/// pick: Down stays off it, Enter does not complete it.
#[test]
fn a_disabled_slash_command_is_listed_and_picks_nothing() {
    let mut s = side();
    s.type_text("/");
    s.assert_absent("/archive");
    s.type_text("arch");
    s.assert_contains("not available in a side conversation");
    s.keys("down");
    s.keys("enter");
    s.assert_absent("› /archive ");
}
