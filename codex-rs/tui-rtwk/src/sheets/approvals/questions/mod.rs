//! The editor of the questions that wait (oracle `bottom_pane/async_questions`): the questions the
//! model asked while it keeps working, answered in place of the composer. One question shows at a
//! time: a progress row (`2 of 3`), the question as Markdown, its choices (a cursor list whose last
//! row, Other, is a text input of its own) or, for a question without choices, the input; the tips
//! under it. The rows each part gets are the oracle's `layout_sections` ([`layout`]), decided from
//! the measured rows ([`measure`]); the parts are composed in boxes of those heights (Flutter: a
//! `Column` of `SizedBox`es under a `LayoutBuilder`). What the user has typed and chosen is the
//! state's (a `TextEditingController` for each question, so moving between questions keeps each
//! draft); what the question asks is the widget's, and what the user decides leaves as an
//! [`Outcome`].

mod entries;
mod layout;
mod measure;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::editing::{EditingKeys, TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{
    Alignment, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize,
    OverflowBox, Padding, Row, SizedBox, SizedOverflowBox,
};
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{DefaultTextStyle, Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use codex_tui_core::features::async_questions::reply;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{
    CHAT_EDIT_QUEUED_MESSAGE, CHAT_INTERRUPT_TURN, CHAT_PROMPT_STACK_BACK, CHAT_SKIP_QUESTION,
    COMPOSER_SUBMIT, LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_BOTTOM, LIST_JUMP_TOP, LIST_MOVE_DOWN,
    LIST_MOVE_LEFT, LIST_MOVE_RIGHT, LIST_MOVE_UP, LIST_PAGE_DOWN, LIST_PAGE_UP,
};

pub use entries::debug_entries;

use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::keymap::{label, only_shift, pressed, unmodified};
use crate::widgets::markdown;
use crate::widgets::measure::Measure;

/// The longest a choice may be (bytes): what the oracle bounds the model's suggestions to.
pub const MAX_OPTION_BYTES: usize = 512;
/// The rows the input of a freeform answer or of Other grows to.
const MAX_INPUT_ROWS: u16 = 8;
/// The cells the editor is inset: two on each side, one above and below.
const INSET_X: i32 = 2;
const INSET_Y: i32 = 1;

/// One question that waits.
#[derive(Clone, Debug, PartialEq)]
pub struct AQuestion {
    /// What the question is called to the model: the reply says which question it answers.
    pub id: String,
    /// The question, Markdown.
    pub title: String,
    /// The suggested answers; none for a question that is answered freely.
    pub options: Vec<String>,
}

/// What the editor starts with (the debug menu's states): the question on show, the choice of
/// each question and what has been typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seed {
    pub current: usize,
    /// The row each question's cursor is on, in the order of the questions; missing: the first.
    pub selected: Vec<Option<usize>>,
    pub drafts: Vec<String>,
}

/// What the editor keeps for the next time it opens (the parent holds it between closings, as the
/// oracle keeps the drafts of the questions while the editor is collapsed): the question on show
/// and, of each question by its id, the row of its cursor and what was typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved {
    pub current: Option<String>,
    pub kept: BTreeMap<String, (Option<usize>, String)>,
}

/// What the user decided; the parent applies it (a reply, a skip, a move out of the editor).
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The answer of the question `id`, as the reply text to send (`queue`: after the turn).
    Submit {
        id: String,
        reply: String,
        queue: bool,
    },
    /// The question `id` is skipped.
    Skip { id: String },
    /// A move past the first or the last question (`forward`).
    Edge { forward: bool },
    /// The chat's interrupt key.
    Interrupt,
}

/// The editor over the questions `questions`.
#[derive(Clone, Debug, PartialEq)]
pub struct AsyncQuestions {
    pub questions: Rc<Vec<AQuestion>>,
    pub seed: Seed,
    /// Messages wait in the queue: the last question's `⇧←` leads to them.
    pub queued: bool,
    /// Only the part of the choices, over the whole area and without the surface (what the
    /// oracle's test of Other in two rows draws).
    pub options_only: bool,
    /// The pane is as tall as the editor wants (`desired_height`) instead of the whole area: the
    /// editor that stands in the composer's place.
    pub fit: bool,
    /// Where the editor leaves its drafts when it closes, and where it starts from.
    pub saved: Option<Rc<RefCell<Saved>>>,
    pub on_outcome: Option<Handler<Outcome>>,
}

/// What one question holds while the editor is open.
struct QState {
    id: String,
    /// The row the cursor is on (the `›` mark); Other is the row after the choices.
    selected: Option<usize>,
    /// The first row of the choices that shows (kept between frames, as the oracle's scroll).
    scroll_top: Rc<Cell<usize>>,
    draft: TextEditingController,
    node: FocusNode,
}

