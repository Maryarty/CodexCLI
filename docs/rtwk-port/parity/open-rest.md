# open-rest: leftovers of open-instant, history, attach conflict

Oracle: `tui/src/app/agents_overview.rs` (the resume at ~l.790, the attach-conflict arm l.808: an active-writer error
reads the thread for viewing, read-only; any other error is `Failed to attach to task: {error}`), `chatwidget/rendering.rs:65`
(`This conversation is open in another app`), snapshot `agents_overview_attach_conflict`,
`app_server_session/rollout_history.rs` (`read_thread_for_viewing`, newest turns page, summary fallback).

| item | rtwk | tests |
|---|---|---|
| 1 Esc cancels the loading page | `Opening` wraps its content in `PopScope.on_pop_invoked_with_result` (the route's back key already pops it); the center sends `Intent::CancelOpen` when the pop comes while `opening` is still set; core keeps `Model.cancelled` and ignores one resume answer per cancel (`apply_answer`, and `screen_answer` does not switch the chat) | `cancelling_the_opening_ignores_the_late_resume_answer`, `esc_on_the_loading_page_returns_to_the_command_center` |
| 2 resume picker opens through the loading page | already so after open-instant (the picker pops itself, the center pushes the page): locked by a test | `a_pick_in_the_resume_picker_shows_the_loading_page` |
| 3 history | an answered resume emits `Command::History` (`thread/turns/list`, newest page first, 20 turns, `itemsView: full`); `screen_answer` turns the page into `ThreadEv::HistoryPage` of the thread's `ThreadModel` (`Transcript::prepend_page`, `history_next` = cursor of the older page); the session page's existing lazy `ListView.builder` shows it | `the_opened_session_shows_its_history`, `an_answered_resume_asks_for_the_history` |
| 4 attach conflict | `is_active_writer` (by message, as the oracle) -> the thread opens read-only: `ThreadModel.read_only`, history paged the same way, the composer is replaced by the oracle's notice lines | `a_thread_with_another_writer_opens_read_only_with_its_history` |
| ADR-003 | a resume of a thread `CodexApp.threads` already holds (not read-only) is no request: `attach_known` shows it again (`agents::attach_loaded`). This replaces the VS1 shortcut where every open resumed the thread again and made a fresh empty `ThreadModel` | `open_a_back_open_b_back_open_a_resumes_each_thread_once` |

Not done (left): paging of OLDER pages on scroll up (the cursor is kept in `ThreadModel.history_next`, no trigger yet);
the transcript still shows only user and agent messages as plain lines (the session page has no cells yet: exec, tools,
markdown are owner chat-live); the read-only view's `r retry`, `f fork` keys and the footer hints (only `←/esc`
is shown, every key shown is bound); events of a background thread are routed by `ThreadId` already, but no test
streams into A while B is shown. Oracle's Esc during the loading frame cannot cancel (it awaits inside the event
handler); the person asked for it. The addendum (every RpcError-to-notice mapping of the center) is not done: only the
resume's `Failed to attach to task:` is. The root cause of the person's second-open conflict was not reproduced with
the fake (a second resume of an already attached thread, now avoided; a real writer elsewhere takes the read-only path).
