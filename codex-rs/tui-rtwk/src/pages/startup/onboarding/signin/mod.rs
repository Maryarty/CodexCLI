//! The sign-in step of the startup page (S02) and the provider step's
//! place (S03). Oracle: `onboarding/auth.rs` (`render_pick_mode`,
//! `render_continue_in_browser`, `render_chatgpt_success_message`,
//! `render_api_key_entry`, `handle_key_event`), `auth/headless_chatgpt_login.rs`
//! (`render_device_code_login`) and the activation guard of
//! `activation_key_guard.rs`.
//!
//! In Flutter: a `StatefulWidget` over the model's sign-in (watched by the
//! page), its `State` holding what is the view's alone: the highlighted
//! option (setState), the API key's `TextEditingController` and
//! `FocusNode`, and the guard that keeps a held Enter from activating the
//! next menu. The keys are a `Focus` with bindings; the key repeats and
//! releases, which the focus path does not tell apart from presses, an
//! `InputListener::with_raw_key_repeats` around it. What the step does is
//! an intent to core (`Intent::SignIn`); the logins run in the host.
//!
//! The menu is a column of rows rather than a `SelectionList`: the oracle's
//! cursor wraps round the ends, `j`/`k` and the digits move it, and in a
//! short pane the descriptions and gaps give way to keep every option and
//! the hint in view (`compact_options`), which the rows measure for
//! themselves (`widgets::measure`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, InputListener};
use rtwk::editing::{TextEditingController, TextField, copy_to_clipboard};
use rtwk::focus::{Focus, FocusNode, FocusOnKeyEventCallback, KeyEventResult, KeyPress};
use rtwk::layout::{
    Alignment, Border, BorderTitle, BorderType, BoxWidget, Column, CrossAxisAlignment,
    MainAxisSize, OverflowBox, SizedBox,
};
use rtwk::prelude::Mods;
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::input::{InputEvent, KeyCode, KeyKind};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::{self, Method, Provider, SignIn};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::widgets::measure;

pub mod strings;
use strings::Strings;

/// The sign-in step over the model's sign-in; it also stands in the
/// provider step's place while a provider's setup (S03) is chosen, so its
/// highlight outlives the provider step (Esc there comes back to it).
#[derive(Clone, Debug, PartialEq)]
pub struct SignInStep {
    pub sign_in: SignIn,
}

/// A row of the menu (`SignInOption`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuRow {
    ChatGpt,
    DeviceCode,
    Other,
    ApiKey,
    Provider(Provider),
}

impl MenuRow {
    /// A row of "Sign in another way".
    fn is_alternative(self) -> bool {
        matches!(self, MenuRow::ApiKey | MenuRow::Provider(_))
    }
}

/// The rows of the menu the highlight is in (`displayed_sign_in_options`).
fn options(offer: &auth::Offer, highlighted: MenuRow) -> Vec<MenuRow> {
    if highlighted.is_alternative() {
        let mut rows = vec![MenuRow::ApiKey];
        for provider in [Provider::Bedrock, Provider::Gcp, Provider::Gateway] {
            if offer.providers.offers(provider) {
                rows.push(MenuRow::Provider(provider));
            }
        }
        return rows;
    }
    let mut rows = vec![MenuRow::ChatGpt];
    if offer.chatgpt {
        rows.push(MenuRow::DeviceCode);
    }
    if offer.api_key {
        rows.push(MenuRow::Other);
    }
    rows
}

/// The key a guard holds back: Enter or a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuardKey {
    Enter,
    Char(char),
}

impl GuardKey {
    fn of_press(key: &KeyPress) -> Option<GuardKey> {
        if key.mods != Mods::NONE && key.mods != Mods::SHIFT {
            return None;
        }
        match key.code {
            KeyCode::Enter => Some(GuardKey::Enter),
            KeyCode::Char => key.ch.map(GuardKey::Char),
            _ => None,
        }
    }
}

