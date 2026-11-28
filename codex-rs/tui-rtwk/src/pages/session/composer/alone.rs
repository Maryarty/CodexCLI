//! The composer alone with its footer (`composer/alone`): the real composer
//! and the real key footer at the screen's size and nothing else, as the
//! oracle's composer harness draws them (`snapshot_composer_state_with_width`:
//! a `ChatComposer` rendered over `footer_height + 8` rows). The page's
//! bottom pane is the same two widgets under a transcript and a status row.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::focus::{FocusNode, KeyEventResult};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    MainAxisSize, UNBOUNDED,
};
use rtwk::provider::read;
use rtwk::style::Span;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::{Keymap, fixed};
use codex_tui_core::threads::ThreadModel;

use crate::app::theme;
use crate::keymap::press::is_escape;
use crate::keymap::{pressed_fixed, types_text};
use crate::pages::session::footer::{Flash, Footer, Line, StatusRows, Timed};
use crate::pages::session::popovers::mention;
use crate::sheets::dialogs::{Panel, shortcut_panel_fixed};
use crate::widgets::thread_ui::ThreadUi;

use codex_tui_core::kit::vim::VimMode;

use super::{Composer, clear_sent, search, vim};

/// The footer's row (`FooterMode::Passive`), kept clear of the popup.
const FOOTER_ROWS: i32 = 1;
/// The rows the oracle's composer layout keeps for the draft (`Min(3)`).
const DRAFT_ROWS: i32 = 3;
/// How long the quit hint stays (`QUIT_SHORTCUT_TIMEOUT`).
const QUIT_HINT: Duration = Duration::from_secs(1);

/// The composer of `ui` over its footer, filling the screen.
#[derive(Clone)]
pub struct Alone {
    pub ui: Rc<ThreadUi>,
    /// `set_footer_hint_override`: `(key, label)` items in place of the hints.
    hint_override: Option<Vec<(String, String)>>,
    /// `show_footer_flash`: a line over the hints for a while.
    flash: Option<Flash>,
    /// `enable_agents_navigation`: the empty prompt's `←` is the agents page's key.
    agents: bool,
    /// `set_status_line`: the text of the status line over the hints.
    status: Option<Rc<str>>,
    /// `set_esc_backtrack_hint(true)`: the first Esc has primed the backtrack.
    esc_hint: bool,
    /// `show_quit_shortcut_hint`: `⌃c again to quit` until its time is up.
    quit_hint: bool,
    /// `set_queue_submissions(true)`: the send key queues.
    queue: bool,
    /// The app primes the Esc hint once the shortcut overlay is open
    /// (`set_esc_backtrack_hint(true)` right after `?`).
    esc_after_overlay: bool,
    /// `enhanced_keys_supported`: the overlay names Shift+Enter for the new line.
    enhanced: bool,
}

impl PartialEq for Alone {
    fn eq(&self, other: &Alone) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
            && self.hint_override == other.hint_override
            && self.flash == other.flash
            && self.agents == other.agents
            && self.status == other.status
            && self.esc_hint == other.esc_hint
            && self.quit_hint == other.quit_hint
            && self.queue == other.queue
            && self.esc_after_overlay == other.esc_after_overlay
            && self.enhanced == other.enhanced
    }
}

impl Alone {
    pub fn new(ui: Rc<ThreadUi>) -> Alone {
        Alone {
            ui,
            hint_override: None,
            flash: None,
            agents: false,
            status: None,
            esc_hint: false,
            quit_hint: false,
            queue: false,
            esc_after_overlay: false,
            enhanced: false,
        }
    }

    /// The terminal tells Shift+Enter from Enter (`enhanced_keys_supported`).
    pub fn enhanced_keys(mut self) -> Alone {
        self.enhanced = true;
        self
    }

    /// The backtrack is primed: the footer asks for `esc again` (`set_esc_backtrack_hint`).
    pub fn esc_hint(mut self) -> Alone {
        self.esc_hint = true;
        self
    }

    /// The footer shows `⌃c again to quit` for a second (`show_quit_shortcut_hint`).
    pub fn quit_hint(mut self) -> Alone {
        self.quit_hint = true;
        self
    }

    /// The send key queues the message (`set_queue_submissions`).
    pub fn queue_submissions(mut self) -> Alone {
        self.queue = true;
        self
    }

    /// The app primes the Esc hint right after the shortcut overlay opens.
    pub fn esc_hint_after_overlay(mut self) -> Alone {
        self.esc_after_overlay = true;
        self
    }

    /// The agents page is reachable from the empty prompt: the hints name its key.
    pub fn agents(mut self) -> Alone {
        self.agents = true;
        self
    }

    /// The status line shows `text` over the hints.
    pub fn status_line(mut self, text: &str) -> Alone {
        self.status = Some(Rc::from(text));
        self
    }

    /// The footer shows `items` instead of its hints.
    pub fn hint_override(mut self, items: Vec<(String, String)>) -> Alone {
        self.hint_override = Some(items);
        self
    }

