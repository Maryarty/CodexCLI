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
//! The menu is `Options::view_stacked` between the lines around it: its rows
//! are what the oracle's wrapping paragraph shows (a number and a label, the
//! description under it, the caret `>`), and `Options` keeps the highlighted
//! row (an index into the rows shown, a disabled row skipped by `step`), takes
//! the taps (a tap selects, a tap on the highlighted row or a double tap
//! accepts: `Msg::Accept`, ADR-011) and drops the descriptions and gaps when
//! the room the `Column` leaves it is short (`compact`), so every option and
//! the hint stay in view.

use std::rc::Rc;
use std::time::Duration;

use rtwk::foundation::ValueNotifier;

use rtwk::binding::{AppBinding, InputListener};
use rtwk::editing::{TextEditingController, TextField, copy_to_clipboard};
use rtwk::focus::{Focus, FocusNode, FocusOnKeyEventCallback, KeyEventResult};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    Alignment, Border, BorderTitle, BorderType, BoxWidget, Column, CrossAxisAlignment, FlexFit,
    Flexible, MainAxisSize, OverflowBox, SizedBox,
};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::read;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::input::{InputEvent, KeyKind};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;
use codex_tui_core::screens::onboarding::auth::{self, Method, Provider, SignIn};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{bind_fixed, bind_fixed_with, event_is_fixed, fixed_presses, pressed_fixed};
use crate::l10n::strings as shared;
use crate::widgets::key_guard::{GuardKey, KeyGuard};
use crate::widgets::measure;
use crate::widgets::options::{Options, Stacked, StackedRow};

pub mod provider;
pub mod strings;
use strings::Strings;

/// The sign-in step over the model's sign-in; it also stands in the
/// provider step's place while a provider's setup (S03) is chosen, so its
/// highlight outlives the provider step (Esc there comes back to it).
#[derive(Clone, Debug, PartialEq)]
pub struct SignInStep {
    pub sign_in: SignIn,
    pub resting: Resting,
}

/// Whether the gateway form's mark rests on Save: a signal from the wizard up to the page, which
/// fades its logo only while a field has the keys (the form's mark is the wizard's state,
/// ADR-013, so the core model cannot say). Flutter's `ValueNotifier` the page listens to.
#[derive(Clone)]
pub struct Resting(pub Rc<ValueNotifier<bool>>);

impl Default for Resting {
    fn default() -> Self {
        Resting(Rc::new(ValueNotifier::new(false)))
    }
}

impl Eq for Resting {}

impl PartialEq for Resting {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Resting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Resting").field(&self.0.value()).finish()
    }
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

/// The menu shown: the first one, or "Sign in another way"
/// (`SignInOption::is_alternative` of the highlighted row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShownMenu {
    First,
    Another,
}

/// A row shown, and whether it can be chosen: the ChatGPT row stays on the
/// menu, saying its login is disabled, when the policy forbids it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    choice: MenuRow,
    enabled: bool,
}

/// The rows of `menu` (`displayed_sign_in_options`): the one place that
/// decides which rows show and which can be chosen.
fn rows(offer: &auth::Offer, menu: ShownMenu) -> Vec<Row> {
    let enabled = |choice| Row {
        choice,
        enabled: true,
    };
    if menu == ShownMenu::Another {
        let mut rows = vec![enabled(MenuRow::ApiKey)];
        for provider in [Provider::Bedrock, Provider::Gcp, Provider::Gateway] {
            if offer.providers.offers(provider) {
                rows.push(enabled(MenuRow::Provider(provider)));
            }
        }
        return rows;
    }
    let mut rows = vec![Row {
        choice: MenuRow::ChatGpt,
        enabled: offer.chatgpt,
    }];
    if offer.chatgpt {
        rows.push(enabled(MenuRow::DeviceCode));
    }
    if offer.api_key {
        rows.push(enabled(MenuRow::Other));
    }
    rows
}

/// The rows the highlight moves over (`move_highlight`): their places in the
/// rows shown.
fn reachable(rows: &[Row]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| row.enabled)
        .map(|(at, _)| at)
        .collect()
}

fn first_reachable(rows: &[Row]) -> usize {
    reachable(rows).first().copied().unwrap_or(0)
}