pub struct AsyncState {
    questions: Vec<QState>,
    current: usize,
    /// The choices the last frame showed whole: the first and how many.
    visible: Rc<Cell<(usize, usize)>>,
    flash: Option<&'static str>,
    flash_age: u32,
    timer: Option<Timer>,
    node: FocusNode,
    keymap: Rc<Keymap>,
    saved: Option<Rc<RefCell<Saved>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Press(KeyPress),
    /// Enter in the input.
    Submitted(Rc<str>),
    /// The frame that built the input is done: it takes the keys.
    FocusInput,
    /// A second of a flash passed.
    Tick,
}

fn qstate(question: &AQuestion, selected: Option<Option<usize>>, draft: &str) -> QState {
    let controller = TextEditingController::new(draft);
    controller.move_caret(draft.len(), false);
    QState {
        id: question.id.clone(),
        selected: selected.unwrap_or_else(|| (!question.options.is_empty()).then_some(0)),
        scroll_top: Rc::new(Cell::new(0)),
        draft: controller,
        node: FocusNode::new(),
    }
}

impl StatefulWidget for AsyncQuestions {
    type State = AsyncState;

    fn create_state(&self) -> AsyncState {
        let saved = self.saved.as_ref().map(|saved| saved.borrow().clone());
        let questions: Vec<QState> = self
            .questions
            .iter()
            .enumerate()
            .map(|(at, question)| match saved.as_ref().and_then(|s| s.kept.get(&question.id)) {
                Some((selected, draft)) => qstate(question, Some(*selected), draft),
                None => qstate(
                    question,
                    self.seed.selected.get(at).copied(),
                    self.seed.drafts.get(at).map_or("", String::as_str),
                ),
            })
            .collect();
        let current = saved
            .as_ref()
            .and_then(|s| s.current.as_ref())
            .and_then(|id| self.questions.iter().position(|q| q.id == *id))
            .unwrap_or(self.seed.current);
        AsyncState {
            questions,
            current: current.min(self.questions.len().saturating_sub(1)),
            visible: Rc::new(Cell::new((0, 0))),
            flash: None,
            flash_age: 0,
            timer: None,
            node: FocusNode::new(),
            keymap: Rc::new(Keymap::defaults()),
            saved: self.saved.clone(),
        }
    }
}

impl_widget!(stateful eq AsyncQuestions);

/// Whether `key` types text a plain list does not read (a letter, a digit, a symbol).
fn printable(key: &KeyPress) -> bool {
    key.text().is_some() && (unmodified(key) || only_shift(key))
}

/// The list action `key` is bound to, if the keymap's list has one.
fn list_action(keymap: &Keymap, key: &KeyPress) -> Option<ActionId> {
    [
        LIST_MOVE_UP,
        LIST_MOVE_DOWN,
        LIST_PAGE_UP,
        LIST_PAGE_DOWN,
        LIST_JUMP_TOP,
        LIST_JUMP_BOTTOM,
        LIST_ACCEPT,
        LIST_CANCEL,
        LIST_MOVE_LEFT,
        LIST_MOVE_RIGHT,
    ]
    .into_iter()
    .find(|id| pressed(keymap, *id, key))
}

/// Whether the user bound the list action `id` to something else than its default (a printable
/// key keeps its meaning of typing, unless the user remapped it).
fn remapped(keymap: &Keymap, id: ActionId) -> bool {
    keymap.specs(id) != Keymap::defaults().specs(id)
}

/// The keys the input hands up before the text area reads them: the moves between questions,
/// the skip and, while Other holds the input, the arrows that move the choices.
fn input_takes(keymap: &Keymap, key: &KeyPress, other: bool) -> bool {
    pressed(keymap, CHAT_SKIP_QUESTION, key)
        || pressed(keymap, CHAT_EDIT_QUEUED_MESSAGE, key)
        || pressed(keymap, CHAT_PROMPT_STACK_BACK, key)
        || (other
            && (pressed(keymap, LIST_MOVE_UP, key) || pressed(keymap, LIST_MOVE_DOWN, key))
            && !printable(key))
}

/// The keys the editor reads on its own node: those the choices and the chat use.
fn root_takes(keymap: &Keymap, key: &KeyPress) -> bool {
    input_takes(keymap, key, true)
        || pressed(keymap, CHAT_INTERRUPT_TURN, key)
        || list_action(keymap, key).is_some()
        || printable(key)
}

