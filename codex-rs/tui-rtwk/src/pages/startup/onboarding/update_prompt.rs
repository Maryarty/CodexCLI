//! The update prompt (S01). Oracle: `update_prompt.rs`.
//!
//! In Flutter: a `Column(min)` on the menu surface: a blank row, the title,
//! the release notes, the choices (the room the rest leaves, scrolled to the
//! cursor, an arrow above and below where choices are hidden), the key hint
//! and a blank row. The cursor and the scroll are the shared `Options`
//! controller in the step's `State`; the keys a `Focus` with bindings.

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible, MainAxisSize, Padding,
    SizedBox,
};
use rtwk::provider::read;
use rtwk::scroll::ScrollController;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::{Intent, UpdateChoice, UpdateOffer};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{bind_fixed, fixed_presses};
use crate::l10n::strings as shared;
use crate::widgets::options::{OptionRow, OptionStyles, Options};

use super::strings;

const CHOICES: [UpdateChoice; 3] = [
    UpdateChoice::UpdateNow,
    UpdateChoice::Skip,
    UpdateChoice::SkipUntilNextVersion,
];

/// The update prompt over the model's offer.
#[derive(Clone, Debug, PartialEq)]
pub struct UpdatePrompt {
    pub offer: UpdateOffer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    Step(i64),
    Choose(UpdateChoice),
    Confirm,
    /// A tap on the row the cursor is on, or a double tap: Enter's action.
    Accept(usize),
}

/// Widget state: the cursor and the scroll of the choices (R14).
pub struct UpdatePromptState {
    options: Options,
}

impl StatefulWidget for UpdatePrompt {
    type State = UpdatePromptState;

    fn create_state(&self) -> UpdatePromptState {
        UpdatePromptState {
            options: Options::new(),
        }
    }
}

impl_widget!(stateful eq UpdatePrompt);

impl State for UpdatePromptState {
    type Widget = UpdatePrompt;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, msg: Msg, _: &UpdatePrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let choice = match msg {
            Msg::Step(delta) => {
                self.options.step(delta, CHOICES.len());
                return Rebuild::No;
            }
            Msg::Choose(choice) => choice,
            Msg::Confirm => CHOICES[self.options.cursor().min(CHOICES.len() - 1)],
            Msg::Accept(row) => {
                self.options.set(row);
                CHOICES[row.min(CHOICES.len() - 1)]
            }
        };
        if let Some(at) = CHOICES.iter().position(|c| *c == choice) {
            self.options.set(at);
        }
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Onboarding(Intent::ChooseUpdate(choice)),
        );
        Rebuild::No
    }

    fn build(&self, w: &UpdatePrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let tokens = Tokens::of(cx);
        let dim = tokens.dim();
        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };
        let title = Text::rich(vec![
            Span::styled(strings.update_title, tokens.bold()),
            Span::styled(strings.update_dot, dim.clone()),
            Span::styled(
                strings.update_versions(&w.offer.current, &w.offer.latest, &num),
                dim.clone(),
            ),
        ]);
        let notes = Text::rich(vec![
            Span::styled(strings.release_notes, dim.clone()),
            Span::styled(
                strings.release_notes_url,
                dim.clone().attrs(Attrs::UNDERLINE),
            )
            .link(strings.release_notes_url),
        ]);
        let labels = [
            strings.update_now(&w.offer.command, &num),
            strings.skip.to_owned(),
            strings.skip_version.to_owned(),
        ];
        let rows: Rc<Vec<OptionRow>> = Rc::new(
            labels
                .iter()
                .enumerate()
                .map(|(i, label)| OptionRow {
                    label: strings.numbered(i + 1, label, &num),
                    description: None,
                })
                .collect(),
        );
        let scroll = self.options.scroll();
        let arrow = |above: bool| -> AnyWidget {
            let scroll = scroll.clone();
            let style = tokens.accent();
            SizedBox::new(
                None,
                Some(1),
                Some(
                    ListenableBuilder::new(Rc::new(scroll.clone()), move |_, _| {
                        let shown = hidden(&scroll, above);
                        let mark = match (shown, above) {
                            (false, _) => "",
                            (true, true) => strings.more_above,
                            (true, false) => strings.more_below,
                        };
                        Text::new(mark).style(style.clone()).into()
                    })
                    .into(),
                ),
            )
            .into()
        };
        let picker = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(arrow(true))
            .child(self.options.on_accept(cx.map(Msg::Accept)).view(
                rows,
                OptionStyles {
                    cursor: tokens.selected(),
                    label: Style::new(),
                    description: Style::new(),
                },
                0,
            ))
            .child(arrow(false));
        let [enter, go, esc, skip] = strings.update_hint;
        let hint = Text::rich(vec![
            Span::new(enter),
            Span::styled(go, dim.clone()),
            Span::new(esc),
            Span::styled(skip, dim),
        ]);
        let blank = || SizedBox::new(None, Some(1), None);
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(inset(title.into()))
            .child(inset(notes.into()))
            .child(Flexible {
                fit: FlexFit::Loose,
                ..Flexible::new(1, picker)
            })
            .child(inset(hint.into()))
            .child(blank());
        let panel = BoxWidget::new()
            .style(tokens.prompt_surface(cx, false))
            .child(column);
        let mut focus = Focus::new(panel).autofocus();
        for (id, msg) in [
            (fixed::PICK_PREVIOUS, Msg::Step(-1)),
            (fixed::PICK_NEXT, Msg::Step(1)),
            (fixed::ENTER, Msg::Confirm),
            (fixed::DISMISS, Msg::Choose(UpdateChoice::Skip)),
            (fixed::PICK_QUIT, Msg::Choose(UpdateChoice::Skip)),
        ] {
            focus = bind_fixed(focus, id, cx.send(msg));
        }
        // The first three number keys choose, in the order of the rows.
        for (at, choice) in [
            UpdateChoice::UpdateNow,
            UpdateChoice::Skip,
            UpdateChoice::SkipUntilNextVersion,
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(key) = fixed_presses(fixed::PICK_NUMBERS).into_iter().nth(at) {
                focus = focus.bind(key, cx.send(Msg::Choose(choice)));
            }
        }
        focus.into()
    }
}

/// Whether choices are scrolled out of view above (or below).
fn hidden(scroll: &ScrollController, above: bool) -> bool {
    let Ok(position) = scroll.try_position() else {
        return false;
    };
    if above {
        position.pixels() > 0
    } else {
        position.pixels() < position.max_scroll_extent()
    }
}
