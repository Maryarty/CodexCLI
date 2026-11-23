//! The composer alone with its footer (`composer/alone`): the real composer
//! and the real key footer at the screen's size and nothing else, as the
//! oracle's composer harness draws them (`snapshot_composer_state_with_width`:
//! a `ChatComposer` rendered over `footer_height + 8` rows). The page's
//! bottom pane is the same two widgets under a transcript and a status row.

use std::rc::Rc;

use rtwk::focus::{FocusNode, KeyEventResult};
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::threads::ThreadModel;

use crate::app::theme;
use crate::pages::session::footer::{Flash, Footer, Line, Timed};
use crate::pages::session::popovers::mention;
use crate::sheets::dialogs::shortcut_panel;
use crate::widgets::thread_ui::ThreadUi;

use super::{Composer, search};

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
}

impl PartialEq for Alone {
    fn eq(&self, other: &Alone) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
            && self.hint_override == other.hint_override
            && self.flash == other.flash
    }
}

impl Alone {
    pub fn new(ui: Rc<ThreadUi>) -> Alone {
        Alone {
            ui,
            hint_override: None,
            flash: None,
        }
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
}

impl StatefulWidget for Alone {
    type State = AloneState;

    fn create_state(&self) -> AloneState {
        AloneState {
            node: FocusNode::new(),
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
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            if !ui.composer.text().is_empty()
                || ui.draft.in_burst()
                || key.mods.contains(Mods::CTRL)
                || key.mods.contains(Mods::ALT)
            {
                return KeyEventResult::Ignored;
            }
            match (key.code, key.text()) {
                (KeyCode::Char, Some("?")) => send(Msg::Shortcuts),
                _ => return KeyEventResult::Ignored,
            }
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
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Alone, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let composer =
            Composer::new(w.ui.clone(), self.node.clone()).on_changed(cx.map(|_| Msg::Changed));
        let footer: AnyWidget = if let Some((query, status)) = w.ui.draft.search_state() {
            search::footer(cx, query, status)
        } else {
            Timed {
                footer: Footer {
                    line: if self.shortcuts {
                        Line::Close
                    } else {
                        Line::Passive
                    },
                    agents: false,
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
                },
            }
            .into()
        };
        let running = read::<ThreadModel, _>(cx).is_running();
        let shortcuts = self.shortcuts.then(|| shortcut_panel(cx, running).0);
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
                body = body.child(panel);
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
