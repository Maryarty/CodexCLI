//! An artifact can use the host's validated snapshot while mounted. Native image placements
//! are layout only, outside all the text and selection surfaces of the transcript.

use rtwk::gestures::ExcludeSelection;
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{Align, Alignment, BoxConstraints, ConstrainedBox};
use rtwk::provider::{Provider, select};
use rtwk::{AnyWidget, Cx, LayoutBuilder};
use rtwk_core::graphics::{GraphicsCaps, GraphicsProtocol};
use rtwk_rich::image::Image;

use codex_tui_core::app::effects::{ImagePreview, ImageSize, ImageSnapshot};
pub(crate) use codex_tui_core::features::images::Interest as ArtifactInterest;
use codex_tui_core::features::images::{Images, Readiness, Source, SourceKind};
use codex_tui_core::features::{Feature, FeatureStates};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ItemKey};

use super::FeatureView;
use crate::pages::session::strings::Strings;
use crate::widgets::interest::Interest;

impl FeatureView for Images {
    fn note_words(note: &<Images as Feature>::Note, _: &Strings) -> String {
        note.absurd()
    }
}

pub(crate) fn source(thread: &ThreadId, key: &ItemKey, item: &Item) -> Option<Source> {
    let path = match item {
        Item::ImageView { path, .. } => path.as_str(),
        Item::Generated(image) if !image.failed => image.saved_path.as_deref()?,
        _ => return None,
    };
    (!path.is_empty()).then(|| Source {
        thread: thread.clone(),
        kind: SourceKind::Transcript(key.clone()),
        path: path.to_owned(),
    })
}

pub(crate) fn scope(source: Source, child: AnyWidget) -> AnyWidget {
    Provider::value(source)
        .update_should_notify(|old, new| old != new)
        .child(child)
        .into()
}

pub(crate) fn interest(
    source: Source,
    graphics: Option<GraphicsCaps>,
    preview: bool,
) -> ArtifactInterest {
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
    ArtifactInterest { source, preview }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Receipt {
    Ready(ImageSnapshot),
    Unavailable,
}

pub(crate) fn receipt(features: &FeatureStates, interest: &ArtifactInterest) -> Option<Receipt> {
    match features.images.readiness(interest) {
        Readiness::Pending => None,
        Readiness::Ready(snapshot) => Some(Receipt::Ready(snapshot.clone())),
        Readiness::Unavailable => Some(Receipt::Unavailable),
    }
}

pub(crate) fn watch_readiness<S>(cx: &mut Cx<'_, S>, interest: ArtifactInterest) {
    select::<FeatureStates, _, _>(cx, move |features| {
        !matches!(features.images.readiness(&interest), Readiness::Pending)
    });
}

/// A native row keeps its completed read through the terminal receipt even after the
/// live row and the history watcher release their interest.
pub(crate) fn freeze(receipt: Receipt, child: AnyWidget) -> AnyWidget {
    Provider::value(receipt)
        .update_should_notify(|old, new| old != new)
        .child(child)
        .into()
}

pub(crate) enum Read {
    Live(ArtifactInterest),
    Frozen(Receipt),
}

pub(crate) fn request<S>(cx: &mut Cx<'_, S>, preview: bool) -> Option<Read> {
    if let Some(receipt) = Provider::<Receipt>::maybe_of(cx, true) {
        return Some(Read::Frozen((*receipt).clone()));
    }
    let source = Provider::<Source>::maybe_of(cx, true)?;
    Some(Read::Live(interest(
        (*source).clone(),
        GraphicsScope::of(cx),
        preview,
    )))
}

pub(crate) fn snapshot<S>(cx: &mut Cx<'_, S>, read: &Read) -> Option<ImageSnapshot> {
    match read {
        Read::Live(read) => live_snapshot(cx, read),
        Read::Frozen(Receipt::Ready(snapshot)) => Some(snapshot.clone()),
        Read::Frozen(Receipt::Unavailable) => None,
    }
}

fn live_snapshot<S>(cx: &mut Cx<'_, S>, read: &ArtifactInterest) -> Option<ImageSnapshot> {
    let read = read.clone();
    select::<FeatureStates, Option<ImageSnapshot>, _>(cx, move |features| {
        features.images.snapshot(&read).cloned()
    })
}

pub(crate) fn watch(read: Option<Read>, child: AnyWidget) -> AnyWidget {
    match read {
        Some(Read::Live(read)) => Interest::new(read, child).into(),
        _ => child,
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
    let read = ArtifactInterest {
        source: Source {
            thread,
            kind: SourceKind::Attachment(sequence),
            path,
        },
        preview: Some(size),
    };
    let preview = live_snapshot(cx, &read).and_then(|snapshot| snapshot.preview);
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
        ExcludeSelection::new(Align::new(Alignment::CENTER, image).height_factor(1)).into()
    })
    .into()
}
