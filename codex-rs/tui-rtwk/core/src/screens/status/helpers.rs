//! The pure helpers of the `/status` card, carried from the oracle's
//! `status/helpers.rs` as small functions of their own (reuse-map.md rows for
//! `status/`). Every word the card shows is the view's string table's: these
//! return typed values (a plan, a reset time) or the data of the session (a
//! path), and take `now`, the time zone and the home directory as facts.

use std::path::Path;

use codex_protocol::account::PlanType;

/// The label a subscription plan shows under (oracle `plan_type_display_name`,
/// `subscription.rs` `SubscriptionDisplay::Status`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanLabel {
    Free,
    Go,
    Plus,
    Pro200,
    Pro100,
    Pro500,
    Business,
    BusinessPremium,
    Enterprise,
    EnterpriseAutomation,
    Edu,
    EduPlus,
    EduPro,
    AcademicResearchers,
    AcademicResearchersPlus,
    Unknown,
}

// oracle: ../status/helpers.rs fn plan_type_display_name (adapted: typed, no prose)
pub fn plan_label(plan: PlanType) -> PlanLabel {
    match plan {
        PlanType::Free => PlanLabel::Free,
        PlanType::Go => PlanLabel::Go,
        PlanType::Plus => PlanLabel::Plus,
        PlanType::Pro => PlanLabel::Pro200,
        PlanType::ProLite => PlanLabel::Pro100,
        PlanType::ProMax => PlanLabel::Pro500,
        PlanType::Team | PlanType::SelfServeBusinessUsageBased => PlanLabel::Business,
        PlanType::Business
        | PlanType::Enterprise
        | PlanType::Ent26
        | PlanType::EnterpriseCbpUsageBased => PlanLabel::Enterprise,
        PlanType::SelfServeBusinessProLite => PlanLabel::BusinessPremium,
        PlanType::EnterpriseCbpAutomation => PlanLabel::EnterpriseAutomation,
        PlanType::Edu => PlanLabel::Edu,
        PlanType::EduPlus => PlanLabel::EduPlus,
        PlanType::EduPro => PlanLabel::EduPro,
        PlanType::Sci => PlanLabel::AcademicResearchers,
        PlanType::SciPlus => PlanLabel::AcademicResearchersPlus,
        PlanType::Unknown => PlanLabel::Unknown,
    }
}

/// An instruction file (`AGENTS.md`) the session loaded: a path of this
/// machine, or the display text of a path that belongs to another system
/// (the host makes it from the path URI, `inferred_native_path_string`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentsSource {
    Native(String),
    Foreign(String),
}

// oracle: ../status/helpers.rs fn format_directory_display (the card never
// passes a `max_width`: the cut belongs to rtwk's `Text`, G44 for kana)
pub fn format_directory_display(directory: &str, home: Option<&str>) -> String {
    let separator = std::path::MAIN_SEPARATOR;
    match home.and_then(|home| Path::new(directory).strip_prefix(home).ok()) {
        Some(rel) if rel.as_os_str().is_empty() => "~".to_string(),
        Some(rel) => format!("~{separator}{}", rel.display()),
        None => directory.to_string(),
    }
}

// oracle: ../status/helpers.rs fn compose_agents_summary; the sources are the
// loaded files, `cwd` the session directory. Empty when none was loaded (the
// view says `<none>`).
pub fn compose_agents_summary(
    cwd: &str,
    home: Option<&str>,
    sources: &[AgentsSource],
) -> Vec<String> {
    let cwd = Path::new(cwd);
    let normalize = |path: &Path| format_directory_display(&path.display().to_string(), home);
    sources
        .iter()
        .map(|source| {
            let path = match source {
                AgentsSource::Foreign(text) => return text.clone(),
                AgentsSource::Native(path) => Path::new(path),
            };
            let file_name = path.file_name().map_or_else(
                || "<unknown>".to_string(),
                |name| name.to_string_lossy().to_string(),
            );
            let Some(parent) = path.parent() else {
                return normalize(path);
            };
            if parent == cwd {
                return file_name;
            }
            let mut current = cwd;
            let mut ups = 0usize;
            let mut reached = false;
            while let Some(above) = current.parent() {
                if current == parent {
                    reached = true;
                    break;
                }
                current = above;
                ups += 1;
            }
            if reached {
                let up = format!("..{}", std::path::MAIN_SEPARATOR);
                format!("{}{file_name}", up.repeat(ups))
            } else if let Ok(stripped) = path.strip_prefix(cwd) {
                normalize(stripped)
            } else {
                normalize(path)
            }
        })
        .collect()
}

