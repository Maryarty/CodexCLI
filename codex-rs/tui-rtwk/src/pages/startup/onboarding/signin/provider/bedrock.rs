//! Amazon Bedrock setup (`onboarding/bedrock.rs`, `BedrockState::render`):
//! the credentials the server finds, a method list, the entries (a profile
//! name, access keys, a Bedrock API key, the region) and "setting up".
//!
//! In Flutter: a `StatefulWidget` over the core model whose `State` holds a
//! `TextEditingController` and a `FocusNode` per field, the list's cursor
//! (the shared `Options` controller) and a `ScrollController` for a pane too
//! short for the text (the oracle scrolls the marked row into view, the
//! footer stays). The wizard asks for the discovery once it is up
//! (`initState`), the keys are the fields' own and the list's node's; Esc
//! bubbles up to the step (one view back).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::AnyWidget;
use rtwk::binding::AppBinding;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, Flexible, MainAxisSize, Row, SizedBox};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::read;
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::{self, provider};
use provider::bedrock::{Bedrock, CredentialKind, Error, List, Method, Stage};

use super::super::{Line, Look, guarded_keys, rows_of, width_of};
use super::{sized, strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{fixed_presses, pressed_fixed, same_press};
use crate::widgets::key_guard::KeyGuard;
use crate::widgets::options::Options;

/// The wizard over its model.
#[derive(Clone, PartialEq, Eq)]
pub struct BedrockWizard {
    pub bedrock: Bedrock,
    pub guard: KeyGuard,
}

/// The fields: the profile name, the API key, the region, the access key
/// ID, the secret and the session token.
const PROFILE: usize = 0;
const API_KEY: usize = 1;
const REGION: usize = 2;
const KEYS: usize = 3;
/// The node of the list and of the views without a field.
const IDLE: usize = 6;

/// Widget state (R14): the fields, their nodes, the list's cursor, the
/// scroll of a short pane.
pub struct BedrockState {
    fields: [TextEditingController; 6],
    nodes: [FocusNode; 7],
    options: Options,
    scroll: ScrollController,
    /// The row of the marked line (the list's cursor, the access key field)
    /// as the last layout measured it.
    marked: Rc<Cell<i64>>,
}

pub enum Msg {
    Up,
    Down,
    /// Enter on the list: the cursor's method.
    Confirm,
    /// A digit: the method of that number.
    Digit(usize),
    /// A tap on a row of the list.
    Tap(usize),
    /// Enter in a field.
    Submit,
    /// The text changed: any key handled clears the error.
    Edited,
    /// Tab, Up, Down in the access keys: the field to mark.
    Field(usize),
    /// An access key field took or lost the focus (a click).
    Focused(usize, bool),
    Quit,
    /// The view's field takes the focus once the frame mounted it, and the
    /// marked row is scrolled into view.
    Settle,
}

impl StatefulWidget for BedrockWizard {
    type State = BedrockState;

    fn create_state(&self) -> BedrockState {
        BedrockState {
            fields: std::array::from_fn(|_| TextEditingController::new("")),
            nodes: std::array::from_fn(|_| FocusNode::new()),
            options: Options::new(),
            scroll: ScrollController::default(),
            marked: Rc::new(Cell::new(0)),
        }
    }
}

impl_widget!(stateful eq BedrockWizard);

/// Which field a view types in.
fn field_of(view: &Stage) -> Option<usize> {
    match view {
        Stage::ProfileEntry => Some(PROFILE),
        Stage::ApiKeyEntry => Some(API_KEY),
        Stage::RegionEntry { .. } => Some(REGION),
        Stage::AccessKeyEntry { field } => Some(KEYS + field),
        _ => None,
    }
}

/// The views a list shows in.
fn lists(view: &Stage) -> bool {
    matches!(view, Stage::Methods(_) | Stage::EnvironmentInstructions)
}

/// Two views are the same screen (their fields keep their text).
fn same_screen(a: &Stage, b: &Stage) -> bool {
    a == b
        || matches!(
            (a, b),
            (Stage::AccessKeyEntry { .. }, Stage::AccessKeyEntry { .. })
        )
}

impl BedrockState {
    fn after_frame(cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::Settle));
        }
    }

    fn emit(cx: &mut Cx<'_, Self>, intent: provider::bedrock::Intent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Onboarding(Intent::SignIn(auth::Intent::Provider(
                provider::Intent::Bedrock(intent),
            ))),
        );
    }

    /// Any key handled clears the error.
    fn edited(cx: &mut Cx<'_, Self>, w: &BedrockWizard) {
        if w.bedrock.error.is_some() {
            let hs = read::<Hs, _>(cx);
            cx.emit(
                &hs.screen,
                ScreenIntent::Onboarding(Intent::SignIn(auth::Intent::Provider(
                    provider::Intent::Edited,
                ))),
            );
        }
    }

    fn text(&self, field: usize) -> String {
        self.fields[field].text().to_string()
    }

    /// A new screen: its fields start empty (the region from what was
    /// tried), the list on its first row.
    fn enter(&mut self, view: &Stage) {
        match view {
            Stage::ProfileEntry => self.fields[PROFILE].set_text(""),
            Stage::ApiKeyEntry => self.fields[API_KEY].set_text(""),
            Stage::RegionEntry { prefill, .. } => self.fields[REGION].set_text(prefill),
            Stage::AccessKeyEntry { .. } => {
                for field in &self.fields[KEYS..] {
                    field.set_text("");
                }
            }
            _ => {}
        }
        self.options.set(0);
    }

    fn choose(cx: &mut Cx<'_, Self>, w: &BedrockWizard, index: usize) {
        if let Some(method) = w.bedrock.methods().get(index).copied() {
            Self::emit(cx, provider::bedrock::Intent::Authenticate(method));
        }
    }

    /// The list's keys and the access keys' moves (`move_provider_field`);
    /// the rest goes on to the field or up to the step.
    fn keys(&self, w: &BedrockWizard, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let now: Rc<dyn Fn() -> Duration> =
            Rc::new(move || clock.as_ref().map_or(Duration::ZERO, |clock| clock.now()));
        let quit = {
            let send = send.clone();
            Rc::new(move || send(Msg::Quit))
        };
        for (index, node) in self.nodes.iter().enumerate() {
            let typing = index != IDLE;
            let guarded = guarded_keys(
                w.guard.clone(),
                now.clone(),
                Rc::new(move || typing),
                quit.clone(),
            );
            let send = send.clone();
            node.set_on_key_event(Some(Rc::new(move |node, key: KeyPress| {
                if guarded(node, key.clone()) == KeyEventResult::Handled {
                    return KeyEventResult::Handled;
                }
                let is = |id| pressed_fixed(id, &key);
                let msg = if index == IDLE {
                    let digit = fixed_presses(fixed::PICK_NUMBERS)
                        .iter()
                        .position(|digit| same_press(digit, &key));
                    if is(fixed::ONBOARD_PREVIOUS) {
                        Some(Msg::Up)
                    } else if is(fixed::ONBOARD_NEXT) {
                        Some(Msg::Down)
                    } else if is(fixed::ONBOARD_CONFIRM) {
                        Some(Msg::Confirm)
                    } else {
                        digit.map(Msg::Digit)
                    }
                } else if index >= KEYS {
                    let field = index - KEYS;
                    if is(fixed::FORM_PREVIOUS) {
                        Some(Msg::Field((field + 2) % 3))
                    } else if is(fixed::FORM_NEXT) {
                        Some(Msg::Field((field + 1) % 3))
                    } else {
                        None
                    }
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

impl State for BedrockState {
    type Widget = BedrockWizard;
    type Msg = Msg;

    fn init_state(&mut self, w: &BedrockWizard, cx: &mut Cx<'_, Self>) {
        self.keys(w, cx);
        Self::after_frame(cx);
        if w.bedrock.stage == Stage::Start {
            Self::emit(cx, provider::bedrock::Intent::Discover);
        }
    }

    fn did_update_widget(&mut self, old: &BedrockWizard, w: &BedrockWizard, cx: &mut Cx<'_, Self>) {
        if !same_screen(&old.bedrock.stage, &w.bedrock.stage) {
            self.enter(&w.bedrock.stage);
        }
        if old.bedrock.stage != w.bedrock.stage {
            Self::after_frame(cx);
        }
    }

    fn dispose(&mut self) {
        for field in &self.fields {
            field.dispose();
        }
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, w: &BedrockWizard, cx: &mut Cx<'_, Self>) -> Rebuild {
        let len = w.bedrock.methods().len();
        match msg {
            Msg::Up | Msg::Down if lists(&w.bedrock.stage) => {
                self.options
                    .step(if matches!(msg, Msg::Up) { -1 } else { 1 }, len);
                Self::after_frame(cx);
                Self::edited(cx, w);
                return Rebuild::Yes;
            }
            Msg::Up | Msg::Down => {}
            Msg::Confirm if lists(&w.bedrock.stage) => Self::choose(cx, w, self.options.cursor()),
            Msg::Confirm => {}
            Msg::Digit(index) if lists(&w.bedrock.stage) && index < len => {
                Self::choose(cx, w, index);
            }
            Msg::Digit(_) => {}
            Msg::Tap(index) => {
                if self.options.cursor() == index {
                    Self::choose(cx, w, index);
                } else {
                    self.options.set(index);
                    Self::after_frame(cx);
                    return Rebuild::Yes;
                }
            }
            Msg::Submit => {
                let intent = match &w.bedrock.stage {
                    Stage::AccessKeyEntry { .. } => {
                        provider::bedrock::Intent::SubmitKeys(std::array::from_fn(|field| {
                            self.text(KEYS + field)
                        }))
                    }
                    view => match field_of(view) {
                        Some(field) => provider::bedrock::Intent::Submit(self.text(field)),
                        None => return Rebuild::No,
                    },
                };
                Self::emit(cx, intent);
            }
            Msg::Edited => Self::edited(cx, w),
            Msg::Focused(field, true) => {
                if matches!(w.bedrock.stage, Stage::AccessKeyEntry { field: f } if f != field) {
                    Self::emit(cx, provider::bedrock::Intent::Field(field));
                }
            }
            Msg::Focused(_, false) => {}
            // The focus moves at once (`FocusNode.requestFocus` in the key
            // handler); its change marks the field in the model.
            Msg::Field(field) => self.nodes[KEYS + field].request_focus(),
            Msg::Settle => {
                let node = field_of(&w.bedrock.stage).unwrap_or(IDLE);
                if !self.nodes[node].has_focus() {
                    self.nodes[node].request_focus();
                }
                if self.scroll.has_clients() {
                    let view = self.scroll.position().viewport_dimension();
                    self.scroll.jump_to((self.marked.get() + 2 - view).max(0));
                }
            }
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &BedrockWizard, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let tokens = Tokens::of(cx);
        let look = Look {
            dim: Style::new().attrs(Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        };
        let bedrock = &w.bedrock;
        let [mark, title] = strings.bedrock_title;
        let mut top: Vec<Line> = vec![vec![Span::new(mark), Span::new(title).bold()], Vec::new()];
        let dim = |text: &str| vec![Span::styled(text.to_owned(), look.dim.clone())];
        let plain = |text: &str| vec![Span::new(text.to_owned())];
        let found = &bedrock.discovery;
        match &bedrock.stage {
            Stage::Start | Stage::Discovering(_) => top.push(dim(strings.bedrock_discovering)),
            Stage::Configuring { .. } => top.push(dim(strings.bedrock_configuring)),
            Stage::Methods(list) => {
                let detected = *list == List::Detected;
                if detected && found.profiles.len() == 1 {
                    let profile = &found.profiles[0];
                    top.push(plain(
                        &[strings.bedrock_profile_detected, &profile.name].concat(),
                    ));
                    if let Some(region) = &profile.region {
                        top.push(dim(&[strings.bedrock_region, region].concat()));
                    }
                } else if detected && found.profiles.len() > 1 {
                    top.push(plain(strings.bedrock_choose_profile));
                } else if detected && !found.environment.is_empty() {
                    top.push(plain(strings.bedrock_environment_detected));
                } else {
                    if detected {
                        top.push(plain(strings.bedrock_none_found));
                    }
                    top.push(plain(strings.bedrock_choose_method));
                }
                top.push(Vec::new());
            }
            Stage::ProfileEntry => {
                top.extend([plain(strings.bedrock_profile_guidance), Vec::new()])
            }
            Stage::ApiKeyEntry => top.extend([plain(strings.bedrock_api_key_guidance), Vec::new()]),
            Stage::RegionEntry { .. } => {
                top.extend([plain(strings.bedrock_region_guidance), Vec::new()]);
            }
            Stage::AccessKeyEntry { .. } => {
                top.extend([plain(strings.bedrock_access_keys_guidance), Vec::new()]);
            }
            Stage::EnvironmentInstructions => top.extend([
                plain(strings.bedrock_environment_guidance),
                Vec::new(),
                vec![
                    Span::new(strings.bedrock_setup_guide),
                    Span::styled(
                        strings.bedrock_setup_guide_url,
                        look.cyan.clone().attrs(Attrs::UNDERLINE),
                    )
                    .link(strings.bedrock_setup_guide_url.to_owned()),
                ],
                Vec::new(),
            ]),
        }
        // The method list: `> 1. title` (cyan on the cursor), the dim
        // description under it, a blank line after each.
        let rows: Vec<(Line, Option<Line>)> = if lists(&bedrock.stage) {
            bedrock
                .methods()
                .into_iter()
                .enumerate()
                .map(|(index, method)| {
                    let (title, description) = method_text(strings, bedrock, method);
                    let on = index == self.options.cursor();
                    let head = format!("{} {}. {title}", if on { ">" } else { " " }, index + 1);
                    let head = if on {
                        vec![Span::styled(head, look.cyan.clone())]
                    } else {
                        vec![Span::new(head)]
                    };
                    let body = (!description.is_empty() && bedrock.describes())
                        .then(|| dim(&format!("     {description}")));
                    (head, body)
                })
                .collect()
        } else {
            Vec::new()
        };
        // The fields of an entry: a label and the field, the marked one of
        // the access keys cyan.
        let field = |index: usize, label: String, secret: bool, marked: bool| -> AnyWidget {
            let style = if marked {
                look.cyan.clone()
            } else {
                Style::new()
            };
            let input: AnyWidget = TextField::new()
                .controller(self.fields[index].clone())
                .focus_node(self.nodes[index].clone())
                .obscure_text(secret)
                .obscure_reveal_last(secret)
                .obscure_reveal_timeout(None)
                .obscuring_character(strings.secret)
                .style(Style::new())
                .focused_style(look.cyan.clone())
                .selection_style(Style::new())
                .on_changed(cx.map(|_| Msg::Edited))
                .on_submitted(cx.map(|_| Msg::Submit))
                .into();
            // A click on an access key field marks it (`Focus.onFocusChange`).
            let input: AnyWidget = if index >= KEYS {
                Focus::new(input)
                    .can_request_focus(false)
                    .skip_traversal(true)
                    .on_focus_change(cx.map(move |on: bool| Msg::Focused(index - KEYS, on)))
                    .into()
            } else {
                input
            };
            Row::new()
                .child(Text::rich([Span::styled(label, style)]).wrap(TextWrap::None))
                .child(Expanded::new(1, input))
                .into()
        };
        let fields: Vec<AnyWidget> = match &bedrock.stage {
            Stage::ProfileEntry => vec![field(
                PROFILE,
                strings.bedrock_profile_label.into(),
                false,
                false,
            )],
            Stage::ApiKeyEntry => vec![field(
                API_KEY,
                strings.bedrock_api_key_label.into(),
                true,
                false,
            )],
            Stage::RegionEntry { .. } => vec![field(
                REGION,
                strings.bedrock_region_label.into(),
                false,
                false,
            )],
            Stage::AccessKeyEntry { field: marked } => strings
                .bedrock_access_key_labels
                .iter()
                .enumerate()
                .map(|(index, label)| {
                    let on = index == *marked;
                    let label = format!("{} {label}", if on { ">" } else { " " });
                    field(KEYS + index, label, index > 0, on)
                })
                .collect(),
            _ => Vec::new(),
        };
        let marked_field = match &bedrock.stage {
            Stage::AccessKeyEntry { field } => Some(*field),
            _ => None,
        };
        let mut footer: Vec<Line> = Vec::new();
        if !matches!(bedrock.stage, Stage::Start | Stage::Discovering(_)) {
            footer.push(Vec::new());
            let press = |what: &'static str, key: &'static str| {
                vec![
                    Span::styled(strings.press, look.dim.clone()),
                    Span::styled(key, look.key.clone()),
                    Span::styled(what, look.dim.clone()),
                ]
            };
            if !matches!(bedrock.stage, Stage::Configuring { .. }) {
                footer.push(press(strings.press_to_continue, strings.enter));
            }
            footer.push(press(strings.press_to_go_back, strings.esc));
        }
        if let Some(error) = &bedrock.error {
            let message = match error {
                Error::Discover(message) => [strings.bedrock_discover_failed, message].concat(),
                Error::Setup(message) => [strings.bedrock_setup_failed, message].concat(),
            };
            footer.extend([Vec::new(), vec![Span::styled(message, look.red.clone())]]);
        }
        let idle: AnyWidget = Focus::new(SizedBox::shrink())
            .focus_node(self.nodes[IDLE].clone())
            .into();
        let tap = |index: usize| -> Handler<PointerEvent> { cx.map(move |_| Msg::Tap(index)) };
        let taps: Vec<Handler<PointerEvent>> = (0..rows.len()).map(tap).collect();
        let marked = self.marked.clone();
        let cursor = self.options.cursor();
        let scroll = self.scroll.clone();
        LayoutBuilder::new(move |cx, constraints| {
            let width = width_of(constraints.max_width);
            let top_rows = rows_of(cx, &top, width);
            let mut content = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(sized(cx, top.clone(), width));
            let mut row_at = i64::try_from(top_rows).unwrap_or(0);
            let mut mark_at = 0;
            for (index, (head, body)) in rows.iter().enumerate() {
                let mut lines = vec![head.clone()];
                lines.extend(body.clone());
                lines.push(Vec::new());
                if index == cursor {
                    mark_at = row_at;
                }
                row_at += i64::try_from(rows_of(cx, &lines, width)).unwrap_or(0);
                content = content.child(
                    GestureDetector::new()
                        .behavior(HitTestBehavior::Opaque)
                        .on_tap(taps[index].clone())
                        .child(sized(cx, lines, width)),
                );
            }
            for (index, field) in fields.iter().enumerate() {
                if marked_field == Some(index) {
                    mark_at = row_at;
                }
                row_at += 1;
                content = content.child(field.clone());
            }
            content = content.child(idle.clone());
            marked.set(mark_at);
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Flexible::new(
                    1,
                    SingleChildScrollView::new(content).controller(scroll.clone()),
                ));
            if !footer.is_empty() {
                column = column.child(sized(cx, footer.clone(), width));
            }
            column.into()
        })
        .into()
    }
}

/// A method's title and description (`render_methods`).
fn method_text(
    strings: &'static strings::Strings,
    bedrock: &Bedrock,
    method: Method,
) -> (String, String) {
    let found = &bedrock.discovery;
    let pair = |[title, description]: [&str; 2]| (title.to_owned(), description.to_owned());
    match method {
        Method::Profile(index) => {
            let profile = &found.profiles[index];
            if found.profiles.len() == 1 {
                (
                    [strings.bedrock_continue_with, &profile.name].concat(),
                    strings.bedrock_existing_credentials.to_owned(),
                )
            } else {
                (
                    profile.name.clone(),
                    profile.region.clone().unwrap_or_default(),
                )
            }
        }
        Method::Environment => {
            let api_key = found
                .environment
                .iter()
                .any(|credential| credential.kind == CredentialKind::BedrockApiKey);
            pair([
                strings.bedrock_detected_credentials,
                if api_key {
                    strings.bedrock_environment_api_key
                } else {
                    strings.bedrock_environment_access_keys
                },
            ])
        }
        Method::OtherMethods if bedrock.stage == Stage::EnvironmentInstructions => {
            pair([strings.bedrock_another_method, ""])
        }
        Method::OtherMethods => pair([
            strings.bedrock_other_methods,
            strings.bedrock_other_methods_description,
        ]),
        Method::ManualProfile => pair(strings.bedrock_manual_profile),
        Method::AccessKeys => pair(strings.bedrock_access_keys),
        Method::EnvironmentInstructions => pair(strings.bedrock_environment_variables),
        Method::ApiKey => pair(strings.bedrock_api_key),
    }
}
