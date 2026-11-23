//! Account usage banners as data. The account read is authoritative; sparse
//! rate-limit notifications never replace a banner occurrence.

use codex_protocol::account::PlanType;
use serde::Deserialize;

use crate::ids::Ticket;
use crate::screens::select::models::{Preset, RESERVE_MODEL};

/// What the account read says of the Luna Reserve (`update_backend_banner`): whether ordinary
/// usage is available again (the reserve-only `/model` ends) and the normal model the reserve
/// quota stands for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reserve {
    pub recovered: bool,
    pub normal_model: Option<String>,
    /// The account the read was for (`backend_banner_state.account_id`): a return target saved
    /// for another account is not followed.
    pub account_id: Option<String>,
}

impl Reserve {
    /// A full, identity-validated read can authorize recovery; percentages, sparse
    /// notifications and reset timestamps cannot prove it.
    pub fn of(response: &codex_app_server_protocol::GetAccountRateLimitsResponse) -> Self {
        let credits = response
            .rate_limits
            .credits
            .as_ref()
            .is_some_and(|credits| credits.unlimited || credits.has_credits);
        let recovered = response.ordinary_usage_allowed.is_some()
            && (response.ordinary_usage_allowed == Some(true) || credits)
            && response.rate_limit_upsell.is_none()
            && response.rate_limits.spend_control_reached != Some(true)
            && response.rate_limits.rate_limit_reached_type.is_none();
        let normal_model = response
            .rate_limits_by_limit_id
            .iter()
            .flatten()
            .map(|(_, snapshot)| snapshot)
            .find(|snapshot| snapshot.limit_name.as_deref() == Some(RESERVE_MODEL))
            .and_then(|snapshot| snapshot.normal_model_slug.clone());
        Self { recovered, normal_model, account_id: response.account_id.clone() }
    }
}

/// Why the app changed the model by itself (`AutomaticModelSwitchReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchReason {
    /// The banner blocks the model in use.
    UsageLimit,
    /// Luna Reserve was in use and ordinary usage is available again.
    UsageRecovered,
}

