//! Display values shared by the chat footer and the terminal title.

use codex_protocol::account::PlanType;
use codex_tui_core::features::settings::items::{DEFAULT_TITLE, status_item, title_item};
use codex_tui_core::format::format_tokens_compact;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::chat::{self, DEFAULT_STATUS_LINE_ITEMS, StatusFacts};
use codex_tui_core::screens::status::{
    self, Figure, UsageInterest, format_credit_micros, format_estimated_usd_micros,
};
use rtwk::AnyWidget;

use rtwk_core::unicode::grapheme_boundaries;

use crate::l10n::strings::Shared;
use crate::widgets::interest::Interest;

/// Read-only inputs for the status line and terminal title.
pub(super) struct StatusContext<'a> {
    pub thread_id: Option<&'a ThreadId>,
    pub thread: Option<&'a chat::ThreadPart>,
    pub usage: Option<&'a status::ThreadPart>,
    pub rate_limits: Option<&'a status::RateLimits>,
    pub run_state: Option<&'a str>,
    pub strings: &'a Shared,
    /// The model catalog: the tier the model runs on and its name.
    pub catalog: &'a codex_tui_core::screens::select::models::Catalog,
    /// The app-wide facts: the git summary and the workspace headline the probes found.
    pub chat: &'a chat::facts::Facts,
    /// `/raw`: the transcript is in raw scrollback mode.
    pub raw_output: bool,
    /// The resumed thread has counted no tokens yet (`token_usage_pending`).
    pub usage_pending: bool,
    /// What the backend estimates the thread cost (`estimated_thread_usage`): none until the
    /// first answer.
    pub cost: Option<&'a Figure>,
    /// The plan of the account (`plan_type`): `thread-credits` is not shown on Ent26.
    pub plan: Option<PlanType>,
}

