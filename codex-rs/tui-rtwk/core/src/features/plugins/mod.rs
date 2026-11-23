//! `/plugins` (oracle `chatwidget/plugins.rs`, `plugin_catalog.rs`, `app/background_requests.rs`)
//! as an ADR-013 feature: the domain of the plugin browser, none of its pages. The server's
//! answers are `Remote`s cached by directory and plugin, the changes the user asks for are ops
//! under an `OpKey`, a surface that shows the list says so (`Interest`), and the setup of the
//! apps an installed plugin needs signed in is a `Wish` the surface presents once. The sheet, its
//! tabs and pages and their words are `src/features/plugins/`.
//!
//! The ADR-013 template for a feature: `mod.rs` (this: the marker, the verbs, the keys of the
//! asks and the words said), `state.rs` (the domain), `wire.rs` (the RPCs), `catalog.rs` (the
//! pure derivations).

pub mod catalog;
mod state;
mod wire;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::app::effects::Done;
use crate::ids::{Cwd, ThreadId};
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use catalog::{
    Action, Catalog, Config, Configured, Detail, Kind, Location, Marketplace, Part, Placeholder,
    Plugin, Product, Row, RowKey, Section, SectionError, Source, Status as PluginStatus, Tab, Target,
    Toggle,
};
pub use state::{Gen, Market, Op, OpKey, Outcome, PluginKey, Setup, State, Status};
pub use wire::{AuthApp, Found, Want, Wanted, add_source, decode, wire_request};

/// The marker of the feature (`feature_list!`).
pub struct Plugins;

/// What the user or a surface asks of the feature: verbs, with ids (a directory, a plugin's row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/plugins` was run in `cwd`: says so when the feature is off.
    Run { cwd: Cwd },
    /// The detail of a plugin of the list.
    Read { key: PluginKey },
    Install { key: PluginKey },
    Uninstall { key: PluginKey },
    /// The switch of a plugin of the list set to `enabled` (coalesced while a write runs).
    SetEnabled { key: PluginKey, enabled: bool },
    AddMarketplace { cwd: Cwd, source: String },
    RemoveMarketplace { cwd: Cwd, name: String },
    /// One marketplace, or all of the configured git ones.
    UpgradeMarketplace { cwd: Cwd, name: Option<String> },
    /// The page of one of the apps of the setup (`app` is its id) opens in the browser.
    OpenSetupPage { key: PluginKey, app: String },
    /// The setup of an installed plugin's apps is over.
    EndSetup { key: PluginKey, how: SetupEnd },
}

/// How the setup of an installed plugin's apps ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupEnd {
    /// Every app is signed in: the history says so.
    Completed,
    /// The user gave it up on the page: the history says so.
    Skipped,
    /// The sheet was closed on the page: nothing is said.
    Dismissed,
}

/// What the feature asks the surface to show, once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// The setup of the apps the installed plugin needs signed in (`Setup` under the key).
    Auth(PluginKey),
}

/// What a surface can want loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interest {
    /// The list of the plugins of `cwd`.
    Listing { cwd: Cwd },
}

/// What the host tells the feature unasked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// What the user's config says of the plugins and their marketplaces.
    Config(Config),
}

impl Fact {
    /// The config as the host read it: the three switches, the home directory and the
    /// marketplaces of the user's config as (name, is a git one).
    pub fn config(
        enabled: bool,
        remote: bool,
        sharing: bool,
        home: Option<String>,
        configured: impl IntoIterator<Item = (String, bool)>,
    ) -> Fact {
        Fact::Config(Config {
            enabled,
            remote,
            sharing,
            home,
            configured: configured
                .into_iter()
                .map(|(name, git)| Configured { name, git })
                .collect(),
        })
    }
}

/// What an ask was for, given back with its answer: the key the answer is filed under and the
/// number that tells it from an older ask of the same key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    List { cwd: Cwd, request: u64 },
    Section { cwd: Cwd, request: u64, part: Part },
    Read { key: PluginKey, request: u64 },
    Install { key: PluginKey, at: Gen, name: String, target: Target },
    Uninstall { key: PluginKey, at: Gen, name: String },
    Add { cwd: Cwd, at: Gen },
    Remove { cwd: Cwd, at: Gen, name: String, label: String },
    Upgrade { cwd: Cwd, at: Gen, name: Option<String> },
    Enable { cwd: Cwd, id: String, enabled: bool },
    /// A page opened in the browser (the sign-in page of an app): the host's answer says whether
    /// it opened.
    Page { url: String },
}

/// What the feature says in the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Say {
    /// The plugins feature is off.
    Disabled,
    /// `Installed {name} plugin.`; the apps still to sign in (none: nothing more is needed).
    Installed { name: String, apps: Vec<String> },
    Uninstalled { name: String },
    /// The setup of the apps was given up, or went through.
    Skipped { name: String },
    Completed { name: String },
    EnableFailed { id: String, error: RpcError },
    MarketAdded { name: String, root: String, already: bool },
    MarketRemoved { label: String, name: String, root: Option<String> },
    /// No configured git marketplace to upgrade.
    NothingToUpgrade,
    UpToDate { selected: Vec<String> },
    Upgraded { roots: Vec<String> },
    UpgradeFailed(Vec<(String, String)>),
    /// The request itself failed.
    UpgradeError(RpcError),
    /// The browser did not open `url`.
    PageFailed { url: String, error: String },
}

impl Say {
    pub fn error(&self) -> bool {
        matches!(
            self,
            Say::EnableFailed { .. }
                | Say::UpgradeFailed(_)
                | Say::UpgradeError(_)
                | Say::PageFailed { .. }
        )
    }
}

impl Feature for Plugins {
    const NAME: &'static str = "plugins";

    type State = State;
    type ThreadPart = ();
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Wanted;
    type Reply = Found;
    type Ctx = Ctx;
    type Menu = Never<Self>;
    type Pick = Never<Self>;
    type Prompt = Never<Self>;
    type Note = Say;
    type Fact = Fact;
    type Wish = Wish;
    type Interest = Interest;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        state.intent(intent, cx);
    }

    fn answered(state: &mut State, ctx: Ctx, result: Result<Found, RpcError>, cx: &mut AppCx<'_>) {
        state.answered(ctx, result, cx);
    }

    fn note_tone(note: &Say) -> Tone {
        if note.error() { Tone::Error } else { Tone::Info }
    }

    fn effect_done(_state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        if let (Ctx::Page { url }, Done::Opened(Err(error))) = (ctx, done) {
            cx.note(None, Say::PageFailed { url, error });
        }
    }

    fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
        let Fact::Config(config) = fact;
        state.set_config(config);
    }

    fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
        match interest {
            Interest::Listing { cwd } if open => state.prefetch(cwd, cx),
            Interest::Listing { .. } => {}
        }
    }

    fn wire(command: &Wanted, _thread: &ThreadId, id: RequestId) -> ClientRequest {
        wire_request(command, id)
    }

    fn decode(command: &Wanted, result: serde_json::Value) -> Result<Found, RpcError> {
        decode(command, result).map_err(RpcError::Malformed)
    }
}
