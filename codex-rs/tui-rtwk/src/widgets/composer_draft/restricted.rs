//! What a restricted composer does to its draft (`chat_composer/reconnect.rs`
//! `handle_restricted_key`): the history search ends with the draft as it
//! was, no row of a remote image is selected, the popups are closed and the
//! pastes are expanded where they stand, so the draft shows what Enter would
//! run. Images and mentions stay, and so does the caret.

use rtwk::editing::{TextEditingController, TextSelection};

use super::ComposerDraft;

impl ComposerDraft {
    /// Puts the draft in the state a restricted key finds it in. Whether a
    /// paste was expanded: that key does not run a command (the command
    /// must be on the screen before Enter sends it).
    pub fn restrict(&self, c: &TextEditingController) -> bool {
        self.search_cancel(c);
        self.clear_remote_selection();
        self.mentions().close();
        self.expand_pastes(c)
    }

    /// Each paste placeholder is replaced by the text it stands for, last
    /// first so the ranges before it stay valid; the elements after it move
    /// with the change and the caret keeps its place in the text. Whether
    /// there was one.
    pub fn expand_pastes(&self, c: &TextEditingController) -> bool {
        let pastes: Vec<_> = self
            .elements(c)
            .into_iter()
            .filter(|e| e.image.is_none() && e.binding.is_none() && e.payload != e.label)
            .collect();
        if pastes.is_empty() {
            return false;
        }
        let mut caret = c.selection().extent_offset;
        for e in pastes.iter().rev() {
            c.replace(e.start, e.end, &e.payload);
            self.sync(c);
            if caret >= e.end {
                caret = caret + e.payload.len() - (e.end - e.start);
            } else if caret > e.start {
                caret = e.start + e.payload.len();
            }
        }
        c.set_selection(TextSelection::collapsed(caret));
        true
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn big() -> String {
        ["x"; 1001].concat()
    }

    fn paste(d: &ComposerDraft, c: &TextEditingController) {
        assert!(d.paste(c, &big(), |n| format!("[{n}]"), |n| format!("<{n}>")));
    }

    /// `reconnect_expands_pastes_preserving_images_and_cursor`.
    #[test]
    fn pastes_expand_and_the_image_and_the_caret_stay() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        paste(&d, &c);
        d.attach_image(&c, PathBuf::from("local.png"), |n| format!("<{n}>"));
        paste(&d, &c);
        let expanded = d.expanded(&c);
        assert!(d.expand_pastes(&c));
        assert_eq!(&*c.text(), expanded.as_str());
        assert_eq!(c.selection().extent_offset, expanded.len());
        let left = d.elements(&c);
        assert_eq!(left.len(), 1);
        assert!(left[0].image.is_some());
        // the pastes are text now: a second call has nothing to do
        assert!(!d.expand_pastes(&c));
        c.insert("!");
        assert_eq!(&*c.text(), [expanded.as_str(), "!"].concat());
    }

    #[test]
    fn a_caret_before_a_paste_stays_where_it_was() {
        let c = TextEditingController::new("ab");
        let d = ComposerDraft::default();
        c.set_selection(TextSelection::collapsed(2));
        paste(&d, &c);
        c.set_selection(TextSelection::collapsed(1));
        assert!(d.expand_pastes(&c));
        assert_eq!(c.selection().extent_offset, 1);
        assert_eq!(&*c.text(), ["ab", big().as_str()].concat());
    }

    #[test]
    fn restrict_ends_the_history_search_and_the_remote_row() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        d.set_remote_images(&c, vec!["https://a/b.png".to_string()], |n| {
            format!("<{n}>")
        });
        assert!(d.remote_up(&c));
        assert_eq!(d.remote_selected(), Some(0));
        d.restrict(&c);
        assert_eq!(d.remote_selected(), None);
        assert_eq!(d.remote_count(), 1);
    }
}
