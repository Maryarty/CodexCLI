//! The elicitations the oracle shows as its app link dialog (`bottom_pane/app_link_view.rs`):
//! an MCP tool suggestion (install or enable an app, with the reason), a sign-in a connector
//! asks for, and any other server's URL elicitation. The model reads the request once, here, and
//! keeps what the dialog draws and asks: the kind, the app, the reason and the page to open. The
//! words of the dialog, the screen it is on and its cursor are the view's (`sheets/select/app_link`).
//!
//! What is shown is only a link the oracle accepts (`validate_external_url`: https, a host, no
//! credentials). A request that asks for one it refuses is declined at once (`Parsed::Reject`,
//! `bottom_pane/mod.rs` `push_mcp_server_elicitation_request`), with no dialog and no queue entry.

use codex_app_server_protocol::McpServerElicitationRequest as Wire;
use codex_app_server_protocol::McpServerElicitationRequestParams as Params;
use codex_app_server_protocol::ServerRequest;
use codex_protocol::mcp_approval_meta::{APPROVAL_KIND_KEY, APPROVAL_KIND_TOOL_SUGGESTION};
use serde_json::Value;
use url::Url;

use super::Form;

/// The server whose links are the apps' own: only its sign-in is trusted to name a connector,
/// and only its answers read the directory of apps again.
pub const APPS_SERVER: &str = "codex_apps";

const APPS_META_KEY: &str = "_codex_apps";
const AUTH_FAILURE_KEY: &str = "connector_auth_failure";
const IS_AUTH_FAILURE_KEY: &str = "is_auth_failure";
const CONNECTOR_ID_KEY: &str = "connector_id";
const CONNECTOR_NAME_KEY: &str = "connector_name";
const TOOL_TYPE_KEY: &str = "tool_type";
const TOOL_ID_KEY: &str = "tool_id";
const TOOL_NAME_KEY: &str = "tool_name";
const SUGGEST_TYPE_KEY: &str = "suggest_type";
const SUGGEST_REASON_KEY: &str = "suggest_reason";
const INSTALL_URL_KEY: &str = "install_url";

/// What the request asks the user to do (the oracle's `AppLinkSuggestionType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Install an app: its page, then "I already Installed it".
    Install,
    /// Switch an installed app on, or open its page.
    Enable,
    /// Sign in to an app or a server, in the browser.
    Auth,
    /// Any other action a server wants done in the browser (`mode: "url"`).
    Url,
}

/// One app link request, as the dialog needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppLinkPrompt {
    pub kind: Kind,
    /// The server that asked.
    pub server: String,
    /// The app (connector or plugin) the link is about; the elicitation's own id for a URL.
    pub app_id: String,
    /// The app's name, the dialog's title. Empty for a URL request: the dialog says what the
    /// request is for.
    pub name: String,
    /// What the app is. The wire carries none, the oracle's request neither: only a fixture sets it.
    pub description: Option<String>,
    /// Why it is asked (the suggestion's reason, the URL request's message).
    pub reason: Option<String>,
    /// The page to open: validated and normalized as the oracle's `Url::as_str`.
    pub url: String,
}

impl AppLinkPrompt {
    /// The app is installed already: its page manages it, and the dialog can switch it.
    pub fn installed(&self) -> bool {
        self.kind != Kind::Install
    }

    /// The page is a browser flow (a sign-in, an action) rather than the app's own page.
    pub fn browser_flow(&self) -> bool {
        matches!(self.kind, Kind::Auth | Kind::Url)
    }

    /// Finishing it reads the directory of apps again: only the apps server's links are about it.
    pub fn refreshes_apps(&self) -> bool {
        self.server == APPS_SERVER
    }
}

/// What a request is, as the dialog reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// Shown as the app link dialog.
    Link(AppLinkPrompt),
    /// A link request whose URL the oracle refuses, or a form kind it does not take: declined.
    Reject,
    /// Not a link request: the other dialogs read it.
    NotALink,
}

/// Reads an elicitation request (`from_url_app_server_request`, then the tool suggestion of
/// `push_mcp_server_elicitation_request`).
pub fn parse(params: &Params) -> Parsed {
    match &params.request {
        Wire::Url {
            meta,
            message,
            url,
            elicitation_id,
        } => from_url(
            &params.server_name,
            meta.as_ref(),
            message,
            url,
            elicitation_id,
        ),
        Wire::Form {
            meta,
            requested_schema,
            ..
        } => from_suggestion(&params.server_name, meta.as_ref(), requested_schema),
        // The oracle has no view for these two: it declines them (`handle_elicitation_request_now`).
        Wire::OpenAiForm { .. } | Wire::OpenAiElicitationForm { .. } => Parsed::Reject,
        Wire::UserVerification { .. } => Parsed::NotALink,
    }
}

