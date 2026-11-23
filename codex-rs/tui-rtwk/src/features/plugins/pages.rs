//! The pages of the sheet of `/plugins`: the routes of its nested `Navigator` (ADR-013 O1). The
//! home is the browser, the other pages are pushed over it and popped back to it: the home's
//! `State` keeps its picker (cursor, query, tab) under them, so a page that goes back returns to
//! the same row. Esc follows the oracle page by page: it closes the sheet (the oracle's detail,
//! loading and failure pages have no `on_cancel`; the way back is their `Back to plugins` row),
//! and goes back only where the oracle does (the removal's confirmation, the source prompt). A
//! page that waits for a change the user asked for (an install, an add, a remove, an upgrade) keeps
//! `awaiting` (the op's key and the generation before it was asked), and reads the feature's op
//! record under that key: while there is no newer one, or it runs, the page says it loads; when it
//! failed the page says so; when it is done the page goes on or pops. The feature never sees a
//! page: it sees verbs with ids (`Intent`) and keeps requests, ops and the data (`State`). In
//! Flutter these are the `MaterialPageRoute`s of a `Navigator` inside the bottom sheet, each a
//! `ConsumerStatefulWidget` over a provider that selects the feature's part of the app.

use std::rc::Rc;

use rtwk::layout::{Align, Alignment, EdgeInsets, Padding};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::plugins::catalog::{ALL, RowKey, tab_id_of_root};
use codex_tui_core::features::plugins::{
    Action, Detail as DetailData, Gen, Intent, OpKey, Outcome, PluginKey, SetupEnd,
    State as Domain, Status, Toggle,
};
use codex_tui_core::features::{FeatureStates, Remote};
use codex_tui_core::ids::Cwd;
use codex_tui_core::rpc::RpcError;
use codex_tui_core::screens::select::Intent as SelectIntent;
use codex_tui_core::screens::select::list::List;
use codex_tui_core::screens::{ScreenIntent, ScreenStates, Shared};

use super::lists::{self, AuthStep};
use super::strings;
use crate::app::view::Hs;
use crate::features::verb;
use crate::keymap::keymap;
use crate::sheets::select::prompt::Answer;
use crate::widgets::picker::{Event, Picker};

// ---------------------------------------------------------------------------------- reading

/// The feature's part of the app, selected: a page wakes only when this changed.
fn domain<S: 'static>(cx: &mut Cx<'_, S>) -> Shared<Domain> {
    select::<FeatureStates, Shared<Domain>, _>(cx, |features| features.plugins.clone())
}

/// The same, read once in a handler (no dependency).
fn current<S: 'static>(cx: &mut Cx<'_, S>) -> Shared<Domain> {
    read::<FeatureStates, _>(cx).plugins.clone()
}

/// The op record under `key` that is newer than what the page saw before it asked.
fn newer<'a>(state: &'a Domain, awaiting: &Option<(OpKey, Gen)>) -> Option<&'a Status> {
    let (key, before) = awaiting.as_ref()?;
    state
        .op(key)
        .filter(|op| op.at > *before)
        .map(|op| &op.status)
}

/// The tab of the catalog a tab id (a marketplace's root) names now.
fn resolved(state: &Domain, cwd: &Cwd, saved: String) -> String {
    state.tab_now(cwd, &saved).unwrap_or(saved)
}

// ---------------------------------------------------------------------------------- moving

/// Back to the page under this one.
fn pop<S: 'static>(cx: &mut Cx<'_, S>) {
    Navigator::of(cx).pop(cx);
}

/// Back to the browser, whatever is over it.
fn pop_home<S: 'static>(cx: &mut Cx<'_, S>) {
    Navigator::of(cx).pop_until(cx, rtwk::navigation::Route::is_first);
}

/// Closes the whole sheet.
fn close<S: 'static>(cx: &mut Cx<'_, S>) {
    close_dialog::<_, ()>(cx, None);
}

