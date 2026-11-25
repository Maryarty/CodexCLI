//! The name prompt of `/rename` (ADR-013): the shared text prompt over the thread's name, which
//! says it wants the thread's title suggestion (`Interest::Title`) for as long as it is open and
//! shows what the feature keeps of it (`State::title`). In Flutter it is a `TextField` sheet with a
//! `FutureBuilder` for the suggestion: the prompt puts it in the field once, while the user has not
//! typed (`apply_text_suggestion`).

use std::rc::Rc;

use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::rename::{Intent, Interest};
use codex_tui_core::features::{FeatureStates, Remote};
use codex_tui_core::ids::ThreadId;

use super::strings;
use crate::features::prompt::{Suggest, TextPrompt};
use crate::features::verb;
use crate::widgets::interest::Interest as Wanted;

/// The name prompt of `thread`, whose current name (empty: none) fills the field.
#[derive(Clone, Debug, PartialEq)]
pub struct NamePrompt {
    pub thread: ThreadId,
    pub existing: String,
}

pub struct NamePromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Submit(Rc<str>),
}

impl StatefulWidget for NamePrompt {
    type State = NamePromptState;

    fn create_state(&self) -> NamePromptState {
        NamePromptState
    }
}

impl_widget!(stateful eq NamePrompt);

impl State for NamePromptState {
    type Widget = NamePrompt;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &NamePrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Submit(name) = msg;
        verb(
            cx,
            Intent::Set {
                thread: w.thread.clone(),
                name: name.to_string(),
            },
        );
        Rebuild::No
    }

    fn build(&self, w: &NamePrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // The suggestion of this thread's prompt only: another thread's answer does not rebuild it.
        let thread = w.thread.clone();
        let suggest = select::<FeatureStates, Suggest, _>(cx, move |features| {
            match features.rename.title(&thread) {
                Some(Remote::Ready(title)) => Suggest::Ready(title.clone()),
                Some(Remote::Failed(_)) => Suggest::Ready(None),
                _ => Suggest::Waiting,
            }
        });
        let prompt = TextPrompt::new(
            strings::title(!w.existing.is_empty()),
            strings::PLACEHOLDER,
            w.existing.clone(),
            suggest,
            cx.map(Msg::Submit),
        );
        Wanted::new(
            Interest::Title {
                thread: w.thread.clone(),
            },
            prompt,
        )
        .into()
    }
}