    /// The footer flashes `flash` over everything else until it runs out.
    pub fn flash(mut self, flash: Flash) -> Alone {
        self.flash = Some(flash);
        self
    }
}

impl_widget!(stateful eq Alone);

/// What the footer reads of the draft, kept by the state as the page keeps it.
pub struct AloneState {
    node: FocusNode,
    /// The keymap the build last saw, for the key handler.
    keymap: Rc<RefCell<Rc<Keymap>>>,
    draft_empty: bool,
    draft_shell: bool,
    /// What the footer is in (`FooterMode`).
    mode: Mode,
    /// `esc_backtrack_hint`: the Esc line (and the overlay's row) asks for one more press.
    esc_hint: bool,
    /// What the key handler needs to know of the build.
    mirror: Rc<Mirror>,
    /// The end of the quit hint.
    quit_timer: Option<Timer>,
}

/// The footer's mode (`FooterMode`, for the ones a key or the app sets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// `ComposerEmpty` / `ComposerHasDraft`: the passive hints.
    Passive,
    /// `ShortcutOverlay`: the shortcut overlay is open above the composer.
    Overlay,
    /// `QuitShortcutReminder`.
    Quit,
    /// `EscHint`.
    Esc,
}

/// What the build last saw, for the key handler.
struct Mirror {
    mode: Cell<Mode>,
    running: Cell<bool>,
}

/// What the composer's footer is told of.
#[derive(Clone, PartialEq)]
pub enum Msg {
    /// The draft changed (the field's own notification).
    Changed,
    /// `?` on an empty draft.
    Shortcuts,
    /// Enter: the oracle's harness composer clears with what it submitted.
    Submitted,
    /// Esc on an empty draft: the footer says what a second one does.
    Esc,
    /// Any other key: the transient lines and the overlay go (`reset_mode_after_activity`).
    Activity,
    /// The quit hint ran out.
    QuitExpired,
}

impl StatefulWidget for Alone {
    type State = AloneState;

    fn create_state(&self) -> AloneState {
        AloneState {
            node: FocusNode::new(),
            keymap: Rc::new(RefCell::new(Rc::new(Keymap::defaults()))),
            draft_empty: self.ui.composer.text().is_empty(),
            draft_shell: self.ui.draft.shell(),
            mode: if self.quit_hint {
                Mode::Quit
            } else if self.esc_hint {
                Mode::Esc
            } else {
                Mode::Passive
            },
            esc_hint: self.esc_hint,
            mirror: Rc::new(Mirror {
                mode: Cell::new(Mode::Passive),
                running: Cell::new(false),
            }),
            quit_timer: None,
        }
    }
}

impl State for AloneState {
    type Widget = Alone;
    type Msg = Msg;

