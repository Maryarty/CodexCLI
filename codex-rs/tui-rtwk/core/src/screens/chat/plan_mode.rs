//! Plan mode, the controller of one thread (oracle `chatwidget/plan_implementation.rs`,
//! `transcript.rs` `saw_plan_item_this_turn`, `turn_runtime.rs` `maybe_prompt_plan_implementation`):
//! what the thread's turn has shown of a proposed plan, and the question asked when the turn
//! ends. The page only shows the question (`Prompt`) and says what the user answered; the
//! decision to ask is made here, from the turn's own events.

use codex_protocol::config_types::{CollaborationMode, ModeKind, Settings};

/// The message that starts the work after a plan is approved
/// (`PLAN_IMPLEMENTATION_CODING_MESSAGE`).
pub const IMPLEMENT_MESSAGE: &str = "Implement the plan.";

/// Which completion prompt has priority for this turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PromptPriority {
    #[default]
    Ordinary,
    RateLimit,
}

/// The question "Implement this plan?" the turn's end raised. `seq` is its number in the
/// thread: the page opens each number once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub seq: u64,
    /// The markdown of the latest proposed plan: the row that clears the context needs it.
    pub plan: Option<String>,
}

/// The answers of the question (the ids of its rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Switch to Default and send the implement message.
    Implement,
    /// Start a fresh thread with the plan.
    ImplementFresh,
}

impl Answer {
    pub const YES: &'static str = "plan.yes";
    pub const CLEAR: &'static str = "plan.clear";
    pub const NO: &'static str = "plan.no";

    /// The answer a row id stands for; `NO` and unknown ids answer nothing.
    pub fn of_row(id: &str) -> Option<Answer> {
        match id {
            Self::YES => Some(Answer::Implement),
            Self::CLEAR => Some(Answer::ImplementFresh),
            _ => None,
        }
    }
}

/// The per-thread state of Plan mode, changed only through `ThreadPart`'s `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanMode {
    /// A plan item completed in the running turn and no steer came after it.
    saw_plan: bool,
    /// The plan's text streams (`is_plan_streaming_in_tui`): Enter queues instead of steering.
    streaming: bool,
    /// The latest completed plan's markdown (`latest_proposed_plan_markdown`).
    proposed: Option<String>,
    prompt: Option<Prompt>,
    asked: u64,
}

impl PlanMode {
    /// The question the last turn's end raised, if any.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    /// The plan streams in the running turn.
    pub fn streaming(&self) -> bool {
        self.streaming
    }

    pub(crate) fn plan_delta(&mut self) {
        self.streaming = true;
    }

    /// `on_plan_item_completed`.
    pub(crate) fn plan_completed(&mut self, text: &str) {
        self.streaming = false;
        self.saw_plan = true;
        self.proposed = Some(text.to_string());
    }

    /// A steer was sent: a plan before it is not the one to ask about
    /// (`saw_plan_item_this_turn = false`).
    pub(crate) fn steered(&mut self) {
        self.saw_plan = false;
    }

    /// The running turn ended (`maybe_prompt_plan_implementation`): asks when the turn
    /// completed in Plan mode with a proposed plan and nothing waits to be sent.
    pub(crate) fn turn_ended(
        &mut self,
        mode: ModeKind,
        completed: bool,
        waiting: bool,
        priority: PromptPriority,
    ) {
        self.streaming = false;
        if priority == PromptPriority::RateLimit {
            self.prompt = None;
        } else if completed && !waiting && mode == ModeKind::Plan && self.saw_plan {
            self.asked += 1;
            self.prompt = Some(Prompt {
                seq: self.asked,
                plan: self.proposed.clone(),
            });
        }
        self.saw_plan = false;
    }
}

/// The collaboration mode `kind` for the thread whose model is `model`; the server fills in
/// the mode's own instructions (the oracle's mask carries none either).
pub fn collaboration_mode(kind: ModeKind, model: Option<&str>) -> CollaborationMode {
    CollaborationMode {
        mode: kind,
        settings: Settings {
            model: model.unwrap_or_default().to_string(),
            reasoning_effort: None,
            developer_instructions: None,
        },
    }
}

/// How much of the context window the thread has used, as the prompt's clear-context row
/// says it (`plan_implementation_context_usage_label`): a percent when the window is known,
/// else the tokens in it; nothing for a fresh or unknown context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Used {
    Percent(i64),
    Tokens(i64),
}

pub fn used(
    context: Option<crate::screens::status::Context>,
    tokens: crate::screens::status::Tokens,
) -> Option<Used> {
    match context {
        Some(context) => {
            let percent = 100 - context.percent_left.clamp(0, 100);
            (percent > 0).then_some(Used::Percent(percent))
        }
        None => (tokens.total > 0).then_some(Used::Tokens(tokens.total)),
    }
}