fn push<S: 'static>(cx: &mut Cx<'_, S>, route: PageRoute<()>) {
    drop(Navigator::of(cx).push::<()>(route));
}

/// A message to this page after the build under way: what a change of the feature's state asks of
/// the navigator is done between builds, never inside one.
fn soon<S: State>(cx: &mut Cx<'_, S>, msg: S::Msg)
where
    S::Msg: Clone + PartialEq,
{
    let go = cx.send(msg);
    cx.emit(&go, ());
}

/// The picker of a page, keyed by the page it is: a page that changes into another is a new
/// picker, a refresh of the same page keeps its cursor.
fn picker(list: List, on: Handler<Event>, page: u32) -> AnyWidget {
    AnyWidget::from(Picker::new(Rc::new(list), on)).with_key(page)
}

/// The question over the chat: 72 cells wide at most, two cells in from the screen.
fn floating(picker: Picker, page: u32) -> AnyWidget {
    let boxed: AnyWidget = Padding::new(
        EdgeInsets::symmetric(2, 0),
        Dialog::bare(picker).max_width(72),
    )
    .into();
    boxed.with_key(page)
}

// ------------------------------------------------------------------------------- the routes

/// A page at the bottom of the sheet. The navigator is as tall as the sheet's cap (a `Navigator`
/// under bounded constraints fills them) and gives each route that size tight; the page is as
/// tall as what it shows, so it sits at the bottom edge where the oracle's pane is, under the
/// transparent rest of the cap. (`IntrinsicHeight` over the navigator cannot size it: a page of
/// the picker is built by a `LayoutBuilder` from the height it is given, which has no intrinsic
/// height before it is built; `tests/rtwk_o1.rs`.)
pub fn bottom(page: impl Into<AnyWidget>) -> AnyWidget {
    Align::new(Alignment::BOTTOM_CENTER, page).into()
}

pub fn detail_route(key: PluginKey) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Detail { key: key.clone() }))
}

pub fn add_route(cwd: Cwd) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Add { cwd: cwd.clone() }))
}

pub fn remove_route(cwd: Cwd, name: String, label: String) -> PageRoute<()> {
    PageRoute::new(move |_| {
        bottom(Remove {
            cwd: cwd.clone(),
            name: name.clone(),
            label: label.clone(),
        })
    })
}

pub fn upgrade_route(cwd: Cwd, name: Option<String>, before: Gen) -> PageRoute<()> {
    PageRoute::new(move |_| {
        bottom(Upgrade {
            cwd: cwd.clone(),
            name: name.clone(),
            before,
        })
    })
}

/// The setup of the apps an installed plugin needs signed in.
pub fn auth_route(key: PluginKey) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Auth { key: key.clone() }))
}

// ------------------------------------------------------------------------------- the browser

/// The browser: the tabs of the plugins of `cwd`, or the page that stands in while there are none.
#[derive(Clone, Debug, PartialEq)]
pub struct Home {
    pub cwd: Cwd,
}

pub struct HomeState {
    /// The tab the user is at (the picker keeps it; this is what a new list opens at).
    tab: String,
    /// The tab of the marketplace just added, until the user leaves it.
    fresh: Option<String>,
    /// The op a page over the browser waits for: what it did to the list sets the tab when done.
    awaiting: Option<(OpKey, Gen)>,
    /// The setups whose page was pushed: a setup the feature holds and the sheet has not shown
    /// gets its page (the wish to show it only opened this sheet when none was open).
    shown: Vec<PluginKey>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HomeMsg {
    Picked(Event),
    /// Show the setup of the apps over the browser.
    Setup(PluginKey),
}

impl StatefulWidget for Home {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState {
            tab: ALL.to_owned(),
            fresh: None,
            awaiting: None,
            shown: Vec::new(),
        }
    }
}

impl_widget!(stateful eq Home);

