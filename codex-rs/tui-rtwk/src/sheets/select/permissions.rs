//! The sheet of `/permissions` (oracle `chatwidget/permission_popups.rs`, `permissions_menu.rs`,
//! `permission_discovery.rs`): the legacy presets, or the server's catalog (the wait for it, the
//! built-in modes and profiles, or the `Retry` page of a read that failed), and the question
//! `Enable full access?` a pick of full access asks first (ADR-013 O1).
//!
//! The pages are the routes of a nested `Navigator`, the menu each shows its widget's own: the
//! one the sheet opened with, replaced by the one a pick leads to (the oracle dismisses the popup
//! and opens the confirmation as a new view, so Esc on it just closes, and a declined
//! confirmation opens the permissions menu again from the start). Which row leads where is the
//! model's pure `PermissionsMenu::choose`; what a pick means to the app is a verb
//! (`SetPermissions`, `LoadProfiles`, `AbandonPermissions`). What the model holds the page reads
//! as facts through one projection: its rows (the catalog of profiles and the model catalog are
//! the model's, the answer of the server arrives while the sheet is open), the scope the catalog
//! was read under (a change dismisses the pages that wait on it or show it,
//! `invalidate_permission_discovery`) and whether an inline `/permissions <profile>` pick waits
//! on the sheet (the catalog takes it, and the sheet that showed the wait is gone). In Flutter
//! this is the `showModalBottomSheet` builder of a step-by-step flow, each step a
//! `ConsumerStatefulWidget` that `ref.watch`es `provider.select(..)` and `pushReplacement`s the
//! next.

use std::rc::Rc;

use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::screens::select::permissions::{
    self, Chosen, Disabled, Notice, Page, PermissionsMenu, Phase, Preset, Recommend, Reviewer,
};
use codex_tui_core::screens::select::{Intent, Opening};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use super::centered;
use super::strings::{self, Strings};
use crate::app::debug::seeded::Seeded;
use crate::app::view::Hs;
use crate::widgets::picker::list::{Item, List, Role, Span};
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

/// The sheet of a menu the model asked for (`Opening::Permissions`): a navigator as tall as the
/// sheet's cap whose home is the page of the menu, each page at the bottom edge of it (where the
/// oracle's pane is; `features/plugins/sheet.rs`).
pub fn sheet(menu: PermissionsMenu) -> AnyWidget {
    Navigator::new().home(placed(menu)).back_key(None).into()
}

/// A page where the oracle shows its view: the confirmation (`SelectionViewParams::confirmation()`,
/// `ViewPresentation::Centered`) floats in the middle over the composer, the menus sit at the
/// bottom edge.
fn placed(menu: PermissionsMenu) -> AnyWidget {
    if matches!(menu, PermissionsMenu::FullAccess { .. }) {
        centered(Step { menu })
    } else {
        bottom(Step { menu })
    }
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}

/// The sheet of the first permission menu a seeded model has asked for (a debug entry).
pub fn seeded(seeded: &Seeded<'_>) -> AnyWidget {
    seeded
        .core
        .screens()
        .select
        .shows
        .pending()
        .find_map(|show| match &show.opening {
            Opening::Permissions(menu) => Some(sheet(menu.clone())),
            _ => None,
        })
        .unwrap_or_default()
}

fn route(menu: PermissionsMenu) -> PageRoute<()> {
    PageRoute::new(move |_| placed(menu.clone()))
}

/// One page of the sheet: the menu it shows.
#[derive(Clone, Debug, PartialEq)]
struct Step {
    menu: PermissionsMenu,
}

struct StepState {
    /// The scope the catalog was first seen under (the page began under it).
    scope: Option<u64>,
    /// An inline pick waited on the catalog while the page showed the wait: once the catalog
    /// came and took the pick, the sheet is gone.
    waited_inline: bool,
    closed: bool,
}

/// What the page reads of the model.
#[derive(Clone, Debug, PartialEq)]
struct Facts {
    page: Page,
    phase: Phase,
    scope: u64,
    /// An inline pick waits on the catalog (`Profiles::inline_waits`).
    inline: bool,
    /// The non-admin Windows sandbox is on: `Ask for approval` says so, and the page hints at
    /// `/setup-default-sandbox` when the organization allows the admin one
    /// (`windows_degraded_sandbox_enabled`, `show_elevate_sandbox_hint`).
    sandbox: Sandbox,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Sandbox {
    degraded: bool,
    elevate_hint: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
}

impl StatefulWidget for Step {
    type State = StepState;

