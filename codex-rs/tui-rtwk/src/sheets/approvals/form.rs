//! The form of an MCP server elicitation (S13, family B): the oracle's
//! `McpServerElicitationOverlay`. One field is shown at a time: a progress
//! row, the prompt (the message, the tool parameters, the field's label and
//! description), the input (option rows for a select, a boolean or the
//! approval fallback; a text area for a string) and the footer tips. The
//! model is core's `Form` (`screens::approvals::form`): what the schema asks
//! and the answer it makes; what the user has entered so far is this widget's
//! state (the Flutter counterpart: a `Form` of `FormField`s whose
//! `TextEditingController`s and `Options` cursors the state holds), and the
//! answer returns to the app as one `Respond` intent.

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::TextEditingController;
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Flexible,
    MainAxisAlignment, MainAxisSize, Padding, UNBOUNDED,
};
use rtwk::overlay::Dialog;
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::RequestKey;
use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{COMPOSER_SUBMIT, LIST_ACCEPT};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::form::{Action, Field, FieldInput, Form};
use codex_tui_core::screens::approvals::{ElicitationPrompt, Entered, Intent, Prompt};
use codex_tui_core::screens::composer::{MAX_INPUT_CHARS, too_large};

use super::keys::{Press, press_of};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::pages::session::composer::{Composer, plain};
use crate::widgets::options::{OptionRow, OptionStyles, Options};
use crate::widgets::thread_ui::ThreadUi;

/// The form dialog of the elicitation `key` asks (`prompt` holds a form).
#[derive(Clone, Debug, PartialEq)]
pub struct FormView {
    pub key: RequestKey,
    pub prompt: Rc<Prompt>,
}

impl FormView {
    fn parts(&self) -> (&ElicitationPrompt, &Form) {
        match &*self.prompt {
            Prompt::Elicitation(elicitation) => match &elicitation.form {
                Some(form) => (elicitation, form),
                None => unreachable!("a form view is made for a prompt that holds a form"),
            },
            _ => unreachable!("a form view is made for an elicitation prompt"),
        }
    }
}

/// What one field holds while the form is open: the cursor of its options,
/// its text, and whether the user committed it.
struct FieldState {
    options: Options,
    /// The answer's controllers: the plain-text composer's (`plain::ui`);
    /// `text` is its text.
    ui: Rc<ThreadUi>,
    text: TextEditingController,
    node: FocusNode,
    committed: bool,
}

impl FieldState {
    /// A field's own state: the cursor of its options, its text and its focus node.
    fn new(field: &Field) -> FieldState {
        let options = Options::new();
        options.set(field.input.start());
        let ui = plain::ui();
        FieldState {
            options,
            text: ui.composer.clone(),
            ui,
            node: FocusNode::new(),
            committed: field.input.preselected(),
        }
    }
}

/// Why the last submit did not go (the footer's accent line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fail {
    Required,
    TooLong(usize),
}

pub struct FormViewState {
    fields: Vec<FieldState>,
    current: usize,
    fail: Option<Fail>,
    node: FocusNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Esc: the request is cancelled.
    Cancel,
    /// Ctrl+C: clears a typed answer, else cancels.
    CtrlC,
    /// The previous or the next field.
    Field(bool),
    /// The option cursor a row on (-1 up, 1 down).
    Cursor(i64),
    /// Backspace or Delete on a list: the answer is not committed.
    Uncommit,
    /// Space: commits the option the cursor is on.
    Pick,
    /// Enter or a tap on the cursor's row: commits it and goes on.
    Accept,
    /// A digit: that option, committed, and on.
    Digit(usize),
    /// Enter in the text area.
    Submitted(Rc<str>),
    /// The text changed under the user's hands.
    Edited,
    /// The frame that built a text field is done: it takes the keys.
    FocusText,
}

impl StatefulWidget for FormView {
    type State = FormViewState;

