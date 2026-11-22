//! The chat composer of SessionPage (S10): the prompt and the `TextArea` the
//! user writes in, with Codex's editing on top of rtwk's (paste placeholders
//! as atomic elements, the kill ring, prompt history at the draft's ends),
//! the slash/mention suggestions under it, and the key footer below. The text
//! lives in the thread's `TextEditingController` (`ThreadUi::composer`), the
//! placeholders and the kill ring next to it (`ThreadUi::draft`); the page
//! places the composer and runs what it submits. Oracle:
//! `bottom_pane/chat_composer.rs`, `bottom_pane/textarea.rs`.

use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::{EditingKeys, TextArea};
use rtwk::focus::FocusNode;
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{BoxWidget, CrossAxisAlignment, Expanded, Row, Shadow};
use rtwk::overlay::{OptionItem, OptionList, Panel, Popover, fuzzy_match};
use rtwk::provider::{read, watch};
use rtwk::style::{Attrs, Style, Theme};
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};
use rtwk_core::presenter::CursorShape;

use codex_tui_core::screens::composer::history::Lookup;
use codex_tui_core::screens::composer::paste_burst::PasteBurst;
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

pub mod burst;
pub mod editing;
pub mod footer;
pub mod search;
pub mod strings;
pub mod suggest;
pub mod vim;

use burst::BurstEnv;
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
    /// Told the queue key (`composer.queue`) when it is not Tab.
    on_queue: Option<Handler<()>>,
    /// The text area alone: no prompt, no placeholder, no suggestions.
    bare: bool,
    /// Shell mode as the page built this widget: the page resets it when
    /// the composer clears, and the prompt follows.
    shell: bool,
}

impl PartialEq for Composer {
    fn eq(&self, other: &Composer) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
            && self.node == other.node
            && self.on_changed == other.on_changed
            && self.on_submitted == other.on_submitted
            && self.on_picked == other.on_picked
            && self.on_queue == other.on_queue
            && self.bare == other.bare
            && self.shell == other.shell
    }
}

impl Composer {
    /// The composer of `ui`'s draft, focused through `node`.
    pub fn new(ui: Rc<ThreadUi>, node: FocusNode) -> Composer {
        Composer {
            shell: ui.draft.shell(),
            ui,
            node: Some(node),
            on_changed: None,
            on_submitted: None,
            on_picked: None,
            on_queue: None,
            bare: false,
        }
    }

