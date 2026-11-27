// test root: pages
//! Images in the composer: a dragged image file arrives as a pasted path and
//! is attached as `[Image #N]`; the placeholders are numbered in the order
//! the images were attached, with no gap; a submit carries the files with the
//! turn (`LocalImage` items first, then the text with its `text_elements`).
//! Oracle: `chat_composer.rs` (`attach_image`, `handle_paste_image_path`),
//! `chat_composer/attachment_state.rs`, `chatwidget/input_submission.rs`.

// the test files of an image are written to a directory of this process
#![allow(clippy::disallowed_methods, clippy::expect_used)]

use std::path::PathBuf;

use crate::common;
use crate::kit::{self, Owner, Screen};

use codex_tui_core::app::Command;
use codex_tui_core::threads::LocalImage;
use codex_tui_rtwk::widgets::composer_draft::ComposerDraft;
use common::{Harness, start};
use rtwk::editing::{TextEditingController, TextSelection};

fn image(n: usize) -> String {
    format!("[Image #{n}]")
}

fn png(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codex-composer-images-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(name);
    std::fs::write(&path, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").expect("png");
    path
}

mod draft {
    use super::*;

    fn attached(names: &[&str]) -> (ComposerDraft, TextEditingController) {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        for name in names {
            d.attach_image(&c, png(name), image);
        }
        (d, c)
    }

    #[test]
    fn images_are_numbered_as_attached() {
        let (_, c) = attached(&["a.png", "b.png"]);
        assert_eq!(&*c.text(), "[Image #1][Image #2]");
    }

    #[test]
    fn a_deleted_image_leaves_no_gap_and_keeps_its_file() {
        let (d, c) = attached(&["a.png", "b.png", "c.png"]);
        c.replace(0, "[Image #1]".len(), "");
        d.relabel_images(&c, image);
        assert_eq!(&*c.text(), "[Image #1][Image #2]");
        let message = d.submission(&c, image);
        let files: Vec<_> = message.images.iter().map(|i| i.path.clone()).collect();
        assert_eq!(files, vec![png("b.png"), png("c.png")]);
        assert_eq!(message.images[0].range, 0..10);
        assert_eq!(message.images[1].range, 10..20);
    }

    #[test]
    fn numbers_follow_the_attach_order_not_the_place_in_the_text() {
        let (d, c) = attached(&["a.png", "b.png"]);
        c.set_selection(TextSelection::collapsed(0));
        d.attach_image(&c, png("c.png"), image);
        assert_eq!(&*c.text(), "[Image #3][Image #1][Image #2]");
        // the images go in the order attached; each has its own range
        let message = d.submission(&c, image);
        let sent: Vec<(PathBuf, std::ops::Range<usize>)> = message
            .images
            .into_iter()
            .map(|LocalImage { path, range }| (path, range))
            .collect();
        assert_eq!(
            sent,
            vec![
                (png("a.png"), 10..20),
                (png("b.png"), 20..30),
                (png("c.png"), 0..10)
            ]
        );
    }

    #[test]
    fn a_killed_image_comes_back_as_the_newest() {
        let (d, c) = attached(&["a.png", "b.png"]);
        d.kill(&c, 0, "[Image #1]".len());
        c.replace(0, "[Image #1]".len(), "");
        d.relabel_images(&c, image);
        assert_eq!(&*c.text(), "[Image #1]");
        c.set_selection(TextSelection::collapsed(c.text().len()));
        assert!(d.yank(&c));
        d.relabel_images(&c, image);
        assert_eq!(&*c.text(), "[Image #1][Image #2]");
        let message = d.submission(&c, image);
        let files: Vec<_> = message.images.iter().map(|i| i.path.clone()).collect();
        assert_eq!(files, vec![png("b.png"), png("a.png")]);
    }

    #[test]
    fn typing_inside_a_placeholder_breaks_it_and_drops_the_image() {
        let (d, c) = attached(&["a.png"]);
        c.replace(3, 3, "x");
        let message = d.submission(&c, image);
        assert!(message.images.is_empty(), "{message:?}");
    }

    fn urls(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| format!("https://x/{n}.png")).collect()
    }

    #[test]
    fn local_images_are_numbered_after_the_remote_ones() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        d.set_remote_images(&c, urls(&["a", "b"]), image);
        d.attach_image(&c, png("a.png"), image);
        assert_eq!(&*c.text(), "[Image #3]");
        d.attach_image(&c, png("b.png"), image);
        assert_eq!(&*c.text(), "[Image #3][Image #4]");
        // a remote row less, and the local ones close the gap
        d.take_remote_images(&c, image);
        assert_eq!(&*c.text(), "[Image #1][Image #2]");
    }

    #[test]
    fn setting_the_remote_images_renumbers_the_local_ones() {
        let (d, c) = attached(&["a.png"]);
        assert_eq!(&*c.text(), "[Image #1]");
        d.set_remote_images(&c, urls(&["a"]), image);
        assert_eq!(&*c.text(), "[Image #2]");
        let message = d.submission(&c, image);
        assert_eq!(message.images[0].range, 0..10);
    }

    #[test]
    fn up_at_the_start_selects_the_last_row_and_delete_removes_it() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        d.set_remote_images(&c, urls(&["a", "b"]), image);
        d.attach_image(&c, png("a.png"), image);
        c.set_selection(TextSelection::collapsed(c.text().len()));
        assert!(!d.remote_up(&c), "the caret is not at the start");
        c.set_selection(TextSelection::collapsed(0));
        assert!(d.remote_up(&c));
        assert_eq!(d.remote_selected(), Some(1));
        assert!(d.remote_up(&c));
        assert_eq!(d.remote_selected(), Some(0));
        assert!(d.remote_down(&c));
        assert_eq!(d.remote_selected(), Some(1));
        assert!(d.remote_remove(&c, image));
        assert_eq!(d.remote_images(), urls(&["a"]));
        assert_eq!(d.remote_selected(), Some(0), "the row that took its place");
        assert_eq!(&*c.text(), "[Image #2]");
        assert!(d.remote_remove(&c, image));
        assert_eq!(d.remote_selected(), None);
        assert_eq!(&*c.text(), "[Image #1]");
        assert!(!d.remote_remove(&c, image), "no row is left");
    }

    #[test]
    fn down_past_the_last_row_selects_none() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        d.set_remote_images(&c, urls(&["a", "b"]), image);
        assert!(
            !d.remote_down(&c),
            "no row is selected: the key is the field's"
        );
        assert!(d.remote_up(&c));
        assert!(d.remote_down(&c));
        assert_eq!(d.remote_selected(), None);
    }

    #[test]
    fn a_moved_caret_ends_the_selection() {
        let (d, c) = attached(&["a.png"]);
        d.set_remote_images(&c, urls(&["a"]), image);
        c.set_selection(TextSelection::collapsed(0));
        assert!(d.remote_up(&c));
        assert!(!d.remote_deselect_on_move(&c));
        c.set_selection(TextSelection::collapsed(3));
        assert!(d.remote_deselect_on_move(&c));
        assert_eq!(d.remote_selected(), None);
    }

    #[test]
    fn a_kept_draft_comes_back_with_its_remote_images() {
        use codex_tui_core::data::history::Store;
        let c = TextEditingController::new("hello");
        let d = ComposerDraft::default();
        d.set_remote_images(&c, urls(&["a", "b"]), image);
        c.set_selection(TextSelection::collapsed(c.text().len()));
        d.cancelled(&c);
        c.set_text("");
        assert!(d.remote_images().is_empty(), "the draft is clear");
        let store = Store::new();
        let mut fx = Vec::new();
        assert!(d.history_step(&c, &store, true, &mut fx));
        assert_eq!(&*c.text(), "hello");
        assert_eq!(d.remote_images(), urls(&["a", "b"]));
        assert_eq!(d.remote_selected(), None);
    }
}

