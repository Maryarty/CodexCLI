//! `startup` (S05, startup prompts): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/startup/prompts/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::app::Command as AppCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::onboarding;
use crate::threads::ThreadFx;

pub mod cwd;
pub mod daemon;
pub mod launch;

pub const NAME: &str = "startup";
pub const ID: &str = "S05";
pub const KIND: Kind = Kind::Dialog;

/// Whether the session is resumed or forked (the oracle's `SessionStartAction`
/// verb, the cwd prompt's `CwdPromptAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Resume,
    Fork,
}

/// What the cwd prompt returns (oracle `CwdSelection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CwdSelection {
    Current,
    Session,
    CurrentAndRemember,
    SessionAndRemember,
}

impl CwdSelection {
    /// The choice that is remembered in the config, if it is one.
    pub fn remembers(self) -> bool {
        matches!(self, Self::CurrentAndRemember | Self::SessionAndRemember)
    }
}

/// The cwd prompt (oracle `cwd_prompt.rs`): paths are data, the words the view's.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct CwdPrompt {
    pub action: Action,
    pub current: String,
    pub session: String,
    /// The directory "always use current" would remember.
    pub remembered_current: String,
    pub allow_remember_current: bool,
}

impl CwdPrompt {
    /// The rows in order; the last is absent where remembering current is not
    /// allowed (the oracle drops it and `4` does nothing).
    pub fn choices(&self) -> Vec<CwdSelection> {
        let mut rows = vec![
            CwdSelection::Session,
            CwdSelection::Current,
            CwdSelection::SessionAndRemember,
        ];
        if self.allow_remember_current {
            rows.push(CwdSelection::CurrentAndRemember);
        }
        rows
    }

    /// Whether the last row carries the directory it remembers: only when that
    /// is not the current one.
    pub fn shows_remembered_current(&self) -> bool {
        self.remembered_current != self.current
    }
}

/// The model-migration prompt (oracle `model_migration.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct MigrationPrompt {
    pub current_model: String,
    pub target_model: String,
    pub target_display_name: String,
    #[serde(default)]
    pub target_description: Option<String>,
    /// The server's own copy; it replaces the description and the line that
    /// recommends the switch.
    #[serde(default)]
    pub copy: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    /// A template with `{model_from}` and `{model_to}`; it replaces the copy.
    #[serde(default)]
    pub markdown: Option<String>,
    pub can_opt_out: bool,
    /// The target's default reasoning effort, saved with it on accept.
    #[serde(default)]
    pub effort: Option<String>,
}

impl MigrationPrompt {
    /// The markdown with the model names filled in (oracle
    /// `fill_migration_markdown`).
    pub fn filled_markdown(&self) -> Option<String> {
        self.markdown.as_ref().map(|template| {
            template
                .replace("{model_from}", &self.current_model)
                .replace("{model_to}", &self.target_model)
        })
    }
}

/// What the migration prompt returns (oracle `ModelMigrationOutcome`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationOutcome {
    Accepted,
    Rejected,
    Exit,
}

/// Where a trust of the hooks stands.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    #[default]
    Idle,
    /// Writing; the request id is what the result must carry.
    Trusting(u64),
    /// The last write failed with this detail.
    Failed(String),
}

/// The startup hooks review (oracle `startup_hooks_review.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct HooksPrompt {
    /// Hooks that are new or changed.
    pub count: usize,
    #[serde(default)]
    pub trust: Trust,
    /// The hooks "Trust all" trusts, at their hashes (`HookTrustUpdate`).
    #[serde(default)]
    pub updates: Vec<HookTrust>,
}

/// One hook to trust at its current hash (`HookTrustUpdate`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct HookTrust {
    pub key: String,
    pub hash: String,
}

/// What the hooks review returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HooksChoice {
    Review,
    TrustAll,
    Skip,
}

/// The archived-conversation prompt (oracle `unarchive_prompt.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct UnarchivePrompt {
    pub thread: String,
    pub action: Action,
}

