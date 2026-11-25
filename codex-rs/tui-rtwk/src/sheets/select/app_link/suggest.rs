//! What the app link dialog is about (oracle `bottom_pane/app_link_view.rs`): the app of `/apps`,
//! or a request a server made (a tool suggestion to install or enable an app, a sign-in, a URL to
//! visit). The flavour decides the words of both screens, the rows of each and the label of a
//! row; the target decides what an action asks of the model (`app_link.rs`). The words are
//! `strings.rs`'s; this file picks among them.

use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::approvals::link::{AppLinkPrompt, Kind};
use codex_tui_core::screens::select::apps::Link;

use super::strings::Strings;

/// Which screen of the dialog is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Link,
    Confirm,
}

/// What the dialog is for.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// An app picked in `/apps`: its page, its switch.
    Apps(Link),
    /// A request of a server, held by the queue under the key.
    Request(RequestKey, AppLinkPrompt),
}

/// The kinds of dialog the oracle's view is (`AppLinkSuggestionType`, and none for `/apps`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    Apps,
    Install,
    Enable,
    /// A sign-in; `apps` when the apps server asked (its words name ChatGPT).
    Auth {
        apps: bool,
    },
    Url,
}

/// What an action row of a screen does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the page of the app in the browser.
    Open,
    /// Switch the app on or off.
    Switch,
    /// Close the dialog (a request is declined).
    Close,
    /// The page was visited: finish.
    Done,
    /// Back to the link screen.
    Back,
}

impl Flavor {
    /// The page is a browser flow, not the app's own page (a sign-in, a URL).
    pub fn browser(self) -> bool {
        matches!(self, Flavor::Auth { .. } | Flavor::Url)
    }

    /// Opening the page goes on to the confirmation screen (`open_external_url`).
    pub fn goes_on(self, installed: bool) -> bool {
        !installed || self.browser()
    }

    /// The actions of a screen, in order (`action_labels`). On the confirmation screen a request
    /// declines on its second row, where `/apps` goes back to the first screen.
    pub fn actions(self, screen: Screen, installed: bool) -> &'static [Action] {
        match screen {
            Screen::Link => {
                // The switch is the row of an installed app of `/apps` and of an enable suggestion.
                let switch = match self {
                    Flavor::Apps => installed,
                    Flavor::Enable => true,
                    _ => false,
                };
                if switch {
                    &[Action::Open, Action::Switch, Action::Close]
                } else {
                    &[Action::Open, Action::Close]
                }
            }
            Screen::Confirm if self == Flavor::Apps => &[Action::Done, Action::Back],
            Screen::Confirm => &[Action::Done, Action::Close],
        }
    }

    /// The label of an action row.
    pub fn label(self, action: Action, installed: bool, enabled: bool, words: &Strings) -> &str {
        match action {
            Action::Open => match self {
                Flavor::Auth { .. } => words.open_sign_in,
                Flavor::Url => words.open_link,
                _ if installed => words.manage,
                _ => words.install,
            },
            Action::Switch if enabled => words.disable,
            Action::Switch => words.enable,
            Action::Close | Action::Back => words.back,
            Action::Done => match self {
                Flavor::Auth { .. } => words.signed_in,
                Flavor::Url => words.finished,
                _ => words.installed_it,
            },
        }
    }

    /// The instructions of the link screen.
    pub fn instructions(self, installed: bool, words: &Strings) -> &str {
        match self {
            Flavor::Apps if installed => words.instructions_manage,
            Flavor::Apps => words.instructions_install,
            Flavor::Install => words.instructions_install_request,
            Flavor::Enable => words.instructions_enable,
            Flavor::Auth { .. } => words.instructions_auth,
            Flavor::Url => words.instructions_url,
        }
    }

    /// The title, the two lines and the label of the URL on the confirmation screen.
    pub fn finish<'a>(self, words: &'a Strings) -> Finish<'a> {
        match self {
            Flavor::Auth { apps } => Finish {
                title: if apps {
                    words.finish_sign_in
                } else {
                    words.finish_auth
                },
                open: if apps {
                    words.sign_in_open_apps
                } else {
                    words.sign_in_open
                },
                then: words.sign_in_then,
                url: words.sign_in_url,
            },
            Flavor::Url => Finish {
                title: words.finish_browser,
                open: words.action_open,
                then: words.action_then,
                url: words.link_url,
            },
            _ => Finish {
                title: words.finish_title,
                open: words.finish_open,
                then: words.finish_then,
                url: words.setup_url,
            },
        }
    }
}

/// The words of a confirmation screen.
pub struct Finish<'a> {
    pub title: &'a str,
    pub open: &'a str,
    pub then: &'a str,
    pub url: &'a str,
}

impl Target {
    pub fn flavor(&self) -> Flavor {
        match self {
            Target::Apps(_) => Flavor::Apps,
            Target::Request(_, prompt) => match prompt.kind {
                Kind::Install => Flavor::Install,
                Kind::Enable => Flavor::Enable,
                Kind::Auth => Flavor::Auth {
                    apps: prompt.refreshes_apps(),
                },
                Kind::Url => Flavor::Url,
            },
        }
    }

    pub fn installed(&self) -> bool {
        match self {
            Target::Apps(link) => link.installed,
            Target::Request(_, prompt) => prompt.installed(),
        }
    }

    /// The switch as the dialog opens (`is_enabled`): a request of the oracle starts with the
    /// apps off, but a sign-in and a URL, which have no switch.
    pub fn enabled(&self) -> bool {
        match self {
            Target::Apps(link) => link.enabled,
            Target::Request(_, prompt) => prompt.browser_flow(),
        }
    }

    pub fn url(&self) -> &str {
        match self {
            Target::Apps(link) => &link.url,
            Target::Request(_, prompt) => &prompt.url,
        }
    }

    /// The title: the app's name, or what a URL request is for.
    pub fn title<'a>(&'a self, words: &'a Strings) -> &'a str {
        match self {
            Target::Request(_, prompt) if prompt.kind == Kind::Url => words.action_required,
            Target::Apps(link) => &link.name,
            Target::Request(_, prompt) => &prompt.name,
        }
    }

    /// The line under the title: what the app is, or the server a URL request came from.
    pub fn description(&self, words: &Strings) -> Option<String> {
        match self {
            Target::Apps(link) => link.description.clone(),
            Target::Request(_, prompt) if prompt.kind == Kind::Url => {
                Some(format!("{}{}", words.server_prefix, prompt.server))
            }
            Target::Request(_, prompt) => prompt.description.clone(),
        }
    }

    /// Why it is asked.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Target::Apps(_) => None,
            Target::Request(_, prompt) => prompt.reason.as_deref(),
        }
    }
}