fn chat() -> Harness {
    let mut h = start();
    h.wait_idle();
    h
}

fn turns(h: &Harness) -> Vec<codex_tui_core::threads::UserMessage> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message),
            _ => None,
        })
        .collect()
}

#[test]
fn a_pasted_image_path_is_attached_and_goes_with_the_turn() {
    let mut h = chat();
    let path = png("pasted.png");
    h.t.type_text("look ");
    h.t.paste(&path.to_string_lossy());
    h.advance(100);
    assert!(
        h.t.screen().contains("› look [Image #1]"),
        "{}",
        h.t.screen()
    );
    h.press("enter");
    common::wait::wait_until(&mut h, "the turn", |h| !turns(h).is_empty());
    let sent = turns(&h);
    // the text keeps its placeholder; the file goes as an image of the turn
    assert_eq!(sent[0].text, "look [Image #1]");
    assert_eq!(sent[0].images, vec![LocalImage { path, range: 5..15 }]);
}

#[test]
fn a_pasted_path_that_is_no_image_stays_text() {
    let mut h = chat();
    let dir = png("x.png");
    let text = dir.with_file_name("notes.txt");
    std::fs::write(&text, "plain").expect("txt");
    h.t.paste(&text.to_string_lossy());
    h.advance(100);
    assert!(!h.t.screen().contains("[Image #1]"), "{}", h.t.screen());
}