impl StatusContext<'_> {
    pub fn items_for(&self, facts: &StatusFacts) -> Vec<(String, String)> {
        match &facts.items {
            Some(items) => items.iter().map(String::as_str).collect::<Vec<_>>(),
            None => DEFAULT_STATUS_LINE_ITEMS.to_vec(),
        }
        .into_iter()
        .filter_map(status_item)
        .filter_map(|id| self.value_for(id, facts).map(|text| (id.to_string(), text)))
        .collect()
    }

    /// The values a setup previews for `ids` (`status_surface_preview_data`): the live value of an
    /// item (as the title shows it for the title setup); an empty text where the item has none
    /// and its placeholder is left out (a limit once the account's limits are known, a cost the
    /// plan has no figure for).
    pub fn preview_values(
        &self,
        title: bool,
        ids: &[&str],
        facts: &StatusFacts,
    ) -> std::collections::BTreeMap<String, String> {
        ids.iter()
            .filter_map(|id| {
                let value = if title {
                    self.title_value(id, facts)
                } else {
                    self.value_for(id, facts)
                };
                let text = match value {
                    Some(text) => text,
                    None if self.suppressed(id) => String::new(),
                    None => return None,
                };
                Some(((*id).to_string(), text))
            })
            .collect()
    }

    /// `suppress_placeholder`: the item shows nothing rather than its placeholder.
    fn suppressed(&self, id: &str) -> bool {
        match id {
            "five-hour-limit" | "weekly-limit" => self.rate_limits.is_some(),
            "estimated-thread-cost" => self
                .cost
                .is_some_and(|figure| figure.cost_micros(self.plan).is_none()),
            _ => false,
        }
    }

    /// What the item `id` links to: the pull request number, to its page
    /// (`status_line_pull_request_url`).
    pub fn link_for(&self, id: &str) -> Option<String> {
        (id == "pull-request-number")
            .then(|| self.chat.pull_request_url())
            .flatten()
            .map(str::to_owned)
    }

    /// The terminal title (`terminal_title_text_for_selections`): the configured items (or the
    /// default three) that are valid ids, each cut to its length; while the agent waits for the
    /// user the action-required line replaces the spinner and holds the other items.
    pub fn title(&self, facts: &StatusFacts, state: TitleInputs) -> Title {
        let items = title_items(facts);
        if let Some(prefix) = state
            .action_required
            .filter(|_| items.iter().any(|id| is_spinner(id)))
        {
            let parts = items
                .iter()
                .filter(|id| !is_spinner(id) && !matches!(**id, "run-state" | "status"))
                .filter_map(|id| self.title_value(id, facts));
            let text = std::iter::once(prefix.to_string())
                .chain(parts)
                .collect::<Vec<_>>();
            return Title {
                pieces: vec![Piece::Text(text.join(" | "))],
            };
        }
        let pieces = items
            .iter()
            .filter_map(|id| {
                if is_spinner(id) {
                    state.spinner.then_some(Piece::Spinner)
                } else {
                    self.title_value(id, facts).map(Piece::Text)
                }
            })
            .collect();
        Title { pieces }
    }

    /// `terminal_title_value_for_item`: the value of an item as the title shows it.
    fn title_value(&self, id: &str, facts: &StatusFacts) -> Option<String> {
        let value = match id {
            "app-name" => "codex".to_string(),
            "project" | "project-name" => facts.project_root.clone().or_else(|| {
                let cwd = self
                    .thread
                    .and_then(|thread| thread.cwd.as_deref())
                    .or(facts.cwd.as_deref())?;
                cwd.rsplit(['/', '\\'])
                    .find(|part| !part.is_empty())
                    .map(str::to_owned)
            })?,
            "thread" => self.value_for("thread-title", facts)?,
            _ => self.value_on(id, facts, true)?,
        };
        Some(match title_cut(id) {
            Some(max) => cut(value, max),
            None => value,
        })
    }

    fn value_for(&self, id: &str, facts: &StatusFacts) -> Option<String> {
        self.value_on(id, facts, false)
    }

    /// The value of an item on the status line, or (`title`) in the terminal title, which spells
    /// the tier of `model-with-reasoning` as the catalog names it where the line lowercases it.
    fn value_on(&self, id: &str, facts: &StatusFacts, title: bool) -> Option<String> {
        let thread = self.thread;
        let name = thread
            .and_then(|thread| thread.name.as_deref())
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let thread_id = || self.thread_id.map(ToString::to_string);
        let model = thread
            .and_then(|thread| thread.model.as_deref())
            .or_else(|| facts.model_name());
        let slug = model;
        let model = model.map(|model| {
            if model.eq_ignore_ascii_case("gpt-reserve") {
                "Luna Reserve"
            } else {
                model
            }
        });
        // the catalog's name of the model, else the slug (`model_display_name`)
        let model = model.map(|model| {
            self.catalog
                .presets
                .iter()
                .find(|preset| Some(preset.model.as_str()) == slug)
                .map_or(model, |preset| preset.name.as_str())
        });
        // an empty model reads as the default display name (`model_display_name`)
        let model = model.map(|model| {
            if model.is_empty() {
                chat::facts::LOADING_MODEL
            } else {
                model
            }
        });
        let effort = thread
            .and_then(|thread| thread.effort.as_deref())
            .unwrap_or_else(|| facts.effort_label());
        let tier = thread
            .and_then(|thread| thread.service_tier.as_ref())
            .map_or(facts.service_tier.as_deref(), |tier| tier.as_deref());
        let tier = chat::facts::effective_service_tier(facts, tier, slug, self.catalog);
        let tier_name = chat::facts::tier_display_name(
            self.catalog,
            slug,
            tier.as_deref(),
            facts.flex_tier_fallback,
        );
        let usage = self.usage.filter(|usage| usage.received);
        let tokens = usage.map(|usage| usage.tokens);
        let context_usage = usage.and_then(|usage| usage.context);
        // `status_line_context_remaining_percent`: a full window until a count and a window are
        // known; nothing while a resumed thread waits for its first count
        let remaining =
            (!self.usage_pending).then(|| context_usage.map_or(100, |r| r.percent_left));
        let counted = |count: fn(&status::Tokens) -> i64| {
            (!self.usage_pending).then(|| tokens.map_or(0, |t| count(&t)))
        };
        match id {
            "model" | "model-name" => model.map(str::to_owned),
            "model-with-reasoning" => model.map(|model| match tier_name {
                Some(name) if title => format!("{model} {effort} {name}"),
                Some("Fast") => self.strings.status_model_fast(model, effort),
                Some(name) => format!("{model} {effort} {}", name.to_lowercase()),
                None => format!("{model} {effort}"),
            }),
            "fast-mode" => Some(tier_name.unwrap_or("Standard").to_string()),
            // the preference is off until the thread says it is on (`daybreak_enabled` is false by default)
            "daybreak" => Some(
                if thread.and_then(|thread| thread.daybreak).unwrap_or(false) {
                    "Daybreak on"
                } else {
                    "Daybreak off"
                }
                .to_string(),
            ),
            "reasoning" => Some(effort.to_string()),
            "current-dir" => thread
                .and_then(|thread| thread.cwd.as_deref())
                .map(|cwd| display_dir(cwd, facts.home_directory.as_deref()))
                .or_else(|| facts.cwd.clone()),
            "thread-name" => name.map(str::to_owned),
            "thread-title" => name.map(str::to_owned).or_else(thread_id),
            "thread-id" | "session-id" => thread_id(),
            "hostname" => facts.hostname.clone(),
            "profile" => facts
                .profile_name
                .as_ref()
                .map(|name| format_label("profile", name)),
            "provider" if self.thread_id.is_some() => thread
                .and_then(|thread| thread.provider.as_deref())
                .or(facts.provider.as_deref())
                .map(|name| format_label("provider", name)),
            "codex-version" => facts.version.clone(),
            "pull-request-number" => self.chat.git_summary()?.pull_request_text(),
            "branch-changes" => self.chat.git_summary()?.changes_text(),
            "workspace-headline" => self.chat.headline().map(str::to_owned),
            "raw-output" => self.raw_output.then(|| "raw output".to_string()),
            "status" | "run-state" => self.run_state.map(str::to_owned),
            "permissions" => Some(
                match facts.profile {
                    status::Profile::ReadOnly => "Read Only",
                    status::Profile::Workspace => "Workspace Write",
                    status::Profile::Full => "Full Access",
                }
                .to_string(),
            ),
            "context-remaining" => remaining.map(|left| self.strings.status_context(left, false)),
            "context-used" | "context-usage" => {
                remaining.map(|left| self.strings.status_context(left, true))
            }
            "context-window-size" => context_usage.map(|r| format_units(r.window, "window")),
            "used-tokens" => tokens
                .map(|t| t.blended_total())
                .filter(|total| *total > 0)
                .map(|total| format_units(total, "used")),
            "total-input-tokens" => counted(|t| t.input).map(|n| format_units(n, "in")),
            "total-output-tokens" => counted(|t| t.output).map(|n| format_units(n, "out")),
            "five-hour-limit" => limit_text(self.strings, five_hour_window(self.rate_limits?)),
            "weekly-limit" => limit_text(self.strings, weekly_window(self.rate_limits?)),
            "thread-credits" => self
                .cost
                .filter(|_| self.plan != Some(PlanType::Ent26))
                .map(|figure| {
                    self.strings
                        .status_credits(&format_credit_micros(figure.credits_micros()))
                }),
            "estimated-thread-cost" => self
                .cost
                .and_then(|figure| figure.cost_micros(self.plan))
                .and_then(format_estimated_usd_micros),
            "task-progress" => thread
                .and_then(|thread| thread.progress)
                .filter(|&(_, total)| total > 0)
                .map(|(done, total)| self.strings.status_progress(done, total)),
            _ => facts.value(id),
        }
    }
}