impl AsyncState {
    fn question<'a>(&self, w: &'a AsyncQuestions) -> Option<&'a AQuestion> {
        w.questions.get(self.current)
    }

    fn options<'a>(&self, w: &'a AsyncQuestions) -> &'a [String] {
        self.question(w).map_or(&[], |q| q.options.as_slice())
    }

    fn selected(&self) -> Option<usize> {
        self.questions.get(self.current).and_then(|q| q.selected)
    }

    fn other_selected(&self, w: &AsyncQuestions) -> bool {
        let options = self.options(w);
        !options.is_empty() && self.selected() == Some(options.len())
    }

    fn focus_is_input(&self, w: &AsyncQuestions) -> bool {
        self.options(w).is_empty() || self.other_selected(w)
    }

    fn select(&mut self, w: &AsyncQuestions, row: usize) {
        if let Some(q) = self.questions.get_mut(self.current) {
            q.selected = Some(row);
        }
        self.after_select(w);
    }

    fn after_select(&mut self, w: &AsyncQuestions) {
        if self.focus_is_input(w)
            && let Some(q) = self.questions.get(self.current)
        {
            q.node.request_focus();
        } else {
            self.node.request_focus();
        }
    }

    fn navigate(&mut self, w: &AsyncQuestions, forward: bool, cx: &mut Cx<'_, Self>) {
        let next = if forward {
            self.current.checked_add(1)
        } else {
            self.current.checked_sub(1)
        };
        match next.filter(|at| *at < w.questions.len()) {
            Some(at) => {
                self.current = at;
                self.visible.set((0, 0));
                self.after_select(w);
            }
            None => self.emit(w, Outcome::Edge { forward }, cx),
        }
    }

    fn emit(&self, w: &AsyncQuestions, outcome: Outcome, cx: &mut Cx<'_, Self>) {
        if let Some(handler) = &w.on_outcome {
            cx.emit(handler, outcome);
        }
    }

    fn flash(&mut self, text: &'static str, cx: &mut Cx<'_, Self>) {
        self.flash = Some(text);
        self.flash_age = 0;
        if self.timer.is_none()
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            self.timer =
                Some(binding.start_periodic(Duration::from_secs(1), cx.map(|(): ()| Msg::Tick)));
        }
    }

    /// The oracle's `go_next_or_submit`: the answer of the question on show leaves, unless a
    /// choice is not shown whole (it must be read before it speaks for the user) or the reply
    /// is too long.
    fn submit(&mut self, w: &AsyncQuestions, queue: bool, cx: &mut Cx<'_, Self>) {
        let s = strings::of(cx);
        let Some(question) = self.question(w) else {
            return;
        };
        let Some(state) = self.questions.get(self.current) else {
            return;
        };
        let input = self.focus_is_input(w);
        let (first, count) = self.visible.get();
        let at = state.selected.unwrap_or(0);
        if !input && !(first..first + count).contains(&at) {
            self.flash(s.aq_expand, cx);
            return;
        }
        let text = if input {
            state.draft.text().to_string()
        } else {
            question.options.get(at).cloned().unwrap_or_default()
        };
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let id = question.id.clone();
        let reply = reply::render(&id, &question.title, text);
        if reply.chars().take(MAX_USER_INPUT_TEXT_CHARS + 1).count() > MAX_USER_INPUT_TEXT_CHARS {
            self.flash(s.aq_too_long, cx);
            return;
        }
        self.emit(w, Outcome::Submit { id, reply, queue }, cx);
    }

    /// A key the list reads: the cursor moves, the page moves, the jumps.
    fn list_key(&mut self, w: &AsyncQuestions, key: &KeyPress, cx: &mut Cx<'_, Self>) -> bool {
        let count = self.options(w).len() + 1;
        let visible = self.visible.get().1.max(1);
        let Some(action) = list_action(&self.keymap, key) else {
            return false;
        };
        let current = self.selected();
        let clamp = |at: usize| at.min(count - 1);
        let next = match action {
            a if a == LIST_MOVE_UP => Some(match current {
                Some(at) if at > 0 => at - 1,
                Some(_) => count - 1,
                None => 0,
            }),
            a if a == LIST_MOVE_DOWN => Some(match current {
                Some(at) if at + 1 < count => at + 1,
                _ => 0,
            }),
            a if a == LIST_PAGE_UP => Some(clamp(current.unwrap_or(0)).saturating_sub(visible)),
            a if a == LIST_PAGE_DOWN => Some(clamp(clamp(current.unwrap_or(0)) + visible)),
            a if a == LIST_JUMP_TOP => Some(0),
            a if a == LIST_JUMP_BOTTOM => Some(count - 1),
            a if a == LIST_CANCEL => {
                // The oracle collapses the editor; the chat page owns that.
                self.emit(w, Outcome::Edge { forward: false }, cx);
                return true;
            }
            a if a == LIST_MOVE_LEFT || a == LIST_MOVE_RIGHT => return true,
            _ => None,
        };
        match next {
            Some(row) => {
                self.select(w, row);
                true
            }
            None => false,
        }
    }

    fn press(&mut self, key: &KeyPress, w: &AsyncQuestions, cx: &mut Cx<'_, Self>) {
        let keymap = Rc::clone(&self.keymap);
        let input = self.focus_is_input(w);
        let options = !self.options(w).is_empty();
        if pressed(&keymap, CHAT_INTERRUPT_TURN, key) && !(input && printable(key)) {
            self.emit(w, Outcome::Interrupt, cx);
            return;
        }
        if pressed(&keymap, CHAT_SKIP_QUESTION, key) {
            if let Some(question) = self.question(w) {
                let id = question.id.clone();
                self.emit(w, Outcome::Skip { id }, cx);
            }
            return;
        }
        if pressed(&keymap, CHAT_EDIT_QUEUED_MESSAGE, key) {
            self.navigate(w, true, cx);
            return;
        }
        if pressed(&keymap, CHAT_PROMPT_STACK_BACK, key) {
            self.navigate(w, false, cx);
            return;
        }
        if options {
            // Plain letters start Other; a list key the user remapped to a letter keeps its
            // meaning.
            let action = list_action(&keymap, key);
            let is_remapped = action.is_some_and(|id| remapped(&keymap, id));
            let up_down =
                pressed(&keymap, LIST_MOVE_UP, key) || pressed(&keymap, LIST_MOVE_DOWN, key);
            if (!printable(key) || is_remapped)
                && (!input || up_down)
                && action.is_some_and(|id| id != LIST_ACCEPT)
                && self.list_key(w, key, cx)
            {
                return;
            }
        }
        if input {
            return;
        }
        if pressed(&keymap, LIST_ACCEPT, key) {
            self.submit(w, false, cx);
        } else if printable(key) && unmodified(key) | only_shift(key) {
            let text = key.text().unwrap_or_default().to_string();
            let digit = text
                .chars()
                .next()
                .and_then(|c| c.to_digit(10))
                .and_then(|d| usize::try_from(d).ok())
                .and_then(|d| d.checked_sub(1))
                .filter(|at| *at <= self.options(w).len());
            match digit {
                Some(row) => {
                    self.select(w, row);
                    if !self.other_selected(w) {
                        self.submit(w, false, cx);
                    }
                }
                None => {
                    let other = self.options(w).len();
                    self.select(w, other);
                    if let Some(state) = self.questions.get(self.current) {
                        state.draft.insert(&text);
                    }
                }
            }
        }
    }
}

