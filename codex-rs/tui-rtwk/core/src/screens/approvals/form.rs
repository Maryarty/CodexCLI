//! The form of an MCP server elicitation (`mcpServer/elicitation/request`, mode `form`): what the
//! request's `requestedSchema` and `_meta` ask, and what an answer sends back. Pure data and
//! logic, no rtwk, no words of its own (the view names the actions and the booleans). Carried
//! from the oracle's `bottom_pane/mcp_server_elicitation.rs`
//! (`McpServerElicitationFormRequest::from_parts`, `parse_field`, `submit_answers`): the fields
//! (select, boolean or text), the approval fallback of an empty schema (Allow, the persist
//! choices the meta offers, then Cancel for a tool approval or Deny and Cancel for another
//! request), the parameter summary of a tool approval and the answer.

use codex_protocol::mcp_approval_meta::{
    APPROVAL_KIND_KEY, APPROVAL_KIND_MCP_TOOL_CALL, APPROVAL_KIND_TOOL_SUGGESTION,
    CANCELLATION_REASON_KEY, PERSIST_ALWAYS, PERSIST_KEY, PERSIST_SESSION, TOOL_PARAMS_DISPLAY_KEY,
    TOOL_PARAMS_KEY,
};
use serde_json::Value;

use super::{Decision, RequestAnswer};

/// How many parameters of a tool approval the prompt lists.
const PARAM_DISPLAY_LIMIT: usize = 3;
/// How many characters of a parameter's value the prompt shows (the oracle counts graphemes;
/// core has no segmenter).
const PARAM_VALUE_TRUNCATE: usize = 60;

/// How the answer is sent: the typed values of the fields, or one of the approval actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMode {
    Content,
    Approval,
}

/// One choice of the approval fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Allow,
    AllowForSession,
    AlwaysAllow,
    Deny,
    Cancel,
}

/// An option of a select field: the label and description the schema gives, and the value sent.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldOption {
    pub label: String,
    pub description: Option<String>,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldInput {
    /// An enum or oneOf string; `default` the index of the option the schema preselects.
    Select {
        options: Vec<FieldOption>,
        default: Option<usize>,
    },
    /// True is choice 0, False is choice 1.
    Boolean { default: Option<bool> },
    Text,
    /// The approval fallback's single field.
    Actions(Vec<Action>),
}

impl FieldInput {
    /// How many choices a list shows (none for text).
    pub fn choices(&self) -> usize {
        match self {
            FieldInput::Select { options, .. } => options.len(),
            FieldInput::Boolean { .. } => 2,
            FieldInput::Actions(actions) => actions.len(),
            FieldInput::Text => 0,
        }
    }

    /// The choice the field starts on: the schema's default, else the first.
    pub fn start(&self) -> usize {
        match self {
            FieldInput::Select { default, .. } => default.unwrap_or(0),
            FieldInput::Boolean { default } => default.map_or(0, |value| usize::from(!value)),
            FieldInput::Actions(_) | FieldInput::Text => 0,
        }
    }

    /// Whether the schema preselects a choice (the field then counts as answered at once).
    pub fn preselected(&self) -> bool {
        match self {
            FieldInput::Select { default, .. } => default.is_some(),
            FieldInput::Boolean { default } => default.is_some(),
            FieldInput::Actions(_) => true,
            FieldInput::Text => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub prompt: String,
    pub required: bool,
    pub input: FieldInput,
}

/// One parameter of a tool approval, as the prompt lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub value: String,
}

/// What the user committed for one field: the chosen option or the typed text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entered {
    Choice(usize),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub mode: FormMode,
    pub tool_approval: bool,
    /// The first three parameters of a tool approval.
    pub summary: Vec<Param>,
    pub fields: Vec<Field>,
}

// JSON numbers are never NaN, so equality on a `Value` is reflexive.
impl Eq for Form {}

impl Form {
    /// The form a request's schema and meta ask; none when the schema holds a field the form
    /// cannot take (numbers, multi-select), a tool suggestion's empty schema, or no object.
    pub fn parse(meta: Option<&Value>, schema: &Value) -> Option<Form> {
        let kind = meta
            .and_then(|meta| meta.get(APPROVAL_KIND_KEY))
            .and_then(Value::as_str);
        let tool_approval = kind == Some(APPROVAL_KIND_MCP_TOOL_CALL);
        let message_only = schema.is_null()
            || schema.get("type").and_then(Value::as_str) == Some("object")
                && schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(serde_json::Map::is_empty);
        if message_only && kind == Some(APPROVAL_KIND_TOOL_SUGGESTION) {
            return None;
        }
        if !message_only {
            return Some(Form {
                mode: FormMode::Content,
                tool_approval,
                summary: Vec::new(),
                fields: parse_fields(schema)?,
            });
        }
        let mut actions = vec![Action::Allow];
        if persist_offers(meta, PERSIST_SESSION) {
            actions.push(Action::AllowForSession);
        }
        if persist_offers(meta, PERSIST_ALWAYS) {
            actions.push(Action::AlwaysAllow);
        }
        if !tool_approval {
            actions.push(Action::Deny);
        }
        actions.push(Action::Cancel);
        Some(Form {
            mode: FormMode::Approval,
            tool_approval,
            summary: if tool_approval {
                param_summary(meta)
            } else {
                Vec::new()
            },
            fields: vec![Field {
                id: "__approval".to_string(),
                label: String::new(),
                prompt: String::new(),
                required: true,
                input: FieldInput::Actions(actions),
            }],
        })
    }

