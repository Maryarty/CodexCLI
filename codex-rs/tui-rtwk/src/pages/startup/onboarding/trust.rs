//! The directory trust step (S04). Oracle: `onboarding/trust_directory.rs`.
//!
//! In Flutter: a `LayoutBuilder` over a `Column(min)` on the menu surface,
//! its blank rows `Flexible(loose)` so they give way first, the highlighted
//! choice in the step's `State`, the keys a `Focus` with bindings. The paths
//! get the rows the mandatory content leaves, measured as the oracle
//! measures them (`measure::rows`, `TextPainter`'s counterpart); a path that
//! does not fit its rows is cut in the middle by whole segments.

use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible, MainAxisSize, Padding,
    SizedBox,
};
use rtwk::prelude::Mods;
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::{Intent, TrustAsk, TrustCancel};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::l10n::strings as shared;
use crate::pages::agents::path::center_truncate_path;
use crate::widgets::measure;

use super::strings::{self, Strings};

/// The trust step over the model's question.
#[derive(Clone, Debug, PartialEq)]
pub struct Trust {
    pub ask: TrustAsk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    Trust,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    Up,
    Down,
    /// `1` or `y`: the highlight goes to trust, nothing is chosen (a
    /// terminal response can start with `1`).
    First,
    Cancel,
    Confirm,
}

/// Widget state: the highlighted choice (R14).
pub struct TrustState {
    highlighted: Choice,
}

impl StatefulWidget for Trust {
    type State = TrustState;

    fn create_state(&self) -> TrustState {
        TrustState {
            highlighted: Choice::Trust,
        }
    }
}

impl_widget!(stateful eq Trust);

impl State for TrustState {
    type Widget = Trust;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Trust, cx: &mut Cx<'_, Self>) -> Rebuild {
        let intent = match msg {
            Msg::Up | Msg::First => {
                self.highlighted = Choice::Trust;
                None
            }
            Msg::Down => {
                self.highlighted = Choice::Cancel;
                None
            }
            Msg::Cancel => {
                self.highlighted = Choice::Cancel;
                Some(Intent::CancelTrust)
            }
            Msg::Confirm => Some(match self.highlighted {
                Choice::Trust => Intent::Trust,
                Choice::Cancel => Intent::CancelTrust,
            }),
        };
        if let Some(intent) = intent {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Onboarding(intent));
        }
        Rebuild::Yes
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
        let options: [String; 2] = [
            strings.numbered(1, first_choice(strings, &ask), &num),
            strings.numbered(2, cancel_choice(strings, ask.cancel), &num),
        ];
        let highlighted = self.highlighted;
        let panel = LayoutBuilder::new(move |cx, constraints| {
            panel(
                cx,
                strings,
                &look,
                &ask,
                &options,
                highlighted,
                constraints.max_width,
                constraints.max_height,
            )
        });
        let ctrl = |c: char| KeyPress::char(c).with_mods(Mods::CTRL);
        let mut focus = Focus::new(panel).autofocus();
        for (key, msg) in [
            (KeyPress::UP, Msg::Up),
            (KeyPress::char('k'), Msg::Up),
            (KeyPress::DOWN, Msg::Down),
            (KeyPress::char('j'), Msg::Down),
            (KeyPress::char('1'), Msg::First),
            (KeyPress::char('y'), Msg::First),
            (KeyPress::char('2'), Msg::Cancel),
            (KeyPress::char('n'), Msg::Cancel),
            (KeyPress::char('q'), Msg::Cancel),
            (ctrl('c'), Msg::Cancel),
            (ctrl('d'), Msg::Cancel),
            (KeyPress::ESC, Msg::Cancel),
            (KeyPress::ENTER, Msg::Confirm),
        ] {
            focus = focus.bind(key, cx.send(msg));
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

/// A paragraph two cells in from each side.
fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child.into()).into()
}

#[allow(clippy::too_many_arguments)]
fn panel(
    cx: &mut Cx<'_>,
    strings: &'static Strings,
    look: &Look,
    ask: &TrustAsk,
    options: &[String; 2],
    highlighted: Choice,
    width: i32,
    height: i32,
) -> AnyWidget {
    let seg = segmenter_of(cx);
    let inner = usize::try_from(width - 4).unwrap_or(0).max(1);
    let rows = |text: &str| measure::rows(seg, text, inner).len();
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
    // The mandatory rows first (the oracle's `reserved_height`): the
    // heading, the disclosure, the two choices, the error, the hint, the note.
    let reserved = 1
        + rows(disclosure(strings, ask))
        + 2
        + ask.error.as_deref().map_or(0, rows)
        + rows(&hint.concat())
        + note.map_or(0, rows);
    let path_height = usize::try_from(height)
        .unwrap_or(0)
        .saturating_sub(reserved);
    // One path row left shows the repository root that receives trust.
    let cwd_height = if note.is_some() {
        path_height / 2
    } else {
        path_height
    };
    let path = |text: &str, max: usize| -> AnyWidget {
        let text = if measure::grapheme_rows(seg, text, inner) <= max {
            text.to_owned()
        } else {
            center_truncate_path(seg, text, inner)
        };
        inset(
            Text::new(text)
                .wrap(TextWrap::Grapheme)
                .style(look.dim.clone()),
        )
    };

    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(gap())
        .child(inset(
            Text::new(strings.folder_access).style(look.bold.clone()),
        ));
    if cwd_height > 0 {
        column = column.child(path(&ask.cwd, cwd_height));
    }
    column = column.child(gap());
    if let Some(note) = note {
        column = column.child(inset(Text::new(note).style(look.warning.clone())));
        let root_height = path_height - cwd_height;
        if root_height > 0 {
            column = column.child(path(&ask.target, root_height));
        }
        column = column.child(gap());
    }
    column = column
        .child(inset(Text::new(disclosure(strings, ask))))
        .child(gap());
    for (option, choice) in options.iter().zip([Choice::Trust, Choice::Cancel]) {
        let (mark, style) = if choice == highlighted {
            (strings.cursor_mark, look.selected.clone())
        } else {
            (strings.other_mark, Style::new())
        };
        column = column.child(Text::new([mark, option.as_str()].concat()).style(style));
    }
    column = column.child(gap());
    if let Some(error) = &ask.error {
        column = column
            .child(inset(Text::new(error.clone()).style(look.error.clone())))
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