impl State for AsyncState {
    type Widget = AsyncQuestions;
    type Msg = Msg;

    fn init_state(&mut self, w: &AsyncQuestions, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.keymap = read::<Keymap, _>(cx);
        let keymap = Rc::clone(&self.keymap);
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key: KeyPress| {
                if root_takes(&keymap, &key) {
                    send(Msg::Press(key));
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            })));
        if self.focus_is_input(w) {
            self.after_select(w);
        }
        // mounted inside a page, where something else may hold the focus: take it once the
        // frame has put the nodes in the tree
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::FocusInput));
        }
    }

    fn did_update_widget(
        &mut self,
        _old: &AsyncQuestions,
        w: &AsyncQuestions,
        _cx: &mut Cx<'_, Self>,
    ) {
        // The parent drops a question that was answered or skipped: the state follows by id.
        let mut next = Vec::new();
        for question in w.questions.iter() {
            match self.questions.iter().position(|q| q.id == question.id) {
                Some(at) => next.push(self.questions.remove(at)),
                None => next.push(qstate(question, None, "")),
            }
        }
        let shown = self.questions.get(self.current).map(|q| q.id.clone());
        let _ = shown;
        self.questions = next;
        self.current = self.current.min(w.questions.len().saturating_sub(1));
    }

    fn dispose(&mut self) {
        if let Some(saved) = &self.saved {
            *saved.borrow_mut() = Saved {
                current: self.questions.get(self.current).map(|q| q.id.clone()),
                kept: self
                    .questions
                    .iter()
                    .map(|q| (q.id.clone(), (q.selected, q.draft.text().to_string())))
                    .collect(),
            };
        }
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        self.node.set_on_key_event(None);
        for q in &self.questions {
            q.draft.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &AsyncQuestions, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Press(key) => self.press(&key, w, cx),
            Msg::Submitted(text) => {
                if text.trim().is_empty() {
                    return Rebuild::No;
                }
                self.submit(w, false, cx);
            }
            Msg::FocusInput => {
                if self.focus_is_input(w)
                    && let Some(q) = self.questions.get(self.current)
                {
                    q.node.request_focus();
                } else {
                    self.node.request_focus();
                }
                return Rebuild::No;
            }
            Msg::Tick => {
                self.flash_age += 1;
                if self.flash_age >= 5 {
                    self.flash = None;
                    if let Some(timer) = self.timer.take() {
                        timer.cancel();
                    }
                }
            }
        }
        if self.focus_is_input(w)
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::FocusInput));
        }
        Rebuild::Yes
    }

    fn build(&self, w: &AsyncQuestions, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let surface = tokens.prompt_surface(cx, false);
        let Some(question) = self.question(w) else {
            return BoxWidget::new().style(surface).into();
        };
        let state = &self.questions[self.current];
        let keymap = crate::keymap::keymap(cx);
        let send = cx.local_sender();
        let other = self.other_selected(w);
        let view = View {
            title: question.title.clone(),
            options: question.options.clone(),
            selected: state.selected,
            at: self.current,
            total: w.questions.len(),
            draft: state.draft.clone(),
            scroll_top: Rc::clone(&state.scroll_top),
            visible: Rc::clone(&self.visible),
            node: state.node.clone(),
            flash: self.flash,
            queued: w.queued,
            options_only: w.options_only,
            fit: w.fit,
            tokens: tokens.clone(),
            s,
            keymap: Rc::clone(&keymap),
            markdown: tokens.transcript_markdown(cx),
            on_submit: cx.map(Msg::Submitted),
            take: {
                let keymap = Rc::clone(&keymap);
                Rc::new(move |key: &KeyPress| {
                    if input_takes(&keymap, key, other) {
                        send(Msg::Press(key.clone()));
                        true
                    } else {
                        false
                    }
                })
            },
        };
        let options_only = w.options_only;
        let body: AnyWidget =
            LayoutBuilder::new(move |cx, constraints| view.layout(cx, constraints)).into();
        if options_only {
            return body;
        }
        let focus = Focus::new(
            BoxWidget::new()
                .style(surface)
                .child(Padding::new(EdgeInsets::symmetric(INSET_X, INSET_Y), body)),
        )
        .focus_node(self.node.clone());
        if self.focus_is_input(w) {
            focus.into()
        } else {
            focus.autofocus().into()
        }
    }
}

