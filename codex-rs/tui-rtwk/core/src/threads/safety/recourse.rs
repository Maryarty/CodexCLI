//! Validates the desktop safety-recourse metadata contract (oracle
//! `chatwidget/safety_recourse/metadata.rs`, carried unchanged but for the two text checks the
//! oracle takes from crates core does not use, `tool_mention_in` and `is_async_reply`). The
//! explanation of the oracle is made of the source and the topic kept here as data; its words are
//! the UI's.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Bio,
    Cyber,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    schema_version: u8,
    domain: Domain,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResult {
    schema_version: u8,
    domain: Domain,
    source: String,
    source_message: SourceMessage,
    topic: String,
    suggestions: [Suggestion; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceMessage {
    id: String,
    kind: String,
    role: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suggestion {
    label: String,
    prompt_text: String,
}

/// What the flagged content was (the contract's `source` and message kind): the explanation's
/// lead ("with requests involving", "when content from a file involves", ...) is the UI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    UserRequest,
    AssistantResponse,
    FileContent,
    WebSearchResult,
    OtherTool,
}

/// What it concerned (the contract's `topic` of its domain): the explanation's concern and the
/// help offered ("access processes and safeguards") are the UI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    BioAccessAndSafeguards,
    BioGrowthOrProduction,
    BioExperimentationOrModification,
    BioExposureOrRelease,
    BioRiskyActivities,
    CyberUnauthorizedAccess,
    CyberHighRiskTools,
    CyberSensitiveData,
    CyberVulnerabilities,
    CyberRiskyActivities,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recourse {
    pub source: Source,
    pub topic: Topic,
    /// The offered rewrites: label and the prompt a pick starts a new chat with.
    pub suggestions: Vec<(String, String)>,
}

pub fn parse(mut value: Value) -> Option<(Domain, Option<Recourse>)> {
    let object = value.as_object_mut()?;
    let recourse = object.remove("safety_recourse").filter(|v| !v.is_null());
    let Some(value) = recourse else {
        let block: Block = serde_json::from_value(object.remove("safety_block")?).ok()?;
        return (block.schema_version == 1 && block.status == "loading")
            .then_some((block.domain, None));
    };
    let wire: WireResult = serde_json::from_value(value).ok()?;
    let message = &wire.source_message;
    if wire.schema_version != 1 || !valid_text(&message.id, /*max_chars*/ 256) {
        return None;
    }
    let source = match (
        wire.source.as_str(),
        message.kind.as_str(),
        message.role.as_str(),
    ) {
        ("user_request", "ordinary_message", "user") => Source::UserRequest,
        ("assistant_response", "ordinary_message", "assistant") => Source::AssistantResponse,
        ("file_content", "file_tool_call", "assistant")
        | ("file_content", "file_tool_result", "tool") => Source::FileContent,
        ("web_search_result", "web_tool_call", "assistant")
        | ("web_search_result", "web_tool_result", "tool") => Source::WebSearchResult,
        ("other_tool", "other_tool_call", "assistant")
        | ("other_tool", "other_tool_result", "tool") => Source::OtherTool,
        _ => return None,
    };
    let topic = match (wire.domain, wire.topic.as_str()) {
        (Domain::Bio, "biological_agent_access_and_safeguards") => Topic::BioAccessAndSafeguards,
        (Domain::Bio, "biological_agent_growth_or_production") => Topic::BioGrowthOrProduction,
        (Domain::Bio, "risky_biological_experimentation_or_modification") => {
            Topic::BioExperimentationOrModification
        }
        (Domain::Bio, "harmful_biological_exposure_or_release") => Topic::BioExposureOrRelease,
        (Domain::Bio, "risky_biological_activities") => Topic::BioRiskyActivities,
        (Domain::Cyber, "unauthorized_account_or_system_access") => Topic::CyberUnauthorizedAccess,
        (Domain::Cyber, "high_risk_cybersecurity_tools") => Topic::CyberHighRiskTools,
        (Domain::Cyber, "sensitive_data_or_private_communications") => Topic::CyberSensitiveData,
        (Domain::Cyber, "software_vulnerabilities_or_security_weaknesses") => {
            Topic::CyberVulnerabilities
        }
        (Domain::Cyber, "risky_cybersecurity_activities") => Topic::CyberRiskyActivities,
        _ => return None,
    };
    let mut prompts = HashSet::new();
    let mut suggestions = Vec::new();
    let labels = wire
        .suggestions
        .each_ref()
        .map(|s| s.label.trim().to_lowercase());
    for suggestion in &wire.suggestions {
        if !valid_text(&suggestion.label, /*max_chars*/ 80)
            || !valid_text(&suggestion.prompt_text, /*max_chars*/ 2_000)
            || !prompts.insert(suggestion.prompt_text.trim())
        {
            return None;
        }
        // Only offer actions the composer can safely submit or restore.
        let label = suggestion.label.trim();
        let prompt = suggestion.prompt_text.trim();
        let label_key = label.to_lowercase();
        if suggestion.label.contains(char::is_control)
            || suggestion.label.contains(['\u{2028}', '\u{2029}'])
            || prompt.len() > 1_000
            || prompt.starts_with(['!', '/'])
            || is_async_reply(prompt)
            || tool_mention_in(prompt)
            || (wire.domain == Domain::Cyber && label.eq_ignore_ascii_case("Enable Daybreak"))
            || labels.iter().filter(|key| **key == label_key).count() > 1
        {
            continue;
        }
        suggestions.push((label.into(), prompt.into()));
    }
    Some((
        wire.domain,
        Some(Recourse {
            source,
            topic,
            suggestions,
        }),
    ))
}

fn valid_text(value: &str, max_chars: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= max_chars
}

/// The desktop's async-question reply envelope (`async_question_reply::parse`), optionally behind
/// the IDE context prefix. Only its frame is read here: a prompt in the frame is never offered
/// (a reply with a body the oracle would not parse is withheld too, the safe side).
fn is_async_reply(text: &str) -> bool {
    let text = text.trim();
    let text = if text.starts_with("# Context from my IDE setup:\n") {
        match text.rsplit_once("\n## My request for Codex:\n") {
            Some((_, request)) => request.trim(),
            None => return false,
        }
    } else {
        text
    };
    text.starts_with("<send_user_message_question_reply>")
        && text.ends_with("</send_user_message_question_reply>")
}

/// A suggestion must not activate a skill, app or plugin through its text (`contains_tool_
/// mention`): a `$name` mention, or a link to a skill, app or plugin. Wider than the oracle's
/// scan (any `$` before a name character, whatever the environment-variable list says), so a
/// prompt the oracle would keep can be withheld, never the reverse.
fn tool_mention_in(text: &str) -> bool {
    let named = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-');
    text.char_indices()
        .any(|(at, c)| c == '$' && text[at + 1..].chars().next().is_some_and(named))
        || ["](skill://", "](app://", "](plugin://"]
            .iter()
            .any(|link| text.contains(link))
}
