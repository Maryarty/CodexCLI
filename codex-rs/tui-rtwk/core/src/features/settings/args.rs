//! The inline forms of `/title`, `/statusline` and `/model` (oracle
//! `chatwidget/status_surface_commands.rs`, `model_command.rs`): the arguments parsed and
//! checked, with no IO. A line that does not pass is a typed `Refusal` (the words are the UI's,
//! `src/features/settings/strings.rs`); one that does is what the session applies: a status
//! override of the chat (session-only: nothing is written to the config) or a model pick for
//! this session.

use super::items::{STATUS_LINE, TERMINAL_TITLE};
use crate::commands::args::{ArgError, Spec, parse};
use crate::screens::chat::StatusOverride;
use crate::screens::status::{RateLimits, RateWindow};
use crate::screens::select::models::{Apply, Current, Preset, Selection, warning};

/// The surface an item list is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Title,
    StatusLine,
}

/// Why the arguments of a command are refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The line did not tokenize.
    Args(ArgError),
    /// `Cannot combine --clear with {surface} items.`
    ClearWithItems(Surface),
    /// `Unknown {surface} item '{token}'. Use {command} to see available items.`
    UnknownItem { surface: Surface, token: String },
    /// `Ambiguous {surface} item '{token}'. Use 'five-hour-limit' or 'weekly-limit'.`
    AmbiguousItem { surface: Surface, token: String },
    /// `Unexpected argument '{0}'. Use '--reasoning=<level>' or '--speed=<tier>'.`
    Unexpected(String),
    /// `Session is still starting; try /model again in a moment.`
    Starting,
    /// `Model arguments are unavailable while using Reserve. Use /model to adjust effort.`
    Reserve,
    /// `Model '{0}' is unavailable. Use /model to see available models.`
    ModelUnavailable(String),
    /// `Reasoning level '{effort}' is unavailable for {model}. Available: {available}.`
    EffortUnavailable {
        model: String,
        effort: String,
        available: String,
    },
    /// `Speed '{speed}' is unavailable for {model}. Available: {available}.`
    SpeedUnavailable {
        model: String,
        speed: String,
        available: String,
    },
}

impl From<ArgError> for Refusal {
    fn from(error: ArgError) -> Self {
        Refusal::Args(error)
    }
}

/// The name a window gives the item that shows it (`rate_limit_preview_copy`): the window's
/// label (`get_limits_duration`, within five percent of its span), else the fallback of its slot.
fn limit_name(window: &RateWindow, secondary: bool) -> &'static str {
    const HOUR: i64 = 60;
    const DAY: i64 = 24 * HOUR;
    let minutes = window.window_minutes.map(|minutes| minutes.max(0));
    let named = minutes.and_then(|minutes| {
        [
            (5 * HOUR, "five-hour-limit"),
            (DAY, "daily-limit"),
            (7 * DAY, "weekly-limit"),
            (30 * DAY, "monthly-limit"),
            (365 * DAY, "annual-limit"),
        ]
        .into_iter()
        .find(|(span, _)| {
            let (minutes, span) = (minutes as f64, *span as f64);
            minutes >= span * 0.95 && minutes <= span * 1.05
        })
        .map(|(_, name)| name)
    });
    named.unwrap_or(if secondary {
        "secondary-usage-limit"
    } else {
        "usage-limit"
    })
}

/// One item of a line (`parse_item`): the id as it is, an alias of it (`model-name`), or the name
/// the limit item has now (`monthly-limit` for a primary window of a month: the item that shows
/// the primary window is `five-hour-limit`, the secondary's `weekly-limit`). A name both items
/// have is ambiguous.
fn item(
    surface: Surface,
    known: &[&str],
    token: &str,
    rates: Option<&RateLimits>,
) -> Result<String, Refusal> {
    let unknown = || Refusal::UnknownItem {
        surface,
        token: token.to_owned(),
    };
    let token = if token == "model-name" { "model" } else { token };
    if known.contains(&token) {
        return Ok(token.to_owned());
    }
    let named = |window: &Option<RateWindow>, secondary: bool| {
        window
            .as_ref()
            .is_some_and(|window| limit_name(window, secondary) == token)
    };
    let (primary, secondary) = rates.map_or((false, false), |rates| {
        (
            named(&rates.primary, false),
            named(&rates.secondary, true),
        )
    });
    let id = match (primary, secondary) {
        (true, false) => "five-hour-limit",
        (false, true) => "weekly-limit",
        (true, true) => {
            return Err(Refusal::AmbiguousItem {
                surface,
                token: token.to_owned(),
            });
        }
        (false, false) => return Err(unknown()),
    };
    known
        .contains(&id)
        .then(|| id.to_owned())
        .ok_or_else(unknown)
}

fn items(
    surface: Surface,
    known: &[&str],
    tokens: &[String],
    rates: Option<&RateLimits>,
) -> Result<Vec<String>, Refusal> {
    tokens
        .iter()
        .map(|token| item(surface, known, token, rates))
        .collect()
}

/// `/title <items> | --clear`: the terminal title of this session.
pub fn title(args: &str) -> Result<StatusOverride, Refusal> {
    title_with(args, None)
}

