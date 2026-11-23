//! The chat composer of SessionPage (S10): the prompt and the `TextArea` the
//! user writes in, with Codex's editing on top of rtwk's (paste placeholders
//! as atomic elements, the kill ring, prompt history at the draft's ends),
//! the slash/mention suggestions under it, and the key footer below. The text
//! lives in the thread's `TextEditingController` (`ThreadUi::composer`), the
//! placeholders and the kill ring next to it (`ThreadUi::draft`); the page
//! places the composer and runs what it submits. Oracle:
//! `bottom_pane/chat_composer.rs`, `bottom_pane/textarea.rs`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::editing::{EditingKeys, TextArea};
use rtwk::focus::FocusNode;
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    Shadow, SizedBox,
};
use rtwk::overlay::{OptionItem, OptionList, Panel, Popover, fuzzy_match};
use rtwk::provider::{read, select, watch};
use rtwk::style::{Attrs, Style, Theme};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};
use rtwk_core::presenter::CursorShape;

use codex_tui_core::app::{ConnectionState, SessionSlice};
use codex_tui_core::ids::BackendId;
use codex_tui_core::screens::composer;
use codex_tui_core::data::history::Lookup;
use codex_tui_core::kit::paste_burst::PasteBurst;
use codex_tui_core::screens::composer::restricted::Restricted;
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat, popovers};
use codex_tui_core::threads::{Note, NoteKey, ThreadModel, Toggle};

use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as l10n;
use crate::pages::session::actions::prompt;
use crate::pages::session::footer::{Aura, Flash, Prompt, Tone};
use crate::pages::session::popup;
use crate::pages::session::strings as chat_strings;
use crate::widgets::thread_ui::ThreadUi;

pub mod alone;
pub mod burst;
pub mod editing;
pub mod footer;
pub mod mentions;
pub mod plain;
pub mod search;
pub mod strings;
pub mod suggest;
pub mod vim;

use burst::BurstEnv;
use editing::KeyData;

