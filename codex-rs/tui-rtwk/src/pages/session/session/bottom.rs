//! The bottom pane (`bottom_pane_renderable`): everything below the transcript, which gets the
//! rows left. The status row, the queued messages and the notices are widgets of their own
//! (`activity`, `pending_input`, `banners`) that read what they show from the providers; this
//! stacks them with the shortcut overlay, the backend control, the composer's place (its gap,
//! the `@`/`$` popup, the sparkle) and the footer, and lets the Luna Reserve selector take the
//! pane over.

use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Row, SizedBox,
};
use rtwk::provider::select;
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key};

use codex_tui_core::app::Session;
use codex_tui_core::screens::ScreenStates;

use crate::pages::session::activity::StatusSlot;
use crate::pages::session::backtrack::EditWatch;
use crate::pages::session::banners::{BackendBanners, NoticeLine, ReserveSelector, reserve_of};
use crate::pages::session::footer::{Sparkle, StatusRows};
use crate::pages::session::live::limits;
use crate::pages::session::popovers::mention;
use crate::pages::session::status::QuestionSummary;
use crate::pages::session::status_surfaces::{line_interests, raise};
use crate::pages::session::transcript;
use crate::sheets::approvals::PendingThreadsBanner;
use crate::sheets::dialogs::shortcut_panel;
use crate::sheets::select::hud;
use crate::widgets::status_line::{SEPARATOR, status_spans, thread_color};

use codex_tui_core::screens::startup::Action;

use super::frame::Frame;
use super::slot::Slot;
use super::{BannerBlocked, Chat, ChatState};

