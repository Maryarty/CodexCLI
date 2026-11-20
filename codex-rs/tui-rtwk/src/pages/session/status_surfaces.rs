//! Display values shared by the chat footer and the terminal title.

use codex_tui_core::format::format_tokens_compact;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::chat::{self, DEFAULT_STATUS_LINE_ITEMS, StatusFacts};
use codex_tui_core::screens::status;

use crate::l10n::strings::Shared;

/// Read-only inputs for the status line and terminal title.
pub(super) struct StatusContext<'a> {
    pub thread_id: Option<&'a ThreadId>,
    pub thread: Option<&'a chat::ThreadPart>,
    pub usage: Option<&'a status::ThreadPart>,
    pub rate_limits: Option<&'a status::RateLimits>,
    pub run_state: Option<&'a str>,
    pub strings: &'a Shared,
}

impl StatusContext<'_> {
    pub fn items_for(&self, facts: &StatusFacts) -> Vec<(String, String)> {
        match &facts.items {
            Some(items) => items.iter().map(String::as_str).collect::<Vec<_>>(),
            None => DEFAULT_STATUS_LINE_ITEMS.to_vec(),
        }
        .into_iter()
        .filter_map(|id| self.value_for(id, facts).map(|text| (id.to_string(), text)))
        .collect()
    }

    pub fn title_parts(&self, facts: &StatusFacts, activity: Option<&str>) -> Vec<String> {
        const DEFAULT: [&str; 3] = ["activity", "thread-name", "project-name"];
        match &facts.terminal_title {
            Some(items) => items.iter().map(String::as_str).collect::<Vec<_>>(),
            None => DEFAULT.to_vec(),
        }
        .into_iter()
        .filter_map(|id| match id {
            "activity" | "spinner" => activity.map(str::to_owned),
            "app-name" => Some("codex".to_string()),
            "project" | "project-name" => facts.project_root.clone().or_else(|| {
                let cwd = self
                    .thread
                    .and_then(|thread| thread.cwd.as_deref())
                    .or(facts.cwd.as_deref())?;
                cwd.rsplit(['/', '\\'])
                    .find(|part| !part.is_empty())
                    .map(str::to_owned)
            }),
            "thread" => self.value_for("thread-title", facts),
            _ => self.value_for(id, facts),
        })
        .collect()
    }

    pub fn title_has_activity(facts: &StatusFacts) -> bool {
        facts.terminal_title.as_ref().is_none_or(|items| {
            items
                .iter()
                .any(|item| matches!(item.as_str(), "activity" | "spinner"))
        })
    }

    fn value_for(&self, id: &str, facts: &StatusFacts) -> Option<String> {
        let thread = self.thread;
        let name = thread
            .and_then(|thread| thread.name.as_deref())
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let thread_id = || self.thread_id.map(ToString::to_string);
        let model = thread
            .and_then(|thread| thread.model.as_deref())
            .or_else(|| facts.model_name());
        let model = model.map(|model| {
            if model.eq_ignore_ascii_case("gpt-reserve") {
                "Luna Reserve"
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
        let fast = tier == Some("priority");
        let usage = self.usage.filter(|usage| usage.received);
        let tokens = usage.map(|usage| usage.tokens);
        let context_usage = usage.and_then(|usage| usage.context);
        match id {
            "model" | "model-name" => model.map(str::to_owned),
            "model-with-reasoning" => model.map(|model| {
                if fast {
                    self.strings.status_model_fast(model, effort)
                } else {
                    format!("{model} {effort}")
                }
            }),
            "fast-mode" => Some(if fast { "Fast" } else { "Standard" }.to_string()),
            "daybreak" => thread.and_then(|thread| thread.daybreak).map(|enabled| {
                if enabled {
                    "Daybreak on"
                } else {
                    "Daybreak off"
                }
                .to_string()
            }),
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
            "status" | "run-state" => self.run_state.map(str::to_owned),
            "permissions" => Some(
                match facts.profile {
                    status::Profile::ReadOnly => "Read Only",
                    status::Profile::Workspace => "Workspace Write",
                    status::Profile::Full => "Full Access",
                }
                .to_string(),
            ),
            "context-remaining" => {
                context_usage.map(|r| self.strings.status_context(r.percent_left, false))
            }
            "context-used" | "context-usage" => {
                context_usage.map(|r| self.strings.status_context(r.percent_left, true))
            }
            "context-window-size" => context_usage.map(|r| format_units(r.window, "window")),
            "used-tokens" => tokens
                .map(|t| t.blended_total())
                .filter(|total| *total > 0)
                .map(|total| format_units(total, "used")),
            "total-input-tokens" => tokens.map(|t| format_units(t.input, "in")),
            "total-output-tokens" => tokens.map(|t| format_units(t.output, "out")),
            "five-hour-limit" => percent_remaining(self.rate_limits?, 300)
                .map(|percent| self.strings.status_quota("5h", percent)),
            "weekly-limit" => percent_remaining(self.rate_limits?, 7 * 24 * 60)
                .map(|percent| self.strings.status_quota("Weekly", percent)),
            "task-progress" => thread
                .and_then(|thread| thread.progress)
                .filter(|&(_, total)| total > 0)
                .map(|(done, total)| self.strings.status_progress(done, total)),
            _ => facts.value(id),
        }
    }
}

fn format_label(label: &str, value: &str) -> String {
    format!("{label}: {value}")
}

fn format_units(value: i64, units: &str) -> String {
    format!("{} {units}", format_tokens_compact(value))
}

fn percent_remaining(limits: &status::RateLimits, duration: i64) -> Option<u64> {
    let limit = limits
        .primary
        .iter()
        .chain(&limits.secondary)
        .find(|window| window.window_minutes == Some(duration))?;
    Some((100 - limit.used_percent.clamp(0, 100)) as u64)
}

fn display_dir(cwd: &str, home: Option<&str>) -> String {
    let cwd = std::path::Path::new(cwd);
    match home.and_then(|home| cwd.strip_prefix(home).ok()) {
        Some(path) if path.as_os_str().is_empty() => "~".to_string(),
        Some(path) => format!("~{}{}", std::path::MAIN_SEPARATOR, path.display()),
        None => cwd.display().to_string(),
    }
}
