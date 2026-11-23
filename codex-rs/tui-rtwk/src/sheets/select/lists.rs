//! The named parameterisations as the generic list: the permissions menu and the apps
//! directory are data in the model (`Menu`), their words are here (ADR-010). The `/model`
//! family is the sheet of its own (`models.rs`, `models/list.rs`).

use crate::widgets::picker::list::{Columns, Item, List, Role, Span};
use codex_tui_core::screens::select::apps;
use codex_tui_core::screens::select::permissions::{
    self, Disabled, Notice, Preset, Recommend, Reviewer,
};
use codex_tui_core::screens::select::{Confirm, Menu, Page};

use super::strings::Strings;

/// The list the menu on top of the stack is shown as; `page` is its rows when
/// it is a menu of a typed family (`Model::page`).
pub fn of(menu: &Menu, page: Option<&Page>, s: &Strings) -> Option<List> {
    Some(match (menu, page) {
        (Menu::Permissions(_), Some(Page::Permissions(page))) => permissions(page, s),
        (Menu::Apps, Some(Page::Apps(page))) => apps(page, s),
        _ => return None,
    })
}

/// The `/apps` directory (`connectors_popup_params`, the loading and error
/// popups of `chatwidget/connectors.rs`).
fn apps(page: &apps::Page, s: &Strings) -> List {
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
                items: rows.iter().map(|app| app_item(app, s)).collect(),
                searchable: true,
                search_placeholder: Some(s.apps_search.to_string()),
                columns: Columns::AutoAllRows,
                ..List::default()
            }
        }
    }
}

fn app_item(app: &apps::App, s: &Strings) -> Item {
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

/// A menu of the `/permissions` family (`permissions_menu.rs`,
/// `permission_popups.rs`, `permission_discovery.rs`).
fn permissions(page: &permissions::Page, s: &Strings) -> List {
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
        items: page
            .rows
            .iter()
            .map(|row| permission_item(row, s))
            .collect(),
        ..List::default()
    }
}

/// The header under `Enable full access?` (`open_full_access_confirmation`):
/// one paragraph with the caution in red; a cyber model's has blank rows
/// between its sentences and its first sentence in red.
fn full_access_header(cyber: Option<&Recommend>, s: &Strings) -> Vec<Vec<Span>> {
    let run = |text: &str, role| Span {
        text: text.to_string(),
        role,
    };
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

fn permission_item(row: &permissions::Row, s: &Strings) -> Item {
    let words = |name: &str, description: &str| (name.to_string(), Some(description.to_string()));
    let (name, description) = match &row.kind {
        permissions::Kind::Mode { preset, reviewer } => match (preset, reviewer) {
            (Preset::Auto, Reviewer::AutoReview) => {
                words(s.mode_approve, s.mode_approve_description)
            }
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

/// The two rows of a confirmation, No first (`SelectionViewParams::confirmation`:
/// the panel picker with the standard hint, its columns placed from all rows
/// and the description hidden when narrow).
pub fn confirmation(confirm: Confirm, s: &Strings) -> List {
    let (title, subtitle, no, yes) = match confirm {
        Confirm::Archive { running } => (
            s.archive_title,
            if running {
                s.archive_subtitle_running
            } else {
                s.archive_subtitle
            },
            (s.archive_no, s.archive_no_description),
            (s.archive_yes, s.archive_yes_description),
        ),
        Confirm::Delete => (
            s.delete_title,
            s.delete_subtitle,
            (s.delete_no, s.delete_no_description),
            (s.delete_yes, s.delete_yes_description),
        ),
    };
    let row = |id: &str, (name, description): (&str, &str)| Item {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        ..Item::default()
    };
    List {
        title: Some(title.to_string()),
        subtitle: Some(subtitle.to_string()),
        standard_hint: true,
        items: vec![row(Confirm::NO, no), row(Confirm::YES, yes)],
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
