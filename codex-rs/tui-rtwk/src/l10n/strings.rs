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
    pub ready: &'static str,
    pub error: Msg<1>,
    pub queued: Msg<1>,
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
    pub hint: &'static str,
    pub empty_slot: Msg<2>,
}

/// One locale's shared table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shared {
    /// Digits and separators for the messages' numbers.
    pub num: NumSyms,
    pub chat: Chat,
    pub presenter: Presenter,
    pub debug: Debug,
}

/// English. The skeleton's own text (no oracle string).
pub const EN: Shared = Shared {
    num: NumSyms::ENGLISH,
    chat: Chat {
        placeholder: "Ask Codex to do anything",
        starting: "starting…",
        working: "working…",
        ready: "ready",
        error: msg!("error: {0}"),
        queued: msg!("(queued) {0}"),
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
        hint: "up/down move, enter opens, q quits",
        empty_slot: msg!("{0} ({1}): empty slot"),
    },
};

/// Russian: three strings, the rest English (a proof of the path).
pub const RU: Shared = Shared {
    chat: Chat {
        placeholder: "Спросите Codex о чём угодно",
        working: "работаю…",
        ready: "готово",
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

    pub fn queued<'a>(&'a self, text: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.chat.queued, [Arg::Text(text)], &self.num)
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