/// The oracle's `ActivationKeyGuard` with `reported_repeats_only` false: the
/// key that changed the view is held back until it is released, another
/// key is pressed, or a second passes without it (a terminal that reports
/// no repeats repeats with presses).
#[derive(Clone, Copy, Debug)]
struct Guard {
    key: GuardKey,
    until: Duration,
}

const GUARD: Duration = Duration::from_secs(1);

#[derive(Default)]
struct Guards {
    guard: Option<Guard>,
}

impl Guards {
    /// Whether the press of `key` at `now` is held back.
    fn press(&mut self, key: Option<GuardKey>, now: Duration) -> bool {
        let Some(guard) = self.guard else {
            return false;
        };
        if key == Some(guard.key) && now <= guard.until {
            self.guard = Some(Guard {
                key: guard.key,
                until: guard.until.max(now + GUARD),
            });
            return true;
        }
        self.guard = None;
        false
    }

    /// A repeat of `key`: held back while guarded, the guard prolonged.
    fn held(&mut self, key: Option<GuardKey>, now: Duration) {
        match self.guard {
            Some(guard) if key == Some(guard.key) => {
                self.guard = Some(Guard {
                    key: guard.key,
                    until: guard.until.max(now + GUARD),
                });
            }
            Some(_) => self.guard = None,
            None => {}
        }
    }

    fn release(&mut self, key: Option<GuardKey>) {
        if self.guard.is_some_and(|guard| Some(guard.key) == key) {
            self.guard = None;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Up,
    Down,
    /// A digit (or `y`/`n`): the option at the index, pressed as `key`.
    Index(usize, char),
    Confirm,
    Cancel,
    CopyLink,
    Submitted(Rc<str>),
    /// The key entry is mounted: its field takes the focus.
    FocusField,
    /// A quit key: the app ends, a login started cancelled.
    Quit,
}

/// Widget state: the highlighted option, the API key's field and the
/// activation guard (R14).
pub struct SignInState {
    highlighted: MenuRow,
    field: TextEditingController,
    field_node: FocusNode,
    node: FocusNode,
    guards: Rc<RefCell<Guards>>,
    /// "Copy requested; check your clipboard" under the link, until the
    /// browser login ends.
    copied: bool,
}

impl StatefulWidget for SignInStep {
    type State = SignInState;

    fn create_state(&self) -> SignInState {
        SignInState {
            highlighted: if self.sign_in.offer.chatgpt {
                MenuRow::ChatGpt
            } else {
                MenuRow::Other
            },
            field: TextEditingController::default(),
            field_node: FocusNode::new(),
            node: FocusNode::new(),
            guards: Rc::new(RefCell::new(Guards::default())),
            copied: false,
        }
    }
}

impl_widget!(stateful eq SignInStep);

/// The time on the app's clock (a test's fake clock moves it).
fn now<S: State>(cx: &Cx<'_, S>) -> Duration {
    AppBinding::maybe_of(cx).map_or(Duration::ZERO, |binding| binding.clock().now())
}

impl SignInState {
    fn emit(cx: &mut Cx<'_, Self>, intent: auth::Intent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::SignIn(intent)));
    }

    /// The API key entry opened: the field starts from the key the
    /// environment gave, selected so that typing or a paste replaces it and
    /// Backspace clears it (`prepopulated_from_env`).
    fn open_field(&mut self, prefill: Option<&str>) {
        let text = prefill.unwrap_or_default();
        self.field.set_text(text);
        if !text.is_empty() {
            self.field.select_all();
        }
    }

    fn focus_field_after_frame(cx: &mut Cx<'_, Self>) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.map(|_: Duration| Msg::FocusField));
        }
    }

    fn guard(&self, key: GuardKey, cx: &Cx<'_, Self>) {
        self.guards.borrow_mut().guard = Some(Guard {
            key,
            until: now(cx) + GUARD,
        });
    }

    /// `activate_highlighted_mode` and `handle_sign_in_option`.
    fn activate(&mut self, choice: MenuRow, key: GuardKey, cx: &mut Cx<'_, Self>) {
        self.highlighted = choice;
        if choice == MenuRow::Other || choice.is_alternative() {
            self.guard(key, cx);
        }
        match choice {
            MenuRow::Other => {
                self.highlighted = MenuRow::ApiKey;
                Self::emit(cx, auth::Intent::Menu);
            }
            MenuRow::ChatGpt => Self::emit(cx, auth::Intent::Choose(Method::ChatGpt)),
            MenuRow::DeviceCode => Self::emit(cx, auth::Intent::Choose(Method::DeviceCode)),
            MenuRow::ApiKey => Self::emit(cx, auth::Intent::Choose(Method::ApiKey)),
            MenuRow::Provider(provider) => {
                Self::emit(cx, auth::Intent::Choose(Method::Provider(provider)));
            }
        }
    }

    /// `move_highlight`: round the ends, past ChatGPT when its login is not
    /// allowed.
    fn step(&mut self, delta: isize, w: &SignInStep) {
        let mut rows = options(&w.sign_in.offer, self.highlighted);
        if !w.sign_in.offer.chatgpt {
            rows.retain(|row| *row != MenuRow::ChatGpt);
        }
        if rows.is_empty() {
            return;
        }
        let len = rows.len() as isize;
        let at = rows
            .iter()
            .position(|row| *row == self.highlighted)
            .unwrap_or(0) as isize;
        self.highlighted = rows[(at + delta).rem_euclid(len) as usize];
    }
}

impl State for SignInState {
    type Widget = SignInStep;
    type Msg = Msg;