    fn create_state(&self) -> StepState {
        StepState {
            scope: None,
            waited_inline: false,
            closed: false,
        }
    }
}

impl_widget!(stateful eq Step);

impl StepState {
    /// The projection both `did_change_dependencies` and `build` register: the same selector, so
    /// whichever ran last leaves the registration right.
    fn facts(&self, w: &Step, cx: &mut Cx<'_, Self>) -> Facts {
        let menu = w.menu.clone();
        let (page, phase, scope, inline) = select::<ScreenStates, _, _>(cx, move |states| {
            let select = &states.select;
            (
                menu.page(&select.profiles, &select.catalog),
                menu.phase(&select.profiles),
                select.profiles.scope,
                select.profiles.inline_waits(),
            )
        });
        let sandbox = select::<FeatureStates, Sandbox, _>(cx, |features| Sandbox {
            degraded: features.windows_sandbox.degraded(),
            elevate_hint: features.windows_sandbox.elevate_hint(),
        });
        Facts {
            page,
            phase,
            scope,
            inline,
            sandbox,
        }
    }

    fn close(&mut self, cx: &mut Cx<'_, Self>) {
        if !self.closed {
            self.closed = true;
            close_dialog::<_, ()>(cx, None);
        }
    }
}

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, StepState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(intent));
}

impl State for StepState {
    type Widget = Step;
    type Msg = Msg;

    // The pages of the catalog go when the scope changes, and the sheet that showed the wait of
    // an inline pick goes when the catalog took the pick. Esc, a pick and the rest are `update`.
    fn did_change_dependencies(&mut self, w: &Step, cx: &mut Cx<'_, Self>) {
        let facts = self.facts(w, cx);
        let scope = *self.scope.get_or_insert(facts.scope);
        if !matches!(w.menu, PermissionsMenu::Catalog(_)) {
            return;
        }
        let gone = facts.scope != scope
            || (facts.phase == Phase::Listed && self.waited_inline && !facts.inline);
        if gone {
            self.close(cx);
        } else if facts.phase == Phase::Loading {
            self.waited_inline |= facts.inline;
        }
    }

    fn update(&mut self, msg: Msg, w: &Step, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let profiles = read::<ScreenStates, _>(cx).select.profiles.clone();
                match w.menu.choose(&profiles, &id) {
                    Some(Chosen::Done(selection)) => {
                        say(cx, Intent::SetPermissions(selection));
                        self.close(cx);
                    }
                    // The page that replaces this one begins under the scope of now.
                    Some(Chosen::Menu(next)) => {
                        drop(Navigator::of(cx).push_replacement::<()>(route(next)));
                    }
                    Some(Chosen::Retry) => say(cx, Intent::LoadProfiles),
                    None => {}
                }
            }
            // Esc leaves the sheet; an inline pick that waited on it is given up.
            Event::Cancel => {
                say(cx, Intent::AbandonPermissions);
                self.close(cx);
            }
            // The permission lists have no secondary action, toggles, order or tabs.
            Event::Secondary(_)
            | Event::Toggle(_)
            | Event::Reorder(..)
            | Event::Confirm
            | Event::Tab(_)
            | Event::Hot(_) => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Step, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let facts = self.facts(w, cx);
        let words = strings::of(cx);
        let picker: AnyWidget = Picker::new(
            Rc::new(list(&facts.page, facts.sandbox, words)),
            cx.map(Msg::Picked),
        )
        .into();
        // The catalog that came is a new picker with its own cursor; a refresh of the same page
        // keeps cursor and scroll.
        picker.with_key(shape(facts.phase))
    }
}

/// Which page of the family a phase is.
fn shape(phase: Phase) -> u32 {
    match phase {
        Phase::Presets => 1,
        Phase::Loading => 2,
        Phase::Listed => 3,
        Phase::Failed => 4,
        Phase::Confirm => 5,
    }
}

