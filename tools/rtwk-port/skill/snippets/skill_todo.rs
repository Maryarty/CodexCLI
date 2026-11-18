// skill: app
// src/todo.rs
use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::controls::{KeyHintsBar, SelectionList};
use rtwk::data::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::KeyBindings;
use rtwk::layout::{Border, BorderTitle, BorderType, BoxWidget, Column, CrossAxisAlignment, Expanded};
use rtwk::style::Style;
use rtwk::widgets::Text;

/// A to-do list: Enter adds the task typed, Tab goes to the list, Space
/// marks a task done there, Ctrl+Q quits.
pub struct Todo {
    pub tasks: Vec<String>,
    // controllers are shared handles: kept in the model, they outlive every view
    text: TextEditingController,
    done: SelectionController<usize>,
}

impl Default for Todo {
    fn default() -> Self {
        Todo {
            tasks: Vec::new(),
            text: TextEditingController::new(""),
            done: SelectionController::with(true, [], None),
        }
    }
}

pub fn view(app: &Todo, cx: &mut ViewCx<'_, '_, Todo>) -> AnyWidget {
    let tasks: Rc<[String]> = app.tasks.clone().into();
    let list = SelectionList::new(tasks.len(), |i| i, move |i| Text::new(tasks[i].clone()).into())
        .controller(app.done.clone());
    let add = cx.on_with(|app: &mut Todo, task: Rc<str>, _| {
        if task.trim().is_empty() {
            return;
        }
        app.tasks.push(task.trim().to_owned());
        app.text.clear();
    });
    let body = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Expanded::new(
            1,
            BoxWidget::new()
                .border(Border::all(Style::new()).kind(BorderType::Rounded))
                .title(BorderTitle::new(" To do "))
                .child(list),
        ))
        .child(
            TextField::new()
                .controller(app.text.clone())
                .placeholder("a task, Enter adds it")
                .on_submitted(add)
                .autofocus(true),
        )
        .child(KeyHintsBar::new());
    // not 'q': the focused field would take it as text
    KeyBindings::new([("ctrl+q", cx.on(|_, cx| cx.quit()))], body)
        .hints([("ctrl+q", "quit")])
        .into()
}
// skill: test
// src/todo.rs, at its end
#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester, find};

    #[test]
    fn enter_adds_the_task_typed_space_in_the_list_marks_it_done() {
        let mut t = Tester::with(Options::new().size(30, 6));
        t.pump_app(AppView::new(Todo::default(), view));
        t.type_text("buy milk");
        t.press("enter");
        // pump: Space needs the row that Enter's frame builds
        t.pump();
        t.press("tab space");
        t.pump();
        t.assert_screen(
            "╭ To do ─────────────────────╮\n\
             │▎[x] buy milk               │\n\
             │                            │\n\
             ╰────────────────────────────╯\n\
             a task, Enter adds it\n\
             space toggle   ctrl+q quit",
        );
        assert_eq!(find::text("buy milk").count(&t), 1);
        t.read_app(|app: &Todo| assert_eq!(app.tasks, ["buy milk"]));
    }
}
