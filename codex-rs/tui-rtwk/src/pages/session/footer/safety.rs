//! Session-owned menus for a live safety check and a stopped conversation.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::binding::AppBinding;
use rtwk::focus::{Focus, FocusNode};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::overlay::show_dialog;
use rtwk::provider::{read, select};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Key, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::commands::{SessionIntent, SlashCommand, Target};
use codex_tui_core::ids::{ThreadId, TurnId};
use codex_tui_core::keymap::{actions, fixed};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, transcript::PagerDoc};
use codex_tui_core::threads::ThreadModel;
use codex_tui_core::threads::composer::composer_state;
use codex_tui_core::threads::safety::{RetryOffer, Review, ReviewId, Verb};

use crate::app::{theme::Tokens, view::Hs};
use crate::pages::session::{open::open, transcript::pager};
use crate::widgets::options::{OptionStyles, Options, RichRow};

use super::{Chat, ChatState};

#[path = "safety/strings.rs"]
mod strings;

pub(in crate::pages::session) use strings::WAITING;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Check {
    turn: TurnId,
    revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Surface {
    Waiting {
        check: Check,
        retry: Option<RetryOffer>,
    },
    ConfirmRetry {
        check: Check,
        offer: RetryOffer,
    },
    Retrying,
    Precaution(Option<ReviewId>),
    Findings {
        review: ReviewId,
        can_continue: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    OfferRetry,
    KeepWaiting,
    StopAndRetry,
    Dismiss,
    Learn,
    Review,
    Acknowledge,
    Back,
    New,
    Agents,
    Resume,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::OfferRetry => strings::RETRY,
            Self::KeepWaiting => strings::KEEP_WAITING,
            Self::StopAndRetry => strings::STOP_AND_RETRY,
            Self::Dismiss => strings::DISMISS,
            Self::Learn => strings::LEARN,
            Self::Review => strings::REVIEW,
            Self::Acknowledge => strings::ACKNOWLEDGE,
            Self::Back => strings::BACK,
            Self::New => strings::NEW,
            Self::Agents => strings::AGENTS,
            Self::Resume => strings::RESUME,
        }
    }
}

impl Surface {
    fn actions(&self) -> &'static [Action] {
        use Action::*;
        match self {
            Self::Waiting { retry: Some(_), .. } => &[OfferRetry, Dismiss, Learn],
            Self::Waiting { .. } => &[Dismiss, Learn],
            Self::ConfirmRetry { .. } => &[KeepWaiting, StopAndRetry],
            Self::Retrying => &[],
            Self::Precaution(Some(_)) => &[Review, Agents, New, Resume],
            Self::Precaution(_) => &[New, Agents, Resume],
            Self::Findings { .. } => &[Acknowledge, Back],
        }
    }
}

pub(super) fn slot(
    w: &Chat,
    state: &ChatState,
    cx: &mut Cx<'_, ChatState>,
    child: AnyWidget,
) -> AnyWidget {
    // The first thread inherits the provisional composer's controllers.
    let key = Key::object(&w.ui);
    let w = w.clone();
    let on_stop = cx.callback(move |state, ()| {
        w.ui.draft.discard(&w.ui.composer);
        state.close_editor();
        state.sync_draft(&w);
    });
    AnyWidget::from(Slot {
        child,
        on_stop,
        composer_focus: state.node.clone(),
    })
    .with_key(key)
}

#[derive(Clone, PartialEq)]
struct Slot {
    child: AnyWidget,
    on_stop: Handler<()>,
    composer_focus: FocusNode,
}

#[derive(Default)]
struct SlotState {
    dismissed: Option<Check>,
    confirming: Option<RetryOffer>,
    viewed: Option<ReviewId>,
    stopped: Cell<bool>,
    showing_menu: Cell<bool>,
}

impl StatefulWidget for Slot {
    type State = SlotState;

    fn create_state(&self) -> Self::State {
        SlotState::default()
    }
}

impl_widget!(stateful eq Slot);

#[derive(Clone, PartialEq)]
struct Facts {
    thread: ThreadId,
    check: Option<Check>,
    retry: Option<RetryOffer>,
    retrying: bool,
    // Outer Option: this chat is stopped. Inner Option: it can show live findings.
    precaution: Option<Option<ReviewId>>,
    can_continue: bool,
}

