//! The editable image references stay in the field. This strip shows the visible local
//! attachments in text order and releases the reads when the strip leaves the layout.

use std::rc::Rc;

use rtwk::binding::{AppBinding, InputListener, ScreenMode};
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::ExcludeSelection;
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, Row, SizedBox};
use rtwk::provider::Provider;
use rtwk::widgets::{Builder, Text};
use rtwk::{AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_core::input::InputEvent;
use rtwk_rich::image::Image;

use codex_tui_core::app::effects::ImageSize;
use codex_tui_core::ids::ThreadId;

use crate::app::theme::Tokens;
use crate::features::attachment_thumbnail;
use crate::widgets::composer_draft::Element;
use crate::widgets::thread_ui::ThreadUi;

const TILE_COLS: i32 = 6;
const TILE_ROWS: i32 = 3;
const GAP: i32 = 1;
const INSET: i32 = super::PROMPT_COLS;
// One row under the strip, one row for the input, the composer's top and bottom insets,
// and one footer row. The text references keep the space at smaller heights.
const REQUIRED_PAGE_ROWS: i32 = TILE_ROWS + GAP + 4;

struct Attachment {
    sequence: u64,
    path: String,
    label: Rc<str>,
}

#[derive(Clone, Copy, PartialEq)]
struct PageRows {
    rows: i32,
    inline: bool,
}

pub(super) fn owned_composer_rows<S>(cx: &mut Cx<'_, S>) -> Option<i32> {
    let page = Provider::<PageRows>::maybe_of(cx, true)?;
    if page.inline {
        return None;
    }
    // The fullscreen pane leaves room above the composer for the transcript.
    Some((page.rows.saturating_mul(2) / 3).max(8).min(page.rows))
}

pub(in crate::pages::session) fn page_room(child: AnyWidget) -> AnyWidget {
    PageRoom { child }.into()
}

#[derive(Clone, PartialEq)]
struct PageRoom {
    child: AnyWidget,
}

#[derive(Default)]
struct RoomState {
    terminal_rows: Option<i32>,
}

impl StatefulWidget for PageRoom {
    type State = RoomState;

    fn create_state(&self) -> RoomState {
        RoomState::default()
    }
}

impl_widget!(stateful eq PageRoom);

impl State for RoomState {
    type Widget = PageRoom;
    type Msg = i32;

    fn update(&mut self, rows: i32, _: &PageRoom, _: &mut Cx<'_, Self>) -> Rebuild {
        if self.terminal_rows.replace(rows) == Some(rows) {
            Rebuild::No
        } else {
            Rebuild::Yes
        }
    }

    fn build(&self, widget: &PageRoom, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let inline =
            AppBinding::maybe_of(cx).is_some_and(|binding| binding.mode() == ScreenMode::Inline);
        if inline && image_bounds(cx).is_none() {
            return widget.child.clone();
        }
        let reported = self.terminal_rows.filter(|_| inline);
        let child = widget.child.clone();
        let layout = LayoutBuilder::new(move |_, room| {
            Provider::value(PageRows {
                rows: reported.unwrap_or(room.max_height),
                inline,
            })
            .update_should_notify(|old, new| old != new)
            .child(child.clone())
            .into()
        });
        if !inline {
            return layout.into();
        }
        // An inline region grows from its intrinsic height. If a short terminal hides the
        // strip, its current layout stays short even after the physical terminal grows.
        let tell = cx.local_sender();
        InputListener::new(layout, move |event| {
            if let InputEvent::Resize { height, .. } = event
                && let Ok(rows) = i32::try_from(*height)
            {
                tell(rows);
            }
            false
        })
        .into()
    }
}

pub(super) fn strip(ui: &Rc<ThreadUi>, thread: ThreadId) -> AnyWidget {
    let ui = ui.clone();
    Builder::new(move |cx| {
        let Some(size) = preview_size(cx) else {
            return SizedBox::shrink().into();
        };
        let (ui, thread) = (ui.clone(), thread.clone());
        ListenableBuilder::new(ui.composer.listenable(), move |_, _| {
            let files: Vec<_> = ui
                .draft
                .elements(&ui.composer)
                .into_iter()
                .filter_map(
                    |Element {
                         seq: sequence,
                         image,
                         label,
                         ..
                     }| {
                        Some(Attachment {
                            sequence,
                            path: image?.to_str()?.to_owned(),
                            label,
                        })
                    },
                )
                .collect();
            if files.is_empty() {
                return SizedBox::shrink().into();
            }
            let thread = thread.clone();
            LayoutBuilder::new(move |cx, room| {
                row(
                    cx,
                    &thread,
                    &files,
                    size,
                    room.max_width.saturating_sub(INSET).max(0) as usize,
                )
            })
            .into()
        })
        .into()
    })
    .into()
}

fn preview_size(cx: &mut Cx<'_>) -> Option<ImageSize> {
    let rows = Provider::<PageRows>::maybe_of(cx, true)?;
    if rows.rows < REQUIRED_PAGE_ROWS {
        return None;
    }
    image_bounds(cx)
}

fn image_bounds<S>(cx: &mut Cx<'_, S>) -> Option<ImageSize> {
    let caps = GraphicsScope::of(cx)?;
    let reported = caps.cell_pixel_width > 0 && caps.cell_pixel_height > 0;
    let (cell_width, cell_height) = match caps.protocol {
        GraphicsProtocol::Kitty | GraphicsProtocol::Sixel if reported => {
            (caps.cell_pixel_width, caps.cell_pixel_height)
        }
        GraphicsProtocol::Kitty => (8, 16),
        _ => return None,
    };
    Some(ImageSize {
        width: cell_width.saturating_mul(TILE_COLS as u32).min(384) as u16,
        height: cell_height.saturating_mul(TILE_ROWS as u32).min(160) as u16,
    })
}

fn row(
    cx: &mut Cx<'_>,
    thread: &ThreadId,
    files: &[Attachment],
    size: ImageSize,
    columns: usize,
) -> AnyWidget {
    let visible = fit(columns, files.len());
    let hidden = files.len() - visible;
    let style = Tokens::of(cx).dim();
    let mut row = Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_size(MainAxisSize::Min);
    for (at, file) in files.iter().take(visible).enumerate() {
        if at > 0 {
            row = row.child(SizedBox::new(Some(GAP), None, None));
        }
        let tile = attachment_thumbnail(
            cx,
            thread.clone(),
            file.sequence,
            file.path.clone(),
            size,
            |preview| match preview {
                Some(preview) => Image::memory(preview.png)
                    .width(TILE_COLS)
                    .height(TILE_ROWS)
                    .into(),
                None => SizedBox::new(
                    Some(TILE_COLS),
                    Some(TILE_ROWS),
                    Some(
                        Text::new(file.label.to_string())
                            .style(style.clone())
                            .into(),
                    ),
                )
                .into(),
            },
        );
        row = row.child(tile.with_key(Key::value(file.sequence)));
    }
    if hidden > 0 {
        let badge = format!("+{hidden}");
        if visible == 0 && columns < badge.len() {
            return SizedBox::shrink().into();
        }
        if visible > 0 {
            row = row.child(SizedBox::new(Some(GAP), None, None));
        }
        row = row.child(Text::new(badge).style(style));
    }
    ExcludeSelection::new(Padding::new(EdgeInsets::only(INSET, 0, 0, GAP), row)).into()
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