    fn did_update_widget(&mut self, old: &SignInStep, w: &SignInStep, cx: &mut Cx<'_, Self>) {
        let (was, is) = (&old.sign_in.state, &w.sign_in.state);
        if std::mem::discriminant(was) == std::mem::discriminant(is) {
            return;
        }
        if !matches!(is, auth::State::Browser { .. }) {
            self.copied = false;
        }
        // The key entry gets the keys while it is shown (its node once the
        // frame mounted it: `addPostFrameCallback`), the step's own node
        // the others.
        if let auth::State::ApiKey { prefill } = is {
            self.open_field(prefill.as_deref());
            Self::focus_field_after_frame(cx);
        } else {
            self.node.request_focus();
        }
    }

    fn init_state(&mut self, w: &SignInStep, cx: &mut Cx<'_, Self>) {
        // Every key press meets the guard first, on the step's node and on
        // the key entry's (whose field takes Enter and the arrows).
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let send = cx.local_sender();
        let on_key = |typing: Option<TextEditingController>| -> FocusOnKeyEventCallback {
            let (guards, clock, send) = (self.guards.clone(), clock.clone(), send.clone());
            Rc::new(move |_, key| {
                let now = clock.as_ref().map_or(Duration::ZERO, |clock| clock.now());
                if guards.borrow_mut().press(GuardKey::of_press(&key), now) {
                    return KeyEventResult::Handled;
                }
                // `keys::QUIT`; a `q` is text once the key entry holds some
                // (`suppress_quit_while_typing`).
                let chord = key.mods == Mods::CTRL && matches!(key.ch, Some('c' | 'd'));
                let q = key.mods == Mods::NONE && key.ch == Some('q');
                let typed = typing
                    .as_ref()
                    .is_some_and(|field| !field.text().is_empty());
                if chord || (q && !typed) {
                    send(Msg::Quit);
                    return KeyEventResult::Handled;
                }
                KeyEventResult::Ignored
            })
        };
        self.node.set_on_key_event(Some(on_key(None)));
        self.field_node
            .set_on_key_event(Some(on_key(Some(self.field.clone()))));
        if let auth::State::ApiKey { prefill } = &w.sign_in.state {
            self.open_field(prefill.as_deref());
            Self::focus_field_after_frame(cx);
        }
    }