mod page {
    //! The composer on the chat page: the remote images' rows over the draft
    //! and the keys that select and remove them, and Ctrl+V with the host's
    //! answer. The remote images are set on the first thread's controllers
    //! before it exists (the blank ones move to it), the way a restored
    //! message sets them.

    use std::cell::RefCell;
    use std::rc::Rc;

    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::popovers::{Ask, ClipboardRead, Ev};
    use codex_tui_rtwk::widgets::thread_ui::ThreadUi;

    use super::*;

    fn with_remote(names: &[&str]) -> (Harness, Rc<ThreadUi>) {
        let slot: Rc<RefCell<Option<Rc<ThreadUi>>>> = Rc::default();
        let grab = slot.clone();
        let mut h = common::start_app(move |app| {
            *grab.borrow_mut() = Some(app.ui.thread_ui.get(None));
            app
        });
        let ui = slot.borrow().clone().expect("the blank controllers");
        let urls = names.iter().map(|n| format!("https://x/{n}.png")).collect();
        ui.draft.set_remote_images(&ui.composer, urls, image);
        h.connect();
        h.wait_idle();
        (h, ui)
    }

    fn rows(h: &Harness) -> Vec<String> {
        h.t.screen()
            .lines()
            .map(|row| row.trim_end().to_string())
            .collect()
    }

    /// The rows from the first remote one to the prompt, blank ones kept.
    fn block(h: &Harness) -> Vec<String> {
        let rows = rows(h);
        let from = rows.iter().position(|r| r.trim() == "[Image #1]");
        let to = rows.iter().rposition(|r| r.starts_with('›'));
        match (from, to) {
            (Some(from), Some(to)) => rows[from..=to].to_vec(),
            _ => panic!("no remote rows over the prompt:\n{}", h.t.screen()),
        }
    }

