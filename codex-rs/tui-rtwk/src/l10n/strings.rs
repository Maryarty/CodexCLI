//! The strings shared by the skeleton: the chat's frame (status line,
//! transcript marks, queue, composer placeholder), the presenter's request
//! dialog and the debug menu. They are the skeleton's own text, not the
//! oracle's: the slices that fill `chat` and `approvals` replace them with
//! the oracle's in their own `strings.rs`. Russian is a three-string proof
//! that the path works; every other field of it is English.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use super::Lang;
use super::Tables;
use super::lang;

/// The chat frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chat {
    pub placeholder: &'static str,
    pub starting: &'static str,
    pub working: &'static str,
    pub reconnecting: &'static str,
    pub thinking: &'static str,
    pub action_required: &'static str,
    pub compacting: &'static str,
    pub backtrack_hint: &'static str,
    pub queue_hint: &'static str,
    pub model_fast: Msg<2>,
    pub tasks_progress: Msg<2>,
    pub context_remaining: Msg<1>,
    pub context_used: Msg<1>,
    pub quota_remaining: Msg<2>,
    /// The credits a thread used, in the status line and the title (`thread-credits`).
    pub thread_credits: Msg<1>,
    pub usage_limit_title: &'static str,
    pub usage_limit_detail: &'static str,
    /// The backend stopped (`Failure::Stopped`).
    pub server_stopped: &'static str,
    /// A reply of another screen than its ticket's (`Failure::ReplyMismatch`).
    pub reply_mismatch: &'static str,
    /// The key hints under the composer.
    pub keys: &'static str,
    pub error: Msg<1>,
    pub lagged: Msg<1>,
    pub user_line: Msg<1>,
    pub agent_line: Msg<1>,
}

/// The request dialog the presenter shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presenter {
    pub title: &'static str,
    pub needs_answer: Msg<1>,
    pub cannot_answer: &'static str,
    pub dismiss: &'static str,
}

/// The debug menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Debug {
    pub title: &'static str,
    /// The hint of a menu that was pushed (Esc and q go back).
    pub hint: &'static str,
    /// The hint of the menu as the home (q quits).
    pub hint_home: &'static str,
    pub empty_slot: Msg<2>,
}

/// What the pages share for their keys (`ui/page_keys.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pages {
    /// The popup of a key whose action is not built yet.
    pub not_yet: Msg<1>,
    /// The name of the expanded transcript (Ctrl+T, `/diff`).
    pub action_transcript: &'static str,
    pub action_subagents: &'static str,
    /// The name of the shortcut overlay (`?`).
    pub action_shortcuts: &'static str,
    /// The key of the shortcut overlay, as the footer shows it, and its hint
    /// (oracle: `footer.rs`, `FooterMode::ShortcutSummary`).
    pub key_agents: &'static str,
    pub hint_agents: &'static str,
    pub key_shortcuts: &'static str,
    pub hint_shortcuts: &'static str,
}

/// One locale's shared table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shared {
    /// Digits and separators for the messages' numbers.
    pub num: NumSyms,
    pub chat: Chat,
    pub presenter: Presenter,
    pub debug: Debug,
    pub pages: Pages,
}

/// English. The skeleton's own text (no oracle string).
pub const EN: Shared = Shared {
    num: NumSyms::ENGLISH,
    chat: Chat {
        placeholder: "Ask Codex to do anything",
        starting: "starting…",
        working: "Working",
        reconnecting: "Reconnecting…",
        thinking: "Thinking",
        action_required: "[ ! ] Action Required",
        compacting: "Compacting context",
        backtrack_hint: "Esc again to edit the previous message",
        queue_hint: "tab to queue message",
        model_fast: msg!("{0} {1} fast"),
        tasks_progress: msg!("Tasks {0}/{1}"),
        context_remaining: msg!("Context {0}% left"),
        context_used: msg!("Context {0}% used"),
        quota_remaining: msg!("{0} {1}% left"),
        thread_credits: msg!("{0} credits"),
        usage_limit_title: "Usage limit reached",
        usage_limit_detail: "Your included usage is exhausted. Choose an option below to continue.",
        server_stopped: "error: the app-server stopped",
        reply_mismatch: "error: a reply of another screen",
        keys: "F12 screens  Ctrl+C quit",
        error: msg!("error: {0}"),
        lagged: msg!("error: the backend dropped {0} events"),
        user_line: msg!("› {0}"),
        agent_line: msg!("• {0}"),
    },
    presenter: Presenter {
        title: "Request",
        needs_answer: msg!("{0} needs an answer."),
        cannot_answer: "This build cannot answer it yet.",
        dismiss: "Dismiss",
    },
    debug: Debug {
        title: "codex-rtwk debug screens",
        hint: "up/down move, enter opens, esc/q back",
        hint_home: "up/down move, enter opens, q quits",
        empty_slot: msg!("{0} ({1}): empty slot"),
    },
    pages: Pages {
        not_yet: msg!("Not implemented yet: {0}"),
        action_transcript: "expanded transcript",
        action_subagents: "subagent picker",
        action_shortcuts: "shortcut overlay",
        key_agents: "←",
        hint_agents: "for agents",
        key_shortcuts: "?",
        hint_shortcuts: "for shortcuts",
    },
};

