//! `/permissions <profile> [--approval-policy=<policy>] [--reviewer=user|auto_review]` (oracle
//! `chatwidget/permission_discovery.rs`, `permissions_menu.rs` `try_select_permission_profile`):
//! the profile is picked without a menu. The arguments are parsed on their own; the profile is
//! resolved against the catalog the server lists (the built-in modes `:ask-for-approval` and
//! `:approve-for-me` are `:workspace` with a reviewer) and either becomes the pick of the menus or
//! is refused with a typed reason. While the catalog is still being read the arguments wait in
//! `Profiles::inline`; a retry after a failed read keeps them. The words of every refusal are the
//! view's (`strings.rs`).

use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;

use super::permissions::{Approval, Current, Disabled, Discovery, Label, Preset, Reviewer, Selection};
use crate::commands::args::{self, ArgError, Spec};

/// The built-in mode names `/permissions` accepts besides the catalog's profile ids.
pub const ASK_FOR_APPROVAL: &str = ":ask-for-approval";
pub const APPROVE_FOR_ME: &str = ":approve-for-me";

/// What the command line names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub profile: String,
    pub approval: Option<Approval>,
    pub reviewer: Option<Reviewer>,
}

/// Why the arguments are not a selection (`parse_permission_profile_args`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The tokens: the quoting, or a setting that does not fit.
    Args(ArgError),
    /// `Usage: /permissions <profile-id> [--approval-policy=<policy>] [--reviewer=user|auto_review]`
    Usage,
    /// `Unexpected permission argument `{argument}`.`
    Unexpected(String),
    /// `Unknown approval policy `{value}`. Expected `untrusted`, `on-request`, `never`, or a
    /// granular policy.`
    Approval(String),
    /// `Unknown permission reviewer `{value}`. Expected `user` or `auto_review`.`
    Reviewer(String),
}

const SETTINGS: Spec<'static> = Spec {
    values: &["--approval-policy", "--reviewer"],
    switches: &[],
    booleans: &[],
};

fn approval(value: &str) -> Result<Approval, Error> {
    match value {
        "untrusted" => Ok(Approval::UnlessTrusted),
        "on-request" => Ok(Approval::OnRequest),
        "never" => Ok(Approval::Never),
        value => serde_json::from_str(value).map_err(|_| Error::Approval(value.to_owned())),
    }
}

fn reviewer(value: &str) -> Result<Reviewer, Error> {
    match value {
        "user" => Ok(Reviewer::User),
        "auto_review" => Ok(Reviewer::AutoReview),
        value => Err(Error::Reviewer(value.to_owned())),
    }
}

/// The profile and settings of the command line.
pub fn parse(line: &str) -> Result<Args, Error> {
    let parsed = args::parse(line, SETTINGS).map_err(Error::Args)?;
    let profile = parsed.positionals.first().cloned().ok_or(Error::Usage)?;
    if let Some(extra) = parsed.positionals.get(1) {
        return Err(Error::Unexpected(extra.clone()));
    }
    Ok(Args {
        profile,
        approval: parsed.value("--approval-policy").map(approval).transpose()?,
        reviewer: parsed.value("--reviewer").map(reviewer).transpose()?,
    })
}

/// Why a named profile is not picked (`try_select_permission_profile`'s error lines).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// `Approve for me is unavailable. Run `/permissions` to see available profiles.`
    ApproveUnavailable,
    /// `Unknown permission profile `{id}`. Run `/permissions` to see available profiles.`
    Unknown(String),
    /// `Permission profile `{profile}` is unavailable: {reason}.`
    Unavailable { profile: String, why: Disabled },
}

/// The pick the arguments make over `discovery`; `guardian` is `Feature::GuardianApproval`.
pub fn resolve(args: &Args, discovery: &Discovery, guardian: bool) -> Result<Selection, Refusal> {
    let by = match args.profile.as_str() {
        APPROVE_FOR_ME if !guardian => return Err(Refusal::ApproveUnavailable),
        APPROVE_FOR_ME => Reviewer::AutoReview,
        _ => Reviewer::User,
    };
    let id = match args.profile.as_str() {
        ASK_FOR_APPROVAL | APPROVE_FOR_ME => BUILT_IN_PERMISSION_PROFILE_WORKSPACE,
        id => id,
    };
    if !discovery.profiles.iter().any(|profile| profile.id == id) {
        return Err(Refusal::Unknown(id.to_owned()));
    }
    let preset = Preset::of_profile(id);
    let approval = args.approval.or(preset.map(Preset::approval));
    let reviewer = args.reviewer.or(preset.map(|_| by));
    if reviewer == Some(Reviewer::AutoReview) && !guardian {
        return Err(Refusal::ApproveUnavailable);
    }
    if let Some(why) = discovery.disabled(id, approval, reviewer) {
        return Err(Refusal::Unavailable { profile: id.to_owned(), why });
    }
    let label = match preset {
        Some(preset) => Label::Mode { preset, reviewer: reviewer.unwrap_or(Reviewer::User) },
        None => Label::Profile(id.to_owned()),
    };
    Ok(Selection::Profile { id: id.to_owned(), approval, reviewer, label, unchanged: false })
}

/// An inline pick that has not settled (`InlinePermissionProfileSelection`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// The catalog is being read: the arguments (and what the thread had when the command ran)
    /// wait for it.
    Discovering { args: Args, current: Current },
    /// The pick is made and waits on the full-access confirmation.
    Applying,
}
