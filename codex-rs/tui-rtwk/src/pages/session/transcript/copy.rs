//! What the reader copied out of the transcript and what came of it (oracle
//! `transcript_view/selection.rs` `copy_selected_text_with`, `composer_gap.rs`
//! `show_copy_feedback`): the UI asks the host's clipboard with `Intent::Effect` (the app's one
//! door for asks outside the thread RPCs), core keeps the host's answer under the request
//! number (`CodexApp::effect`), and what the gap above the composer says of it is drawn state,
//! kept here in the UI (ADR-013: `Done::Copy` is the fact, the line is the view's).
//!
//! The log is the UI's follower of core's answers, like the controllers `follow_core` keeps in
//! step: after each event it reads the newest answer, and when that is one of its own copies it
//! becomes the feedback. The oracle's rule that a second copy refused as busy leaves the copy
//! under way showing is kept here (`show_copy_feedback`).

use super::TranscriptMode;
use std::collections::BTreeMap;

use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::facts::TuiSettings;

/// The bit of the request numbers of the copies the transcript asks (the other bits are listed
/// in `core/src/app/effects.rs`: `1 << 58` is the features', `1 << 59` to `1 << 62` the screens').
pub const REQUEST_BIT: u64 = 1 << 57;

/// How many sent copies are remembered: the host answers them in order and the app keeps as
/// many answers.
const KEPT: usize = 32;

/// What the reader asks to be copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyJob {
    pub text: String,
    pub format: CopyFormat,
    pub destination: CopyDestination,
    /// The text the selection had, to let it go once the host confirms the copy
    /// (`tui.copy_on_select_clear_selection`; oracle `pending_copy`): only a selection that
    /// still reads the same, not one made since.
    pub clear_after: Option<String>,
    /// The transcript goes back to its last row once the host confirms the copy (Enter on a
    /// selection: copy and follow; oracle `CopyAndFollow`).
    pub follow: bool,
}

impl CopyJob {
    /// What the reader selected, as plain text for the clipboard. An `explicit` copy (a key or
    /// the right button) lets the selection go once the host confirms it; one a release made
    /// does so only when `tui.copy_on_select_clear_selection` says (oracle `selection_copy`:
    /// `SelectionCopy::EXPLICIT`, `SelectionCopy::on_select`).
    pub fn selected(text: String, explicit: bool, settings: &TuiSettings) -> CopyJob {
        CopyJob {
            clear_after: (explicit || settings.deselect_after_copy).then(|| text.clone()),
            text,
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            follow: false,
        }
    }

    /// Text that is not a selection (a warning's details): copied as it is, nothing to let go.
    pub fn plain(text: String) -> CopyJob {
        CopyJob {
            text,
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            clear_after: None,
            follow: false,
        }
    }

    /// Back to the last row once the copy is confirmed.
    #[must_use]
    pub fn following(mut self) -> CopyJob {
        self.follow = true;
        self
    }

    /// The ask the host runs.
    pub fn ask(&self) -> Ask {
        Ask::Copy {
            text: self.text.clone(),
            format: self.format.clone(),
            destination: self.destination.clone(),
        }
    }
}

/// What the last copy came to, until the next answer: the gap above the composer words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feedback {
    /// How many answers have been taken in: a view tells a new answer from the one it has
    /// shown (a pending copy that ends is a new answer, with its own five seconds).
    pub version: u64,
    /// The copy this is the answer of.
    pub request: u64,
    /// How the copy was delivered; the host's failure text is the view's to word, so only
    /// that it failed is kept (oracle `Result<CopyStatus, ()>`).
    pub result: Result<CopyStatus, ()>,
    /// How many characters the copy carried (the oracle counts what was copied, not what is
    /// selected).
    pub characters: usize,
    /// The text of the selection to let go of when the copy is delivered.
    pub clear_after: Option<String>,
    /// The reader goes back to the last row when the copy is delivered.
    pub follow: bool,
}

/// What the view provides to the transcript and the composer gap: the feedback to show.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown(pub Option<Feedback>);

struct Sent {
    characters: usize,
    clear_after: Option<String>,
    follow: bool,
}

/// The UI's record of the copies it asked and the feedback they came to.
#[derive(Default)]
pub struct CopyLog {
    asks: u64,
    sent: BTreeMap<u64, Sent>,
    /// The newest answer already taken in.
    seen: Option<(u64, Result<CopyStatus, String>)>,
    answers: u64,
    feedback: Option<Feedback>,
}

