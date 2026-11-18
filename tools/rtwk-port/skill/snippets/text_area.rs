// TextArea with a retained controller: the text lives in a `TextEditingController` the model keeps
// (a cheap clone is the handle), so it survives rebuilds, a screen change and a thread switch.
// Read it in a handler with `controller.text()`; write it with `set_text` / `clear`.
// From the rtwk-gallery TextArea page (crates/rtwk-gallery/tests/code/editing_area_submit.rs).
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::editing::{TextArea, TextEditingController};
use rtwk::layout::{Column, CrossAxisAlignment, SizedBox};
use rtwk::widgets::Text;
use rtwk::AnyWidget;

pub struct App {
    pub composer: TextEditingController, // retained: created once, with the model
    pub sent: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        App { composer: TextEditingController::new(""), sent: Vec::new() }
    }
}

pub fn view(app: &App, cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let area = TextArea::new()
        .controller(app.composer.clone())
        .placeholder("message")
        .autofocus(true)
        .min_lines(1)
        .max_lines(3)
        // Enter submits: the handler gets the text; clear the retained controller there
        .on_submitted(cx.on_with(|a, text: Rc<str>, _| {
            a.sent.push(text.to_string());
            a.composer.clear();
        }));
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(SizedBox::new(Some(30), None, Some(area.into())))
        .child(Text::new(format!("sent: {}", app.sent.len())))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    #[test]
    fn enter_submits_the_retained_text() {
        let mut t = Tester::with(Options::new().size(40, 6));
        t.pump_app(AppView::new(App::default(), view));
        t.type_text("hello");
        t.pump_for(Duration::from_millis(50));
        t.read_app(|a: &App| assert_eq!(&*a.composer.text(), "hello"));
        t.press("enter");
        t.pump_for(Duration::from_millis(50));
        t.read_app(|a: &App| {
            assert_eq!(a.sent, ["hello"]);
            assert_eq!(&*a.composer.text(), "");
        });
        t.assert_line("sent: 1");
    }
}