impl Facts {
    fn of(thread: &ThreadModel) -> Self {
        let safety = thread.safety();
        let waiting = safety
            .buffering()
            .filter(|held| held.waiting() && thread.is_running() && !thread.stream_lost());
        let violation = safety.violation();
        let review = violation.and_then(|violation| violation.review.as_ref());
        Self {
            thread: thread.id().clone(),
            check: waiting.map(|held| Check {
                turn: held.turn.clone(),
                revision: held.revision,
            }),
            retry: waiting.and_then(|held| held.retry_offer()).cloned(),
            retrying: safety.retrying(),
            precaution: violation.map(|_| review.map(|review| review.id().clone())),
            can_continue: review.and_then(Review::continuation).is_some()
                && composer_state(thread, false).takes_text(),
        }
    }
}

fn can_offer_retry(states: &ScreenStates, thread: &ThreadId) -> bool {
    !states.chat.flags.side_conversation_active
        && !states
            .agents
            .is_cloud(states.agents.owner_of(thread.as_str()))
}

impl SlotState {
    fn surface(&self, facts: Facts, can_retry: bool) -> Option<Surface> {
        if let Some(review) = facts.precaution {
            return Some(match review {
                Some(review) if self.viewed.as_ref() == Some(&review) => Surface::Findings {
                    review,
                    can_continue: facts.can_continue,
                },
                review => Surface::Precaution(review),
            });
        }
        if facts.retrying {
            return Some(Surface::Retrying);
        }
        let check = facts.check?;
        if self.dismissed.as_ref() == Some(&check) {
            return None;
        }
        let retry = facts.retry.filter(|_| can_retry);
        match retry {
            Some(offer) if self.confirming.as_ref() == Some(&offer) => {
                Some(Surface::ConfirmRetry { check, offer })
            }
            retry => Some(Surface::Waiting { check, retry }),
        }
    }
}

struct Decision {
    owner: ThreadId,
    surface: Surface,
    action: Action,
}

impl State for SlotState {
    type Widget = Slot;
    type Msg = Decision;

