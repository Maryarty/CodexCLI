//! The chat composer of SessionPage (S10): the prompt and the `TextArea` the
//! user writes in, with Codex's editing on top of rtwk's (paste placeholders
//! as atomic elements, the kill ring, prompt history at the draft's ends),
//! the slash/mention suggestions under it, and the key footer below. The text
//! lives in the thread's `TextEditingController` (`ThreadUi::composer`), the
//! placeholders and the kill ring next to it (`ThreadUi::draft`); the page
//! places the composer and runs what it submits. Oracle:
//! `bottom_pane/chat_composer.rs`, `bottom_pane/textarea.rs`.

use std::rc::Rc;

use rtwk::editing::{Autocomplete, EditingKeys, Suggestion, TextArea};
use rtwk::focus::FocusNode;
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{CrossAxisAlignment, Expanded, Row};
use rtwk::provider::{read, watch};
use rtwk::style::Attrs;
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};
use rtwk_core::presenter::CursorShape;

use codex_tui_core::screens::{ScreenIntent, ScreenStates, popovers};

use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::actions::prompt;
use crate::pages::session::popup;
use crate::pages::session::strings as chat_strings;
use crate::widgets::thread_ui::ThreadUi;

pub mod draft;
pub mod editing;
pub mod footer;
pub mod strings;

use editing::KeyData;

/// What the debug menu lists for the composer: the bare text area, the
/// composer's editing alone at the screen's size (the oracle's
/// `textarea.rs` tests draw it so).
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![DebugEntry {
        name: "textarea",
        description: "the composer's text area alone, with its editing keys",
        script: include_str!("fixtures/textarea.jsonl"),
        open: EntryOpen::Route(|seeded: &Seeded| {
            let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
            Composer::bare(ui).into()
        }),
    }]
}

/// Opens the bare text area on a blank draft (debug menu), as a page.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Composer::bare(Rc::new(ThreadUi::default())).into())
}

/// The composer of one thread: the prompt, the text area and its
/// suggestions. A plain `Text` submit is `on_submitted`'s, with the paste
/// placeholders expanded; a pick of the suggestions is `on_picked`'s.
#[derive(Clone)]
pub struct Composer {
    ui: Rc<ThreadUi>,
    node: Option<FocusNode>,
    on_changed: Option<Handler<Rc<str>>>,
    on_submitted: Option<Handler<Rc<str>>>,
    on_picked: Option<Handler<Rc<str>>>,
    /// The text area alone: no prompt, no placeholder, no suggestions.
    bare: bool,
}

impl PartialEq for Composer {
    fn eq(&self, other: &Composer) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
            && self.node == other.node
            && self.on_changed == other.on_changed
            && self.on_submitted == other.on_submitted
            && self.on_picked == other.on_picked
            && self.bare == other.bare
    }
}

impl Composer {
    /// The composer of `ui`'s draft, focused through `node`.
    pub fn new(ui: Rc<ThreadUi>, node: FocusNode) -> Composer {
        Composer {
            ui,
            node: Some(node),
            on_changed: None,
            on_submitted: None,
            on_picked: None,
            bare: false,
        }
    }

    /// The text area alone (the oracle's bare `TextArea`).
    pub fn bare(ui: Rc<ThreadUi>) -> Composer {
        Composer {
            ui,
            node: None,
            on_changed: None,
            on_submitted: None,
            on_picked: None,
            bare: true,
        }
    }

    /// Told each text the draft changes to.
    #[must_use]
    pub fn on_changed(mut self, h: Handler<Rc<str>>) -> Composer {
        self.on_changed = Some(h);
        self
    }

    /// Told the text Enter submits, placeholders expanded.
    #[must_use]
    pub fn on_submitted(mut self, h: Handler<Rc<str>>) -> Composer {
        self.on_submitted = Some(h);
        self
    }

    /// Told the text of a suggestion picked.
    #[must_use]
    pub fn on_picked(mut self, h: Handler<Rc<str>>) -> Composer {
        self.on_picked = Some(h);
        self
    }
}

/// The text the composer of `ui` would submit now: its draft with every
/// paste placeholder expanded (Tab's queue takes it so).
pub fn submission(ui: &ThreadUi) -> Rc<str> {
    Rc::from(ui.draft.expanded(&ui.composer))
}

impl StatefulWidget for Composer {
    type State = ComposerState;

    fn create_state(&self) -> ComposerState {
        ComposerState { listener: None }
    }
}

impl_widget!(stateful eq Composer);

/// The listener that keeps the elements in step with the text and the caret
/// out of them, and what the editing keys read.
pub struct ComposerState {
    listener: Option<(Rc<ThreadUi>, VoidCallback)>,
}

