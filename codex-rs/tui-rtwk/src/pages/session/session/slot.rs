//! What stands where the composer is, and the footer under it: the composer with its footer
//! (the shortcuts' close hint, the Esc-primed hint, the reverse search's row, the passive keys),
//! or, in its place, the fork's wait, the provider lock's notice or the read-only notice of a
//! thread another process writes.

use rtwk::AnyWidget;
use rtwk::layout::{EdgeInsets, Padding, SizedBox};
use rtwk::provider::select;
use rtwk::style::Theme;
use rtwk::widgets::Text;
use rtwk::{Cx, Key};

use codex_tui_core::ids::BackendId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::backtrack::Targets;
use codex_tui_core::screens::composer::restricted::Restricted;
use codex_tui_core::threads::composer::{ComposerState, composer_state};

use crate::pages::session::composer::strings as composer_strings;
use crate::pages::session::composer::vim as composer_vim;
use crate::pages::session::composer::{Composer, Placeholder};
use crate::pages::session::external_writer;
use crate::pages::session::footer::{Flash, Footer, Line};
use crate::pages::session::keys::Action;
use crate::pages::session::live::provider_error;
use crate::pages::session::transcript;
use crate::widgets::status_line::{Notice, with_warning_notice};

use super::cloud_wait;
use super::frame::Frame;
use super::{Chat, ChatMsg, ChatState, Presentation};

/// The composer's place and the footer's, and the footer's props where the page's own keys need
/// them (the status rows show the same hints).
pub(super) struct Slot {
    pub composer: AnyWidget,
    pub footer: AnyWidget,
    pub hints: Option<Footer>,
    /// What the empty draft says (the sparkle keeps its dots off the words).
    pub placeholder: &'static str,
    /// The composer's place is the editor of the model's questions (it brings its own surface
    /// and no footer).
    pub editing: bool,
}

