//! The directory trust step (S04). Oracle: `onboarding/trust_directory.rs`.
//!
//! In Flutter: a `Column(min)` on the menu surface, its blank rows
//! `Flexible(loose)` so they give way, the two choices the shared `Options`
//! (cursor in its controller, in the step's `State`), the keys a `Focus` with
//! bindings. Each path is a `Flexible(loose)` child: a `LayoutBuilder` over
//! the rows the `Column` gives it wraps the path where it fits them, else one
//! line cut in the middle by whole segments (`TextPainter`'s
//! `didExceedMaxLines`). The oracle gives the paths their rows before the
//! blanks; `Column` shares by flex factor: G51 (the three long-path
//! snapshots are pending).

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible, IntrinsicHeight,
    MainAxisSize, Padding, SizedBox,
};
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::{Intent, TrustAsk, TrustCancel};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;
use crate::l10n::strings as shared;
use crate::pages::agents::path::center_truncate_path;
use crate::widgets::measure;
use crate::widgets::options::{OptionRow, OptionStyles, Options};

use super::strings::{self, Strings};

/// The two choices: trust, and cancel.
const CHOICES: usize = 2;

/// The trust step over the model's question.
#[derive(Clone, Debug, PartialEq)]
pub struct Trust {
    pub ask: TrustAsk,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    /// Up and down: the cursor goes to the first or the last choice and
    /// stays there (the oracle's `MOVE_UP` / `MOVE_DOWN` do not wrap).
    Up,
    Down,
    /// `1` or `y`: the highlight goes to trust, nothing is chosen (a
    /// terminal response can start with `1`).
    First,
    Cancel,
    Confirm,
    /// A tap on the selected row or a double tap (ADR-011): the row is
    /// chosen as Enter would choose it.
    Accept(usize),
}

/// Widget state: the cursor of the two choices, the shared `Options`
/// controller (R14).
pub struct TrustState {
    options: Options,
}

impl StatefulWidget for Trust {
    type State = TrustState;

    fn create_state(&self) -> TrustState {
        TrustState {
            options: Options::new(),
        }
    }
}

impl_widget!(stateful eq Trust);

impl State for TrustState {
    type Widget = Trust;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, _: &Trust, cx: &mut Cx<'_, Self>) -> Rebuild {
        let intent = match msg {
            Msg::Up | Msg::First => {
                self.options.set(0);
                None
            }
            Msg::Down => {
                self.options.set(CHOICES - 1);
                None
            }
            Msg::Cancel => {
                self.options.set(CHOICES - 1);
                Some(Intent::CancelTrust)
            }
            Msg::Accept(row) => {
                self.options.set(row.min(CHOICES - 1));
                Some(if row == 0 {
                    Intent::Trust
                } else {
                    Intent::CancelTrust
                })
            }
            Msg::Confirm => Some(if self.options.cursor() == 0 {
                Intent::Trust
            } else {
                Intent::CancelTrust
            }),
        };
        if let Some(intent) = intent {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Onboarding(intent));
        }
        Rebuild::No
    }

    fn build(&self, w: &Trust, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let tokens = Tokens::of(cx);
        let look = Look {
            surface: tokens.prompt_surface(cx, false),
            dim: tokens.dim(),
            bold: tokens.bold(),
            selected: tokens.selected(),
            warning: tokens.warning(),
            error: tokens.error(),
        };
        let ask = Rc::new(w.ask.clone());
        let rows: Rc<Vec<OptionRow>> = Rc::new(
            [
                strings.numbered(1, first_choice(strings, &ask), &num),
                strings.numbered(2, cancel_choice(strings, ask.cancel), &num),
            ]
            .into_iter()
            .map(|label| OptionRow {
                label,
                description: None,
            })
            .collect(),
        );
        let options = self.options.on_accept(cx.map(Msg::Accept));
        let error = ask
            .error
            .as_deref()
            .map(|message| strings.trust_failed(&ask.target, message, &num));
        let panel = panel(
            strings,
            &look,
            &ask,
            choices(&options, &rows, &look),
            error.as_deref(),
        );
        let mut focus = Focus::new(panel).autofocus();
        for (id, msg) in [
            (fixed::PICK_PREVIOUS, Msg::Up),
            (fixed::PICK_NEXT, Msg::Down),
            (fixed::PICK_YES, Msg::First),
            (fixed::PICK_NO, Msg::Cancel),
            (fixed::PICK_LEAVE, Msg::Cancel),
            (fixed::PICK_QUIT, Msg::Cancel),
            (fixed::DISMISS, Msg::Cancel),
            (fixed::ENTER, Msg::Confirm),
        ] {
            focus = bind_fixed(focus, id, cx.send(msg));
        }
        focus.into()
    }
}