/// The oracle's `.is_alternative()`: a row of "Sign in another way".
fn is_alternative(row: MenuRow) -> bool {
    matches!(row, MenuRow::ApiKey | MenuRow::Provider(_))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Up,
    Down,
    /// A digit (or `y`/`n`): the option at the index, pressed as `key`.
    Index(usize, char),
    Confirm,
    /// A tap on the highlighted row, or a double tap: Enter at that row.
    Accept(usize),
    Cancel,
    CopyLink,
    /// Enter or a tap on the failed gateway sign-in's retry line: asks again.
    Retry,
    Submitted(Rc<str>),
    /// The key entry is mounted: its field takes the focus.
    FocusField,
    /// A quit key: the app ends, a login started cancelled.
    Quit,
}

/// Widget state: the highlighted option, the API key's field and the
/// activation guard (R14).
pub struct SignInState {
    menu: ShownMenu,
    /// The highlighted row of the menu: an index into the rows shown.
    cursor: Options,
    field: TextEditingController,
    field_node: FocusNode,
    node: FocusNode,
    guard: KeyGuard,
    /// "Copy requested; check your clipboard" under the link, until the
    /// browser login ends.
    copied: bool,
}

impl StatefulWidget for SignInStep {
    type State = SignInState;

    fn create_state(&self) -> SignInState {
        let cursor = Options::new();
        // ChatGPT is the first row, or the other-ways row when its login is
        // not allowed (the row stays shown, not reachable).
        cursor.set(first_reachable(&rows(
            &self.sign_in.offer,
            ShownMenu::First,
        )));
        SignInState {
            menu: ShownMenu::First,
            cursor,
            field: TextEditingController::default(),
            field_node: FocusNode::new(),
            node: FocusNode::new(),
            guard: KeyGuard::default(),
            copied: false,
        }
    }
}

impl_widget!(stateful eq SignInStep);

/// The time on the app's clock (a test's fake clock moves it).
fn now<S: State>(cx: &Cx<'_, S>) -> Duration {
    AppBinding::maybe_of(cx).map_or(Duration::ZERO, |binding| binding.clock().now())
}

/// What every key press meets first, on a node of the step or of a field in
/// it: the activation guard (a held Enter acts once), then the quit keys
/// (`fixed::PICK_QUIT`: Ctrl+C and Ctrl+D, and a `q` unless `typing` says it is
/// text). The provider wizards' fields use it too.
pub(crate) fn guarded_keys(
    guard: KeyGuard,
    now: Rc<dyn Fn() -> Duration>,
    typing: Rc<dyn Fn() -> bool>,
    quit: Rc<dyn Fn()>,
) -> FocusOnKeyEventCallback {
    Rc::new(move |_, key| {
        if guard.press(GuardKey::of_press(&key), now()) {
            return KeyEventResult::Handled;
        }
        let chord = pressed_fixed(fixed::PICK_QUIT, &key);
        let q = pressed_fixed(fixed::PICK_LEAVE, &key);
        if chord || (q && !typing()) {
            quit();
            return KeyEventResult::Handled;
        }
        KeyEventResult::Ignored
    })
}

impl SignInState {
    fn emit(cx: &mut Cx<'_, Self>, intent: auth::Intent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::SignIn(intent)));
    }

    /// The gateway's sign-in failed and is asked again
    /// (`RetryGatewayOAuth`: the model starts it over).
    fn retry(cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Onboarding(Intent::RetryGatewayOAuth),
        );
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

    /// The row the highlight is on.
    fn highlighted(&self, w: &SignInStep) -> Option<MenuRow> {
        rows(&w.sign_in.offer, self.menu)
            .get(self.cursor.cursor())
            .filter(|row| row.enabled)
            .map(|row| row.choice)
    }

    /// The highlight goes to `choice` (a digit chose it).
    fn highlight(&self, choice: MenuRow, w: &SignInStep) {
        if let Some(at) = rows(&w.sign_in.offer, self.menu)
            .iter()
            .position(|row| row.choice == choice)
        {
            self.cursor.set(at);
        }
    }

