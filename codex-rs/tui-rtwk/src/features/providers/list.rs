//! The home page of the providers sheet (oracle `app/provider.rs` `list`, `load_error`,
//! `progress`): the list the server answered with, or the page that says it is being read, why it
//! could not be, or that a default is being written. What it shows is the feature's state; what it
//! asks is a verb (`Intent::Choose`), the read is the sheet's interest. A provider's Enter opens
//! the details pager; its `Edit configuration` and the row that creates a gateway push the form.
//!
//! Every phase is a page of its own (the oracle replaces the view), so its cursor starts on the
//! first row: the picker is keyed by the phase.

use std::rc::Rc;

use codex_app_server_protocol::{ModelProviderSummary, ProviderSettings};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::providers::form::Draft;
use codex_tui_core::features::providers::{Intent, Interest as Wanted, Load, Saving};
use codex_tui_core::keymap::actions::{LIST_ACCEPT, LIST_CANCEL};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::transcript::{PagerAction, PagerDoc};
use codex_tui_core::threads::ThreadModel;

use super::form::Form;
use super::strings::{self, Keys};
use crate::features::verb;
use crate::keymap::{keymap, label};
use crate::pages::session::transcript::pager::{StaticPager, cover_options, covering_with};
use crate::widgets::interest::Interest;
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

/// The home page of the sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Home;

pub struct HomeState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
    /// `Edit configuration` of the details of the n-th provider.
    Edit(usize),
}

impl StatefulWidget for Home {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState
    }
}

impl_widget!(stateful eq Home);

/// What the page reads of the feature.
#[derive(Clone, Debug, PartialEq)]
struct View {
    load: Load,
    saving: Option<Saving>,
    client_override: Option<String>,
}

fn view_of(features: &FeatureStates) -> View {
    let providers = &features.providers;
    View {
        load: providers.load().clone(),
        saving: providers.saving().cloned(),
        client_override: providers.client_override().map(str::to_owned),
    }
}

/// The provider of the chat the user is in: none when no chat is (the oracle: a thread id, then
/// the chat's configuration).
fn current<S: 'static>(cx: &mut Cx<'_, S>) -> Option<String> {
    let thread = select::<ThreadModel, Option<Option<String>>, _>(cx, |thread| {
        (!thread.id().as_str().is_empty()).then(|| thread.screen_slots().chat.provider.clone())
    });
    let status =
        select::<ScreenStates, Option<String>, _>(cx, |states| states.chat.status.provider.clone());
    thread?.or(status)
}

/// The providers of the catalog the page shows now.
fn catalog<S: 'static>(cx: &mut Cx<'_, S>) -> Vec<ModelProviderSummary> {
    match read::<FeatureStates, _>(cx).providers.load() {
        Load::Ready(page) => page.list.data.clone(),
        _ => Vec::new(),
    }
}

impl HomeState {
    /// The form of the n-th provider of the catalog (none: a new gateway) pushed over the list.
    fn edit(cx: &mut Cx<'_, Self>, index: Option<usize>) {
        let catalog = catalog(cx);
        let provider = match index {
            Some(n) => match catalog.get(n) {
                Some(provider) => Some(provider.clone()),
                None => return,
            },
            None => None,
        };
        let draft = Draft::new(provider, &catalog);
        drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(move |_| {
            bottom(Form {
                draft: draft.clone(),
            })
        })));
    }

    /// The details of the n-th provider in a pager over the screen (`Event::Details`): Enter
    /// edits what can be edited.
    fn details(cx: &mut Cx<'_, Self>, index: usize) {
        let Some(provider) = catalog(cx).get(index).cloned() else {
            return;
        };
        let editable = provider.can_edit && !matches!(provider.settings, ProviderSettings::OpenAi);
        let doc = PagerDoc {
            title: strings::DETAILS_TITLE.to_owned(),
            text: strings::details(&provider),
            action: editable.then_some(PagerAction::EditConfiguration),
        };
        let pager = StaticPager::over(doc).on_action(cx.map(move |()| Msg::Edit(index)));
        drop(show_dialog::<(), Self>(
            cx,
            move |_| covering_with(pager.clone()),
            cover_options(),
        ));
    }

    fn choose(cx: &mut Cx<'_, Self>, id: &str) {
        match id {
            strings::BACK => close_dialog::<_, ()>(cx, None),
            strings::CREATE => Self::edit(cx, None),
            id => {
                if let Some(index) = index_of(id) {
                    Self::details(cx, index);
                }
            }
        }
    }
}

/// The catalog index a row id names.
fn index_of(id: &str) -> Option<usize> {
    id.strip_prefix(strings::PROVIDER)?.parse().ok()
}

impl State for HomeState {
    type Widget = Home;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _w: &Home, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(Event::Choose(id)) => Self::choose(cx, &id),
            Msg::Picked(Event::Secondary(id)) => {
                let provider = index_of(&id).and_then(|index| catalog(cx).get(index).cloned());
                if let Some(provider) = provider {
                    verb(cx, Intent::Choose(provider.id));
                }
            }
            // Esc leaves the sheet, whatever it shows: a write under way is not waited for.
            Msg::Picked(Event::Cancel) => close_dialog::<_, ()>(cx, None),
            Msg::Picked(_) => {}
            Msg::Edit(index) => Self::edit(cx, Some(index)),
        }
        Rebuild::No
    }

    fn build(&self, _w: &Home, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let view = select::<FeatureStates, View, _>(cx, view_of);
        let current = current(cx);
        let keys = {
            let keymap = keymap(cx);
            Keys {
                accept: label(&keymap, LIST_ACCEPT),
                cancel: label(&keymap, LIST_CANCEL),
            }
        };
        let (list, phase, saving) = match (&view.saving, &view.load) {
            (Some(Saving::Default(_)), _) => (strings::saving(), 3u8, true),
            (_, Load::Idle | Load::Loading) => (strings::loading(), 0, false),
            (_, Load::Failed(failure)) => (strings::failed(failure), 1, false),
            (_, Load::Ready(page)) => (
                strings::list(
                    page,
                    current.as_deref(),
                    view.client_override.as_deref(),
                    &keys,
                ),
                2,
                false,
            ),
        };
        let picker: AnyWidget = AnyWidget::from(Picker::new(Rc::new(list), cx.map(Msg::Picked)))
            .with_key(Key::value(phase));
        if saving {
            Interest::new(Wanted::Saving, picker).into()
        } else {
            picker
        }
    }
}