/// What the note of an automatic switch says: the model it landed on, the effort word after it
/// (none for the `codex-auto-*` models) and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomaticSwitch {
    pub model: String,
    pub effort: Option<String>,
    pub reason: SwitchReason,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Banner {
    banner_type: String,
    title: String,
    description: String,
    ctas: Vec<Cta>,
    reset_at: Option<i64>,
    model_slug: Option<String>,
    blocked_model_slug: Option<String>,
    #[serde(default)]
    fallback_model_slugs: Vec<String>,
    #[serde(default)]
    presentation: Presentation,
    request_url: Option<String>,
    #[serde(skip)]
    account_id: Option<String>,
    #[serde(skip)]
    plan_type: Option<PlanType>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Presentation {
    #[default]
    Inline,
    Dismissible,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Cta {
    action: String,
    label: String,
}

impl Banner {
    pub(crate) fn parse(
        raw: serde_json::Value,
        account_id: Option<String>,
        plan_type: Option<PlanType>,
    ) -> Option<Self> {
        let mut banner: Self = serde_json::from_value(raw).ok()?;
        let bounded_slug = |value: &str| {
            !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
        };
        if banner.title.trim().is_empty()
            || banner.title.len() > 1024
            || banner.description.len() > 4096
            || banner.title.lines().count() > 3
            || banner.description.lines().count() > 12
            || banner.title.chars().chain(banner.description.chars())
                .any(|ch| ch.is_control() && ch != '\n')
            || banner.ctas.len() > 8
            || banner.fallback_model_slugs.len() > 16
            || !banner.fallback_model_slugs.iter().all(|slug| bounded_slug(slug))
            || !banner.blocked_model_slug.as_deref().is_none_or(bounded_slug)
            || !banner.model_slug.as_deref().is_none_or(bounded_slug)
        {
            return None;
        }
        banner.account_id = account_id;
        banner.plan_type = plan_type;
        Some(banner)
    }

    pub(crate) fn same_occurrence(&self, next: &Self) -> bool {
        self.account_id == next.account_id
            && self.banner_type == next.banner_type
            && self.reset_at == next.reset_at
            && self.presentation == next.presentation
            && self.blocked_model_slug.as_ref().or(self.model_slug.as_ref())
                == next.blocked_model_slug.as_ref().or(next.model_slug.as_ref())
            && self.fallback_model_slugs == next.fallback_model_slugs
    }

    /// Whether the banner applies to the model in use (`refresh_backend_banner_visibility`).
    /// `notice` is the model an automatic switch just settled on: the copy describes that
    /// replacement, so the banner stays for it.
    pub fn visible_for(&self, model: Option<&str>, notice: Option<&str>) -> bool {
        if self.is_reserve() {
            return true;
        }
        match self.blocked_model_slug.as_deref() {
            // Explicit fallback payloads describe the selected replacement, not a pending
            // switch.
            Some(blocked) if !self.fallback_model_slugs.is_empty() => model.is_some_and(|model| {
                model != blocked && self.fallback_model_slugs.iter().any(|slug| slug == model)
            }),
            _ => {
                self.model_slug.as_deref().is_none_or(|slug| Some(slug) == model)
                    || (notice.is_some() && notice == model)
            }
        }
    }

    /// Whether a switch away from `current` is wanted at all (the cheap half of
    /// `backend_banner_fallback`, before the catalog is asked): the Reserve banner wants one off
    /// an ordinary model, a blocked-model banner one off the blocked model.
    pub fn wants_switch(&self, current: &str) -> bool {
        if self.is_reserve() {
            current != RESERVE_MODEL
                && self.blocked_model_slug.as_deref().is_none_or(|blocked| blocked == current)
        } else {
            self.blocked_model_slug.as_deref() == Some(current)
                && !self.fallback_model_slugs.is_empty()
        }
    }

    /// The model an automatic switch lands on (`backend_banner_fallback`): the first fallback
    /// the picker lists and the catalog has, or Luna Reserve for its banner (hidden from the
    /// picker on purpose).
    pub fn fallback_target<'a>(&self, current: &str, catalog: &'a [Preset]) -> Option<&'a Preset> {
        if !self.wants_switch(current) {
            return None;
        }
        if self.is_reserve() {
            return catalog.iter().find(|preset| preset.model == RESERVE_MODEL);
        }
        self.fallback_model_slugs.iter().find_map(|candidate| {
            catalog.iter().find(|preset| {
                preset.show_in_picker && preset.model == *candidate && preset.model != current
            })
        })
    }

    pub fn reserve_pending(&self, model: Option<&str>) -> bool {
        self.is_reserve() && model != Some(RESERVE_MODEL)
    }

    /// The account the banner was read for.
    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    /// The Luna Reserve banner: shown as a focused selector, not as the inline banner
    /// (`LUNA_RESERVE_BANNER`).
    pub fn is_reserve(&self) -> bool {
        self.banner_type == "luna_reserve"
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// The banner's actions in the order the backend gave them (oracle
    /// `BackendBanner::actionable_banner`): a cta whose label is empty, longer
    /// than 256 bytes or holds a control character, or whose action this
    /// client cannot do, is left out.
    pub fn ctas(&self) -> Vec<BannerCta> {
        self.ctas
            .iter()
            .filter_map(|cta| {
                let label = cta.label.as_str();
                if label.trim().is_empty() || label.len() > 256 || label.chars().any(char::is_control) {
                    return None;
                }
                Some(BannerCta {
                    label: label.to_owned(),
                    action: self.resolve(&cta.action)?,
                })
            })
            .collect()
    }

    /// When the limit resets (unix seconds): the copy's `{time}`.
    pub fn reset_at(&self) -> Option<i64> {
        self.reset_at
    }

    /// Oracle `BackendBanner::resolve_action`.
    fn resolve(&self, action: &str) -> Option<BannerAction> {
        match action {
            "notify_owner" | "contact_owner" => return Some(BannerAction::NotifyOwner(Nudge::Credits)),
            "request_increase" => {
                let Some(destination) = self.request_url.as_deref() else {
                    return Some(BannerAction::NotifyOwner(Nudge::UsageLimit));
                };
                return web_url(destination).map(BannerAction::OpenUrl);
            }
            // the existing picker and its confirmation come before a reset is consumed
            "reset_usage" => return Some(BannerAction::ResetUsage),
            _ => {}
        }
        self.destination(action).map(BannerAction::OpenUrl)
    }

    fn destination(&self, action: &str) -> Option<String> {
        let destination = match action {
            "add_credits" | "buy_credits" if self.plan_type.is_some_and(PlanType::is_workspace_account) => {
                "https://chatgpt.com/admin/billing?codex_credit_action=add_credits"
            }
            "add_credits" | "buy_credits" => "https://chatgpt.com/settings/usage?credits_modal=true",
            "buy_reset" => "https://chatgpt.com/codex/purchase/reset",
            "view_usage" | "request_increase_usage_settings" => "https://chatgpt.com/settings/usage",
            "view_workspace_usage" | "increase_spend_cap" => "https://chatgpt.com/admin/usage-limits/workspace",
            "open_plus_pricing_web" => "https://chatgpt.com/explore/plus",
            "open_pro_pricing_web" => "https://chatgpt.com/explore/pro",
            "open_pricing_dialog" => match self.plan_type {
                Some(PlanType::ProLite) => "https://chatgpt.com/?cta_tab=personal&highlight_plan=pro&pro_variant=2x#pricing",
                Some(PlanType::Plus) => "https://chatgpt.com/?cta_tab=personal&highlight_plan=pro#pricing",
                _ => "https://chatgpt.com/?cta_tab=personal&highlight_plan=plus#pricing",
            },
            _ => return None,
        };
        if destination.starts_with("https://chatgpt.com/admin/") {
            let id = self.account_id.as_deref().filter(|id| {
                !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })?;
            let query = if destination.contains('?') { '&' } else { '?' };
            Some([destination, &query.to_string(), "account_id=", id].concat())
        } else {
            Some(destination.to_string())
        }
    }
}

/// What a banner action does (oracle `BannerAction`): the browser opens a
/// page, the owner is told by e-mail, or the reset-credit flow opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerAction {
    OpenUrl(String),
    NotifyOwner(Nudge),
    ResetUsage,
}