/// `/title` with the limits the chat holds, which the names of the limit items are judged by.
pub fn title_with(args: &str, rates: Option<&RateLimits>) -> Result<StatusOverride, Refusal> {
    let parsed = parse(
        args,
        Spec {
            switches: &["--clear"],
            ..Spec::default()
        },
    )?;
    let items = items(Surface::Title, &TERMINAL_TITLE, &parsed.positionals, rates)?;
    let clear = parsed.has_switch("--clear");
    if clear && !items.is_empty() {
        return Err(Refusal::ClearWithItems(Surface::Title));
    }
    Ok(StatusOverride {
        terminal_title: Some(if clear { Vec::new() } else { items }),
        ..StatusOverride::default()
    })
}

/// `/statusline <items> | --clear | --colors | --no-colors`: the status line of this session.
pub fn status_line(args: &str) -> Result<StatusOverride, Refusal> {
    status_line_with(args, None)
}

/// `/statusline` with the limits the chat holds, which the names of the limit items are judged by.
pub fn status_line_with(args: &str, rates: Option<&RateLimits>) -> Result<StatusOverride, Refusal> {
    let parsed = parse(
        args,
        Spec {
            switches: &["--clear"],
            booleans: &["colors"],
            ..Spec::default()
        },
    )?;
    let items = items(Surface::StatusLine, &STATUS_LINE, &parsed.positionals, rates)?;
    let clear = parsed.has_switch("--clear");
    if clear && !items.is_empty() {
        return Err(Refusal::ClearWithItems(Surface::StatusLine));
    }
    Ok(StatusOverride {
        status_line: if clear {
            Some(Vec::new())
        } else if items.is_empty() {
            None
        } else {
            Some(items)
        },
        status_line_colors: parsed.boolean("colors"),
        ..StatusOverride::default()
    })
}

/// What `/model <args>` asks for, with the speed it names (the pick has no tier of its own).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelPick {
    pub selection: Selection,
    /// The service tier id the line asked for, checked against the model's tiers.
    pub speed: Option<String>,
}

/// `resolve_model_command`: `/model [<name>] [--reasoning=<level>] [--speed=<tier>]` against the
/// catalog the session holds. Without a name the current model is kept (and its effort).
pub fn model(
    args: &str,
    started: bool,
    current: &Current,
    presets: &[Preset],
) -> Result<ModelPick, Refusal> {
    let parsed = parse(
        args,
        Spec {
            values: &["--reasoning", "--speed"],
            ..Spec::default()
        },
    )?;
    if let Some(unexpected) = parsed.positionals.get(1) {
        return Err(Refusal::Unexpected(unexpected.clone()));
    }
    if !started {
        return Err(Refusal::Starting);
    }
    if current.reserve.is_some() {
        return Err(Refusal::Reserve);
    }
    let named = parsed.positionals.first();
    let name = named.cloned().unwrap_or_else(|| current.model.clone());
    let Some(preset) = presets
        .iter()
        .find(|preset| preset.model == name && (named.is_none() || preset.show_in_picker))
    else {
        return Err(Refusal::ModelUnavailable(name));
    };
    let effort = match parsed.value("--reasoning") {
        None if named.is_none() => current
            .effort
            .clone()
            .unwrap_or_else(|| preset.default_effort.clone()),
        None => preset.default_effort.clone(),
        Some(wanted) => {
            let supported = if preset.efforts.is_empty() {
                (preset.default_effort.to_string() == wanted).then(|| preset.default_effort.clone())
            } else {
                preset
                    .efforts
                    .iter()
                    .find(|option| option.effort.to_string() == wanted)
                    .map(|option| option.effort.clone())
            };
            supported.ok_or_else(|| Refusal::EffortUnavailable {
                model: preset.model.clone(),
                effort: wanted.to_owned(),
                available: if preset.efforts.is_empty() {
                    preset.default_effort.to_string()
                } else {
                    preset
                        .efforts
                        .iter()
                        .map(|option| option.effort.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            })?
        }
    };
    let speed = match parsed.value("--speed") {
        None => None,
        Some("standard") => Some("standard".to_owned()),
        Some("flex") => Some("flex".to_owned()),
        Some(wanted) => Some(
            preset
                .service_tiers
                .iter()
                .find(|tier| tier.id == wanted || tier.name.eq_ignore_ascii_case(wanted))
                .map(|tier| tier.id.clone())
                .ok_or_else(|| Refusal::SpeedUnavailable {
                    model: preset.model.clone(),
                    speed: wanted.to_owned(),
                    available: ["standard", "flex"]
                        .into_iter()
                        .map(str::to_owned)
                        .chain(preset.service_tiers.iter().map(|tier| tier.id.clone()))
                        .collect::<Vec<_>>()
                        .join(", "),
                })?,
        ),
    };
    let warning = warning(Some(current), Some(&effort));
    Ok(ModelPick {
        selection: Selection {
            model: preset.model.clone(),
            effort: Some(effort),
            apply: Apply::Session,
            warning,
        },
        speed,
    })
}
