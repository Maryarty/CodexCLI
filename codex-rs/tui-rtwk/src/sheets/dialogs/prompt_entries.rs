//! The debug entries of the text prompt (`custom_prompt*`): the views the oracle's tests of
//! `custom_prompt_view_tests.rs` build, drawn into the whole screen as they draw them
//! (`TextPrompt::fill`). A prompt whose text was edited before its suggestion came is set up by
//! the harness after the prompt is mounted: the edit reaches the field, then the suggestion is
//! delivered (the two never in one build), as the oracle's tests call `handle_key_event` and
//! `handle_paste` and then `apply_text_suggestion`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::editing::TextSelection;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::kit::vim::{Input, Normal};

use super::strings;
use crate::app::debug::{DebugEntry, EntryOpen};
use crate::features::prompt::{Suggest, TextPrompt};
use crate::pages::session::composer::plain;
use crate::widgets::composer_draft::VimHost;
use crate::widgets::thread_ui::ThreadUi;

/// What the entry shows: the oracle's `custom_prompt_view()` with the setup its test makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The view as it is made.
    Plain,
    /// Vim editing, in Insert mode (`enable_vim_in_insert_mode`).
    Vim,
    /// The text `rename` in the field.
    Rename,
    /// The text `rename`, Vim editing.
    RenameVim,
    /// A suggestion asked for (`with_text_suggestion`): it arrives and fills the field.
    Prefill,
    /// A character typed and deleted before the suggestion: it is not applied.
    PrefillTyped,
    /// Text pasted before the suggestion: it is not applied.
    PrefillPasted,
    /// The text `Existing` with the caret moved left: the suggestion replaces it.
    PrefillCursor,
    /// Vim in Replace mode (`Esc`, `R`): the suggestion is applied and the mode stays.
    PrefillReplace,
}

impl Kind {
    fn vim(self) -> bool {
        matches!(self, Kind::Vim | Kind::RenameVim | Kind::PrefillReplace)
    }

    fn initial(self) -> &'static str {
        match self {
            Kind::Rename | Kind::RenameVim => "rename",
            Kind::PrefillCursor => "Existing",
            _ => "",
        }
    }

    /// What the suggestion says, when one is asked for.
    fn suggestion(self) -> Option<&'static str> {
        match self {
            Kind::Prefill => Some("Suggested text"),
            Kind::PrefillTyped | Kind::PrefillPasted | Kind::PrefillCursor => Some("Generated"),
            Kind::PrefillReplace => Some("abc"),
            _ => None,
        }
    }
}

/// The prompt of an entry, with the setup of its test.
#[derive(Clone, PartialEq)]
pub struct PromptEntry {
    pub kind: Kind,
}

pub struct PromptEntryState {
    ui: Rc<ThreadUi>,
    suggest: Suggest,
    started: Cell<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The prompt is mounted: the test's edits go in, then the suggestion arrives.
    Start,
    Submitted,
}

impl StatefulWidget for PromptEntry {
    type State = PromptEntryState;

    fn create_state(&self) -> PromptEntryState {
        PromptEntryState {
            ui: plain::ui(),
            suggest: if self.kind.suggestion().is_some() {
                Suggest::Waiting
            } else {
                Suggest::Off
            },
            started: Cell::new(false),
        }
    }
}

impl_widget!(stateful eq PromptEntry);

impl PromptEntryState {
    /// What the test does to the prompt before the suggestion arrives.
    fn edit(&self, kind: Kind) {
        let c = &self.ui.composer;
        match kind {
            Kind::PrefillTyped => {
                c.reset("x");
                c.reset("");
            }
            Kind::PrefillPasted => c.reset("Manual title"),
            Kind::PrefillCursor => {
                let at = c.text().len().saturating_sub(1);
                c.set_selection(TextSelection::collapsed(at));
            }
            Kind::PrefillReplace => {
                let mut host = VimHost::new(c, &self.ui.draft);
                self.ui.draft.with_vim(|v| {
                    v.escape(&mut host);
                    v.input(&mut host, Input::Normal(Normal::EnterReplaceMode));
                });
            }
            _ => {}
        }
    }
}

impl State for PromptEntryState {
    type Widget = PromptEntry;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &PromptEntry, _cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Start => {
                self.edit(w.kind);
                match w.kind.suggestion() {
                    Some(text) => {
                        self.suggest = Suggest::Ready(Some(text.to_owned()));
                        Rebuild::Yes
                    }
                    None => Rebuild::No,
                }
            }
            Msg::Submitted => Rebuild::No,
        }
    }

    fn build(&self, w: &PromptEntry, cx: &mut Cx<'_, Self>) -> AnyWidget {
        if !self.started.replace(true) {
            cx.local_sender()(Msg::Start);
        }
        let s = &strings::EN;
        let prompt = TextPrompt::new(
            s.debug_prompt_title,
            s.debug_prompt_placeholder,
            w.kind.initial(),
            self.suggest.clone(),
            cx.map(|_| Msg::Submitted),
        )
        .vim(w.kind.vim())
        .with_ui(self.ui.clone())
        .fill();
        if w.kind.suggestion().is_some() {
            prompt
                .labels(s.debug_prompt_loading, s.debug_prompt_ready)
                .into()
        } else {
            prompt.into()
        }
    }
}

/// The entries the debug menu lists for the prompt, each on a seeded session.
pub fn entries() -> Vec<DebugEntry> {
    const SCRIPT: &str = include_str!("fixtures/prompt.jsonl");
    macro_rules! entry {
        ($name:literal, $description:literal, $kind:expr) => {
            DebugEntry {
                name: $name,
                description: $description,
                script: SCRIPT,
                open: EntryOpen::Dialog(|_| PromptEntry { kind: $kind }.into()),
            }
        };
    }
    vec![
        entry!("custom_prompt", "the text prompt, empty", Kind::Plain),
        entry!(
            "custom_prompt_vim",
            "the text prompt with Vim editing, in Insert mode",
            Kind::Vim
        ),
        entry!(
            "custom_prompt_rename",
            "the text prompt over the text `rename`",
            Kind::Rename
        ),
        entry!(
            "custom_prompt_rename_vim",
            "the text prompt over the text `rename`, Vim editing",
            Kind::RenameVim
        ),
        entry!(
            "custom_prompt_prefill",
            "the text prompt with a suggestion that fills the field",
            Kind::Prefill
        ),
        entry!(
            "custom_prompt_prefill_typed",
            "a prompt typed in and emptied before its suggestion: kept empty",
            Kind::PrefillTyped
        ),
        entry!(
            "custom_prompt_prefill_pasted",
            "a prompt pasted into before its suggestion: the paste kept",
            Kind::PrefillPasted
        ),
        entry!(
            "custom_prompt_prefill_cursor",
            "a prompt whose caret moved before its suggestion: replaced",
            Kind::PrefillCursor
        ),
        entry!(
            "custom_prompt_prefill_replace",
            "a prompt in Vim Replace mode when its suggestion comes",
            Kind::PrefillReplace
        ),
    ]
}