/// What the owner is asked for (`AddCreditsNudgeCreditType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    Credits,
    UsageLimit,
}

/// How the add-credits nudge ended (`finish_add_credits_nudge_email_request`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NudgeOutcome {
    /// The e-mail went out.
    Sent,
    /// The owner was told recently (`cooldown_active`).
    Cooldown,
    /// The request failed.
    Failed,
}

/// One numbered row of a banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerCta {
    pub label: String,
    pub action: BannerAction,
}

/// `request_increase`'s own page: an http(s) URL with a host and no
/// credentials (oracle `resolve_action`).
fn web_url(destination: &str) -> Option<String> {
    let rest = destination
        .strip_prefix("https://")
        .or_else(|| destination.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    (!authority.is_empty() && !authority.contains('@') && !destination.chars().any(char::is_whitespace))
        .then(|| destination.to_owned())
}

/// A generation minted by the reducer ties a dismissal to the shown occurrence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerOccurrence {
    pub banner: Banner,
    pub ticket: Ticket,
    dismissed: bool,
    /// The model an automatic switch settled on (`backend_banner_notice_model`): the banner
    /// shows for it whatever its own model says, until the occurrence is replaced.
    notice_model: Option<String>,
}

impl BannerOccurrence {
    pub(crate) fn new(banner: Banner, ticket: Ticket) -> Self {
        Self { banner, ticket, dismissed: false, notice_model: None }
    }

    pub fn is_visible(&self, model: Option<&str>) -> bool {
        !self.dismissed && self.banner.visible_for(model, self.notice_model.as_deref())
    }

    /// A failed switch into Luna Reserve still needs the recovery actions
    /// (`show_unavailable_reserve_recovery`): the banner is shown again.
    pub(crate) fn reopen(&mut self) {
        self.dismissed = false;
    }

    /// An automatic switch to `model` was accepted (`finish_backend_banner_fallback`): the
    /// post-switch notice is distinct from having seen the blocked state, so the banner is
    /// shown again for the new model.
    pub(crate) fn after_switch(&mut self, model: &str) {
        self.dismissed = false;
        self.notice_model = Some(model.to_owned());
    }

    pub fn is_dismissible(&self) -> bool {
        self.banner.presentation == Presentation::Dismissible
    }

    /// The user dismissed the occurrence (Esc on a dismissible banner; Esc or a pick in the Luna
    /// selector, whatever its presentation: `picker_dismissed`).
    pub(crate) fn dismiss(&mut self, ticket: Ticket) {
        if self.ticket == ticket && (self.is_dismissible() || self.banner.is_reserve()) {
            self.dismissed = true;
        }
    }