    fn create_state(&self) -> FormViewState {
        let (_, form) = self.parts();
        let fields = form.fields.iter().map(FieldState::new).collect();
        FormViewState {
            fields,
            current: 0,
            fail: None,
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq FormView);

/// The form-level keys, whichever field is current: what the oracle's
/// `handle_key_event` does before and in the select branch. A key a list
/// does not take is ignored by `update` on a text field.
fn msg_of(keymap: &Keymap, key: &KeyPress) -> Option<Msg> {
    match press_of(keymap, key)? {
        Press::Cancel => Some(Msg::Cancel),
        Press::Interrupt => Some(Msg::CtrlC),
        Press::Question(next) | Press::Side(next) => Some(Msg::Field(next)),
        Press::Cursor(delta) => Some(Msg::Cursor(delta)),
        Press::Clear => Some(Msg::Uncommit),
        Press::Pick => Some(Msg::Pick),
        Press::Accept => Some(Msg::Accept),
        Press::Digit(digit) => Some(Msg::Digit(digit)),
        Press::Tab => None,
    }
}

/// The keys the form reads on a text field before the text area does: cancel, and the move
/// between fields (the other keys of `msg_of` belong to the list fields).
fn text_field_msg(keymap: &Keymap, key: &KeyPress) -> Option<Msg> {
    match press_of(keymap, key)? {
        Press::Cancel => Some(Msg::Cancel),
        Press::Interrupt => Some(Msg::CtrlC),
        Press::Question(next) => Some(Msg::Field(next)),
        _ => None,
    }
}

impl FormViewState {
    /// What each field holds as committed (the oracle's `field_value`).
    fn entered(&self, form: &Form) -> Vec<Option<Entered>> {
        self.fields
            .iter()
            .zip(&form.fields)
            .map(|(state, field)| {
                state.committed.then(|| match field.input {
                    FieldInput::Text => Entered::Text(state.text.text().to_string()),
                    _ => Entered::Choice(state.options.cursor()),
                })
            })
            .collect()
    }

    fn is_text(&self, form: &Form) -> bool {
        matches!(form.fields[self.current].input, FieldInput::Text)
    }

    fn respond(
        &self,
        w: &FormView,
        answer: codex_tui_core::screens::approvals::RequestAnswer,
        cx: &mut Cx<'_, Self>,
    ) {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Approvals(Intent::Respond {
                key: w.key.clone(),
                answer,
            }),
        );
    }

    /// Goes to field `index`; a list takes the keys itself, a text area
    /// takes the focus as it is built.
    fn go_to(&mut self, form: &Form, index: usize) {
        self.current = index;
        if !matches!(form.fields[index].input, FieldInput::Text) {
            self.node.request_focus();
        }
    }

    fn move_field(&mut self, form: &Form, next: bool) {
        let len = form.fields.len();
        let offset = if next { 1 } else { len - 1 };
        self.fail = None;
        self.go_to(form, (self.current + offset) % len);
    }

    fn go_next_or_submit(&mut self, w: &FormView, form: &Form, cx: &mut Cx<'_, Self>) {
        if self.current + 1 < form.fields.len() {
            self.move_field(form, true);
            return;
        }
        match form.answer(&self.entered(form)) {
            Ok(answer) => {
                self.fail = None;
                self.respond(w, answer, cx);
            }
            Err(index) => {
                self.fail = Some(Fail::Required);
                self.go_to(form, index);
            }
        }
    }
}

impl State for FormViewState {
    type Widget = FormView;
    type Msg = Msg;

