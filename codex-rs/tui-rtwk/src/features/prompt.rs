//! The text prompt of a feature (`/rename`, the file name of `/export`, `/goal edit`; oracle
//! `bottom_pane/custom_prompt_view.rs` on the shared-picker layout `custom_prompt_view/picker.rs`):
//! a title, the line that says where a suggestion stands, the input behind the `›` mark and the
//! hint. In Flutter it is a `TextField` over a `TextEditingController` in a bottom sheet; here the
//! field is the plain-text composer (`ChatComposerConfig::plain_text`), which brings what the
//! oracle's prompt shares with the chat's composer: the paste-burst detector (a fast Enter is a
//! newline, not a submit), the Vim engine and the keymap. The controller, the focus node, whether
//! the user typed and the mode the hint names are this widget's state (ADR-013: field text before
//! submit is UI state), and the feature hears only the verb the opener makes of the submitted text.
//!
//! A generated suggestion (the title `/rename` asks the host for) is a prop: it fills the field
//! once, and only while the user has not edited the text (`apply_text_suggestion`): any change of
//! the text since the prompt put it in, even one that left it as it was, is an edit.
//!
//! The layout follows `picker_areas`: given less room than its rows it drops the blank rows (the
//! top one under four rows of panel, the two around the input under three of input) and the input
//! takes what is left; given all the room (`fill`: the oracle's tests draw the view into a whole
//! area) it takes the rows down to the hint on the last one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::binding::AppBinding;
use rtwk::editing::{KeyCx, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyPress};
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded,
    Flexible, MainAxisSize, Padding, Row, SizedBox, UNBOUNDED,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::Provider;
use rtwk::style::{Style, Theme};
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::unicode::TextSegmenter;

use codex_tui_core::keymap::fixed;
use codex_tui_core::kit::vim::{Vim, VimMode};

use crate::app::theme::Tokens;
use crate::keymap::{bind_fixed, pressed_fixed};
use crate::pages::session::composer::{Composer, plain};
use crate::sheets::dialogs::strings;
use crate::widgets::thread_ui::ThreadUi;

/// The rows the input grows to (`desired_height` clamps its height to `1..=8`).
const MAX_ROWS: usize = 8;
/// The rows of the input given all the room: as many as the room has.
const ALL_ROWS: usize = 1000;

/// Where the generated suggestion of a prompt stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suggest {
    /// None is asked for: no label.
    Off,
    /// `Generating a title suggestion…`: the answer is awaited.
    Waiting,
    /// The answer: the text for the field, or nothing came out.
    Ready(Option<String>),
}

/// Whether the chat's composer edits modally: provided by the app's view for the prompts that
/// start in Vim's Insert mode when it does (`show_text_prompt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatVim(pub bool);

/// The chat composer's Vim flag (no provider, as in the debug entries: off).
pub fn chat_vim<S>(cx: &mut Cx<'_, S>) -> bool {
    Provider::<ChatVim>::maybe_of(cx, false).is_some_and(|vim| vim.0)
}

/// A text prompt: what it asks, what it starts with, and who hears the submitted text.
#[derive(Clone)]
pub struct TextPrompt {
    pub title: String,
    pub placeholder: &'static str,
    /// The text the field starts with.
    pub initial: String,
    pub suggest: Suggest,
    /// The trimmed text of a submit; a blank one is not an answer and the prompt stays open
    /// (`CustomPromptView`). The opener makes its verb of it; the prompt closes after (unless
    /// `keep_open`).
    pub on_submit: Handler<Rc<str>>,
    /// The two lines of `with_text_suggestion`, while the suggestion is awaited and once it is in
    /// (the prompt's own words when none).
    pub labels: Option<(&'static str, &'static str)>,
    /// The accent line a prompt that waits for no suggestion has (`context_label`).
    pub context: Option<&'static str>,
    /// The field edits modally, in Insert mode (`enable_vim_in_insert_mode`); none: as the chat's
    /// composer does (`show_text_prompt`: `composer_is_vim_enabled`).
    pub vim: Option<bool>,
    /// What Enter says in the hint (`submit` when none: `with_child_flow("select")`).
    pub submit_label: Option<&'static str>,
    /// Told Esc (`on_ctrl_c`); the prompt closes itself when none.
    pub on_cancel: Option<Handler<()>>,
    /// The submit leaves the prompt open, its owner closes it (`dismiss_after_child_accept`).
    pub keep_open: bool,
    /// The controllers of the field, when the owner holds them (the debug entries set up a
    /// prompt's state before the suggestion arrives); the prompt makes its own when none.
    pub ui: Option<Rc<ThreadUi>>,
    /// The prompt takes all the room it is given, its hint on the last row.
    pub fill: bool,
}

