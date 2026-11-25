//! The guide of a held editor chord (oracle `keymap/chords.rs` `PendingChord::guide`, drawn by
//! `bottom_pane/shortcut_hud.rs` `render_overlay`): once the composer has held the first key of a
//! chord for the reveal delay (350 ms) a block at the bottom right of the page names the key
//! and lists what each second key does, and it goes with the chord, completed, cancelled with
//! Esc or one second old (`KEY_CHORD_TIMEOUT`). The prefix of the page's leader (`ctrl+x`,
//! `global.leader`) is held in the same place and shows the same guide, with the leader's own
//! chords after the editor's; it never gets old (`persistent`).
//!
//! In Flutter this is an `Align` over the page in a `Stack`, a `Material` panel of a title row
//! and the rows of `Text.rich`, shown by a `Timer` for the reveal and hidden by another for the
//! timeout. The held key is the composer's (`ComposerDraft::chord`, a `ChangeNotifier` the guide
//! listens to); both timers run on the app clock the composer stamped the key with.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{
    Align, Alignment, BorderType, BoxWidget, Column, CrossAxisAlignment, Divider, EdgeInsets,
    Expanded, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{
    CHAT_OPEN_PRESETS, GLOBAL_CHANGE_HOST, GLOBAL_SELECT_TRANSCRIPT,
};
use codex_tui_core::keymap::{ActionId, Binding, Keymap};
use rtwk::focus::KeyPress;

use crate::app::theme::Tokens;
use crate::keymap::{combo_of, stroke_label};
use crate::pages::session::composer::editing::chord_rows;
use crate::pages::session::strings::{self, Strings};
use crate::widgets::composer_draft::{CHORD_REVEAL_DELAY, ComposerDraft, KEY_CHORD_TIMEOUT};
use crate::widgets::hud::{column_count, grid, item};

/// How much the block tints the page's background (`SHORTCUT_HUD_TINT`).
const TINT: f64 = 0.052;
/// The narrowest the block is (`MIN_COLUMN_WIDTH`).
const MIN_WIDTH: i32 = 32;
/// The cells the block keeps from the right edge, and the rows from the bottom.
const MARGIN_RIGHT: i32 = 2;
const MARGIN_BOTTOM: i32 = 1;
/// A window shorter than this is covered whole by a compact block (`viewport.height < 7`).
const COMPACT_BELOW_ROWS: i32 = 7;
/// The cells of the left and right inset of the block's rows.
const INSET: i32 = 2;

/// The guide of the chord `draft` holds, over the page: nothing until there is one to show.
#[derive(Clone)]
pub(super) struct ChordGuide {
    pub draft: ComposerDraft,
    pub keymap: Rc<Keymap>,
}

impl PartialEq for ChordGuide {
    fn eq(&self, other: &ChordGuide) -> bool {
        self.draft == other.draft && Rc::ptr_eq(&self.keymap, &other.keymap)
    }
}

impl_widget!(stateful eq ChordGuide);

pub(super) enum Msg {
    /// The held chord started, ended or was replaced: ask again.
    Poke,
    /// The reveal or the timeout of the held chord is due.
    Tick,
}

#[derive(Default)]
pub(super) struct GuideState {
    /// The timer for the next change (the reveal, then the timeout), with the app clock's time
    /// it is due.
    timer: RefCell<Option<Timer>>,
    armed: Cell<Option<Duration>>,
    listening: Option<(ComposerDraft, VoidCallback)>,
}

impl GuideState {
    fn listen(&mut self, w: &ChordGuide, cx: &mut Cx<'_, Self>) {
        self.unlisten();
        let tell = cx.local_sender();
        let poke: VoidCallback = Rc::new(move || tell(Msg::Poke));
        w.draft.chord_listenable().add_listener(poke.clone());
        self.listening = Some((w.draft.clone(), poke));
    }

    fn unlisten(&mut self) {
        if let Some((draft, poke)) = self.listening.take() {
            draft.chord_listenable().remove_listener(&poke);
        }
    }

    fn cancel_timer(&self) {
        self.armed.set(None);
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.cancel();
        }
    }

    /// The next change is due at `at` on the app clock, `after` from now.
    fn schedule(&self, at: Duration, after: Duration, due: Handler<()>, binding: &AppBinding) {
        if self.armed.get() == Some(at)
            && self.timer.borrow().as_ref().is_some_and(Timer::is_active)
        {
            return;
        }
        self.cancel_timer();
        self.armed.set(Some(at));
        *self.timer.borrow_mut() = Some(binding.start_timer(after, due));
    }
}

impl StatefulWidget for ChordGuide {
    type State = GuideState;

    fn create_state(&self) -> GuideState {
        GuideState::default()
    }
}