    fn init_state(&mut self, _: &FormView, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let keymap = read::<Keymap, _>(cx);
        // The text area takes PageUp, PageDown, ctrl+p and ctrl+n for its rows;
        // the form changes field with them (the oracle's `handle_key_event`
        // reads them before the composer), so each field's node meets them first.
        for state in &self.fields {
            let send = send.clone();
            let keymap = keymap.clone();
            state
                .node
                .set_on_key_event(Some(Rc::new(move |_, key: KeyPress| {
                    match text_field_msg(&keymap, &key) {
                        Some(msg) => send(msg),
                        None => return KeyEventResult::Ignored,
                    }
                    KeyEventResult::Handled
                })));
        }
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key: KeyPress| {
                match msg_of(&keymap, &key) {
                    Some(msg) => {
                        send(msg);
                        KeyEventResult::Handled
                    }
                    None => KeyEventResult::Ignored,
                }
            })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        for state in &self.fields {
            state.node.set_on_key_event(None);
            state.options.dispose();
            state.text.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &FormView, cx: &mut Cx<'_, Self>) -> Rebuild {
        let (_, form) = w.parts();
        let text = self.is_text(form);
        let choices = form.fields[self.current].input.choices();
        match msg {
            Msg::Cancel => {
                self.respond(w, form.cancel(), cx);
                return Rebuild::No;
            }
            Msg::CtrlC => {
                let state = &mut self.fields[self.current];
                if text && !state.text.text().is_empty() {
                    state.text.clear();
                    state.committed = false;
                    self.fail = None;
                } else {
                    self.respond(w, form.cancel(), cx);
                    return Rebuild::No;
                }
            }
            Msg::Field(next) => self.move_field(form, next),
            Msg::Cursor(delta) if !text => {
                let state = &mut self.fields[self.current];
                state.options.step(delta, choices);
                state.committed = false;
                self.fail = None;
            }
            Msg::Uncommit if !text => {
                self.fields[self.current].committed = false;
                self.fail = None;
            }
            Msg::Pick if !text => {
                self.fields[self.current].committed = true;
                self.fail = None;
            }
            Msg::Accept if !text => {
                self.fields[self.current].committed = true;
                self.fail = None;
                self.go_next_or_submit(w, form, cx);
            }
            Msg::Digit(digit) if !text && digit <= choices => {
                let state = &mut self.fields[self.current];
                state.options.set(digit - 1);
                state.committed = true;
                self.fail = None;
                self.go_next_or_submit(w, form, cx);
            }
            Msg::Submitted(answer) if text => {
                if let Some(typed) = too_large(&answer) {
                    self.fail = Some(Fail::TooLong(typed));
                } else {
                    self.fields[self.current].committed = !answer.trim().is_empty();
                    self.fail = None;
                    self.go_next_or_submit(w, form, cx);
                }
            }
            Msg::Edited if text => {
                self.fields[self.current].committed = false;
                self.fail = None;
            }
            Msg::FocusText => {
                if self.is_text(form) {
                    self.fields[self.current].node.request_focus();
                }
                return Rebuild::No;
            }
            _ => return Rebuild::No,
        }
        if self.is_text(form) {
            // A text area that mounts takes the focus (autofocus) only where its scope holds
            // none, so a field the user comes to from a list is asked once the frame that built
            // it is done (Flutter: `addPostFrameCallback(requestFocus)`).
            if let Some(binding) = AppBinding::maybe_of(cx) {
                binding.add_post_frame_callback(cx.map(|_: Duration| Msg::FocusText));
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &FormView, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (elicitation, form) = w.parts();
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let entered = self.entered(form);
        let field = &form.fields[self.current];
        let state = &self.fields[self.current];
        let answered = form
            .value(self.current, entered[self.current].as_ref())
            .is_some();
        let unanswered = form.unanswered(&entered);

        let mut progress = vec![Span::styled(
            s.field_progress(self.current + 1, form.fields.len())
                .to_string(),
            tokens.dim(),
        )];
        if unanswered > 0 {
            progress.push(Span::styled(
                s.required_unanswered(unanswered).to_string(),
                tokens.dim(),
            ));
        }
        let prompt = Text::new(prompt_text(s, form, &elicitation.message, self.current))
            .wrap(TextWrap::Word)
            .style(if answered {
                Style::new()
            } else {
                tokens.accent()
            });

        let input: AnyWidget = match &field.input {
            FieldInput::Text => {
                let placeholder = if field.required {
                    s.answer_placeholder
                } else {
                    s.optional_answer_placeholder
                };
                // The oracle's composer (`ChatComposerConfig::plain_text`): a blank
                // row, the `›` prompt and the answer, a blank row.
                let composer = Composer::plain(state.ui.clone(), state.node.clone())
                    .placeholder(placeholder)
                    .prompt_style(tokens.dim().attrs(Attrs::BOLD))
                    .on_submitted(cx.map(Msg::Submitted))
                    .on_changed(cx.map(|_: Rc<str>| Msg::Edited));
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Text::new(String::new()))
                    .child(composer)
                    .child(Text::new(String::new()))
                    .into()
            }
            _ => {
                let styles = OptionStyles {
                    cursor: tokens.accent().attrs(Attrs::BOLD),
                    label: Style::new(),
                    description: Style::new(),
                };
                // A tap moves the cursor, a tap on the cursor's row accepts it.
                let rows = state.options.on_accept(cx.map(|_: usize| Msg::Accept));
                rows.view(Rc::new(option_rows(s, form, &field.input)), styles, 0)
            }
        }
        .with_key(u32::try_from(self.current).unwrap_or(0));

        let footer = footer(
            s,
            &tokens,
            &crate::keymap::keymap(cx),
            form,
            self.current,
            text_input(&field.input),
            self.fail,
        );

        // The oracle clips a dialog taller than its room: so does this one (the validation line
        // can take the row the footer needs).
        let inner = Column::new()
            .overflow_intended(true)
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::rich(progress))
            .child(prompt)
            .child(input);
        // The oracle's overlay is at least eight rows with its padding; the
        // footer stays at its foot.
        let content = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Flexible::new(1, inner))
            .child(footer);
        let surface = tokens.prompt_surface(cx, false);
        let focus = Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(
            Padding::new(
                EdgeInsets::symmetric(2, 1),
                ConstrainedBox::new(BoxConstraints::new(0, UNBOUNDED, 6, UNBOUNDED), content),
            ),
        )))
        .focus_node(self.node.clone());
        // The keys of the form bubble up from a text area (it holds the focus
        // of a text field), so only a list takes the focus at the form.
        if form.fields.first().is_some_and(|f| text_input(&f.input)) {
            focus.into()
        } else {
            focus.autofocus().into()
        }
    }
}

fn text_input(input: &FieldInput) -> bool {
    matches!(input, FieldInput::Text)
}