/// What the title is made of while the page draws it.
pub(super) struct TitleInputs<'a> {
    /// The spinner has a frame (animations on, a turn or a compaction running).
    pub spinner: bool,
    /// The agent waits for the user: the line that says so (`[ ! ] Action Required`).
    pub action_required: Option<&'a str>,
}

/// The title's pieces in order: the spinner takes its frame from the animation.
#[derive(Clone, PartialEq)]
pub(super) struct Title {
    pieces: Vec<Piece>,
}

#[derive(Clone, PartialEq)]
enum Piece {
    Spinner,
    Text(String),
}

impl Title {
    /// A spinner slot to animate.
    pub fn has_spinner(&self) -> bool {
        self.pieces.contains(&Piece::Spinner)
    }

    /// `separator_from_previous`: a space beside the spinner, ` | ` between the others. A
    /// spinner without a `frame` has no value and leaves no separator.
    pub fn text(&self, frame: Option<&str>) -> String {
        let mut out = String::new();
        let mut previous: Option<bool> = None;
        for piece in &self.pieces {
            let (spinner, value) = match (piece, frame) {
                (Piece::Spinner, Some(frame)) => (true, frame),
                (Piece::Spinner, None) => continue,
                (Piece::Text(value), _) => (false, value.as_str()),
            };
            if let Some(before) = previous {
                out.push_str(if before || spinner { " " } else { " | " });
            }
            out.push_str(value);
            previous = Some(spinner);
        }
        out
    }
}