impl HomeState {
    /// What the op a page over the browser did to the list is the tab the browser shows.
    fn waited(&mut self, state: &Domain, cwd: &Cwd) {
        let Some((key, before)) = self.awaiting.clone() else {
            return;
        };
        let Some(op) = state.op(&key).filter(|op| op.at > before) else {
            return;
        };
        let Status::Done(outcome) = &op.status else {
            return;
        };
        match outcome {
            Outcome::Added { root, already } => {
                let tab = resolved(state, cwd, tab_id_of_root(root));
                self.fresh = (!already).then(|| tab.clone());
                self.tab = tab;
            }
            Outcome::Removed => {
                self.tab = ALL.to_owned();
                self.fresh = None;
            }
            Outcome::Upgraded { root: Some(root) } => {
                self.tab = resolved(state, cwd, tab_id_of_root(root));
            }
            Outcome::Upgraded { root: None } | Outcome::Nothing => {}
        }
        self.awaiting = None;
    }

    /// The op of `key` is what the next page over the browser waits for.
    fn await_op(&mut self, state: &Domain, key: OpKey) -> Gen {
        let before = state.op_at(&key);
        self.awaiting = Some((key, before));
        before
    }

    fn picked(&mut self, event: Event, w: &Home, cx: &mut Cx<'_, Self>) -> Rebuild {
        let state = current(cx);
        let cwd = w.cwd.clone();
        match event {
            Event::Choose(id) if id == lists::ADD_ROW => {
                self.await_op(&state, OpKey::Add { cwd: cwd.clone() });
                push(cx, add_route(cwd));
            }
            Event::Choose(id) => {
                if let Some(row) = RowKey::parse(&id) {
                    let key = PluginKey { cwd, row };
                    if state.readable(&key) {
                        verb(cx, Intent::Read { key: key.clone() });
                        push(cx, detail_route(key));
                    }
                }
            }
            Event::Toggle(id) => {
                let Some(row) = RowKey::parse(&id) else {
                    return Rebuild::No;
                };
                let switch = state
                    .tabs(&cwd, self.fresh.as_deref())
                    .into_iter()
                    .next()
                    .and_then(|all| all.rows.into_iter().find(|entry| entry.key == row))
                    .map(|entry| entry.toggle);
                if let Some(Toggle::Switch(on)) = switch {
                    let key = PluginKey { cwd, row };
                    verb(cx, Intent::SetEnabled { key, enabled: !on });
                }
            }
            Event::Tab(id) => {
                if self.tab != id {
                    self.tab = id;
                    return if self.fresh.take().is_some() {
                        Rebuild::Yes
                    } else {
                        Rebuild::No
                    };
                }
            }
            Event::Hot(lists::REMOVE_KEY) => {
                if let Some(market) = state.market(&cwd, &self.tab, self.fresh.as_deref()) {
                    let key = OpKey::Remove {
                        cwd: cwd.clone(),
                        name: market.name.clone(),
                    };
                    self.await_op(&state, key);
                    push(cx, remove_route(cwd, market.name, market.label));
                }
            }
            Event::Hot(lists::UPGRADE_KEY) => {
                if let Some(market) = state.market(&cwd, &self.tab, self.fresh.as_deref())
                    && market.upgrade
                {
                    let key = OpKey::Upgrade {
                        cwd: cwd.clone(),
                        name: Some(market.name.clone()),
                    };
                    let before = self.await_op(&state, key);
                    verb(
                        cx,
                        Intent::UpgradeMarketplace {
                            cwd: cwd.clone(),
                            name: Some(market.name.clone()),
                        },
                    );
                    push(cx, upgrade_route(cwd, Some(market.name), before));
                }
            }
            Event::Cancel => close(cx),
            Event::Secondary(_) | Event::Reorder(..) | Event::Confirm | Event::Hot(_) => {}
        }
        Rebuild::No
    }
}

impl State for HomeState {
    type Widget = Home;
    type Msg = HomeMsg;

