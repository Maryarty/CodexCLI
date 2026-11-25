//! The plain-text composer (`ChatComposerConfig::plain_text`): the composer
//! the answers inside question sheets are written in. In Flutter it is the
//! same `TextField` with a `plain` configuration: the controller and the
//! draft are the chat composer's, and the popups (slash commands, `@` and `$`
//! mentions), shell mode, image paths, the queue and the shortcut overlay are
//! not built, so `!`, `/` and `?` stay literal text. A `> ` paste is quoted
//! and the answer starts the next block (`blockquote_paste_enabled`), a large
//! paste is a placeholder, and an over-long message is refused whole with the
//! draft kept (`prepare_submission_text`).
//!
//! What makes it plain lives where the decision is: the draft cannot enter
//! shell mode or attach an image (`ComposerDraft::plain`), the composer
//! builds no popup and asks the app for no connection state
//! (`Composer::plain`), and Tab submits as the queue key does when nothing
//! runs and no shell command waits (`editing::composer_key`).
//!
//! Oracle: `bottom_pane/chat_composer.rs` (`ChatComposerConfig`),
//! `chat_composer/embedded_input_tests.rs`.

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::editing::{KeyCx, TakeKey, TextEditingController};
use rtwk::focus::{FocusNode, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::composer::MAX_INPUT_CHARS;

use crate::app::theme::{self, Tokens};
use crate::pages::session::actions::prompt;
use crate::pages::session::footer::{Footer, Line};
use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::thread_ui::ThreadUi;

use super::{Composer, strings};

/// How long the footer's flash stays (`show_footer_flash`).
const FLASH_FOR: Duration = Duration::from_secs(5);

/// What the owner of a plain-text composer sets: the placeholder it shows on
/// an empty draft (`set_placeholder_text`; the chat's when none), the style of
/// its prompt (bold when none; the sheets dim it with the rest of their
/// composer) and the keys it reads before the composer does (the oracle's
/// overlay reads its own keys, Tab among them, before the composer's).
#[derive(Clone)]
pub struct Look {
    /// The field takes the focus when it is built (on by default; a sheet whose
    /// options hold the keys leaves it off).
    pub autofocus: bool,
    pub placeholder: Option<&'static str>,
    pub prompt: Option<Style>,
    /// The field draws no `›` before its text (the inline editor of the async questions: the
    /// answer starts where the question's text does).
    pub unprompted: bool,
    pub keys: Option<TakeKey>,
    /// The most rows the field grows to (the chat's six when none).
    pub max_lines: Option<usize>,
}

impl Default for Look {
    fn default() -> Look {
        Look {
            autofocus: true,
            placeholder: None,
            prompt: None,
            unprompted: false,
            keys: None,
            max_lines: None,
        }
    }
}

impl PartialEq for Look {
    fn eq(&self, other: &Look) -> bool {
        self.autofocus == other.autofocus
            && self.placeholder == other.placeholder
            && self.prompt == other.prompt
            && self.unprompted == other.unprompted
            && self.max_lines == other.max_lines
            && match (&self.keys, &other.keys) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// The controllers of one plain-text composer: its own text and its own
/// plain draft, nothing of a thread's.
pub fn ui() -> Rc<ThreadUi> {
    Rc::new(ThreadUi {
        draft: ComposerDraft::plain(),
        ..ThreadUi::default()
    })
}

impl Composer {
    /// The plain-text composer of `ui` (made by `ui()`: its draft is a plain
    /// one), focused through `node`.
    pub fn plain(ui: Rc<ThreadUi>, node: FocusNode) -> Composer {
        debug_assert!(
            ui.draft.is_plain(),
            "a plain-text composer needs the controllers of `plain::ui()`"
        );
        Composer {
            plain: Some(Look::default()),
            ..Composer::new(ui, node)
        }
    }

    /// The text shown while the draft is empty (a plain-text composer).
    #[must_use]
    pub fn placeholder(mut self, text: &'static str) -> Composer {
        if let Some(look) = &mut self.plain {
            look.placeholder = Some(text);
        }
        self
    }

    /// The style of the prompt `›` (a plain-text composer).
    #[must_use]
    pub fn prompt_style(mut self, style: Style) -> Composer {
        if let Some(look) = &mut self.plain {
            look.prompt = Some(style);
        }
        self
    }

    /// The field draws no prompt before its text (a plain-text composer).
    #[must_use]
    pub fn unprompted(mut self) -> Composer {
        if let Some(look) = &mut self.plain {
            look.unprompted = true;
        }
        self
    }

    /// The most rows the field grows to (a plain-text composer): the text prompt's eight, or all
    /// the room its owner gives it.
    #[must_use]
    pub fn max_lines(mut self, rows: usize) -> Composer {
        if let Some(look) = &mut self.plain {
            look.max_lines = Some(rows);
        }
        self
    }

    /// Whether the field takes the focus when it is built (a plain-text composer).
    #[must_use]
    pub fn autofocus(mut self, on: bool) -> Composer {
        if let Some(look) = &mut self.plain {
            look.autofocus = on;
        }
        self
    }

    /// The keys the owner reads first: a key `take` takes goes no further
    /// (the sheets' Esc, Ctrl+C, Tab and their moves between questions).
    #[must_use]
    pub fn on_keys(
        mut self,
        take: impl Fn(&TextEditingController, &KeyPress, KeyCx) -> bool + 'static,
    ) -> Composer {
        if let Some(look) = &mut self.plain {
            look.keys = Some(Rc::new(take));
        }
        self
    }

    /// Told the size of a message over the limit that Enter or Tab refused:
    /// the draft stays, and `on_submitted` is not told. Without it the text
    /// goes to `on_submitted` as it is, for an owner that checks it itself.
    #[must_use]
    pub fn on_too_long(mut self, h: Handler<usize>) -> Composer {
        self.on_too_long = Some(h);
        self
    }
}

/// The prompt and the field of a plain-text composer: no aura or effort
/// glyph (an answer is not a turn), no shell prompt, no remote images.
pub(super) fn row(
    look: &Look,
    tokens: &Tokens,
    label: &'static str,
    field: AnyWidget,
) -> AnyWidget {
    if look.unprompted {
        return field;
    }
    let style = look.prompt.clone().unwrap_or_else(|| tokens.bold());
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(prompt(label, style))
        .child(Expanded::new(1, field))
        .into()
}

/// The plain-text composer of `ui` over its key footer, filling the screen
/// (debug menu `composer/plain`): what the oracle's composer harness draws
/// for `ChatComposerConfig::plain_text()`. An over-long message is refused
/// with the footer's red flash for five seconds, the draft kept.
#[derive(Clone)]
pub struct Embedded {
    pub ui: Rc<ThreadUi>,
}

impl PartialEq for Embedded {
    fn eq(&self, other: &Embedded) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
    }
}

impl_widget!(stateful eq Embedded);

/// What the footer reads of the draft, and the flash it shows.
pub struct EmbeddedState {
    node: FocusNode,
    draft_empty: bool,
    /// The limit-flash shows, until its timer is up.
    flash: bool,
    timer: Option<Timer>,
}

/// What the harness is told of.
#[derive(Clone, PartialEq)]
pub enum Msg {
    /// The draft changed.
    Changed,
    /// A submit of this many characters was refused.
    TooLong(usize),
    /// The flash's five seconds are up.
    Expired,
}

impl StatefulWidget for Embedded {
    type State = EmbeddedState;

    fn create_state(&self) -> EmbeddedState {
        EmbeddedState {
            node: FocusNode::new(),
            draft_empty: self.ui.composer.text().is_empty(),
            flash: false,
            timer: None,
        }
    }
}

impl State for EmbeddedState {
    type Widget = Embedded;
    type Msg = Msg;

    fn dispose(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, msg: Msg, w: &Embedded, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Changed => self.draft_empty = w.ui.composer.text().is_empty(),
            Msg::TooLong(_) => {
                self.flash = true;
                if let Some(old) = self.timer.take() {
                    old.cancel();
                }
                if let Some(binding) = AppBinding::maybe_of(cx) {
                    self.timer = Some(binding.start_timer(FLASH_FOR, cx.map(|()| Msg::Expired)));
                }
            }
            Msg::Expired => {
                self.flash = false;
                self.timer = None;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Embedded, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let composer = Composer::plain(w.ui.clone(), self.node.clone())
            .on_changed(cx.map(|_| Msg::Changed))
            .on_too_long(cx.map(Msg::TooLong));
        let footer: AnyWidget = if self.flash {
            // the limit is the protocol's, the flash names it
            let table = strings::of(cx);
            Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::line(table.too_long(MAX_INPUT_CHARS)).style(tokens.error()),
            )
            .into()
        } else {
            Footer {
                line: Line::Passive,
                agents: false,
                draft_empty: self.draft_empty,
                shell: false,
                vim: None,
                hint_override: None,
                flash: None,
                on_queue: None,
                on_warnings: None,
                on_close: None,
            }
            .into()
        };
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Max)
            .child(Expanded::new(
                1,
                BoxWidget::new()
                    .style(tokens.prompt_surface(cx, false))
                    .padding(EdgeInsets::only(0, 1, 1, 1))
                    .child(composer),
            ))
            .child(footer)
            .into()
    }
}