/// Whether the oracle declines `request` at once, with no dialog.
pub fn rejected(request: &ServerRequest) -> bool {
    matches!(
        request,
        ServerRequest::McpServerElicitationRequest { params, .. } if parse(params) == Parsed::Reject
    )
}

fn from_url(
    server: &str,
    meta: Option<&Value>,
    message: &str,
    url: &str,
    elicitation_id: &str,
) -> Parsed {
    if server == APPS_SERVER {
        // The apps server asks only for the sign-in of a connector, on a ChatGPT host.
        let Some(url) = external_url(url, true) else {
            return Parsed::Reject;
        };
        return match auth_failure(meta, elicitation_id) {
            Some((app_id, name)) => Parsed::Link(AppLinkPrompt {
                kind: Kind::Auth,
                server: server.to_string(),
                app_id,
                name,
                description: None,
                reason: Some(message.to_string()),
                url,
            }),
            None => Parsed::Reject,
        };
    }
    match external_url(url, false) {
        Some(url) => Parsed::Link(AppLinkPrompt {
            kind: Kind::Url,
            server: server.to_string(),
            app_id: elicitation_id.to_string(),
            name: String::new(),
            description: None,
            reason: Some(message.to_string()),
            url,
        }),
        None => Parsed::Reject,
    }
}

/// The connector whose sign-in the request is (`from_codex_apps_auth_url_parts`): its id and name.
fn auth_failure(meta: Option<&Value>, elicitation_id: &str) -> Option<(String, String)> {
    let failure = meta?
        .as_object()?
        .get(APPS_META_KEY)?
        .as_object()?
        .get(AUTH_FAILURE_KEY)?
        .as_object()?;
    if failure.get(IS_AUTH_FAILURE_KEY).and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let text = |key: &str| {
        failure
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let app_id = text(CONNECTOR_ID_KEY).unwrap_or(elicitation_id).to_string();
    let name = text(CONNECTOR_NAME_KEY).unwrap_or(&app_id).to_string();
    Some((app_id, name))
}

/// A form request that suggests a tool (`parse_tool_suggestion_request`) and names where to
/// install it: the dialog. One that names none is the plain form's.
fn from_suggestion(server: &str, meta: Option<&Value>, schema: &impl serde::Serialize) -> Parsed {
    let Some(meta) = meta.and_then(Value::as_object) else {
        return Parsed::NotALink;
    };
    if meta.get(APPROVAL_KIND_KEY).and_then(Value::as_str) != Some(APPROVAL_KIND_TOOL_SUGGESTION) {
        return Parsed::NotALink;
    }
    let text = |key: &str| meta.get(key).and_then(Value::as_str);
    if !matches!(text(TOOL_TYPE_KEY), Some("connector" | "plugin")) {
        return Parsed::NotALink;
    }
    let kind = match text(SUGGEST_TYPE_KEY) {
        Some("install") => Kind::Install,
        Some("enable") => Kind::Enable,
        _ => return Parsed::NotALink,
    };
    let (Some(reason), Some(app_id), Some(name)) = (
        text(SUGGEST_REASON_KEY),
        text(TOOL_ID_KEY),
        text(TOOL_NAME_KEY),
    ) else {
        return Parsed::NotALink;
    };
    let Some(install_url) = text(INSTALL_URL_KEY) else {
        return Parsed::NotALink;
    };
    // The oracle's request is built only when its schema is the empty one or a form it can show.
    let schema = serde_json::to_value(schema).unwrap_or(Value::Null);
    let empty = schema.get("type").and_then(Value::as_str) == Some("object")
        && schema
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(serde_json::Map::is_empty);
    if !schema.is_null()
        && !empty
        && Form::parse(Some(&Value::Object(meta.clone())), &schema).is_none()
    {
        return Parsed::NotALink;
    }
    match external_url(install_url, false) {
        Some(url) => Parsed::Link(AppLinkPrompt {
            kind,
            server: server.to_string(),
            app_id: app_id.to_string(),
            name: name.to_string(),
            description: None,
            reason: Some(reason.to_string()),
            url,
        }),
        None => Parsed::Reject,
    }
}

/// `validate_external_url`: the URL as the oracle shows and opens it, when it is https with a host
/// and no credentials (and, for the apps server's sign-in, on a ChatGPT host).
pub fn external_url(url: &str, require_chatgpt_host: bool) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return None;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    if require_chatgpt_host && !is_chatgpt_host(parsed.host_str()?) {
        return None;
    }
    Some(parsed.to_string())
}

fn is_chatgpt_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "chatgpt.com"
        || host == "chatgpt-staging.com"
        || host.ends_with(".chatgpt.com")
        || host.ends_with(".chatgpt-staging.com")
}