    /// The text area alone (the oracle's bare `TextArea`).
    pub fn bare(ui: Rc<ThreadUi>) -> Composer {
        Composer {
            shell: ui.draft.shell(),
            ui,
            node: None,
            on_changed: None,
            on_submitted: None,
            on_picked: None,
            on_queue: None,
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

    /// Told a press of the queue key when it is not Tab (the page binds Tab
    /// itself; `composer.queue` may be any key).
    #[must_use]
    pub fn on_queue(mut self, h: Handler<()>) -> Composer {
        self.on_queue = Some(h);
        self
    }
}

/// The text the composer of `ui` would submit now: its draft with every
/// paste placeholder expanded and shell mode's `!` in front.
pub fn submission(ui: &ThreadUi) -> Rc<str> {
    Rc::from(ui.draft.expanded(&ui.composer))
}

/// The composer of `ui` emptied by Ctrl+C: the draft is kept whole for Up
/// and Ctrl+R (`clear_for_ctrl_c`), the file is not written.
pub fn clear_cancelled(ui: &ThreadUi) {
    ui.draft.cancelled(&ui.composer);
    ui.composer.clear();
}

/// The composer of `ui` emptied for a line that went out or a command that
/// took it (not for a submit the thread did not accept: that clears
/// nothing): the draft's history entry, walk and shell mode end with the
/// text (`record_local_submission`).
pub fn clear_sent(ui: &ThreadUi) {
    ui.draft.cleared(&ui.composer);
    ui.composer.clear();
}

impl StatefulWidget for Composer {
    type State = ComposerState;

    fn create_state(&self) -> ComposerState {
        ComposerState {
            listener: None,
            suggest: Rc::default(),
        }
    }
}

impl_widget!(stateful eq Composer);

/// The listener that keeps the elements in step with the text and the caret
/// out of them, and what the editing keys read.
pub struct ComposerState {
    listener: Option<(Rc<ThreadUi>, VoidCallback, VoidCallback)>,
    /// The popup's rows and highlight, read by the key hook.
    suggest: Rc<suggest::Suggest>,
}

/// What the composer's own children tell it.
#[derive(Clone, Debug)]
pub enum ComposerMsg {
    Changed(Rc<str>),
    Submitted(Rc<str>),
    Pasted,
    /// A suggestion put in by Enter or a click: a command runs.
    Picked(Rc<str>),
    /// A suggestion completed as text by Tab.
    Completed(Rc<str>),
    /// The text or the caret changed: the popup asks for its rows again.
    Refresh,
    /// The popup's highlight moved: draw it again.
    Redraw,
    /// The pointer or the wheel is over row `i` of the popup.
    Highlight(i32),
    /// A click on row `i` of the popup.
    Click(i32),
    /// The popup lost the focus or was dismissed: closed until the text
    /// changes.
    Dismiss,
    /// Ctrl+C on a draft: it is cleared and kept in history.
    ClearForCtrlC,
    /// The history needs reads the store lacks: the app gets them from the
    /// file.
    Lookups(Vec<Lookup>),
    /// The store changed while the draft waits for a read: it takes what
    /// came in.
    Polled,
    /// The draft entered or left shell mode.
    Shell,
    /// The queue key (`composer.queue`) other than Tab.
    Queue,
    /// Reverse history search opened, took a key or ended: the preview and
    /// the footer's query are drawn again.
    Searched,
    /// The timer of the paste-burst detector: what it holds goes in if its
    /// time is up.
    BurstDue,
}

/// The reads the history needs from the file go to the app, which asks the
/// host (`popovers::Intent::Lookup`).
fn ask(cx: &mut Cx<'_, ComposerState>, lookups: Vec<Lookup>) {
    if lookups.is_empty() {
        return;
    }
    let hs = read::<Hs, _>(cx);
    for lookup in lookups {
        cx.emit(
            &hs.screen,
            ScreenIntent::from(popovers::Intent::Lookup(lookup)),
        );
    }
}

impl ComposerState {
    fn listen(&mut self, ui: &Rc<ThreadUi>, tell: Rc<dyn Fn(ComposerMsg)>) {
        self.unlisten();
        let (c, d) = (ui.composer.clone(), ui.draft.clone());
        let snap: VoidCallback = Rc::new(move || d.snap_caret(&c));
        ui.composer.listenable().add_listener(snap.clone());
        // the popup's rows follow the text and the caret
        let refresh: VoidCallback = Rc::new(move || tell(ComposerMsg::Refresh));
        ui.composer.listenable().add_listener(refresh.clone());
        self.listener = Some((ui.clone(), snap, refresh));
    }

    fn unlisten(&mut self) {
        if let Some((ui, snap, refresh)) = self.listener.take() {
            ui.composer.listenable().remove_listener(&snap);
            ui.composer.listenable().remove_listener(&refresh);
        }
    }

    /// Asks the popup's sources for the rows of the text before the caret
    /// (none in shell mode: `allow_command_popup`); whether they changed.
    fn refresh(&self, w: &Composer, states: &ScreenStates, table: &chat_strings::Strings) -> bool {
        if w.bare || w.ui.draft.shell() {
            return self.suggest.refresh(&w.ui.composer, |_| Vec::new());
        }
        self.suggest.refresh(&w.ui.composer, |before| {
            popup::suggestions(states, table, before)
        })
    }
}

impl ComposerState {
    /// `field` with the popup of the suggestions over or under it: a
    /// `Popover` of an `OptionList` (no focus of its own, the caret stays
    /// in the field), closed by Esc, a press outside or the focus leaving.
    /// Enter and a click pick, Tab completes (`suggest`).
    fn popup(&self, field: AnyWidget, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let shown = self.suggest.shown();
        let open = !shown.items.is_empty();
        let popup: AnyWidget = if open {
            let rows: Vec<OptionItem> = shown
                .items
                .iter()
                .map(|s| {
                    let label = s.label.as_ref().unwrap_or(&s.text);
                    let marks = fuzzy_match(&shown.query, label)
                        .map(|m| m.positions)
                        .unwrap_or_default();
                    let row = OptionItem::new(label.clone()).marks(marks);
                    match &s.detail {
                        Some(detail) => row.detail(detail.clone()),
                        None => row,
                    }
                })
                .collect();
            let list = OptionList::new(rows)
                .highlighted(i32::try_from(shown.highlighted).unwrap_or(i32::MAX))
                .rows(i32::try_from(popup::ROWS).unwrap_or(i32::MAX))
                .on_highlight(cx.map(ComposerMsg::Highlight))
                .on_selected(cx.map(ComposerMsg::Click));
            // rtwk's own look of a suggestion list (`PanelLook::Raised`)
            Shadow::new(
                BoxWidget::new()
                    .style(theme.surface.merge(&theme.text))
                    .child(list),
            )
            .shadow(Panel::shadow_of(&theme))
            .into()
        } else {
            AnyWidget::default()
        };
        Popover::new(open, popup, field, Some(cx.map(|()| ComposerMsg::Dismiss)))
            .fade_in(Duration::from_millis(120))
            .into()
    }
}

/// What the paste-burst detector reads of the app: its clock, a timer that
/// comes back as `BurstDue`, and the large-paste placeholder's text.
fn burst_env(
    binding: &AppBinding,
    cx: &mut Cx<'_, ComposerState>,
    table: &'static strings::Strings,
) -> BurstEnv {
    let clock = binding.clock();
    let timer = binding.clone();
    let due = cx.map(|()| ComposerMsg::BurstDue);
    BurstEnv {
        now: Rc::new(move || clock.now()),
        arm: Rc::new(move || {
            timer.start_timer(burst::FLUSH_AFTER, due.clone());
        }),
        label: Rc::new(move |chars| table.pasted(chars)),
        image: Rc::new(move |n| table.image(n)),
    }
}

impl State for ComposerState {
    type Widget = Composer;
    type Msg = ComposerMsg;

    fn init_state(&mut self, w: &Composer, cx: &mut Cx<'_, Self>) {
        let tell = cx.local_sender();
        self.listen(&w.ui, tell);
    }

    fn did_update_widget(&mut self, old: &Composer, w: &Composer, cx: &mut Cx<'_, Self>) {
        if !Rc::ptr_eq(&old.ui, &w.ui) {
            let tell = cx.local_sender();
            self.listen(&w.ui, tell);
        }
    }

    fn dispose(&mut self) {
        self.unlisten();
    }

    fn update(&mut self, msg: ComposerMsg, w: &Composer, cx: &mut Cx<'_, Self>) -> rtwk::Rebuild {
        match msg {
            ComposerMsg::Changed(text) => {
                // an image gone or yanked back: the rest are numbered again
                let table = strings::of(cx);
                w.ui.draft
                    .relabel_images(&w.ui.composer, |n| table.image(n));
                if let Some(h) = &w.on_changed {
                    cx.emit(h, text);
                }
            }
            ComposerMsg::Pasted => {
                let table = strings::of(cx);
                w.ui.draft
                    .relabel_images(&w.ui.composer, |n| table.image(n));
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                // a paste at the start may have made it shell mode
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Queue => {
                if let Some(h) = &w.on_queue {
                    cx.emit(h, ());
                }
            }
            ComposerMsg::Submitted(_) => {
                // the draft clears when the thread takes it (`clear_sent`)
                if let Some(h) = &w.on_submitted {
                    cx.emit(h, submission(&w.ui));
                }
            }
            ComposerMsg::Shell => {
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::ClearForCtrlC => {
                clear_cancelled(&w.ui);
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Lookups(lookups) => {
                ask(cx, lookups);
                return rtwk::Rebuild::No;
            }
            ComposerMsg::Polled => {
                let states = read::<ScreenStates, _>(cx);
                let mut asks = Vec::new();
                let changed =
                    w.ui.draft
                        .history_poll(&w.ui.composer, &states.popovers.history, &mut asks);
                ask(cx, asks);
                if !changed {
                    return rtwk::Rebuild::No;
                }
                // the page's footer and the app hear of the new draft
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::BurstDue => {
                let Some(binding) = AppBinding::maybe_of(cx) else {
                    return rtwk::Rebuild::No;
                };
                let table = strings::of(cx);
                let tell = cx.local_sender();
                let env = burst_env(&binding, cx, table);
                let skill = w.ui.draft.take_skill_space();
                let now = binding.clock().now();
                if burst::on_timer(&w.ui.draft, &*tell, &env, &w.ui.composer, now, skill) {
                    return rtwk::Rebuild::Yes;
                }
                // a first char still held keeps its completion's space
                w.ui.draft.keep_skill_space(skill);
            }
            ComposerMsg::Picked(text) => {
                w.ui.draft.completed(&w.ui.composer, &text);
                if let Some(h) = &w.on_picked {
                    cx.emit(h, text);
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Completed(text) => {
                w.ui.draft.completed(&w.ui.composer, &text);
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Refresh => {
                let states = read::<ScreenStates, _>(cx);
                let table = chat_strings::of(cx);
                return if self.refresh(w, &states, table) {
                    rtwk::Rebuild::Yes
                } else {
                    rtwk::Rebuild::No
                };
            }
            ComposerMsg::Redraw => return rtwk::Rebuild::Yes,
            ComposerMsg::Searched => {
                // the page's footer is the query: it hears of every change
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Highlight(at) => {
                if let Ok(at) = usize::try_from(at) {
                    self.suggest.highlight(at);
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Click(at) => {
                // a click picks as Enter does
                if let Ok(at) = usize::try_from(at)
                    && let Some(text) = self.suggest.pick(&w.ui.composer, at)
                {
                    w.ui.draft.completed(&w.ui.composer, &text);
                    if let Some(h) = &w.on_picked {
                        cx.emit(h, text);
                    }
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Dismiss => {
                self.suggest.hide(w.ui.composer.text());
                return rtwk::Rebuild::Yes;
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
            // the oracle draws the selection REVERSED (`textarea.rs`)
            .selection_style(Style::new().attrs(Attrs::REVERSE))
            .on_changed(cx.map(ComposerMsg::Changed));
        if !w.bare {
            // the bare area breaks the line on Enter, as the oracle's does
            area = area.on_submitted(cx.map(ComposerMsg::Submitted));
        }
        if let Some(node) = &w.node {
            area = area.focus_node(node.clone());
        }
        let states = watch::<ScreenStates, _>(cx);
        let table = strings::of(cx);
        if w.ui.draft.history_waiting() {
            // a read the history waits for may be in the store now
            cx.local_sender()(ComposerMsg::Polled);
        }
        let data = KeyData {
            draft: w.ui.draft.clone(),
            history: states.popovers.history.clone(),
            tell: cx.local_sender(),
            burst: AppBinding::maybe_of(cx).map(|binding| burst_env(&binding, cx, table)),
            suggest: self.suggest.clone(),
            keymap: crate::keymap::keymap(cx),
        };
        let env = data.burst.clone();
        let searching = data.clone();
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
        let area = area.on_paste(move |text| {
            // a real paste first lets what a burst holds go in, and ends the
            // burst's window (`handle_paste`, `apply_paste`)
            if let Some(env) = &env {
                burst::flush_held_with(&draft, &*tell, env, &pasted_into);
            }
            draft.burst(PasteBurst::clear_after_explicit_paste);
            // a paste while history search is open goes into its query
            if draft.searching() {
                search::paste(&searching, &pasted_into, text);
                return true;
            }
            // a paste while the `/` query is typed goes into the query
            if vim::paste_into_query(&draft, text) {
                tell(ComposerMsg::Redraw);
                return true;
            }
            draft.vim_begin_edit(&pasted_into);
            let mut took = draft.paste(
                &pasted_into,
                text,
                |chars| table.pasted(chars),
                |n| table.image(n),
            );
            // a plain paste is the field's own insert, after this returns;
            // Vim's step has to close with it, so the draft inserts it here
            if !took && draft.vim_enabled() {
                pasted_into.insert(text);
                took = true;
            }
            if took {
                vim::pasted(&draft, &pasted_into, text);
                tell(ComposerMsg::Pasted);
            }
            took
        });
        let strings = l10n::of(cx);
        let mut area = area
            .placeholder_style(tokens.dim().attrs(Attrs::DIM))
            .max_lines(6);
        // shell mode shows its `!` and no placeholder
        if !w.ui.draft.shell() {
            area = area.placeholder(strings.chat.placeholder);
        }
        let table = chat_strings::of(cx);
        // the rows of the text as it is now (the states may have changed)
        self.refresh(w, &states, table);
        let field = self.popup(keys(area.into()), cx);
        let mark = if w.ui.draft.shell() {
            prompt(
                strings::of(cx).shell_prompt,
                // `Span::from("!").light_red().bold()`; the footer's label is not bold
                tokens.shell().attrs(Attrs::BOLD),
            )
        } else {
            prompt(table.prompt, tokens.bold())
        };
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(mark)
            .child(Expanded::new(1, field))
            .into()
    }
}