    /// What the composer asked of the host: the app drains the model's asks into one
    /// `Command::Popovers` each, the fake records them.
    fn asks(h: &Harness) -> Vec<Ask> {
        h.fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Effect {
                    ask: codex_tui_core::app::effects::Ask::Popovers { ask, .. },
                    ..
                } => Some(ask),
                _ => None,
            })
            .collect()
    }

    fn last_image_read(h: &Harness) -> Option<ClipboardRead> {
        asks(h).into_iter().rev().find_map(|ask| match ask {
            Ask::ClipboardImage(read) => Some(read),
            _ => None,
        })
    }

    #[test]
    fn remote_images_are_rows_over_the_draft_with_a_blank_row_between() {
        // rows 1 to 4 of `remote_image_rows` (the oracle's composer stands
        // alone in its snapshot, with a footer of its own below)
        let (mut h, _ui) = with_remote(&["a", "b"]);
        h.t.type_text("describe these");
        h.advance(100);
        assert_eq!(
            block(&h),
            ["  [Image #1]", "  [Image #2]", "", "› describe these"],
            "{}",
            h.t.screen()
        );
        // `remote_image_rows_after_delete_first`: the first of two is
        // removed and the second is the first now
        h.press("home up up delete");
        h.advance(100);
        assert_eq!(
            block(&h),
            ["  [Image #1]", "", "› describe these"],
            "{}",
            h.t.screen()
        );
    }

    #[test]
    fn up_up_delete_removes_the_first_row_and_the_local_images_close_the_gap() {
        let (mut h, ui) = with_remote(&["a", "b"]);
        h.t.paste(&png("p.png").to_string_lossy());
        h.advance(100);
        assert!(h.t.screen().contains("› [Image #3]"), "{}", h.t.screen());
        h.press("home up up delete");
        h.advance(100);
        assert_eq!(
            ui.draft.remote_images(),
            vec!["https://x/b.png".to_string()]
        );
        assert_eq!(ui.draft.remote_selected(), Some(0));
        assert_eq!(
            block(&h)[0],
            "  [Image #1]",
            "the row that is left is the first now:\n{}",
            h.t.screen()
        );
        assert!(h.t.screen().contains("› [Image #2]"), "{}", h.t.screen());
    }

    #[test]
    fn up_off_the_start_of_the_draft_is_the_fields() {
        let (mut h, ui) = with_remote(&["a"]);
        h.t.type_text("one two");
        h.advance(100);
        h.press("up");
        assert_eq!(ui.draft.remote_selected(), None);
        h.press("home up");
        assert_eq!(ui.draft.remote_selected(), Some(0));
        // any other key ends the selection and is the field's
        h.press("right");
        assert_eq!(ui.draft.remote_selected(), None);
    }

    #[test]
    fn a_click_in_the_draft_ends_the_selection() {
        let (mut h, ui) = with_remote(&["a"]);
        h.t.type_text("one two");
        h.press("home up");
        assert_eq!(ui.draft.remote_selected(), Some(0));
        let prompt_row = rows(&h)
            .iter()
            .rposition(|r| r.starts_with('›'))
            .and_then(|y| u16::try_from(y).ok())
            .expect("prompt");
        h.t.click(6, prompt_row);
        h.advance(100);
        assert_eq!(ui.draft.remote_selected(), None);
    }

    #[test]
    fn ctrl_v_asks_the_host_for_the_clipboard_image_and_its_answer_is_attached() {
        let mut h = chat();
        h.t.type_text("see ");
        h.press("ctrl+v");
        h.wait_until("the ask", |h| last_image_read(h).is_some());
        let path = png("clip.png");
        h.inject([AppEv::Screen(ScreenEv::Popovers(Ev::ClipboardResult {
            read: last_image_read(&h).expect("the clipboard ask"),
            result: Ok(path.clone()),
        }))]);
        h.advance(100);
        assert!(
            h.t.screen().contains("› see [Image #1]"),
            "{}",
            h.t.screen()
        );
        h.press("enter");
        common::wait::wait_until(&mut h, "the turn", |h| !turns(h).is_empty());
        assert_eq!(turns(&h)[0].images, vec![LocalImage { path, range: 4..14 }]);
    }

    #[test]
    fn an_image_nobody_asked_for_is_not_attached() {
        let mut h = chat();
        h.t.type_text("see ");
        h.advance(100);
        h.inject([AppEv::Screen(ScreenEv::Popovers(Ev::ClipboardResult {
            read: ClipboardRead {
                thread: codex_tui_core::ids::ThreadId::new("thread-1"),
                number: 1,
            },
            result: Ok(png("x.png")),
        }))]);
        h.advance(100);
        assert!(!h.t.screen().contains("[Image #1]"), "{}", h.t.screen());
    }

    #[test]
    fn a_second_ctrl_v_within_the_repeat_window_asks_once() {
        let mut h = chat();
        h.t.press("ctrl+v");
        h.t.press("ctrl+v");
        h.wait_until("the ask", |h| last_image_read(h).is_some());
        h.advance(100);
        let asked = asks(&h)
            .iter()
            .filter(|ask| matches!(ask, Ask::ClipboardImage(_)))
            .count();
        assert_eq!(asked, 1);
    }
}

#[test]
fn failed_image_preparation_restores_the_local_and_remote_draft_before_retry() {
    let remote = "https://example.com/remote.png";
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .remote_images(&[remote])
        .open();
    s.hold_image_preparation();
    let path = s.attach_image("bad-image.png");
    s.type_text(" describe");
    s.assert_composer("[Image #2] describe");
    s.keys("enter");
    s.assert_sole_turn(
        kit::Turn::text("[Image #2] describe")
            .image_paths(&[&path])
            .remote_images(&[remote]),
    );
    s.assert_no_request("turn/start");
    s.fail_image_preparation("Failed to prepare image: missing file");
    s.assert_contains("Failed to prepare image: missing file");
    s.assert_composer("[Image #2] describe");
    s.keys("enter");
    s.assert_turn_texts(&["[Image #2] describe", "[Image #2] describe"]);
    s.assert_no_request("turn/start");
}

