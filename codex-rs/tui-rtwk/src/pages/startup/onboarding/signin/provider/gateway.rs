//! The custom provider or gateway wizard (`onboarding/gateway.rs`, the
//! settings picker and `app/provider/editor.rs`, the form): the saved
//! gateways to use as the default, one row that creates a gateway, and the
//! form that does it (name, base URL, the variable of the API key, Save).
//!
//! In Flutter: a `StatefulWidget` over the core model whose `State` holds a
//! `TextEditingController` and a `FocusNode` per field (and one for Save and
//! one for the list), the list's cursor (the shared `Options` controller)
//! and a `ScrollController` for a pane too short for the form (the oracle
//! scrolls the form as one paragraph: the marked row stays in view, Save
//! shows the feedback below it as far as the room goes). The keys are the
//! fields' own and the list's node's; Esc bubbles up to the step (one stage
//! back); the form's marks are `Focus.onFocusChange` of its nodes, so a click
//! marks like a key.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::AnyWidget;
use rtwk::binding::AppBinding;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::foundation::{ListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Expanded, Flexible, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::read;
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::{self, provider};
use provider::gateway::{FIELDS, Fields, Form, FormError, Gateway, Stage};

use super::super::{Line, Look, Resting, guarded_keys, now, rows_of, width_of};
use super::{frame, frame_of, sized, strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::pressed_fixed;
use crate::widgets::key_guard::{GuardKey, KeyGuard};
use crate::widgets::options::{OptionRow, OptionStyles, Options, RichRow};

/// The wizard over its model.
#[derive(Clone, PartialEq, Eq)]
pub struct GatewayWizard {
    pub gateway: Gateway,
    pub guard: KeyGuard,
    /// Set while the form's mark is on Save (the logo of the step fades only while a field has
    /// the keys: the step reads it).
    pub resting: Resting,
}

/// The nodes: the three fields, Save, and the list's (which the views
/// without a field keep the focus on, so that Esc and Ctrl+C reach the step).
const SAVE: usize = FIELDS;
const LIST: usize = FIELDS + 1;
const VARIABLE: usize = FIELDS - 1;

/// Widget state (R14): the fields, their nodes, the field marked (the fields in turn, then
/// Save: the form's own, ADR-013), the list's cursor, the scroll of a short pane.
pub struct GatewayState {
    at: usize,
    fields: [TextEditingController; FIELDS],
    nodes: [FocusNode; FIELDS + 2],
    options: Options,
    scroll: ScrollController,
    /// The row of the marked line and the rows of the form, as the last
    /// layout measured them.
    marked: Rc<Cell<i64>>,
    total: Rc<Cell<i64>>,
    /// The step's `Resting`, set to false when the wizard goes.
    rested: Rc<ValueNotifier<bool>>,
}

pub enum Msg {
    Up,
    Down,
    Top,
    Bottom,
    /// PageUp, PageDown: the rows of a page, with the sign.
    Page(i64),
    /// Enter on the list, or the digit of its one numbered row.
    Confirm,
    /// A tap on the row of the list the cursor was on, or a double tap.
    Accept(usize),
    /// A tap moved the cursor: the hint under the list follows it.
    Moved,
    /// Enter in a field.
    Submit,
    /// Enter on Save, or a tap on it while it is marked.
    Save,
    /// A key moves the mark to this field (or Save).
    Mark(usize),
    /// A node took or lost the focus (a click, a key): the mark follows.
    Focused(usize, bool),
    /// A tap on Save while it is not marked: the mark goes there.
    TapSave,
    /// Ctrl+Space in the variable's field.
    Complete,
    Quit,
    /// The view's node takes the focus once the frame mounted it, and the
    /// marked row is scrolled into view.
    Settle,
    /// A new size of the form: the marked row is scrolled into view (the
    /// focus is the model's to follow, not a layout's).
    Reveal,
}

impl StatefulWidget for GatewayWizard {
    type State = GatewayState;

    fn create_state(&self) -> GatewayState {
        GatewayState {
            at: 0,
            fields: std::array::from_fn(|_| TextEditingController::new("")),
            nodes: std::array::from_fn(|_| FocusNode::new()),
            options: Options::new(),
            scroll: ScrollController::default(),
            marked: Rc::new(Cell::new(0)),
            total: Rc::new(Cell::new(0)),
            rested: self.resting.0.clone(),
        }
    }
}

impl_widget!(stateful eq GatewayWizard);

/// The form of the stages that have one (a save keeps its fields).
fn form_of(stage: &Stage) -> Option<&Form> {
    match stage {
        Stage::Editing { form, .. } | Stage::Saving { form, .. } => Some(form),
        _ => None,
    }
}

/// The rows of the list a stage shows (the saved gateways and the one that
/// creates, or the error's "Back"); none for the other stages.
fn list_len(stage: &Stage) -> usize {
    match stage {
        Stage::Picking(picker) => picker.list.create_row() + 1,
        Stage::LoadError(_) => 1,
        _ => 0,
    }
}

impl GatewayState {
    /// The form scrolled so that its marked row shows; false: there is no
    /// form to scroll (`visible_layout`: Save shows what is below it as far
    /// as the room goes, a field ends at the bottom).
    fn reveal(&self, w: &GatewayWizard) -> bool {
        if !matches!(w.gateway.stage, Stage::Editing { .. }) {
            return false;
        }
        if !self.scroll.has_clients() {
            return false;
        }
        let view = self.scroll.position().viewport_dimension();
        let (marked, total) = (self.marked.get(), self.total.get());
        let top = if self.at == SAVE {
            (total - view).max(0).min(marked)
        } else {
            (marked + 1 - view).max(0)
        };
        self.scroll.jump_to(top);
        true
    }

    /// The step's logo is told whether the mark rests on Save of the form.
    fn rest(&self, w: &GatewayWizard) {
        let on_save = matches!(w.gateway.stage, Stage::Editing { .. }) && self.at == SAVE;
        w.resting.0.set_value(on_save);
    }

    fn after_frame(cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Settle));
        }
    }

    fn emit(cx: &mut Cx<'_, Self>, intent: auth::Intent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::SignIn(intent)));
    }

    fn emit_gateway(cx: &mut Cx<'_, Self>, intent: provider::gateway::Intent) {
        Self::emit(
            cx,
            auth::Intent::Provider(provider::Intent::Gateway(intent)),
        );
    }

    fn texts(&self) -> Fields {
        Fields {
            name: self.fields[0].text().to_string(),
            url: self.fields[1].text().to_string(),
            variable: self.fields[VARIABLE].text().to_string(),
        }
    }

    /// The row a list's Enter, tap or digit takes: a saved gateway is used,
    /// the last row creates one, "Back" leaves.
    fn take(cx: &mut Cx<'_, Self>, w: &GatewayWizard, row: usize, guard: Option<GuardKey>) {
        // The key that took the row does not act again on what it opens.
        if let Some(key) = guard {
            w.guard.arm(key, now(cx));
        }
        match &w.gateway.stage {
            // Past the saved gateways: the row that creates one.
            Stage::Picking(picker) if row < picker.list.create_row() => {
                Self::emit_gateway(cx, provider::gateway::Intent::Use(row));
            }
            Stage::Picking(_) => Self::emit_gateway(cx, provider::gateway::Intent::Create),
            Stage::LoadError(_) => Self::emit(cx, auth::Intent::ProviderBack),
            _ => {}
        }
    }

    /// The list's keys and the form's moves (`move_provider_field`), the
    /// rest goes on to the field or up to the step.
    fn keys(&self, w: &GatewayWizard, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let time: Rc<dyn Fn() -> Duration> =
            Rc::new(move || clock.as_ref().map_or(Duration::ZERO, |clock| clock.now()));
        let quit = {
            let send = send.clone();
            Rc::new(move || send(Msg::Quit))
        };
        let count = FIELDS + 1;
        for (index, node) in self.nodes.iter().enumerate() {
            // Save and the list have no text: a `q` quits there.
            let typing = index < FIELDS;
            let guarded = guarded_keys(
                w.guard.clone(),
                time.clone(),
                Rc::new(move || typing),
                quit.clone(),
            );
            let send = send.clone();
            node.set_on_key_event(Some(Rc::new(move |node, key: KeyPress| {
                if guarded(node, key.clone()) == KeyEventResult::Handled {
                    return KeyEventResult::Handled;
                }
                let is = |id| pressed_fixed(id, &key);
                let msg = if index == LIST {
                    if is(fixed::GATEWAY_PREVIOUS) {
                        Some(Msg::Up)
                    } else if is(fixed::GATEWAY_NEXT) {
                        Some(Msg::Down)
                    } else if is(fixed::GATEWAY_TOP) {
                        Some(Msg::Top)
                    } else if is(fixed::GATEWAY_BOTTOM) {
                        Some(Msg::Bottom)
                    } else if is(fixed::GATEWAY_PAGE_UP) {
                        Some(Msg::Page(-1))
                    } else if is(fixed::GATEWAY_PAGE_DOWN) {
                        Some(Msg::Page(1))
                    } else if is(fixed::GATEWAY_CHOOSE) {
                        Some(Msg::Confirm)
                    } else {
                        None
                    }
                } else if is(fixed::FORM_PREVIOUS) {
                    Some(Msg::Mark((index + count - 1) % count))
                } else if is(fixed::FORM_NEXT) {
                    Some(Msg::Mark((index + 1) % count))
                } else if is(fixed::ENTER) && index == SAVE {
                    Some(Msg::Save)
                } else if is(fixed::GATEWAY_COMPLETE) && index == VARIABLE {
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
}

impl State for GatewayState {
    type Widget = GatewayWizard;
    type Msg = Msg;

    fn init_state(&mut self, w: &GatewayWizard, cx: &mut Cx<'_, Self>) {
        self.keys(w, cx);
        Self::after_frame(cx);
    }

    fn did_update_widget(&mut self, old: &GatewayWizard, w: &GatewayWizard, cx: &mut Cx<'_, Self>) {
        if old.gateway.stage == w.gateway.stage {
            return;
        }
        // A new form starts empty, on its first field; a save that failed keeps what was typed
        // and where the mark was.
        if form_of(&old.gateway.stage).is_none() && form_of(&w.gateway.stage).is_some() {
            for field in &self.fields {
                field.set_text("");
            }
            self.at = 0;
        }
        self.rest(w);
        // A list opens on the row it names (the gateway just created, the
        // one the form came from), the error's on "Back".
        match (&old.gateway.stage, &w.gateway.stage) {
            (Stage::Picking(a), Stage::Picking(b)) if a.lands_on == b.lands_on => {}
            (_, Stage::Picking(picker)) => self.options.set(picker.lands_on.unwrap_or(0)),
            (Stage::LoadError(_), Stage::LoadError(_)) => {}
            (_, Stage::LoadError(_)) => self.options.set(0),
            _ => {}
        }
        // Holding Enter on Save must not carry onto what a save opens.
        if matches!(w.gateway.stage, Stage::Saving { .. })
            && !matches!(old.gateway.stage, Stage::Saving { .. })
        {
            w.guard.arm(GuardKey::Enter, now(cx));
        }
        Self::after_frame(cx);
    }

    fn dispose(&mut self) {
        self.rested.set_value(false);
        for field in &self.fields {
            field.dispose();
        }
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &GatewayWizard, cx: &mut Cx<'_, Self>) -> Rebuild {
        let len = list_len(&w.gateway.stage);
        match msg {
            Msg::Up | Msg::Down if len > 0 => {
                self.options
                    .step(if matches!(msg, Msg::Up) { -1 } else { 1 }, len);
                return Rebuild::Yes;
            }
            Msg::Top if len > 0 => {
                self.options.set(0);
                return Rebuild::Yes;
            }
            Msg::Bottom if len > 0 => {
                self.options.set(len - 1);
                return Rebuild::Yes;
            }
            Msg::Page(sign) if len > 0 => {
                let page = if self.options.scroll().has_clients() {
                    self.options.scroll().position().viewport_dimension().max(1)
                } else {
                    1
                };
                let at = i64::try_from(self.options.cursor()).unwrap_or(0) + sign * page;
                let last = i64::try_from(len - 1).unwrap_or(0);
                self.options
                    .set(usize::try_from(at.clamp(0, last)).unwrap_or(0));
                return Rebuild::Yes;
            }
            Msg::Up | Msg::Down | Msg::Top | Msg::Bottom | Msg::Page(_) => {}
            Msg::Confirm if len > 0 => {
                Self::take(cx, w, self.options.cursor(), Some(GuardKey::Enter));
            }
            Msg::Confirm => {}
            Msg::Accept(row) => Self::take(cx, w, row, None),
            Msg::Moved => return Rebuild::Yes,
            Msg::Submit => {
                // From the last field the mark goes to Save: a held Enter
                // does not go on to save (`KeyEventKind::Press`).
                if matches!(w.gateway.stage, Stage::Editing { .. }) && self.at < SAVE {
                    if self.at + 1 == SAVE {
                        w.guard.arm(GuardKey::Enter, now(cx));
                    }
                    self.nodes[self.at + 1].request_focus();
                }
            }
            Msg::Save => Self::emit_gateway(cx, provider::gateway::Intent::Save(self.texts())),
            Msg::TapSave => match form_of(&w.gateway.stage) {
                Some(_) if self.at == SAVE => {
                    Self::emit_gateway(cx, provider::gateway::Intent::Save(self.texts()));
                }
                Some(_) => self.nodes[SAVE].request_focus(),
                None => {}
            },
            // The focus moves at once (`FocusNode.requestFocus` in the key
            // handler); its change marks the field in the model.
            Msg::Mark(index) => {
                if matches!(w.gateway.stage, Stage::Editing { .. }) {
                    self.nodes[index].request_focus();
                }
            }
            Msg::Focused(index, true) => {
                if matches!(w.gateway.stage, Stage::Editing { .. }) && self.at != index {
                    self.at = index;
                    self.rest(w);
                    return Rebuild::Yes;
                }
            }
            Msg::Focused(_, false) => {}
            Msg::Complete => {
                if let Stage::Editing { form, .. } = &w.gateway.stage
                    && self.at == VARIABLE
                {
                    let text = self.fields[VARIABLE].text();
                    let at = self.fields[VARIABLE].selection().extent_offset;
                    if at == text.len()
                        && let Some(name) = form.suggestion(&text)
                    {
                        self.fields[VARIABLE].insert(&name[text.len()..]);
                    }
                }
            }
            Msg::Settle => {
                let node = match &w.gateway.stage {
                    Stage::Editing { .. } => self.at.min(SAVE),
                    _ => LIST,
                };
                if !self.nodes[node].has_focus() {
                    self.nodes[node].request_focus();
                }
                if !self.reveal(w) && len > 0 {
                    // The cursor on a row the last layout did not have yet.
                    self.options.set(self.options.cursor());
                }
            }
            Msg::Reveal => {
                self.reveal(w);
            }
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &GatewayWizard, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let tokens = Tokens::of(cx);
        let look = Look {
            dim: Style::new().attrs(Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        };
        let idle: AnyWidget = Focus::new(SizedBox::shrink())
            .focus_node(self.nodes[LIST].clone())
            .into();
        let bold = tokens.bold();
        match &w.gateway.stage {
            Stage::Loading { .. } | Stage::Saving { .. } | Stage::Selecting { .. } => {
                let text = if matches!(w.gateway.stage, Stage::Loading { .. }) {
                    strings.gateway_loading
                } else {
                    strings.gateway_saving
                };
                let waiting: Vec<Line> = vec![vec![Span::styled(text, look.dim.clone())]];
                LayoutBuilder::new(move |cx, constraints| {
                    let width = width_of(constraints.max_width);
                    let body = Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(sized(cx, waiting.clone(), width))
                        .child(idle.clone());
                    frame(cx, body.into(), Vec::new(), width, constraints.max_height)
                })
                .into()
            }
            Stage::LoadError(message) => {
                let message = message.clone();
                let options = self
                    .options
                    .on_accept(cx.map(Msg::Accept))
                    .on_move(cx.map(|_: usize| Msg::Moved));
                let rows = Rc::new(vec![OptionRow {
                    label: strings.gateway_back_row.to_owned(),
                    description: None,
                }]);
                let styles = OptionStyles {
                    cursor: tokens.selected(),
                    label: Style::new(),
                    description: look.dim.clone(),
                };
                LayoutBuilder::new(move |cx, constraints| {
                    let width = width_of(constraints.max_width);
                    let head = vec![vec![Span::styled(strings.gateway_title, bold.clone())]];
                    let subtitle = vec![vec![Span::styled(message.clone(), look.dim.clone())]];
                    let content = Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(sized(cx, head, width))
                        // The subtitle keeps the two columns the list insets,
                        // wrapped lines too.
                        .child(Padding::new(
                            EdgeInsets::only(2, 0, 0, 0),
                            sized(cx, subtitle, width.saturating_sub(2)),
                        ))
                        .child(sized(cx, vec![Vec::new(), Vec::new()], width))
                        .child(options.view(rows.clone(), styles.clone(), 0))
                        .child(idle.clone());
                    frame(
                        cx,
                        content.into(),
                        Vec::new(),
                        width,
                        constraints.max_height,
                    )
                })
                .into()
            }
            Stage::Picking(picker) => {
                let page = &picker.list;
                let options = self
                    .options
                    .on_accept(cx.map(Msg::Accept))
                    .on_move(cx.map(|_: usize| Msg::Moved));
                let mut rows: Vec<RichRow> = page
                    .gateways
                    .iter()
                    .map(|saved| {
                        let [on, off] = strings.gateway_dots;
                        let mark = if saved.id == page.default_id {
                            Span::styled(on, look.green.clone())
                        } else {
                            Span::styled(off, look.dim.clone())
                        };
                        RichRow {
                            prefix: String::new(),
                            label: vec![mark, Span::new(saved.name.clone())],
                            description: Some(saved.id.clone()),
                        }
                    })
                    .collect();
                rows.push(RichRow {
                    prefix: String::new(),
                    label: vec![Span::new(strings.gateway_create)],
                    description: None,
                });
                let rows = Rc::new(rows);
                let styles = OptionStyles {
                    cursor: tokens.selected(),
                    label: Style::new(),
                    description: look.dim.clone(),
                };
                // The hint follows the cursor: the row that creates says so.
                let creating = self.options.cursor() >= page.create_row();
                let hint = vec![
                    Span::styled(strings.press, look.dim.clone()),
                    Span::styled(strings.enter, look.key.clone()),
                    Span::styled(
                        if creating {
                            strings.gateway_to_create
                        } else {
                            strings.gateway_to_use
                        },
                        look.dim.clone(),
                    ),
                    Span::styled(strings.gateway_or, look.dim.clone()),
                    Span::styled(strings.esc, look.key.clone()),
                    Span::styled(strings.press_to_go_back, look.dim.clone()),
                ];
                let notice = picker.notice.clone();
                LayoutBuilder::new(move |cx, constraints| {
                    let width = width_of(constraints.max_width);
                    let head = vec![
                        vec![Span::styled(strings.gateway_title, bold.clone())],
                        Vec::new(),
                        Vec::new(),
                    ];
                    let content = Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(sized(cx, head, width))
                        .child(options.view_rich(rows.clone(), styles.clone(), 0))
                        .child(idle.clone());
                    // The note wraps inside the two columns the list insets
                    // (`footer_note_lines`), the hint follows it.
                    let mut footer = Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(sized(cx, vec![Vec::new()], width));
                    if let Some(notice) = &notice {
                        footer = footer.child(Padding::new(
                            EdgeInsets::only(2, 0, 0, 0),
                            sized(
                                cx,
                                vec![vec![Span::new(notice.clone())]],
                                width.saturating_sub(2),
                            ),
                        ));
                    }
                    footer = footer.child(sized(cx, vec![hint.clone()], width));
                    frame_of(content.into(), Some(footer.into()), constraints.max_height)
                })
                .into()
            }
            Stage::Editing { form, .. } => self.form(form, cx, &look, &bold, idle),
        }
    }
}

impl GatewayState {
    /// The form (`ProviderForm::layout`): one scrolling paragraph of the
    /// title, the fields, Save, the error and the hints.
    fn form(
        &self,
        form: &Form,
        cx: &mut Cx<'_, Self>,
        look: &Look,
        bold: &Style,
        idle: AnyWidget,
    ) -> AnyWidget {
        let strings = strings::of(cx);
        let [marked_mark, plain_mark] = strings.gateway_marks;
        let selected = self.at;
        // A field: its label (the mark, the name, a star for the required
        // ones) and the text; a click marks it (`Focus.onFocusChange`).
        let inputs: Vec<(String, AnyWidget)> = (0..FIELDS)
            .map(|index| {
                let label = format!(
                    "{}{}{}  ",
                    if index == selected {
                        marked_mark
                    } else {
                        plain_mark
                    },
                    strings.gateway_labels[index],
                    if index < VARIABLE {
                        strings.gateway_required_mark
                    } else {
                        ""
                    },
                );
                let input: AnyWidget = TextField::new()
                    .controller(self.fields[index].clone())
                    .focus_node(self.nodes[index].clone())
                    .placeholder(strings.gateway_examples[index])
                    .placeholder_style(look.dim.clone())
                    .style(Style::new())
                    .focused_style(look.cyan.clone())
                    .selection_style(Style::new())
                    .on_submitted(cx.map(|_| Msg::Submit))
                    .into();
                let input: AnyWidget = Focus::new(input)
                    .can_request_focus(false)
                    .skip_traversal(true)
                    .on_focus_change(cx.map(move |on: bool| Msg::Focused(index, on)))
                    .into();
                (label, input)
            })
            .collect();
        let tap: Handler<PointerEvent> = cx.map(|_| Msg::TapSave);
        let save: AnyWidget = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .on_tap(tap)
            .child(
                Focus::new(Text::rich([if selected == SAVE {
                    Span::styled(strings.gateway_save[0], bold.clone())
                } else {
                    Span::new(strings.gateway_save[1])
                }]))
                .focus_node(self.nodes[SAVE].clone())
                .on_focus_change(cx.map(|on: bool| Msg::Focused(SAVE, on))),
            )
            .into();
        // The suggestion follows the text and the caret of the variable's
        // field: the line is a listener of its controller.
        let suggestion: AnyWidget = {
            let (controller, form, at) = (self.fields[VARIABLE].clone(), form.clone(), self.at);
            let (key, dim) = (look.key.clone(), look.dim.clone());
            let [shortcut, complete] = strings.gateway_complete;
            ListenableBuilder::new(self.fields[VARIABLE].listenable(), move |_, _| {
                let text = controller.text();
                let at_end = controller.selection().extent_offset == text.len();
                let name = (at == VARIABLE && at_end)
                    .then(|| form.suggestion(&text))
                    .flatten();
                match name {
                    Some(name) => Text::rich([
                        Span::styled(shortcut, key.clone()),
                        Span::styled([complete, name].concat(), dim.clone()),
                    ])
                    .into(),
                    None => SizedBox::shrink().into(),
                }
            })
            .into()
        };
        let top: Vec<Line> = vec![
            vec![Span::styled(strings.gateway_create, bold.clone())],
            Vec::new(),
        ];
        let error: Option<Line> = form.error.as_ref().map(|error| {
            let message = match error {
                FormError::EnterName => strings.gateway_errors[0].to_owned(),
                FormError::EnterUrl => strings.gateway_errors[1].to_owned(),
                FormError::BadVariable => strings.gateway_errors[2].to_owned(),
                FormError::Server(message) => message.clone(),
            };
            vec![Span::styled(message, look.red.clone())]
        });
        let notes: Vec<Line> = vec![
            Vec::new(),
            vec![Span::styled(strings.gateway_required, look.dim.clone())],
            vec![Span::styled(
                strings.gateway_variable_hint,
                look.dim.clone(),
            )],
        ];
        let dim = |text: &'static str| Span::styled(text, look.dim.clone());
        let keys: Vec<Line> = vec![vec![
            Span::styled(strings.gateway_tab, look.key.clone()),
            dim(strings.gateway_move),
            Span::styled(strings.enter, look.key.clone()),
            dim(strings.gateway_next_save),
            Span::styled(strings.esc, look.key.clone()),
            dim(strings.back),
        ]];
        let dollar = Span::styled(strings.gateway_dollar, look.dim.clone());
        let label_style = bold.clone();
        let (marked, total, scroll) =
            (self.marked.clone(), self.total.clone(), self.scroll.clone());
        // A new size settles the scroll too (the room is known only after
        // the layout: `addPostFrameCallback` from the builder).
        let reveal = cx.map(|_: Duration| Msg::Reveal);
        LayoutBuilder::new(move |cx, constraints| {
            if let Some(binding) = AppBinding::maybe_of(cx) {
                binding.add_post_frame_callback(reveal.clone());
            }
            let width = width_of(constraints.max_width);
            let mut content = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            let mut row_at = 0i64;
            let mut mark_at = 0i64;
            content = content.child(sized(cx, top.clone(), width));
            row_at += i64::try_from(rows_of(cx, &top, width)).unwrap_or(0);
            for (index, (label, input)) in inputs.iter().enumerate() {
                // The value goes to a row of its own when the label would
                // crowd the cursor (`line.width() + 8 > width`).
                let crowded = segmenter_of(cx).width(label) + 8 > width;
                let mut value = Row::new();
                if index == VARIABLE {
                    value = value.child(Text::rich([dollar.clone()]));
                }
                let value = value.child(Expanded::new(1, input.clone()));
                if crowded {
                    let words = vec![vec![Span::styled(label.clone(), label_style.clone())]];
                    let rows = i64::try_from(rows_of(cx, &words, width)).unwrap_or(1);
                    if index == selected {
                        mark_at = row_at + rows;
                    }
                    row_at += rows + 1;
                    content = content.child(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(sized(cx, words, width))
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
                                Text::rich([Span::styled(label.clone(), label_style.clone())])
                                    .wrap(TextWrap::None),
                            )
                            .child(Expanded::new(1, value)),
                    );
                }
            }
            if selected == SAVE {
                mark_at = row_at;
            }
            row_at += 1;
            content = content.child(save.clone());
            if let Some(error) = &error {
                let lines = vec![error.clone()];
                row_at += i64::try_from(rows_of(cx, &lines, width)).unwrap_or(1);
                content = content.child(sized(cx, lines, width));
            }
            row_at += i64::try_from(rows_of(cx, &notes, width)).unwrap_or(0);
            content = content.child(sized(cx, notes.clone(), width));
            content = content.child(suggestion.clone());
            row_at += i64::try_from(rows_of(cx, &keys, width)).unwrap_or(0);
            content = content.child(sized(cx, keys.clone(), width));
            content = content.child(idle.clone());
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
    }
}
