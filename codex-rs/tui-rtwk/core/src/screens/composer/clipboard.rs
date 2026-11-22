//! The clipboard's image (Ctrl+V): the host reads it into a file, the
//! answer is the file's path (`clipboard_paste.rs` `paste_image_to_temp_png`,
//! `chatwidget/clipboard.rs` `paste_image`). A composer that asked takes the
//! answer that comes after its question: it counts the answers when it asks
//! and attaches the latest once there are more.

use std::path::{Path, PathBuf};

/// What the host answered to the Ctrl+V asks so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pasted {
    answers: u64,
    latest: Option<PathBuf>,
}

impl Pasted {
    /// The host wrote the clipboard's image to `path`.
    pub fn on_image(&mut self, path: PathBuf) {
        self.answers += 1;
        self.latest = Some(path);
    }

    /// How many images the host has answered with.
    pub fn answers(&self) -> u64 {
        self.answers
    }

    /// The file of the latest answer.
    pub fn latest(&self) -> Option<&Path> {
        self.latest.as_deref()
    }

    /// The file of the first answer after the `answers`-th, if there is one
    /// by now.
    pub fn after(&self, answers: u64) -> Option<&Path> {
        (self.answers > answers).then(|| self.latest()).flatten()
    }
}