    fn did_change_dependencies(&mut self, w: &Home, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        // A setup that ended may come again for the same plugin: it is new then.
        self.shown.retain(|key| state.setup(key).is_some());
        let fresh: Vec<PluginKey> = state
            .setups_in(&w.cwd)
            .filter(|key| !self.shown.contains(key))
            .cloned()
            .collect();
        for key in fresh {
            self.shown.push(key.clone());
            soon(cx, HomeMsg::Setup(key));
        }
        self.waited(&state, &w.cwd);
    }

    fn update(&mut self, msg: HomeMsg, w: &Home, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            HomeMsg::Setup(key) => {
                push(cx, auth_route(key));
                Rebuild::No
            }
            HomeMsg::Picked(event) => self.picked(event, w, cx),
        }
    }

    fn build(&self, w: &Home, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let state = domain(cx);
        let words = strings::of(cx);
        let keys = keymap(cx);
        let on = cx.map(HomeMsg::Picked);
        let waiting = || picker(lists::loading_list(words), on.clone(), 0);
        match state.catalog(&w.cwd) {
            None | Some(Remote::Loading(None)) => waiting(),
            Some(Remote::Failed(error)) => picker(lists::failed_list(error, words), on, 1),
            Some(_) => {
                match lists::browser(
                    &state,
                    &w.cwd,
                    &self.tab,
                    self.fresh.as_deref(),
                    &keys,
                    words,
                ) {
                    Some((list, extras)) => {
                        AnyWidget::from(Picker::new(Rc::new(list), on).extras(extras)).with_key(2)
                    }
                    None => waiting(),
                }
            }
        }
    }
}

// --------------------------------------------------------------------------- one plugin

/// The detail of a plugin: what it is, and the install or uninstall it offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub key: PluginKey,
}

pub struct DetailState {
    /// The install or uninstall asked on this page, and the generation before it.
    awaiting: Option<(OpKey, Gen)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DetailMsg {
    Picked(Event),
    Pop,
}

/// What the page says now.
enum Shown<'a> {
    Loading(String),
    Installing(String),
    Uninstalling(String),
    Failed(RpcError),
    Ready(&'a DetailData),
}

impl StatefulWidget for Detail {
    type State = DetailState;

    fn create_state(&self) -> DetailState {
        DetailState { awaiting: None }
    }
}

impl_widget!(stateful eq Detail);

impl DetailState {
    fn shown<'a>(&self, state: &'a Domain, key: &PluginKey) -> Shown<'a> {
        let name = || state.plugin_name(key).unwrap_or_default();
        if let Some((op, _)) = &self.awaiting {
            match newer(state, &self.awaiting) {
                Some(Status::Failed(error)) => return Shown::Failed(error.clone()),
                Some(Status::Done(_)) => {}
                Some(Status::Running) | None => {
                    return match op {
                        OpKey::Uninstall(_) => Shown::Uninstalling(name()),
                        _ => Shown::Installing(name()),
                    };
                }
            }
        }
        match state.detail(key) {
            None | Some(Remote::Loading(None)) => Shown::Loading(name()),
            Some(Remote::Failed(error)) => Shown::Failed(error.clone()),
            Some(Remote::Ready(detail) | Remote::Loading(Some(detail))) => Shown::Ready(detail),
        }
    }
}

impl State for DetailState {
    type Widget = Detail;
    type Msg = DetailMsg;

    fn did_change_dependencies(&mut self, _: &Detail, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        let Some(Status::Done(_)) = newer(&state, &self.awaiting) else {
            return;
        };
        // An uninstalled plugin has no page: the list it left is read again, the page goes.
        if matches!(self.awaiting, Some((OpKey::Uninstall(_), _))) {
            soon(cx, DetailMsg::Pop);
        }
        self.awaiting = None;
    }