/// A day of the calendar and its month (1 to 12): the date a reset falls on
/// when it is not the day the limit was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetDate {
    pub day: u8,
    pub month: u8,
}

/// A reset in the user's local time: the clock reading, and the date when it
/// is not the day of the capture (oracle `format_reset_timestamp`; the view
/// spells it in the clock preference).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetTime {
    pub hour: u8,
    pub minute: u8,
    pub date: Option<ResetDate>,
}

const DAY: i64 = 86_400;

/// Days since 1970-01-01 to the (month, day) of the civil calendar (Hinnant's
/// `civil_from_days`).
fn civil(days: i64) -> (u8, u8) {
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (month as u8, day as u8)
}

// oracle: ../status/helpers.rs fn format_reset_timestamp, over unix seconds
// and the offset of the local time zone from UTC.
pub fn reset_time(at: i64, captured_at: i64, utc_offset_secs: i32) -> ResetTime {
    let local = at + i64::from(utc_offset_secs);
    let captured = captured_at + i64::from(utc_offset_secs);
    let seconds = local.rem_euclid(DAY);
    let (month, day) = civil(local.div_euclid(DAY));
    ResetTime {
        hour: (seconds / 3_600) as u8,
        minute: (seconds % 3_600 / 60) as u8,
        date: (local.div_euclid(DAY) != captured.div_euclid(DAY))
            .then_some(ResetDate { day, month }),
    }
}

/// Credits as `1.5K`, `12M` (oracle `status/thread_usage.rs` `format_credit_micros`): what the
/// status line, the terminal title and the card spell a figure of credits with.
pub fn format_credit_micros(micros: i64) -> String {
    let micros = i128::from(micros.max(0));
    let (unit_micros, suffix) = [
        (1_000_000_000_000_000_000_i128, "T"),
        (1_000_000_000_000_000_i128, "B"),
        (1_000_000_000_000_i128, "M"),
        (1_000_000_000_i128, "K"),
    ]
    .into_iter()
    .find(|(unit, _)| micros >= *unit)
    .unwrap_or((1_000_000, ""));
    let tenths = (micros * 10 + unit_micros / 2) / unit_micros;
    let whole = tenths / 10;
    let fraction = tenths % 10;
    if fraction == 0 {
        format!("{whole}{suffix}")
    } else {
        format!("{whole}.{fraction}{suffix}")
    }
}

/// Dollars as `~$0.0123` (oracle `format_estimated_usd_micros`); none for a negative figure.
pub fn format_estimated_usd_micros(micros: i64) -> Option<String> {
    if micros < 0 {
        return None;
    }
    if micros > 0 && micros < 100 {
        return Some(format!("~$0.{micros:06}"));
    }
    if micros > 0 && micros < 10_000 {
        let ten_thousandths = micros.saturating_add(50) / 100;
        return Some(format!("~$0.{ten_thousandths:04}"));
    }
    let cents = micros.saturating_add(5_000) / 10_000;
    Some(format!("~${}.{:02}", cents / 100, cents % 100))
}

/// ENT26 is billed directly in dollars; a missing native usage is not a zero estimate (oracle
/// `status/thread_usage.rs` `thread_cost_micros`).
pub fn thread_cost_micros(
    usage: &codex_app_server_protocol::ThreadUsage,
    plan: Option<PlanType>,
) -> Option<i64> {
    if plan == Some(PlanType::Ent26) {
        usage.native_usage_usd_micros
    } else {
        usage.estimated_usage_usd_micros
    }
}