impl State for GuideState {
    type Widget = ChordGuide;
    type Msg = Msg;

    fn init_state(&mut self, w: &ChordGuide, cx: &mut Cx<'_, Self>) {
        self.listen(w, cx);
    }

    fn did_update_widget(&mut self, old: &ChordGuide, w: &ChordGuide, cx: &mut Cx<'_, Self>) {
        if old.draft != w.draft {
            self.listen(w, cx);
        }
    }

    fn dispose(&mut self) {
        self.unlisten();
        self.cancel_timer();
    }

    fn update(&mut self, msg: Msg, w: &ChordGuide, cx: &mut Cx<'_, Self>) -> Rebuild {
        if matches!(msg, Msg::Tick) {
            // the timer that brought it is spent; a chord that outlived its second is dropped
            self.timer.borrow_mut().take();
            self.armed.set(None);
            if let Some((_, since)) = w.draft.chord()
                && !w.draft.chord_is_leader()
                && let Some(binding) = AppBinding::maybe_of(cx)
                && binding.clock().now().saturating_sub(since) >= KEY_CHORD_TIMEOUT
            {
                w.draft.set_chord(None);
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &ChordGuide, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let nothing = || -> AnyWidget { SizedBox::shrink().into() };
        let Some((first, since)) = w.draft.chord() else {
            self.cancel_timer();
            return nothing();
        };
        let Some(binding) = AppBinding::maybe_of(cx) else {
            return nothing();
        };
        let elapsed = binding.clock().now().saturating_sub(since);
        // (a leader's prefix waits for ever)
        let leader = w.draft.chord_is_leader();
        if !leader && elapsed >= KEY_CHORD_TIMEOUT {
            self.cancel_timer();
            return nothing();
        }
        // the guide shows after the reveal delay (`visible_pending`) and the chord ends with the
        // timeout: one timer for the next of the two
        if elapsed < CHORD_REVEAL_DELAY {
            let due = cx.map(|()| Msg::Tick);
            self.schedule(
                since + CHORD_REVEAL_DELAY,
                CHORD_REVEAL_DELAY - elapsed,
                due,
                &binding,
            );
            return nothing();
        }
        if leader {
            self.cancel_timer();
        } else {
            let due = cx.map(|()| Msg::Tick);
            self.schedule(
                since + KEY_CHORD_TIMEOUT,
                KEY_CHORD_TIMEOUT - elapsed,
                due,
                &binding,
            );
        }
        let Some((title, rows)) = guide_rows(&w.keymap, &first, leader) else {
            return nothing();
        };
        let words = strings::of(cx);
        let tokens = Tokens::of(cx);
        let surface = tokens.tint(cx, TINT, TINT);
        let guide = Guide {
            title,
            rows: rows
                .into_iter()
                .map(|(keys, label)| (keys.join(words.hud_key_separator), label))
                .collect(),
            words,
            tokens,
            surface,
        };
        LayoutBuilder::new(move |cx, constraints| {
            if constraints.max_height < COMPACT_BELOW_ROWS {
                guide.compact(cx, constraints.max_width)
            } else {
                guide.block(constraints.max_width)
            }
        })
        .into()
    }
}

/// What the block says: the prefix the user pressed and, for each action, its keys and its name.
#[derive(Clone)]
struct Guide {
    title: String,
    rows: Vec<(String, String)>,
    words: &'static Strings,
    tokens: Tokens,
    surface: Style,
}

impl Guide {
    fn items(&self) -> Vec<AnyWidget> {
        let (accent, dim) = (self.tokens.accent(), self.tokens.dim());
        self.rows
            .iter()
            .map(|(keys, label)| {
                item(
                    keys.clone(),
                    label.clone(),
                    self.words.hud_separator,
                    &accent,
                    &dim,
                )
            })
            .collect()
    }

    fn cancel(&self) -> AnyWidget {
        Text::line(self.words.chord_cancel)
            .style(self.tokens.dim())
            .into()
    }

    /// The block at the bottom right (`render_overlay`): a title row ruled to the edge, a blank
    /// row, the entries, a blank row and the key that cancels, in a surface a little off the
    /// page's.
    fn block(&self, window: i32) -> AnyWidget {
        let width = window
            .saturating_add(1)
            .div_euclid(2)
            .max(MIN_WIDTH)
            .min((window - MARGIN_RIGHT).max(0));
        let (dim, bold) = (self.tokens.dim(), self.tokens.bold());
        let lead = Text::rich(vec![
            Span::styled(self.words.chord_rule_lead, dim.clone()),
            Span::styled(self.title.clone(), bold),
            Span::new(self.words.chord_rule_gap),
        ])
        .wrap(TextWrap::None)
        .overflow(TextOverflow::Clip);
        let title = Row::new().child(lead).child(Expanded::new(
            1,
            Divider::new()
                .kind(BorderType::Single)
                .style(dim)
                .merge(false),
        ));
        let entries = self.items();
        let columns = column_count(width - 2 * INSET, entries.len());
        let rows = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .overflow_intended(true)
            .child(Text::new(String::new()))
            .child(grid(&entries, columns))
            .child(Text::new(String::new()))
            .child(self.cancel());
        let panel = BoxWidget::new().style(self.surface.clone()).child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .overflow_intended(true)
                .child(title)
                .child(Padding::new(EdgeInsets::only(INSET, 0, INSET, 0), rows)),
        );
        Align::new(
            Alignment::BOTTOM_RIGHT,
            Padding::new(
                EdgeInsets::only(0, 0, MARGIN_RIGHT, MARGIN_BOTTOM),
                SizedBox::new(Some(width), None, Some(panel.into())),
            ),
        )
        .into()
    }

    /// A window under seven rows is covered whole: the title with the cancel key beside it when
    /// they fit, the entries below, the key on its own row when it did not fit.
    fn compact(&self, cx: &mut Cx<'_, impl Sized>, window: i32) -> AnyWidget {
        let segmenter = segmenter_of(cx);
        let width = |text: &str| i32::try_from(segmenter.width(text)).unwrap_or(i32::MAX);
        let (dim, bold) = (self.tokens.dim(), self.tokens.bold());
        let gap = self.words.chord_command_gap;
        let beside = width(&self.title)
            .saturating_add(width(gap))
            .saturating_add(width(self.words.chord_cancel))
            <= window;
        let mut head = vec![Span::styled(self.title.clone(), bold)];
        if beside {
            head.push(Span::new(gap));
            head.push(Span::styled(self.words.chord_cancel, dim));
        }
        let entries = self.items();
        let columns = column_count(window, entries.len());
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .overflow_intended(true)
            .child(
                Text::rich(head)
                    .wrap(TextWrap::None)
                    .overflow(TextOverflow::Clip),
            )
            .child(grid(&entries, columns));
        if !beside {
            column = column.child(Text::new(String::new())).child(self.cancel());
        }
        BoxWidget::new()
            .style(self.surface.clone())
            .child(column)
            .into()
    }
}

/// The leader chords the guide lists, in the order the oracle registers them
/// (`leader_defaults::register`); the side-pane ones are not ported.
const LEADER_ACTIONS: [ActionId; 3] = [
    GLOBAL_CHANGE_HOST,
    GLOBAL_SELECT_TRANSCRIPT,
    CHAT_OPEN_PRESETS,
];

/// `change_host` is labelled for what it does in the keymap setup (`action_label`).
const CHANGE_HOST_LABEL: &str = "Change Where to Run";

/// The rows of the block: the keys and the action's label.
type GuideRows = Vec<(Vec<String>, String)>;

/// The block's title and its rows (the keys, the action's label) for the held `first`: the
/// editor chords it begins, then (a leader's prefix) the leader's chords, which the oracle lists
/// whether or not they can act now. `None`: `first` begins nothing to list.
fn guide_rows(keymap: &Keymap, first: &KeyPress, leader: bool) -> Option<(String, GuideRows)> {
    let (mut title, mut rows) = match chord_rows(keymap, first) {
        Some((title, rows)) => (
            Some(title),
            rows.into_iter()
                .map(|(id, keys)| (keys, action_title(id.name)))
                .collect(),
        ),
        None => (None, Vec::new()),
    };
    if leader {
        for id in LEADER_ACTIONS {
            let mut keys = Vec::new();
            for binding in keymap.bindings(id) {
                let Binding::Chord { prefix, completion } = binding else {
                    continue;
                };
                if combo_of(&prefix).is_some_and(|combo| combo.accepts_press(first)) {
                    title.get_or_insert_with(|| stroke_label(&prefix));
                    keys.push(stroke_label(&completion));
                }
            }
            if !keys.is_empty() {
                let label = if id == GLOBAL_CHANGE_HOST {
                    CHANGE_HOST_LABEL.to_string()
                } else {
                    action_title(id.name)
                };
                rows.push((keys, label));
            }
        }
    }
    (!rows.is_empty()).then(|| (title.unwrap_or_default(), rows))
}

/// An action's name as the guide titles it: `move_line_end` as `Move Line End`.
fn action_title(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The guide over `page`'s window: the page's draft and keymap say what it shows.
pub(super) fn guide(draft: ComposerDraft, keymap: Rc<Keymap>) -> ChordGuide {
    ChordGuide { draft, keymap }
}