    fn dispose(&mut self) {
        self.field.dispose();
    }

    fn update(&mut self, msg: Msg, w: &SignInStep, cx: &mut Cx<'_, Self>) -> Rebuild {
        let state = &w.sign_in.state;
        match msg {
            Msg::Up | Msg::Down => {
                if *state == auth::State::Pick {
                    self.step(if msg == Msg::Up { -1 } else { 1 }, w);
                }
            }
            Msg::Index(index, key) => {
                if *state != auth::State::Pick {
                    return Rebuild::No;
                }
                let rows = options(&w.sign_in.offer, self.highlighted);
                if let Some(choice) = rows.get(index).copied()
                    && (choice != MenuRow::ChatGpt || w.sign_in.offer.chatgpt)
                {
                    self.activate(choice, GuardKey::Char(key), cx);
                }
            }
            Msg::Confirm => match state {
                auth::State::Pick => self.activate(self.highlighted, GuardKey::Enter, cx),
                auth::State::SuccessMessage => Self::emit(cx, auth::Intent::Continue),
                _ => {}
            },
            Msg::Submitted(text) => {
                Self::emit(cx, auth::Intent::SaveApiKey(text.to_string()));
            }
            Msg::Cancel => {
                if *state == auth::State::Pick && self.highlighted.is_alternative() {
                    self.highlighted = MenuRow::Other;
                    Self::emit(cx, auth::Intent::Menu);
                } else {
                    Self::emit(cx, auth::Intent::Cancel);
                }
            }
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
                return Rebuild::No;
            }
            Msg::FocusField => {
                self.field_node.request_focus();
                return Rebuild::No;
            }
            Msg::CopyLink => {
                if let auth::State::Browser { url, .. } = state
                    && !url.is_empty()
                {
                    copy_to_clipboard(cx, url);
                    self.copied = true;
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &SignInStep, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let tokens = Tokens::of(cx);
        let look = Look {
            // The oracle's `.dim()`: the attribute on the terminal's colour
            // (the theme's `dim` is a grey of its own).
            dim: Style::new().attrs(Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        };
        let error = w.sign_in.error.as_ref().map(|error| match error {
            auth::Error::ApiKeyDisabled => strings.api_key_disabled.to_owned(),
            auth::Error::EmptyApiKey => strings.empty_api_key.to_owned(),
            auth::Error::SaveFailed(message) => strings.save_failed(message, &num),
            auth::Error::Unexpected(response) => strings.unexpected(response, &num),
            auth::Error::Server(message) => message.clone(),
        });
        let mut focus = Focus::new(SizedBox::shrink())
            .focus_node(self.node.clone())
            .autofocus();
        let body: AnyWidget = match &w.sign_in.state {
            auth::State::Pick => {
                let pick = Pick {
                    offer: w.sign_in.offer.clone(),
                    highlighted: self.highlighted,
                    error,
                };
                LayoutBuilder::new(move |cx, constraints| {
                    let width = width_of(constraints.max_width);
                    let height = usize::try_from(constraints.max_height).unwrap_or(usize::MAX);
                    paragraph(pick.lines(cx, strings, &look, &num, width, height))
                })
                .into()
            }
            auth::State::Browser { url, .. } => {
                focus = focus.bind(KeyPress::char('c'), cx.send(Msg::CopyLink));
                let lines = browser_lines(strings, &look, url, self.copied);
                paragraph(lines)
            }
            auth::State::DeviceCode { code, .. } => {
                let lines = device_code_lines(strings, &look, code.as_ref());
                paragraph(lines)
            }
            auth::State::SuccessMessage => {
                let lines = success_lines(strings, &look);
                paragraph(lines)
            }
            auth::State::Success => {
                Text::rich([Span::styled(strings.signed_in, look.green)]).into()
            }
            auth::State::ApiKeyConfigured => Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::rich([Span::styled(
                    strings.api_key_configured,
                    look.green,
                )]))
                .child(SizedBox::new(None, Some(1), None))
                .child(Text::new(strings.api_key_billing))
                .into(),
            auth::State::ApiKey { .. } => {
                let prefilled = w.sign_in.offer.env_api_key.is_some()
                    && matches!(&w.sign_in.state, auth::State::ApiKey { prefill: Some(_) });
                api_key_entry(
                    strings,
                    &look,
                    prefilled,
                    error,
                    TextField::new()
                        .controller(self.field.clone())
                        .focus_node(self.field_node.clone())
                        .autofocus(true)
                        .placeholder(strings.api_key_placeholder)
                        .placeholder_style(look.dim.clone())
                        .style(Style::new())
                        .focused_style(Style::new())
                        .selection_style(Style::new())
                        .on_submitted(cx.map(Msg::Submitted)),
                )
            }
            // The provider's setup (S03) is not built yet: its place is
            // empty, and Esc comes back to the menu.
            auth::State::Provider(_) => SizedBox::shrink().into(),
        };
        focus.child = body;
        for (key, msg) in [
            (KeyPress::UP, Msg::Up),
            (KeyPress::char('k'), Msg::Up),
            (KeyPress::DOWN, Msg::Down),
            (KeyPress::char('j'), Msg::Down),
            (KeyPress::char('1'), Msg::Index(0, '1')),
            (KeyPress::char('y'), Msg::Index(0, 'y')),
            (KeyPress::char('2'), Msg::Index(1, '2')),
            (KeyPress::char('n'), Msg::Index(1, 'n')),
            (KeyPress::char('3'), Msg::Index(2, '3')),
            (KeyPress::char('4'), Msg::Index(3, '4')),
            (KeyPress::ENTER, Msg::Confirm),
            (KeyPress::ESC, Msg::Cancel),
        ] {
            focus = focus.bind(key, cx.send(msg));
        }
        // The repeats and releases the focus path cannot tell from presses:
        // a held key moves the highlight and deletes, nothing else
        // (`KeyEventKind::Press` checks of `handle_key_event`); a release
        // ends the guard of its key.
        let guards = self.guards.clone();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let pick = w.sign_in.state == auth::State::Pick;
        let entry = matches!(w.sign_in.state, auth::State::ApiKey { .. });
        InputListener::with_raw_key_repeats(focus, move |event| {
            let InputEvent::Key(key) = event else {
                return false;
            };
            let guard_key = KeyPress::from_event(key)
                .as_ref()
                .and_then(GuardKey::of_press)
                .or(match (key.code, key.text.chars().next()) {
                    (KeyCode::Enter, _) => Some(GuardKey::Enter),
                    (KeyCode::Char, Some(c)) => Some(GuardKey::Char(c)),
                    _ => None,
                });
            match key.kind {
                KeyKind::Release => {
                    guards.borrow_mut().release(guard_key);
                    false
                }
                KeyKind::Repeat => {
                    let now = clock.as_ref().map_or(Duration::ZERO, |clock| clock.now());
                    guards.borrow_mut().held(guard_key, now);
                    let moves = matches!(key.code, KeyCode::Up | KeyCode::Down)
                        || (pick && matches!(guard_key, Some(GuardKey::Char('j' | 'k'))));
                    let edits = entry && matches!(key.code, KeyCode::Backspace | KeyCode::Escape);
                    !(moves || edits)
                }
                _ => false,
            }
        })
        .into()
    }
}

/// The styles of the step.
#[derive(Clone)]
struct Look {
    dim: Style,
    /// A key's name in a hint (`key_hint_style`: bold, not dim).
    key: Style,
    cyan: Style,
    green: Style,
    red: Style,
}

type Line = Vec<Span>;

fn width_of(max_width: i32) -> usize {
    usize::try_from(max_width).unwrap_or(0)
}

/// The text of a line, for measuring it.
fn text_of(line: &[Span]) -> String {
    line.iter().map(|span| span.text.as_ref()).collect()
}

/// A line that is one word after its indent (a link, a code): ratatui's
/// wrapper breaks a word too long for the line where it stands, the indent
/// kept before it, which is wrapping between graphemes for such a line;
/// `TextWrap::Word` would move the word under the indent first.
fn one_word(text: &str) -> bool {
    !text.trim_start_matches(' ').contains(' ')
}

/// The rows the lines take at `width`.
fn rows_of<S>(cx: &mut Cx<'_, S>, lines: &[Line], width: usize) -> usize {
    let seg = segmenter_of(cx);
    let width = width.max(1);
    lines
        .iter()
        .map(|line| {
            let text = text_of(line);
            if one_word(&text) {
                seg.width(&text).div_ceil(width).max(1)
            } else {
                measure::rows(seg, &text, width).len().max(1)
            }
        })
        .sum()
}

/// The lines as a paragraph that wraps them and is cut off at the bottom of
/// its room (`Paragraph::wrap(Wrap { trim: false })`).
fn paragraph(lines: Vec<Line>) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let rows = rows_of(cx, &lines, width_of(constraints.max_width));
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for line in lines.iter().cloned() {
            column = if line.is_empty() {
                column.child(SizedBox::new(None, Some(1), None))
            } else if one_word(&text_of(&line)) {
                column.child(Text::rich(line).wrap(TextWrap::Grapheme))
            } else {
                column.child(Text::rich(line))
            };
        }
        OverflowBox::new(column)
            .max_height(i32::try_from(rows).unwrap_or(i32::MAX))
            .alignment(Alignment::TOP_LEFT)
            .into()
    })
    .into()
}