    fn update(&mut self, msg: DetailMsg, w: &Detail, cx: &mut Cx<'_, Self>) -> Rebuild {
        let event = match msg {
            DetailMsg::Pop => {
                pop(cx);
                return Rebuild::No;
            }
            DetailMsg::Picked(event) => event,
        };
        let state = current(cx);
        match event {
            Event::Choose(id) => match (id.as_str(), state.detail(&w.key)) {
                (lists::BACK, _) => pop(cx),
                (lists::INSTALL, Some(Remote::Ready(detail)))
                    if matches!(detail.action(), Action::Install(_)) =>
                {
                    let key = OpKey::Install(w.key.clone());
                    self.awaiting = Some((key.clone(), state.op_at(&key)));
                    verb(cx, Intent::Install { key: w.key.clone() });
                    return Rebuild::Yes;
                }
                (lists::UNINSTALL, Some(Remote::Ready(detail)))
                    if matches!(detail.action(), Action::Uninstall { .. }) =>
                {
                    let key = OpKey::Uninstall(w.key.clone());
                    self.awaiting = Some((key.clone(), state.op_at(&key)));
                    verb(cx, Intent::Uninstall { key: w.key.clone() });
                    return Rebuild::Yes;
                }
                _ => {}
            },
            // The oracle's detail pages (loading, failed, shown) have no `on_cancel`: Esc closes.
            Event::Cancel => close(cx),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Detail, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let state = domain(cx);
        let words = strings::of(cx);
        let on = cx.map(DetailMsg::Picked);
        match self.shown(&state, &w.key) {
            Shown::Loading(name) => picker(lists::detail_loading(&name, words), on, 10),
            Shown::Installing(name) => picker(lists::install_loading(&name, words), on, 13),
            Shown::Uninstalling(name) => picker(lists::uninstall_loading(&name, words), on, 14),
            Shown::Failed(error) => picker(lists::detail_failed(&error, words), on, 12),
            Shown::Ready(detail) => picker(lists::detail_page(detail, words), on, 11),
        }
    }
}

// ------------------------------------------------------------------------- a marketplace

/// The prompt of a marketplace source, the wait for it to be added, and the failure.
#[derive(Clone, Debug, PartialEq)]
pub struct Add {
    pub cwd: Cwd,
}

pub struct AddState {
    awaiting: Option<(OpKey, Gen)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AddMsg {
    Source(Answer),
    Picked(Event),
    Pop,
}

impl StatefulWidget for Add {
    type State = AddState;

    fn create_state(&self) -> AddState {
        AddState { awaiting: None }
    }
}

impl_widget!(stateful eq Add);

impl State for AddState {
    type Widget = Add;
    type Msg = AddMsg;

    fn did_change_dependencies(&mut self, _: &Add, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        if let Some(Status::Done(_)) = newer(&state, &self.awaiting) {
            self.awaiting = None;
            soon(cx, AddMsg::Pop);
        }
    }

    fn update(&mut self, msg: AddMsg, w: &Add, cx: &mut Cx<'_, Self>) -> Rebuild {
        let state = current(cx);
        match msg {
            AddMsg::Pop | AddMsg::Source(Answer::Back) => pop(cx),
            AddMsg::Source(Answer::Submit(source)) => {
                let key = OpKey::Add { cwd: w.cwd.clone() };
                self.awaiting = Some((key.clone(), state.op_at(&key)));
                verb(
                    cx,
                    Intent::AddMarketplace {
                        cwd: w.cwd.clone(),
                        source,
                    },
                );
                return Rebuild::Yes;
            }
            AddMsg::Picked(Event::Choose(id)) if id == lists::RETRY => {
                self.awaiting = None;
                return Rebuild::Yes;
            }
            AddMsg::Picked(Event::Choose(id)) if id == lists::BACK => pop(cx),
            AddMsg::Picked(Event::Cancel) => {
                if matches!(newer(&state, &self.awaiting), Some(Status::Failed(_))) {
                    pop(cx);
                } else {
                    close(cx);
                }
            }
            AddMsg::Picked(_) => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Add, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let state = domain(cx);
        let words = strings::of(cx);
        if self.awaiting.is_none() {
            let prompt: AnyWidget = lists::add_prompt(cx.map(AddMsg::Source), words).into();
            return prompt.with_key(3);
        }
        let on = cx.map(AddMsg::Picked);
        let _ = &w.cwd;
        match newer(&state, &self.awaiting) {
            Some(Status::Failed(_)) => picker(lists::add_failed(words), on, 5),
            Some(Status::Done(_)) => AnyWidget::default(),
            Some(Status::Running) | None => picker(lists::add_loading(words), on, 4),
        }
    }
}

/// The question before a marketplace is removed, the wait for it, and the failure.
#[derive(Clone, Debug, PartialEq)]
pub struct Remove {
    pub cwd: Cwd,
    pub name: String,
    pub label: String,
}

pub struct RemoveState {
    awaiting: Option<(OpKey, Gen)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RemoveMsg {
    Picked(Event),
    Pop,
}

impl StatefulWidget for Remove {
    type State = RemoveState;