impl PartialEq for TextPrompt {
    fn eq(&self, other: &TextPrompt) -> bool {
        self.title == other.title
            && self.placeholder == other.placeholder
            && self.initial == other.initial
            && self.suggest == other.suggest
            && self.on_submit == other.on_submit
            && self.labels == other.labels
            && self.context == other.context
            && self.vim == other.vim
            && self.submit_label == other.submit_label
            && self.on_cancel == other.on_cancel
            && self.keep_open == other.keep_open
            && self.fill == other.fill
            && match (&self.ui, &other.ui) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

impl TextPrompt {
    /// A prompt that asks `title`, starts with `initial` and tells `on_submit` the answer.
    pub fn new(
        title: impl Into<String>,
        placeholder: &'static str,
        initial: impl Into<String>,
        suggest: Suggest,
        on_submit: Handler<Rc<str>>,
    ) -> TextPrompt {
        TextPrompt {
            title: title.into(),
            placeholder,
            initial: initial.into(),
            suggest,
            on_submit,
            labels: None,
            context: None,
            vim: None,
            submit_label: None,
            on_cancel: None,
            keep_open: false,
            ui: None,
            fill: false,
        }
    }

    /// The field edits modally (the chat's composer does).
    #[must_use]
    pub fn vim(mut self, on: bool) -> TextPrompt {
        self.vim = Some(on);
        self
    }

    /// The lines while the suggestion is awaited and once it is in.
    #[must_use]
    pub fn labels(mut self, loading: &'static str, applied: &'static str) -> TextPrompt {
        self.labels = Some((loading, applied));
        self
    }

    /// The accent line under the title.
    #[must_use]
    pub fn context(mut self, line: &'static str) -> TextPrompt {
        self.context = Some(line);
        self
    }

    /// What Enter says in the hint.
    #[must_use]
    pub fn submit_label(mut self, label: &'static str) -> TextPrompt {
        self.submit_label = Some(label);
        self
    }

    /// Esc goes to `h` instead of closing the prompt.
    #[must_use]
    pub fn on_cancel(mut self, h: Handler<()>) -> TextPrompt {
        self.on_cancel = Some(h);
        self
    }

    /// The submit leaves the prompt open: its owner closes it.
    #[must_use]
    pub fn keep_open(mut self) -> TextPrompt {
        self.keep_open = true;
        self
    }

    /// The field's controllers are the owner's.
    #[must_use]
    pub fn with_ui(mut self, ui: Rc<ThreadUi>) -> TextPrompt {
        self.ui = Some(ui);
        self
    }

    /// The prompt takes all the room it is given.
    #[must_use]
    pub fn fill(mut self) -> TextPrompt {
        self.fill = true;
        self
    }
}

pub struct TextPromptState {
    ui: Rc<ThreadUi>,
    node: FocusNode,
    /// The text the prompt knows: what it put in the field, or what the field held when it last
    /// changed. Shared with the field's listener.
    seen: Rc<RefCell<Rc<str>>>,
    /// The text changed since the prompt put it in (typed, pasted, deleted: not a caret move).
    edited: Rc<Cell<bool>>,
    watch: Option<VoidCallback>,
    /// The suggestion was settled: it came (put in the field or not) and is not asked again.
    settled: bool,
    /// The suggestion was put in the field.
    applied: bool,
    /// What the hint said at the last build: the mode and whether Esc goes to the editor.
    shown: Cell<(Option<VimMode>, bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Esc reached the dialog: the field did not take it.
    Esc,
    /// A key went to the field: the mode or the pending command may have changed with it.
    Keyed,
    /// Enter in the field.
    Submit(Rc<str>),
}

impl StatefulWidget for TextPrompt {
    type State = TextPromptState;

    fn create_state(&self) -> TextPromptState {
        TextPromptState {
            ui: self.ui.clone().unwrap_or_else(plain::ui),
            node: FocusNode::new(),
            seen: Rc::new(RefCell::new(Rc::from(""))),
            edited: Rc::new(Cell::new(false)),
            watch: None,
            settled: false,
            applied: false,
            shown: Cell::new((None, false)),
        }
    }
}

impl_widget!(stateful eq TextPrompt);

/// The mode the hint names, and whether Esc is the editor's (Insert or Replace mode, or a
/// command waiting: `should_handle_vim_insert_escape`).
fn editing_state(ui: &ThreadUi) -> (Option<VimMode>, bool) {
    ui.draft.with_vim(|v| {
        (
            v.is_enabled().then(|| v.mode()),
            v.is_enabled() && v.owns_escape(),
        )
    })
}

impl TextPromptState {
    /// Puts a ready suggestion in the field, once, while the user has not edited the text.
    fn apply(&mut self, w: &TextPrompt) {
        let Suggest::Ready(answer) = &w.suggest else {
            return;
        };
        if self.settled {
            return;
        }
        self.settled = true;
        let Some(text) = answer else {
            return;
        };
        if self.edited.get() {
            return;
        }
        // the field's own change of text is no edit: the prompt knows it before it happens
        *self.seen.borrow_mut() = Rc::from(text.as_str());
        self.ui.composer.reset(text);
        if self.ui.draft.vim_enabled() {
            self.ui.draft.with_vim(Vim::text_replaced);
        }
        self.applied = true;
    }

    fn unwatch(&mut self) {
        if let Some(callback) = self.watch.take() {
            self.ui.composer.listenable().remove_listener(&callback);
        }
    }
}

impl State for TextPromptState {
    type Widget = TextPrompt;
    type Msg = Msg;

    fn init_state(&mut self, w: &TextPrompt, cx: &mut Cx<'_, Self>) {
        *self.seen.borrow_mut() = Rc::from(w.initial.as_str());
        self.ui.composer.reset(&w.initial);
        if w.vim.unwrap_or_else(|| chat_vim(cx)) {
            self.ui.draft.start_in_vim_insert(&self.ui.composer);
        }
        let (c, seen, edited) = (
            self.ui.composer.clone(),
            self.seen.clone(),
            self.edited.clone(),
        );
        let watch: VoidCallback = Rc::new(move || {
            let now = c.text();
            if now != *seen.borrow() {
                *seen.borrow_mut() = now;
                edited.set(true);
            }
        });
        self.ui.composer.listenable().add_listener(watch.clone());
        self.watch = Some(watch);
        self.apply(w);
    }

    fn did_update_widget(&mut self, _old: &TextPrompt, w: &TextPrompt, _cx: &mut Cx<'_, Self>) {
        self.apply(w);
    }

    fn dispose(&mut self) {
        self.unwatch();
    }

    fn update(&mut self, msg: Msg, w: &TextPrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Esc => {
                // Esc a command waits for is the editor's, whatever reached the dialog
                if editing_state(&self.ui).1 {
                    return Rebuild::No;
                }
                match &w.on_cancel {
                    Some(h) => cx.emit(h, ()),
                    None => close_dialog::<_, ()>(cx, None),
                }
                Rebuild::No
            }
            Msg::Keyed => {
                if editing_state(&self.ui) == self.shown.get() {
                    Rebuild::No
                } else {
                    Rebuild::Yes
                }
            }
            Msg::Submit(text) if text.trim().is_empty() => Rebuild::No,
            Msg::Submit(text) => {
                cx.emit(&w.on_submit, Rc::from(text.trim()));
                if !w.keep_open {
                    close_dialog::<_, ()>(cx, None);
                }
                Rebuild::No
            }
        }
    }

    fn build(&self, w: &TextPrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let (mode, escape) = editing_state(&self.ui);
        self.shown.set((mode, escape));
        let (loading, applied) = w
            .labels
            .unwrap_or((s.prompt_suggestion_loading, s.prompt_suggestion_ready));
        let context = match (&w.suggest, self.applied) {
            (Suggest::Waiting, _) => Some(loading),
            (_, true) => Some(applied),
            _ => w.context,
        };
        let hint = format!(
            "{} {}{}{} {}",
            s.prompt_hint_enter,
            w.submit_label.unwrap_or(s.prompt_submit),
            s.prompt_hint_join,
            s.prompt_hint_esc,
            if escape {
                s.prompt_normal_mode
            } else {
                s.prompt_back
            },
        );
        let vim = mode.map(|mode| {
            let (text, style) = match mode {
                VimMode::Normal => (s.prompt_vim_normal, tokens.accent()),
                VimMode::Insert => (s.prompt_vim_insert, tokens.success()),
                VimMode::Replace => (s.prompt_vim_replace, tokens.info()),
            };
            (text, style)
        });

        // The keys the prompt reads before the field: the field's Tab is the composer's submit
        // (a burst's Tab is the burst's), the prompt's is nothing; each key may change the mode.
        let send = cx.local_sender();
        let ui = self.ui.clone();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let take = move |_: &TextEditingController, key: &KeyPress, _: KeyCx| -> bool {
            // Esc the editor does not own is the prompt's, before Normal mode's own cancel takes it
            // (`prefer_esc_to_handle_key_event`: the view cancels, the text area never sees it)
            if pressed_fixed(fixed::DISMISS, key) && !editing_state(&ui).1 {
                send(Msg::Esc);
                return true;
            }
            send(Msg::Keyed);
            if !pressed_fixed(fixed::TAB, key) {
                return false;
            }
            let in_burst = clock.as_ref().is_some_and(|clock| {
                let now = clock.now();
                ui.draft
                    .burst(|b| b.direct_insert_newline_should_insert(now))
            });
            ui.draft.with_vim(|v| v.is_normal()) || !in_burst
        };
        let input = Composer::plain(self.ui.clone(), self.node.clone())
            .placeholder(w.placeholder)
            .prompt_style(tokens.accent())
            .max_lines(if w.fill { ALL_ROWS } else { MAX_ROWS })
            .on_keys(take)
            .on_submitted(cx.map(Msg::Submit));

        let title = w.title.clone();
        let fill = w.fill;
        let surface = tokens.prompt_surface(cx, false);
        let background = Theme::of(cx).background.clone();
        let body = LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            let width = constraints.max_width;
            let header_rows = rows(seg, &title, width - 4)
                + context.map_or(0, |context| rows(seg, context, width - 4));
            let (top, header, gap) = spacing(constraints.max_height, header_rows);
            let blank = |rows: i32| SizedBox::new(None, Some(rows), None);
            let sides = |child: AnyWidget| -> AnyWidget {
                Padding::new(EdgeInsets::symmetric(2, 0), child).into()
            };
            let mut head = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(sides(
                    Text::new(title.clone())
                        .style(tokens.bold())
                        .wrap(TextWrap::Word)
                        .into(),
                ));
            if let Some(context) = context {
                head = head.child(sides(
                    Text::new(context)
                        .style(tokens.accent())
                        .wrap(TextWrap::Word)
                        .into(),
                ));
            }
            let head: AnyWidget = if header < header_rows {
                ConstrainedBox::new(BoxConstraints::new(0, UNBOUNDED, 0, header), head).into()
            } else {
                head.into()
            };
            let field = Row::new()
                .child(Expanded::new(1, input.clone()))
                .child(SizedBox::new(Some(2), None, None));
            let hint = hint_row(&tokens, seg, &hint, vim.clone(), width);
            let panel = Column::new()
                .main_axis_size(if fill {
                    MainAxisSize::Max
                } else {
                    MainAxisSize::Min
                })
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(blank(top))
                .child(head)
                .child(blank(gap));
            let panel = if fill {
                panel.child(Expanded::new(1, field))
            } else {
                panel.child(Flexible::new(1, field))
            };
            let panel = BoxWidget::new()
                .style(surface.clone())
                .child(panel.child(blank(gap)));
            // The panel is the user-message surface; the hint row under it is the cleared area.
            let hint = BoxWidget::new().style(background.clone()).child(hint);
            let column = Column::new()
                .main_axis_size(if fill {
                    MainAxisSize::Max
                } else {
                    MainAxisSize::Min
                })
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            if fill {
                column.child(Expanded::new(1, panel))
            } else {
                column.child(Flexible::new(1, panel))
            }
            .child(hint)
            .into()
        });
        // The text area leaves Esc to the nodes above it, the Vim engine takes the one it owns.
        let focus = Focus::new(Dialog::bare(body)).can_request_focus(false);
        bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Esc)).into()
    }
}