/// What the debug menu lists for the composer: the bare text area (the
/// oracle's `textarea.rs` tests draw it so) and the composer with its footer
/// (the oracle's composer harness).
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "textarea",
            description: "the composer's text area alone, with its editing keys",
            script: include_str!("fixtures/textarea.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                Composer::bare(ui).into()
            }),
        },
        DebugEntry {
            name: "plain",
            description: "the plain-text composer of question sheets, with its footer",
            script: include_str!("fixtures/plain.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| plain::Embedded { ui: plain::ui() }.into()),
        },
        DebugEntry {
            name: "alone",
            description: "the composer with its key footer and nothing else",
            script: include_str!("fixtures/alone.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_remote",
            description: "the composer with its key footer and two remote images above the draft",
            script: include_str!("fixtures/alone.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                // the oracle's `set_remote_image_urls` of its remote-rows snapshots
                let urls = ["https://example.com/one.png", "https://example.com/two.png"]
                    .map(String::from)
                    .to_vec();
                ui.draft
                    .set_remote_images(&ui.composer, urls, |n| strings::EN.image(n));
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_history",
            description: "the composer with its key footer and two prompts of history (Ctrl+R)",
            script: include_str!("fixtures/alone_history.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_history_separators",
            description: "the composer with its key footer and history holding pasted separators",
            script: include_str!("fixtures/alone_history_separators.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_plan",
            description: "the composer with its key footer in Plan mode",
            script: include_str!("fixtures/alone_plan.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_queue",
            description: "the composer with its key footer while a turn runs, 98% of the context left",
            script: include_str!("fixtures/alone_queue.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_plan_queue",
            description: "the composer with its key footer in Plan mode while a turn runs",
            script: include_str!("fixtures/alone_plan_queue.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
        DebugEntry {
            name: "alone_flash",
            description: "the composer with its key footer: the hint override `K label` under a ten-second flash `FLASH`",
            script: include_str!("fixtures/alone.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui)
                    .hint_override(vec![("K".to_owned(), "label".to_owned())])
                    .flash(Flash {
                        text: "FLASH".to_owned(),
                        tone: Tone::Plain,
                        epoch: 1,
                        lasts: std::time::Duration::from_secs(10),
                    })
                    .into()
            }),
        },
        DebugEntry {
            name: "alone_running",
            description: "the composer with its key footer while a turn runs",
            script: include_str!("fixtures/alone_running.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| {
                let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
                alone::Alone::new(ui).into()
            }),
        },
    ]
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
    /// The plain-text composer (`ChatComposerConfig::plain_text`) of the
    /// answers inside question sheets: no popups, no shell mode, no images,
    /// no queue; see `plain`.
    plain: Option<plain::Look>,
    /// An over-long submit of a plain-text composer (the text is kept).
    on_too_long: Option<Handler<usize>>,
    /// Shell mode as the page built this widget: the page resets it when
    /// the composer clears, and the prompt follows.
    shell: bool,
    /// Why the page restricts the composer (a thread that cannot take a
    /// turn, a draft handed over to a cloud switch); a lost connection is
    /// the composer's own to see.
    restricted: Option<Restricted>,
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
            && self.plain == other.plain
            && self.on_too_long == other.on_too_long
            && self.shell == other.shell
            && self.restricted == other.restricted
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
            plain: None,
            on_too_long: None,
            restricted: None,
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
            plain: None,
            on_too_long: None,
            restricted: None,
        }
    }

    /// Told each text the draft changes to.
    #[must_use]
    pub fn on_changed(mut self, h: Handler<Rc<str>>) -> Composer {
        self.on_changed = Some(h);
        self
    }

    /// The composer only edits, and Enter runs only the commands `mode` lets
    /// pass (`handle_restricted_key`).
    #[must_use]
    pub fn restricted(mut self, mode: Restricted) -> Composer {
        self.restricted = Some(mode);
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
            restricted: Cell::new(false),
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
    /// Whether the last build saw a restricted composer: a change into it
    /// restricts the draft.
    restricted: Cell<bool>,
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
    /// Modal editing was turned on or off by its key: the transcript says so.
    VimToggled(bool),
    /// A key changed the Vim mode: the page's footer names it.
    VimMode,
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
    /// Row `i` of the `@`/`$` popup was picked (Enter, Tab or a click).
    MentionPick(usize),
    /// The view was built while a mention popup is up: the app's data may
    /// have changed under it, it asks them again.
    SyncMentions,
    /// Ctrl+V: the clipboard's image is asked of the app.
    PasteImage,
    /// The view was built after the image this draft asked for came in:
    /// it is attached.
    ImageArrived,
    /// A remote image row was selected, deselected or removed.
    Remote,
    /// The composer became restricted: what a burst holds goes in, the
    /// history search ends, the popups close and the pastes are expanded.
    Restrict,
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
        // a click on a row of the mention popup picks as Enter does
        let chosen = tell.clone();
        ui.draft
            .mentions()
            .on_choose(Rc::new(move |at| chosen(ComposerMsg::MentionPick(at))));
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
    /// (none in shell mode, nor where `@` or `$` calls for the mention
    /// popup: `allow_command_popup`); whether they changed.
    fn refresh(
        &self,
        w: &Composer,
        popovers: &popovers::Model,
        slash: &popup::Slash,
        table: &chat_strings::Strings,
    ) -> bool {
        let (draft, c) = (&w.ui.draft, &w.ui.composer);
        if w.bare
            || w.plain.is_some()
            || draft.shell()
            || draft.mentions_claim_caret(c, &popovers.sources())
        {
            return self.suggest.refresh(c, |_| Vec::new());
        }
        self.suggest.refresh(&w.ui.composer, |before| {
            popup::suggestions(slash, table, before)
        })
    }
}

/// The slice of the app's states a composer draws (`select`ed: it wakes when this changes, not
/// on every change of any screen).
#[derive(Clone, PartialEq)]
struct Drawn {
    popovers: Rc<popovers::Model>,
    slash: popup::Slash,
}

impl ComposerState {
    /// Why this composer is restricted now: the page's reason, or the lost
    /// connection of the thread's backend (`reconnect.offline`: reconnecting
    /// or given up). The bare text area is never.
    fn mode(&self, w: &Composer, cx: &mut Cx<'_, Self>) -> Option<Restricted> {
        if w.bare || w.plain.is_some() {
            return None;
        }
        if w.restricted.is_some() {
            return w.restricted;
        }
        let session = watch::<SessionSlice, _>(cx);
        let thread = read::<ThreadModel, _>(cx);
        let id = thread.id().as_str().to_owned();
        let backend = select::<ScreenStates, BackendId, _>(cx, move |states| {
            states.agents.owner_of(&id)
        });
        matches!(
            session.connections.get(&backend),
            Some(ConnectionState::Reconnecting | ConnectionState::Disconnected)
        )
        .then_some(Restricted::Disconnected)
    }

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
                // an offline draft shows what Enter would run at once (the
                // app's Null key after a paste); the other modes expand it
                // on the next key
                if self.mode(w, cx).is_some_and(Restricted::expands_on_paste) {
                    w.ui.draft.expand_pastes(&w.ui.composer);
                }
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
                let text = submission(&w.ui);
                // a plain-text composer refuses an over-long message whole
                // and keeps the draft (`prepare_submission_text`)
                if let Some(h) = &w.on_too_long
                    && w.plain.is_some()
                    && let Some(chars) = composer::too_large(&text)
                {
                    cx.emit(h, chars);
                    return rtwk::Rebuild::No;
                }
                // the draft clears when the thread takes it (`clear_sent`)
                if let Some(h) = &w.on_submitted {
                    cx.emit(h, text);
                }
            }
            ComposerMsg::Shell => {
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::VimMode => {
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
                mentions::sync(w, &states, cx);
                // a click or a key of the field ended the row's selection
                let deselected = w.ui.draft.remote_deselect_on_move(&w.ui.composer);
                let model = read::<ThreadModel, _>(cx).screen_slots().chat.model.clone();
                let slash = popup::slash(&states, model.as_deref());
                return if self.refresh(w, &states.popovers, &slash, table) || deselected {
                    rtwk::Rebuild::Yes
                } else {
                    rtwk::Rebuild::No
                };
            }
            ComposerMsg::Redraw => {
                // a Vim `/` query may have opened: no mention popup meanwhile
                let states = read::<ScreenStates, _>(cx);
                mentions::sync(w, &states, cx);
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Searched => {
                // history search owns the keys: no mention popup meanwhile
                let states = read::<ScreenStates, _>(cx);
                mentions::sync(w, &states, cx);
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
            ComposerMsg::MentionPick(at) => {
                let states = read::<ScreenStates, _>(cx);
                let table = strings::of(cx);
                if !mentions::pick(w, at, |n| table.image(n), &states, cx) {
                    return rtwk::Rebuild::No;
                }
                // the page's footer and the app hear of the new draft
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::SyncMentions => {
                // the popup draws itself again: the composer stays as it is
                let states = read::<ScreenStates, _>(cx);
                mentions::sync(w, &states, cx);
            }
            ComposerMsg::VimToggled(on) => {
                let hs = read::<Hs, _>(cx);
                let thread = read::<SessionSlice, _>(cx).active.clone();
                let note = Note::from(NoteKey::Toggled(Toggle::Vim, on));
                let intent = chat::Intent::Note { thread, note };
                cx.emit(&hs.screen, ScreenIntent::from(intent));
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::PasteImage => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::from(popovers::Intent::PasteImage));
            }
            ComposerMsg::ImageArrived => {
                let states = read::<ScreenStates, _>(cx);
                let table = strings::of(cx);
                let (draft, c) = (&w.ui.draft, &w.ui.composer);
                let took = draft.vim_direct(c, || {
                    draft.take_pasted_image(
                        c,
                        |n| states.popovers.pasted.after(n).map(Into::into),
                        |n| table.image(n),
                    )
                });
                if !took {
                    return rtwk::Rebuild::No;
                }
                // the page's footer and the app hear of the new draft
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
                return rtwk::Rebuild::Yes;
            }
            ComposerMsg::Remote => return rtwk::Rebuild::Yes,
            ComposerMsg::Restrict => {
                if let Some(binding) = AppBinding::maybe_of(cx) {
                    let table = strings::of(cx);
                    let tell = cx.local_sender();
                    let env = burst_env(&binding, cx, table);
                    burst::flush_held_with(&w.ui.draft, &*tell, &env, &w.ui.composer);
                }
                w.ui.draft.restrict(&w.ui.composer);
                self.suggest.hide(w.ui.composer.text());
                if let Some(h) = &w.on_changed {
                    cx.emit(h, w.ui.composer.text());
                }
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
            .autofocus(w.plain.as_ref().is_none_or(|look| look.autofocus))
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
        // the slice of the app's states the composer draws: what the popups list and the host
        // answered (the file, the clipboard's image), the slash popup's commands
        let model = read::<ThreadModel, _>(cx).screen_slots().chat.model.clone();
        let Drawn { popovers, slash } =
            select::<ScreenStates, Drawn, _>(cx, move |states| Drawn {
                popovers: Rc::new(states.popovers.clone()),
                slash: popup::slash(states, model.as_deref()),
            });
        let table = strings::of(cx);
        if w.ui.draft.history_waiting() {
            // a read the history waits for may be in the store now
            cx.local_sender()(ComposerMsg::Polled);
        }
        if w.ui.draft.mentions().is_open() {
            // the mentions or the file matches may have changed under it
            cx.local_sender()(ComposerMsg::SyncMentions);
        }
        if w.ui.draft.image_arrived(|n| popovers.pasted.after(n).is_some()) {
            // the clipboard's image this draft asked for is in
            cx.local_sender()(ComposerMsg::ImageArrived);
        }
        let restricted = self.mode(w, cx);
        if self.restricted.replace(restricted.is_some()) != restricted.is_some()
            && restricted.is_some()
        {
            cx.local_sender()(ComposerMsg::Restrict);
        }
        let data = KeyData {
            restricted,
            draft: w.ui.draft.clone(),
            history: popovers.history.clone(),
            tell: cx.local_sender(),
            burst: AppBinding::maybe_of(cx).map(|binding| burst_env(&binding, cx, table)),
            suggest: self.suggest.clone(),
            keymap: crate::keymap::keymap(cx),
            popovers: popovers.clone(),
            sparkle: w.ui.sparkle.clone(),
        };
        let env = data.burst.clone();
        let searching = data.clone();
        // a plain-text composer's owner reads its keys first
        let owner = w.plain.as_ref().and_then(|look| look.keys.clone());
        let keys = |child: AnyWidget| -> AnyWidget {
            EditingKeys::new(
                move |c, key, kcx| {
                    owner.as_ref().is_some_and(|take| take(c, key, kcx))
                        || editing::take_key(&data, c, key, kcx)
                },
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
        let sparkle = w.ui.sparkle.clone();
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
            // the sparkle hears of the paste before the field takes it
            // (`note_sparkle_paste`)
            let current = pasted_into.text();
            sparkle.pasted(
                text,
                &current,
                pasted_into.selection().extent_offset,
                current.is_empty() && !draft.shell(),
            );
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
            area = area.placeholder(
                w.plain
                    .as_ref()
                    .and_then(|look| look.placeholder)
                    .unwrap_or(strings.chat.placeholder),
            );
        }
        // the plain-text composer: the prompt and the field, nothing around
        if let Some(look) = &w.plain {
            return plain::row(
                look,
                &tokens,
                chat_strings::of(cx).prompt,
                keys(area.into()),
            );
        }
        let table = chat_strings::of(cx);
        // the rows of the text as it is now (the states may have changed)
        self.refresh(w, &popovers, &slash, table);
        let field = self.popup(keys(area.into()), cx);
        let mark = if w.ui.draft.shell() {
            prompt(
                strings::of(cx).shell_prompt,
                // `Span::from("!").light_red().bold()`; the footer's label is not bold
                tokens.shell().attrs(Attrs::BOLD),
            )
        } else {
            Prompt {
                label: table.prompt,
                style: tokens.bold(),
            }
            .into()
        };
        let draft_row = Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(mark)
            .child(Expanded::new(1, field));
        let remote = w.ui.draft.remote_count();
        if remote == 0 {
            return Aura {
                child: draft_row.into(),
            }
            .into();
        }
        // the remote images stand above the draft, one blank row between
        let selected = w.ui.draft.remote_selected();
        let labels = strings::of(cx);
        let start = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min);
        let rows = (0..remote).fold(start, |column, at| {
            let style = if selected == Some(at) {
                tokens.info().attrs(Attrs::REVERSE)
            } else {
                tokens.info()
            };
            column.child(Padding::new(
                EdgeInsets::only(PROMPT_COLS, 0, 0, 0),
                Text::line(labels.image(at + 1)).style(style),
            ))
        });
        Aura {
            child: rows
                .child(SizedBox::new(None, Some(1), None))
                .child(draft_row)
                .into(),
        }
        .into()
    }
}

/// The columns the prompt takes before the draft's text
/// (`LIVE_PREFIX_COLS`): the remote image rows start there.
const PROMPT_COLS: i32 = 2;
