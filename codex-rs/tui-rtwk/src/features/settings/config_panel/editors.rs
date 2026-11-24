//! The editors of the panel's rows (`config_panel/render.rs`): the choice lists and the effects
//! switches, drawn in place of the rows in the same sheet.

use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::overlay::close_dialog;
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::settings::{
    ConfigPanel, Edit, SettingId, Value, Which, choice_label,
};

use super::{Act, ConfigState, Msg, strings::Choice};
use crate::app::theme::Tokens;
use crate::widgets::session_list::target;
use rtwk::gestures::{GestureDetector, PointerEvent};

use super::super::strings::Strings;

/// What is editing in place of the rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    Choice(SettingId),
    Effects,
}

/// The dialog a row opens in place of an editor of the panel.
pub(super) fn dialog_of(id: SettingId) -> Option<Which> {
    match id {
        SettingId::Theme => Some(Which::Theme),
        SettingId::StatusLineDisplay => Some(Which::StatusLine),
        SettingId::TerminalTitleDisplay => Some(Which::Title),
        SettingId::KeyboardShortcuts => Some(Which::Keymap),
        _ => None,
    }
}

pub(super) fn is_choice(id: SettingId) -> bool {
    !options(id).is_empty()
}

/// The values a choice row offers, in the order of its strings' items.
fn options(id: SettingId) -> &'static [Option<&'static str>] {
    match id {
        SettingId::NotifyWhen => &[Some("unfocused"), Some("always")],
        SettingId::CopyOnSelect => &[Some("auto"), Some("always"), Some("never")],
        SettingId::RightClickPaste => &[Some("auto"), Some("on"), Some("off")],
        SettingId::SessionPickerLayout => &[Some("dense"), Some("comfortable")],
        SettingId::ResumeDirectory => &[None, Some("current"), Some("session")],
        _ => &[],
    }
}

/// The label of every value every choice row can show: the value column is as wide as the widest.
pub(super) fn choice_labels() -> impl Iterator<Item = String> {
    [
        SettingId::NotifyWhen,
        SettingId::CopyOnSelect,
        SettingId::RightClickPaste,
        SettingId::SessionPickerLayout,
        SettingId::ResumeDirectory,
    ]
    .into_iter()
    .flat_map(|id| options(id).iter().map(move |option| choice_label(id, *option)))
}

fn choice_of(s: &Strings, id: SettingId) -> Option<Choice> {
    match id {
        SettingId::NotifyWhen => Some(s.editor_notify_when),
        SettingId::CopyOnSelect => Some(s.editor_copy_on_select),
        SettingId::RightClickPaste => Some(s.editor_right_click_paste),
        SettingId::SessionPickerLayout => Some(s.editor_session_layout),
        SettingId::ResumeDirectory => Some(s.editor_resume_directory),
        _ => None,
    }
}

impl ConfigState {
    pub(super) fn act_editor(
        &mut self,
        editor: Editor,
        act: Act,
        panel: &ConfigPanel,
        cx: &mut Cx<'_, Self>,
    ) {
        let len = match editor {
            Editor::Choice(id) => options(id).len(),
            Editor::Effects => 2,
        };
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(len, Some(self.pick), &|_| true, mv, len) {
                    self.pick = to;
                }
            }
            Act::Close => self.editor = None,
            Act::Accept => match editor {
                Editor::Choice(id) => {
                    if let Some(choice) = options(id).get(self.pick) {
                        super::super::request(
                            cx,
                            Edit::Chosen(id, choice.map(str::to_string)),
                        );
                    }
                    self.editor = None;
                }
                Editor::Effects => {
                    let (id, on) = if self.pick == 0 {
                        (SettingId::Animations, panel.effects.animations)
                    } else if panel.effects.animations {
                        (SettingId::ComposerStars, panel.effects.composer_stars)
                    } else {
                        return;
                    };
                    super::super::request(cx, Edit::Toggled(id, !on));
                }
            },
            _ => {}
        }
        let _ = close_dialog::<(), ()>;
    }

    pub(super) fn editor_body(
        &self,
        editor: Editor,
        panel: &ConfigPanel,
        s: &'static Strings,
        tokens: &Tokens,
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let gutter = EdgeInsets::symmetric(2, 0);
        let blank = || Text::new(String::new());
        let (title, subtitle, hint, lines) = match editor {
            Editor::Choice(id) => {
                let choice = choice_of(s, id).unwrap_or(s.editor_notify_when);
                let current = match panel.row(id).map(|row| &row.value) {
                    Some(Value::Choice(label)) => label.clone(),
                    _ => String::new(),
                };
                let lines: Vec<(String, bool)> = options(id)
                    .iter()
                    .enumerate()
                    .map(|(i, option)| {
                        let is_current = choice_label(id, *option) == current
                            || (option == &Some("auto") && current.starts_with("Auto"));
                        let name = choice.items[i];
                        let name = if is_current {
                            format!("{name} {}", s.current_marker)
                        } else {
                            name.to_string()
                        };
                        (format!("{}. {name}", i + 1), false)
                    })
                    .collect();
                (choice.title, choice.subtitle, s.popup_hint, lines)
            }
            Editor::Effects => {
                let stars = panel.effects.animations;
                let mark = |on: bool| if on { s.value_on } else { s.value_off };
                let lines = vec![
                    (
                        format!(
                            "{} {}",
                            mark(panel.effects.animations).replace(" On", "").replace(" Off", ""),
                            s.effects_animations
                        ),
                        false,
                    ),
                    (
                        format!(
                            "{} {}",
                            if stars { "[x]" } else { "[-]" },
                            s.effects_stars
                        ),
                        !stars,
                    ),
                ];
                (s.effects_title, s.effects_subtitle, s.effects_hint, lines)
            }
        };
        let mut list = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, (line, dim)) in lines.into_iter().enumerate() {
            let on = index == self.pick;
            let style = if on {
                tokens.selected()
            } else if dim {
                tokens.dim()
            } else {
                Style::new()
            };
            let marker = if on { s.cursor_marker } else { s.indent };
            let tap = cx.map(move |_: PointerEvent| Msg::Row(index));
            list = list.child(
                GestureDetector::new()
                    .on_tap(tap)
                    .child(Text::line(format!("{marker}{line}")).style(style)),
            );
        }
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(Text::line(format!("{}{title}", s.indent)).style(tokens.bold()))
            .child(Padding::new(
                gutter,
                Text::new(subtitle.to_string())
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            ))
            .child(blank())
            .child(blank())
            .child(list)
            .child(blank())
            .child(Padding::new(
                gutter,
                Text::new(hint.to_string()).style(tokens.dim()),
            ))
            .into()
    }
}
