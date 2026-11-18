// ListView::builder over a model: rows are built as they scroll into view; a row keeps its state
// by a key made from its id, so it survives inserts above it. `stick_to_bottom` follows the newest
// row until the user scrolls up (a transcript).
// From crates/rtwk/SKILL.md ("Layout, focus, and a form") and the rtwk-gallery ListView page.
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::layout::SizedBox;
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Key};

#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: u32,
    pub text: String,
}

#[derive(Default)]
pub struct App {
    pub rows: Rc<Vec<Row>>, // cheap to hand to the builder closure; `Rc::make_mut` on write
}

pub fn view(app: &App, _cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let rows = app.rows.clone();
    let list = ListView::builder(rows.len(), move |i| {
        let row = &rows[i];
        // the key follows the row, not its index
        AnyWidget::from(Text::new(format!("{}: {}", row.id, row.text))).with_key(Key::value(row.id))
    })
    .item_extent(1) // every row one cell high: no measuring, a true scroll extent
    .stick_to_bottom(true);
    // a list takes all the room it gets: give it a height
    SizedBox::new(None, Some(3), Some(list.into())).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    #[test]
    fn the_list_follows_the_newest_row() {
        let mut t = Tester::with(Options::new().size(20, 3));
        t.pump_app(AppView::new(App::default(), view));
        for id in 1..=5 {
            t.update_app(move |a: &mut App, _| Rc::make_mut(&mut a.rows).push(Row { id, text: format!("line {id}") }));
            t.pump_for(Duration::from_millis(50));
        }
        t.assert_screen("3: line 3\n4: line 4\n5: line 5");
    }
}
