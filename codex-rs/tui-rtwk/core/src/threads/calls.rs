//! What an item says about the tool calls it stands for when a finished turn is folded
//! (oracle `HistoryCell::tool_call_labels` and the `ToolGroup` of
//! `transcript_view/turn_details.rs`). Typed facts only: a call's label is words, the view's
//! string table composes it from the item's own data; a call's id is its transcript key.

use super::{Item, ToolKind};

/// How a finished shell call counts in the folded summary: "Explored files", "Ran 2 commands"
/// or both ("Explored files · ran 2 commands").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellClass {
    /// Only read, listed or searched (the item is an exploration).
    Explored,
    /// Anything else the model ran.
    Ran,
}

impl Item {
    /// How many calls the item adds to a folded turn: one for a tool, an edit, a search, a
    /// viewed or generated image and an `update_plan` report; none for messages, notices and
    /// the user's own shell commands (oracle: the cell's `tool_call_labels().len()`).
    pub fn call_count(&self) -> usize {
        match self {
            Self::Tool(tool) => usize::from(!matches!(
                &tool.kind,
                ToolKind::Shell {
                    user_shell: true,
                    ..
                }
            )),
            Self::Patch(_)
            | Self::Web(_)
            | Self::ImageView { .. }
            | Self::Generated(_)
            | Self::PlanUpdate(_) => 1,
            _ => 0,
        }
    }

    /// The class a model's shell command counts under once it finished with exit code 0; none
    /// while it runs, when it failed or was declined, and for anything that is not one
    /// (oracle: `exec.iter_calls().all(|call| exit_code == 0)` then `is_exploring_cell`).
    pub fn shell_class(&self) -> Option<ShellClass> {
        let Self::Tool(tool) = self else {
            return None;
        };
        let ToolKind::Shell {
            actions,
            exit_code: Some(0),
            user_shell: false,
            ..
        } = &tool.kind
        else {
            return None;
        };
        Some(if actions.is_empty() {
            ShellClass::Ran
        } else {
            ShellClass::Explored
        })
    }
}