/// `  Press <key> <what>`, the words dim.
fn hint(strings: &Strings, look: &Look, key: &'static str, what: &'static str) -> Line {
    vec![
        Span::styled(strings.press, look.dim.clone()),
        Span::styled(key, look.key.clone()),
        Span::styled(what, look.dim.clone()),
    ]
}

/// The menu (`render_pick_mode`).
struct Pick {
    offer: auth::Offer,
    highlighted: MenuRow,
    error: Option<String>,
}

impl Pick {
    fn lines<S>(
        &self,
        cx: &mut Cx<'_, S>,
        strings: &'static Strings,
        look: &Look,
        num: &rtwk::l10n::NumSyms,
        width: usize,
        height: usize,
    ) -> Vec<Line> {
        let providers = self.offer.providers.any();
        let mut lines: Vec<Line> = if self.highlighted.is_alternative() {
            vec![vec![
                Span::new(strings.indent),
                Span::new(strings.heading_other),
            ]]
        } else if providers {
            vec![vec![
                Span::new(strings.indent),
                Span::new(strings.heading_providers),
            ]]
        } else {
            strings
                .heading_plain
                .iter()
                .map(|text| vec![Span::new(strings.indent), Span::new(*text)])
                .collect()
        };
        lines.push(Vec::new());
        let start = lines.len();
        let mut compact = Vec::new();
        for (index, choice) in options(&self.offer, self.highlighted)
            .into_iter()
            .enumerate()
        {
            let (label, description) = match choice {
                MenuRow::ChatGpt => (
                    strings.chatgpt,
                    if self.offer.chatgpt {
                        strings.chatgpt_description
                    } else {
                        strings.chatgpt_disabled
                    },
                ),
                MenuRow::DeviceCode => (strings.device_code, strings.device_code_description),
                MenuRow::Other => (
                    strings.other,
                    if providers {
                        strings.other_description_providers
                    } else {
                        strings.other_description_plain
                    },
                ),
                MenuRow::ApiKey => (strings.api_key, strings.api_key_description),
                MenuRow::Provider(Provider::Bedrock) => {
                    (strings.bedrock, strings.bedrock_description)
                }
                MenuRow::Provider(Provider::Gcp) => (strings.gcp, strings.gcp_description),
                MenuRow::Provider(Provider::Gateway) => {
                    (strings.gateway, strings.gateway_description)
                }
            };
            let on = choice == self.highlighted;
            let number = strings.option_number(index + 1, on, num);
            let (first, second) = if on {
                let cyan_dim = look.cyan.clone().merge(&look.dim);
                (
                    vec![
                        Span::styled(number, cyan_dim.clone()),
                        Span::styled(label, look.cyan.clone()),
                    ],
                    vec![Span::styled(
                        [strings.description_indent, description].concat(),
                        cyan_dim,
                    )],
                )
            } else {
                (
                    vec![Span::new([number.as_str(), label].concat())],
                    vec![Span::styled(
                        [strings.description_indent, description].concat(),
                        look.dim.clone(),
                    )],
                )
            };
            compact.push(first.clone());
            lines.push(first);
            lines.push(second);
            lines.push(Vec::new());
        }
        let end = lines.len();
        if !self.offer.api_key {
            lines.push(vec![Span::styled(
                strings.api_login_disabled_note,
                look.dim.clone(),
            )]);
            lines.push(Vec::new());
        }
        lines.push(hint(strings, look, strings.enter, strings.to_continue));
        if self.highlighted.is_alternative() {
            lines.push(hint(strings, look, strings.esc, strings.to_go_back));
        }
        if let Some(error) = &self.error {
            lines.push(Vec::new());
            lines.push(vec![Span::styled(error.clone(), look.red.clone())]);
        }
        // Keep the options and the hint in view in a short pane.
        if rows_of(cx, &lines, width) > height {
            lines.splice(start..end, compact);
        }
        lines
    }
}

