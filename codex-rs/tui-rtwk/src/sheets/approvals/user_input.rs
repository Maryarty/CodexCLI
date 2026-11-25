//! The questions of a `request_user_input` request (S13, family C): the oracle's
//! `RequestUserInputOverlay`. One question is shown at a time: a progress row, the question,
//! its options (a list with a cursor), the notes (a text area: the notes of an option question,
//! the answer of a freeform one) and the footer tips. What the answer holds and when a question
//! counts as answered is core's `input` (`screens::approvals::input`); what the user has done so
//! far is this widget's state (the Flutter counterpart: a `Form` of `FormField`s whose
//! `TextEditingController`s and `Options` cursors the state holds), and the answer returns to
//! the app as one `Respond` intent. A submit with unanswered questions asks first, in the same
//! dialog (a mode of the state, not a second dialog).

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::editing::{EditingKeys, TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    Flexible, MainAxisAlignment, MainAxisSize, Padding, Row, UNBOUNDED,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::RequestKey;
use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{CHAT_INTERRUPT_TURN, COMPOSER_SUBMIT, LIST_ACCEPT};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::input::{self, AutoResolution, Draft};
use codex_tui_core::screens::approvals::{Intent, Prompt, Question};

use super::keys::{Press, is_backspace, is_escape, press_of, types_text};
use super::strings::{self, Strings};
use super::tips;
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::pages::session::composer::{Composer, plain};
use crate::widgets::options::{OptionRow, OptionStyles, Options};
use crate::widgets::thread_ui::ThreadUi;

/// The dialog of the `request_user_input` request `key` (`prompt` holds its questions).
#[derive(Clone, Debug, PartialEq)]
pub struct UserInputView {
    pub key: RequestKey,
    pub prompt: Rc<Prompt>,
    /// The dialog opens on the confirmation of a submit with unanswered questions.
    pub confirming: bool,
}

impl UserInputView {
    fn questions(&self) -> &[Question] {
        match &*self.prompt {
            Prompt::UserInput(input) => &input.questions,
            _ => &[],
        }
    }

    /// The request is not blocking: it answers itself when nobody does.
    fn can_auto_resolve(&self) -> bool {
        matches!(&*self.prompt, Prompt::UserInput(input) if input.can_auto_resolve)
    }
}

/// What one question holds while the dialog is open: the cursor of its options, whether the
/// cursor's row is selected (Backspace clears the selection), whether the answer is committed,
/// whether the notes are open, and the notes.
struct QuestionState {
    options: Options,
    selected: bool,
    committed: bool,
    notes_open: bool,
    /// The notes' controllers: the plain-text composer's (`plain::ui`); `notes` is its text.
    ui: Rc<ThreadUi>,
    notes: TextEditingController,
    node: FocusNode,
}

impl QuestionState {
    /// A question's own state: its options' cursor, its notes and their focus node.
    fn new(question: &Question) -> QuestionState {
        let options = input::has_options(question);
        let ui = plain::ui();
        QuestionState {
            options: Options::new(),
            selected: options,
            committed: false,
            notes_open: !options,
            notes: ui.composer.clone(),
            ui,
            node: FocusNode::new(),
        }
    }
}

pub struct UserInputViewState {
    questions: Vec<QuestionState>,
    current: usize,
    /// The keys go to the notes (the oracle's `Focus::Notes`).
    notes_focus: bool,
    /// The confirmation of a submit with unanswered questions is open.
    confirming: bool,
    confirm: Options,
    node: FocusNode,
    /// The countdown of a request that answers itself (Flutter: a `Timer.periodic` the state
    /// owns): the app clock when the dialog opened, the seconds since, and whether a key or a
    /// paste stopped it.
    clock: Option<Duration>,
    elapsed: Duration,
    snoozed: bool,
    timer: Option<Timer>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// A second of the countdown passed.
    Tick,
    /// A key or a paste: the request no longer answers itself.
    Snooze,
    /// Esc: closes the notes, else interrupts; in the confirmation, goes back.
    Cancel,
    /// Ctrl+C: clears typed notes, else interrupts.
    CtrlC,
    /// The chat's interrupt key when the user bound it to something else than Esc: it always
    /// interrupts, with the notes open too (the oracle: only Esc closes them); in the
    /// confirmation it does nothing.
    Stop,
    /// The previous or the next question (ctrl+p/n, PageUp/PageDown).
    Question(bool),
    /// The previous or the next question from the options (h/l, the arrows).
    Side(bool),
    /// The option cursor a row on (-1 up, 1 down).
    Cursor(i64),
    /// Backspace or Delete on the options: the selection is cleared.
    Clear,
    /// Space: commits the option the cursor is on.
    Pick,
    /// Tab: opens the notes, or clears them and goes back to the options.
    Tab,
    /// Backspace on empty notes: closes them.
    HideNotes,
    /// Enter, or a tap on the cursor's row.
    Accept,
    /// A digit: that option, committed, and on.
    Digit(usize),
    /// A tap moved the cursor to a row.
    Moved(usize),
    /// Enter in the notes.
    Submitted(Rc<str>),
    /// The notes changed under the user's hands.
    Edited,
    /// The frame that built the notes is done: they take the keys.
    FocusNotes,
}

impl StatefulWidget for UserInputView {
    type State = UserInputViewState;

    fn create_state(&self) -> UserInputViewState {
        let questions: Vec<QuestionState> =
            self.questions().iter().map(QuestionState::new).collect();
        let notes_focus = self
            .questions()
            .first()
            .is_some_and(|question| !input::has_options(question));
        UserInputViewState {
            questions,
            current: 0,
            notes_focus: notes_focus && !self.confirming,
            confirming: self.confirming,
            confirm: Options::new(),
            node: FocusNode::new(),
            clock: None,
            elapsed: Duration::ZERO,
            snoozed: false,
            timer: None,
        }
    }
}

impl_widget!(stateful eq UserInputView);

/// The keys of the dialog, whatever the question: what the oracle's `handle_key_event` reads
/// (a key a mode does not take is ignored by `update`).
fn msg_of(keymap: &Keymap, key: &KeyPress) -> Option<Msg> {
    Some(match press_of(keymap, key)? {
        Press::Cancel if is_escape(key) => Msg::Cancel,
        Press::Cancel => Msg::Stop,
        Press::Interrupt => Msg::CtrlC,
        Press::Question(next) => Msg::Question(next),
        Press::Side(next) => Msg::Side(next),
        Press::Cursor(delta) => Msg::Cursor(delta),
        Press::Clear => Msg::Clear,
        Press::Pick => Msg::Pick,
        Press::Tab => Msg::Tab,
        Press::Accept => Msg::Accept,
        Press::Digit(digit) => Msg::Digit(digit),
    })
}

/// The keys the dialog reads on the notes before the text area does (the oracle reads them
/// before the composer): cancel, the move between questions, and on an option question the
/// tab, the arrows (they move the options) and Backspace on empty notes.
fn notes_msg(keymap: &Keymap, key: &KeyPress, options: bool, empty: bool) -> Option<Msg> {
    let msg = msg_of(keymap, key)?;
    // The composer's submit key is read before the moves between questions (the oracle's
    // `handle_key_event`): it is the notes' own, whatever else it is bound to.
    if matches!(msg, Msg::Question(_) | Msg::Tab | Msg::Cursor(_))
        && crate::keymap::pressed(keymap, COMPOSER_SUBMIT, key)
    {
        return None;
    }
    match msg {
        Msg::Cancel | Msg::Stop | Msg::CtrlC | Msg::Question(_) => Some(msg),
        Msg::Tab | Msg::Cursor(_) if options && !types_text(key) => Some(msg),
        Msg::Clear if options && empty && is_backspace(key) => Some(Msg::HideNotes),
        _ => None,
    }
}

impl UserInputViewState {
    fn drafts(&self) -> Vec<Draft> {
        self.questions
            .iter()
            .map(|state| Draft {
                choice: state.selected.then(|| state.options.cursor()),
                committed: state.committed,
                notes: state.notes.text().to_string(),
            })
            .collect()
    }

    /// The notes show for a freeform question, an opened one and one that holds text.
    fn notes_visible(&self, index: usize, question: &Question) -> bool {
        !input::has_options(question)
            || self
                .questions
                .get(index)
                .is_some_and(|state| state.notes_open || !state.notes.text().trim().is_empty())
    }

    fn respond(&self, w: &UserInputView, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        let answer = input::answer(w.questions(), &self.drafts());
        cx.emit(
            &hs.screen,
            ScreenIntent::Approvals(Intent::Respond {
                key: w.key.clone(),
                answer,
            }),
        );
    }

    /// Where the countdown of a request that answers itself stands.
    fn auto_resolution(&self, w: &UserInputView) -> AutoResolution {
        input::auto_resolution(w.can_auto_resolve(), self.snoozed, self.elapsed)
    }

    /// A key, a paste or typed text: the request waits for the user from now on.
    fn stop_countdown(&mut self) {
        self.snoozed = true;
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }

    /// A second of the countdown: the clock decides how much time passed (a frame that came late
    /// counts what it missed); a request that ran out is answered empty.
    fn tick(&mut self, w: &UserInputView, cx: &mut Cx<'_, Self>) -> Rebuild {
        self.elapsed = match (
            self.clock,
            AppBinding::maybe_of(cx).and_then(|b| b.ticker()),
        ) {
            (Some(start), Some(ticker)) => ticker.now().saturating_sub(start),
            _ => self.elapsed + Duration::from_secs(1),
        };
        match self.auto_resolution(w) {
            AutoResolution::Due => {
                self.confirming = false;
                self.stop_countdown();
                let hs = read::<Hs, _>(cx);
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Approvals(Intent::Respond {
                        key: w.key.clone(),
                        answer: input::empty_answer(),
                    }),
                );
                Rebuild::No
            }
            AutoResolution::VisibleCountdown { .. } => Rebuild::Yes,
            AutoResolution::HiddenGrace { .. } | AutoResolution::Disabled => Rebuild::No,
        }
    }

    /// The oracle's interrupt: the request is dismissed and the dialog closes.
    fn interrupt(&self, w: &UserInputView, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.dismiss, w.key.clone());
        close_dialog(cx, Some(()));
    }

    /// The keys follow the question the user came to: a freeform one takes them in its notes,
    /// an option question keeps them in the notes only while they show.
    fn ensure_focus(&mut self, w: &UserInputView) {
        let Some(question) = w.questions().get(self.current) else {
            return;
        };
        if !input::has_options(question) {
            self.notes_focus = true;
        } else if self.notes_focus && !self.notes_visible(self.current, question) {
            self.notes_focus = false;
        }
        if !self.notes_focus {
            self.node.request_focus();
        }
    }

    fn jump_to(&mut self, w: &UserInputView, index: usize) {
        if index < w.questions().len() {
            self.current = index;
            self.ensure_focus(w);
        }
    }

    fn move_question(&mut self, w: &UserInputView, next: bool) {
        let len = w.questions().len();
        if len == 0 {
            return;
        }
        let offset = if next { 1 } else { len - 1 };
        self.jump_to(w, (self.current + offset) % len);
    }

    /// The notes and the answer of the current question are dropped and the keys go to the
    /// options (`clear_notes_and_focus_options`).
    fn close_notes(&mut self) {
        let state = &mut self.questions[self.current];
        state.notes.clear();
        state.committed = false;
        state.notes_open = false;
        self.notes_focus = false;
        self.node.request_focus();
    }

    fn open_notes(&mut self) {
        let state = &mut self.questions[self.current];
        state.notes_open = true;
        self.notes_focus = true;
        state.node.request_focus();
    }

    fn go_next_or_submit(&mut self, w: &UserInputView, cx: &mut Cx<'_, Self>) {
        if self.current + 1 < w.questions().len() {
            self.move_question(w, true);
        } else if input::unanswered(w.questions(), &self.drafts()) > 0 {
            self.confirming = true;
            self.confirm.set(0);
            self.node.request_focus();
        } else {
            self.respond(w, cx);
        }
    }

    /// Back from the confirmation to the first question that is not answered.
    fn go_back(&mut self, w: &UserInputView) {
        self.confirming = false;
        if let Some(index) = input::first_unanswered(w.questions(), &self.drafts()) {
            self.jump_to(w, index);
        } else {
            self.ensure_focus(w);
        }
    }

    fn update_confirm(&mut self, msg: Msg, w: &UserInputView, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Cancel | Msg::Clear => self.go_back(w),
            Msg::Stop => return Rebuild::No,
            Msg::CtrlC => {
                self.interrupt(w, cx);
                return Rebuild::No;
            }
            Msg::Cursor(delta) => self.confirm.step(delta, 2),
            Msg::Digit(digit @ (1 | 2)) => self.confirm.set(digit - 1),
            Msg::Accept => {
                if self.confirm.cursor() == 0 {
                    self.confirming = false;
                    self.respond(w, cx);
                } else {
                    self.go_back(w);
                }
            }
            _ => return Rebuild::No,
        }
        Rebuild::Yes
    }
}

