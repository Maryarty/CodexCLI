//! The hooks browser (`bottom_pane/hooks_browser_view.rs`): the lifecycle
//! hooks the server listed for the working directory (`hooks/list`), by
//! event, and what the user asked of them. The model part with no rtwk; the
//! cursor of either page and its scroll are the view's controllers
//! (`SelectionController`, `ListController`), so what is here is the hooks,
//! the issues the server reported and what a toggle or a trust changes in
//! them (the sheet's draft applies them to a copy until the app takes them).
//!
//! The types are the browser's own, not the protocol's (`HookMetadata`):
//! the host maps the server's entry to them (the source path as the
//! directory display writes it, the hooks in the server's display order) and
//! opens the dialog with `Ev::Offered(Offer::Hooks(..))`.

/// A lifecycle event a hook can be attached to, in the oracle's order
/// (`codex_protocol::protocol::HookEventName`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    SubagentStart,
    SubagentStop,
    Stop,
    Interrupt,
    PreModelRequest,
    PostModelRequest,
}

impl HookEvent {
    pub const ALL: [HookEvent; 14] = [
        HookEvent::PreToolUse,
        HookEvent::PermissionRequest,
        HookEvent::PostToolUse,
        HookEvent::PreCompact,
        HookEvent::PostCompact,
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
        HookEvent::UserPromptSubmit,
        HookEvent::SubagentStart,
        HookEvent::SubagentStop,
        HookEvent::Stop,
        HookEvent::Interrupt,
        HookEvent::PreModelRequest,
        HookEvent::PostModelRequest,
    ];

    /// The event's row in the list of events.
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|event| *event == self)
            .unwrap_or(0)
    }
}

/// Where a hook is declared (`HookSource`); the view names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookSource {
    System,
    User,
    Project,
    Mdm,
    SessionFlags,
    Plugin,
    CloudRequirements,
    CloudManagedConfig,
    LegacyManagedConfigFile,
    LegacyManagedConfigMdm,
    Unknown,
}

/// Whether the user vouched for a hook (`HookTrustStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrust {
    Managed,
    Trusted,
    Untrusted,
    Modified,
}

/// What a hook runs (`HookHandlerMetadata`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "handler", rename_all = "snake_case")]
pub enum HookHandler {
    Command {
        command: String,
        #[serde(default, rename = "async")]
        is_async: bool,
    },
    McpTool {
        server: String,
        tool: String,
    },
    Prompt,
    Agent,
}

fn timeout() -> u64 {
    30
}

fn hash() -> String {
    "sha256:current".to_string()
}

/// One hook (`HookMetadata`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Hook {
    pub key: String,
    pub event: HookEvent,
    #[serde(flatten)]
    pub handler: HookHandler,
    #[serde(default)]
    pub matcher: Option<String>,
    #[serde(default = "timeout")]
    pub timeout_sec: u64,
    /// The title the browser shows; blank or none: "Unnamed hook".
    #[serde(default)]
    pub status_message: Option<String>,
    /// The `additionalContext` spill threshold; `0` is unlimited.
    #[serde(default)]
    pub additional_context_limit: Option<usize>,
    pub source: HookSource,
    /// The path of the file declaring it, written as the directory display
    /// writes it (`~/...`).
    #[serde(default)]
    pub source_path: String,
    #[serde(default)]
    pub plugin_id: Option<String>,
    #[serde(default)]
    pub environment_id: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub is_managed: bool,
    #[serde(default = "hash")]
    pub current_hash: String,
    pub trust: HookTrust,
}

impl Hook {
    /// `hook_needs_review`: new or changed since it was trusted.
    pub fn needs_review(&self) -> bool {
        matches!(self.trust, HookTrust::Untrusted | HookTrust::Modified)
    }

    /// `hook_is_active`: on, and managed or trusted.
    pub fn is_active(&self) -> bool {
        self.enabled && matches!(self.trust, HookTrust::Managed | HookTrust::Trusted)
    }
}

/// A file the server could not read hooks from (`HookErrorInfo`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct HookIssue {
    pub path: String,
    pub message: String,
}

/// One row of the events page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub event: HookEvent,
    pub installed: usize,
    pub active: usize,
    pub needs_review: usize,
}

/// `HooksBrowserView`: the hooks, the server's warnings and errors. The page open (the events,
/// or the hooks of an event) is the sheet's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hooks {
    pub hooks: Vec<Hook>,
    pub warnings: Vec<String>,
    pub errors: Vec<HookIssue>,
}

impl Hooks {
    pub fn new(hooks: Vec<Hook>, warnings: Vec<String>, errors: Vec<HookIssue>) -> Self {
        Self {
            hooks,
            warnings,
            errors,
        }
    }

    /// The events page: a row for every event.
    pub fn rows(&self) -> Vec<EventRow> {
        HookEvent::ALL
            .iter()
            .map(|event| {
                let of = || self.hooks.iter().filter(|hook| hook.event == *event);
                EventRow {
                    event: *event,
                    installed: of().count(),
                    active: of().filter(|hook| hook.is_active()).count(),
                    needs_review: of().filter(|hook| hook.needs_review()).count(),
                }
            })
            .collect()
    }

    /// The hooks of an event, in the server's order.
    pub fn of(&self, event: HookEvent) -> Vec<&Hook> {
        self.hooks
            .iter()
            .filter(|hook| hook.event == event)
            .collect()
    }

    /// How many hooks wait for a review.
    pub fn review_total(&self) -> usize {
        self.hooks.iter().filter(|hook| hook.needs_review()).count()
    }

    /// The row the events page starts on: the first event with a hook to
    /// review, else the first.
    pub fn initial_event(&self) -> usize {
        self.rows()
            .iter()
            .position(|row| row.needs_review > 0)
            .unwrap_or(0)
    }

    /// `toggle_selected_hook`: managed hooks and those waiting for a review
    /// stay as they are. Returns what changed.
    pub fn toggle(&mut self, key: &str) -> Option<bool> {
        let hook = self.hooks.iter_mut().find(|hook| hook.key == key)?;
        if hook.is_managed || hook.needs_review() {
            return None;
        }
        hook.enabled = !hook.enabled;
        Some(hook.enabled)
    }

    /// `trust_selected_hook`: a hook waiting for a review is trusted, its
    /// enablement as it was. Returns the hash vouched for.
    pub fn trust(&mut self, key: &str) -> Option<String> {
        let hook = self.hooks.iter_mut().find(|hook| hook.key == key)?;
        if !hook.needs_review() {
            return None;
        }
        hook.trust = HookTrust::Trusted;
        Some(hook.current_hash.clone())
    }

    /// `trust_all_hooks`: every hook waiting for a review is trusted.
    pub fn trust_all(&mut self) -> Vec<(String, String)> {
        let mut updates = Vec::new();
        for hook in self.hooks.iter_mut().filter(|hook| hook.needs_review()) {
            hook.trust = HookTrust::Trusted;
            updates.push((hook.key.clone(), hook.current_hash.clone()));
        }
        updates
    }
}