    /// `activate_highlighted_mode` and `handle_sign_in_option`; `key` is the
    /// key that chose (a click has none: nothing is held to guard).
    fn activate(
        &mut self,
        choice: MenuRow,
        key: Option<GuardKey>,
        w: &SignInStep,
        cx: &mut Cx<'_, Self>,
    ) {
        self.highlight(choice, w);
        if let Some(key) = key
            && (choice == MenuRow::Other || is_alternative(choice))
        {
            self.guard.arm(key, now(cx));
        }
        match choice {
            MenuRow::Other => {
                self.menu = ShownMenu::Another;
                self.cursor.set(0);
                Self::emit(cx, auth::Intent::Menu);
            }
            MenuRow::ChatGpt => Self::emit(cx, auth::Intent::UseMethod(Method::ChatGpt)),
            MenuRow::DeviceCode => Self::emit(cx, auth::Intent::UseMethod(Method::DeviceCode)),
            MenuRow::ApiKey => Self::emit(cx, auth::Intent::UseMethod(Method::ApiKey)),
            MenuRow::Provider(provider) => {
                Self::emit(cx, auth::Intent::UseMethod(Method::Provider(provider)));
            }
        }
    }

    /// `move_highlight`: round the ends, past ChatGPT when its login is not
    /// allowed (the rows `reachable` names).
    fn step(&self, delta: i64, w: &SignInStep) {
        let reach = reachable(&rows(&w.sign_in.offer, self.menu));
        let Ok(len) = i64::try_from(reach.len()) else {
            return;
        };
        if len == 0 {
            return;
        }
        let from = reach
            .iter()
            .position(|at| *at == self.cursor.cursor())
            .and_then(|at| i64::try_from(at).ok())
            .unwrap_or(0);
        if let Some(to) = usize::try_from((from + delta).rem_euclid(len))
            .ok()
            .and_then(|at| reach.get(at))
        {
            self.cursor.set(*to);
        }
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
        let now: Rc<dyn Fn() -> Duration> =
            Rc::new(move || clock.as_ref().map_or(Duration::ZERO, |clock| clock.now()));
        let send = cx.local_sender();
        let on_key = |typing: Option<TextEditingController>| -> FocusOnKeyEventCallback {
            // `suppress_quit_while_typing`: a `q` is text once the key
            // entry holds some.
            let typed: Rc<dyn Fn() -> bool> = Rc::new(move || {
                typing
                    .as_ref()
                    .is_some_and(|field| !field.text().is_empty())
            });
            let send = send.clone();
            guarded_keys(
                self.guard.clone(),
                now.clone(),
                typed,
                Rc::new(move || send(Msg::Quit)),
            )
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
        self.cursor.dispose();
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
                if let Some(row) = rows(&w.sign_in.offer, self.menu).get(index).copied()
                    && row.enabled
                {
                    self.activate(row.choice, Some(GuardKey::Char(key)), w, cx);
                }
            }
            Msg::Accept(index) => {
                if *state == auth::State::Pick
                    && let Some(row) = rows(&w.sign_in.offer, self.menu).get(index).copied()
                    && row.enabled
                {
                    self.activate(row.choice, None, w, cx);
                }
            }
            Msg::Confirm => match state {
                auth::State::Pick => {
                    if let Some(choice) = self.highlighted(w) {
                        self.activate(choice, Some(GuardKey::Enter), w, cx);
                    }
                }
                auth::State::SuccessMessage => Self::emit(cx, auth::Intent::Continue),
                auth::State::GatewayOAuthFailed { .. } => Self::retry(cx),
                _ => {}
            },
            Msg::Retry => {
                if matches!(state, auth::State::GatewayOAuthFailed { .. }) {
                    Self::retry(cx);
                }
            }
            Msg::Submitted(text) => {
                Self::emit(cx, auth::Intent::SaveApiKey(text.to_string()));
            }
            Msg::Cancel => {
                if *state == auth::State::Pick && self.menu == ShownMenu::Another {
                    self.menu = ShownMenu::First;
                    self.highlight(MenuRow::Other, w);
                    Self::emit(cx, auth::Intent::Menu);
                } else if matches!(state, auth::State::Provider(_)) {
                    Self::emit(cx, auth::Intent::ProviderBack);
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
        let look = Look::of(&tokens);
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
            auth::State::Pick => pick(
                &self.cursor.on_accept(cx.map(Msg::Accept)),
                &w.sign_in.offer,
                self.menu,
                error,
                strings,
                &look,
                &num,
            ),
            auth::State::Browser { url, .. } => {
                focus = bind_fixed(focus, fixed::COPY_LINK, cx.send(Msg::CopyLink));
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
            // The provider's wizard (S03); one whose wizard is not built
            // has an empty place, and Esc comes back to the menu.
            auth::State::Provider(_) => match &w.sign_in.wizard {
                Some(wizard) => provider::body(wizard, &self.guard, &w.resting),
                None => SizedBox::shrink().into(),
            },
            auth::State::ProviderConfigured(chosen) => {
                provider::configured(*chosen, &look, provider::strings::of(cx))
            }
            // The gateway is the default provider (`GatewaySelected`) and
            // signed in to (`GatewayConfigured`): the same line.
            auth::State::GatewaySelected(_) | auth::State::GatewayConfigured => {
                provider::configured(Provider::Gateway, &look, provider::strings::of(cx))
            }
            auth::State::GatewayOAuth { url, .. } => {
                provider::gateway_login::browser(provider::strings::of(cx), &look, url.as_deref())
            }
            auth::State::GatewayOAuthFailed { message, .. } => provider::gateway_login::failed(
                provider::strings::of(cx),
                &look,
                message,
                cx.map(|_| Msg::Retry),
            ),
        };
        focus.child = body;
        for (id, msg) in [
            (fixed::PICK_PREVIOUS, Msg::Up),
            (fixed::PICK_NEXT, Msg::Down),
            (fixed::ENTER, Msg::Confirm),
            (fixed::DISMISS, Msg::Cancel),
        ] {
            focus = bind_fixed(focus, id, cx.send(msg));
        }
        // The number keys and the `y`/`n` beside `1` and `2`: each key names its row and the
        // character it typed.
        for (id, row) in [(fixed::PICK_YES, 0), (fixed::PICK_NO, 1)] {
            focus = bind_fixed_with(focus, id, |_, key| {
                cx.send(Msg::Index(row, key.ch.unwrap_or('1')))
            });
        }
        // Rows three and four have no other key.
        for (row, key) in fixed_presses(fixed::PICK_NUMBERS)
            .into_iter()
            .enumerate()
            .take(4)
            .skip(2)
        {
            focus = focus.bind(key.clone(), cx.send(Msg::Index(row, key.ch.unwrap_or('1'))));
        }
        // The repeats and releases the focus path cannot tell from presses:
        // a held key moves the highlight and deletes, nothing else
        // (`KeyEventKind::Press` checks of `handle_key_event`); a release
        // ends the guard of its key.
        let guard = self.guard.clone();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let pick = w.sign_in.state == auth::State::Pick;
        let entry = matches!(w.sign_in.state, auth::State::ApiKey { .. });
        InputListener::with_raw_key_repeats(focus, move |event| {
            let InputEvent::Key(key) = event else {
                return false;
            };
            let guard_key = GuardKey::of_event(key);
            match key.kind {
                KeyKind::Release => {
                    guard.release(guard_key);
                    false
                }
                KeyKind::Repeat => {
                    let now = clock.as_ref().map_or(Duration::ZERO, |clock| clock.now());
                    guard.held(guard_key, now);
                    let is = |id| event_is_fixed(id, key);
                    let moves = is(fixed::CONFIRM_PREVIOUS)
                        || is(fixed::CONFIRM_NEXT)
                        || (pick && matches!(guard_key, Some(GuardKey::Char('j' | 'k'))));
                    let edits = entry && (is(fixed::FIELD_DELETE_BACKWARD) || is(fixed::DISMISS));
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
pub(crate) struct Look {
    pub(crate) dim: Style,
    /// A key's name in a hint (`key_hint_style`: bold, not dim).
    pub(crate) key: Style,
    pub(crate) cyan: Style,
    pub(crate) green: Style,
    pub(crate) red: Style,
}

impl Look {
    /// The step's styles from the theme.
    pub(crate) fn of(tokens: &Tokens) -> Look {
        Look {
            // The oracle's `.dim()`: the attribute on the terminal's colour
            // (the theme's `dim` is a grey of its own).
            dim: Style::new().attrs(Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        }
    }
}

pub(crate) type Line = Vec<Span>;

pub(crate) fn width_of(max_width: i32) -> usize {
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
pub(crate) fn rows_of<S>(cx: &mut Cx<'_, S>, lines: &[Line], width: usize) -> usize {
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

/// The lines as a column of their own height: each wraps where it stands, a
/// blank line is a row.
fn column_of(lines: &[Line]) -> Column {
    column_tapping(lines, None)
}

/// The same, one line of it (by index) a tap target.
fn column_tapping(lines: &[Line], tap: Option<(usize, Handler<PointerEvent>)>) -> Column {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for (index, line) in lines.iter().cloned().enumerate() {
        let row: AnyWidget = if line.is_empty() {
            SizedBox::new(None, Some(1), None).into()
        } else if one_word(&text_of(&line)) {
            Text::rich(line).wrap(TextWrap::Grapheme).into()
        } else {
            Text::rich(line).into()
        };
        column = match &tap {
            Some((at, on_tap)) if *at == index => column.child(
                GestureDetector::new()
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(on_tap.clone())
                    .child(row),
            ),
            _ => column.child(row),
        };
    }
    column
}

/// The lines as a paragraph that wraps them and is cut off at the bottom of
/// its room (`Paragraph::wrap(Wrap { trim: false })`).
pub(crate) fn paragraph(lines: Vec<Line>) -> AnyWidget {
    tapping(lines, None)
}

/// The same, one line of it (by index) a tap target.
pub(crate) fn tapping(lines: Vec<Line>, tap: Option<(usize, Handler<PointerEvent>)>) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let rows = rows_of(cx, &lines, width_of(constraints.max_width));
        OverflowBox::new(column_tapping(&lines, tap.clone()))
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

/// The menu (`render_pick_mode`): the heading, the rows (`Options`, which
/// takes the room the lines around it leave), the note, the hints and the
/// error.
fn pick(
    options: &Options,
    offer: &auth::Offer,
    menu: ShownMenu,
    error: Option<String>,
    strings: &'static Strings,
    look: &Look,
    num: &rtwk::l10n::NumSyms,
) -> AnyWidget {
    let providers = offer.providers.any();
    let mut head: Vec<Line> = if menu == ShownMenu::Another {
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
    head.push(Vec::new());
    let menu_rows: Vec<StackedRow> = rows(offer, menu)
        .into_iter()
        .enumerate()
        .map(|(index, Row { choice, enabled })| {
            let (label, description) = match choice {
                MenuRow::ChatGpt => (
                    strings.chatgpt,
                    if offer.chatgpt {
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
            StackedRow {
                lead_on: strings.option_number(index + 1, true, num),
                lead_off: strings.option_number(index + 1, false, num),
                label: label.to_owned(),
                description: description.to_owned(),
                enabled,
            }
        })
        .collect();
    let cyan_dim = look.cyan.clone().merge(&look.dim);
    let stacked = Stacked {
        indent: strings.description_indent,
        cursor_lead: cyan_dim.clone(),
        cursor_label: look.cyan.clone(),
        cursor_description: cyan_dim,
        label: Style::new(),
        description: look.dim.clone(),
    };
    let mut tail: Vec<Line> = Vec::new();
    if !offer.api_key {
        tail.push(vec![Span::styled(
            strings.api_login_disabled_note,
            look.dim.clone(),
        )]);
        tail.push(Vec::new());
    }
    tail.push(hint(strings, look, strings.enter, strings.to_continue));
    if menu == ShownMenu::Another {
        tail.push(hint(strings, look, strings.esc, strings.to_go_back));
    }
    if let Some(error) = error {
        tail.push(Vec::new());
        tail.push(vec![Span::styled(error, look.red.clone())]);
    }
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(column_of(&head))
        .child(Flexible {
            fit: FlexFit::Loose,
            ..Flexible::new(
                1,
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(options.view_stacked(Rc::new(menu_rows), stacked)),
            )
        })
        .child(column_of(&tail))
        .into()
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
    device_lines(strings, look, banner, strings.device_code_steps, code)
}

/// `render_device_code_prompt`: the heading, the link and the code under
/// their steps (or the request under way), the warning and how to cancel;
/// the ChatGPT login and the gateway's device code draw the same.
pub(crate) fn device_lines(
    strings: &'static Strings,
    look: &Look,
    heading: &'static str,
    steps: [&'static str; 2],
    code: Option<&auth::DeviceCode>,
) -> Vec<Line> {
    let mut lines = vec![
        vec![Span::new(strings.indent), Span::new(heading)],
        Vec::new(),
    ];
    match code {
        Some(code) => {
            let [open, enter] = steps;
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
