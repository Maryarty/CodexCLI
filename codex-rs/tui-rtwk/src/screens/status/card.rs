//! The card: the oracle's `content_lines` (`status/card.rs:756`) as rows of a
//! dialog, the labels padded to one column.

use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::screens::ScreenIntent;
use codex_tui_rtwk_core::screens::ScreenStates;
use codex_tui_rtwk_core::screens::status::{Approval, Card, Intent, Profile};

use super::format;
use super::strings::{self, Strings};
use crate::theme;
use crate::ui::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct CardDialog;

pub struct CardDialogState {
    shown: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
}

impl StatefulWidget for CardDialog {
    type State = CardDialogState;

    fn create_state(&self) -> CardDialogState {
        CardDialogState { shown: false }
    }
}

impl_widget!(stateful eq CardDialog);

impl State for CardDialogState {
    type Widget = CardDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &CardDialog, cx: &mut Cx<'_, Self>) {
        let present = select::<ScreenStates, bool, _>(cx, |states| states.status.card.is_some());
        if !present && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
    }

    fn update(&mut self, msg: Msg, _: &CardDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Close = msg;
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Status(Intent::Close));
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, _: &CardDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let card = select::<ScreenStates, Option<Card>, _>(cx, |states| states.status.card.clone());
        let Some(card) = &card else {
            return AnyWidget::default();
        };
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .child(Text::new(s.header(&card.version).to_string()).style(tokens.bold()))
            .child(Text::new(String::new()))
            .child(Text::new(s.usage_link_1).style(tokens.accent()))
            .child(Text::new(s.usage_link_2).style(tokens.accent()))
            .child(Text::new(String::new()));
        for line in lines(card, s) {
            column = column.child(Text::new(line));
        }
        Focus::new(Dialog::new(column).max_width(Some(84)).title(s.command))
            .can_request_focus(false)
            .bind(rtwk::focus::KeyPress::ESC, cx.send(Msg::Close))
            .into()
    }
}

/// The labelled rows, the labels padded to the longest (`FieldFormatter`).
fn lines(card: &Card, s: &Strings) -> Vec<String> {
    let mut rows: Vec<(String, String)> = vec![
        (s.model.into(), model_value(card, s)),
        (s.directory.into(), card.directory.clone()),
        (s.permissions.into(), permissions_value(card, s)),
        (
            s.agents.into(),
            if card.agents.is_empty() {
                s.agents_none.to_string()
            } else {
                card.agents.join(", ")
            },
        ),
    ];
    let blank_before = [rows.len()];
    let tokens = &card.tokens;
    rows.push((
        s.token_usage.into(),
        s.tokens_total(
            &format::tokens_compact(tokens.blended_total()),
            &format::tokens_compact((tokens.input - tokens.cached_input).max(0)),
            &format::tokens_compact(tokens.output),
        )
        .to_string(),
    ));
    if let Some(context) = &card.context {
        rows.push((
            s.context_window.into(),
            s.context_value(
                context.percent_left,
                &format::tokens_compact(context.used),
                &format::tokens_compact(context.window),
            )
            .to_string(),
        ));
    }
    for limit in &card.limits {
        let label = if limit.window_minutes >= 10_080 {
            s.limit_weekly.to_string()
        } else {
            s.limit_hours(limit.window_minutes / 60).to_string()
        };
        let left = 100 - limit.used_percent;
        let resets = limit
            .reset
            .map(|reset| {
                let (hour, pm) = format::twelve_hour(reset.hour);
                let minute = format!("{:02}", reset.minute);
                s.resets(&hour.to_string(), &minute, if pm { s.pm } else { s.am })
                    .to_string()
            })
            .unwrap_or_default();
        rows.push((
            label,
            s.limit_value(&format::progress_bar(left), left, &resets)
                .to_string(),
        ));
    }
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (index, (label, value)) in rows.iter().enumerate() {
        if blank_before.contains(&index) {
            lines.push(String::new());
        }
        let pad = " ".repeat(width + 2 - label.chars().count());
        lines.push(format!("{label}{pad}{value}"));
    }
    lines
}

fn model_value(card: &Card, s: &Strings) -> String {
    let details: Vec<String> = [
        card.reasoning_effort
            .as_deref()
            .map(|effort| s.reasoning(effort).to_string()),
        card.reasoning_summary
            .as_deref()
            .map(|summary| s.summaries(summary).to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();
    if details.is_empty() {
        card.model.clone()
    } else {
        s.model_value(&card.model, &details.join(s.details_separator))
            .to_string()
    }
}

fn permissions_value(card: &Card, s: &Strings) -> String {
    let profile = match card.profile {
        Profile::ReadOnly => s.profile_read_only,
        Profile::Workspace => s.profile_workspace,
        Profile::Full => s.profile_full,
    };
    let approval = match (card.approval, card.auto_review) {
        (Approval::OnRequest, true) => s.approval_auto,
        (Approval::OnRequest, false) => s.approval_ask,
        (Approval::Never, _) => s.approval_never,
        (Approval::OnFailure, _) => s.approval_on_failure,
        (Approval::UnlessTrusted, _) => s.approval_untrusted,
    };
    s.permissions_value(profile, approval).to_string()
}