/// What the unarchive prompt returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnarchiveChoice {
    Unarchive,
    Cancel,
    Quit,
}

/// The state of a local provider's server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    NotRunning,
    Unknown,
}

/// The open-source providers the picker offers, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    LmStudio,
    Ollama,
}

/// The open-source provider picker (oracle `oss_selection.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct OssPrompt {
    pub lmstudio: Status,
    pub ollama: Status,
}

/// The background-server recovery menu (oracle `daemon_recovery.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct DaemonPrompt {
    pub reason: String,
    /// The feature settings a restart would use; None where it cannot help.
    #[serde(default)]
    pub restart_features: Option<Vec<(String, bool)>>,
    pub managed: bool,
}

impl DaemonPrompt {
    /// Whether the restart row can be taken.
    pub fn can_restart(&self) -> bool {
        self.managed && self.restart_features.is_some()
    }
}

/// What the recovery menu returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonChoice {
    RunWithout,
    Restart,
    Cancel,
}

/// One startup prompt with its data.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "prompt", rename_all = "snake_case")]
pub enum Prompt {
    /// The GovCloud guidance (`onboarding/bedrock.rs`, shown once before the
    /// hooks review): no choice rows, Enter acknowledges.
    GovCloud,
    Cwd(CwdPrompt),
    Migration(MigrationPrompt),
    Hooks(HooksPrompt),
    Unarchive(UnarchivePrompt),
    Oss(OssPrompt),
    Daemon(DaemonPrompt),
}

/// What a prompt returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The GovCloud guidance was acknowledged.
    GovCloud,
    Cwd(CwdSelection),
    /// Ctrl-C on the cwd prompt (the oracle's `CwdPromptOutcome::Exit`).
    CwdExit,
    Migration(MigrationOutcome),
    Hooks(HooksChoice),
    Unarchive(UnarchiveChoice),
    Oss(Provider),
    /// Ctrl-C on the provider picker (the oracle's `__CANCELLED__`).
    OssCancelled,
    Daemon(DaemonChoice),
    /// Ctrl-C / Ctrl-D on the recovery menu.
    DaemonQuit,
}

impl Prompt {
    /// Whether `answer` is one this prompt can return.
    fn takes(&self, answer: Answer) -> bool {
        matches!(
            (self, answer),
            (Prompt::GovCloud, Answer::GovCloud)
                | (Prompt::Cwd(_), Answer::Cwd(_) | Answer::CwdExit)
                | (Prompt::Migration(_), Answer::Migration(_))
                | (Prompt::Hooks(_), Answer::Hooks(_))
                | (Prompt::Unarchive(_), Answer::Unarchive(_))
                | (Prompt::Oss(_), Answer::Oss(_) | Answer::OssCancelled)
                | (Prompt::Daemon(_), Answer::Daemon(_) | Answer::DaemonQuit)
        )
    }
}

/// A row of a prompt's choice list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Shown without a number, never the cursor's, never taken.
    pub disabled: bool,
    /// A number key moves the cursor here but does not take the row (it needs
    /// Enter: a terminal's reply must not grant trust).
    pub explicit: bool,
}

const PLAIN: Row = Row {
    disabled: false,
    explicit: false,
};

/// What taking a row, a digit or a step does to a prompt: the view's key
/// bindings (`pages/startup/prompts/keys.rs`) choose which one a key is, the
/// prompt answers what it comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    Nothing,
    /// The cursor moves to this row.
    Cursor(usize),
    /// The prompt returns this answer.
    Answer(Answer),
    /// "Trust all and continue" was taken: the write starts.
    TrustAll,
}