    fn update(
        &mut self,
        Decision {
            owner,
            surface,
            action,
        }: Self::Msg,
        _: &Slot,
        cx: &mut Cx<'_, Self>,
    ) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        let session = read::<SessionSlice, _>(cx);
        let states = read::<ScreenStates, _>(cx);
        if thread.id() != &owner
            || session.active.as_ref() != Some(&owner)
            || self
                .surface(Facts::of(&thread), can_offer_retry(&states, &owner))
                .as_ref()
                != Some(&surface)
        {
            return Rebuild::No;
        }
        let hs = read::<Hs, _>(cx);
        match (surface, action) {
            (Surface::Waiting { retry, .. }, Action::OfferRetry) => self.confirming = retry,
            (Surface::ConfirmRetry { check, .. }, Action::KeepWaiting) => {
                self.dismissed = Some(check);
                self.confirming = None;
            }
            (Surface::ConfirmRetry { check, offer }, Action::StopAndRetry) => {
                self.dismissed = Some(check);
                self.confirming = None;
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Chat(chat::Intent::Safety {
                        thread: owner,
                        verb: Verb::Retry(offer),
                    }),
                );
            }
            (Surface::Waiting { check, .. }, Action::Dismiss) => self.dismissed = Some(check),
            (Surface::Waiting { .. }, Action::Learn) => {
                crate::features::open_transcript_link(
                    cx,
                    owner,
                    strings::LEARN_MORE_URL.to_owned(),
                );
                return Rebuild::No;
            }
            (Surface::Precaution(Some(id)), Action::Review) => {
                if let Some(review) = thread.safety().violation().and_then(|v| v.review.as_ref()) {
                    self.viewed = Some(id);
                    show_findings(cx, review);
                }
            }
            (
                Surface::Findings {
                    review,
                    can_continue: true,
                },
                Action::Acknowledge,
            ) => {
                self.viewed = None;
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Chat(chat::Intent::Safety {
                        thread: owner,
                        verb: Verb::Continue { review },
                    }),
                );
            }
            (Surface::Findings { .. }, Action::Back) => self.viewed = None,
            (Surface::Precaution(_), Action::New) => cx.emit(
                &hs.screen,
                ScreenIntent::Chat(chat::Intent::Session {
                    thread: Some(owner),
                    command: SlashCommand::New,
                    intent: SessionIntent::New { name: None },
                }),
            ),
            (Surface::Precaution(_), Action::Agents) => open(cx, Target::Agents),
            (Surface::Precaution(_), Action::Resume) => open(cx, Target::Resume),
            _ => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Slot, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let current = select::<ThreadModel, _, _>(cx, Facts::of);
        let owner = current.thread.clone();
        let stopped = current.precaution.is_some();
        let can_retry = current.retry.is_some()
            && select::<ScreenStates, _, _>(cx, {
                let owner = owner.clone();
                move |states| can_offer_retry(states, &owner)
            });
        let surface = self.surface(current, can_retry);
        // Edit controllers and focus only after the old editor has left the tree.
        let first_stop = !self.stopped.replace(stopped) && stopped;
        if first_stop && let Some(binding) = AppBinding::maybe_of(cx) {
            let on_stop = w.on_stop.clone();
            binding.add_post_frame_callback(cx.callback_with(move |_, _, cx| {
                cx.emit(&on_stop, ());
                Rebuild::No
            }));
        }
        if self.showing_menu.replace(surface.is_some())
            && surface.is_none()
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let focus = w.composer_focus.clone();
            binding.add_post_frame_callback(cx.callback_with(move |_, _, _| {
                focus.request_focus();
                Rebuild::No
            }));
        }
        let Some(surface) = surface else {
            return w.child.clone();
        };
        let on_action = {
            let surface = surface.clone();
            cx.map(move |action| Decision {
                owner: owner.clone(),
                surface: surface.clone(),
                action,
            })
        };
        Menu { surface, on_action }.into()
    }
}

fn show_findings(cx: &mut Cx<'_, SlotState>, review: &Review) {
    // The pager can display ANSI, so server-provided findings are supplied as plain text.
    let findings: String = review
        .findings()
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect();
    let text = if let Some(message) = review.continuation() {
        strings::quoted(message, &findings)
    } else {
        findings
    };
    let doc = PagerDoc {
        title: strings::FINDINGS.to_owned(),
        text,
        action: None,
    };
    drop(show_dialog::<(), _>(
        cx,
        move |_| pager::covering(doc.clone()),
        pager::cover_options(),
    ));
}

#[derive(Clone, PartialEq)]
struct Menu {
    surface: Surface,
    on_action: Handler<Action>,
}

#[derive(Clone, Copy, PartialEq)]
enum KeyMsg {
    Move(i64),
    Enter,
    Pick(usize),
    Escape,
}

impl StatefulWidget for Menu {
    type State = OptionsState;

    fn create_state(&self) -> Self::State {
        OptionsState {
            options: Options::new(),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Menu);

struct OptionsState {
    options: Options,
    node: FocusNode,
}

impl State for OptionsState {
    type Widget = Menu;
    type Msg = KeyMsg;

    fn init_state(&mut self, _: &Menu, cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            let node = self.node.clone();
            binding.add_post_frame_callback(cx.callback_with(move |_, _, _| {
                node.request_focus();
                Rebuild::No
            }));
        }
    }

    fn did_update_widget(&mut self, old: &Menu, new: &Menu, _: &mut Cx<'_, Self>) {
        if old.surface != new.surface {
            self.options.set(0);
        }
    }

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: KeyMsg, w: &Menu, cx: &mut Cx<'_, Self>) -> Rebuild {
        let actions = w.surface.actions();
        let action = match msg {
            KeyMsg::Move(step) => {
                self.options.step(step, actions.len());
                return Rebuild::No;
            }
            KeyMsg::Enter => actions.get(self.options.cursor()).copied(),
            KeyMsg::Pick(row) => actions.get(row).copied(),
            KeyMsg::Escape if matches!(w.surface, Surface::Waiting { .. }) => Some(Action::Dismiss),
            KeyMsg::Escape if matches!(w.surface, Surface::ConfirmRetry { .. }) => {
                Some(Action::KeepWaiting)
            }
            KeyMsg::Escape => None,
        };
        if let Some(action) = action {
            cx.emit(&w.on_action, action);
        }
        Rebuild::No
    }