/// `render_continue_in_browser`.
fn browser_lines(strings: &'static Strings, look: &Look, url: &str, copied: bool) -> Vec<Line> {
    let mut lines = vec![
        vec![
            Span::new(strings.indent),
            Span::new(strings.finish_in_browser),
        ],
        Vec::new(),
    ];
    if !url.is_empty() {
        let [before, key, after] = strings.copy_link;
        lines.push(vec![
            Span::new(before),
            Span::styled(key, look.key.clone()),
            Span::new(after),
        ]);
        if copied {
            lines.push(vec![Span::styled(
                [strings.indent, strings.copy_requested].concat(),
                look.dim.clone(),
            )]);
        }
        lines.push(Vec::new());
        lines.push(vec![
            Span::new(strings.indent),
            Span::styled(url.to_owned(), look.cyan.clone())
                .underline()
                .link(url.to_owned()),
        ]);
        lines.push(Vec::new());
        let [before, choose, device_code, end] = strings.headless;
        lines.push(vec![
            Span::new(before),
            Span::styled(strings.esc, look.key.clone()),
            Span::new(choose),
            Span::styled(device_code, look.cyan.clone()),
            Span::new(end),
        ]);
        lines.push(Vec::new());
    }
    lines.push(hint(strings, look, strings.esc, strings.to_cancel));
    lines
}

