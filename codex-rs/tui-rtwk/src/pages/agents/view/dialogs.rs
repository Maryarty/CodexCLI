//! Help, the confirm and failure dialogs.

use super::*;

/// The shortcut help in place of the list and the details (oracle:
/// `center_help_lines` and the `help` arm of `render`): three groups flowed
/// into as many columns as the width holds, cut to the room with a note.
pub fn help(look: &Look, strings: &Strings, model: &Model, center: &Center) -> AnyWidget {
    let pair = |entries: &[(&str, &str)]| -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(key, action)| ((*key).to_owned(), (*action).to_owned()))
            .collect()
    };
    // Pinning is offered while the server has pins and a task is selected.
    let can_pin = model.pins_available() && center.selected_row().is_some();
    let tasks: Vec<(&str, &str)> = strings
        .help_entries_tasks
        .iter()
        .copied()
        .filter(|(_, action)| {
            (can_pin || *action != "Pin/unpin") && (model.worktrees || *action != "New worktree")
        })
        .collect();
    let mut view = vec![(
        strings.filter_keys.to_owned(),
        strings.help_filter.to_owned(),
    )];
    view.extend(pair(&strings.help_entries_view));
    let group = |title: &str, entries| help::Group {
        title: title.to_owned(),
        entries,
    };
    let groups = [
        group(strings.help_navigate, pair(&strings.help_entries_navigate)),
        group(strings.help_tasks, pair(&tasks)),
        group(strings.help_view, view),
    ];
    let ink = Ink {
        title: look.bold.clone(),
        key: look.accent.clone(),
        plain: look.plain.clone(),
        dim: look.dim.clone(),
    };
    let (title, resize) = (strings.help_title, strings.help_resize);
    inset(LayoutBuilder::new(move |cx, constraints| {
        let room = |cells: i32| usize::try_from(cells).unwrap_or(0);
        let size = (room(constraints.max_width), room(constraints.max_height));
        help::help(segmenter_of(cx), title, &groups, size, resize, &ink)
    }))
}

/// The name a confirmation asks about: the task's name or its preview, the
/// first non-empty line (oracle: `confirm_agents_overview_action`).
pub fn confirm_name(strings: &Strings, model: &Model, confirm: &Confirm) -> String {
    let row = model.row(&confirm.thread);
    let name = row
        .as_ref()
        .map(|row| row.name.as_deref().unwrap_or(&row.preview))
        .unwrap_or_default();
    name.trim()
        .lines()
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(strings.untitled_task)
        .to_owned()
}

/// A line of a dialog, two cells in from its sides, wrapped by word.
fn note(text: &str, style: &Style) -> AnyWidget {
    inset(
        Text::new(text.to_owned())
            .style(style.clone())
            .wrap(TextWrap::Word),
    )
}

/// A dialog's panel: its rows over a background that clears what lies below
/// (oracle: the centered view clears its area).
fn panel(look: &Look, rows: Vec<AnyWidget>) -> AnyWidget {
    BoxWidget::new()
        .style(look.plain.clone())
        .child(stacked(rows))
        .into()
}

/// What the confirmation asks: the removal, the task's name and whether the
/// archive is the cloud's. They are the question's own text, taken when it
/// is asked: the dialog is a route of the `Navigator`, above the providers
/// of a seeded debug screen, so it does not read the model by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub confirm: Confirm,
    pub name: String,
    pub cloud: bool,
}

/// The archive and delete confirmation: a centered panel over the screen
/// (oracle: `CenteredView` with `SelectionViewParams::confirmation()`): the
/// title and the description wrapped, the two rows, a hint.
pub fn confirm_dialog(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    ask: &Ask,
    options: &Options,
) -> AnyWidget {
    let name = &ask.name;
    let (title, description, label) = match ask.confirm.action {
        Action::Archive if ask.cloud => (
            strings.archive_title(name, num),
            strings.archive_description_cloud,
            strings.archive_label_cloud,
        ),
        Action::Archive => (
            strings.archive_title(name, num),
            strings.archive_description,
            strings.archive_label,
        ),
        Action::Delete => (
            strings.delete_title(name, num),
            strings.delete_description,
            strings.delete_label,
        ),
    };
    let choices = options.view(
        Rc::new(vec![
            OptionRow {
                label: format!("1. {}", strings.cancel),
                description: None,
            },
            OptionRow {
                label: format!("2. {label}"),
                description: None,
            },
        ]),
        OptionStyles {
            cursor: look.accent.clone().attrs(Attrs::BOLD),
            label: look.plain.clone(),
            description: look.plain.clone(),
        },
        0,
    );
    let blank = || gap(1);
    let rows = vec![
        blank(),
        note(&title, &look.bold),
        note(description, &look.dim),
        blank(),
        blank(),
        choices,
        blank(),
        note(strings.select_hint, &look.dim),
    ];
    // The panel is 72 cells wide at most and two cells in from the screen.
    inset(Middle::new(Dialog::bare(panel(look, rows)).max_width(72)))
}

/// The dialog of a refused removal (oracle: the `LifecycleHeader` selection
/// view of `run_agents_overview_action`): the verb in bold red, the server's
/// error in red, the note that work may have stopped when the request was
/// sent, and one row, `Back to agents`. It is the screen's whole content.
pub fn failure_dialog(look: &Look, strings: &Strings, failure: &Failure) -> AnyWidget {
    let title = match failure.action {
        Action::Archive => strings.failure_archive,
        Action::Delete => strings.failure_delete,
    };
    let accent = look.accent.clone().attrs(Attrs::BOLD);
    let mut rows = vec![
        note(title, &look.red.clone().attrs(Attrs::BOLD)),
        note(&failure.error, &look.red),
    ];
    if failure.attempted {
        rows.push(note(strings.failure_note, &look.dim));
    }
    rows.extend([
        gap(1),
        gap(1),
        one(vec![span(
            format!("\u{203a} 1. {}", strings.back_to_agents),
            &accent,
        )]),
        gap(1),
        note(strings.select_hint, &look.dim),
    ]);
    Dialog::bare(panel(look, rows)).into()
}
