//! The form of a provider (oracle `app/provider/editor.rs`): a single screen that edits the
//! fields the server reports for a provider, or the three of a new custom gateway, with `Save`
//! under them. Validation keeps what was typed; a save that failed keeps it too.
//!
//! In Flutter: a `StatefulWidget` whose `State` holds a `TextEditingController` and a `FocusNode`
//! per field (and one for Save), the field marked (the fields in turn, then Save: the form's own),
//! and a `ScrollController` for a pane too short for the form (the oracle scrolls the form as one
//! paragraph: the marked row stays in view, Save shows the feedback below it as far as the room
//! goes). What is asked of the server is a verb (`Intent::Save`, the patch `Draft::patch` makes of
//! the changes), what happens to it is the feature's state: while it is written the form shows
//! `Saving model provider…` (the page holds the feature's `Interest::Saving`, so a write that
//! ends while the user is here is shown here), a write that went through pops the form (the list
//! behind it reads again), one that failed is the error line of the form.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, Flexible, MainAxisSize, Row, SizedBox};
use rtwk::navigation::Navigator;
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::{read, select};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{
    AnyWidget, Cx, Handler, Key, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget,
};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::providers::form::{Draft, Field};
use codex_tui_core::features::providers::{Intent, Interest as Wanted, Trouble};
use codex_tui_core::keymap::fixed;

use super::strings;
use crate::app::theme::Tokens;
use crate::features::verb;
use crate::keymap::press::is_escape;
use crate::keymap::pressed_fixed;
use crate::widgets::interest::Interest;
use crate::widgets::key_guard::{GuardKey, KeyGuard};
use crate::widgets::measure;
use crate::widgets::picker::{Event, Picker};

/// The form of a draft: an edit of a provider, or of a new gateway.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub draft: Draft,
}

/// A line of the form: its runs.
type Line = Vec<Span>;

/// Widget state (R14): the fields, their nodes, the field marked, the scroll of a short pane.
pub struct FormState {
    /// The field marked; the number of fields is Save.
    at: usize,
    fields: Vec<TextEditingController>,
    /// The fields' nodes, and Save's after them.
    nodes: Vec<FocusNode>,
    scroll: ScrollController,
    /// The row of the marked line and the rows of the form, as the last layout measured them.
    marked: Rc<Cell<i64>>,
    total: Rc<Cell<i64>>,
    /// A held Enter acts once: it does not go on from a field to Save.
    guard: KeyGuard,
    /// The error line: why the form cannot be saved, or why the write failed.
    error: Option<String>,
    /// The write this form asked for, as the number of writes the feature had before it.
    attempt: Option<u64>,
    /// The feature said the write is under way.
    seen_saving: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// A key moves the mark to this field (or Save).
    Mark(usize),
    /// A node took or lost the focus (a click, a key): the mark follows.
    Focused(usize, bool),
    /// Enter in a field: the mark goes on.
    Submit,
    /// Enter on Save.
    Save,
    /// A tap on Save while it is not marked: the mark goes there.
    TapSave,
    /// Ctrl+Space in the key's field.
    Complete,
    /// Esc.
    Back,
    /// The marked node takes the focus once the frame mounted it.
    Settle,
    /// A new size of the form: the marked row is scrolled into view.
    Reveal,
    /// `Back` of the saving page.
    Left(Event),
}

/// What the form reads of the feature: a write is under way, how the one it asked for failed.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    saving: bool,
    failure: Option<Trouble>,
}

impl StatefulWidget for Form {
    type State = FormState;

    fn create_state(&self) -> FormState {
        let fields: Vec<TextEditingController> = self
            .draft
            .fields()
            .into_iter()
            .map(|(_, original)| TextEditingController::new(&Draft::initial(original)))
            .collect();
        FormState {
            at: 0,
            nodes: (0..=fields.len()).map(|_| FocusNode::new()).collect(),
            fields,
            scroll: ScrollController::default(),
            marked: Rc::new(Cell::new(0)),
            total: Rc::new(Cell::new(0)),
            guard: KeyGuard::default(),
            error: None,
            attempt: None,
            seen_saving: false,
        }
    }
}