impl CopyLog {
    /// A copy is asked: the number it goes under and what the host is asked.
    pub fn ask(&mut self, job: &CopyJob) -> (u64, Ask) {
        self.asks += 1;
        let request = REQUEST_BIT | self.asks;
        self.sent.insert(
            request,
            Sent {
                characters: job.ask().characters(),
                clear_after: job.clear_after.clone(),
                follow: job.follow,
            },
        );
        while self.sent.len() > KEPT {
            self.sent.pop_first();
        }
        (request, job.ask())
    }

    /// Takes in the newest answer of the app (`CodexApp::latest_effect`), once, if it is one of
    /// the copies asked.
    pub fn take_in(&mut self, latest: Option<(u64, &Done)>) {
        let Some((request, Done::Copy(result))) = latest else {
            return;
        };
        if request & REQUEST_BIT == 0 || self.seen.as_ref() == Some(&(request, result.clone())) {
            return;
        }
        self.seen = Some((request, result.clone()));
        let Some(sent) = self.sent.get(&request) else {
            return;
        };
        let result = result.clone().map_err(|_| ());
        // A second copy while one is under way is refused by the host as busy; the feedback
        // of the one under way stays (oracle `show_copy_feedback`).
        if result == Ok(CopyStatus::Busy)
            && self
                .feedback
                .as_ref()
                .is_some_and(|shown| matches!(shown.result, Ok(CopyStatus::Pending(_))))
        {
            return;
        }
        self.answers += 1;
        self.feedback = Some(Feedback {
            version: self.answers,
            request,
            result,
            characters: sent.characters,
            clear_after: sent.clear_after.clone(),
            follow: sent.follow,
        });
    }

    pub fn shown(&self) -> Shown {
        Shown(self.feedback.clone())
    }
}

/// The copy of a block as a mode of the transcript (`mode_list!`): no fixture states of its own.
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "copy";
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn job(text: &str) -> CopyJob {
        CopyJob {
            text: text.to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
            clear_after: None,
            follow: false,
        }
    }

    /// The app has kept `done` as the newest answer, under `request`.
    fn answer(log: &mut CopyLog, request: u64, done: Done) {
        log.take_in(Some((request, &done)));
    }

    fn feedback(log: &CopyLog) -> Option<Feedback> {
        log.shown().0
    }

    #[test]
    fn nothing_is_shown_until_the_host_answers() {
        let mut log = CopyLog::default();
        log.ask(&job("row 03"));
        assert_eq!(feedback(&log), None);
    }

    #[test]
    fn a_confirmed_copy_is_counted_in_what_was_copied_not_in_bytes() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("größe ✓"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Confirmed)));
        let got = feedback(&log).expect("feedback");
        assert_eq!(got.result, Ok(CopyStatus::Confirmed));
        assert_eq!(got.characters, 7);
    }

    #[test]
    fn a_failure_keeps_only_that_it_failed() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("x"));
        answer(&mut log, request, Done::Copy(Err("no tool".into())));
        assert_eq!(feedback(&log).expect("feedback").result, Err(()));
    }

    #[test]
    fn a_pending_copy_that_ends_is_a_new_answer() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("abc"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Pending(1))));
        let pending = feedback(&log).expect("feedback");
        assert_eq!(pending.result, Ok(CopyStatus::Pending(1)));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Confirmed)));
        let done = feedback(&log).expect("feedback");
        assert_eq!(done.result, Ok(CopyStatus::Confirmed));
        assert!(done.version > pending.version);
    }

    #[test]
    fn a_busy_answer_leaves_the_copy_under_way_showing() {
        let mut log = CopyLog::default();
        let (first, _) = log.ask(&job("abc"));
        answer(&mut log, first, Done::Copy(Ok(CopyStatus::Pending(1))));
        let under_way = feedback(&log);
        let (second, _) = log.ask(&job("def"));
        answer(&mut log, second, Done::Copy(Ok(CopyStatus::Busy)));
        assert_eq!(feedback(&log), under_way);
        // and what the first comes to afterwards is shown
        answer(&mut log, first, Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(
            feedback(&log).expect("feedback").result,
            Ok(CopyStatus::Confirmed)
        );
    }

    #[test]
    fn busy_alone_is_shown() {
        let mut log = CopyLog::default();
        let (request, _) = log.ask(&job("abc"));
        answer(&mut log, request, Done::Copy(Ok(CopyStatus::Busy)));
        assert_eq!(
            feedback(&log).expect("feedback").result,
            Ok(CopyStatus::Busy)
        );
    }

    #[test]
    fn an_answer_to_another_ask_is_none_of_its_business() {
        let mut log = CopyLog::default();
        log.ask(&job("abc"));
        answer(&mut log, 7, Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(feedback(&log), None);
    }
}
