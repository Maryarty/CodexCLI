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

use rtwk::binding::AppBinding;
use rtwk::editing::{EditingKeys, TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    Flexible, MainAxisAlignment, MainAxisSize, Padding, Row, UNBOUNDED,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::input::{self, Draft};
use codex_tui_core::screens::approvals::{Intent, Prompt, Question};

use super::strings::{self, Strings};
use super::tips;
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::options::{OptionRow, OptionStyles, Options};

/// The dialog of the `request_user_input` request `key` (`prompt` holds its questions).
#[derive(Clone, Debug, PartialEq)]
pub struct UserInputView {
    pub key: RequestKey,
    pub prompt: Rc<Prompt>,
}

impl UserInputView {
    fn questions(&self) -> &[Question] {
        match &*self.prompt {
            Prompt::UserInput(input) => &input.questions,
            _ => &[],
        }
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
    notes: TextEditingController,
    node: FocusNode,
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Esc: closes the notes, else interrupts; in the confirmation, goes back.
    Cancel,
    /// Ctrl+C: clears typed notes, else interrupts.
    CtrlC,
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
        let questions: Vec<QuestionState> = self
            .questions()
            .iter()
            .map(|question| {
                let options = input::has_options(question);
                QuestionState {
                    options: Options::new(),
                    selected: options,
                    committed: false,
                    notes_open: !options,
                    notes: TextEditingController::new(""),
                    node: FocusNode::new(),
                }
            })
            .collect();
        let notes_focus = self
            .questions()
            .first()
            .is_some_and(|question| !input::has_options(question));
        UserInputViewState {
            questions,
            current: 0,
            notes_focus,
            confirming: false,
            confirm: Options::new(),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq UserInputView);

/// The keys of the dialog, whatever the question: what the oracle's `handle_key_event` reads
/// (a key a mode does not take is ignored by `update`).
fn msg_of(key: &KeyPress) -> Option<Msg> {
    let ctrl = key.mods.contains(Mods::CTRL);
    let letter = key.text().and_then(|text| {
        let mut chars = text.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first)
    });
    let plain = key.mods == Mods::NONE;
    match (key.code, letter) {
        (KeyCode::Escape, _) => Some(Msg::Cancel),
        (KeyCode::Char, Some('c')) if ctrl => Some(Msg::CtrlC),
        (KeyCode::Char, Some('p')) if ctrl => Some(Msg::Question(false)),
        (KeyCode::Char, Some('n')) if ctrl => Some(Msg::Question(true)),
        (KeyCode::PageUp, _) if plain => Some(Msg::Question(false)),
        (KeyCode::PageDown, _) if plain => Some(Msg::Question(true)),
        // the list keymap's defaults (`keymap.rs` `list`): the runtime keymap is the app's, and
        // wires through when it reaches the port
        (KeyCode::Left, _) if plain => Some(Msg::Side(false)),
        (KeyCode::Char, Some('h')) if plain || ctrl => Some(Msg::Side(false)),
        (KeyCode::Right, _) if plain => Some(Msg::Side(true)),
        (KeyCode::Char, Some('l')) if plain || ctrl => Some(Msg::Side(true)),
        (KeyCode::Up, _) if plain => Some(Msg::Cursor(-1)),
        (KeyCode::Char, Some('k')) if plain || ctrl => Some(Msg::Cursor(-1)),
        (KeyCode::Down, _) if plain => Some(Msg::Cursor(1)),
        (KeyCode::Char, Some('j')) if plain || ctrl => Some(Msg::Cursor(1)),
        // `^H` (0x08) is Backspace with Ctrl to rtwk's parser, ctrl+h to the oracle's crossterm:
        // the list keymap's MoveLeft
        (KeyCode::Backspace, _) if key.mods == Mods::CTRL => Some(Msg::Side(false)),
        (KeyCode::Backspace | KeyCode::Delete, _) if plain => Some(Msg::Clear),
        (KeyCode::Char, Some(' ')) if plain => Some(Msg::Pick),
        (KeyCode::Tab, _) if plain => Some(Msg::Tab),
        (KeyCode::Enter, _) if plain => Some(Msg::Accept),
        (KeyCode::Char, Some(digit)) if plain => digit
            .to_digit(10)
            .filter(|digit| *digit > 0)
            .and_then(|digit| usize::try_from(digit).ok())
            .map(Msg::Digit),
        _ => None,
    }
}

/// The keys the dialog reads on the notes before the text area does (the oracle reads them
/// before the composer): cancel, the move between questions, and on an option question the
/// tab, the arrows (they move the options) and Backspace on empty notes.
fn notes_msg(key: &KeyPress, options: bool, empty: bool) -> Option<Msg> {
    let msg = msg_of(key)?;
    let ctrl_pn =
        key.mods.contains(Mods::CTRL) && key.text().is_some_and(|text| text == "p" || text == "n");
    match msg {
        Msg::Cancel | Msg::CtrlC => Some(msg),
        Msg::Question(_) if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) || ctrl_pn => {
            Some(msg)
        }
        Msg::Tab | Msg::Cursor(_) if options && !matches!(key.code, KeyCode::Char) => Some(msg),
        Msg::Clear if options && empty && key.code == KeyCode::Backspace => Some(Msg::HideNotes),
        _ => None,
    }
}

impl UserInputViewState {
    fn drafts(&self) -> Vec<Draft> {
        self.questions
            .iter()
            .map(|state| Draft {
                selected: state.selected.then(|| state.options.cursor()),
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

    fn init_state(&mut self, _: &UserInputView, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key: KeyPress| match msg_of(&key) {
                Some(msg) => {
                    send(msg);
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.confirm.dispose();
        for state in &self.questions {
            state.options.dispose();
            state.notes.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &UserInputView, cx: &mut Cx<'_, Self>) -> Rebuild {
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
            Msg::Cancel => {
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
            let area = TextArea::new()
                .controller(state.notes.clone())
                .placeholder(placeholder)
                .placeholder_style(tokens.dim())
                .min_lines(1)
                .max_lines(6)
                .focus_node(state.node.clone())
                .autofocus(self.notes_focus)
                .on_submitted(cx.map(Msg::Submitted))
                .on_changed(cx.map(|_: Rc<str>| Msg::Edited));
            // The text area takes Backspace, the arrows and Ctrl+C for its own text; the dialog
            // reads them first (the oracle's `handle_key_event` reads them before the composer),
            // as the door rtwk gives an app for keys of the fields below it.
            let send = cx.local_sender();
            let area = EditingKeys::new(
                move |notes, key, _| {
                    let empty = notes.text().trim().is_empty();
                    match notes_msg(key, has_options, empty) {
                        Some(msg) => {
                            send(msg);
                            true
                        }
                        None => false,
                    }
                },
                area,
            )
            .controller(state.notes.clone());
            let composer: AnyWidget = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::new(String::new()))
                .child(
                    Row::new()
                        .child(Padding::new(
                            EdgeInsets::only(0, 0, 1, 0),
                            Text::new(s.composer_prompt).style(tokens.dim().attrs(Attrs::BOLD)),
                        ))
                        .child(Expanded::new(1, area)),
                )
                .child(Text::new(String::new()))
                .into();
            column = column.child(composer.with_key(u32::try_from(self.current).unwrap_or(0)));
        }
        let footer = self.footer(tokens, s, question, visible);
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
        question: &Question,
        notes_visible: bool,
    ) -> AnyWidget {
        let dim = tokens.dim();
        let key = |keys: &str, label: &str| -> Vec<Span> {
            vec![
                Span::styled(keys.to_string(), tokens.bold().attrs(Attrs::DIM)),
                Span::styled(format!(" {label}"), dim.clone()),
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
                Span::styled(format!(" {}", s.tip_clear_notes), dim.clone()),
            ]);
        }
        let submit = if count > 1 && self.current + 1 >= count {
            s.tip_submit_all
        } else {
            s.tip_submit_answer
        };
        list.push(key(s.key_enter, submit));
        if count > 1 {
            if has_options && !self.notes_focus {
                list.push(key(s.tip_arrows, s.tip_navigate_questions));
            } else if !has_options {
                list.push(key(s.tip_change_field_keys, s.tip_change_question));
            }
        }
        // Esc closes the notes of an option question, so it is not offered to interrupt then.
        if !(has_options && notes_visible) {
            list.push(key(s.key_esc, s.tip_interrupt));
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
