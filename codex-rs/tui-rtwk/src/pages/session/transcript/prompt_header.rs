//! A short, clickable prompt gives context when the start of the turn is above the owned reader.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ItemKey, ThreadModel, Transcript};
use rtwk::app::ViewportCx;
use rtwk::binding::AppBinding;
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{ExcludeSelection, GestureDetector, PointerEvent};
use rtwk::layout::{EdgeInsets, Padding, Positioned, SizedBox, Stack};
use rtwk::navigation::Route;
use rtwk::style::{Attrs, Span};
use rtwk::text::{Text, TextOverflow};
use rtwk::{AnyWidget, Cx, Eid, GlobalKey, Key, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_render::objects::list::ListParentData;
use rtwk_render::objects::stack::StackFit;

use super::{
    Finder, ListRows, Modes, Row, TextSelection, activity_focus, copying, select, with_render_list,
};
use crate::app::theme::Tokens;
use crate::pages::session::cells::user;
use crate::pages::session::strings;
use crate::widgets::thread_ui::{Follow, Input};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Target {
    thread: ThreadId,
    reset: u64,
    prompt: ItemKey,
}

impl Target {
    fn current(&self, thread: &ThreadModel) -> bool {
        thread.id() == &self.thread && thread.transcript().reset_revision() == self.reset
    }
}

/// What a plain press of the header lets go of before it jumps: the range or the copy target
/// under it, Find, a fold in focus (oracle `end_selection`, `cancel_search`,
/// `clear_activity_focus`). The select mode's caret stays, as the oracle keeps its select mode.
#[derive(Clone, PartialEq)]
pub(super) struct Held {
    pub selection: TextSelection,
    pub find: Finder,
    pub modes: Modes,
}

impl Held {
    /// Lets go of all of it, and tells whether anything was held: the list then builds again
    /// before the jump (the live rows in place of a retained revision, Find's footer gone).
    fn release(&self) -> bool {
        let (copier, caret) = (copying::mode_of(&self.find), select::of(&self.find));
        let activity = activity_focus::of(&self.find);
        let held = self.find.active()
            || self.selection.held()
            || copier.active()
            || caret.extending()
            || activity.focused();
        // Find closes where the reader is: the jump below is the only move, no way back
        // (`cancel_search` restores a position that `jump_to_entry` replaces at once).
        self.find.accepted();
        if copier.active() {
            copier.leave();
            self.modes.hold(Input::Plain);
        }
        caret.end_extension();
        if !caret.active() {
            self.selection.controller().clear();
        }
        activity.leave();
        held
    }
}

#[derive(Clone)]
pub(super) struct Reader {
    pub child: AnyWidget,
    pub list: GlobalKey,
    pub rows: Rc<ListRows>,
    pub thread: Rc<ThreadModel>,
    pub follow: Follow,
    pub held: Held,
    /// The header takes a press: a reader that browses the prompts (the one marked is the
    /// reader's place) does not.
    pub interactive: bool,
    pub room: (i32, i32),
}

impl Reader {
    fn enough_room(&self) -> bool {
        let (width, height) = self.room;
        width >= 16 && height >= 4
    }
}

impl PartialEq for Reader {
    fn eq(&self, other: &Self) -> bool {
        self.list == other.list
            && Rc::ptr_eq(&self.rows, &other.rows)
            && Rc::ptr_eq(&self.thread, &other.thread)
            && self.follow == other.follow
            && self.held == other.held
            && self.interactive == other.interactive
            && self.room == other.room
            && self.child == other.child
    }
}

#[derive(PartialEq, Eq)]
struct Header {
    target: Target,
    label: String,
}

#[derive(Default)]
pub(super) struct ReaderState {
    header: Option<Header>,
}

pub(super) enum Msg {
    ReadViewport,
    Jump(Target, PointerEvent),
    /// The jump of a press that let go of a range, Find or a mode first: the frame it asked for
    /// has built the list again.
    Go(Target),
    Align(Target, u64),
}

impl StatefulWidget for Reader {
    type State = ReaderState;

    fn create_state(&self) -> Self::State {
        ReaderState::default()
    }
}

impl_widget!(stateful eq Reader);

impl ReaderState {
    /// Scrolls the owned list to the prompt of `target`, and aligns its top once laid out.
    fn go(&self, target: &Target, w: &Reader, cx: &mut Cx<'_, Self>) {
        let Some(logical) = w.rows.logical_index(&Key::value(target.prompt.clone())) else {
            return;
        };
        w.follow.cancel_beginning();
        w.follow.scroll().jump_to_index(w.rows.list_index(logical));
        if w.rows.reverse
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let (target, moves) = (target.clone(), w.follow.moves());
            binding.add_post_frame_callback(
                cx.map(move |_: Duration| Msg::Align(target.clone(), moves)),
            );
        }
    }
}

