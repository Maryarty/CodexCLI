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
    use codex_tui_core::screens::popovers::{Ask, Ev};
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
                Command::Popovers { ask, .. } => Some(ask),
                _ => None,
            })
            .collect()
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
        h.wait_until("the ask", |h| asks(h).contains(&Ask::ClipboardImage));
        let path = png("clip.png");
        h.inject([AppEv::Screen(ScreenEv::Popovers(Ev::ImagePasted(
            path.clone(),
        )))]);
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
        h.inject([AppEv::Screen(ScreenEv::Popovers(Ev::ImagePasted(png(
            "x.png",
        ))))]);
        h.advance(100);
        assert!(!h.t.screen().contains("[Image #1]"), "{}", h.t.screen());
    }

    #[test]
    fn a_second_ctrl_v_within_the_repeat_window_asks_once() {
        let mut h = chat();
        h.t.press("ctrl+v");
        h.t.press("ctrl+v");
        h.wait_until("the ask", |h| asks(h).contains(&Ask::ClipboardImage));
        h.advance(100);
        let asked = asks(&h)
            .iter()
            .filter(|ask| **ask == Ask::ClipboardImage)
            .count();
        assert_eq!(asked, 1);
    }
}
