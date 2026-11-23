//! The sheet of `/apps` (oracle `chatwidget/connectors.rs`): the directory of apps, and the link
//! dialog of the app picked from it (`bottom_pane/app_link_view.rs`), as the routes of a nested
//! `Navigator` (ADR-013 O1, the shape of `/review` and of the `/plugins` pilot). The home is the
//! directory; an app with a page pushes its link dialog over it, an app without one says so (a
//! note in the history) and the sheet closes, as the oracle's list dismisses itself on a pick
//! (`dismiss_on_select`). Esc closes the sheet from either page, the oracle's view stack having
//! nothing left under the dialog.
//!
//! What the directory holds (loading, failed with its Retry row, the apps) is the model's fact
//! the page reads; the page the user is on, the cursor and the dialog's own switch are the
//! sheet's. The model only learns what is a fact for the app: the directory is read again
//! (`Intent::RefreshApps`), an app has no page (`Intent::AppUnlinked`), a page is opened
//! (`Intent::OpenAppPage`), a switch is written (`Intent::SetApp`). The account or the scope
//! changing ends the sheet (`Apps::scope`). In Flutter this is the nested-navigator recipe for a
//! flow inside `showModalBottomSheet`, the directory a `ConsumerStatefulWidget` of a provider that
//! holds the cache.

use std::rc::Rc;

use rtwk::layout::{Align, Alignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::apps::{self, Link};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use super::app_link::AppLinkView;
use super::strings::{self, Strings};
use crate::app::view::Hs;
use crate::widgets::picker::list::{Columns, Item, List, Role, Span};
use crate::widgets::picker::{Event, Picker};

/// The sheet: a navigator whose home is the directory, each page at the bottom edge of it (where
/// the oracle's pane is; `features/plugins/sheet.rs`).
pub fn sheet() -> AnyWidget {
    Navigator::new()
        .home(bottom(Directory))
        .back_key(None)
        .into()
}

/// The sheet of one app's link dialog, with no directory under it (the debug entries of the
/// dialog: a link given, not picked).
pub fn link_sheet(link: Link) -> AnyWidget {
    Navigator::new()
        .home(bottom(AppLinkView { link }))
        .back_key(None)
        .into()
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}

fn bottom(page: impl Into<AnyWidget>) -> AnyWidget {
    Align::new(Alignment::BOTTOM_CENTER, page).into()
}

fn route(link: Link) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(AppLinkView { link: link.clone() }))
}

/// The home page: the directory of apps the model holds.
#[derive(Clone, Debug, PartialEq)]
struct Directory;

struct DirectoryState {
    /// The scope the sheet opened under; another one ends it.
    scope: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
}

impl StatefulWidget for Directory {
    type State = DirectoryState;

    fn create_state(&self) -> DirectoryState {
        DirectoryState { scope: None }
    }
}

impl_widget!(stateful eq Directory);

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, DirectoryState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(intent));
}

impl State for DirectoryState {
    type Widget = Directory;
    type Msg = Msg;

    // The directory is a fact of the model, read in `build`; so is the scope, and the sheet ends
    // with a change of it (the account or the scope of the directory changed: nothing held is
    // trusted, `invalidate_connector_scope`).
    fn did_change_dependencies(&mut self, _: &Directory, cx: &mut Cx<'_, Self>) {
        let scope = select::<ScreenStates, u64, _>(cx, |states| states.select.apps.scope());
        if scope != *self.scope.get_or_insert(scope) {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn update(&mut self, msg: Msg, _: &Directory, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(Event::Choose(id)) if id == apps::RETRY => {
                say(cx, Intent::RefreshApps);
            }
            Msg::Picked(Event::Choose(id)) => {
                // The app is looked up in the directory as it is now.
                let link = read::<ScreenStates, _>(cx)
                    .select
                    .apps
                    .app(&id)
                    .map(apps::App::link);
                match link {
                    Some(Some(link)) => drop(Navigator::of(cx).push::<()>(route(link))),
                    // An app with no page: the note says why, the list is gone.
                    Some(None) => {
                        say(cx, Intent::AppUnlinked(id));
                        close_dialog::<_, ()>(cx, None);
                    }
                    None => {}
                }
            }
            Msg::Picked(Event::Cancel) => close_dialog::<_, ()>(cx, None),
            // The directory has no secondary action, toggles, order or tabs.
            Msg::Picked(_) => {}
        }
        Rebuild::No
    }

    fn build(&self, _: &Directory, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let page = select::<ScreenStates, apps::Page, _>(cx, |states| states.select.apps.page());
        let list = list(&page, strings::of(cx));
        Picker::new(Rc::new(list), cx.map(Msg::Picked)).into()
    }
}

/// The `/apps` directory (`connectors_popup_params`, the loading and error popups of
/// `chatwidget/connectors.rs`).
fn list(page: &apps::Page, s: &Strings) -> List {
    let titled = |subtitle: &str, items: Vec<Item>| List {
        title: Some(s.apps_title.to_string()),
        subtitle: Some(subtitle.to_string()),
        standard_hint: true,
        items,
        ..List::default()
    };
    match page {
        apps::Page::Loading => titled(
            s.apps_loading_subtitle,
            vec![Item {
                id: "loading".to_string(),
                name: s.apps_loading.to_string(),
                description: Some(s.apps_loading_description.to_string()),
                disabled: true,
                ..Item::default()
            }],
        ),
        apps::Page::Failed => titled(
            s.apps_failed_subtitle,
            vec![
                Item {
                    id: "unavailable".to_string(),
                    name: s.apps_unavailable.to_string(),
                    description: Some(s.apps_unavailable_description.to_string()),
                    disabled: true,
                    ..Item::default()
                },
                Item {
                    id: apps::RETRY.to_string(),
                    name: s.apps_retry.to_string(),
                    description: Some(s.apps_retry_description.to_string()),
                    ..Item::default()
                },
            ],
        ),
        apps::Page::Apps {
            installed,
            total,
            rows,
        } => {
            let dim = |text: String| {
                vec![Span {
                    text,
                    role: Role::Dim,
                }]
            };
            List {
                title: Some(s.apps_title.to_string()),
                header: vec![
                    dim(s.apps_mention_hint.to_string()),
                    dim(s.apps_counts(*installed, *total)),
                ],
                standard_hint: true,
                items: rows.iter().map(|app| item(app, s)).collect(),
                searchable: true,
                search_placeholder: Some(s.apps_search.to_string()),
                columns: Columns::AutoAllRows,
                ..List::default()
            }
        }
    }
}

fn item(app: &apps::App, s: &Strings) -> Item {
    let status = app.status();
    let selected = match (app.install_url.is_some(), app.installed) {
        (false, _) => s.app_status_line(s.apps_no_link, status),
        (true, true) => s.app_status_line(s.apps_open_installed, status),
        (true, false) => s.app_status_line(s.apps_open_install, status),
    };
    Item {
        id: app.id.clone(),
        name: app.name.clone(),
        description: Some(match app.about() {
            Some(about) => s.app_brief(status, about),
            None => s.app_status(status).to_string(),
        }),
        selected_description: Some(selected),
        search_value: Some(format!("{} {}", app.name, app.id)),
        ..Item::default()
    }
}