impl Prompt {
    /// The choice rows, in order.
    pub fn rows(&self) -> Vec<Row> {
        match self {
            Prompt::GovCloud => Vec::new(),
            Prompt::Cwd(p) => vec![PLAIN; p.choices().len()],
            Prompt::Migration(p) if p.can_opt_out => vec![PLAIN; 2],
            Prompt::Migration(_) => Vec::new(),
            Prompt::Hooks(p) => {
                let off = matches!(p.trust, Trust::Trusting(_));
                let row = Row {
                    disabled: off,
                    explicit: false,
                };
                vec![
                    row,
                    Row {
                        explicit: true,
                        ..row
                    },
                    row,
                ]
            }
            Prompt::Unarchive(_) | Prompt::Oss(_) => vec![PLAIN; 2],
            Prompt::Daemon(p) => vec![
                PLAIN,
                Row {
                    disabled: !p.can_restart(),
                    explicit: true,
                },
                PLAIN,
            ],
        }
    }

    /// The row the cursor starts on (the recovery menu starts on Cancel).
    pub fn initial_cursor(&self) -> usize {
        match self {
            Prompt::Daemon(_) => 2,
            _ => 0,
        }
    }

    /// What taking `row` returns (Enter on the cursor row, a hotkey).
    pub fn take(&self, row: usize) -> Reaction {
        let answer = match self {
            Prompt::GovCloud => None,
            Prompt::Cwd(p) => p.choices().get(row).copied().map(Answer::Cwd),
            Prompt::Migration(_) => match row {
                0 => Some(Answer::Migration(MigrationOutcome::Accepted)),
                1 => Some(Answer::Migration(MigrationOutcome::Rejected)),
                _ => None,
            },
            Prompt::Hooks(_) => match row {
                0 => Some(Answer::Hooks(HooksChoice::Review)),
                1 => return Reaction::TrustAll,
                2 => Some(Answer::Hooks(HooksChoice::Skip)),
                _ => None,
            },
            Prompt::Unarchive(_) => match row {
                0 => Some(Answer::Unarchive(UnarchiveChoice::Unarchive)),
                1 => Some(Answer::Unarchive(UnarchiveChoice::Cancel)),
                _ => None,
            },
            Prompt::Oss(_) => match row {
                0 => Some(Answer::Oss(Provider::LmStudio)),
                1 => Some(Answer::Oss(Provider::Ollama)),
                _ => None,
            },
            Prompt::Daemon(_) => match row {
                0 => Some(Answer::Daemon(DaemonChoice::RunWithout)),
                1 => Some(Answer::Daemon(DaemonChoice::Restart)),
                2 => Some(Answer::Daemon(DaemonChoice::Cancel)),
                _ => None,
            },
        };
        answer.map_or(Reaction::Nothing, Reaction::Answer)
    }

    /// The cursor one row on (`delta` 1) or back, round the ends, over the
    /// rows that are not disabled.
    pub fn step(&self, cursor: usize, delta: i64) -> Reaction {
        let rows = self.rows();
        let len = i64::try_from(rows.len()).unwrap_or(0);
        let mut at = i64::try_from(cursor).unwrap_or(0);
        for _ in 0..rows.len() {
            at = (at + delta).rem_euclid(len);
            let row = usize::try_from(at).unwrap_or(0);
            if !rows[row].disabled {
                return Reaction::Cursor(row);
            }
        }
        Reaction::Nothing
    }

    /// A number key: the n-th row that is not disabled (an explicit row only
    /// takes the cursor, it needs Enter).
    pub fn number(&self, n: usize) -> Reaction {
        let rows = self.rows();
        let Some(row) = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| !row.disabled)
            .nth(n.saturating_sub(1))
            .map(|(at, _)| at)
        else {
            return Reaction::Nothing;
        };
        if rows[row].explicit {
            return Reaction::Cursor(row);
        }
        self.take(row)
    }
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The prompt the flow asked and awaits the answer of.
    pub asked: Option<Prompt>,
    /// What the prompts returned, oldest first; the flow that asked reads its
    /// answer here and the host acts on it (a config write, a restart).
    pub answers: Vec<Answer>,
    /// The model an accepted migration prompt chose: new threads start on it
    /// (oracle: `config.model` is set before the session starts).
    accepted_model: Option<String>,
    /// What the prompts left for the history (the oracle's
    /// `insert_history_lines`), oldest first; the chat shows them.
    pub history: Vec<HistoryLine>,
    /// The write of the cwd choice awaited: an answer under another number
    /// is stale.
    saving_cwd: Option<u64>,
}