/// A menu of the `/permissions` family (`permissions_menu.rs`,
/// `permission_popups.rs`, `permission_discovery.rs`).
fn list(page: &Page, sandbox: Sandbox, s: &Strings) -> List {
    let (title, subtitle, header) = match &page.heading {
        permissions::Heading::Permissions { notice } => (
            s.permissions_title.to_string(),
            notice.as_ref().map(|notice| match notice {
                Notice::NoProfiles => s.permissions_no_profiles.to_string(),
                Notice::Failed(failure) => s.failure(failure),
            }),
            Vec::new(),
        ),
        permissions::Heading::FullAccess { cyber } => (
            s.full_title.to_string(),
            None,
            full_access_header(cyber.as_ref(), s),
        ),
    };
    List {
        title: Some(title),
        subtitle,
        header,
        standard_hint: true,
        items: page.rows.iter().map(|row| item(row, sandbox, s)).collect(),
        footer_note: (sandbox.elevate_hint && page.rows.iter().any(is_mode_row))
            .then(|| s.elevate_hint.to_owned()),
        ..List::default()
    }
}

/// The header under `Enable full access?` (`open_full_access_confirmation`):
/// one paragraph with the caution in red; a cyber model's has blank rows
/// between its sentences and its first sentence in red.
fn full_access_header(cyber: Option<&Recommend>, s: &Strings) -> Vec<Vec<Span>> {
    let run = |text: &str, role| Span::new(text, role);
    match cyber {
        None => vec![vec![
            run(s.full_warning, Role::Plain),
            run(s.full_caution, Role::Danger),
        ]],
        Some(recommend) => vec![
            Vec::new(),
            vec![run(s.full_warning.trim_end(), Role::Plain)],
            Vec::new(),
            vec![
                run(s.cyber_risk, Role::Danger),
                run(s.cyber_safeguards, Role::Plain),
                run(
                    match recommend {
                        Recommend::AutoReview => s.cyber_recommend_auto,
                        Recommend::Ask => s.cyber_recommend_ask,
                    },
                    Role::Plain,
                ),
            ],
        ],
    }
}

/// A row of the legacy presets (`Ask for approval`, `Read Only`, ...): the page that hints at the
/// elevated setup is theirs, not the full access question's.
fn is_mode_row(row: &permissions::Row) -> bool {
    matches!(row.kind, permissions::Kind::Mode { .. })
}

fn item(row: &permissions::Row, sandbox: Sandbox, s: &Strings) -> Item {
    let words = |name: &str, description: &str| (name.to_string(), Some(description.to_string()));
    let (name, description) = match &row.kind {
        permissions::Kind::Mode { preset, reviewer } => match (preset, reviewer) {
            (Preset::Auto, Reviewer::AutoReview) => {
                words(s.mode_approve, s.mode_approve_description)
            }
            (Preset::Auto, _) if sandbox.degraded => (
                s.mode_ask_non_admin.to_owned(),
                Some(s.mode_ask_description.to_string()),
            ),
            (Preset::Auto, _) => words(s.mode_ask, s.mode_ask_description),
            (Preset::FullAccess, _) => words(s.mode_full, s.mode_full_description),
            (Preset::ReadOnly, _) => words(s.mode_read, s.mode_read_description),
        },
        permissions::Kind::Profile { description } => (
            row.name.clone(),
            Some(
                description
                    .clone()
                    .unwrap_or_else(|| s.profile_configured.to_string()),
            ),
        ),
        permissions::Kind::Unlisted => (row.name.clone(), Some(s.profile_unlisted.to_string())),
        permissions::Kind::Loading => (s.permissions_loading.to_string(), None),
        permissions::Kind::Retry => (s.permissions_retry.to_string(), None),
        permissions::Kind::Continue => words(s.full_continue, s.full_continue_description),
        permissions::Kind::Cancel => words(s.full_cancel, s.full_cancel_description),
    };
    let reason = match &row.disabled {
        Some(Disabled::NotOnServer) => Some(s.not_on_server.to_string()),
        Some(Disabled::Requirements) => Some(s.disabled_by_requirements.to_string()),
        Some(Disabled::Reason(text)) => Some(text.clone()),
        Some(Disabled::Idle) | None => None,
    };
    Item {
        id: row.id.clone(),
        name,
        description,
        current: row.current,
        disabled: matches!(row.disabled, Some(Disabled::Idle)),
        disabled_reason: reason,
        ..Item::default()
    }
}
