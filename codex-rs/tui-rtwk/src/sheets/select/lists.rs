//! The named parameterisations as the generic list: the `/model` family and the apps directory
//! are data in the model (`Menu`), their words are here (ADR-010). (The permission menus are a
//! sheet of their own: `permissions.rs`.)

use crate::widgets::picker::list::{Columns, Item, List, Role, Secondary, Span};
use codex_tui_core::screens::select::models::{
    self, Apply, Choice, Effort, Heading, Kind, PlanSource,
};
use codex_tui_core::screens::select::{Confirm, Menu, Page};

use super::strings::Strings;

/// The key of the secondary action of a model row (`session_model_selection_action`).
const SESSION_KEY: char = 's';

/// The list the menu on top of the stack is shown as; `page` is its rows when
/// it is a menu of a typed family (`Model::page`).
pub fn of(menu: &Menu, page: Option<&Page>, s: &Strings) -> Option<List> {
    Some(match (menu, page) {
        (Menu::Model(_), Some(Page::Model(page))) => models(page, s),
        _ => return None,
    })
}

/// A menu of the `/model` family: the heading says what it is, each row what
/// it does (`model_popups.rs`).
fn models(page: &models::Page, s: &Strings) -> List {
    let (title, subtitle, role) = heading(&page.heading, s);
    List {
        title: Some(title),
        subtitle,
        subtitle_role: role,
        standard_hint: true,
        items: page
            .rows
            .iter()
            .map(|row| item(row, &page.heading, s))
            .collect(),
        initial: page.initial,
        numbered_rows: Some(9),
        ..List::default()
    }
}

fn heading(heading: &Heading, s: &Strings) -> (String, Option<String>, Role) {
    let plain = |title: &str, subtitle: Option<String>| (title.to_string(), subtitle, Role::Plain);
    match heading {
        Heading::Loading => plain(s.loading, None),
        Heading::Quick => plain(s.quick_title, Some(s.quick_subtitle.to_string())),
        Heading::All => plain(s.model_title, None),
        Heading::Reserve => plain(s.quick_title, Some(s.reserve_subtitle.to_string())),
        Heading::Reasoning { model } => (s.reasoning_title(model), None, Role::Plain),
        Heading::Advanced => (
            s.advanced_title.to_string(),
            Some(s.advanced_warning.to_string()),
            Role::Info,
        ),
        Heading::PlanScope { effort, .. } => (
            s.scope_title.to_string(),
            Some(s.scope_subtitle(&s.reasoning_phrase(effort.as_ref()))),
            Role::Plain,
        ),
    }
}

fn item(row: &Choice, heading: &Heading, s: &Strings) -> Item {
    let text = |text: String| Some(text).filter(|text| !text.is_empty());
    let (name, description, selected) = match &row.kind {
        Kind::Model => (row.name.clone(), text(row.description.clone()), None),
        Kind::AllModels => (
            s.all_models.to_string(),
            Some(s.all_models_description(&row.description)),
            None,
        ),
        Kind::Effort {
            effort,
            default,
            warns,
        } => {
            let mut name = s.effort(effort);
            if *default {
                name.push_str(s.default_mark);
            }
            let description = match heading {
                Heading::Advanced => Some(advanced_description(effort, s)),
                _ => text(row.description.clone()),
            };
            // The highlighted row says what a high effort costs, under the
            // model's own words.
            let selected = warns.then(|| {
                let warning = s.rate_limit_warning(effort);
                match &description {
                    Some(description) => format!("{description}\n{warning}"),
                    None => warning,
                }
            });
            (name, description, selected)
        }
        Kind::More(efforts) => (
            s.more_reasoning.to_string(),
            Some(s.more_description(efforts)),
            None,
        ),
        Kind::Scope(apply) => {
            let Heading::PlanScope { effort, source } = heading else {
                return Item {
                    id: row.id.clone(),
                    ..Item::default()
                };
            };
            match apply {
                Apply::AllModes => {
                    let source = source_phrase(source, s);
                    (
                        s.scope_all_modes.to_string(),
                        Some(s.scope_all_modes_description(&source)),
                        None,
                    )
                }
                _ => {
                    let phrase = s.reasoning_phrase(effort.as_ref());
                    (
                        s.scope_plan_only.to_string(),
                        Some(s.scope_plan_only_description(&phrase)),
                        None,
                    )
                }
            }
        }
    };
    Item {
        id: row.id.clone(),
        name,
        description,
        selected_description: selected,
        current: row.current,
        secondary: row.session.then(|| session(row.effort.as_ref(), s)),
        ..Item::default()
    }
}

/// The footer of a row with the `s` key: `enter default · s session · esc
/// back`, `apply` for Ultra (`session_model_selection_action`).
fn session(effort: Option<&Effort>, s: &Strings) -> Secondary {
    let accept = if effort == Some(&Effort::Ultra) {
        s.word_apply
    } else {
        s.word_default
    };
    Secondary {
        key: SESSION_KEY,
        hint: s.word_session.to_string(),
        accept: Some(accept.to_string()),
    }
}

fn advanced_description(effort: &Effort, s: &Strings) -> String {
    match effort {
        Effort::Ultra => s.ultra_description.to_string(),
        _ => s.max_description.to_string(),
    }
}

/// `plan_reasoning_source`: where the Plan default the prompt replaces comes from.
fn source_phrase(source: &PlanSource, s: &Strings) -> String {
    match source {
        PlanSource::Override(effort) => s.source_override(&s.effort_sentence(effort)),
        PlanSource::Default(Some(effort)) => s.source_default(&s.effort_sentence(effort)),
        PlanSource::Default(None) => s.source_default_none.to_string(),
        PlanSource::Unknown => s.source_unknown.to_string(),
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
