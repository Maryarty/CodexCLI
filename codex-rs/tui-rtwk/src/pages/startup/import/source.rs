//! The source prompt of the import (oracle: `external_agent_config_migration/
//! source.rs`): the app whose setup is imported, asked when several sources
//! have items. The oracle draws it as a screen of its own at the top left;
//! here it is the startup prompts' look (a `Column(min)` on the prompt surface,
//! the choices the shared `Options` controller: cursor and scroll) with the
//! sources of the model's detection. The keys are a `Focus` with bindings; the
//! answer is the flow's intent (`Source(id)`, `Cancel`). The widget keeps the
//! cursor only (ADR-003).

use std::rc::Rc;

use rtwk::focus::{Focus, KeyBindings};
use rtwk::layout::{
    Align, Alignment, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, FlexFit, Flexible,
    MainAxisSize, Padding, SizedBox,
};
use rtwk::provider::{read, select};
use rtwk::style::{Span, Style};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed::{IMPORT_ACCEPT, IMPORT_CANCEL, PICK_NEXT, PICK_PREVIOUS};
use codex_tui_core::screens::import::{Intent, Source};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::fixed_names;
use crate::l10n::strings as shared;
use crate::widgets::options::{OptionRow, OptionStyles, Options};

use super::labels;
use super::strings;

/// The prompt over the sources the model detected.
#[derive(Clone, Debug, PartialEq)]
pub struct SourcePrompt;

/// A key of the prompt (the oracle's `handle_key`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Up,
    Down,
    /// A digit: the source at that place is taken.
    Number(u8),
    Enter,
    /// Esc, Ctrl+C and Ctrl+D.
    Cancel,
    /// A tap on the selected row or a double tap (ADR-011): the source at
    /// that row is taken, as Enter takes it.
    Accept(usize),
}

/// Widget state: the cursor and the scroll of the choices (R14).
pub struct SourcePromptState {
    options: Options,
}

impl StatefulWidget for SourcePrompt {
    type State = SourcePromptState;

    fn create_state(&self) -> SourcePromptState {
        SourcePromptState {
            options: Options::new(),
        }
    }
}

impl_widget!(stateful eq SourcePrompt);

/// The sources the prompt offers, in the model's order.
fn sources(cx: &mut Cx<'_, SourcePromptState>) -> Vec<Source> {
    select::<ScreenStates, Vec<Source>, _>(cx, |states| {
        states
            .import
            .detected
            .iter()
            .map(|found| found.source)
            .collect()
    })
}

impl State for SourcePromptState {
    type Widget = SourcePrompt;
    type Msg = Pick;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn update(&mut self, key: Pick, _: &SourcePrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let sources = sources(cx);
        let take = |at: usize, cx: &mut Cx<'_, Self>| {
            if let Some(source) = sources.get(at) {
                let hs = read::<Hs, _>(cx);
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Import(Intent::Source(source.id().to_owned())),
                );
            }
        };
        match key {
            // The oracle's highlight stops at the ends.
            Pick::Up => {
                self.options.set(self.options.cursor().saturating_sub(1));
                Rebuild::Yes
            }
            Pick::Down => {
                let last = sources.len().saturating_sub(1);
                self.options.set((self.options.cursor() + 1).min(last));
                Rebuild::Yes
            }
            Pick::Number(number) => {
                if let Some(at) = usize::from(number).checked_sub(1) {
                    take(at, cx);
                }
                Rebuild::No
            }
            Pick::Enter => {
                take(self.options.cursor(), cx);
                Rebuild::No
            }
            Pick::Accept(row) => {
                self.options.set(row);
                take(row, cx);
                Rebuild::No
            }
            Pick::Cancel => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Import(Intent::Cancel));
                Rebuild::No
            }
        }
    }

    fn build(&self, _: &SourcePrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let tokens = Tokens::of(cx);
        let rows: Vec<OptionRow> = sources(cx)
            .into_iter()
            .enumerate()
            .map(|(at, source)| OptionRow {
                label: strings.source_row(at + 1, labels::source(strings, source), &num),
                description: None,
            })
            .collect();
        let blank = || -> AnyWidget { SizedBox::new(None, Some(1), None).into() };
        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::only(2, 0, 0, 0), child).into()
        };
        let dim = tokens.dim();
        let hint = Text::rich(
            strings
                .source_hint
                .iter()
                .enumerate()
                .map(|(at, word)| {
                    if at % 2 == 0 {
                        Span::new(*word)
                    } else {
                        Span::styled(*word, dim.clone())
                    }
                })
                .collect::<Vec<_>>(),
        );
        let choices = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(self.options.on_accept(cx.map(Pick::Accept)).view(
                Rc::new(rows),
                OptionStyles {
                    cursor: tokens.selected(),
                    label: Style::new(),
                    description: Style::new(),
                },
                0,
            ));
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(Text::new(strings.source_title).style(tokens.bold()))
            .child(blank())
            .child(inset(Text::new(strings.source_subtitle).style(dim).into()))
            .child(blank())
            .child(Flexible {
                fit: FlexFit::Loose,
                ..Flexible::new(1, choices)
            })
            .child(blank())
            .child(inset(hint.into()))
            .child(blank());
        let panel = BoxWidget::new()
            .style(tokens.prompt_surface(cx, false))
            .child(column);
        let key = |key: Pick| cx.send(key);
        let mut bindings: Vec<(String, _)> = Vec::new();
        for (id, pick) in [
            (PICK_PREVIOUS, Pick::Up),
            (PICK_NEXT, Pick::Down),
            (IMPORT_ACCEPT, Pick::Enter),
            (IMPORT_CANCEL, Pick::Cancel),
        ] {
            bindings.extend(fixed_names(id).into_iter().map(|name| (name, key(pick))));
        }
        for digit in 1..=9u8 {
            bindings.push((
                DIGITS[usize::from(digit) - 1].to_owned(),
                key(Pick::Number(digit)),
            ));
        }
        KeyBindings::new(
            bindings,
            Focus::new(Align::new(Alignment::TOP_LEFT, panel)).autofocus(),
        )
        .into()
    }
}

const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