/// What the items of the status line ask of the thread's usage: a figure that is configured is
/// wanted whether or not it shows yet (`thread_usage_selected`).
pub(super) fn line_interests(facts: &StatusFacts) -> Vec<UsageInterest> {
    match &facts.items {
        Some(items) => interests_of(items.iter().map(String::as_str)),
        None => interests_of(DEFAULT_STATUS_LINE_ITEMS),
    }
}

/// The same for the items of the terminal title.
pub(super) fn title_interests(facts: &StatusFacts) -> Vec<UsageInterest> {
    interests_of(title_items(facts))
}

fn interests_of<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<UsageInterest> {
    let mut wanted = Vec::new();
    for id in ids {
        let interest = match id {
            "thread-credits" => UsageInterest::Credits,
            "estimated-thread-cost" => UsageInterest::EstimatedCost,
            _ => continue,
        };
        if !wanted.contains(&interest) {
            wanted.push(interest);
        }
    }
    wanted
}

/// `child`, for as long as it is drawn, wants the usage of the thread the `interests` name.
pub(super) fn raise(interests: Vec<UsageInterest>, child: AnyWidget) -> AnyWidget {
    interests.into_iter().fold(child, |child, interest| {
        Interest::new(interest, child).into()
    })
}

fn is_spinner(id: &str) -> bool {
    matches!(id, "activity" | "spinner")
}

/// `configured_terminal_title_items` that parse (`DEFAULT_TERMINAL_TITLE_ITEMS` when unset).
fn title_items(facts: &StatusFacts) -> Vec<&str> {
    match &facts.terminal_title {
        Some(items) => items.iter().filter_map(|id| title_item(id)).collect(),
        None => DEFAULT_TITLE.to_vec(),
    }
}

/// The most graphemes the title takes of an item (`terminal_title_value_for_item`).
fn title_cut(id: &str) -> Option<usize> {
    match id {
        "app-name" | "activity" | "spinner" | "run-state" | "status" | "daybreak"
        | "task-progress" => None,
        "project" | "project-name" => Some(24),
        "thread" | "thread-name" | "thread-title" => Some(48),
        _ => Some(32),
    }
}

/// `truncate_terminal_title_part`: at most `max` graphemes, the last three dots when it is cut.
fn cut(value: String, max: usize) -> String {
    let Some((head, _)) = grapheme_boundaries(&value).nth(max) else {
        return value;
    };
    if max <= 3 {
        return value[..head].to_string();
    }
    let keep = grapheme_boundaries(&value)
        .nth(max - 3)
        .map_or(head, |(start, _)| start);
    format!("{}...", &value[..keep])
}

fn format_label(label: &str, value: &str) -> String {
    format!("{label}: {value}")
}

fn format_units(value: i64, units: &str) -> String {
    format!("{} {units}", format_tokens_compact(value))
}

/// `is_approximate_window`: within five percent of the span.
fn is_approximate_window(minutes: i64, expected: i64) -> bool {
    let (minutes, expected) = (minutes as f64, expected as f64);
    minutes >= expected * 0.95 && minutes <= expected * 1.05
}

/// `get_limits_duration`: what a window of that many minutes is called.
fn limit_duration(minutes: i64) -> Option<&'static str> {
    const HOUR: i64 = 60;
    const DAY: i64 = 24 * HOUR;
    let minutes = minutes.max(0);
    [
        (5 * HOUR, "5h"),
        (DAY, "daily"),
        (7 * DAY, "weekly"),
        (30 * DAY, "monthly"),
        (365 * DAY, "annual"),
    ]
    .into_iter()
    .find(|(span, _)| is_approximate_window(minutes, *span))
    .map(|(_, name)| name)
}

/// `limit_label_for_window`: the window's name, else the fallback of its slot.
fn limit_label(window: &status::RateWindow, secondary: bool) -> &'static str {
    window
        .window_minutes
        .and_then(limit_duration)
        .unwrap_or(if secondary {
            "secondary usage"
        } else {
            "usage"
        })
}

fn has_label(window: &status::RateWindow, label: &str) -> bool {
    window.window_minutes.and_then(limit_duration) == Some(label)
}

type Slot<'a> = Option<(&'a status::RateWindow, bool)>;