/// The rows of the blank above the title, of the title and context, and of the blank around the
/// input in `room` rows (`picker_areas`: the panel is the room but its hint row).
fn spacing(room: i32, header: i32) -> (i32, i32, i32) {
    if room >= UNBOUNDED {
        return (1, header, 1);
    }
    let panel = (room - 1).max(0);
    let top = i32::from(panel >= 4);
    let header = header.min((panel - top - 1).max(0));
    let remaining = panel - top - header;
    (top, header, i32::from(remaining >= 3))
}

/// The rows `text` takes wrapped by words in `width` columns.
fn rows(seg: TextSegmenter, text: &str, width: i32) -> i32 {
    let width = usize::try_from(width.max(1)).unwrap_or(1);
    let mut rows = 1;
    let mut used = 0;
    for word in text.split(' ') {
        let need = seg.width(word);
        if used > 0 && used + 1 + need > width {
            rows += 1;
            used = need;
        } else {
            used += usize::from(used > 0) + need;
        }
        while used > width {
            rows += 1;
            used -= width;
        }
    }
    rows
}

/// The hint, two cells in, and the mode of Vim at the right end when both fit the row
/// (`max_left_width_for_right`: the mode keeps two cells of margin and a cell of gap before it,
/// the hint starts two cells in).
fn hint_row(
    tokens: &Tokens,
    seg: TextSegmenter,
    hint: &str,
    vim: Option<(&'static str, Style)>,
    width: i32,
) -> AnyWidget {
    let dim = tokens.dim();
    let row = (width - 4).max(0);
    let text =
        |text: String, style: Style| Text::line(text).style(style).overflow(TextOverflow::Clip);
    let mut line = Row::new();
    let fits = vim.as_ref().is_some_and(|(mode, _)| {
        let mode = i32::try_from(seg.width(mode)).unwrap_or(i32::MAX);
        let max = row - 2;
        let at = if mode >= max { 2 } else { row - mode - 2 };
        let room = if at <= 2 + 1 { 0 } else { at - 3 };
        max > 0 && i32::try_from(seg.width(hint)).unwrap_or(i32::MAX) <= room
    });
    match vim {
        Some((mode, style)) if fits => {
            line = line
                .child(text(hint.to_owned(), dim))
                .child(Expanded::new(1, SizedBox::new(None, Some(1), None)))
                .child(text(mode.to_owned(), style))
                .child(SizedBox::new(Some(2), None, None));
        }
        _ => {
            line = line.child(Expanded::new(1, text(hint.to_owned(), dim)));
        }
    }
    Padding::new(EdgeInsets::only(2, 0, 2, 0), line).into()
}
