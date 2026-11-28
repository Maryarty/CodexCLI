//! The terminal takes the settled prefix; reading modes keep the canonical transcript.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::{Item, ItemKey, ThreadModel};
use rtwk::binding::Commit;
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{
    Align, Alignment, BoxConstraints, Column, CrossAxisAlignment, MainAxisSize, SizedBox, UNBOUNDED,
};
use rtwk::provider::{Provider, read};
use rtwk::style::Theme;
use rtwk::text::segmenter_of;
use rtwk::{AnyWidget, Children, Cx, RenderObjectWidget, State, WidgetTree, impl_widget};
use rtwk_core::graphics::GraphicsCaps;
use rtwk_render::{
    Change, ChildModel, IntrinsicCx, LayoutCx, PaintCx, RenderError, RenderObject, RenderTree, Size,
};

use super::super::transcript::turn_details::Row;
use crate::features::{
    ArtifactInterest, ArtifactReceipt, artifact_interest, artifact_receipt, artifact_source,
};

/// One thread's record of which immutable rows have already entered this terminal's
/// scrollback. Kept with the thread's `Finder` across A → B → A and view remounts.
#[derive(Clone, Default)]
pub(super) struct History(Rc<RefCell<Presented>>);

#[derive(Default)]
struct Presented {
    keys: HashSet<ItemKey>,
    header: bool,
    /// The first item whose settlement can change the prefix without changing its layout.
    blocker: Option<Blocker>,
}

struct Blocker {
    index: usize,
    key: ItemKey,
    revision: Option<u64>,
    awaiting_image: bool,
}

pub(super) struct Settled {
    /// Logical rows before this offset may leave the compact live viewport.
    pub upto: usize,
    /// The suffix of those rows that the terminal has not seen.
    pub from: usize,
    pub hide_header: bool,
    pub commit_header: bool,
    pub waiting: Option<ArtifactInterest>,
    pub artifacts: BTreeMap<usize, ArtifactReceipt>,
}

impl History {
    pub fn blocker_settled(&self, thread: &ThreadModel) -> Option<bool> {
        let state = self.0.borrow();
        let blocker = state.blocker.as_ref()?;
        let transcript = thread.transcript();
        Some(transcript.get(blocker.index).is_none_or(|(key, item)| {
            key != &blocker.key
                || if blocker.awaiting_image {
                    // The same key may now point to a different path.
                    transcript.item_revision(key) != blocker.revision
                } else {
                    settled(thread, key, item)
                }
        }))
    }

    pub fn settled(
        &self,
        thread: &ThreadModel,
        rows: &[Row],
        has_header: bool,
        features: &FeatureStates,
        graphics: Option<GraphicsCaps>,
        raw: bool,
    ) -> Settled {
        let transcript = thread.transcript();
        let mut presented = self.0.borrow_mut();
        // A history page prepended before an already presented row belongs in the reader.
        // A terminal has no insertion point for it before what it has already scrolled out.
        let from = if presented.keys.is_empty() {
            0
        } else {
            rows.iter()
                .rposition(|row| {
                    row.range.clone().any(|index| {
                        transcript
                            .get(index)
                            .is_some_and(|(key, _)| presented.keys.contains(key))
                    })
                })
                .map_or(0, |last| last + 1)
        };
        let mut artifacts = BTreeMap::new();
        let mut waiting = None;
        let mut blocker = None;
        let upto = rows
            .iter()
            .enumerate()
            .skip(from)
            .find_map(|(position, row)| {
                for index in row.range.clone() {
                    let Some((key, item)) = transcript.get(index) else {
                        continue;
                    };
                    let block = |awaiting_image| Blocker {
                        index,
                        key: key.clone(),
                        revision: transcript.item_revision(key),
                        awaiting_image,
                    };
                    if !settled(thread, key, item) {
                        blocker = Some(block(false));
                        return Some(position);
                    }
                    if let Some(source) = artifact_source(thread.id(), key, item) {
                        let preview = matches!(item, Item::Generated(_)) && !raw;
                        let interest = artifact_interest(source, graphics, preview);
                        let Some(receipt) = artifact_receipt(features, &interest) else {
                            blocker = Some(block(true));
                            waiting = Some(interest);
                            return Some(position);
                        };
                        artifacts.insert(index, receipt);
                    }
                }
                None
            })
            .unwrap_or(rows.len());
        presented.blocker = blocker;
        Settled {
            upto,
            from,
            hide_header: presented.header || upto != 0,
            commit_header: has_header && !presented.header && upto != 0,
            waiting,
            artifacts,
        }
    }

