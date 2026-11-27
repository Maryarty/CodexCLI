//! The composer alone with its footer (`composer/alone`): the real composer
//! and the real key footer at the screen's size and nothing else, as the
//! oracle's composer harness draws them (`snapshot_composer_state_with_width`:
//! a `ChatComposer` rendered over `footer_height + 8` rows). The page's
//! bottom pane is the same two widgets under a transcript and a status row.

use std::cell::RefCell;
use std::rc::Rc;

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
use crate::keymap::{pressed_fixed, types_text};
use crate::pages::session::footer::{Flash, Footer, Line, StatusRows, Timed};
use crate::pages::session::popovers::mention;
use crate::sheets::dialogs::shortcut_panel_fixed;
use crate::widgets::thread_ui::ThreadUi;

use super::{Composer, clear_sent, search, vim};

/// The footer's row (`FooterMode::Passive`), kept clear of the popup.
const FOOTER_ROWS: i32 = 1;
/// The rows the oracle's composer layout keeps for the draft (`Min(3)`).
const DRAFT_ROWS: i32 = 3;

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
}

impl PartialEq for Alone {
    fn eq(&self, other: &Alone) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
            && self.hint_override == other.hint_override
            && self.flash == other.flash
            && self.agents == other.agents
            && self.status == other.status
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
        }
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
    /// The shortcut overlay is open above the composer (`FooterMode::ShortcutOverlay`).
    shortcuts: bool,
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
}

impl StatefulWidget for Alone {
    type State = AloneState;

    fn create_state(&self) -> AloneState {
        AloneState {
            node: FocusNode::new(),
            keymap: Rc::new(RefCell::new(Rc::new(Keymap::defaults()))),
            draft_empty: self.ui.composer.text().is_empty(),
            draft_shell: self.ui.draft.shell(),
            shortcuts: false,
        }
    }
}

impl State for AloneState {
    type Widget = Alone;
    type Msg = Msg;

    fn init_state(&mut self, w: &Alone, cx: &mut Cx<'_, Self>) {
        // `?` on an empty composer opens the shortcut overlay (the page's key,
        // as in `Chat`); with text, a paste or a modifier it goes to the field.
        let send = cx.local_sender();
        let ui = w.ui.clone();
        let keymap = self.keymap.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            if !ui.composer.text().is_empty() || ui.draft.in_burst() || !types_text(&key) {
                return KeyEventResult::Ignored;
            }
            // Normal mode's search keys come first: `?` searches backward there
            if vim::wants_search_key(&ui.draft, &keymap.borrow(), &key) {
                return KeyEventResult::Ignored;
            }
            if !pressed_fixed(fixed::OPEN_SHORTCUTS, &key) {
                return KeyEventResult::Ignored;
            }
            send(Msg::Shortcuts);
            KeyEventResult::Handled
        })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
    }

    fn update(&mut self, msg: Msg, w: &Alone, _: &mut Cx<'_, Self>) -> Rebuild {
        self.draft_shell = w.ui.draft.shell();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
        match msg {
            Msg::Shortcuts => self.shortcuts = !self.shortcuts,
            // any other edit puts the overlay away
            Msg::Changed if !self.draft_empty => self.shortcuts = false,
            Msg::Changed => {}
            Msg::Submitted => {
                clear_sent(&w.ui);
                self.draft_shell = false;
                self.draft_empty = true;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Alone, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        *self.keymap.borrow_mut() = crate::keymap::keymap(cx);
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
                line: if self.shortcuts {
                    Line::Close
                } else {
                    Line::Passive
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
        let running = read::<ThreadModel, _>(cx).is_running();
        let shortcuts = self
            .shortcuts
            .then(|| shortcut_panel_fixed(cx, running, w.agents));
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
