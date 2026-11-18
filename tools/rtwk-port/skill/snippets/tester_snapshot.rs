// A screen test with a snapshot: the app on an in-memory terminal at a given size, input as keys,
// the screen compared with the expected text. `assert_screen` takes the rows from the top without
// trailing blanks or empty rows (what `insta::assert_snapshot!` of `screen()` would store); for a
// stored expected file use `assert_screen_file`. Pump after input, and move the fake clock with
// `pump_for` past animations (a dialog fades in over ~160 ms).
// From crates/rtwk/SKILL.md ("App state, handlers, and testing") and rtwk-weather's tests.
use rtwk::app::ViewCx;
use rtwk::controls::Button;
use rtwk::layout::{Border, BorderType, BoxWidget, Column, CrossAxisAlignment};
use rtwk::widgets::Text;
use rtwk::AnyWidget;

#[derive(Default)]
pub struct Counter {
    pub count: u32,
}

pub fn view(app: &Counter, cx: &mut ViewCx<'_, '_, Counter>) -> AnyWidget {
    let body = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(format!("Count: {}", app.count)))
        .child(Button::new("Increment").autofocus(true).on_pressed(cx.on(|a, _| a.count += 1)));
    BoxWidget::new()
        .border(Border::all(rtwk::style::Style::new()).kind(BorderType::Rounded))
        .child(body)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};

    #[test]
    fn enter_on_the_focused_button_counts() {
        let mut t = Tester::with(Options::new().size(18, 4));
        t.pump_app(AppView::new(Counter::default(), view));
        t.press("enter");
        t.pump();
        t.assert_screen(
            "╭────────────────╮\n\
             │Count: 1        │\n\
             │ Increment      │\n\
             ╰────────────────╯",
        );
        t.read_app(|a: &Counter| assert_eq!(a.count, 1));
    }
}