/// The text above the input of field `index`: the request's message, the
/// parameter lines of a tool approval, then the field's label and
/// description (`current_prompt_text`, `format_tool_approval_display_message`).
fn prompt_text(s: &Strings, form: &Form, message: &str, index: usize) -> String {
    let message = message.trim();
    let mut request = message.to_string();
    if !form.summary.is_empty() {
        let mut sections = Vec::new();
        if !message.is_empty() {
            sections.push(message.to_string());
        }
        sections.push(
            form.summary
                .iter()
                .map(|param| s.param_line(&param.name, &param.value).to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        request = sections.join("\n\n");
        request.push('\n');
    }
    let Some(field) = form.fields.get(index) else {
        return request;
    };
    let mut sections = Vec::new();
    if !request.trim().is_empty() {
        sections.push(request);
    }
    let prompt = if field.label.trim().is_empty()
        || field.prompt.trim().is_empty()
        || field.label == field.prompt
    {
        if field.prompt.trim().is_empty() {
            field.label.clone()
        } else {
            field.prompt.clone()
        }
    } else {
        format!("{}\n{}", field.label, field.prompt)
    };
    if !prompt.trim().is_empty() {
        sections.push(prompt);
    }
    sections.join("\n\n")
}

/// The rows of a list field: the number, the label, the description column.
fn option_rows(s: &Strings, form: &Form, input: &FieldInput) -> Vec<OptionRow> {
    let tool = form.tool_approval;
    let rows: Vec<(String, Option<String>)> = match input {
        FieldInput::Select { options, .. } => options
            .iter()
            .map(|option| (option.label.clone(), option.description.clone()))
            .collect(),
        FieldInput::Boolean { .. } => vec![
            (s.bool_true.to_string(), None),
            (s.bool_false.to_string(), None),
        ],
        FieldInput::Actions(actions) => actions
            .iter()
            .map(|action| {
                let (label, tool_text, request_text) = match action {
                    Action::Allow => (s.act_allow, s.allow_tool, s.allow_request),
                    Action::AllowForSession => {
                        (s.act_allow_session, s.session_tool, s.session_request)
                    }
                    Action::AlwaysAllow => (s.act_always_allow, s.always_tool, s.always_request),
                    Action::Deny => (s.act_deny, s.deny_request, s.deny_request),
                    Action::Cancel => (s.act_cancel, s.cancel_tool, s.cancel_request),
                };
                (
                    label.to_string(),
                    Some((if tool { tool_text } else { request_text }).to_string()),
                )
            })
            .collect(),
        FieldInput::Text => Vec::new(),
    };
    rows.into_iter()
        .enumerate()
        .map(|(index, (label, description))| OptionRow {
            label: format!("{}. {label}", index + 1),
            description,
        })
        .collect()
}

/// The tips under the input, dim, apart by ` | `: the failure of the last
/// submit first, then what Enter does, the keys between fields, and Esc
/// (`footer_tips`).
fn footer(
    s: &Strings,
    tokens: &Tokens,
    keymap: &Keymap,
    form: &Form,
    current: usize,
    text: bool,
    fail: Option<Fail>,
) -> AnyWidget {
    let dim = tokens.dim();
    let key = |keys: &str, label: &str, spans: &mut Vec<Span>| {
        spans.push(Span::styled(
            keys.to_string(),
            tokens.bold().attrs(Attrs::DIM),
        ));
        spans.push(Span::styled(s.key_gap, dim.clone()));
        spans.push(Span::styled(label.to_string(), dim.clone()));
    };
    let mut spans: Vec<Span> = Vec::new();
    let separate = |spans: &mut Vec<Span>| {
        if !spans.is_empty() {
            spans.push(Span::styled(s.tip_separator.to_string(), dim.clone()));
        }
    };
    if let Some(fail) = fail {
        let line = match fail {
            Fail::Required => s.answer_required.to_string(),
            Fail::TooLong(typed) => s.too_long(MAX_INPUT_CHARS, typed).to_string(),
        };
        spans.push(Span::styled(line, tokens.accent().attrs(Attrs::BOLD)));
    }
    let count = form.fields.len();
    let submit = if count == 1 {
        s.tip_submit
    } else if current + 1 >= count {
        s.tip_submit_all
    } else {
        s.tip_submit_answer
    };
    // A text field submits by the composer's key, a list by the list's accept; a key the user
    // unbound has no tip.
    let submit_key = crate::keymap::label(keymap, if text { COMPOSER_SUBMIT } else { LIST_ACCEPT });
    if let Some(submit_key) = submit_key {
        separate(&mut spans);
        key(&submit_key, submit, &mut spans);
    }
    if count > 1 {
        separate(&mut spans);
        if text {
            key(s.tip_change_field_keys, s.tip_change_field, &mut spans);
        } else {
            key(s.tip_arrows, s.tip_navigate, &mut spans);
        }
    }
    separate(&mut spans);
    key(s.key_esc, s.tip_cancel, &mut spans);
    Text::rich(spans).wrap(TextWrap::Word).into()
}
