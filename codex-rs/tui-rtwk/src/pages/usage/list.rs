//! What the list keys do on the surface in front (oracle: `analytics.rs`
//! `handle_key` after the controls, `summary_action`, `plan_action`): the
//! help and the Overview scroll, the dashboard focuses a card, a report moves
//! its cursor and opens its details. The cursors and scroll positions are
//! the widget's (R14); the one thing that reaches the model is closing.

use rtwk::Cx;
use rtwk::navigation::Navigator;
use rtwk::provider::read;
use rtwk::scroll::ScrollController;

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::usage::chats::ChatStep;
use codex_tui_core::screens::usage::plan::PlanStep;
use codex_tui_core::screens::usage::{Intent as UsageIntent, Model, Section};

use super::keys::ListAct;
use super::{DASHBOARD_SCROLL, HELP_SCROLL, ScreenState, report};
use crate::app::view::Hs;

/// Moves a scroll surface by what a list action asks (oracle:
/// `summary_action`): a row, a page, the top or the bottom.
pub(super) fn scroll(controller: &ScrollController, act: ListAct) {
    let Ok(position) = controller.try_position() else {
        return;
    };
    let view = position.viewport_dimension().max(1);
    let at = position.pixels();
    let target = match act {
        ListAct::Up => at - 1,
        ListAct::Down => at + 1,
        ListAct::PageUp => at - view,
        ListAct::PageDown => at + view,
        ListAct::Top => 0,
        ListAct::Bottom => i64::MAX,
        _ => return,
    };
    controller.jump_to(target.max(0));
}

impl ScreenState {
    /// Closes the page (oracle: `is_done`).
    fn close(cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Leave));
        Navigator::maybe_pop(cx);
    }

    /// The list action `act` on the surface in front; the intent it makes of
    /// the model, if any.
    pub(super) fn list(
        &mut self,
        act: ListAct,
        model: &Model,
        section: Section,
        cx: &mut Cx<'_, Self>,
    ) -> Option<UsageIntent> {
        if self.help {
            if act == ListAct::Cancel {
                self.help = false;
            } else {
                scroll(&self.scrolls[HELP_SCROLL], act);
            }
            return None;
        }
        if !self.zoomed && act == ListAct::Accept {
            self.zoomed = true;
            return None;
        }
        if model.visible_sections().is_empty() {
            if act == ListAct::Cancel {
                Self::close(cx);
            }
            return None;
        }
        if !self.zoomed && act == ListAct::Cancel {
            Self::close(cx);
            return None;
        }
        let index = section as usize;
        if section == Section::Summary {
            if act == ListAct::Cancel {
                Self::close(cx);
            } else {
                scroll(&self.scrolls[self.surface(section)], act);
            }
            return None;
        }
        match act {
            ListAct::PageUp | ListAct::PageDown => {
                scroll(&self.scrolls[self.surface(section)], act);
                return None;
            }
            _ if section == Section::Plan => {
                let step = match act {
                    ListAct::Up => PlanStep::Up,
                    ListAct::Down => PlanStep::Down,
                    ListAct::Left | ListAct::Right => PlanStep::Switch,
                    ListAct::Top => PlanStep::Top,
                    ListAct::Bottom => PlanStep::Bottom,
                    ListAct::Accept => PlanStep::Toggle,
                    ListAct::Cancel | ListAct::PageUp | ListAct::PageDown => PlanStep::Back,
                };
                if self
                    .plan
                    .step(step, model.plan.ready().map(|report| &**report))
                {
                    Self::close(cx);
                }
                return None;
            }
            _ if section == Section::Chats => {
                let step = match act {
                    ListAct::Up => ChatStep::Up,
                    ListAct::Down => ChatStep::Down,
                    ListAct::Top => ChatStep::Top,
                    ListAct::Bottom => ChatStep::Bottom,
                    ListAct::Accept | ListAct::Right => ChatStep::Toggle,
                    ListAct::Left => ChatStep::Collapse,
                    ListAct::Cancel | ListAct::PageUp | ListAct::PageDown => ChatStep::Back,
                };
                let has_details = model.chat_has_details(&self.chat);
                if self.chat.step(step, model.chat_row_count(), has_details) {
                    Self::close(cx);
                }
                return None;
            }
            ListAct::Cancel => {
                if self.details[index].take().is_none() {
                    Self::close(cx);
                }
                return None;
            }
            _ => {}
        }
        if report::is_chart(section) {
            self.chart(act, model, section);
        }
        None
    }

    /// The controller of the surface that shows `section`: its report, or
    /// the dashboard while the cards are in front.
    pub(super) fn surface(&self, section: Section) -> usize {
        if self.zoomed {
            section as usize
        } else {
            DASHBOARD_SCROLL
        }
    }

    /// A chart report: the selected day and the open detail.
    fn chart(&mut self, act: ListAct, model: &Model, section: Section) {
        let index = section as usize;
        let last = model.day_count(section).max(1) - 1;
        let cursor = self.cursors[index];
        match act {
            ListAct::Up | ListAct::Left => self.cursors[index] = cursor.saturating_sub(1),
            ListAct::Down | ListAct::Right => self.cursors[index] = (cursor + 1).min(last),
            ListAct::Top => self.cursors[index] = 0,
            ListAct::Bottom => self.cursors[index] = last,
            ListAct::Accept => {
                // The counted reports have nothing to open on an empty day.
                let opens = !matches!(
                    section,
                    Section::Plugins | Section::Activity | Section::Skills
                ) || model.day_has_details(section, cursor);
                if opens {
                    let detail = &mut self.details[index];
                    *detail = if *detail == Some(cursor) {
                        None
                    } else {
                        Some(cursor)
                    };
                }
            }
            ListAct::Cancel | ListAct::PageUp | ListAct::PageDown => {}
        }
        // An open detail follows the selected day.
        if self.details[index].is_some() {
            self.details[index] = Some(self.cursors[index]);
        }
    }
}