    /// The value field `index` holds for what was committed: an option's value, or the trimmed
    /// text when it is not empty; an action has none (`answer` reads it).
    pub fn value(&self, index: usize, entered: Option<&Entered>) -> Option<Value> {
        let field = self.fields.get(index)?;
        match (&field.input, entered?) {
            (FieldInput::Select { options, .. }, Entered::Choice(option)) => {
                options.get(*option).map(|option| option.value.clone())
            }
            (FieldInput::Boolean { .. }, Entered::Choice(option)) => {
                (*option < 2).then_some(Value::Bool(*option == 0))
            }
            (FieldInput::Actions(actions), Entered::Choice(option)) => (*option < actions.len())
                .then(|| Value::String(actions[*option].wire().to_string())),
            (FieldInput::Text, Entered::Text(text)) => {
                let text = text.trim();
                (!text.is_empty()).then(|| Value::String(text.to_string()))
            }
            _ => None,
        }
    }

    /// The first required field with no value, from `entered` (one slot per field).
    pub fn first_unanswered(&self, entered: &[Option<Entered>]) -> Option<usize> {
        (0..self.fields.len()).find(|&index| {
            self.fields[index].required && self.value(index, entered[index].as_ref()).is_none()
        })
    }

    /// How many required fields have no value.
    pub fn unanswered(&self, entered: &[Option<Entered>]) -> usize {
        (0..self.fields.len())
            .filter(|&index| {
                self.fields[index].required && self.value(index, entered[index].as_ref()).is_none()
            })
            .count()
    }

    /// The answer of a submit: the typed content, or the approval action chosen; the index of the
    /// first required field still without a value when there is one.
    pub fn answer(&self, entered: &[Option<Entered>]) -> Result<RequestAnswer, usize> {
        if let Some(index) = self.first_unanswered(entered) {
            return Err(index);
        }
        if self.mode == FormMode::Approval {
            let action = match (&self.fields[0].input, entered[0].as_ref()) {
                (FieldInput::Actions(actions), Some(Entered::Choice(option))) => {
                    actions.get(*option).copied()
                }
                _ => None,
            };
            let persist = |mode: &str| Some(serde_json::json!({ PERSIST_KEY: mode }));
            let (decision, meta) = match action {
                Some(Action::Allow) => (Decision::Accept, None),
                Some(Action::AllowForSession) => (Decision::Accept, persist(PERSIST_SESSION)),
                Some(Action::AlwaysAllow) => (Decision::Accept, persist(PERSIST_ALWAYS)),
                Some(Action::Deny) => (Decision::Decline, None),
                Some(Action::Cancel) => (Decision::Cancel, self.cancellation_meta()),
                None => (Decision::Cancel, None),
            };
            return Ok(RequestAnswer::Elicitation {
                decision,
                content: None,
                meta,
            });
        }
        let content: serde_json::Map<String, Value> = self
            .fields
            .iter()
            .enumerate()
            .filter_map(|(index, field)| {
                self.value(index, entered[index].as_ref())
                    .map(|value| (field.id.clone(), value))
            })
            .collect();
        Ok(RequestAnswer::Elicitation {
            decision: Decision::Accept,
            content: Some(Value::Object(content)),
            meta: None,
        })
    }

    /// The answer of Esc or ctrl+c: cancel, saying so was the user's when it is a tool approval.
    pub fn cancel(&self) -> RequestAnswer {
        RequestAnswer::Elicitation {
            decision: Decision::Cancel,
            content: None,
            meta: self.cancellation_meta(),
        }
    }

    fn cancellation_meta(&self) -> Option<Value> {
        self.tool_approval
            .then(|| serde_json::json!({ CANCELLATION_REASON_KEY: "user" }))
    }
}

impl Action {
    fn wire(self) -> &'static str {
        match self {
            Action::Allow => "accept",
            Action::AllowForSession => "accept_session",
            Action::AlwaysAllow => "accept_always",
            Action::Deny => "decline",
            Action::Cancel => "cancel",
        }
    }
}

fn persist_offers(meta: Option<&Value>, expected: &str) -> bool {
    match meta.and_then(|meta| meta.get(PERSIST_KEY)) {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .any(|value| value == expected),
        _ => false,
    }
}