/// `find_codex_window`: the primary or else the secondary window of that name.
fn find_window<'a>(limits: &'a status::RateLimits, label: &str) -> Slot<'a> {
    limits
        .primary
        .as_ref()
        .filter(|window| has_label(window, label))
        .map(|window| (window, false))
        .or_else(|| {
            limits
                .secondary
                .as_ref()
                .filter(|window| has_label(window, label))
                .map(|window| (window, true))
        })
}

/// `five_hour_status_window`: the primary window when it is the 5h one; the secondary when it is
/// and a weekly one exists; else the primary when it is not the weekly one; else the secondary
/// under a weekly primary when it is not weekly too.
fn five_hour_window(limits: &status::RateLimits) -> Slot<'_> {
    let weekly = |window: &status::RateWindow| has_label(window, "weekly");
    limits
        .primary
        .as_ref()
        .filter(|window| has_label(window, "5h"))
        .map(|window| (window, false))
        .or_else(|| {
            find_window(limits, "weekly")?;
            limits
                .secondary
                .as_ref()
                .filter(|window| has_label(window, "5h"))
                .map(|window| (window, true))
        })
        .or_else(|| {
            limits
                .primary
                .as_ref()
                .filter(|window| !weekly(window))
                .map(|window| (window, false))
        })
        .or_else(|| {
            limits.primary.as_ref().filter(|window| weekly(window))?;
            limits
                .secondary
                .as_ref()
                .filter(|window| !weekly(window))
                .map(|window| (window, true))
        })
}

/// `weekly_status_window`: the weekly window, else the secondary one.
fn weekly_window(limits: &status::RateLimits) -> Slot<'_> {
    find_window(limits, "weekly").or_else(|| limits.secondary.as_ref().map(|window| (window, true)))
}

/// `status_line_limit_display`'s words: the window's name and what is left of it.
fn limit_text(strings: &Shared, slot: Slot<'_>) -> Option<String> {
    let (window, secondary) = slot?;
    let left = (100 - window.used_percent.clamp(0, 100)) as u64;
    Some(strings.status_quota(limit_label(window, secondary), left))
}

pub(super) fn display_dir(cwd: &str, home: Option<&str>) -> String {
    let cwd = std::path::Path::new(cwd);
    match home.and_then(|home| cwd.strip_prefix(home).ok()) {
        Some(path) if path.as_os_str().is_empty() => "~".to_string(),
        Some(path) => format!("~{}{}", std::path::MAIN_SEPARATOR, path.display()),
        None => cwd.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title(pieces: Vec<Piece>) -> Title {
        Title { pieces }
    }

    fn text(value: &str) -> Piece {
        Piece::Text(value.to_string())
    }

    /// `separator_from_previous`: a space beside the spinner, ` | ` between the others.
    #[test]
    fn the_title_joins_its_items_as_the_oracle_does() {
        let t = title(vec![Piece::Spinner, text("name"), text("project")]);
        assert_eq!(t.text(Some("⠋")), "⠋ name | project");
        // no frame: no value, and no separator for it
        assert_eq!(t.text(None), "name | project");
        let t = title(vec![text("name"), Piece::Spinner, text("project")]);
        assert_eq!(t.text(Some("⠙")), "name ⠙ project");
        assert_eq!(t.text(None), "name | project");
        assert_eq!(title(vec![Piece::Spinner]).text(None), "");
    }

    /// `truncate_terminal_title_part`: the last three of `max` graphemes are dots when it is cut.
    #[test]
    fn a_title_part_is_cut_to_its_graphemes_with_dots() {
        assert_eq!(cut("abcdef".to_string(), 6), "abcdef");
        assert_eq!(cut("abcdefg".to_string(), 6), "abc...");
        assert_eq!(cut("abcdefg".to_string(), 3), "abc");
        assert_eq!(cut("日本語の文字列".to_string(), 5), "日本...");
        let id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        assert_eq!(cut(id, 32), "550e8400-e29b-41d4-a716-44665...");
    }

    #[test]
    fn only_the_ids_of_the_oracle_are_title_items() {
        let facts = StatusFacts {
            terminal_title: Some(vec![
                "project".into(),
                "no-such-item".into(),
                "status".into(),
            ]),
            ..StatusFacts::default()
        };
        assert_eq!(title_items(&facts), ["project", "status"]);
        assert_eq!(
            title_items(&StatusFacts::default()),
            ["activity", "thread-name", "project-name"]
        );
    }
}