/// What the composer's own children tell it.
#[derive(Clone, Debug)]
pub enum ComposerMsg {
    Changed(Rc<str>),
    Submitted(Rc<str>),
    Pasted,
    Picked(Rc<str>),
    /// Ctrl+C on a draft: it is cleared and kept in history.
    ClearForCtrlC,
}

impl ComposerState {
    fn listen(&mut self, ui: &Rc<ThreadUi>) {
        self.unlisten();
        let (c, d) = (ui.composer.clone(), ui.draft.clone());
        let snap: VoidCallback = Rc::new(move || d.snap_caret(&c));
        ui.composer.listenable().add_listener(snap.clone());
        self.listener = Some((ui.clone(), snap));
    }

    fn unlisten(&mut self) {
        if let Some((ui, snap)) = self.listener.take() {
            ui.composer.listenable().remove_listener(&snap);
        }
    }
}

impl State for ComposerState {
    type Widget = Composer;
    type Msg = ComposerMsg;

    fn init_state(&mut self, w: &Composer, _: &mut Cx<'_, Self>) {
        self.listen(&w.ui);
    }

    fn did_update_widget(&mut self, old: &Composer, w: &Composer, _: &mut Cx<'_, Self>) {
        if !Rc::ptr_eq(&old.ui, &w.ui) {
            self.listen(&w.ui);
        }
    }

    fn dispose(&mut self) {
        self.unlisten();
    }

    fn update(&mut self, msg: ComposerMsg, w: &Composer, cx: &mut Cx<'_, Self>) -> rtwk::Rebuild {
        match msg {
            ComposerMsg::Changed(text) => {
                if let Some(h) = &w.on_changed {
                    cx.emit(h, text);
                }
            }
            ComposerMsg::Pasted => {
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
            }
            ComposerMsg::Submitted(_) => {
                let text = submission(&w.ui);
                // `record_local_submission`: the next Up starts at the newest.
                w.ui.draft.record_local(&w.ui.composer);
                if let Some(h) = &w.on_submitted {
                    cx.emit(h, text);
                }
            }
            ComposerMsg::ClearForCtrlC => {
                let text = submission(&w.ui);
                w.ui.draft.record_local(&w.ui.composer);
                w.ui.composer.clear();
                let hs = read::<Hs, _>(cx);
                cx.emit(
                    &hs.screen,
                    ScreenIntent::from(popovers::Intent::Record(text.to_string())),
                );
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
            }
            ComposerMsg::Picked(text) => {
                if let Some(h) = &w.on_picked {
                    cx.emit(h, text);
                }
            }
        }
        rtwk::Rebuild::No
    }

    fn build(&self, w: &Composer, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let c = w.ui.composer.clone();
        let mut area = TextArea::new()
            .controller(c.clone())
            .cursor_shape(CursorShape::TerminalDefault)
            .min_lines(1)
            .autofocus(true)
            .on_changed(cx.map(ComposerMsg::Changed));
        if !w.bare {
            // the bare area breaks the line on Enter, as the oracle's does
            area = area.on_submitted(cx.map(ComposerMsg::Submitted));
        }
        if let Some(node) = &w.node {
            area = area.focus_node(node.clone());
        }
        let states = watch::<ScreenStates, _>(cx);
        let data = KeyData {
            draft: w.ui.draft.clone(),
            history: Rc::from(states.popovers.history.as_slice()),
            tell: cx.local_sender(),
        };
        let keys = |child: AnyWidget| -> AnyWidget {
            EditingKeys::new(
                move |c, key, kcx| editing::take_key(&data, c, key, kcx),
                child,
            )
            .controller(c.clone())
            .into()
        };
        if w.bare {
            return keys(area.into());
        }
        let tell = cx.local_sender();
        let (draft, pasted_into) = (w.ui.draft.clone(), c.clone());
        let table = strings::of(cx);
        let area = area.on_paste(move |text| {
            let took = draft.paste(&pasted_into, text, |chars| table.pasted(chars));
            if took {
                tell(ComposerMsg::Pasted);
            }
            took
        });
        let strings = l10n::of(cx);
        let area = area
            .placeholder(strings.chat.placeholder)
            .placeholder_style(tokens.dim().attrs(Attrs::DIM))
            .max_lines(6);
        let table = chat_strings::of(cx);
        let field = Autocomplete::new(
            c.clone(),
            move |before: &str| popup::suggestions(&states, table, before),
            keys(area.into()),
        )
        .rows(popup::ROWS)
        .on_selected(cx.map(|picked: Suggestion| ComposerMsg::Picked(Rc::from(&*picked.text))));
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(prompt(table.prompt, tokens.bold()))
            .child(Expanded::new(1, field))
            .into()
    }
}
