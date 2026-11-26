//! The footer, the editor and the page body.

use super::*;

/// What the footer shows: the notice that replaces the hints, or the hints
/// (key, label).
pub enum Footer<'a> {
    Notice(Line),
    Hints(Vec<(String, &'a str)>),
}

/// The footer: the hints, or the notice that replaces them.
pub fn footer<'a>(
    look: &Look,
    strings: &'a Strings,
    hints: &Hints,
    model: &Model,
    ui: &Ui,
    can_return: bool,
) -> Footer<'a> {
    let notice = match &model.notice {
        Some(Notice::Reconnecting) => Some(strings.reconnecting.to_owned()),
        Some(Notice::ReconnectFailed) => Some(strings.reconnect_failed.to_owned()),
        Some(Notice::Removing(Action::Archive)) => Some(strings.archiving.to_owned()),
        Some(Notice::Removing(Action::Delete)) => Some(strings.deleting.to_owned()),
        Some(Notice::CreatingWorktree) => Some(strings.creating_worktree.to_owned()),
        Some(Notice::WorktreeRetained { root, reason }) => {
            Some(strings.retained_worktree(root, reason))
        }
        Some(Notice::LoadFailed) => Some(strings.load_failed.to_owned()),
        Some(Notice::TrustCheckFailed(error)) => Some(
            crate::oracle_strings::live::RESUME_CONFIG_UNABLE_TO_CHECK_FOLDER_TRUST
                .replace("{error}", error),
        ),
        Some(Notice::Error(text)) => Some(text.clone()),
        None => {
            let failed =
                model.paging.failed || model.history.as_ref().is_some_and(|history| history.failed);
            (failed && !model.paging.loading)
                .then(|| strings.load_failed.to_owned())
                .or_else(|| model.server_version.clone())
        }
    };
    if let Some(text) = notice.filter(|_| !ui.help) {
        return Footer::Notice(vec![span(text, &look.dim)]);
    }
    // The oracle's `center_footer_hints`: a hint is there while its action is
    // bound, and the list's keys a task action took are not offered.
    let editing = ui.editing();
    let mut items: Vec<(String, &str)> = Vec::new();
    if !ui.help && !editing && hints.help_is_offered() {
        items.push((strings.key_help.to_owned(), strings.help));
    }
    let back = if ui.help {
        hints.list_any(LIST_CANCEL)
    } else if editing || can_return {
        hints.list(LIST_CANCEL)
    } else {
        None
    };
    if let Some(key) = back {
        items.push((key, strings.back));
    }
    if ui.help {
        return Footer::Hints(items);
    }
    if !editing {
        let movement = hints.movement();
        if !movement.is_empty() {
            items.push((movement, strings.move_));
        }
    }
    if let Some(key) = hints.list(LIST_ACCEPT) {
        items.push((
            key,
            if ui.renaming {
                strings.rename
            } else {
                strings.open
            },
        ));
    }
    if !editing && let Some(key) = hints.task(AGENTS_NEW_TASK) {
        items.push((key, strings.new));
    }
    Footer::Hints(items)
}

/// The search or rename row above the list: the label and a `TextField`
/// that edits the controller's text (a search tells the model each change, a
/// rename its text on Enter).
pub fn editor(look: &Look, strings: &Strings, ui: &Ui, controls: &Controls) -> AnyWidget {
    let label = if ui.renaming {
        strings.rename_label
    } else {
        strings.search_label
    };
    let label_style = look.cyan.clone().attrs(Attrs::BOLD);
    let mut field = TextField::new()
        .controller(controls.edit.clone())
        .style(look.tab.clone())
        .focused_style(look.tab.clone())
        .key_hints(false)
        .focus_node(controls.edit_focus.clone())
        .on_submitted(controls.submit.clone());
    if !ui.renaming {
        field = field.on_changed(controls.search.clone());
    }
    let keys = if ui.renaming {
        &controls.keys.rename
    } else {
        &controls.keys.search
    };
    let row = Row::new()
        .child(Text::line(label).style(label_style))
        .child(Expanded::new(1, KeyBindings::new(keys.clone(), field)));
    BoxWidget::new()
        .style(look.tab.clone())
        .child(SizedBox::new(None, Some(1), Some(row.into())))
        .into()
}

/// Everything below the header and above the footer, for the unfiltered
/// frame: the list, the gap and the details.
#[allow(clippy::too_many_arguments)]
pub fn body(
    look: &Look,
    strings: &'static Strings,
    num: &NumSyms,
    agents: &Shared<Model>,
    ui: &Ui,
    center: &Rc<Center>,
    controls: &Controls,
    taps: &Taps,
) -> AnyWidget {
    let editing = ui.editing();
    let mut pane = Column::new()
        .overflow_intended(true)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    // The line stays mounted, offstage and out of the focus while closed:
    // opening and closing swap which of the two the focus may rest on.
    pane = pane.child(Offstage::new(
        !editing,
        ExcludeFocus::new(editor(look, strings, ui, controls)).excluding(!editing),
    ));
    let rows = list(look, strings, agents, center, controls, &taps.rows);
    let pane: AnyWidget = pane
        // A rename refuses the mouse at the list. The list stays a focus
        // stop (not `enabled(false)`, not `ExcludeFocus`): the focus goes
        // back to the nearest stop when the line closes.
        .child(Expanded::new(
            1,
            ExcludeFocus::new(
                IgnorePointer::new()
                    .ignoring(ui.renaming)
                    .child(KeyBindings::new(controls.keys.browse.clone(), rows)),
            )
            .excluding(editing),
        ))
        .into();
    let details = details(look, strings, num, center, agents);
    let rule = VerticalDivider::new()
        .kind(BorderType::Single)
        .style(look.dim.clone())
        .merge(false);
    let rule = SizedBox::new(
        Some(GAP_WIDTH),
        None,
        Some(Padding::new(EdgeInsets::only(1, 0, 1, 0), rule).into()),
    );
    inset(LayoutBuilder::new(move |_, constraints| {
        if constraints.max_width < TWO_PANES {
            return pane.clone();
        }
        Row::new()
            .overflow_intended(true)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Expanded::new(1, pane.clone()))
            .child(rule.clone())
            .child(SizedBox::new(
                Some(DETAILS_WIDTH as i32),
                None,
                Some(details.clone()),
            ))
            .into()
    }))
}

/// The footer row, inset two cells.
pub fn footer_row(
    look: &Look,
    strings: &Strings,
    hints: &Hints,
    model: &Model,
    ui: &Ui,
    can_return: bool,
) -> AnyWidget {
    match footer(look, strings, hints, model, ui, can_return) {
        Footer::Notice(line) => inset(one(line)),
        // The hints are the page's key list on the shared bottom menu
        // (`ui::page_keys::footer`); each key is one `KeyBindings` binds.
        Footer::Hints(items) => inset(crate::widgets::page_keys::footer(
            items
                .into_iter()
                .map(|(key, label)| (key, label.to_owned())),
            look.plain.clone(),
            look.muted.clone(),
        )),
    }
}