pub(super) fn body(
    state: &ChatState,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    frame: &Frame,
    slot: Slot,
) -> Column {
    let (tokens, thread, session) = (&frame.tokens, &frame.thread, &frame.session);
    let (run, facts, searching) = (&frame.run, &frame.drawn.status, &frame.searching);
    let running = run.running();
    let startup = super::super::transcript_header::startup_draft(session);
    let (panel, _) = shortcut_panel(cx, running);
    let chat_strings = frame.chat_strings;
    let Slot {
        composer,
        footer,
        hints,
        placeholder,
        editing,
    } = slot;
    // Whether `StatusSlot` shows a row: the reader's gap then sets itself
    // apart from it with a blank row.
    let status_above = !run.read_only
        && (run.word(session, frame.reconnecting) != "Ready"
            || run.has_activity_rows()
            || (matches!(session.status, Session::Starting) && !startup));
    // The reader's gap above the composer is a flexible row, the first thing a short window clips.
    // The menu of `/presets` stands over the whole pane (`above_composer`); where it is closed a
    // blank child keeps the others' places, so their state is not lost.
    let menu = state.banner.quick_menu.get();
    let mut body = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_size(MainAxisSize::Min)
        .overflow_intended(true)
        .child(if menu {
            hud::view(cx)
        } else {
            SizedBox::shrink().into()
        })
        .child(StatusSlot {
            hook: run.hook.clone(),
        })
        // `  ? N questions`: the questions that wait while the editor is closed
        .child(if run.asking > 0 && !editing {
            AnyWidget::from(QuestionSummary {
                count: run.asking,
                countdown: super::questions::countdown(cx, thread.id()),
            })
        } else {
            SizedBox::shrink().into()
        })
        .child(NoticeLine)
        .child(EditWatch);
    if state.shortcuts && searching.is_none() {
        body = body.child(panel);
    }
    let context = frame.context();
    let thread_color = thread_color(
        cx,
        session
            .active
            .as_ref()
            .map(codex_tui_core::ids::ThreadId::as_str),
    );
    let mut line = status_spans(
        context.items_for(facts),
        facts.colors,
        tokens,
        thread_color,
        |id| context.link_for(id),
    );
    // the viewed agent joins the status line (`passive_footer_status_line`); with the line
    // off, the footer's hints carry it (`Kind::AgentName`)
    if facts.items.as_ref().is_none_or(|items| !items.is_empty())
        && let Some(label) = select::<ScreenStates, Option<String>, _>(cx, |states| {
            states.chat.facts.agent_label.clone()
        })
    {
        if !line.is_empty() {
            line.push(Span::styled(SEPARATOR, tokens.dim()));
        }
        line.push(Span::styled(label, tokens.dim()));
    }
    // The backend control above the composer (`rendering.rs`,
    // `visible_new_backend_control`): the displayed backend, and the key.
    let control = session
        .cycle_label(thread, frame.drawn.offer)
        .filter(|_| searching.is_none() && !state.shortcuts && !menu);
    if let Some(label) = control {
        body = body.child(
            Row::new()
                .child(SizedBox::new(Some(2), None, None))
                .child(Text::new(chat_strings.backend(label)))
                .child(SizedBox::new(Some(2), None, None))
                .child(Text::new(chat_strings.backend_key))
                .child(Text::new(chat_strings.backend_cycle).style(tokens.dim())),
        );
    }
    state.banner_blocked.set(BannerBlocked {
        running,
        overlay: state.shortcuts,
        edit: !editing
            && !menu
            && !state.shortcuts
            && (run.asking > 0
                || !thread.queue().is_empty()
                || !thread.rejected_steers().is_empty()),
    });
    // the threads that wait for an approval, right above the composer (`bottom_pane`)
    let body = body.child(PendingThreadsBanner);
    let body = body.child(BackendBanners {
        gate: state.banner.clone(),
        dismissed: state.banner.dismissed.get(),
    });
    let mut body = if thread.read_only() || frame.provider.is_some() {
        body.child(composer)
    } else {
        // the row the reader's control takes (`composer_gap`); a popup
        // above the draft takes the rows before it does
        let body = body.child({
            // the usage notice takes the row while the reader is at the end (`composer_hint`)
            let model = thread.screen_slots().chat.model.as_deref().or(frame
                .drawn
                .status
                .model
                .as_deref());
            let notice = limits::notice_of(session, frame.drawn.owner, model).map(AnyWidget::from);
            let gap = match (startup, session.launching) {
                // the named session opening: its line stands in the reader's gap
                (true, Some(action)) => Text::new(match action {
                    Action::Resume => chat_strings.resuming_session,
                    Action::Fork => chat_strings.forking_session,
                })
                .style(tokens.dim())
                .into(),
                _ => transcript::follow::gap(cx, w.ui.follow.clone(), status_above, notice),
            };
            let mentions = w.ui.draft.mentions();
            ListenableBuilder::new(mentions.listenable(), move |_, _| {
                // the editor's own surface leaves the blank row (`menu_surface_inset`)
                if editing || mentions.is_open() {
                    SizedBox::shrink().into()
                } else {
                    gap.clone()
                }
            })
        });
        // the editor of the model's questions stands in the composer's place
        if editing {
            body.child(composer)
        } else {
            // the `@`/`$` popup stands right above the draft and takes the
            // rows the draft leaves, 17 at most (`ActivePopup::MentionV2`
            // above the composer: the draft first, then what remains)
            body.child(mention::slot(w.ui.clone())).child(
                // keyed: the blank rows and the status come and go above it
                // and the footer's type changes below, and an unkeyed child
                // between two changed neighbours is a new element (its state,
                // and the messages queued for it, lost)
                AnyWidget::from(Sparkle {
                    ui: w.ui.clone(),
                    focus: state.node.clone(),
                    eligible: thread.screen_slots().chat.sparkle_eligible(facts),
                    // the footer's overlay and search row, and a turn that runs,
                    // take the stage (`render_sparkle_at`)
                    hidden: state.shortcuts || running || searching.is_some(),
                    placeholder,
                    child: BoxWidget::new()
                        .style(tokens.prompt_surface(cx, false))
                        .padding(EdgeInsets::only(0, 1, 1, 1))
                        .child(composer)
                        .into(),
                })
                .with_key(Key::value("composer")),
            )
        }
    };
    // a status line that names the credits or the cost wants them, whether or not it shows them
    // yet (`thread_usage_selected`): without a figure the line is empty
    let wanted = line_interests(facts);
    let rows = (!line.is_empty()
        && !startup
        && !thread.read_only()
        && frame.provider.is_none()
        && !editing)
        .then(|| {
            AnyWidget::from(StatusRows {
                spans: line,
                composer: w.ui.composer.clone(),
                hints,
            })
        });
    body = match (rows, wanted.is_empty()) {
        (Some(rows), true) => body.child(rows),
        (rows, false) => body.child(raise(
            wanted,
            rows.unwrap_or_else(|| SizedBox::new(Some(0), Some(0), None).into()),
        )),
        (None, true) => body,
    };
    // The find bar's slot (`find_footer::slot`): while Find is open its query row and status row
    // stand where the footer does, below the composer; closed, the footer the page would draw.
    body.child(transcript::find_footer::slot(cx, &w.ui.find, footer))
}

/// The terminal title keeps being told while the selector takes the bottom pane; the Luna
/// Reserve selector owns the pane (`show_selection_view`): no composer, no footer, the draft
/// waits in `UiApp.ui`.
pub(super) fn with_titles(frame: &Frame, body: Column, titles: Vec<AnyWidget>) -> Column {
    let model = frame.drawn.status.model.as_deref();
    let mut body = if reserve_of(&frame.session, frame.drawn.owner, model, &frame.thread).is_some()
    {
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .child(ReserveSelector)
    } else {
        body
    };
    for title in titles {
        body = body.child(title);
    }
    body
}