/// `tool_params_display` (name, value, display name, in the request's order), else the
/// `tool_params` sorted by name; the first three as `Name: value`.
fn param_summary(meta: Option<&Value>) -> Vec<Param> {
    let Some(meta) = meta.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut params: Vec<(String, Value)> = meta
        .get(TOOL_PARAMS_DISPLAY_KEY)
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(display_param).collect())
        .unwrap_or_default();
    if params.is_empty() {
        params = meta
            .get(TOOL_PARAMS_KEY)
            .and_then(Value::as_object)
            .map(|tool| tool.iter().map(|(name, value)| (name.clone(), value.clone())).collect())
            .unwrap_or_default();
        params.sort_by(|left, right| left.0.cmp(&right.0));
    }
    params
        .into_iter()
        .take(PARAM_DISPLAY_LIMIT)
        .map(|(name, value)| Param {
            name,
            value: param_value(&value),
        })
        .collect()
}

fn display_param(value: &Value) -> Option<(String, Value)> {
    let value = value.as_object()?;
    let name = value.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let display = value
        .get("display_name")
        .and_then(Value::as_str)
        .unwrap_or(name)
        .trim();
    if display.is_empty() {
        return None;
    }
    Some((display.to_string(), value.get("value")?.clone()))
}

fn param_value(value: &Value) -> String {
    let text = match value {
        Value::String(text) => text.split_whitespace().collect::<Vec<_>>().join(" "),
        other => compact_json(other),
    };
    truncate(&text, PARAM_VALUE_TRUNCATE)
}

/// The oracle's `format_json_compact`: the pretty form on one line, a blank after `:` and `,`.
fn compact_json(value: &Value) -> String {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = pretty.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if !escaped => {
                in_string = !in_string;
                out.push(ch);
            }
            '\\' if in_string => {
                escaped = !escaped;
                out.push(ch);
            }
            '\n' | '\r' if !in_string => {}
            ' ' | '\t' if !in_string => {
                if let (Some(&next), Some(last)) = (chars.peek(), out.chars().last())
                    && (last == ':' || last == ',')
                    && !matches!(next, '}' | ']')
                {
                    out.push(' ');
                }
            }
            _ => {
                if escaped && in_string {
                    escaped = false;
                }
                out.push(ch);
            }
        }
    }
    out
}

/// The oracle's `truncate_text`: at most `max` units, `...` replacing the cut.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(3)).collect();
    format!("{kept}...")
}

fn parse_fields(schema: &Value) -> Option<Vec<Field>> {
    let schema = schema.as_object()?;
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return None;
    }
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let properties = schema.get("properties")?.as_object()?;
    let mut fields = Vec::new();
    for (id, property) in properties {
        fields.push(parse_field(id, property, required.contains(&id.as_str()))?);
    }
    (!fields.is_empty()).then_some(fields)
}

fn parse_field(id: &str, property: &Value, required: bool) -> Option<Field> {
    let text = |key: &str| property.get(key).and_then(Value::as_str).map(str::to_string);
    let label = text("title").unwrap_or_else(|| id.to_string());
    let prompt = text("description").unwrap_or_else(|| label.clone());
    let input = match property.get("type").and_then(Value::as_str)? {
        "boolean" => FieldInput::Boolean {
            default: property.get("default").and_then(Value::as_bool),
        },
        "string" => {
            if let Some(values) = property.get("enum").and_then(Value::as_array) {
                let values: Vec<String> = values
                    .iter()
                    .map(|value| value.as_str().map(str::to_string))
                    .collect::<Option<_>>()?;
                let names: Vec<&str> = property
                    .get("enumNames")
                    .and_then(Value::as_array)
                    .map(|names| names.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let default = text("default").and_then(|d| values.iter().position(|v| *v == d));
                FieldInput::Select {
                    options: values
                        .iter()
                        .enumerate()
                        .map(|(index, value)| FieldOption {
                            label: names.get(index).map_or_else(|| value.clone(), ToString::to_string),
                            description: None,
                            value: Value::String(value.clone()),
                        })
                        .collect(),
                    default,
                }
            } else if let Some(choices) = property.get("oneOf").and_then(Value::as_array) {
                let choices: Vec<(String, String)> = choices
                    .iter()
                    .map(|choice| {
                        Some((
                            choice.get("const")?.as_str()?.to_string(),
                            choice.get("title")?.as_str()?.to_string(),
                        ))
                    })
                    .collect::<Option<_>>()?;
                let default = text("default").and_then(|d| choices.iter().position(|(v, _)| *v == d));
                FieldInput::Select {
                    options: choices
                        .into_iter()
                        .map(|(value, title)| FieldOption {
                            label: title,
                            description: None,
                            value: Value::String(value),
                        })
                        .collect(),
                    default,
                }
            } else {
                FieldInput::Text
            }
        }
        _ => return None,
    };
    Some(Field {
        id: id.to_string(),
        label,
        prompt,
        required,
        input,
    })
}