    fn init_state(&mut self, w: &Alone, cx: &mut Cx<'_, Self>) {
        // `?` on an empty composer opens the shortcut overlay (the page's key,
        // as in `Chat`); with text, a paste or a modifier it goes to the field.
        // Esc on an empty draft says what a second one does, and any other key puts the
        // transient lines away (`handle_key_event_without_popup`).
        let send = cx.local_sender();
        let ui = w.ui.clone();
        let keymap = self.keymap.clone();
        let mirror = self.mirror.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            let empty = ui.composer.text().is_empty() && !ui.draft.in_burst();
            let mode = mirror.mode.get();
            if is_escape(&key) {
                // Esc closes the overlay first (`handle_empty_prompt_shortcut`)
                if mode == Mode::Overlay {
                    send(Msg::Activity);
                    return KeyEventResult::Handled;
                }
                // Vim's Insert and Replace take their own Esc; shell mode's Esc leaves the mode
                let vim_escape = ui
                    .draft
                    .with_vim(|vim| vim.is_enabled() && vim.mode() != VimMode::Normal);
                if !empty || ui.draft.shell() || vim_escape {
                    return KeyEventResult::Ignored;
                }
                // the composer takes the Esc whether or not the hint changes (`esc_hint_mode`:
                // a running turn and the line already shown leave the footer as it is)
                if !mirror.running.get() && mode != Mode::Esc {
                    send(Msg::Esc);
                }
                return KeyEventResult::Handled;
            }
            let opens = empty
                && types_text(&key)
                // Normal mode's search keys come first: `?` searches backward there
                && !vim::wants_search_key(&ui.draft, &keymap.borrow(), &key)
                && pressed_fixed(fixed::OPEN_SHORTCUTS, &key);
            if opens {
                send(Msg::Shortcuts);
                return KeyEventResult::Handled;
            }
            if mode != Mode::Passive {
                send(Msg::Activity);
            }
            KeyEventResult::Ignored
        })));
        if w.quit_hint
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            self.quit_timer = Some(binding.start_timer(QUIT_HINT, cx.map(|()| Msg::QuitExpired)));
        }
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        if let Some(timer) = self.quit_timer.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, msg: Msg, w: &Alone, _: &mut Cx<'_, Self>) -> Rebuild {
        self.draft_shell = w.ui.draft.shell();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
        match msg {
            // `toggle_shortcut_mode`: the quit reminder keeps the footer while it shows
            Msg::Shortcuts => match self.mode {
                Mode::Quit => {}
                Mode::Overlay => self.mode = Mode::Passive,
                Mode::Passive | Mode::Esc => {
                    self.mode = Mode::Overlay;
                    if w.esc_after_overlay {
                        // `set_esc_backtrack_hint(true)`: the overlay gives way to the Esc line
                        self.esc_hint = true;
                        self.mode = Mode::Esc;
                    }
                }
            },
            // any other edit puts the overlay and the transient lines away
            Msg::Changed if !self.draft_empty => self.mode = Mode::Passive,
            Msg::Changed => {}
            Msg::Esc => self.mode = Mode::Esc,
            Msg::Activity => self.mode = Mode::Passive,
            Msg::QuitExpired => {
                self.quit_timer = None;
                if self.mode == Mode::Quit {
                    self.mode = Mode::Passive;
                }
            }
            Msg::Submitted => {
                clear_sent(&w.ui);
                self.draft_shell = false;
                self.draft_empty = true;
                self.mode = Mode::Passive;
            }
        }
        self.mirror.mode.set(self.mode);
        Rebuild::Yes
    }

    fn build(&self, w: &Alone, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        *self.keymap.borrow_mut() = crate::keymap::keymap(cx);
        let running = read::<ThreadModel, _>(cx).is_running();
        self.mirror.mode.set(self.mode);
        self.mirror.running.set(running);
        let composer = Composer::new(w.ui.clone(), self.node.clone())
            .with_history_field(w.ui.draft.searching())
            .on_changed(cx.map(|_| Msg::Changed))
            .on_submitted(cx.map(|_| Msg::Submitted));
        let footer: AnyWidget = if let Some((query, status)) = w.ui.draft.search_state() {
            search::footer(
                &w.ui,
                self.node.clone(),
                cx.map(|_| Msg::Changed),
                query,
                status,
            )
        } else if let Some(row) = search::query_footer(cx, &w.ui.draft) {
            row
        } else {
            let footer = Footer {
                line: match self.mode {
                    Mode::Passive => Line::Passive,
                    Mode::Overlay => Line::Close,
                    Mode::Quit => Line::Quit,
                    Mode::Esc if self.esc_hint => Line::EscPrimed,
                    Mode::Esc => Line::EscIdle,
                },
                agents: w.agents,
                draft_empty: self.draft_empty,
                shell: self.draft_shell,
                vim: w
                    .ui
                    .draft
                    .with_vim(|vim| vim.is_enabled().then(|| vim.mode())),
                hint_override: w.hint_override.clone(),
                flash: w.flash.clone(),
                on_queue: None,
                on_warnings: None,
                on_close: Some(cx.send(Msg::Shortcuts)),
            };
            let hints: AnyWidget = Timed {
                footer: footer.clone(),
            }
            .into();
            match &w.status {
                // the status line stands over the hints (`status_surface`)
                Some(text) => Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .main_axis_size(MainAxisSize::Min)
                    .child(StatusRows {
                        spans: vec![Span::styled(text.to_string(), tokens.dim())],
                        composer: w.ui.composer.clone(),
                        hints: Some(footer),
                        footer: None,
                    })
                    .child(hints)
                    .into(),
                None => hints,
            }
        };
        let panel = Panel {
            running,
            agents: w.agents,
            queue: w.queue,
            esc_primed: self.esc_hint,
            enhanced: w.enhanced,
        };
        let shortcuts = (self.mode == Mode::Overlay).then(|| shortcut_panel_fixed(cx, panel));
        let surface = tokens.prompt_surface(cx, false);
        let ui = w.ui.clone();
        // The `@`/`$` popup stands above the draft and takes the rows the
        // draft's three rows (`Min(3)`) and the footer leave, up to its own
        // height; the draft takes the rest of the screen.
        LayoutBuilder::new(move |_, constraints| {
            let room = constraints.max_height - FOOTER_ROWS - DRAFT_ROWS;
            let mut body = Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .main_axis_size(MainAxisSize::Max);
            if let Some(panel) = shortcuts.clone() {
                // the overlay takes its rows and the composer the rest
                body = body.child(ConstrainedBox::new(
                    BoxConstraints::new(0, UNBOUNDED, 0, room.max(0)),
                    panel,
                ));
            }
            body.child(mention::capped(ui.clone(), room))
                .child(Expanded::new(
                    1,
                    BoxWidget::new()
                        .style(surface.clone())
                        .padding(EdgeInsets::only(0, 1, 1, 1))
                        .child(composer.clone()),
                ))
                .child(footer.clone())
                .into()
        })
        .into()
    }
}
