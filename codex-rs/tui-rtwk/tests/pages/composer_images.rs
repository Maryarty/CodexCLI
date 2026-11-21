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