impl State for UserInputViewState {
    type Widget = UserInputView;
    type Msg = Msg;

    fn init_state(&mut self, w: &UserInputView, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let keymap = read::<Keymap, _>(cx);
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key: KeyPress| {
                // Any key of the dialog ends the countdown, whether the dialog takes it or not.
                send(Msg::Snooze);
                match msg_of(&keymap, &key) {
                    Some(msg) => {
                        send(msg);
                        KeyEventResult::Handled
                    }
                    None => KeyEventResult::Ignored,
                }
            })));
        if w.can_auto_resolve()
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            self.clock = binding.ticker().map(|ticker| ticker.now());
            self.timer =
                Some(binding.start_periodic(Duration::from_secs(1), cx.map(|(): ()| Msg::Tick)));
        }
    }

    fn dispose(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        self.node.set_on_key_event(None);
        self.confirm.dispose();
        for state in &self.questions {
            state.options.dispose();
            state.notes.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &UserInputView, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Snooze => {
                self.stop_countdown();
                return Rebuild::Yes;
            }
            Msg::Tick => return self.tick(w, cx),
            Msg::Edited => self.stop_countdown(),
            _ => {}
        }
        let questions = w.questions();
        let Some(question) = questions.get(self.current) else {
            return Rebuild::No;
        };
        if self.confirming {
            return self.update_confirm(msg, w, cx);
        }
        let has_options = input::has_options(question);
        let choices = input::choices(question);
        let in_options = has_options && !self.notes_focus;
        let visible = self.notes_visible(self.current, question);
        match msg {
            Msg::Cancel if has_options && visible => self.close_notes(),
            Msg::Cancel | Msg::Stop => {
                self.interrupt(w, cx);
                return Rebuild::No;
            }
            Msg::CtrlC => {
                let state = &mut self.questions[self.current];
                if self.notes_focus && !state.notes.text().is_empty() {
                    state.notes.clear();
                    state.committed = false;
                    state.notes_open = true;
                } else {
                    self.interrupt(w, cx);
                    return Rebuild::No;
                }
            }
            Msg::Question(next) => self.move_question(w, next),
            Msg::Side(next) if in_options => self.move_question(w, next),
            Msg::Cursor(delta) if has_options => {
                let state = &mut self.questions[self.current];
                if state.selected {
                    state.options.step(delta, choices);
                } else {
                    // No row is selected: either arrow selects the first.
                    state.selected = true;
                    state.options.set(0);
                }
                state.committed = false;
            }
            Msg::Clear if in_options => {
                let state = &mut self.questions[self.current];
                state.selected = false;
                state.options.clear();
                state.committed = false;
                state.notes_open = false;
                state.notes.clear();
            }
            Msg::Pick if in_options => {
                let state = &mut self.questions[self.current];
                state.committed = state.selected;
            }
            Msg::Tab if has_options => {
                if self.notes_focus {
                    self.close_notes();
                } else if self.questions[self.current].selected {
                    self.open_notes();
                }
            }
            Msg::HideNotes if has_options => {
                self.questions[self.current].notes_open = false;
                self.notes_focus = false;
                self.node.request_focus();
            }
            Msg::Accept if in_options => {
                let state = &self.questions[self.current];
                // Enter on the other row asks for the details first.
                if state.selected
                    && input::other_enabled(question)
                    && state.options.cursor() == question.options.len()
                {
                    self.open_notes();
                } else {
                    if state.selected {
                        self.questions[self.current].committed = true;
                    }
                    self.go_next_or_submit(w, cx);
                }
            }
            Msg::Digit(digit) if in_options && digit <= choices => {
                let state = &mut self.questions[self.current];
                state.options.set(digit - 1);
                state.selected = true;
                state.committed = true;
                self.go_next_or_submit(w, cx);
            }
            Msg::Moved(row) if has_options => {
                let state = &mut self.questions[self.current];
                state.options.set(row);
                state.selected = true;
                state.committed = false;
            }
            Msg::Submitted(text) if self.notes_focus || !has_options => {
                if text.trim().is_empty() {
                    // The composer submits nothing empty: Enter does not advance.
                    return Rebuild::No;
                }
                let state = &mut self.questions[self.current];
                state.committed = has_options || !text.trim().is_empty();
                self.go_next_or_submit(w, cx);
            }
            Msg::Edited => {
                self.questions[self.current].committed = false;
            }
            Msg::FocusNotes => {
                if self.notes_focus {
                    self.questions[self.current].node.request_focus();
                }
                return Rebuild::No;
            }
            _ => return Rebuild::No,
        }
        if self.notes_focus {
            // The notes are built by the rebuild this update asks for: a text area that mounts
            // takes the focus (autofocus) only where its scope holds none, so the node is
            // asked once that frame is done (Flutter: `addPostFrameCallback(requestFocus)`).
            if let Some(binding) = AppBinding::maybe_of(cx) {
                binding.add_post_frame_callback(cx.map(|_: Duration| Msg::FocusNotes));
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &UserInputView, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let questions = w.questions();
        let body: AnyWidget = if self.confirming {
            self.confirmation(&tokens, s, questions, cx)
        } else {
            self.question(w, &tokens, s, cx)
        };
        let surface = tokens.prompt_surface(cx, false);
        let focus = Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(
            Padding::new(
                EdgeInsets::symmetric(2, 1),
                ConstrainedBox::new(BoxConstraints::new(0, UNBOUNDED, 6, UNBOUNDED), body),
            ),
        )))
        .focus_node(self.node.clone());
        // The keys bubble up from a text area (it holds the focus of a freeform question), so
        // only a list takes the focus at the dialog.
        if self.notes_focus {
            focus.into()
        } else {
            focus.autofocus().into()
        }
    }
}