#[test]
fn pending_image_preparation_allows_editing_and_keeps_the_next_draft() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.hold_image_preparation();
    let path = s.attach_image("held-prepare.png");
    s.type_text(" describe").keys("enter");
    s.assert_no_request("turn/start");
    s.type_text("new draft").keys("left");
    s.assert_composer("new draft");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("[Image #1] describe").image_paths(&[&path]));
    s.assert_contains("new draft");
    s.fail_image_preparation("Failed to prepare image: interrupted");
    s.assert_contains("[Image #1] describe");
    s.assert_contains("new draft");
    s.assert_no_request("turn/start");
}

#[test]
fn clipboard_read_stays_with_the_thread_that_held_the_bang() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_burst("!");
    let first = s.ask_clipboard_image("alt+v");
    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_active_thread("thread-2");
    s.type_text("draft for the second thread");
    let second = s.ask_clipboard_image("alt+v");
    let image = kit::image_file("first-thread.png");
    s.answer_clipboard_read(first, Ok(image.clone())).wait(300);
    s.assert_composer("draft for the second thread");
    s.assert_absent("[Image #1]");
    s.answer_clipboard_read(second, Err("no image".to_owned()))
        .wait(300);
    s.assert_contains("Failed to paste image: no image");
    s.assert_composer("draft for the second thread");
    s.server_resumes("thread-1").visit_thread("thread-1");
    s.assert_active_thread("thread-1");
    s.wait(1000);
    s.assert_composer("[Image #1]!");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("[Image #1]!").images(&[(&image, 0..10)]));
    s.assert_started_turn_on("thread-1");
    s.assert_no_shell_command();
}

#[test]
fn clipboard_answer_to_a_cleared_draft_cannot_replace_the_next_read() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("first draft");
    let first = s.ask_clipboard_image("alt+v");
    s.keys("ctrl+c");
    s.assert_composer("");
    s.type_text("next draft");
    let second = s.ask_clipboard_image("alt+v");
    let old = kit::image_file("old.png");
    let new = kit::image_file("new.png");
    s.answer_clipboard_read(first, Ok(old)).wait(300);
    s.assert_composer("next draft");
    s.answer_clipboard_read(second, Ok(new.clone())).wait(300);
    s.assert_composer("next draft[Image #1]");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("next draft[Image #1]").images(&[(&new, 10..20)]));
    s.assert_no_shell_command();
}

#[test]
fn a_history_search_preview_leaves_the_original_clipboard_read_pending() {
    for (answer_in_search, accept) in [(true, false), (false, false), (false, true)] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .live()
            .message_history(&["earlier prompt"])
            .open();
        s.type_text("original draft");
        let read = s.ask_clipboard_image("alt+v");
        s.keys("ctrl+r").type_text("earlier");
        s.assert_contains("reverse-i-search: earlier");
        s.assert_contains("earlier prompt");
        let image = kit::image_file("search.png");
        if answer_in_search {
            s.answer_clipboard_read(read.clone(), Ok(image.clone()))
                .wait(300);
            s.assert_absent("[Image #1]");
        }
        s.keys(if accept { "enter" } else { "esc" });
        s.assert_absent("reverse-i-search");
        if !answer_in_search {
            s.answer_clipboard_read(read, Ok(image.clone())).wait(300);
        }
        if accept {
            s.assert_composer("earlier prompt");
        } else {
            s.assert_composer("original draft[Image #1]");
            s.keys("enter");
            s.assert_sole_turn(
                kit::Turn::text("original draft[Image #1]").images(&[(&image, 14..24)]),
            );
        }
    }
}

