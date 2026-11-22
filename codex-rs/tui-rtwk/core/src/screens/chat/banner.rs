//! Account usage banners as data. The account read is authoritative; sparse
//! rate-limit notifications never replace a banner occurrence.

use codex_protocol::account::PlanType;
use serde::Deserialize;

use crate::ids::Ticket;

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

    pub fn visible_for(&self, model: Option<&str>) -> bool {
        if self.is_reserve() {
            return true;
        }
        match self.blocked_model_slug.as_deref() {
            Some(blocked) if !self.fallback_model_slugs.is_empty() => model.is_some_and(|model| {
                model != blocked && self.fallback_model_slugs.iter().any(|slug| slug == model)
            }),
            _ => self.model_slug.as_deref().is_none_or(|slug| Some(slug) == model),
        }
    }

    pub fn reserve_pending(&self, model: Option<&str>) -> bool {
        self.is_reserve() && model != Some("gpt-reserve")
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
}

impl BannerOccurrence {
    pub(crate) fn new(banner: Banner, ticket: Ticket) -> Self {
        Self { banner, ticket, dismissed: false }
    }

    pub fn is_visible(&self, model: Option<&str>) -> bool {
        !self.dismissed && self.banner.visible_for(model)
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
}
