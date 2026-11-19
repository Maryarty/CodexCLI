# open-instant: opening a session from the Agent command center

Oracle: `tui/src/app/agents_overview_loading.rs` (`draw`: bold `Loading task…` at the top left of an otherwise empty
screen, drawn at once while the selection waits for the session to load); test `agents_overview_tests.rs:2314`
(snapshot `Loading task…`).

| rtwk | what |
|---|---|
| core `screens/agents` `Model.opening` | the task being opened: set by `Accept`/`Resume`, cleared by the resume's answer (success or failure) |
| `pages/agents/opening.rs` | the loading page (`PageRoute`), `Navigator.push` over the center when `opening` appears; popped when it clears; a success also pops the center (the chat shows), a failure leaves the center as it was with the error notice |
| tests `tests/services/shell_agents.rs` | `enter_shows_the_session_page_before_the_resume_reply` (oracle frame, resume unanswered), `the_resume_reply_fills_the_session_page`, `a_failed_resume_returns_to_the_command_center_with_the_error`; core `opening_a_task_is_marked_until_the_resume_answers` |

Deltas: none (the loading frame equals the oracle's). The picker's resume (`o`) is unchanged: it pushes no loading page
(the center is not the current route then).

S12 (N=15, release, interleaved, load 5-11): Enter -> first frame (command center gone, page shown), median ms:
oracle 0.5 (its `Loading task…`), rtwk before 45.7 (N=7, same machine, the page changed at the resume reply), rtwk after
0.5 (max 21.5, one outlier of 15). Enter -> composer: oracle 89, rtwk after 35.6.
