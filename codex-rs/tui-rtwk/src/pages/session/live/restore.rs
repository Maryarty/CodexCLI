//! The composer takes back messages the model gave it (`restore_composer_state`):
//! the steers and the queue after an interrupt, a steer that failed, the
//! newest queued message for editing. The model decides what and in which
//! order (`Restore::onto`); the composer controller only holds the text.

use std::path::PathBuf;

use codex_tui_core::threads::UserMessage;

use crate::widgets::thread_ui::ThreadUi;

/// Puts `message` in the composer of `ui`: its text, each local image's
/// placeholder an attached element again, the caret at the end. `image` words
/// a placeholder from its number.
pub(crate) fn put_back(ui: &ThreadUi, message: &UserMessage, image: impl Fn(usize) -> String) {
    ui.draft.clear();
    ui.draft.set_shell(false);
    ui.composer.reset("");
    let mut images: Vec<(&std::ops::Range<usize>, &PathBuf)> = message
        .images
        .iter()
        .map(|image| (&image.range, &image.path))
        .collect();
    images.sort_by_key(|(range, _)| range.start);
    let mut cursor = 0;
    for (range, path) in images {
        if range.start < cursor || range.end > message.text.len() {
            continue;
        }
        ui.composer.insert(&message.text[cursor..range.start]);
        ui.draft.attach_image(&ui.composer, path.clone(), &image);
        cursor = range.end;
    }
    ui.composer.insert(&message.text[cursor..]);
    ui.draft.sync(&ui.composer);
}