    fn build(&self, w: &Menu, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let (title, detail) = match &w.surface {
            Surface::Waiting { retry, .. } => {
                (WAITING, retry.as_ref().map(|_| strings::WITH_RETRY))
            }
            Surface::ConfirmRetry { .. } => (strings::RETRY_TITLE, Some(strings::RETRY_DETAIL)),
            Surface::Retrying => (strings::RETRYING, None),
            Surface::Precaution(Some(_)) => (strings::PAUSED, Some(strings::PAUSED_DETAIL)),
            Surface::Precaution(_) => (strings::STOPPED, Some(strings::STOPPED_DETAIL)),
            Surface::Findings { .. } => (strings::PAUSED, None),
        };
        let inset = |text: String, style| {
            Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::new(text).style(style).wrap(TextWrap::Word),
            )
        };
        let mut pane = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(inset(title.to_owned(), tokens.bold()));
        if let Some(detail) = detail {
            if matches!(w.surface, Surface::ConfirmRetry { .. }) {
                pane = pane.child(SizedBox::new(None, Some(1), None));
            }
            pane = pane.child(inset(detail.to_owned(), tokens.dim()));
        }
        if let Surface::ConfirmRetry { offer, .. } = &w.surface {
            let model = offer.model().to_owned();
            let name = select::<ScreenStates, _, _>(cx, move |states| {
                states
                    .select
                    .catalog
                    .presets
                    .iter()
                    .find(|preset| preset.model == model)
                    .map_or_else(|| model.clone(), |preset| preset.name.clone())
            });
            pane = pane
                .child(SizedBox::new(None, Some(1), None))
                .child(inset(strings::retry_model(&name), tokens.dim()));
        }
        if matches!(w.surface, Surface::Retrying) {
            return pane.into();
        }
        let rows: Vec<_> = w
            .surface
            .actions()
            .iter()
            .enumerate()
            .map(|(index, action)| RichRow {
                prefix: format!("{}. ", index + 1),
                label: vec![Span::styled(
                    action.label(),
                    if *action == Action::Acknowledge
                        && matches!(
                            w.surface,
                            Surface::Findings {
                                can_continue: false,
                                ..
                            }
                        )
                    {
                        tokens.dim()
                    } else {
                        Style::new()
                    },
                )],
                description: None,
            })
            .collect();
        let options = self.options.on_accept(cx.map(KeyMsg::Pick)).view_rich(
            Rc::new(rows),
            OptionStyles {
                cursor: tokens.accent(),
                label: Style::new(),
                description: tokens.dim(),
            },
            0,
        );
        pane = pane
            .child(SizedBox::new(None, Some(2), None))
            .child(options);
        if matches!(w.surface, Surface::Waiting { .. }) {
            pane = pane
                .child(SizedBox::new(None, Some(1), None))
                .child(inset(strings::WAIT_FOOTER.to_owned(), tokens.dim()));
        }
        let mut focus = Focus::new(pane).focus_node(self.node.clone()).autofocus();
        let keys = crate::keymap::keymap(cx);
        for (id, msg) in [
            (actions::LIST_MOVE_UP, KeyMsg::Move(-1)),
            (actions::LIST_MOVE_DOWN, KeyMsg::Move(1)),
            (actions::LIST_ACCEPT, KeyMsg::Enter),
            (actions::LIST_CANCEL, KeyMsg::Escape),
            (fixed::ESCAPE, KeyMsg::Escape),
        ] {
            for key in crate::keymap::presses(&keys, id) {
                focus = focus.bind(key, cx.send(msg));
            }
        }
        for (index, digit) in ['1', '2', '3', '4']
            .into_iter()
            .take(w.surface.actions().len())
            .enumerate()
        {
            focus = focus.bind(digit, cx.send(KeyMsg::Pick(index)));
        }
        focus.into()
    }
}