/// `render_device_code_login` (the banner without its shimmer: the
/// animations stop while the code is shown or asked).
fn device_code_lines(
    strings: &'static Strings,
    look: &Look,
    code: Option<&auth::DeviceCode>,
) -> Vec<Line> {
    let banner = if code.is_some() {
        strings.finish_in_browser
    } else {
        strings.preparing_device_code
    };
    let mut lines = vec![
        vec![Span::new(strings.indent), Span::new(banner)],
        Vec::new(),
    ];
    match code {
        Some(code) => {
            let [open, enter] = strings.device_code_steps;
            lines.push(vec![Span::new(open)]);
            lines.push(Vec::new());
            lines.push(vec![
                Span::new(strings.indent),
                Span::styled(code.url.clone(), look.cyan.clone())
                    .underline()
                    .link(code.url.clone()),
            ]);
            lines.push(Vec::new());
            lines.push(vec![Span::new(enter)]);
            lines.push(Vec::new());
            lines.push(vec![
                Span::new(strings.indent),
                Span::styled(code.code.clone(), look.cyan.clone()).bold(),
            ]);
            lines.push(Vec::new());
            lines.push(vec![Span::styled(
                strings.device_code_warning,
                look.dim.clone(),
            )]);
            lines.push(Vec::new());
        }
        None => {
            lines.push(vec![Span::styled(
                strings.requesting_code,
                look.dim.clone(),
            )]);
            lines.push(Vec::new());
        }
    }
    lines.push(hint(strings, look, strings.esc, strings.to_cancel));
    lines
}