    fn create_state(&self) -> RemoveState {
        RemoveState { awaiting: None }
    }
}

impl_widget!(stateful eq Remove);

impl State for RemoveState {
    type Widget = Remove;
    type Msg = RemoveMsg;

    fn did_change_dependencies(&mut self, _: &Remove, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        if let Some(Status::Done(_)) = newer(&state, &self.awaiting) {
            self.awaiting = None;
            soon(cx, RemoveMsg::Pop);
        }
    }

    fn update(&mut self, msg: RemoveMsg, w: &Remove, cx: &mut Cx<'_, Self>) -> Rebuild {
        let event = match msg {
            RemoveMsg::Pop => {
                pop(cx);
                return Rebuild::No;
            }
            RemoveMsg::Picked(event) => event,
        };
        let state = current(cx);
        let failed = matches!(newer(&state, &self.awaiting), Some(Status::Failed(_)));
        match event {
            Event::Choose(id) if id == lists::REMOVE && self.awaiting.is_none() => {
                let key = OpKey::Remove {
                    cwd: w.cwd.clone(),
                    name: w.name.clone(),
                };
                self.awaiting = Some((key.clone(), state.op_at(&key)));
                verb(
                    cx,
                    Intent::RemoveMarketplace {
                        cwd: w.cwd.clone(),
                        name: w.name.clone(),
                    },
                );
                return Rebuild::Yes;
            }
            Event::Choose(id) if id == lists::RETRY && failed => {
                self.awaiting = None;
                return Rebuild::Yes;
            }
            Event::Choose(id) if id == lists::BACK => pop(cx),
            // The confirmation goes back to the list (its `on_cancel`); the pages after it
            // (loading, failed) have none, so Esc closes.
            Event::Cancel => {
                if self.awaiting.is_none() {
                    pop(cx);
                } else {
                    close(cx);
                }
            }
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Remove, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let state = domain(cx);
        let words = strings::of(cx);
        let on = cx.map(RemoveMsg::Picked);
        if self.awaiting.is_none() {
            let list = Rc::new(lists::confirmation(&w.label, words));
            return floating(Picker::new(list, on), 6);
        }
        match newer(&state, &self.awaiting) {
            Some(Status::Failed(_)) => picker(lists::remove_failed(words), on, 8),
            Some(Status::Done(_)) => AnyWidget::default(),
            Some(Status::Running) | None => picker(lists::remove_loading(&w.label, words), on, 7),
        }
    }
}

/// The wait for the upgrade of one marketplace, or of all the git ones.
#[derive(Clone, Debug, PartialEq)]
pub struct Upgrade {
    pub cwd: Cwd,
    pub name: Option<String>,
    /// The generation of the upgrade's op before it was asked.
    pub before: Gen,
}

pub struct UpgradeState;

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeMsg {
    Picked(Event),
    Pop,
}

impl StatefulWidget for Upgrade {
    type State = UpgradeState;

    fn create_state(&self) -> UpgradeState {
        UpgradeState
    }
}

impl_widget!(stateful eq Upgrade);

impl Upgrade {
    fn key(&self) -> OpKey {
        OpKey::Upgrade {
            cwd: self.cwd.clone(),
            name: self.name.clone(),
        }
    }
}

impl State for UpgradeState {
    type Widget = Upgrade;
    type Msg = UpgradeMsg;