impl_widget!(stateful eq Form);

impl FormState {
    /// The index of Save: after the fields.
    fn save_at(&self) -> usize {
        self.fields.len()
    }

    /// What the form watches: asked in `did_change_dependencies` and again in `build`, because a
    /// build forgets the selects of the build before. The failure counts only for the write this
    /// form asked for.
    fn seen(&self, cx: &mut Cx<'_, Self>) -> Seen {
        let since = self.attempt.unwrap_or(u64::MAX);
        select::<FeatureStates, Seen, _>(cx, move |features| {
            let providers = &features.providers;
            Seen {
                saving: providers.saving().is_some(),
                failure: providers.failure_since(since).cloned(),
            }
        })
    }

    /// The write the form asked for ended: the form goes (the list reads again) or says why not.
    fn observe(&mut self, seen: &Seen, cx: &mut Cx<'_, Self>) {
        if self.attempt.is_none() {
            return;
        }
        if seen.saving {
            self.seen_saving = true;
            return;
        }
        if let Some(trouble) = &seen.failure {
            self.attempt = None;
            self.seen_saving = false;
            self.error = Some(strings::trouble_words(trouble));
            Self::after_frame(cx);
        } else if self.seen_saving {
            self.attempt = None;
            self.seen_saving = false;
            Navigator::maybe_pop(cx);
        }
    }

    fn after_frame(cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Settle));
        }
    }

    fn now(cx: &Cx<'_, Self>) -> Duration {
        AppBinding::maybe_of(cx).map_or(Duration::ZERO, |binding| binding.clock().now())
    }

    fn texts(&self) -> Vec<String> {
        self.fields
            .iter()
            .map(|field| field.text().to_string())
            .collect()
    }

    /// The field that holds the API key variable, if the form has one.
    fn variable(w: &Form) -> Option<usize> {
        w.draft
            .fields()
            .iter()
            .position(|(field, _)| *field == Field::Variable)
    }

    /// The form scrolled so that its marked row shows (`visible_layout`: Save shows what is
    /// below it as far as the room goes, a field ends at the bottom); false: nothing to scroll.
    fn reveal(&self) -> bool {
        if !self.scroll.has_clients() {
            return false;
        }
        let view = self.scroll.position().viewport_dimension();
        let (marked, total) = (self.marked.get(), self.total.get());
        let top = if self.at == self.save_at() {
            (total - view).max(0).min(marked)
        } else {
            (marked + 1 - view).max(0)
        };
        self.scroll.jump_to(top);
        true
    }

    /// The keys of each node: the form's moves (`move_provider_field`), Enter on Save, the
    /// completion of the key's name, Esc.
    fn keys(&self, w: &Form, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let count = self.nodes.len();
        let save = self.save_at();
        let variable = Self::variable(w);
        for (index, node) in self.nodes.iter().enumerate() {
            let send = send.clone();
            let guard = self.guard.clone();
            let clock = clock.clone();
            node.set_on_key_event(Some(Rc::new(move |_, key: KeyPress| {
                let now = clock.as_ref().map_or(Duration::ZERO, |clock| clock.now());
                if guard.press(GuardKey::of_press(&key), now) {
                    return KeyEventResult::Handled;
                }
                let is = |id| pressed_fixed(id, &key);
                let msg = if is_escape(&key) {
                    Some(Msg::Back)
                } else if is(fixed::FORM_PREVIOUS) {
                    Some(Msg::Mark((index + count - 1) % count))
                } else if is(fixed::FORM_NEXT) {
                    Some(Msg::Mark((index + 1) % count))
                } else if is(fixed::ONBOARD_CONFIRM) && index == save {
                    Some(Msg::Save)
                } else if is(fixed::GATEWAY_COMPLETE) && Some(index) == variable {
                    Some(Msg::Complete)
                } else {
                    None
                };
                match msg {
                    Some(msg) => {
                        send(msg);
                        KeyEventResult::Handled
                    }
                    None => KeyEventResult::Ignored,
                }
            })));
        }
    }

    /// Asks the feature to write what the form changed, or says why it cannot.
    fn save(&mut self, w: &Form, cx: &mut Cx<'_, Self>) -> Rebuild {
        if self.attempt.is_some() {
            return Rebuild::No;
        }
        let changes = w.draft.changes(&self.texts());
        match w.draft.patch(&changes) {
            Ok(params) => {
                self.error = None;
                self.attempt = Some(read::<FeatureStates, _>(cx).providers.writes());
                self.seen_saving = false;
                verb(cx, Intent::Save(params));
            }
            Err(invalid) => self.error = Some(strings::invalid(invalid).to_owned()),
        }
        Rebuild::Yes
    }
}