/// `render_chatgpt_success_message`.
fn success_lines(strings: &'static Strings, look: &Look) -> Vec<Line> {
    vec![
        vec![Span::styled(strings.signed_in, look.green.clone())],
        Vec::new(),
        vec![Span::new(strings.in_control)],
        vec![
            Span::styled(strings.learn_permissions, look.dim.clone()),
            Span::styled(strings.codex_docs, look.dim.clone())
                .underline()
                .link(strings.codex_docs_url),
        ],
        Vec::new(),
        vec![Span::new(strings.mistakes)],
        vec![Span::styled(strings.review_code, look.dim.clone())],
        Vec::new(),
        vec![Span::new(strings.included)],
        vec![
            Span::styled(strings.plan_limits, look.dim.clone()),
            Span::styled(strings.training_preferences, look.dim.clone())
                .underline()
                .link(strings.training_preferences_url),
            Span::styled(strings.apply, look.dim.clone()),
        ],
        Vec::new(),
        vec![
            Span::styled(strings.press, look.cyan.clone()),
            Span::styled(strings.enter, look.key.clone()),
            Span::styled(strings.to_continue, look.cyan.clone()),
        ],
    ]
}

/// `render_api_key_entry`: the intro, the field in its box, the hints; the
/// intro and the hints share what the box leaves (`Min(4)`, `Length(3)`,
/// `Min(2)`).
fn api_key_entry(
    strings: &'static Strings,
    look: &Look,
    prefilled: bool,
    error: Option<String>,
    field: TextField,
) -> AnyWidget {
    let [mark, title] = strings.api_key_intro;
    let mut intro: Vec<Line> = vec![
        vec![Span::new(mark), Span::new(title).bold()],
        Vec::new(),
        vec![Span::new(strings.api_key_paste)],
        Vec::new(),
    ];
    if prefilled {
        intro.push(vec![Span::new(strings.env_detected)]);
        intro.push(vec![Span::styled(strings.env_other, look.dim.clone())]);
        intro.push(Vec::new());
    }
    let mut footer = vec![
        hint(strings, look, strings.enter, strings.to_save),
        hint(strings, look, strings.esc, strings.to_go_back),
    ];
    if let Some(error) = error {
        footer.push(Vec::new());
        footer.push(vec![Span::styled(error, look.red.clone())]);
    }
    let field_box = BoxWidget::new()
        .border(Border::all(look.cyan.clone()).kind(BorderType::Rounded))
        .title(BorderTitle::new(strings.api_key_title).style(Style::new()))
        .child(field);
    LayoutBuilder::new(move |_, constraints| {
        let height = usize::try_from(constraints.max_height).unwrap_or(0);
        let rest = height.saturating_sub(3);
        let intro_rows = rest.div_ceil(2).max(4).min(rest);
        let footer_rows = rest - intro_rows;
        let rows = |n: usize| Some(i32::try_from(n).unwrap_or(0));
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(SizedBox::new(
                None,
                rows(intro_rows),
                Some(paragraph(intro.clone())),
            ))
            .child(SizedBox::new(None, rows(3), Some(field_box.clone().into())))
            .child(SizedBox::new(
                None,
                rows(footer_rows),
                Some(paragraph(footer.clone())),
            ))
            .into()
    })
    .into()
}