pub(super) fn slot(state: &ChatState, w: &Chat, cx: &mut Cx<'_, ChatState>, frame: &Frame) -> Slot {
    let (tokens, thread, chat_strings) = (&frame.tokens, &frame.thread, frame.chat_strings);
    let searching = &frame.searching;
    let mut hints: Option<Footer> = None;
    let mut placeholder = frame.strings.chat.placeholder;
    let editing = w.presentation.focused()
        && !frame.forking
        && frame.provider.is_none()
        && !thread.read_only()
        && frame.run.asking > 0
        && state.editing(frame.run.asking);
    let (composer, footer): (AnyWidget, AnyWidget) = if frame.forking {
        // `bottom_pane_renderable` while `fork_in_progress`
        (
            Padding::new(
                EdgeInsets::only(2, 1, 2, 1),
                Text::new(chat_strings.forking).style(tokens.dim()),
            )
            .into(),
            SizedBox::shrink().into(),
        )
    } else if let Some(lock) = &frame.provider {
        let location = (frame.drawn.owner == BackendId::default())
            .then(|| {
                select::<ScreenStates, Option<String>, _>(cx, |states| {
                    states
                        .status
                        .remote
                        .as_ref()
                        .filter(|remote| !remote.local_daemon)
                        .map(|remote| remote.address.clone())
                })
            })
            .flatten();
        (
            provider_error::notice(
                cx,
                chat_strings,
                tokens,
                lock,
                frame.can_continue,
                location.as_deref(),
                &frame.drawn.status.provider_names,
            ),
            provider_error::footer(cx, chat_strings, tokens, frame.can_continue),
        )
    } else if thread.read_only() {
        (
            external_writer::notice(cx, chat_strings, tokens),
            external_writer::footer(cx, chat_strings, tokens),
        )
    } else if editing && let Some(editor) = state.editor(cx, thread, &w.ui) {
        (editor, SizedBox::shrink().into())
    } else {
        let mut composer = Composer::new(w.ui.clone(), state.node.clone())
            .passive(!w.presentation.focused())
            .on_changed(cx.map(ChatMsg::Changed))
            .on_submitted(cx.map(ChatMsg::Submit))
            .on_picked(cx.map(ChatMsg::Picked))
            // the queue key when it is not Tab (`composer.queue`): Tab is the page's
            .on_queue(cx.send(ChatMsg::QueueDraft))
            .queueing(thread.task_running());
        let closing = select::<ScreenStates, bool, _>(cx, |states| states.chat.facts.shutting_down);
        let held = composer_state(thread, closing);
        match held {
            ComposerState::ParentOwned => composer = composer.worded(Placeholder::ParentOwned),
            ComposerState::ShuttingDown => composer = composer.disabled(Placeholder::ShuttingDown),
            ComposerState::Unavailable => {
                composer = composer.restricted(Restricted::UnavailableThread)
            }
            ComposerState::Open if frame.setting_up => {
                composer = composer.disabled(Placeholder::SetupRunning);
            }
            // a side conversation asks for a follow-up (`set_side_conversation_active`)
            ComposerState::Open if frame.drawn.flags.side_conversation_active => {
                composer = composer.worded(Placeholder::Side);
                placeholder = Placeholder::Side.text(composer_strings::of(cx));
            }
            ComposerState::ReadOnly | ComposerState::Disabled | ComposerState::Open => {}
        }
        if held == ComposerState::Open
            && let Some(mode) = cloud_wait::restricted(&frame.session)
        {
            composer = composer.restricted(mode);
        }
        let footer = if held == ComposerState::ShuttingDown {
            // `show_shutdown_in_progress`: the empty hint override takes the footer away
            SizedBox::shrink().into()
        } else if let Some((query, status)) = searching.clone() {
            // Reverse history search: its row is the footer
            // (`history_search_footer_line`).
            composer = composer.with_history_field(true);
            crate::pages::session::composer::search::footer(
                &w.ui,
                state.node.clone(),
                cx.map(ChatMsg::Changed),
                query,
                status,
            )
        } else if let Some(row) =
            crate::pages::session::composer::search::query_footer(cx, &w.ui.draft)
        {
            // Normal mode's `/` query: its row is the footer too
            row
        } else {
            let line = if state.shortcuts {
                // Open: the footer is the close hint (`close_hint`).
                Line::Close
            } else if w.ui.modes.primed()
                && Targets::of(thread.transcript(), thread.can_load_history()).count > 0
            {
                Line::EscPrimed
            } else {
                Line::Passive
            };
            let props = Footer {
                line,
                // the startup draft has no agents page to go to yet
                agents: !super::super::transcript_header::startup_draft(&frame.session)
                    && composer_vim::agents_key_available(&w.ui.draft, &crate::keymap::keymap(cx)),
                draft_empty: state.draft_empty,
                shell: state.draft_shell,
                vim: w
                    .ui
                    .draft
                    .with_vim(|vim| vim.is_enabled().then(|| vim.mode())),
                // the menu of `/presets` takes the hints away (`set_footer_hint_override`)
                hint_override: state.banner.quick_menu.get().then(Vec::new).or_else(|| {
                    state.startup_waiting(&frame.session).then(|| {
                        vec![
                            (chat_strings.startup_waiting.to_owned(), String::new()),
                            (
                                chat_strings.startup_cancel_key.to_owned(),
                                chat_strings.startup_cancel.to_owned(),
                            ),
                        ]
                    })
                }),
                flash: None,
                on_queue: Some(cx.send(ChatMsg::QueueDraft)),
                on_warnings: Some(cx.send(ChatMsg::Action(Action::Warnings))),
                on_close: Some(cx.send(ChatMsg::Shortcuts)),
            };
            hints = Some(props.clone());
            // `show_footer_flash`: the line the app or the thread raised (`facts::Ev::Flash`)
            // takes the footer's left side until its time is up
            let flash = select::<ScreenStates, Option<Flash>, _>(cx, |states| {
                states.chat.facts.flash().map(Flash::of)
            });
            let footer = cloud_wait::footer(
                Footer { flash, ..props },
                cloud_wait::Hint::of(&frame.session, chat_strings),
            );
            if line == Line::Passive {
                transcript::follow::footer(
                    cx,
                    w.ui.follow.clone(),
                    w.ui.selection.clone(),
                    state.draft_empty && !w.ui.modes.primed(),
                    footer,
                )
            } else {
                footer
            }
        };
        let composer = AnyWidget::from(composer);
        let composer = if w.presentation == Presentation::Solo {
            composer
        } else {
            composer.with_key(Key::value(thread.id().clone()))
        };
        (composer, footer)
    };
    // `⚠ N warnings · f2 to view` at the right of the read-only footer; the
    // composer's footer holds its own.
    let footer = if searching.is_none() && thread.read_only() && !frame.forking {
        let data = Theme::of(cx);
        with_warning_notice(
            footer,
            frame.drawn.warnings,
            Notice {
                badge: chat_strings.warning_badge,
                singular: chat_strings.warning_singular,
                plural: chat_strings.warning_plural,
                key: chat_strings.warning_key,
                view: chat_strings.warning_view,
                accent: data.warning.clone(),
                secondary: tokens.dim(),
                key_style: tokens.bold(),
            },
        )
    } else {
        footer
    };
    Slot {
        composer,
        footer,
        hints,
        placeholder,
        editing,
    }
}