impl State for FormState {
    type Widget = Form;
    type Msg = Msg;

    fn init_state(&mut self, w: &Form, cx: &mut Cx<'_, Self>) {
        self.keys(w, cx);
        Self::after_frame(cx);
    }

    fn did_change_dependencies(&mut self, _w: &Form, cx: &mut Cx<'_, Self>) {
        let seen = self.seen(cx);
        self.observe(&seen, cx);
    }

    fn dispose(&mut self) {
        for field in &self.fields {
            field.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &Form, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Submit => {
                // From the last field the mark goes to Save: a held Enter does not go on to
                // save (`KeyEventKind::Press`).
                if self.at < self.save_at() {
                    if self.at + 1 == self.save_at() {
                        self.guard.arm(GuardKey::Enter, Self::now(cx));
                    }
                    self.nodes[self.at + 1].request_focus();
                }
            }
            Msg::Save => return self.save(w, cx),
            Msg::TapSave => {
                if self.at == self.save_at() {
                    return self.save(w, cx);
                }
                self.nodes[self.save_at()].request_focus();
            }
            // The focus moves at once (`FocusNode.requestFocus` in the key handler); its change
            // marks the field.
            Msg::Mark(index) => self.nodes[index].request_focus(),
            Msg::Focused(index, true) => {
                if self.at != index {
                    self.at = index;
                    return Rebuild::Yes;
                }
            }
            Msg::Focused(_, false) => {}
            Msg::Complete => {
                if let Some(variable) = Self::variable(w)
                    && self.at == variable
                {
                    let text = self.fields[variable].text().to_string();
                    let at = self.fields[variable].selection().extent_offset;
                    if at == text.len()
                        && let Some(name) = w.draft.suggestion(&text)
                    {
                        let suffix = name[text.len()..].to_owned();
                        self.fields[variable].insert(&suffix);
                    }
                }
            }
            Msg::Back | Msg::Left(Event::Choose(_) | Event::Cancel) => {
                Navigator::maybe_pop(cx);
            }
            Msg::Left(_) => {}
            Msg::Settle => {
                let node = self.at.min(self.save_at());
                if !self.nodes[node].has_focus() {
                    self.nodes[node].request_focus();
                }
                self.reveal();
            }
            Msg::Reveal => {
                self.reveal();
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Form, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let seen = self.seen(cx);
        if seen.saving {
            // The page waits for the write: it says so, and `Back` leaves it (the write goes on
            // and its end is said in the transcript).
            let saving: AnyWidget =
                AnyWidget::from(Picker::new(Rc::new(strings::saving()), cx.map(Msg::Left)))
                    .with_key(Key::value("saving"));
            return Interest::new(Wanted::Saving, saving).into();
        }
        self.form(w, cx)
    }
}

/// The text of a line, for measuring it.
fn text_of(line: &[Span]) -> String {
    line.iter().map(|span| span.text.as_ref()).collect()
}

/// The rows the lines take at `width`.
fn rows_of<S>(cx: &mut Cx<'_, S>, lines: &[Line], width: usize) -> usize {
    let seg = segmenter_of(cx);
    let width = width.max(1);
    lines
        .iter()
        .map(|line| measure::rows(seg, &text_of(line), width).len().max(1))
        .sum()
}

/// The lines as a column: each wraps where it stands, a blank line is a row.
fn block(lines: &[Line]) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for line in lines {
        column = column.child(if line.is_empty() {
            SizedBox::new(None, Some(1), None).into()
        } else {
            AnyWidget::from(Text::rich(line.clone()))
        });
    }
    column.into()
}

impl FormState {
    /// The form (`ProviderForm::layout`): one scrolling paragraph of the title, the fields, Save,
    /// the error and the hints.
    fn form(&self, w: &Form, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let bold = tokens.bold();
        let cyan = tokens.info();
        let red = tokens.error();
        let dim = Style::new().attrs(Attrs::DIM);
        let selected = self.at;
        let save_at = self.save_at();
        let fields: Vec<(Field, Option<String>)> = w
            .draft
            .fields()
            .into_iter()
            .map(|(field, original)| (field, original.map(str::to_owned)))
            .collect();
        // A field: the text, which a click marks (`Focus.onFocusChange`).
        let inputs: Vec<AnyWidget> = fields
            .iter()
            .enumerate()
            .map(|(index, (field, _))| {
                let input: AnyWidget = TextField::new()
                    .controller(self.fields[index].clone())
                    .focus_node(self.nodes[index].clone())
                    .placeholder(strings::example(*field))
                    .placeholder_style(dim.clone())
                    .style(Style::new())
                    .focused_style(cyan.clone())
                    .selection_style(Style::new())
                    .on_submitted(cx.map(|_| Msg::Submit))
                    .into();
                Focus::new(input)
                    .can_request_focus(false)
                    .skip_traversal(true)
                    .on_focus_change(cx.map(move |on: bool| Msg::Focused(index, on)))
                    .into()
            })
            .collect();
        let tap: Handler<PointerEvent> = cx.map(|_| Msg::TapSave);
        let save: AnyWidget = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .on_tap(tap)
            .child(
                Focus::new(Text::rich([if selected == save_at {
                    Span::styled("› Save", bold.clone())
                } else {
                    Span::new("  Save")
                }]))
                .focus_node(self.nodes[save_at].clone())
                .on_focus_change(cx.map(move |on: bool| Msg::Focused(save_at, on))),
            )
            .into();
        let title = w.draft.provider().map_or_else(
            || strings::CREATE_CUSTOM_PROVIDER.to_owned(),
            |provider| format!("Edit {}", provider.name),
        );
        let top: Vec<Line> = vec![vec![Span::styled(title, bold.clone())], Vec::new()];
        let error: Option<Line> = self
            .error
            .as_ref()
            .map(|error| vec![Span::styled(error.clone(), red)]);
        let has = |kind: Field| fields.iter().any(|(field, _)| *field == kind);
        let mut notes: Vec<Line> = vec![Vec::new()];
        if has(Field::Name) {
            let hint = if has(Field::Variable) {
                "Enter the environment variable name on the app server."
            } else {
                "Authentication is managed on the app server."
            };
            notes.push(vec![Span::styled("* Required", dim.clone())]);
            notes.push(vec![Span::styled(hint, dim.clone())]);
        }
        let keys: Vec<Line> = vec![vec![
            Span::styled("tab", bold.clone()),
            Span::styled(" move · ", dim.clone()),
            Span::styled("enter", bold.clone()),
            Span::styled(" next/save · ", dim.clone()),
            Span::styled("esc", bold.clone()),
            Span::styled(" back", dim.clone()),
        ]];
        let variable = Self::variable(w);
        let controllers = self.fields.clone();
        let draft = w.draft.clone();
        let (marked, total, scroll) =
            (self.marked.clone(), self.total.clone(), self.scroll.clone());
        // A new size settles the scroll too (the room is known only after the layout:
        // `addPostFrameCallback` from the builder).
        let reveal = cx.map(|_: Duration| Msg::Reveal);
        // The marks and the suggestion follow what is typed: the form is a listener of its fields.
        let listened: Rc<dyn Listenable> = <dyn Listenable>::merge(
            self.fields
                .iter()
                .map(|field| Some(field.listenable() as Rc<dyn Listenable>)),
        );
        ListenableBuilder::new(Rc::new(listened), move |_, _| {
            let (fields, inputs, controllers) =
                (fields.clone(), inputs.clone(), controllers.clone());
            let (top, notes, keys, error) =
                (top.clone(), notes.clone(), keys.clone(), error.clone());
            let (save, draft, dim, bold) = (save.clone(), draft.clone(), dim.clone(), bold.clone());
            let (marked, total, scroll) = (marked.clone(), total.clone(), scroll.clone());
            let reveal = reveal.clone();
            LayoutBuilder::new(move |cx, constraints| {
                if let Some(binding) = AppBinding::maybe_of(cx) {
                    binding.add_post_frame_callback(reveal.clone());
                }
                let width = usize::try_from(constraints.max_width).unwrap_or(0);
                let texts: Vec<String> = controllers
                    .iter()
                    .map(|field| field.text().to_string())
                    .collect();
                let changes = draft.changes(&texts);
                let mut content = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch);
                let mut row_at = 0i64;
                let mut mark_at = 0i64;
                content = content.child(block(&top));
                row_at += i64::try_from(rows_of(cx, &top, width)).unwrap_or(0);
                for (index, ((field, _), input)) in fields.iter().zip(&inputs).enumerate() {
                    let label = format!(
                        "{}{}{}  ",
                        if index == selected { "› " } else { "  " },
                        strings::label(*field),
                        if draft.marks_required(*field, &changes) {
                            "*"
                        } else {
                            ""
                        },
                    );
                    // The value goes to a row of its own when the label would crowd the cursor
                    // (`line.width() + 8 > width`).
                    let crowded = segmenter_of(cx).width(&label) + 8 > width;
                    let mut value = Row::new();
                    if Some(index) == variable && width > 1 {
                        value = value.child(Text::rich([Span::styled("$", dim.clone())]));
                    }
                    let value = value.child(Expanded::new(1, input.clone()));
                    if crowded {
                        let words = vec![vec![Span::styled(label, bold.clone())]];
                        let rows = i64::try_from(rows_of(cx, &words, width)).unwrap_or(1);
                        if index == selected {
                            mark_at = row_at + rows;
                        }
                        row_at += rows + 1;
                        content = content.child(
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .child(block(&words))
                                .child(value),
                        );
                    } else {
                        if index == selected {
                            mark_at = row_at;
                        }
                        row_at += 1;
                        content = content.child(
                            Row::new()
                                .child(
                                    Text::rich([Span::styled(label, bold.clone())])
                                        .wrap(TextWrap::None),
                                )
                                .child(Expanded::new(1, value)),
                        );
                    }
                }
                if selected == save_at {
                    mark_at = row_at;
                }
                row_at += 1;
                content = content.child(save.clone());
                if let Some(error) = &error {
                    let lines = vec![error.clone()];
                    row_at += i64::try_from(rows_of(cx, &lines, width)).unwrap_or(1);
                    content = content.child(block(&lines));
                }
                row_at += i64::try_from(rows_of(cx, &notes, width)).unwrap_or(0);
                content = content.child(block(&notes));
                // The completion of the key's name: only while the caret ends the text.
                if let Some(variable) = variable
                    && selected == variable
                {
                    let text = texts[variable].clone();
                    let at_end = controllers[variable].selection().extent_offset == text.len();
                    if let Some(name) = at_end.then(|| draft.suggestion(&text)).flatten() {
                        let line = vec![
                            Span::styled("ctrl + space", bold.clone()),
                            Span::styled(format!(" complete {name}"), dim.clone()),
                        ];
                        row_at += i64::try_from(rows_of(cx, std::slice::from_ref(&line), width))
                            .unwrap_or(1);
                        content = content.child(block(&[line]));
                    }
                }
                row_at += i64::try_from(rows_of(cx, &keys, width)).unwrap_or(0);
                content = content.child(block(&keys));
                marked.set(mark_at);
                total.set(row_at);
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Flexible::new(
                        1,
                        SingleChildScrollView::new(content).controller(scroll.clone()),
                    ))
                    .into()
            })
            .into()
        })
        .into()
    }
}
