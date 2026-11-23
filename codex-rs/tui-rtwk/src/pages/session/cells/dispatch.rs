//! One function picks the widget of a transcript item (Flutter: the
//! `itemBuilder` of a `ListView`, a `switch` on the item's type). The pane
//! lays the rows out and calls [`cell`]; what is drawn for an item, and in
//! which mode (rich, or the raw source of alt+r), is decided here and nowhere
//! else.
//!
//! The width is not an input: every cell sizes itself to the constraints it
//! is given (`LayoutBuilder` where it must choose a spelling).

use std::sync::Arc;

use rtwk::AnyWidget;
use rtwk::layout::{EdgeInsets, Padding};
use rtwk::style::Style;
use rtwk_rich::markdown::MarkdownTheme;

use codex_tui_core::threads::{Item, ToolActivity};

use crate::app::theme::Tokens;
use crate::pages::session::strings::Strings as Chat;
use crate::pages::session::transcript_activity::ToolRow;

use super::strings::Strings;
use super::{
    agent, collab, delegated, deprecation, error, generated_image, hook, note, notice, patch, plan,
    plan_update, raw, reasoning, status, user, viewed_image, warning, web,
};

/// What every cell is drawn with: the theme, the tables and the view's modes.
pub struct CellEnv<'a> {
    pub tokens: &'a Tokens,
    pub markdown: &'a MarkdownTheme,
    /// The surface of a prompt.
    pub prompt_surface: &'a Style,
    pub chat: &'static Chat,
    pub cells: &'static Strings,
    /// The agent's bullet and its style.
    pub bullet: &'a str,
    pub bullet_style: &'a Style,
    pub cwd: Option<&'a str>,
    /// The detailed transcript (Ctrl+T).
    pub detailed: bool,
    /// Raw output mode (alt+r, `/raw`): source text, no rendering.
    pub raw: bool,
    /// `show_raw_agent_reasoning`: a reasoning block also draws the raw reasoning.
    pub show_raw_reasoning: bool,
}

/// Where the item sits in the list.
#[derive(Default)]
pub struct Place {
    /// Something is above it: a blank row separates them.
    pub separator: bool,
    /// A prompt that follows another with no turn between (no blank row).
    pub joins_prompt: bool,
    /// The prompt a browsing sheet marks.
    pub highlighted: bool,
    /// The agent message is still being written.
    pub streaming: bool,
    /// The calls of the run an exploration or computer call row shares,
    /// after the first.
    pub rest: Vec<Arc<ToolActivity>>,
}

/// A cell below the one before it, a blank row between them; it takes the
/// whole width (only the agent's answer keeps a column free, oracle
/// messages.rs `right_reserve = 1`).
fn below(separator: bool, child: AnyWidget) -> AnyWidget {
    Padding::new(EdgeInsets::only(0, i32::from(separator), 0, 0), child).into()
}

