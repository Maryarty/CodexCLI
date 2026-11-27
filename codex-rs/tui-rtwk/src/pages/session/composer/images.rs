//! The editable image references stay in the field. This strip shows the visible local
//! attachments in text order and releases the reads when the strip leaves the layout.

use std::rc::Rc;

use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::ExcludeSelection;
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, Row, SizedBox};
use rtwk::provider::Provider;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Key, LayoutBuilder};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_rich::image::Image;

use codex_tui_core::app::effects::ImageSize;
use codex_tui_core::ids::ThreadId;

use crate::app::theme::Tokens;
use crate::features::attachment_thumbnail;
use crate::widgets::thread_ui::ThreadUi;

const TILE_COLS: i32 = 6;
const TILE_ROWS: i32 = 3;
const GAP: i32 = 1;
const INSET: i32 = super::PROMPT_COLS;
// One row under the strip, one row for the input, the composer's top and bottom insets,
// and one footer row. The text references keep the space at smaller heights.
const REQUIRED_PAGE_ROWS: i32 = TILE_ROWS + GAP + 4;

#[derive(Clone, Copy, PartialEq)]
struct PageRows(i32);

pub(in crate::pages::session) fn page_room(child: AnyWidget) -> AnyWidget {
    LayoutBuilder::new(move |_, room| {
        Provider::value(PageRows(room.max_height))
            .update_should_notify(|old, new| old != new)
            .child(child.clone())
            .into()
    })
    .into()
}

pub(super) fn strip(ui: &Rc<ThreadUi>, thread: ThreadId) -> AnyWidget {
    let ui = ui.clone();
    ListenableBuilder::new(ui.composer.listenable(), move |cx, _| {
        if Provider::<PageRows>::maybe_of(cx, true).is_none_or(|rows| rows.0 < REQUIRED_PAGE_ROWS) {
            return SizedBox::shrink().into();
        }
        let Some(caps) = GraphicsScope::of(cx) else {
            return SizedBox::shrink().into();
        };
        let reported = caps.cell_pixel_width > 0 && caps.cell_pixel_height > 0;
        let (cell_width, cell_height) = match caps.protocol {
            GraphicsProtocol::Kitty | GraphicsProtocol::Sixel if reported => {
                (caps.cell_pixel_width, caps.cell_pixel_height)
            }
            GraphicsProtocol::Kitty => (8, 16),
            _ => return SizedBox::shrink().into(),
        };
        let size = ImageSize {
            width: cell_width.saturating_mul(TILE_COLS as u32).min(384) as u16,
            height: cell_height.saturating_mul(TILE_ROWS as u32).min(160) as u16,
        };
        let files: Vec<_> = ui
            .draft
            .elements(&ui.composer)
            .into_iter()
            .filter_map(|e| Some((e.seq, e.image?.to_str()?.to_owned(), e.label)))
            .collect();
        if files.is_empty() {
            return SizedBox::shrink().into();
        }
        let thread = thread.clone();
        LayoutBuilder::new(move |cx, room| {
            let columns = room.max_width.saturating_sub(INSET).max(0) as usize;
            let visible = fit(columns, files.len());
            let hidden = files.len() - visible;
            let mut row = Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .main_axis_size(MainAxisSize::Min);
            for (at, (seq, path, label)) in files.iter().take(visible).enumerate() {
                if at > 0 {
                    row = row.child(SizedBox::new(Some(GAP), None, None));
                }
                let style = Tokens::of(cx).dim();
                let tile =
                    attachment_thumbnail(cx, thread.clone(), *seq, path.clone(), size, |preview| {
                        match preview {
                            Some(preview) => Image::memory(preview.png)
                                .width(TILE_COLS)
                                .height(TILE_ROWS)
                                .into(),
                            None => SizedBox::new(
                                Some(TILE_COLS),
                                Some(TILE_ROWS),
                                Some(Text::new(label.to_string()).style(style).into()),
                            )
                            .into(),
                        }
                    });
                row = row.child(tile.with_key(Key::value(*seq)));
            }
            if hidden > 0 {
                let badge = format!("+{hidden}");
                if visible == 0 && columns < badge.len() {
                    return SizedBox::shrink().into();
                }
                if visible > 0 {
                    row = row.child(SizedBox::new(Some(GAP), None, None));
                }
                row = row.child(Text::new(badge).style(Tokens::of(cx).dim()));
            }
            ExcludeSelection::new(Padding::new(
                EdgeInsets::only(INSET, 0, 0, 0),
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .main_axis_size(MainAxisSize::Min)
                    .child(row)
                    .child(SizedBox::new(None, Some(GAP), None)),
            ))
            .into()
        })
        .into()
    })
    .into()
}

fn fit(columns: usize, count: usize) -> usize {
    let pitch = (TILE_COLS + GAP) as usize;
    let mut visible = ((columns + GAP as usize) / pitch).min(count);
    while visible > 0 && visible < count {
        let badge = 2 + (count - visible).ilog10() as usize;
        if visible * pitch + badge <= columns {
            break;
        }
        visible -= 1;
    }
    visible
}
