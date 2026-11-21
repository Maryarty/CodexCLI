//! `startup` (S05, startup prompts): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/startup/prompts/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

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
            (Prompt::Cwd(_), Answer::Cwd(_) | Answer::CwdExit)
                | (Prompt::Migration(_), Answer::Migration(_))
                | (Prompt::Hooks(_), Answer::Hooks(_))
                | (Prompt::Unarchive(_), Answer::Unarchive(_))
                | (Prompt::Oss(_), Answer::Oss(_) | Answer::OssCancelled)
                | (Prompt::Daemon(_), Answer::Daemon(_) | Answer::DaemonQuit)
        )
    }
}

/// A key a prompt reads (the view maps rtwk's key presses onto these).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Ctrl(char),
    Char(char),
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

/// What a key does to a prompt.
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

    /// What taking `row` returns.
    fn take(&self, row: usize) -> Reaction {
        let answer = match self {
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
    fn step(&self, cursor: usize, delta: i64) -> Reaction {
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

    /// A number key: the n-th row that is not disabled.
    fn number(&self, cursor: usize, n: usize) -> Reaction {
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
        let _ = cursor;
        self.take(row)
    }

    /// What `key` does with the cursor on `cursor` (the oracle's `handle_key`
    /// of each prompt).
    pub fn on_key(&self, cursor: usize, key: Key) -> Reaction {
        let quit = matches!(key, Key::Ctrl('c' | 'd'));
        let digit = |c: char| c.to_digit(10).map(|d| d as usize);
        match self {
            Prompt::Cwd(_) => match key {
                _ if quit => Reaction::Answer(Answer::CwdExit),
                Key::Up | Key::Char('k') => self.step(cursor, -1),
                Key::Down | Key::Char('j') => self.step(cursor, 1),
                Key::Char(c @ '1'..='4') => self.number(cursor, digit(c).unwrap_or(0)),
                Key::Enter => self.take(cursor),
                Key::Esc => Reaction::Answer(Answer::Cwd(CwdSelection::Session)),
                _ => Reaction::Nothing,
            },
            Prompt::Migration(p) => match key {
                _ if quit => Reaction::Answer(Answer::Migration(MigrationOutcome::Exit)),
                Key::Enter | Key::Esc if !p.can_opt_out => {
                    Reaction::Answer(Answer::Migration(MigrationOutcome::Accepted))
                }
                _ if !p.can_opt_out => Reaction::Nothing,
                Key::Up | Key::Char('k') => Reaction::Cursor(0),
                Key::Down | Key::Char('j') => Reaction::Cursor(1),
                Key::Char(c @ ('1' | '2')) => self.number(cursor, digit(c).unwrap_or(0)),
                Key::Enter | Key::Esc => self.take(cursor),
                _ => Reaction::Nothing,
            },
            Prompt::Hooks(_) => {
                if self.rows().iter().all(|row| row.disabled) {
                    return Reaction::Nothing;
                }
                match key {
                    Key::Up | Key::Char('k') => self.step(cursor, -1),
                    Key::Down | Key::Char('j') => self.step(cursor, 1),
                    Key::Char(c @ '1'..='9') => self.number(cursor, digit(c).unwrap_or(0)),
                    Key::Enter => self.take(cursor),
                    Key::Esc => Reaction::Answer(Answer::Hooks(HooksChoice::Skip)),
                    _ => Reaction::Nothing,
                }
            }
            Prompt::Unarchive(_) => match key {
                _ if quit => Reaction::Answer(Answer::Unarchive(UnarchiveChoice::Quit)),
                Key::Up | Key::Down | Key::Char('k' | 'j') => self.step(cursor, 1),
                Key::Char('1' | 'y' | 'Y') => self.take(0),
                Key::Esc | Key::Char('2' | 'n' | 'N') => self.take(1),
                Key::Enter => self.take(cursor),
                _ => Reaction::Nothing,
            },
            Prompt::Oss(_) => match key {
                Key::Ctrl('c') => Reaction::Answer(Answer::OssCancelled),
                Key::Left | Key::Up | Key::Ctrl('h') | Key::Char('k') => self.step(cursor, -1),
                Key::Right | Key::Down | Key::Ctrl('l') | Key::Char('j') => self.step(cursor, 1),
                Key::Enter => self.take(cursor),
                Key::Esc => self.take(0),
                Key::Char('1') => self.take(0),
                Key::Char('2') => self.take(1),
                Key::Char(c) => match c.to_ascii_lowercase() {
                    'l' => self.take(0),
                    'o' => self.take(1),
                    _ => Reaction::Nothing,
                },
                _ => Reaction::Nothing,
            },
            Prompt::Daemon(_) => match key {
                _ if quit => Reaction::Answer(Answer::DaemonQuit),
                Key::Up | Key::Char('k') => self.step(cursor, -1),
                Key::Down | Key::Char('j') => self.step(cursor, 1),
                Key::Char(c @ '1'..='9') => self.number(cursor, digit(c).unwrap_or(0)),
                Key::Enter => self.take(cursor),
                Key::Esc => self.take(2),
                _ => Reaction::Nothing,
            },
        }
    }
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The prompt on screen.
    pub open: Option<Prompt>,
    /// What the prompts returned, oldest first; the flow that asked reads its
    /// answer here and the host acts on it (a config write, a restart).
    pub answers: Vec<Answer>,
    /// The next id of a hooks trust request.
    pub next_request: u64,
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
        Ev::Asked(prompt) => model.open = Some(prompt),
        Ev::HooksTrusted { request, failure } => {
            let Some(Prompt::Hooks(hooks)) = &mut model.open else {
                return;
            };
            if hooks.trust != Trust::Trusting(request) {
                return;
            }
            match failure {
                None => {
                    model.open = None;
                    model.answers.push(Answer::Hooks(HooksChoice::TrustAll));
                }
                Some(detail) => hooks.trust = Trust::Failed(detail),
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Ask(prompt) => model.open = Some(prompt),
        Intent::Answer(answer) => {
            let Some(prompt) = &model.open else { return };
            if !prompt.takes(answer) {
                return;
            }
            // While the trust write runs nothing else is taken (the oracle
            // disables the rows).
            if matches!(prompt, Prompt::Hooks(h) if matches!(h.trust, Trust::Trusting(_))) {
                return;
            }
            model.open = None;
            model.answers.push(answer);
        }
        Intent::TrustHooks => {
            let Some(Prompt::Hooks(hooks)) = &mut model.open else {
                return;
            };
            if matches!(hooks.trust, Trust::Trusting(_)) {
                return;
            }
            model.next_request += 1;
            hooks.trust = Trust::Trusting(model.next_request);
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
/// prompt's fields) puts that prompt on screen.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let prompt: Prompt = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(Ev::Asked(prompt))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hooks(trust: Trust) -> Model {
        Model {
            open: Some(Prompt::Hooks(HooksPrompt { count: 2, trust })),
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
        assert_eq!(model.next_request, 1);
        fx_ev(
            &mut model,
            Ev::HooksTrusted {
                request: 7,
                failure: None,
            },
        );
        assert!(model.open.is_some() && model.answers.is_empty());
        fx_ev(
            &mut model,
            Ev::HooksTrusted {
                request: 1,
                failure: Some("boom".into()),
            },
        );
        assert_eq!(
            model.open,
            Some(Prompt::Hooks(HooksPrompt {
                count: 2,
                trust: Trust::Failed("boom".into())
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
        assert_eq!(model.open, None);
        assert_eq!(model.answers, vec![Answer::Hooks(HooksChoice::TrustAll)]);
    }

    #[test]
    fn nothing_is_taken_while_trusting() {
        let mut model = hooks(Trust::Idle);
        fx_apply(&mut model, Intent::TrustHooks);
        fx_apply(&mut model, Intent::TrustHooks);
        assert_eq!(model.next_request, 1);
        fx_apply(&mut model, Intent::Answer(Answer::Hooks(HooksChoice::Skip)));
        assert!(model.open.is_some() && model.answers.is_empty());
    }

    #[test]
    fn an_answer_of_another_prompt_is_ignored() {
        let mut model = hooks(Trust::Idle);
        fx_apply(&mut model, Intent::Answer(Answer::Unarchive(UnarchiveChoice::Cancel)));
        assert!(model.open.is_some() && model.answers.is_empty());
        fx_apply(&mut model, Intent::Answer(Answer::Hooks(HooksChoice::Review)));
        assert_eq!(model.open, None);
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
                trust: Trust::Idle
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

    /// Plays keys the way a view does: the cursor follows `Cursor`, the first
    /// answer ends it.
    fn play(prompt: &Prompt, keys: &[Key]) -> (usize, Option<Reaction>) {
        let mut cursor = prompt.initial_cursor();
        for key in keys {
            match prompt.on_key(cursor, *key) {
                Reaction::Nothing => {}
                Reaction::Cursor(row) => cursor = row,
                other => return (cursor, Some(other)),
            }
        }
        (cursor, None)
    }

    fn answer(answer: Answer) -> Option<Reaction> {
        Some(Reaction::Answer(answer))
    }

    #[test]
    fn cwd_prompt_selects_session_by_default() {
        assert_eq!(
            play(&cwd(true), &[Key::Enter]).1,
            answer(Answer::Cwd(CwdSelection::Session))
        );
    }

    #[test]
    fn cwd_prompt_can_select_current() {
        assert_eq!(
            play(&cwd(true), &[Key::Down, Key::Enter]).1,
            answer(Answer::Cwd(CwdSelection::Current))
        );
    }

    #[test]
    fn cwd_prompt_omits_unusable_remembered_current_choice() {
        let prompt = cwd(false);
        assert_eq!(play(&prompt, &[Key::Char('4')]), (0, None));
        assert_eq!(play(&prompt, &[Key::Up]), (2, None));
    }

    #[test]
    fn cwd_prompt_ctrl_c_exits_instead_of_selecting() {
        assert_eq!(play(&cwd(true), &[Key::Ctrl('c')]).1, answer(Answer::CwdExit));
    }

    #[test]
    fn cwd_prompt_remembered_choices_select_matching_directory() {
        assert_eq!(
            play(&cwd(true), &[Key::Char('3')]).1,
            answer(Answer::Cwd(CwdSelection::SessionAndRemember))
        );
        assert_eq!(
            play(&cwd(true), &[Key::Char('4')]).1,
            answer(Answer::Cwd(CwdSelection::CurrentAndRemember))
        );
        assert!(CwdSelection::CurrentAndRemember.remembers());
    }

    #[test]
    fn narrow_picker_escape_uses_session() {
        assert_eq!(
            play(&cwd(true), &[Key::Up, Key::Esc]).1,
            answer(Answer::Cwd(CwdSelection::Session))
        );
        assert_eq!(play(&cwd(true), &[Key::Up]), (3, None));
    }

    fn migration(can_opt_out: bool) -> Prompt {
        Prompt::Migration(MigrationPrompt {
            current_model: "gpt-old".into(),
            target_model: "gpt-new".into(),
            target_display_name: "gpt-new".into(),
            target_description: None,
            copy: None,
            link: None,
            markdown: None,
            can_opt_out,
        })
    }

    #[test]
    fn escape_key_accepts_prompt() {
        assert_eq!(
            play(&migration(true), &[Key::Esc]).1,
            answer(Answer::Migration(MigrationOutcome::Accepted))
        );
        assert_eq!(
            play(&migration(false), &[Key::Esc]).1,
            answer(Answer::Migration(MigrationOutcome::Accepted))
        );
    }

    #[test]
    fn selecting_use_existing_model_rejects_upgrade() {
        assert_eq!(
            play(&migration(true), &[Key::Down, Key::Enter]).1,
            answer(Answer::Migration(MigrationOutcome::Rejected))
        );
        assert_eq!(
            play(&migration(true), &[Key::Ctrl('c')]).1,
            answer(Answer::Migration(MigrationOutcome::Exit))
        );
    }

    fn hooks_idle() -> Prompt {
        Prompt::Hooks(HooksPrompt {
            count: 2,
            trust: Trust::Idle,
        })
    }

    #[test]
    fn fragmented_terminal_response_cannot_grant_startup_hook_trust() {
        let prompt = hooks_idle();
        let mut cursor = 0;
        for c in "20;rgb:2222/ffff/ffff".chars() {
            let reaction = prompt.on_key(cursor, Key::Char(c));
            assert_ne!(reaction, Reaction::TrustAll);
            if let Reaction::Cursor(row) = reaction {
                cursor = row;
            }
        }
        assert_eq!(cursor, 1);
        assert_eq!(prompt.on_key(cursor, Key::Enter), Reaction::TrustAll);
    }

    #[test]
    fn non_trust_startup_hook_shortcuts_remain_immediate() {
        assert_eq!(
            play(&hooks_idle(), &[Key::Char('1')]).1,
            answer(Answer::Hooks(HooksChoice::Review))
        );
        assert_eq!(
            play(&hooks_idle(), &[Key::Char('3')]).1,
            answer(Answer::Hooks(HooksChoice::Skip))
        );
    }

    #[test]
    fn picker_progress_blocks_selection() {
        let prompt = Prompt::Hooks(HooksPrompt {
            count: 2,
            trust: Trust::Trusting(1),
        });
        assert!(prompt.rows().iter().all(|row| row.disabled));
        assert_eq!(play(&prompt, &[Key::Enter, Key::Char('1'), Key::Esc]), (0, None));
    }

    #[test]
    fn confirmation_requires_an_accept_key() {
        let prompt = Prompt::Unarchive(UnarchivePrompt {
            thread: "t".into(),
            action: Action::Fork,
        });
        let cancel = answer(Answer::Unarchive(UnarchiveChoice::Cancel));
        for key in [Key::Esc, Key::Char('n'), Key::Char('2')] {
            assert_eq!(prompt.on_key(0, key), cancel.unwrap());
        }
        for c in ['c', 'd'] {
            assert_eq!(
                prompt.on_key(0, Key::Ctrl(c)),
                Reaction::Answer(Answer::Unarchive(UnarchiveChoice::Quit))
            );
        }
        assert_eq!(
            play(&prompt, &[Key::Enter]).1,
            answer(Answer::Unarchive(UnarchiveChoice::Unarchive))
        );
        assert_eq!(
            play(&prompt, &[Key::Down, Key::Enter]).1,
            cancel
        );
    }

    fn oss() -> Prompt {
        Prompt::Oss(OssPrompt {
            lmstudio: Status::Unknown,
            ollama: Status::Unknown,
        })
    }

    #[test]
    fn ctrl_h_l_move_provider_selection() {
        assert_eq!(play(&oss(), &[Key::Ctrl('l')]), (1, None));
        assert_eq!(play(&oss(), &[Key::Ctrl('l'), Key::Ctrl('h')]), (0, None));
    }

    #[test]
    fn navigation_wraps_without_confirming() {
        assert_eq!(play(&oss(), &[Key::Up]), (1, None));
        assert_eq!(play(&oss(), &[Key::Down, Key::Down]), (0, None));
    }

    #[test]
    fn shortcuts_select_the_provider_even_when_unavailable() {
        for (key, provider) in [
            (Key::Char('l'), Provider::LmStudio),
            (Key::Char('O'), Provider::Ollama),
            (Key::Char('2'), Provider::Ollama),
            (Key::Esc, Provider::LmStudio),
        ] {
            assert_eq!(play(&oss(), &[key]).1, answer(Answer::Oss(provider)));
        }
        assert_eq!(
            play(&oss(), &[Key::Ctrl('c')]).1,
            answer(Answer::OssCancelled)
        );
    }

    fn daemon(managed: bool, restartable: bool) -> Prompt {
        Prompt::Daemon(DaemonPrompt {
            reason: "r".into(),
            restart_features: restartable.then(|| vec![("a".into(), true)]),
            managed,
        })
    }

    #[test]
    fn daemon_recovery_requires_explicit_restart_and_defaults_to_cancel() {
        for (managed, restartable) in [(true, true), (false, true), (true, false)] {
            let prompt = daemon(managed, restartable);
            assert_eq!(
                play(&prompt, &[Key::Enter]).1,
                answer(Answer::Daemon(DaemonChoice::Cancel))
            );
            if managed && restartable {
                assert_eq!(play(&prompt, &[Key::Char('2')]), (1, None));
                assert_eq!(
                    play(&prompt, &[Key::Char('2'), Key::Enter]).1,
                    answer(Answer::Daemon(DaemonChoice::Restart))
                );
            } else {
                // The disabled row has no number: `2` is Cancel.
                assert_eq!(
                    play(&prompt, &[Key::Char('2')]).1,
                    answer(Answer::Daemon(DaemonChoice::Cancel))
                );
                assert_eq!(play(&prompt, &[Key::Up]), (0, None));
            }
        }
    }
}