/// The cell of `item`.
pub fn cell(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Place {
        separator,
        joins_prompt,
        highlighted,
        streaming,
        rest,
    } = place;
    let separator = *separator;
    let cells = env.cells;
    let chat = env.chat;
    if env.raw
        && let Some(lines) = raw::lines(item, cells)
    {
        // a prompt that joins the one above has no blank row, raw or not
        let blank = separator && !(matches!(item, Item::User(_)) && *joins_prompt);
        return below(blank, raw::cell(lines, cells));
    }
    let tokens = env.tokens;
    match item {
        Item::User(prompt) => {
            let panel = user::panel(
                prompt,
                &user::Look {
                    surface: env.prompt_surface.clone(),
                    tokens,
                    chat,
                    cells,
                    markdown: env.markdown,
                    highlighted: *highlighted,
                },
                *joins_prompt,
            );
            // A separator belongs to the transcript, outside the prompt surface.
            below(separator && !joins_prompt, panel.into())
        }
        Item::Agent { text, .. } => below(
            separator,
            agent::message(
                text,
                *streaming,
                env.cwd,
                &agent::Look {
                    bullet: env.bullet,
                    bullet_style: env.bullet_style.clone(),
                    markdown: env.markdown,
                    cells,
                },
            ),
        ),
        Item::Tool(tool) => below(
            separator,
            ToolRow {
                first: tool.clone(),
                rest: rest.clone(),
                detailed: env.detailed,
            }
            .into(),
        ),
        Item::Patch(activity) => below(
            separator,
            patch::PatchCell {
                activity: activity.clone(),
                cwd: env.cwd.map(str::to_owned),
                detailed: env.detailed,
            }
            .into(),
        ),
        Item::Web(activity) => below(
            separator,
            web::WebCell {
                activity: activity.clone(),
                detailed: env.detailed,
            }
            .into(),
        ),
        Item::ImageView { name, path } => below(
            separator,
            viewed_image::ViewedImageCell {
                name: name.clone(),
                path: path.clone(),
                detailed: env.detailed,
            }
            .into(),
        ),
        Item::Notice(notice) => below(
            separator,
            notice::NoticeCell {
                notice: notice.clone(),
            }
            .into(),
        ),
        Item::Note(note) => below(separator, note::NoteCell { note: note.clone() }.into()),
        Item::Plan { text } => below(
            separator,
            plan::proposed_plan(
                text,
                env.cwd,
                env.prompt_surface.clone(),
                env.markdown,
                tokens,
                cells,
            ),
        ),
        Item::Delegated(message) => below(
            separator,
            delegated::DelegatedCell {
                message: message.clone(),
            }
            .into(),
        ),
        Item::Generated(image) => below(
            separator,
            generated_image::GeneratedImageCell {
                image: image.clone(),
            }
            .into(),
        ),
        Item::Collab(event) => below(
            separator,
            collab::CollabCell {
                event: event.clone(),
            }
            .into(),
        ),
        Item::Hook(run) => below(separator, hook::HookCell { run: run.clone() }.into()),
        Item::Status(card) => below(separator, status::StatusCell { card: card.clone() }.into()),
        Item::PlanUpdate(update) => below(
            separator,
            plan_update::PlanUpdateCell {
                update: update.clone(),
                detailed: env.detailed,
            }
            .into(),
        ),
        Item::Error { message } => below(
            separator,
            error::ErrorCell {
                message: message.clone(),
            }
            .into(),
        ),
        Item::Deprecation { summary, details } => below(
            separator,
            deprecation::DeprecationCell {
                summary: summary.clone(),
                details: details.clone(),
            }
            .into(),
        ),
        Item::Warning { message } => below(
            separator,
            warning::WarningCell {
                message: message.clone(),
            }
            .into(),
        ),
        Item::LocalWarning(warning) => below(
            separator,
            warning::WarningCell {
                message: chat.local_warning(*warning).to_string(),
            }
            .into(),
        ),
        Item::StartupWarning(warning) => below(
            separator,
            warning::WarningCell {
                message: chat.mcp_warning(warning),
            }
            .into(),
        ),
        Item::HighLoad => below(
            separator,
            error::ErrorCell {
                message: chat.high_load.to_string(),
            }
            .into(),
        ),
        Item::Decision(decided) => below(
            separator,
            crate::sheets::approvals::DecisionCell {
                decided: decided.clone(),
            }
            .into(),
        ),
        Item::Review(line) => below(
            separator,
            crate::sheets::approvals::ReviewLineCell { line: line.clone() }.into(),
        ),
        // A block with nothing to say (its parts empty or `<!-- -->`) draws nothing, not even
        // the blank row above it (oracle: an empty layout takes no separator).
        Item::Reasoning { .. } => {
            let summary = item.reasoning_text(env.show_raw_reasoning);
            if summary.trim().is_empty() {
                return AnyWidget::default();
            }
            below(
                separator,
                reasoning::ReasoningCell {
                    summary: summary.to_owned(),
                }
                .into(),
            )
        }
    }
}