#[test]
fn clipboard_read_follows_the_blank_draft_into_the_started_thread() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .open();
    s.type_text("describe ");
    let read = s.ask_clipboard_image("alt+v");
    assert_ne!(read.thread.as_str(), "thread-1");
    let image = kit::image_file("startup.png");
    s.start_thread();
    s.assert_active_thread("thread-1");
    s.answer_clipboard_read(read, Ok(image.clone())).wait(300);
    s.assert_composer("describe [Image #1]");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("describe [Image #1]").images(&[(&image, 9..19)]));
    s.assert_started_turn_on("thread-1");
}

#[test]
fn restoring_an_older_failed_submission_preserves_the_next_drafts_clipboard_read() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.hold_image_preparation();
    let first = s.attach_image("previous.png");
    s.type_text(" describe").keys("enter");
    s.assert_no_request("turn/start");
    s.type_text("next draft");
    let read = s.ask_clipboard_image("alt+v");
    s.fail_image_preparation("Failed to prepare image");
    s.assert_saved_draft("[Image #1] describe\nnext draft");
    let second = kit::image_file("next.png");
    s.answer_clipboard_read(read, Ok(second.clone())).wait(300);
    let restored = "[Image #1] describe\nnext draft[Image #2]";
    s.assert_saved_draft(restored);
    s.keys("enter");
    let turn = s.turns().pop().expect("the merged draft was submitted");
    assert_eq!(turn.text, restored);
    assert_eq!(
        turn.images,
        [
            LocalImage {
                path: first,
                range: 0..10
            },
            LocalImage {
                path: second,
                range: 30..40
            },
        ]
    );
}

#[test]
fn a_rejected_start_returns_to_its_own_draft_and_preserves_a_later_edit() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .live()
        .open();
    s.hold_image_preparation();
    let first = s.attach_image("rejected-original.png");
    s.type_text(" rejected").keys("enter");
    s.assert_sole_turn(kit::Turn::text("[Image #1] rejected").images(&[(&first, 0..10)]));

    let second = s.attach_image("newer-draft.png");
    s.type_text(" newer").keys("left left");
    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_active_thread("thread-2");
    s.type_text("other draft").keys("left left left left left");

    s.reject_turn_start("submission rejected");
    s.assert_absent("submission rejected");
    s.type_text("X");
    s.assert_composer("other Xdraft");

    s.server_resumes("thread-1").visit_thread("thread-1");
    s.assert_active_thread("thread-1");
    s.assert_composer("[Image #1] rejected\n[Image #2] newer");
    s.type_text("X");
    let restored = "[Image #1] rejected\n[Image #2] newXer";
    s.assert_composer(restored);

    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_composer("other Xdraft");
    s.server_resumes("thread-1").visit_thread("thread-1");
    s.assert_composer(restored);
    s.keys("enter");
    s.assert_turn_texts(&["[Image #1] rejected", restored]);
    let turn = s
        .turns()
        .pop()
        .expect("the returned message was sent explicitly");
    assert_eq!(
        turn.images,
        [
            LocalImage {
                path: first,
                range: 0..10
            },
            LocalImage {
                path: second,
                range: 20..30
            },
        ]
    );
}

#[test]
fn a_rejected_start_keeps_the_remote_attachment_selected_in_the_later_draft() {
    const A: &str = "https://example.com/a.png";
    const B: &str = "https://example.com/b.png";
    const EARLIER: &str = "https://example.com/earlier.png";
    for recovered in [&[][..], &[B, EARLIER][..]] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(100, 24)
            .live()
            .open();
        s.hold_image_preparation();
        s.remote_images(recovered);
        let path = s.attach_image("rejected-image-with-selection.png");
        let original = format!("[Image #{}] original", recovered.len() + 1);
        s.type_text(" original").keys("enter");
        s.assert_sole_turn(
            kit::Turn::text(&original)
                .images(&[(&path, 0..10)])
                .remote_images(recovered),
        );

        s.remote_images(&[A, B]);
        s.type_text("newer").keys("home up");
        s.reject_turn_start("submission rejected");
        s.keys("delete enter");

        let remotes = [recovered, &[A]].concat();
        let restored = format!("[Image #{}] original\nnewer", remotes.len() + 1);
        s.assert_turn_texts(&[&original, &restored]);
        let sent = s.turns().pop().expect("the user resubmitted");
        assert_eq!(sent.remote_images, remotes);
        assert_eq!(sent.images, [LocalImage { path, range: 0..10 }]);
    }
}