/// Everything the layout needs of the question on show, owned (the closure of a `LayoutBuilder`
/// outlives the build).
struct View {
    title: String,
    options: Vec<String>,
    selected: Option<usize>,
    at: usize,
    total: usize,
    draft: TextEditingController,
    scroll_top: Rc<Cell<usize>>,
    visible: Rc<Cell<(usize, usize)>>,
    node: FocusNode,
    flash: Option<&'static str>,
    queued: bool,
    options_only: bool,
    fit: bool,
    tokens: Tokens,
    s: &'static Strings,
    keymap: Rc<Keymap>,
    markdown: rtwk_rich::markdown::MarkdownTheme,
    on_submit: Handler<Rc<str>>,
    take: Rc<dyn Fn(&KeyPress) -> bool>,
}

fn rows_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

fn px(n: u16) -> i32 {
    i32::from(n)
}

impl View {
    fn has_options(&self) -> bool {
        !self.options.is_empty()
    }

    fn other_selected(&self) -> bool {
        self.has_options() && self.selected == Some(self.options.len())
    }

    fn focus_is_input(&self) -> bool {
        !self.has_options() || self.other_selected()
    }

    /// The label Other shows while it is not the row on: what was typed, cut, else its name.
    fn other_label(&self) -> String {
        let typed = self.draft.text();
        let typed = typed.trim();
        if self.other_selected() || typed.is_empty() {
            return self.other_placeholder().to_owned();
        }
        let bounded = &typed[..typed.floor_char_boundary(MAX_OPTION_BYTES)];
        cut_graphemes(bounded, 128)
    }