impl State for ReaderState {
    type Widget = Reader;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &Reader, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::ReadViewport => {
                let Some(element) = cx.global_key_element(&w.list) else {
                    return Rebuild::No;
                };
                let rows = w.rows.clone();
                let thread = w.thread.clone();
                let current_rows = rows.clone();
                let answer = Rc::new(RefCell::new(None));
                let prepared = answer.clone();
                let reserved = self
                    .header
                    .as_ref()
                    .is_some_and(|header| header.target.current(&thread));
                let enough_room = w.enough_room();
                let done = cx.callback_with(move |state: &mut Self, accepted: bool, cx| {
                    if !accepted
                        || cx.with_widget(cx.id(), |w: &Reader| Rc::ptr_eq(&w.rows, &current_rows))
                            != Some(true)
                    {
                        return Rebuild::No;
                    }
                    let next = answer.borrow_mut().take();
                    if state.header == next {
                        return Rebuild::No;
                    }
                    state.header = next;
                    Rebuild::Yes
                });
                let _ = cx.request_viewport(
                    move |viewport| {
                        if !viewport.has_current_layout_size()
                            || viewport.has_pending_rebuild(element)
                        {
                            return false;
                        }
                        if enough_room {
                            *prepared.borrow_mut() =
                                visible_prompt(viewport, element, &rows, &thread, reserved);
                        }
                        true
                    },
                    done,
                );
            }
            Msg::Jump(target, pointer) => {
                if !pointer.modifiers.is_empty()
                    || !w.interactive
                    || !w.enough_room()
                    || self.header.as_ref().map(|header| &header.target) != Some(&target)
                    || !target.current(&w.thread)
                    || !Route::maybe_of(cx).is_none_or(|route| route.is_current())
                {
                    return Rebuild::No;
                }
                // The oracle ends the range, cancels Find and clears the fold's focus first. The
                // list then reads its live rows and the jump is made over them, not over a
                // revision the range retained.
                if w.held.release()
                    && let Some(binding) = AppBinding::maybe_of(cx)
                {
                    binding.add_post_frame_callback(
                        cx.map(move |_: Duration| Msg::Go(target.clone())),
                    );
                    return Rebuild::No;
                }
                self.go(&target, w, cx);
            }
            Msg::Go(target) => {
                if !w.interactive
                    || !w.enough_room()
                    || !target.current(&w.thread)
                    || !Route::maybe_of(cx).is_none_or(|route| route.is_current())
                {
                    return Rebuild::No;
                }
                self.go(&target, w, cx);
            }
            Msg::Align(target, moves) => {
                if !w.interactive
                    || !w.enough_room()
                    || !target.current(&w.thread)
                    || w.follow.moves() != moves
                    || !Route::maybe_of(cx).is_none_or(|route| route.is_current())
                {
                    return Rebuild::No;
                }
                let Some(element) = cx.global_key_element(&w.list) else {
                    return Rebuild::No;
                };
                let Some(logical) = w.rows.logical_index(&Key::value(target.prompt)) else {
                    return Rebuild::No;
                };
                let scroll = w.follow.scroll();
                let _ = cx.request_viewport(
                    move |viewport| {
                        if !viewport.has_current_layout_size()
                            || viewport.has_pending_rebuild(element)
                        {
                            return false;
                        }
                        with_render_list(viewport, element, |_, _, list| {
                            let (offset, _) = list.item_place(logical)?;
                            let position = scroll.try_position().ok()?;
                            position.jump_to(position.pixels().saturating_sub(offset));
                            Some(())
                        })
                        .is_some()
                    },
                    cx.callback_with(|_, _, _| Rebuild::No),
                );
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Reader, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let words = strings::of(cx);
        let tokens = Tokens::of(cx);
        let header = self
            .header
            .as_ref()
            .filter(|header| w.enough_room() && header.target.current(&w.thread))
            .map(|header| {
                let target = &header.target;
                let line = Text::rich_line(vec![
                    Span::styled(words.prompt, tokens.dim().attrs(Attrs::BOLD)),
                    Span::new(header.label.clone()),
                ])
                .overflow(TextOverflow::Ellipsis)
                .style(tokens.prompt_surface(cx, true));
                let widget: AnyWidget = if w.interactive {
                    let target = target.clone();
                    GestureDetector::new()
                        .on_tap_down(cx.map(move |event| Msg::Jump(target.clone(), event)))
                        .child(line)
                        .into()
                } else {
                    line.into()
                };
                AnyWidget::from(ExcludeSelection::new(widget)).with_key(Key::value(target.clone()))
            });

        let scroll = Rc::new(w.follow.scroll().as_scroll_controller().clone());
        let inspect = cx.map(|_: Duration| Msg::ReadViewport);
        // Native metrics include changes to a row's extent with the tail still
        // at zero. Only the zero-height observer rebuilds on scroll.
        let observer = AnyWidget::from(ListenableBuilder::new(scroll, move |cx, _| {
            if let Some(binding) = AppBinding::maybe_of(cx) {
                binding.add_post_frame_callback(inspect.clone());
            }
            SizedBox::shrink().into()
        }))
        .with_key(Key::value("owned transcript observer"));
        let body = Padding::new(
            EdgeInsets::only(0, i32::from(header.is_some()), 0, 0),
            w.child.clone(),
        );
        let stack = Stack::new()
            .fit(StackFit::Passthrough)
            .child(body)
            .child(Positioned::new(observer).top(0).left(0).width(0).height(0));
        match header {
            Some(header) => stack
                .child(Positioned::new(header).top(0).left(0).right(0).height(1))
                .into(),
            None => stack.into(),
        }
    }
}