/// A line a prompt leaves in the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryLine {
    /// The cwd prompt's choice could not be written to the config.
    CwdPreferenceNotSaved,
}

impl Model {
    pub(crate) fn waits_for(&self, request: u64) -> bool {
        self.saving_cwd == Some(request)
            || matches!(&self.asked, Some(Prompt::Hooks(hooks)) if hooks.trust == Trust::Trusting(request))
    }

    pub fn accepted_model(&self) -> Option<&str> {
        self.accepted_model.as_deref()
    }
}

pub(crate) fn connection_lost(model: &mut Model, connector: bool) {
    if let Some(Prompt::Hooks(hooks)) = &mut model.asked
        && matches!(hooks.trust, Trust::Trusting(_))
    {
        hooks.trust = Trust::Failed(crate::app::CONNECTION_LOST.to_owned());
    }
    if connector && model.saving_cwd.take().is_some() {
        model.history.push(HistoryLine::CwdPreferenceNotSaved);
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A prompt was put on screen.
    Asked(Prompt),
    /// The hooks trust write with this request id finished (`None`: it
    /// worked, `Some`: the failure's detail). A result of an older request is
    /// dropped.
    HooksTrusted {
        request: u64,
        failure: Option<String>,
    },
    /// The write of the cwd choice with this request id finished (`true`: it
    /// failed). A result of an older request is dropped.
    CwdSaved { request: u64, failed: bool },
    /// A line for the history (a script's `{"history": ..}`).
    History(HistoryLine),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The flow asks for a prompt.
    Ask(Prompt),
    /// The prompt on screen returned an answer: it closes.
    Answer(Answer),
    /// "Trust all and continue" was confirmed: the write starts.
    TrustHooks,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Asked(prompt) => model.asked = Some(prompt),
        Ev::CwdSaved { request, failed } => {
            if model.saving_cwd != Some(request) {
                return;
            }
            model.saving_cwd = None;
            if failed {
                model.history.push(HistoryLine::CwdPreferenceNotSaved);
            }
        }
        Ev::History(line) => model.history.push(line),
        Ev::HooksTrusted { request, failure } => {
            let Some(Prompt::Hooks(hooks)) = &mut model.asked else {
                return;
            };
            if hooks.trust != Trust::Trusting(request) {
                return;
            }
            match failure {
                None => {
                    model.asked = None;
                    model.answers.push(Answer::Hooks(HooksChoice::TrustAll));
                }
                Some(detail) => hooks.trust = Trust::Failed(detail),
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    match intent {
        Intent::Ask(prompt) => model.asked = Some(prompt),
        Intent::Answer(answer) => {
            let Some(prompt) = &model.asked else { return };
            if !prompt.takes(answer) {
                return;
            }
            // While the trust write runs nothing else is taken (the oracle
            // disables the rows).
            if matches!(prompt, Prompt::Hooks(h) if matches!(h.trust, Trust::Trusting(_))) {
                return;
            }
            // The acknowledgment is saved (`notice.hide_gov_cloud_guidance`).
            if answer == Answer::GovCloud {
                fx.host_command(AppCommand::Startup {
                    ask: onboarding::Ask::AcknowledgeGovCloud,
                });
            }
            // The recovery menu's answer is the host's to carry out (Ctrl-C is Cancel).
            let daemon = match answer {
                Answer::Daemon(choice) => Some(choice),
                Answer::DaemonQuit => Some(DaemonChoice::Cancel),
                _ => None,
            };
            if let Some(choice) = daemon {
                fx.host_command(AppCommand::Startup {
                    ask: onboarding::Ask::DaemonChoice { choice },
                });
            }
            // The provider panel is answered before the config exists: the launch task takes
            // the pick (Ctrl-C is no provider, which ends the launch).
            let oss = match answer {
                Answer::Oss(provider) => Some(Some(provider)),
                Answer::OssCancelled => Some(None),
                _ => None,
            };
            if let Some(provider) = oss {
                fx.host_command(AppCommand::Startup {
                    ask: onboarding::Ask::OssChoice { provider },
                });
            }
            // The migration's answer is saved (`PersistModelMigrationPromptAcknowledged`,
            // accepted: `PersistModelSelection` too); an exit saves nothing.
            if let (Prompt::Migration(m), Answer::Migration(outcome)) = (prompt, answer)
                && outcome != MigrationOutcome::Exit
            {
                let accepted = outcome == MigrationOutcome::Accepted;
                if accepted {
                    model.accepted_model = Some(m.target_model.clone());
                }
                fx.host_command(AppCommand::Startup {
                    ask: onboarding::Ask::SaveMigration {
                        from: m.current_model.clone(),
                        to: m.target_model.clone(),
                        accepted,
                        effort: m.effort.clone().filter(|_| accepted),
                    },
                });
            }
            // A remembered choice is written to the config
            // (`persist_remembered_cwd_selection`); the answer says whether it
            // was.
            if let Answer::Cwd(selection) = answer
                && let Some(mode) = cwd::remembered(selection)
            {
                let request = fx.host_request_number();
                model.saving_cwd = Some(request);
                fx.host_command(AppCommand::Startup {
                    ask: onboarding::Ask::SaveResumeCwd { request, mode },
                });
            }
            model.asked = None;
            model.answers.push(answer);
        }
        Intent::TrustHooks => {
            let Some(Prompt::Hooks(hooks)) = &mut model.asked else {
                return;
            };
            if matches!(hooks.trust, Trust::Trusting(_)) {
                return;
            }
            let request = fx.host_request_number();
            hooks.trust = Trust::Trusting(request);
            fx.host_command(AppCommand::Startup {
                ask: onboarding::Ask::TrustHooks {
                    request,
                    updates: hooks.updates.clone(),
                },
            });
        }
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// A script's `screen` step for this directory: `{"prompt": "cwd", ..}` (the
/// prompt's fields) puts that prompt on screen; `{"history": "cwd_preference_not_saved"}`
/// leaves that line for the history.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    if let Some(line) = value.get("history") {
        let line: HistoryLine = serde_json::from_value(line.clone())
            .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
        return Ok(Ev::History(line));
    }
    let prompt: Prompt = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(Ev::Asked(prompt))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hooks(trust: Trust) -> Model {
        Model {
            asked: Some(Prompt::Hooks(HooksPrompt {
                count: 2,
                trust,
                updates: vec![HookTrust {
                    key: "k".into(),
                    hash: "h".into(),
                }],
            })),
            ..Model::default()
        }
    }

    fn fx_apply(model: &mut Model, intent: Intent) {
        let mut fx = AppFx::default();
        apply_intent(model, intent, &mut fx);
    }

    fn fx_ev(model: &mut Model, ev: Ev) {
        let mut fx = AppFx::default();
        apply(model, ev, &mut fx);
    }

    #[test]
    fn a_stale_trust_result_is_dropped() {
        let mut model = hooks(Trust::Idle);
        fx_apply(&mut model, Intent::TrustHooks);
        assert!(
            matches!(&model.asked, Some(Prompt::Hooks(hooks)) if hooks.trust == Trust::Trusting(1))
        );
        fx_ev(
            &mut model,
            Ev::HooksTrusted {
                request: 7,
                failure: None,
            },
        );
        assert!(model.asked.is_some() && model.answers.is_empty());
        fx_ev(
            &mut model,
            Ev::HooksTrusted {
                request: 1,
                failure: Some("boom".into()),
            },
        );
        assert_eq!(
            model.asked,
            Some(Prompt::Hooks(HooksPrompt {
                count: 2,
                trust: Trust::Failed("boom".into()),
                updates: vec![HookTrust {
                    key: "k".into(),
                    hash: "h".into()
                }],
            }))
        );
    }

    #[test]
    fn a_trust_that_worked_closes_with_trust_all() {
        let mut model = hooks(Trust::Idle);
        fx_apply(&mut model, Intent::TrustHooks);
        fx_ev(
            &mut model,
            Ev::HooksTrusted {
                request: 1,
                failure: None,
            },
        );
        assert_eq!(model.asked, None);
        assert_eq!(model.answers, vec![Answer::Hooks(HooksChoice::TrustAll)]);
    }

    #[test]
    fn nothing_is_taken_while_trusting() {
        let mut model = hooks(Trust::Idle);
        fx_apply(&mut model, Intent::TrustHooks);
        fx_apply(&mut model, Intent::TrustHooks);
        assert!(
            matches!(&model.asked, Some(Prompt::Hooks(hooks)) if hooks.trust == Trust::Trusting(1))
        );
        fx_apply(&mut model, Intent::Answer(Answer::Hooks(HooksChoice::Skip)));
        assert!(model.asked.is_some() && model.answers.is_empty());
    }

    fn asks_of(model: &mut Model, intent: Intent) -> Vec<onboarding::Ask> {
        let mut fx = AppFx::default();
        apply_intent(model, intent, &mut fx);
        fx.into_commands()
            .into_iter()
            .filter_map(|c| match c {
                AppCommand::Startup { ask } => Some(ask),
                _ => None,
            })
            .collect()
    }

    // write_hook_trusts: "Trust all and continue" writes every hook of the
    // review at its current hash.
    #[test]
    fn trusting_the_hooks_asks_for_their_trust_write() {
        let mut model = hooks(Trust::Idle);
        assert_eq!(
            asks_of(&mut model, Intent::TrustHooks),
            [onboarding::Ask::TrustHooks {
                request: 1,
                updates: vec![HookTrust {
                    key: "k".into(),
                    hash: "h".into(),
                }],
            }]
        );
    }

    fn migration() -> Model {
        Model {
            asked: Some(Prompt::Migration(MigrationPrompt {
                current_model: "gpt-old".into(),
                target_model: "gpt-new".into(),
                target_display_name: "GPT New".into(),
                target_description: None,
                copy: None,
                link: None,
                markdown: None,
                can_opt_out: true,
                effort: Some("high".into()),
            })),
            ..Model::default()
        }
    }

    // handle_model_migration_prompt_if_needed: an accepted or rejected
    // migration is recorded as seen, an accepted one also switches the
    // model (with the target's default effort); an exit writes nothing.
    #[test]
    fn a_migration_answer_asks_for_its_config_write() {
        let save = |accepted, effort: Option<&str>| onboarding::Ask::SaveMigration {
            from: "gpt-old".into(),
            to: "gpt-new".into(),
            accepted,
            effort: effort.map(str::to_owned),
        };
        let answer = |o| Intent::Answer(Answer::Migration(o));
        assert_eq!(
            asks_of(&mut migration(), answer(MigrationOutcome::Accepted)),
            [save(true, Some("high"))]
        );
        assert_eq!(
            asks_of(&mut migration(), answer(MigrationOutcome::Rejected)),
            [save(false, None)]
        );
        assert_eq!(
            asks_of(&mut migration(), answer(MigrationOutcome::Exit)),
            []
        );
    }

    fn daemon_menu(managed: bool) -> Model {
        Model {
            asked: Some(Prompt::Daemon(DaemonPrompt {
                reason: "r".into(),
                restart_features: Some(vec![("a".into(), true)]),
                managed,
            })),
            ..Model::default()
        }
    }

    // daemon_recovery.rs: each answer of the recovery menu is the host's to carry out; Ctrl-C
    // is the same as Cancel (the launch ends with the compatibility error).
    #[test]
    fn a_daemon_answer_asks_the_host_to_carry_it_out() {
        let ask = |choice| [onboarding::Ask::DaemonChoice { choice }];
        for (answer, choice) in [
            (Answer::Daemon(DaemonChoice::Restart), DaemonChoice::Restart),
            (
                Answer::Daemon(DaemonChoice::RunWithout),
                DaemonChoice::RunWithout,
            ),
            (Answer::Daemon(DaemonChoice::Cancel), DaemonChoice::Cancel),
            (Answer::DaemonQuit, DaemonChoice::Cancel),
        ] {
            let mut model = daemon_menu(true);
            assert_eq!(asks_of(&mut model, Intent::Answer(answer)), ask(choice));
            assert_eq!(model.answers, vec![answer]);
        }
    }

    fn oss_panel() -> Model {
        Model {
            asked: Some(Prompt::Oss(OssPrompt {
                lmstudio: Status::NotRunning,
                ollama: Status::NotRunning,
            })),
            ..Model::default()
        }
    }

    // oss_selection.rs: the panel is answered before the config exists; the launch task takes
    // the pick (`Ask::OssChoice`).
    #[test]
    fn answering_the_provider_panel_asks_the_host() {
        for provider in [Provider::LmStudio, Provider::Ollama] {
            let mut model = oss_panel();
            assert_eq!(
                asks_of(&mut model, Intent::Answer(Answer::Oss(provider))),
                [onboarding::Ask::OssChoice {
                    provider: Some(provider)
                }]
            );
            assert_eq!(model.answers, vec![Answer::Oss(provider)]);
        }
    }

    // Ctrl-C on the panel is `__CANCELLED__`: no provider, the launch ends.
    #[test]
    fn cancelling_the_provider_panel_asks_the_host_with_none() {
        let mut model = oss_panel();
        assert_eq!(
            asks_of(&mut model, Intent::Answer(Answer::OssCancelled)),
            [onboarding::Ask::OssChoice { provider: None }]
        );
    }

    // A menu that is not asked takes no answer; the disabled restart row has no answer.
    #[test]
    fn the_disabled_restart_row_is_never_answered() {
        let unmanaged = daemon_menu(false).asked.unwrap();
        // The cursor skips it, and no number reaches it: `2` is the row after it.
        assert_eq!(unmanaged.step(2, -1), Reaction::Cursor(0));
        assert_eq!(
            unmanaged.number(1),
            Reaction::Answer(Answer::Daemon(DaemonChoice::RunWithout))
        );
        assert_eq!(
            unmanaged.number(2),
            Reaction::Answer(Answer::Daemon(DaemonChoice::Cancel))
        );
        let mut model = Model::default();
        assert_eq!(
            asks_of(
                &mut model,
                Intent::Answer(Answer::Daemon(DaemonChoice::Cancel))
            ),
            []
        );
    }

    // run_ratatui_app: the GovCloud guidance's acknowledgment is written
    // once, when it is dismissed (`acknowledge_gov_cloud`).
    #[test]
    fn the_gov_cloud_acknowledgment_asks_for_its_write() {
        let mut model = Model {
            asked: Some(Prompt::GovCloud),
            ..Model::default()
        };
        assert_eq!(
            asks_of(&mut model, Intent::Answer(Answer::Hooks(HooksChoice::Skip))),
            []
        );
        assert_eq!(
            asks_of(&mut model, Intent::Answer(Answer::GovCloud)),
            [onboarding::Ask::AcknowledgeGovCloud]
        );
        assert_eq!(model.asked, None);
        assert_eq!(model.answers, vec![Answer::GovCloud]);
    }

    #[test]
    fn an_answer_of_another_prompt_is_ignored() {
        let mut model = hooks(Trust::Idle);
        fx_apply(
            &mut model,
            Intent::Answer(Answer::Unarchive(UnarchiveChoice::Cancel)),
        );
        assert!(model.asked.is_some() && model.answers.is_empty());
        fx_apply(
            &mut model,
            Intent::Answer(Answer::Hooks(HooksChoice::Review)),
        );
        assert_eq!(model.asked, None);
        assert_eq!(model.answers, vec![Answer::Hooks(HooksChoice::Review)]);
    }

    #[test]
    fn the_cwd_prompt_drops_the_remembered_current_row_when_not_allowed() {
        let prompt = CwdPrompt {
            action: Action::Resume,
            current: "/a".into(),
            session: "/b".into(),
            remembered_current: "/a".into(),
            allow_remember_current: false,
        };
        assert_eq!(prompt.choices().len(), 3);
        assert!(!prompt.choices().contains(&CwdSelection::CurrentAndRemember));
    }

    #[test]
    fn the_migration_markdown_fills_both_model_names() {
        let prompt = MigrationPrompt {
            current_model: "gpt-old".into(),
            target_model: "gpt-new".into(),
            target_display_name: "GPT New".into(),
            target_description: None,
            copy: None,
            link: None,
            markdown: Some("# {model_to} from {model_from}".into()),
            can_opt_out: true,
            effort: None,
        };
        assert_eq!(
            prompt.filled_markdown().as_deref(),
            Some("# gpt-new from gpt-old")
        );
    }

    #[test]
    fn a_script_step_puts_the_prompt_on_screen() {
        let ev = seed(serde_json::json!({"prompt": "hooks", "count": 3})).unwrap();
        assert_eq!(
            ev,
            Ev::Asked(Prompt::Hooks(HooksPrompt {
                count: 3,
                trust: Trust::Idle,
                updates: Vec::new(),
            }))
        );
    }

    fn cwd(allow: bool) -> Prompt {
        Prompt::Cwd(CwdPrompt {
            action: Action::Resume,
            current: "/Users/example/current".into(),
            session: "/Users/example/session".into(),
            remembered_current: "/Users/example/launched".into(),
            allow_remember_current: allow,
        })
    }

    #[test]
    fn step_wraps_round_the_ends_over_the_rows_that_are_not_disabled() {
        let oss = Prompt::Oss(OssPrompt {
            lmstudio: Status::Unknown,
            ollama: Status::Unknown,
        });
        assert_eq!(oss.step(0, -1), Reaction::Cursor(1));
        assert_eq!(oss.step(1, 1), Reaction::Cursor(0));
        let daemon = Prompt::Daemon(DaemonPrompt {
            reason: "r".into(),
            restart_features: None,
            managed: true,
        });
        // The disabled middle row is skipped.
        assert_eq!(daemon.step(2, -1), Reaction::Cursor(0));
        assert_eq!(daemon.step(0, 1), Reaction::Cursor(2));
        let trusting = Prompt::Hooks(HooksPrompt {
            count: 1,
            trust: Trust::Trusting(1),
            updates: Vec::new(),
        });
        assert_eq!(trusting.step(0, 1), Reaction::Nothing);
    }

    #[test]
    fn a_number_takes_a_plain_row_and_only_moves_to_an_explicit_one() {
        let hooks = hooks(Trust::Idle).asked.unwrap();
        assert_eq!(
            hooks.number(1),
            Reaction::Answer(Answer::Hooks(HooksChoice::Review))
        );
        assert_eq!(hooks.number(2), Reaction::Cursor(1));
        assert_eq!(hooks.number(4), Reaction::Nothing);
        assert_eq!(hooks.take(1), Reaction::TrustAll);
    }

    #[test]
    fn the_cwd_prompt_has_no_fourth_row_without_the_remembered_current() {
        assert_eq!(cwd(false).number(4), Reaction::Nothing);
        assert_eq!(
            cwd(true).number(4),
            Reaction::Answer(Answer::Cwd(CwdSelection::CurrentAndRemember))
        );
        assert!(CwdSelection::CurrentAndRemember.remembers());
    }
}
