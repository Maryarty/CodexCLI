//! The remote images above the draft (`chat_composer/attachment_state.rs`):
//! the URLs a restored message carried, shown as `[Image #N]` rows over the
//! text, the row Up selected, and the local images numbered after them.
//! They are not in the text: only their count moves the numbers of the
//! local `[Image #N]` placeholders.

use rtwk::editing::TextEditingController;

use super::ComposerDraft;

/// The URLs and the selected row.
#[derive(Default)]
pub(super) struct Remote {
    pub(super) urls: Vec<String>,
    pub(super) selected: Option<usize>,
    /// Where the caret stood when the row was selected: a caret that stands
    /// elsewhere now (a click, a key of the field) ended the selection.
    pub(super) caret: usize,
}

impl ComposerDraft {
    /// The URLs of the remote images, in the order of their rows.
    pub fn remote_images(&self) -> Vec<String> {
        self.0.borrow().remote.urls.clone()
    }

    /// How many remote images the draft has (the local ones are numbered
    /// after them).
    pub fn remote_count(&self) -> usize {
        self.0.borrow().remote.urls.len()
    }

    /// The row Up selected, `None` when no row is.
    pub fn remote_selected(&self) -> Option<usize> {
        self.0.borrow().remote.selected
    }

    /// The remote images are `urls` now (`set_remote_image_urls`): no row
    /// is selected and the local images take the numbers after them.
    pub fn set_remote_images(
        &self,
        c: &TextEditingController,
        urls: Vec<String>,
        image: impl Fn(usize) -> String,
    ) {
        {
            let mut inner = self.0.borrow_mut();
            inner.remote.urls = urls;
            inner.remote.selected = None;
        }
        self.relabel_images(c, image);
    }

    /// The remote images, taken away (`take_remote_image_urls`): the local
    /// images are numbered from 1 again.
    pub fn take_remote_images(
        &self,
        c: &TextEditingController,
        image: impl Fn(usize) -> String,
    ) -> Vec<String> {
        let urls = {
            let mut inner = self.0.borrow_mut();
            inner.remote.selected = None;
            std::mem::take(&mut inner.remote.urls)
        };
        self.relabel_images(c, image);
        urls
    }

    /// Forgets the remote images with no renumbering (the draft went out
    /// or was cleared with them).
    pub fn forget_remote_images(&self) {
        let mut inner = self.0.borrow_mut();
        inner.remote.urls.clear();
        inner.remote.selected = None;
    }

    /// No row is selected any more (any other key, a click in the text).
    /// Whether one was.
    pub fn clear_remote_selection(&self) -> bool {
        self.0.borrow_mut().remote.selected.take().is_some()
    }

    /// Ends the selection of a row once the caret or a selection of the text
    /// is not where it was (a click: `mouse.rs` clears it on a handled
    /// mouse event). Whether it ended.
    pub fn remote_deselect_on_move(&self, c: &TextEditingController) -> bool {
        let mut inner = self.0.borrow_mut();
        let remote = &mut inner.remote;
        let selection = c.selection();
        if remote.selected.is_some()
            && (!selection.is_collapsed() || selection.extent_offset != remote.caret)
        {
            remote.selected = None;
            return true;
        }
        false
    }

    /// Up (`handle_remote_image_selection_key`): with a row selected the
    /// one above is, with the caret at the draft's start the last row is.
    /// `false`: the key is not this one's (no row, a selection in the text,
    /// the caret elsewhere).
    pub fn remote_up(&self, c: &TextEditingController) -> bool {
        let mut inner = self.0.borrow_mut();
        let count = inner.remote.urls.len();
        if count == 0 || !c.selection().is_collapsed() {
            return false;
        }
        match inner.remote.selected {
            Some(at) => inner.remote.selected = Some(at.saturating_sub(1)),
            None if c.selection().extent_offset == 0 => inner.remote.selected = Some(count - 1),
            None => return false,
        }
        inner.remote.caret = c.selection().extent_offset;
        true
    }

    /// Down: the row below is selected, past the last none is. `false` when
    /// no row is selected (the key is the field's).
    pub fn remote_down(&self, c: &TextEditingController) -> bool {
        let mut inner = self.0.borrow_mut();
        let count = inner.remote.urls.len();
        if count == 0 || !c.selection().is_collapsed() {
            return false;
        }
        let Some(at) = inner.remote.selected else {
            return false;
        };
        inner.remote.selected = (at + 1 < count).then_some(at + 1);
        true
    }

    /// Whether Delete or Backspace would remove a row now.
    pub fn remote_removable(&self, c: &TextEditingController) -> bool {
        let inner = self.0.borrow();
        !inner.remote.urls.is_empty()
            && inner.remote.selected.is_some()
            && c.selection().is_collapsed()
    }

    /// Delete or Backspace on the selected row removes it and the next one
    /// (or the last) is selected; the local images are numbered again.
    /// `false` when no row is selected.
    pub fn remote_remove(
        &self,
        c: &TextEditingController,
        image: impl Fn(usize) -> String,
    ) -> bool {
        if !self.remote_removable(c) {
            return false;
        }
        {
            let mut inner = self.0.borrow_mut();
            let remote = &mut inner.remote;
            match remote.selected {
                Some(at) if at < remote.urls.len() => {
                    remote.urls.remove(at);
                    remote.selected =
                        (!remote.urls.is_empty()).then(|| at.min(remote.urls.len() - 1));
                }
                _ => remote.selected = None,
            }
        }
        self.relabel_images(c, image);
        self.0.borrow_mut().remote.caret = c.selection().extent_offset;
        true
    }
}