    fn other_placeholder(&self) -> &'static str {
        if self
            .options
            .iter()
            .any(|o| o.eq_ignore_ascii_case(self.s.aq_other))
        {
            self.s.aq_other_named
        } else {
            self.s.aq_other
        }
    }

    fn placeholder(&self) -> &'static str {
        if self.other_selected() {
            self.other_placeholder()
        } else {
            self.s.aq_placeholder
        }
    }

    /// The width of `› N. `, the mark and the number of a row.
    fn prefix_width(&self, seg: Measure) -> usize {
        seg.width(&format!("› {}. ", self.options.len() + 1))
    }

    fn row_prefix(&self, at: usize) -> String {
        let mark = if self.selected == Some(at) {
            '›'
        } else {
            ' '
        };
        format!("{mark} {}. ", at + 1)
    }

    /// The rows of each choice at `width` (not Other).
    fn choice_rows(&self, seg: Measure, width: usize) -> Vec<usize> {
        self.options
            .iter()
            .enumerate()
            .map(|(at, label)| {
                let indent = seg.width(&self.row_prefix(at));
                if width > indent {
                    measure::markdown_rows(seg, label, width - indent).max(1)
                } else {
                    measure::rows(seg, &format!("{}{label}", self.row_prefix(at)), width)
                }
            })
            .collect()
    }

    fn other_rows(&self, seg: Measure, width: usize) -> usize {
        let indent = seg.width(&self.row_prefix(self.options.len()));
        if width > indent {
            measure::rows(seg, &self.other_label(), width - indent)
        } else {
            measure::rows(
                seg,
                &format!(
                    "{}{}",
                    self.row_prefix(self.options.len()),
                    self.other_label()
                ),
                width,
            )
        }
    }

    fn other_prefix_width(&self, seg: Measure, width: u16) -> u16 {
        let full = rows_u16(self.prefix_width(seg));
        if full + 2 <= width {
            full
        } else {
            width.saturating_sub(2).min(2)
        }
    }

    fn input_rows(&self, seg: Measure, width: usize) -> u16 {
        // the flash of a refused answer takes a row under the input (`inline_input_height`)
        let flash = u16::from(self.flash.is_some() && !self.has_options());
        (rows_u16(measure::rows(seg, &self.draft.text(), width.max(1))) + flash)
            .clamp(1, MAX_INPUT_ROWS)
    }

    /// The rows the choices take at `width`, with the input of Other when it is the row on.
    fn options_rows(&self, seg: Measure, width: u16) -> u16 {
        if !self.has_options() {
            return 0;
        }
        let named: usize = self.choice_rows(seg, usize::from(width)).iter().sum();
        if self.other_selected() {
            let prefix = self.other_prefix_width(seg, width);
            let input = self.input_rows(seg, usize::from(width.saturating_sub(prefix).max(1)));
            rows_u16(named) + input
        } else {
            rows_u16(named + self.other_rows(seg, usize::from(width)))
        }
    }

    fn markdown_source(&self) -> String {
        self.title.clone()
    }

    fn question_rows(&self, seg: Measure, width: u16) -> u16 {
        rows_u16(measure::markdown_rows(
            seg,
            &self.markdown_source(),
            usize::from(width.max(1)),
        ))
    }

    fn tip(&self, keys: &str, text: &str) -> Vec<Span> {
        vec![
            Span::styled(
                keys.to_owned(),
                self.tokens.bold().attrs(rtwk::style::Attrs::DIM),
            ),
            Span::styled(self.s.key_gap, self.tokens.dim()),
            Span::styled(text.to_owned(), self.tokens.dim()),
        ]
    }

    /// The tips under the question (`footer_lines`), packed into rows `width` wide.
    fn footer(&self, seg: Measure, width: u16, option_tip: bool) -> Vec<Vec<Span>> {
        if !self.focus_is_input()
            && let Some(flash) = self.flash
        {
            return vec![vec![Span::styled(flash, self.tokens.dim())]];
        }
        let s = self.s;
        let mut tips: Vec<Vec<Span>> = Vec::new();
        let input = self.focus_is_input();
        let submit = if input { COMPOSER_SUBMIT } else { LIST_ACCEPT };
        // A chat key bound to the same stroke wins over the list's accept.
        let taken = |id: ActionId| {
            let own = self.keymap.specs(submit);
            self.keymap.specs(id).iter().any(|spec| own.contains(spec))
        };
        if let Some(key) = label(&self.keymap, submit)
            && (input || !(taken(CHAT_INTERRUPT_TURN) || taken(CHAT_EDIT_QUEUED_MESSAGE)))
        {
            tips.push(self.tip(&key, s.aq_tip_submit));
        }
        if let Some(key) = label(&self.keymap, CHAT_SKIP_QUESTION) {
            tips.push(self.tip(&key, s.aq_tip_skip));
        }
        if option_tip {
            tips.push(vec![Span::styled(
                s.aq_tip_option(self.selected.unwrap_or(0) + 1, self.options.len() + 1)
                    .to_string(),
                self.tokens.dim(),
            )]);
        }
        if let Some(key) = label(&self.keymap, CHAT_PROMPT_STACK_BACK) {
            let text = if self.at > 0 {
                s.aq_tip_prev
            } else {
                s.aq_tip_main
            };
            tips.push(self.tip(&key, text));
        }
        let next = if self.at + 1 < self.total {
            Some(s.aq_tip_next)
        } else if self.queued {
            Some(s.aq_tip_queued)
        } else {
            None
        };
        if let Some(text) = next
            && let Some(key) = label(&self.keymap, CHAT_EDIT_QUEUED_MESSAGE)
        {
            tips.push(self.tip(&key, text));
        }
        measure::pack(seg, tips, s.aq_tip_gap, usize::from(width))
    }

    fn layout(&self, cx: &mut Cx<'_>, constraints: rtwk::layout::BoxConstraints) -> AnyWidget {
        let seg = segmenter_of(cx);
        let width = constraints.max_width.max(0);
        let height = constraints.max_height.max(0);
        if width == 0 || height == 0 {
            return AnyWidget::default();
        }
        let (w16, h16) = (
            rows_u16(usize::try_from(width).unwrap_or(0)),
            rows_u16(usize::try_from(height).unwrap_or(0)),
        );
        if self.options_only {
            return self.options_widget(seg, width, h16);
        }
        let facts = layout::Facts {
            has_options: self.has_options(),
            other_selected: self.other_selected(),
            several: self.total > 1,
            question_rows: self.question_rows(seg, w16),
            options_rows: self.options_rows(seg, w16),
            input_rows: self.input_rows(seg, usize::from(w16)),
            footer_rows: rows_u16(self.footer(seg, w16, false).len()),
            footer_rows_with_tip: rows_u16(self.footer(seg, w16, true).len()),
        };
        // the pane the oracle sizes to its content (`desired_height`): the question, the choices
        // or the input, the spacers, the progress row and the footer
        let h16 = if self.fit {
            let wanted = facts.question_rows
                + facts.options_rows
                + if facts.has_options { 0 } else { facts.input_rows.clamp(1, 8) }
                + layout::DESIRED_SPACERS
                + u16::from(!facts.has_options && facts.several)
                + facts.footer_rows
                + u16::from(facts.several);
            h16.min(wanted)
        } else {
            h16
        };
        let sections = layout::sections(h16, facts);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let blank = |rows: u16| -> AnyWidget {
            SizedBox::new(None, Some(px(rows)), None::<AnyWidget>).into()
        };
        if sections.progress > 0 {
            let text = self.s.aq_progress(self.at + 1, self.total).to_string();
            column = column.child(boxed(
                width,
                sections.progress,
                1,
                Text::new(text)
                    .style(self.tokens.dim())
                    .wrap(TextWrap::None),
            ));
        }
        if sections.spacer_before_question > 0 {
            column = column.child(blank(sections.spacer_before_question));
        }
        if sections.question > 0 {
            let md = DefaultTextStyle::new(
                self.tokens.accent(),
                markdown::themed(
                    markdown::view(&self.markdown_source()),
                    self.markdown.clone(),
                ),
            );
            column = column.child(boxed(width, sections.question, facts.question_rows, md));
        }
        if sections.spacer_after_question > 0 {
            column = column.child(blank(sections.spacer_after_question));
        }
        if sections.options > 0 {
            column = column.child(self.options_widget(seg, width, sections.options));
        }
        if sections.spacer_after_options > 0 {
            column = column.child(blank(sections.spacer_after_options));
        }
        if sections.notes > 0 {
            column = column.child(self.input_widget(sections.notes, self.placeholder()));
        }
        if sections.spacer_after_input > 0 {
            column = column.child(blank(sections.spacer_after_input));
        }
        if sections.footer > 0 {
            let lines = self.footer(seg, w16, sections.show_option_tip);
            let mut footer = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            let count = lines.len();
            for line in lines {
                let cut = measure::truncate(seg, line, usize::from(w16));
                footer = footer.child(Text::rich(cut).wrap(TextWrap::None));
            }
            column = column.child(boxed(width, sections.footer, rows_u16(count), footer));
        }
        column.into()
    }

    /// The input of a freeform answer or of Other, `rows` high.
    fn input_widget(&self, rows: u16, placeholder: &'static str) -> AnyWidget {
        // under the input, the flash of a refused answer (`render_inline_input`)
        if let Some(flash) = self.flash.filter(|_| !self.has_options() && rows > 1) {
            let input = self.input_widget_rows(rows - 1, placeholder);
            let line = SizedBox::new(
                None,
                Some(px(1)),
                Some(AnyWidget::from(Text::new(flash).wrap(TextWrap::None))),
            );
            return Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(input)
                .child(line)
                .into();
        }
        self.input_widget_rows(rows, placeholder)
    }

    fn input_widget_rows(&self, rows: u16, placeholder: &'static str) -> AnyWidget {
        let lines = usize::from(rows.max(1));
        let take = Rc::clone(&self.take);
        let area = TextArea::new()
            .controller(self.draft.clone())
            .placeholder(placeholder)
            .placeholder_style(self.tokens.dim())
            .min_lines(lines)
            .max_lines(lines)
            .focus_node(self.node.clone())
            .autofocus(true)
            .on_submitted(self.on_submit.clone());
        let keyed =
            EditingKeys::new(move |_, key, _| take(key), area).controller(self.draft.clone());
        SizedBox::new(None, Some(px(rows)), Some(AnyWidget::from(keyed))).into()
    }

    /// The choices (and Other) in the `rows` the layout gave them.
    fn options_widget(&self, seg: Measure, width: i32, rows: u16) -> AnyWidget {
        let w16 = rows_u16(usize::try_from(width).unwrap_or(0));
        let available = usize::from(rows);
        let choice = self.choice_rows(seg, usize::from(w16));
        let count = self.options.len();
        if self.other_selected() {
            let prefix = self.other_prefix_width(seg, w16);
            let input_rows = self
                .input_rows(seg, usize::from(w16.saturating_sub(prefix).max(1)))
                .min(rows);
            let named = available - usize::from(input_rows);
            // The choices from the first row that lets the rest fit, at least the last one.
            let mut first = 0;
            while first + 1 < count && choice[first..].iter().sum::<usize>() > named {
                first += 1;
            }
            if count > 0 && choice[first..].iter().sum::<usize>() <= named {
                self.visible.set((first, count - first));
            } else {
                self.visible.set((0, 0));
            }
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            let total: usize = choice.get(first..).map_or(0, |r| r.iter().sum());
            for (at, rows) in choice.iter().enumerate().take(count).skip(first) {
                column = column.child(self.choice_widget(at, *rows));
            }
            let mark = if i32::from(prefix) < i32::try_from(self.prefix_width(seg)).unwrap_or(0) {
                "› ".to_owned()
            } else {
                format!("› {}. ", count + 1)
            };
            let input = Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(
                    Some(i32::from(prefix)),
                    Some(1),
                    Some(AnyWidget::from(
                        Text::new(mark)
                            .style(self.tokens.accent())
                            .wrap(TextWrap::None),
                    )),
                ))
                .child(Expanded::new(
                    1,
                    self.input_widget(input_rows, self.placeholder()),
                ));
            let mut whole = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            if named > 0 {
                whole = whole.child(boxed(width, rows_u16(named), rows_u16(total), column));
            }
            return whole
                .child(SizedBox::new(
                    None,
                    Some(px(input_rows)),
                    Some(AnyWidget::from(input)),
                ))
                .into();
        }
        // The scroll: the first row the cursor's row fits under, from the last first row.
        let selected = self.selected.unwrap_or(0).min(count);
        let mut heights = choice;
        heights.push(self.other_rows(seg, usize::from(w16)));
        let top = if heights.iter().sum::<usize>() <= available {
            0
        } else {
            self.scroll_top.get()
        };
        let mut first = top.min(selected);
        while first < selected && heights[first..=selected].iter().sum::<usize>() > available {
            first += 1;
        }
        let mut used = 0;
        let visible = heights[first..]
            .iter()
            .take_while(|h| {
                used += **h;
                used <= available
            })
            .count();
        self.scroll_top.set(first);
        // Other is not a choice that speaks for the user: the count of shown choices stops at it.
        self.visible.set((first, visible));
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (at, rows) in heights.iter().enumerate().skip(first) {
            column = if at < count {
                column.child(self.choice_widget(at, *rows))
            } else {
                column.child(self.other_widget(*rows))
            };
        }
        let total: usize = heights[first..].iter().sum();
        boxed(width, rows, rows_u16(total), column)
    }

    fn row_style(&self, at: usize) -> Style {
        if self.selected == Some(at) {
            self.tokens.accent()
        } else {
            Style::new()
        }
    }

    fn choice_widget(&self, at: usize, rows: usize) -> AnyWidget {
        let prefix = Text::new(self.row_prefix(at))
            .style(self.row_style(at))
            .wrap(TextWrap::None);
        let label = markdown::themed(markdown::view(&self.options[at]), self.markdown.clone());
        let row = Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(prefix)
            .child(Expanded::new(1, label));
        SizedBox::new(
            None,
            Some(i32::try_from(rows).unwrap_or(1)),
            Some(AnyWidget::from(row)),
        )
        .into()
    }

    fn other_widget(&self, rows: usize) -> AnyWidget {
        let at = self.options.len();
        let prefix = Text::new(self.row_prefix(at))
            .style(self.row_style(at))
            .wrap(TextWrap::None);
        let row = Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(prefix)
            .child(Expanded::new(1, Text::new(self.other_label())));
        SizedBox::new(
            None,
            Some(i32::try_from(rows).unwrap_or(1)),
            Some(AnyWidget::from(row)),
        )
        .into()
    }
}

/// `child` laid out at its `natural` rows and cut to the `shown` rows of its box, the top first.
fn boxed(width: i32, shown: u16, natural: u16, child: impl Into<AnyWidget>) -> AnyWidget {
    let whole = OverflowBox::new(child)
        .min_height(0)
        .max_height(px(natural.max(shown)))
        .alignment(Alignment::TOP_LEFT);
    SizedOverflowBox::new(width, px(shown), whole)
        .alignment(Alignment::TOP_LEFT)
        .into()
}

/// `text` cut to `max` graphemes with an ellipsis (`truncate_text`).
fn cut_graphemes(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    let kept: String = chars[..max.saturating_sub(3)].iter().collect();
    format!("{kept}...")
}
