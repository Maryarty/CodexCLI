//! An artifact can use the host's validated snapshot while mounted. Native image placements
//! are layout only, outside all the text and selection surfaces of the transcript.

use rtwk::gestures::ExcludeSelection;
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{Align, Alignment, BoxConstraints, ConstrainedBox};
use rtwk::provider::{Provider, select};
use rtwk::{AnyWidget, Cx, LayoutBuilder};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_rich::image::Image;

use codex_tui_core::app::effects::{ImagePreview, ImageSize, ImageSnapshot};
use codex_tui_core::features::images::{Images, Interest as Snapshot, Source, SourceKind};
use codex_tui_core::features::{Feature, FeatureStates};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::ItemKey;

use super::FeatureView;
use crate::pages::session::strings::Strings;
use crate::widgets::interest::Interest;

impl FeatureView for Images {
    fn note_words(note: &<Images as Feature>::Note, _: &Strings) -> String {
        note.absurd()
    }
}

pub(crate) fn scope(thread: ThreadId, item: ItemKey, path: String, child: AnyWidget) -> AnyWidget {
    Provider::value(Source {
        thread,
        kind: SourceKind::Transcript(item),
        path,
    })
    .update_should_notify(|old, new| old != new)
    .child(child)
    .into()
}

pub(crate) fn request<S>(cx: &mut Cx<'_, S>, preview: bool) -> Option<Snapshot> {
    let source = Provider::<Source>::maybe_of(cx, true)?;
    let graphics = GraphicsScope::of(cx);
    let preview = graphics
        .filter(|_| preview)
        .and_then(|caps| match caps.protocol {
            GraphicsProtocol::Kitty => Some(ImageSize {
                width: 384,
                height: 96,
            }),
            GraphicsProtocol::Sixel if caps.cell_pixel_width > 0 && caps.cell_pixel_height > 0 => {
                Some(ImageSize {
                    width: 160,
                    height: 80,
                })
            }
            _ => None,
        });
    Some(Snapshot {
        source: (*source).clone(),
        preview,
    })
}

pub(crate) fn snapshot<S>(cx: &mut Cx<'_, S>, read: &Snapshot) -> Option<ImageSnapshot> {
    let read = read.clone();
    select::<FeatureStates, Option<ImageSnapshot>, _>(cx, move |features| {
        features.images.snapshot(&read).cloned()
    })
}

pub(crate) fn watch(read: Option<Snapshot>, child: AnyWidget) -> AnyWidget {
    match read {
        Some(read) => Interest::new(read, child).into(),
        None => child,
    }
}

/// Read and watch one visible draft attachment through the same host snapshot door as artifacts.
pub(crate) fn attachment<S>(
    cx: &mut Cx<'_, S>,
    thread: ThreadId,
    sequence: u64,
    path: String,
    size: ImageSize,
    child: impl FnOnce(Option<ImagePreview>) -> AnyWidget,
) -> AnyWidget {
    let read = Snapshot {
        source: Source {
            thread,
            kind: SourceKind::Attachment(sequence),
            path,
        },
        preview: Some(size),
    };
    let preview = snapshot(cx, &read).and_then(|snapshot| snapshot.preview);
    Interest::new(read, child(preview)).into()
}

pub(crate) fn preview(snapshot: ImagePreview) -> AnyWidget {
    LayoutBuilder::new(move |_, room| {
        // Leave the two-column transcript inset and enough room to inspect the image.
        let columns = room.max_width.saturating_sub(2).min(48);
        if columns < 8 {
            return AnyWidget::default();
        }
        let image = ConstrainedBox::new(
            BoxConstraints::new(0, columns, 0, 6),
            Image::memory(snapshot.png.clone()),
        );
        ExcludeSelection::new(Align::new(Alignment::CENTER, image)).into()
    })
    .into()
}