/// Index the prompt rows once per displayed row shape, including the prompts
/// whose content is temporarily empty.
pub(super) fn prompt_rows(rows: &[Row], transcript: &Transcript) -> Rc<[usize]> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, row)| {
            matches!(transcript.get(row.range.start), Some((_, Item::User(_)))).then_some(index)
        })
        .collect()
}

fn visible_prompt(
    viewport: &ViewportCx<'_>,
    element: Eid,
    rows: &ListRows,
    thread: &ThreadModel,
    reserved: bool,
) -> Option<Header> {
    let prompts = rows.prompts.as_deref()?;
    with_render_list(viewport, element, |tree, id, list| {
        // In a reverse list reserving a line at the top preserves its bottom.
        // Consider the row immediately above the body too, since dropping the
        // header would let that row back in. This avoids a reserve/release loop
        // at a prompt that would otherwise become the first visible row.
        let first = |top| {
            tree.children(id).find_map(|child| {
                let index = tree.parent_data::<ListParentData>(child)?.index;
                let (offset, height) = list.item_place(index)?;
                (height > 0
                    && offset < i64::from(tree.size(id).h)
                    && offset + i64::from(height) > top)
                    .then_some(index)
            })
        };
        let transcript = thread.transcript();
        let prompt_at = |index: usize| {
            let (key, item) = transcript.get(rows.rows.get(index)?.range.start)?;
            match item {
                Item::User(prompt) if !user::is_blank(prompt) => Some((key, prompt)),
                _ => None,
            }
        };
        let preceding = |logical: usize| {
            let at = logical.checked_sub(rows.lead)?;
            let split = prompts.partition_point(|&index| index < at);
            if prompts.get(split) == Some(&at) && prompt_at(at).is_some() {
                return None;
            }
            prompts[..split]
                .iter()
                .rev()
                .find_map(|&index| prompt_at(index))
        };
        if reserved && rows.reverse {
            preceding(first(-1)?)?;
        }
        let (key, prompt) = preceding(first(0)?)?;
        let prefix: String = prompt.shown().text.trim_start().chars().take(512).collect();
        let label = prefix.split_whitespace().collect::<Vec<_>>().join(" ");
        let label = if label.is_empty() {
            crate::oracle_strings::transcript::PROMPT_HEADER_ATTACHMENTS.to_owned()
        } else {
            label
        };
        Some(Header {
            target: Target {
                thread: thread.id().clone(),
                reset: transcript.reset_revision(),
                prompt: key.clone(),
            },
            label,
        })
    })
}