struct Look {
    surface: Style,
    dim: Style,
    bold: Style,
    selected: Style,
    warning: Style,
    error: Style,
}

fn first_choice(strings: &'static Strings, ask: &TrustAsk) -> &'static str {
    match (ask.restricted, ask.existing_task) {
        (true, true) => strings.open_existing_task,
        (true, false) => strings.open_restricted,
        (false, _) => strings.trust_continue,
    }
}

fn cancel_choice(strings: &'static Strings, cancel: TrustCancel) -> &'static str {
    strings.cancel_choices[cancel_index(cancel)]
}

fn cancel_index(cancel: TrustCancel) -> usize {
    match cancel {
        TrustCancel::Quit => 0,
        TrustCancel::AgentsOverview => 1,
        TrustCancel::CurrentTask => 2,
    }
}

fn disclosure(strings: &'static Strings, ask: &TrustAsk) -> &'static str {
    match (ask.restricted, ask.existing_task) {
        (true, true) => strings.disclosure_existing_task,
        (true, false) => strings.disclosure_restricted,
        (false, _) => strings.disclosure_trust,
    }
}

/// A blank row that gives way when the rows run short.
fn gap() -> AnyWidget {
    Flexible {
        fit: FlexFit::Loose,
        ..Flexible::new(1, SizedBox::new(None, Some(1), None))
    }
    .into()
}

/// The two choices, as tall as they are (`IntrinsicHeight` over the
/// `Options` rows, which would take all the room they are given).
fn choices(options: &Options, rows: &Rc<Vec<OptionRow>>, look: &Look) -> AnyWidget {
    IntrinsicHeight::new(
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(options.view(
                rows.clone(),
                OptionStyles {
                    cursor: look.selected.clone(),
                    label: Style::new(),
                    description: Style::new(),
                },
                0,
            )),
    )
    .into()
}

/// A paragraph two cells in from each side.
fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child.into()).into()
}

/// A path in the rows the column leaves it: wrapped where it fits them, else
/// one line cut in the middle (`TextPainter`'s `didExceedMaxLines` in a
/// `LayoutBuilder`).
fn path(text: String, style: Style) -> AnyWidget {
    Flexible {
        fit: FlexFit::Loose,
        ..Flexible::new(
            1,
            inset(LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let width = usize::try_from(constraints.max_width).unwrap_or(0).max(1);
                let height = usize::try_from(constraints.max_height).unwrap_or(0);
                let shown = if measure::grapheme_rows(seg, &text, width) <= height {
                    text.clone()
                } else {
                    center_truncate_path(seg, &text, width)
                };
                Text::new(shown)
                    .wrap(TextWrap::Grapheme)
                    .style(style.clone())
                    .into()
            })),
        )
    }
    .into()
}

fn panel(
    strings: &'static Strings,
    look: &Look,
    ask: &TrustAsk,
    choices: AnyWidget,
    error: Option<&str>,
) -> AnyWidget {
    let hint: Vec<&str> = {
        let [enter, plain, sandbox, dot, esc] = strings.trust_hint;
        let action = if ask.sandbox_hint && !ask.restricted {
            sandbox
        } else {
            plain
        };
        vec![
            enter,
            action,
            dot,
            esc,
            strings.cancel_words[cancel_index(ask.cancel)],
        ]
    };
    let note = (!ask.restricted && ask.cwd != ask.target).then_some(strings.git_note);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(gap())
        .child(inset(
            Text::new(strings.folder_access).style(look.bold.clone()),
        ))
        .child(path(ask.cwd.clone(), look.dim.clone()))
        .child(gap());
    if let Some(note) = note {
        column = column
            .child(inset(Text::new(note).style(look.warning.clone())))
            .child(path(ask.target.clone(), look.dim.clone()))
            .child(gap());
    }
    column = column
        .child(inset(Text::new(disclosure(strings, ask))))
        .child(gap())
        .child(choices)
        .child(gap());
    if let Some(error) = error {
        column = column
            .child(inset(Text::new(error.to_owned()).style(look.error.clone())))
            .child(gap());
    }
    let spans: Vec<Span> = hint
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if i == 0 || i == 3 {
                Span::new(*part)
            } else {
                Span::styled(*part, look.dim.clone())
            }
        })
        .collect();
    column = column.child(inset(Text::rich(spans))).child(gap());
    BoxWidget::new()
        .style(look.surface.clone())
        .child(column)
        .into()
}
