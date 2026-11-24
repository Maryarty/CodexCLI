//! Validates the desktop safety-recourse metadata contract (oracle
//! `chatwidget/safety_recourse/metadata.rs`, carried unchanged but for the two text checks the
//! oracle takes from crates core does not use, `tool_mention_in` and `is_async_reply`). The
//! explanation is an English sentence of the contract, kept as data with the offers: the server
//! words it, not the UI string tables.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recourse {
    pub explanation: String,
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
    let (bio_lead, cyber_lead, bio_offer) = match (
        wire.source.as_str(),
        message.kind.as_str(),
        message.role.as_str(),
    ) {
        ("user_request", "ordinary_message", "user") => (
            "with requests involving ",
            "with requests that could enable ",
            None,
        ),
        ("assistant_response", "ordinary_message", "assistant") => (
            "when a response would provide information about ",
            "when a response could enable ",
            None,
        ),
        ("file_content", "file_tool_call", "assistant")
        | ("file_content", "file_tool_result", "tool") => (
            "when content from a file involves ",
            "when content could enable ",
            Some("You can continue without this file or ask Codex for an overview of "),
        ),
        ("web_search_result", "web_tool_call", "assistant")
        | ("web_search_result", "web_tool_result", "tool") => (
            "when a web search result contains information about ",
            "when content could enable ",
            Some("You can retry without web search or ask Codex for an overview of "),
        ),
        ("other_tool", "other_tool_call", "assistant")
        | ("other_tool", "other_tool_result", "tool") => {
            ("when content involves ", "when content could enable ", None)
        }
        _ => return None,
    };
    let (concern, help) = match (wire.domain, wire.topic.as_str()) {
        (Domain::Bio, "biological_agent_access_and_safeguards") => (
            "access to biological materials and related safety controls",
            "access processes and safeguards",
        ),
        (Domain::Bio, "biological_agent_growth_or_production") => (
            "growing or producing biological agents",
            "the science and safety considerations",
        ),
        (Domain::Bio, "risky_biological_experimentation_or_modification") => (
            "biological experimentation or modification that could pose safety risks",
            "the research goals and safety considerations",
        ),
        (Domain::Bio, "harmful_biological_exposure_or_release") => (
            "biological exposure or release that could cause harm",
            "the risks, prevention, and response",
        ),
        (Domain::Bio, "risky_biological_activities") => (
            "biological activities that could pose safety risks",
            "biological research and safety considerations",
        ),
        (Domain::Cyber, "unauthorized_account_or_system_access") => (
            "access to accounts or systems beyond permitted scope",
            "account security and recovery",
        ),
        (Domain::Cyber, "high_risk_cybersecurity_tools") => (
            "system disruption or the development of high-risk cyber tools",
            "system protection and incident recovery",
        ),
        (Domain::Cyber, "sensitive_data_or_private_communications") => (
            "sensitive data collection or private communications monitoring",
            "data protection and privacy safeguards",
        ),
        (Domain::Cyber, "software_vulnerabilities_or_security_weaknesses") => (
            "capability or technique development for bypassing security controls",
            "safe assessment and remediation",
        ),
        (Domain::Cyber, "risky_cybersecurity_activities") => (
            "cyber activities that pose security or safety risks",
            "cybersecurity fundamentals and defensive best practices",
        ),
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
    let overview = "You can ask Codex for an overview of ";
    let (lead, offer) = match wire.domain {
        Domain::Bio => (bio_lead, bio_offer.unwrap_or(overview)),
        Domain::Cyber => (cyber_lead, overview),
    };
    let explanation = format!("We take extra caution {lead}{concern}. {offer}{help}.");
    let result = Recourse {
        explanation,
        suggestions,
    };
    Some((wire.domain, Some(result)))
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
    text.char_indices().any(|(at, c)| {
        c == '$' && text[at + 1..].chars().next().is_some_and(named)
    }) || ["](skill://", "](app://", "](plugin://"]
        .iter()
        .any(|link| text.contains(link))
}
