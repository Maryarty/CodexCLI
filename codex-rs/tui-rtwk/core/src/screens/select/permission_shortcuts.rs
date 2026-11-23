//! The permission shortcuts (oracle `chatwidget/permission_shortcuts.rs`): the next and previous
//! built-in mode over the catalog the server listed. Read Only, Ask for approval and Approve for me
//! make the cycle; a mode the requirements forbid is skipped, and the one the thread is in is
//! never the answer.

use super::permissions::{Current, Discovery, Label, Preset, Reviewer, Selection};

/// `auto_review_required_for_model`: whether `models` (the requirements' `requiredOnModels`)
/// name `model`, or its alias under one provider namespace (`provider/model`).
pub fn auto_review_required(models: &[String], model: &str) -> bool {
    let model = match model.split_once('/') {
        Some((namespace, suffix))
            if !namespace.is_empty()
                && !suffix.contains('/')
                && namespace
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
        {
            suffix
        }
        Some(_) => return false,
        None => model,
    };
    models.iter().any(|name| name == model)
}

/// The modes of the cycle in the oracle's order, each with whether the thread is in it.
pub fn choices(current: &Current, discovery: &Discovery) -> Vec<(bool, Selection)> {
    let required = discovery
        .requirements
        .as_ref()
        .and_then(|requirements| requirements.auto_required.as_deref())
        .zip(current.model.as_deref())
        .is_some_and(|(models, model)| auto_review_required(models, model));
    let mut choices = Vec::new();
    for preset in [Preset::ReadOnly, Preset::Auto] {
        for reviewer in [Reviewer::User, Reviewer::AutoReview] {
            if reviewer == Reviewer::AutoReview && (preset != Preset::Auto || !current.guardian) {
                continue;
            }
            let approval = preset.approval();
            if discovery
                .disabled(preset.profile(), Some(approval), Some(reviewer))
                .is_some()
                || (required && reviewer != Reviewer::AutoReview)
            {
                continue;
            }
            let here = current.approval == Some(approval)
                && current.reviewer == reviewer
                && current
                    .profile
                    .as_deref()
                    .map_or(current.matches == Some(preset), |active| active == preset.profile());
            choices.push((
                here,
                Selection::Profile {
                    id: preset.profile().to_owned(),
                    approval: Some(approval),
                    reviewer: Some(reviewer),
                    label: Label::Mode { preset, reviewer },
                    unchanged: false,
                },
            ));
        }
    }
    choices
}

/// The mode after (or before) the one the thread is in: the first one that is not the current,
/// from the one after it, around the cycle. None when no other mode is allowed.
pub fn next(current: &Current, discovery: &Discovery, forward: bool) -> Option<Selection> {
    let mut choices = choices(current, discovery);
    if !forward {
        choices.reverse();
    }
    let start = choices
        .iter()
        .position(|(here, _)| *here)
        .map_or(0, |index| index + 1);
    let count = choices.len();
    choices
        .into_iter()
        .cycle()
        .skip(start)
        .take(count)
        .find(|(here, _)| !here)
        .map(|(_, selection)| selection)
}
