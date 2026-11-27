//! Additional labels and composition for the safety menus.

pub(in crate::pages::session) use crate::oracle_strings::cells::SAFETY_BUFFERING_GIVING_THIS_REQUEST_A_LITTLE_EXTRA as WAITING;
pub(super) use crate::oracle_strings::cells::{
    MISALIGNMENT_POLICY_ACKNOWLEDGE_FINDINGS_AND_CONTINUE as ACKNOWLEDGE,
    MISALIGNMENT_POLICY_CHAT_PAUSED_AS_A_PRECAUTION as PAUSED,
    MISALIGNMENT_POLICY_CHAT_STOPPED_AS_A_PRECAUTION as STOPPED,
    MISALIGNMENT_POLICY_CONTINUATION_REQUEST_QUOTED, MISALIGNMENT_POLICY_NEW_CHAT as NEW,
    MISALIGNMENT_POLICY_RESUME_ANOTHER_CHAT as RESUME,
    MISALIGNMENT_POLICY_WE_COULDN_T_CONFIRM_THE_AGENT as STOPPED_DETAIL,
    MISALIGNMENT_POLICY_WE_COULDN_T_CONFIRM_THE_AGENT_2 as PAUSED_DETAIL,
    MISALIGNMENT_POLICY_WHAT_WE_DETECTED as FINDINGS,
    SAFETY_BUFFERING_DISMISS_AND_KEEP_WAITING as DISMISS,
    SAFETY_BUFFERING_NO_ACTION_IS_REQUIRED_CODEX_WILL as WAIT_FOOTER,
};

pub(super) const LEARN: &str = "Learn more";
pub(super) const LEARN_MORE_URL: &str = "https://help.openai.com/en/articles/20001326";
pub(super) const REVIEW: &str = "Review findings";
pub(super) const BACK: &str = "Back";
pub(super) const AGENTS: &str = "Agent command center";

pub(super) fn quoted(message: &str, findings: &str) -> String {
    format!("{MISALIGNMENT_POLICY_CONTINUATION_REQUEST_QUOTED}\n{message:?}\n\n{findings}")
}