/// Russian: three strings, the rest English (a proof of the path).
pub const RU: Shared = Shared {
    chat: Chat {
        placeholder: "Спросите Codex о чём угодно",
        working: "работаю…",
        reconnecting: "Переподключение…",
        thinking: "размышляю…",
        action_required: "[ ! ] Требуется действие",
        compacting: "Сжатие контекста",
        backtrack_hint: "Нажмите Esc ещё раз, чтобы изменить предыдущее сообщение",
        queue_hint: "Tab — поставить сообщение в очередь",
        model_fast: msg!("{0} {1} быстро"),
        tasks_progress: msg!("Задачи {0}/{1}"),
        context_remaining: msg!("Осталось {0}% контекста"),
        context_used: msg!("Использовано {0}% контекста"),
        quota_remaining: msg!("{0} осталось {1}%"),
        usage_limit_title: "Достигнут лимит использования",
        usage_limit_detail: "Доступный лимит исчерпан. Выберите действие ниже, чтобы продолжить.",
        ..EN.chat
    },
    ..EN
};

static TABLES: Tables<Shared> = Tables::english(&EN).with_ru(&RU);

/// The shared table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Shared {
    TABLES.pick(lang(cx))
}

impl Shared {
    pub fn error<'a>(&'a self, message: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.chat.error, [Arg::Text(message)], &self.num)
    }

    pub fn lagged(&self, skipped: usize) -> Fmt<'_, 1> {
        Fmt::new(
            self.chat.lagged,
            [Arg::Int(u64::try_from(skipped).unwrap_or(u64::MAX))],
            &self.num,
        )
    }

    pub fn user_line<'a>(&'a self, text: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.chat.user_line, [Arg::Text(text)], &self.num)
    }

    pub fn agent_line<'a>(&'a self, text: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.chat.agent_line, [Arg::Text(text)], &self.num)
    }

    pub fn needs_answer<'a>(&'a self, id: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.presenter.needs_answer, [Arg::Text(id)], &self.num)
    }

    pub fn not_yet<'a>(&'a self, action: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.pages.not_yet, [Arg::Text(action)], &self.num)
    }

    pub fn empty_slot<'a>(&'a self, name: &'a str, id: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.debug.empty_slot,
            [Arg::Text(name), Arg::Text(id)],
            &self.num,
        )
    }
}

/// The table of `lang` itself, for code with no tree (tests, the host).
pub fn for_lang(lang: Lang) -> &'static Shared {
    TABLES.pick(lang)
}

impl Shared {
    pub(crate) fn status_model_fast(&self, model: &str, effort: &str) -> String {
        Fmt::new(
            self.chat.model_fast,
            [Arg::Text(model), Arg::Text(effort)],
            &self.num,
        )
        .to_string()
    }

    pub(crate) fn status_progress(&self, done: usize, total: usize) -> String {
        Fmt::new(
            self.chat.tasks_progress,
            [Arg::Int(done as u64), Arg::Int(total as u64)],
            &self.num,
        )
        .to_string()
    }

    pub(crate) fn status_context(&self, left: i64, show_used: bool) -> String {
        let left = left.clamp(0, 100) as u64;
        let (message, percent) = if show_used {
            (self.chat.context_used, 100 - left)
        } else {
            (self.chat.context_remaining, left)
        };
        Fmt::new(message, [Arg::Int(percent)], &self.num).to_string()
    }

    pub(crate) fn status_credits(&self, credits: &str) -> String {
        Fmt::new(self.chat.thread_credits, [Arg::Text(credits)], &self.num).to_string()
    }

    pub(crate) fn status_quota(&self, label: &str, percent: u64) -> String {
        Fmt::new(
            self.chat.quota_remaining,
            [Arg::Text(label), Arg::Int(percent)],
            &self.num,
        )
        .to_string()
    }
}