    pub fn commit<S: State>(
        &self,
        cx: &mut Cx<'_, S>,
        sink: Rc<dyn Commit>,
        thread: &ThreadModel,
        rows: &[Row],
        header: Option<AnyWidget>,
        body: impl Iterator<Item = AnyWidget>,
    ) {
        if rows.is_empty() && header.is_none() {
            return;
        }
        let mut presented = self.0.borrow_mut();
        presented.header |= header.is_some();
        for row in rows {
            presented.keys.extend(
                row.range
                    .clone()
                    .filter_map(|index| thread.transcript().get(index).map(|(key, _)| key.clone())),
            );
        }
        drop(presented);

        // This is the mounted sink used by Static. The source keeps its inherited context,
        // while the native sink owns the rendered rows until it finishes presenting them.
        // Calling it with a single batch avoids keeping a second history of widget payloads.
        let body = rows.iter().zip(body).map(|(row, widget)| {
            if matches!(
                thread.transcript().get(row.range.start),
                Some((_, Item::Plan { .. }))
                    | Some((_, Item::ImageView { .. }))
                    | Some((_, Item::Generated(_)))
            ) {
                MeasureHistoryRow::for_inline(cx, widget)
            } else {
                widget
            }
        });
        let output = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(header.into_iter().chain(body));
        let source = cx.id();
        sink.clone()
            .commit_from(cx.owner_mut(), source, output.into());
        sink.shrink_region();
    }
}

/// A committed table or image can contain a LayoutBuilder. The terminal
/// asks the mounted branch for its height before that builder has run.
/// Lay out the same native cell in a data tree at the requested width.
#[derive(Debug)]
struct MeasureHistoryRow {
    child: AnyWidget,
    measure: AnyWidget,
    segmenter: crate::widgets::measure::Measure,
}

impl MeasureHistoryRow {
    fn for_inline<S: State>(cx: &mut Cx<'_, S>, child: AnyWidget) -> AnyWidget {
        let mut screens = ScreenStates::default();
        screens.cells.link_display = crate::app::theme::Tokens::link_display(cx);
        let measure = Theme::new(
            Theme::of(cx),
            Provider::value(*read::<TuiSettings, _>(cx))
                .child(Provider::value(screens).child(child.clone())),
        );
        let measure = crate::app::run::localized(measure, crate::l10n::lang(cx));
        let measure = match GraphicsScope::of(cx) {
            Some(caps) => GraphicsScope::new(caps, measure).into(),
            None => measure.into(),
        };
        Self {
            child,
            measure,
            segmenter: segmenter_of(cx),
        }
        .into()
    }
}

impl RenderObjectWidget for MeasureHistoryRow {
    type Object = RenderMeasureHistoryRow;

    fn create_render_object(&self, _: &mut Cx<'_>) -> Self::Object {
        RenderMeasureHistoryRow {
            measure: self.measure.clone(),
            segmenter: self.segmenter,
            measured: None,
        }
    }

    fn update_render_object(&self, cx: &mut Cx<'_>, object: &mut Self::Object) -> Change {
        *object = self.create_render_object(cx);
        Change::Layout
    }

    fn children(&self) -> Children<'_> {
        Children::One(Some(&self.child))
    }
}

impl_widget!(render MeasureHistoryRow);

#[derive(Debug)]
struct RenderMeasureHistoryRow {
    measure: AnyWidget,
    segmenter: crate::widgets::measure::Measure,
    measured: Option<(i32, i32)>,
}

impl RenderObject for RenderMeasureHistoryRow {
    fn child_model(&self) -> ChildModel {
        ChildModel::Single
    }

    fn compute_max_intrinsic_height(&mut self, cx: &mut IntrinsicCx<'_>, width: i32) -> i32 {
        if let Some((measured_width, height)) = self.measured
            && measured_width == width
        {
            return height;
        }
        let mut tree =
            WidgetTree::with_render_tree(RenderTree::for_data_with_segmenter(self.segmenter));
        tree.show(Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new(Some(width), None, Some(self.measure.clone())),
        ));
        let height = if tree.layout(width, UNBOUNDED).is_ok()
            && let Some(root) = tree.render_object_of(tree.root())
            && let Some(row) = tree.render_tree().child(root)
        {
            tree.render_tree().size(row).h
        } else {
            cx.child()
                .map_or(0, |child| cx.max_intrinsic_height(child, width))
        };
        self.measured = Some((width, height));
        height
    }

    fn perform_layout(
        &mut self,
        cx: &mut LayoutCx<'_>,
        constraints: BoxConstraints,
    ) -> Result<Size, RenderError> {
        let Some(child) = cx.child() else {
            return Ok(Size::new(constraints.min_width, constraints.min_height));
        };
        let size = cx.layout_child_sized(child, constraints);
        cx.set_child_offset(child, 0, 0);
        Ok(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_, '_>) -> Result<(), RenderError> {
        if let Some(child) = cx.child() {
            cx.paint_child(child);
        }
        Ok(())
    }
}

fn settled(thread: &ThreadModel, key: &ItemKey, item: &Item) -> bool {
    if thread.transcript().is_agent_streaming(key)
        || matches!(
            item,
            Item::Status {
                request: Some(_),
                ..
            }
        )
    {
        return false;
    }
    thread.turn_finished(&key.turn)
        || matches!(item, Item::VoiceTranscript { .. })
        // Idle local entries can be anchored before any turn or after a realtime caption.
        // Their payloads are final even if their placement key never ends a server turn.
        || (!super::in_running_turn(thread, key)
            && matches!(item, Item::Notice(_) | Item::Note(_) | Item::Status { .. }
                | Item::StartupWarning(_) | Item::Error { .. } | Item::HighLoad
                | Item::Decision(_) | Item::Review(_) | Item::Hook(_) | Item::ImageView { .. }))
}