    fn did_change_dependencies(&mut self, w: &Upgrade, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        let over = state
            .op(&w.key())
            .is_some_and(|op| op.at > w.before && !matches!(op.status, Status::Running));
        if over {
            soon(cx, UpgradeMsg::Pop);
        }
    }

    fn update(&mut self, msg: UpgradeMsg, _: &Upgrade, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            UpgradeMsg::Pop => pop(cx),
            UpgradeMsg::Picked(Event::Cancel) => close(cx),
            UpgradeMsg::Picked(_) => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Upgrade, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let words = strings::of(cx);
        let on = cx.map(UpgradeMsg::Picked);
        picker(lists::upgrade_loading(w.name.as_deref(), words), on, 9)
    }
}

// ---------------------------------------------------------------------------- the setup

/// The setup of the apps an installed plugin needs signed in, one app after the other.
#[derive(Clone, Debug, PartialEq)]
pub struct Auth {
    pub key: PluginKey,
}

pub struct AuthState {
    /// The app the user is at.
    step: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuthMsg {
    Picked(Event),
    /// The setup is over elsewhere: nothing to show.
    Over,
}

impl StatefulWidget for Auth {
    type State = AuthState;

    fn create_state(&self) -> AuthState {
        AuthState { step: 0 }
    }
}

impl_widget!(stateful eq Auth);

impl State for AuthState {
    type Widget = Auth;
    type Msg = AuthMsg;

    fn did_change_dependencies(&mut self, w: &Auth, cx: &mut Cx<'_, Self>) {
        let state = domain(cx);
        if state.setup(&w.key).is_none() {
            soon(cx, AuthMsg::Over);
        }
    }

    fn update(&mut self, msg: AuthMsg, w: &Auth, cx: &mut Cx<'_, Self>) -> Rebuild {
        let event = match msg {
            AuthMsg::Over => {
                pop_home(cx);
                return Rebuild::No;
            }
            AuthMsg::Picked(event) => event,
        };
        let state = current(cx);
        let Some(setup) = state.setup(&w.key) else {
            return Rebuild::No;
        };
        let end = |cx: &mut Cx<'_, Self>, how| {
            verb(
                cx,
                Intent::EndSetup {
                    key: w.key.clone(),
                    how,
                },
            );
        };
        match event {
            Event::Choose(id) if id == lists::OPEN => {
                if let Some(app) = setup.apps.get(self.step) {
                    let app = app.id.clone();
                    verb(
                        cx,
                        Intent::OpenSetupPage {
                            key: w.key.clone(),
                            app,
                        },
                    );
                }
            }
            Event::Choose(id) if id == lists::NEXT => {
                // The directory of the apps is read again: the one just visited may be in now.
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Select(SelectIntent::RefreshApps));
                if self.step + 1 >= setup.apps.len() {
                    end(cx, SetupEnd::Completed);
                    pop_home(cx);
                } else {
                    self.step += 1;
                    return Rebuild::Yes;
                }
            }
            Event::Choose(id) if id == lists::SKIP => {
                end(cx, SetupEnd::Skipped);
                pop_home(cx);
            }
            Event::Cancel => {
                end(cx, SetupEnd::Dismissed);
                close(cx);
            }
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Auth, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let state = domain(cx);
        let words = strings::of(cx);
        let on = cx.map(AuthMsg::Picked);
        let Some(setup) = state.setup(&w.key) else {
            return AnyWidget::default();
        };
        let Some(app) = setup.apps.get(self.step) else {
            return AnyWidget::default();
        };
        let id = app.id.clone();
        let installed =
            select::<ScreenStates, bool, _>(cx, move |states| states.select.app_installed(&id));
        let step = AuthStep {
            plugin: setup.plugin.clone(),
            app: app.clone(),
            current: self.step + 1,
            total: setup.apps.len(),
            installed,
        };
        picker(lists::auth_page(&step, words), on, 15)
    }
}