impl UserInputViewState {
    /// The question the user is on: progress, text, options, notes and tips.
    fn question(
        &self,
        w: &UserInputView,
        tokens: &Tokens,
        s: &'static Strings,
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let questions = w.questions();
        let Some(question) = questions.get(self.current) else {
            return Text::new(String::new()).into();
        };
        let state = &self.questions[self.current];
        let drafts = self.drafts();
        let has_options = input::has_options(question);
        let visible = self.notes_visible(self.current, question);
        let unanswered = input::unanswered(questions, &drafts);
        let answered = input::answered(question, &drafts[self.current]);

        let mut progress = vec![Span::styled(
            s.question_progress(self.current + 1, questions.len())
                .to_string(),
            tokens.dim(),
        )];
        if unanswered > 0 {
            progress.push(Span::styled(
                s.question_unanswered(unanswered).to_string(),
                tokens.dim(),
            ));
        }
        if let AutoResolution::VisibleCountdown { remaining } = self.auto_resolution(w) {
            progress.push(Span::styled(s.countdown_separator, tokens.dim()));
            progress.push(Span::styled(
                s.auto_resolves_in(&input::remaining_text(remaining))
                    .to_string(),
                tokens.error(),
            ));
        }
        let text = Text::new(question.text.clone())
            .wrap(TextWrap::Word)
            .style(if answered {
                Style::new()
            } else {
                tokens.accent()
            });

        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::rich(progress))
            .child(text);
        if has_options {
            let styles = OptionStyles {
                cursor: tokens.accent().attrs(Attrs::BOLD),
                label: Style::new(),
                description: Style::new(),
            };
            // A tap moves the cursor, a tap on the cursor's row accepts it.
            let rows = state
                .options
                .on_accept(cx.map(|_: usize| Msg::Accept))
                .on_move(cx.map(Msg::Moved));
            column = column.child(Text::new(String::new())).child(rows.view(
                Rc::new(option_rows(s, question)),
                styles,
                0,
            ));
            if !visible {
                column = column.child(Text::new(String::new()));
            }
        }
        if visible {
            // The oracle's composer: a blank row, the `›` prompt and the notes, a blank row.
            let placeholder = if !has_options {
                s.optional_answer_placeholder
            } else if state.selected {
                s.notes_placeholder
            } else {
                s.select_option_placeholder
            };
            // The text area takes Backspace, the arrows and Ctrl+C for its own text; the dialog
            // reads them first (the oracle's `handle_key_event` reads them before the composer).
            let send = cx.local_sender();
            let keymap = crate::keymap::keymap(cx);
            let take = move |notes: &TextEditingController, key: &KeyPress| {
                let empty = notes.text().trim().is_empty();
                match notes_msg(&keymap, key, has_options, empty) {
                    Some(msg) => {
                        send(msg);
                        true
                    }
                    None => false,
                }
            };
            let input: AnyWidget = if question.is_secret {
                // The oracle masks a secret answer with `*` (`render_with_mask`): Flutter's
                // `TextField(obscureText: true)`, one line, behind the dialog's keys (the door
                // rtwk gives an app for keys of the fields below it).
                let field = TextField::new()
                    .controller(state.notes.clone())
                    .placeholder(placeholder)
                    .placeholder_style(tokens.dim())
                    .obscure_text(true)
                    .obscuring_character("*")
                    .focus_node(state.node.clone())
                    .autofocus(self.notes_focus)
                    .on_submitted(cx.map(Msg::Submitted))
                    .on_changed(cx.map(|_: Rc<str>| Msg::Edited));
                let field = EditingKeys::new(move |notes, key, _| take(notes, key), field)
                    .controller(state.notes.clone());
                Row::new()
                    .child(Padding::new(
                        EdgeInsets::only(0, 0, 1, 0),
                        Text::new(s.composer_prompt).style(tokens.dim().attrs(Attrs::BOLD)),
                    ))
                    .child(Expanded::new(1, field))
                    .into()
            } else {
                // `ChatComposerConfig::plain_text`: `!`, `/` and `?` stay text, no popups.
                Composer::plain(state.ui.clone(), state.node.clone())
                    .placeholder(placeholder)
                    .prompt_style(tokens.dim().attrs(Attrs::BOLD))
                    .autofocus(self.notes_focus)
                    .on_keys(move |notes, key, _| take(notes, key))
                    .on_submitted(cx.map(Msg::Submitted))
                    .on_changed(cx.map(|_: Rc<str>| Msg::Edited))
                    .into()
            };
            let composer: AnyWidget = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::new(String::new()))
                .child(input)
                .child(Text::new(String::new()))
                .into();
            column = column.child(composer.with_key(u32::try_from(self.current).unwrap_or(0)));
        }
        let footer = self.footer(tokens, s, &crate::keymap::keymap(cx), question, visible);
        if !visible {
            // Without the input the spare rows of a tall frame stay under the footer.
            return column.child(footer).into();
        }
        // The oracle's input takes the rows of its area that the others leave: the footer stays
        // at the foot (a frame as tall as the overlay needs leaves none).
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Flexible::new(1, column))
            .child(footer)
            .into()
    }

    /// The tips under the question (`footer_tips`).
    fn footer(
        &self,
        tokens: &Tokens,
        s: &Strings,
        keymap: &Keymap,
        question: &Question,
        notes_visible: bool,
    ) -> AnyWidget {
        let dim = tokens.dim();
        let key = |keys: &str, label: &str| -> Vec<Span> {
            vec![
                Span::styled(keys.to_string(), tokens.bold().attrs(Attrs::DIM)),
                Span::styled(s.key_gap, dim.clone()),
                Span::styled(label.to_string(), dim.clone()),
            ]
        };
        let has_options = input::has_options(question);
        let selected = has_options && self.questions[self.current].selected;
        let count = self.questions.len();
        let mut list: Vec<Vec<Span>> = Vec::new();
        if selected && !notes_visible {
            list.push(key(s.key_tab, s.tip_add_notes));
        }
        if selected && notes_visible {
            list.push(vec![
                Span::styled(s.key_tab.to_string(), tokens.bold().attrs(Attrs::DIM)),
                Span::styled(s.hint_or.to_string(), dim.clone()),
                Span::styled(s.key_esc.to_string(), tokens.bold().attrs(Attrs::DIM)),
                Span::styled(s.key_gap, dim.clone()),
                Span::styled(s.tip_clear_notes.to_string(), dim.clone()),
            ]);
        }
        let submit = if count > 1 && self.current + 1 >= count {
            s.tip_submit_all
        } else {
            s.tip_submit_answer
        };
        // The submit key is the composer's while the keys go to the notes, else the list's
        // accept; the interrupt key is the chat's. A key the user unbound has no tip.
        let submit_key = if self.notes_focus || !has_options {
            crate::keymap::label(keymap, COMPOSER_SUBMIT)
        } else {
            crate::keymap::label(keymap, LIST_ACCEPT)
        };
        if let Some(submit_key) = submit_key {
            list.push(key(&submit_key, submit));
        }
        if count > 1 {
            if has_options && !self.notes_focus {
                list.push(key(s.tip_arrows, s.tip_navigate_questions));
            } else if !has_options {
                list.push(key(s.tip_change_field_keys, s.tip_change_question));
            }
        }
        // Esc closes the notes of an option question, so it is not offered to interrupt then.
        if let Some(interrupt) = crate::keymap::label(keymap, CHAT_INTERRUPT_TURN)
            && !(has_options && notes_visible && interrupt == s.key_esc)
        {
            list.push(key(&interrupt, s.tip_interrupt));
        }
        tips::rows(list, s.tip_separator, dim)
    }

    /// The question that asks whether to submit with unanswered questions.
    fn confirmation(
        &self,
        tokens: &Tokens,
        s: &'static Strings,
        questions: &[Question],
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let unanswered = input::unanswered(questions, &self.drafts());
        let subtitle = if unanswered == 1 {
            s.confirm_subtitle_one.to_string()
        } else {
            s.confirm_subtitle(unanswered).to_string()
        };
        let proceed = if unanswered == 1 {
            s.confirm_proceed_one.to_string()
        } else {
            s.confirm_proceed_many(unanswered).to_string()
        };
        let rows = vec![
            OptionRow {
                label: format!("1. {}", s.confirm_proceed),
                description: Some(proceed),
            },
            OptionRow {
                label: format!("2. {}", s.confirm_go_back),
                description: Some(s.confirm_go_back_desc.to_string()),
            },
        ];
        let styles = OptionStyles {
            cursor: tokens.accent().attrs(Attrs::BOLD),
            label: Style::new(),
            description: Style::new(),
        };
        let list =
            self.confirm
                .on_accept(cx.map(|_: usize| Msg::Accept))
                .view(Rc::new(rows), styles, 0);
        let hint = Text::rich(vec![
            Span::styled(s.hint_press.to_string(), tokens.dim()),
            Span::styled(s.key_enter.to_string(), tokens.bold()),
            Span::styled(s.hint_confirm.to_string(), tokens.dim()),
            Span::styled(s.key_esc.to_string(), tokens.bold()),
            Span::styled(s.hint_go_back.to_string(), tokens.dim()),
        ]);
        let inner = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::new(s.confirm_title).style(tokens.bold()))
            .child(Text::new(subtitle).style(tokens.dim()))
            .child(Text::new(String::new()))
            .child(list)
            .child(Text::new(String::new()));
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Flexible::new(1, inner))
            .child(hint)
            .into()
    }
}

/// The rows of the options: the number, the label, the description column; the other row
/// follows the options of a question that offers it.
fn option_rows(s: &Strings, question: &Question) -> Vec<OptionRow> {
    let row = |index: usize, label: &str, description: &str| OptionRow {
        label: format!("{}. {label}", index + 1),
        description: (!description.is_empty()).then(|| description.to_string()),
    };
    let mut rows: Vec<OptionRow> = question
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| row(index, &option.label, &option.description))
        .collect();
    if input::other_enabled(question) {
        rows.push(row(
            question.options.len(),
            s.none_of_the_above,
            s.none_description,
        ));
    }
    rows
}