    /// A submit starts a turn: a dismissible banner the user was shown (visible
    /// for the model in use) is dismissed, any other stays (oracle
    /// `dismiss_backend_banner_for_new_turn`; a model that hides it does not
    /// dismiss it).
    pub(crate) fn dismiss_for_new_turn(&mut self, model: Option<&str>) {
        if self.is_dismissible() && self.is_visible(model) {
            self.dismissed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ids::TicketMint;

    #[test]
    fn banner_dismissal_is_bound_to_its_occurrence() {
        let payload = json!({
            "banner_type": "credits",
            "title": "Usage limit",
            "description": "Add credits.",
            "ctas": [{"action": "view_usage", "label": "Usage"}],
            "presentation": "dismissible",
            "reset_at": 42
        });
        let Some(banner) = Banner::parse(payload.clone(), Some("account-A".to_string()), None) else {
            panic!("valid banner");
        };
        let Some(other_account) = Banner::parse(payload, Some("account-B".to_string()), None) else {
            panic!("valid banner");
        };
        assert!(!banner.same_occurrence(&other_account));
        assert_eq!(
            banner.ctas(),
            [BannerCta {
                label: "Usage".to_string(),
                action: BannerAction::OpenUrl("https://chatgpt.com/settings/usage".to_string()),
            }]
        );
        let mut mint = TicketMint::default();
        let ticket = mint.next();
        let mut occurrence = BannerOccurrence::new(banner, ticket);
        occurrence.dismiss(mint.next());
        assert!(occurrence.is_visible(None));
        occurrence.dismiss(ticket);
        assert!(!occurrence.is_visible(None));
    }

    fn banner(extra: serde_json::Value) -> Banner {
        let mut payload = json!({
            "banner_type": "selected_model_limit", "title": "Usage limit",
            "description": "Switch.", "ctas": [], "model_slug": "test-model-a",
        });
        payload.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        Banner::parse(payload, Some("workspace-a".to_string()), None).expect("valid banner")
    }

    fn preset(model: &str, show_in_picker: bool) -> Preset {
        serde_json::from_value(json!({
            "model": model, "name": model, "show_in_picker": show_in_picker,
        }))
        .expect("preset")
    }

    /// Oracle `backend_banner_fallback_candidates_and_notice_follow_selected_model`: the
    /// candidate list skips what the picker hides and what the catalog lacks, and only a banner
    /// that blocks the model in use asks for a switch.
    #[test]
    fn the_fallback_is_the_first_listed_candidate_of_the_catalog() {
        let blocked = banner(json!({
            "blocked_model_slug": "test-model-a",
            "fallback_model_slugs": ["hidden-model", "absent", "test-model-c", "test-model-b"],
        }));
        let catalog = [
            preset("hidden-model", false),
            preset("test-model-b", true),
            preset("test-model-c", true),
        ];
        let target = blocked.fallback_target("test-model-a", &catalog);
        assert_eq!(target.map(|preset| preset.model.as_str()), Some("test-model-c"));
        assert!(blocked.wants_switch("test-model-a"));
        // another model in use is not blocked; the current model is never its own fallback
        assert!(blocked.fallback_target("test-model-b", &catalog).is_none());
        assert!(!blocked.wants_switch("test-model-b"));
        assert!(blocked.fallback_target("test-model-a", &catalog[..1]).is_none());
        // a banner without fallbacks (or without a blocked model) has nowhere to go
        assert!(!banner(json!({"blocked_model_slug": "test-model-a"})).wants_switch("test-model-a"));
        assert!(!banner(json!({"fallback_model_slugs": ["test-model-c"]})).wants_switch("test-model-a"));
    }

    /// Luna Reserve is the target of its banner though the picker hides it, off any model the
    /// banner does not exclude, and never from itself.
    #[test]
    fn the_reserve_banner_targets_the_reserve_model() {
        let reserve = banner(json!({"banner_type": "luna_reserve", "model_slug": null}));
        let catalog = [preset("test-model-a", true), preset(RESERVE_MODEL, false)];
        assert_eq!(
            reserve.fallback_target("test-model-a", &catalog).map(|preset| preset.model.as_str()),
            Some(RESERVE_MODEL)
        );
        assert!(!reserve.wants_switch(RESERVE_MODEL));
        let blocked = banner(json!({"banner_type": "luna_reserve", "blocked_model_slug": "test-model-b"}));
        assert!(!blocked.wants_switch("test-model-a"));
        assert!(blocked.wants_switch("test-model-b"));
    }

    /// After a switch the banner is shown again for the replacement the copy describes, unless
    /// the occurrence was replaced; other models keep hiding it.
    #[test]
    fn a_switch_keeps_the_banner_for_the_model_it_settled_on() {
        let mut mint = TicketMint::default();
        let mut occurrence = BannerOccurrence::new(banner(json!({})), mint.next());
        assert!(occurrence.is_visible(Some("test-model-a")));
        assert!(!occurrence.is_visible(Some("test-model-b")));
        occurrence.ticket = mint.next();
        occurrence.dismiss(occurrence.ticket);
        occurrence.after_switch("test-model-b");
        assert!(occurrence.is_visible(Some("test-model-b")));
        assert!(occurrence.is_visible(Some("test-model-a")));
        assert!(!occurrence.is_visible(Some("unrelated-model")));
        assert!(!occurrence.is_visible(None));
    }
}
