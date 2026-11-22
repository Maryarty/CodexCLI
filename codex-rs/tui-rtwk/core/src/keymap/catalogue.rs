//! The catalogue of configurable actions: every context, every action with
//! the oracle's built-in default chords, and the rules the resolver needs.
//!
//! Carried as data from the oracle's `tui/src/keymap.rs`
//! (`RuntimeKeymap::built_in_defaults`), `keymap/bindings.rs`
//! (`define_runtime_action_bindings!`, `KeymapContext::overlaps`,
//! `KeymapActionId::overlaps`), `keymap/vim_search.rs` (defaults) and
//! `keymap/chords/leader_defaults.rs`. The oracle declares each action as a
//! struct field in six places; here one row says what it needs.

/// Where an action is active (`KeymapContext`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Context {
    Global,
    Chat,
    /// Voice-only chat action, inactive in pagers and other overlays.
    Voice,
    Composer,
    Editor,
    VimNormal,
    VimOperator,
    VimSearch,
    VimTextObject,
    Pager,
    List,
    Agents,
    Approval,
}

impl Context {
    /// The table of `tui.keymap` the context is read from (`config_name`).
    pub const fn config_name(self) -> &'static str {
        match self {
            Context::Global => "global",
            Context::Chat | Context::Voice => "chat",
            Context::Composer => "composer",
            Context::Editor => "editor",
            Context::VimNormal => "vim_normal",
            Context::VimOperator => "vim_operator",
            Context::VimSearch => "vim_search",
            Context::VimTextObject => "vim_text_object",
            Context::Pager => "pager",
            Context::List => "list",
            Context::Agents => "agents",
            Context::Approval => "approval",
        }
    }

    pub const fn is_vim(self) -> bool {
        matches!(
            self,
            Context::VimNormal | Context::VimOperator | Context::VimSearch | Context::VimTextObject
        )
    }

    /// Whether a chord may start with a plain character
    /// (`allows_plain_chord_prefix`).
    pub const fn allows_plain_chord_prefix(self) -> bool {
        self.is_vim()
    }

    /// The contexts a key reaches before the editor: the chat's shared
    /// handlers.
    const fn is_main(self) -> bool {
        matches!(
            self,
            Context::Global | Context::Chat | Context::Voice | Context::Composer
        )
    }

    const fn is_editor_side(self) -> bool {
        matches!(
            self,
            Context::Editor
                | Context::VimNormal
                | Context::VimOperator
                | Context::VimSearch
                | Context::VimTextObject
        )
    }

    /// Whether two contexts can be active together
    /// (`KeymapContext::overlaps`).
    pub fn overlaps(self, other: Context) -> bool {
        if self == other {
            return true;
        }
        matches!(
            (self, other),
            (
                Context::VimSearch,
                Context::VimNormal | Context::VimOperator
            ) | (
                Context::VimNormal | Context::VimOperator,
                Context::VimSearch
            ) | (Context::Voice, Context::Pager)
                | (Context::Pager, Context::Voice)
                | (Context::List, Context::Approval)
                | (Context::Approval, Context::List)
                | (Context::List, Context::Agents)
                | (Context::Agents, Context::List)
                | (Context::Chat, Context::List)
                | (Context::List, Context::Chat)
        ) || self.is_main() && other.is_editor_side()
            || other.is_main() && self.is_editor_side()
            || self.is_main() && other.is_main()
    }
}

/// An action's identity: its context and its name in `tui.keymap`
/// (`KeymapActionId`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId {
    pub context: Context,
    pub name: &'static str,
}

impl ActionId {
    pub const fn new(context: Context, name: &'static str) -> ActionId {
        ActionId { context, name }
    }

    /// `tui.keymap.<context>.<action>`, as an error names it.
    pub fn config_path(self) -> String {
        ["tui", "keymap", self.context.config_name(), self.name].join(".")
    }

    /// Whether two actions can be active together
    /// (`KeymapActionId::overlaps`).
    pub fn overlaps(self, other: ActionId) -> bool {
        let side_pane = |a: ActionId| a.context == Context::Global && SIDE_PANE.contains(&a.name);
        if side_pane(self) && matches!(other.context, Context::List | Context::Pager)
            || side_pane(other) && matches!(self.context, Context::List | Context::Pager)
        {
            return true;
        }
        match (self.context, other.context) {
            // Presets cannot open while a list owns input.
            (Context::Chat, Context::List) => self.name != "open_presets",
            (Context::List, Context::Chat) => other.name != "open_presets",
            // Activity focus, Find, and selection remain available during
            // list navigation.
            (Context::Global, Context::List) => matches!(
                self.name,
                "focus_activity" | "find_transcript" | "select_transcript"
            ),
            (Context::List, Context::Global) => matches!(
                other.name,
                "focus_activity" | "find_transcript" | "select_transcript"
            ),
            _ => self.context.overlaps(other.context),
        }
    }
}

/// The global actions of the side panes: the port has no workspace panes, so
/// a configured binding of one is dropped (the oracle: "inactive pane
/// shortcuts must not claim keys in classic mode").
pub const SIDE_PANE: &[&str] = &[
    "focus_main_composer",
    "focus_sidebar_content",
    "focus_sidebar_tabs",
    "hide_pane",
    "close_sidebar_tab",
    "sidebar_up",
    "sidebar_down",
    "focus_pane_left",
    "focus_pane_right",
    "minimize_pane_width",
    "maximize_pane_width",
    "equalize_pane_widths",
    "resize_panes",
    "move_divider_left",
    "move_divider_right",
    "toggle_pane_zoom",
];

/// One action: where it lives, its name and its built-in default specs.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub id: ActionId,
    /// The built-in default, in the order a hint shows them.
    pub defaults: &'static [&'static str],
}

const fn row(context: Context, name: &'static str, defaults: &'static [&'static str]) -> Row {
    Row {
        id: ActionId::new(context, name),
        defaults,
    }
}

use Context::{
    Agents, Approval, Chat, Composer, Editor, Global, List, Pager, VimNormal, VimOperator,
    VimSearch, VimTextObject, Voice,
};

/// Every configurable action, in the oracle's order.
pub const ROWS: &[Row] = &[
    row(Global, "change_host", &[]),
    row(Global, "open_agents", &[]),
    row(Global, "open_transcript", &["ctrl-t"]),
    row(Global, "find_transcript", &["f3"]),
    row(Global, "select_transcript", &[]),
    row(Global, "focus_activity", &["f4"]),
    row(Global, "focus_main_composer", &[]),
    row(Global, "focus_sidebar_content", &[]),
    row(Global, "focus_sidebar_tabs", &[]),
    row(Global, "hide_pane", &[]),
    row(Global, "close_sidebar_tab", &[]),
    row(Global, "sidebar_up", &[]),
    row(Global, "sidebar_down", &[]),
    row(Global, "focus_pane_left", &[]),
    row(Global, "focus_pane_right", &[]),
    row(Global, "minimize_pane_width", &[]),
    row(Global, "maximize_pane_width", &[]),
    row(Global, "equalize_pane_widths", &[]),
    row(Global, "resize_panes", &[]),
    row(Global, "move_divider_left", &[]),
    row(Global, "move_divider_right", &[]),
    row(Global, "toggle_pane_zoom", &[]),
    row(Global, "open_warnings", &["f2"]),
    row(Global, "open_external_editor", &["ctrl-g"]),
    row(Global, "copy", &["ctrl-o"]),
    row(Global, "clear_terminal", &["ctrl-l"]),
    row(Global, "toggle_vim_mode", &[]),
    row(Global, "toggle_fast_mode", &[]),
    row(Global, "toggle_raw_output", &["alt-r"]),
    row(Global, "toggle_side_conversation", &["ctrl-/"]),
    row(Chat, "open_presets", &[]),
    row(Chat, "toggle_voice", &["f8"]),
    row(Chat, "interrupt_turn", &["esc"]),
    row(Chat, "decrease_reasoning_effort", &["alt-,", "shift-down"]),
    row(Chat, "increase_reasoning_effort", &["alt-.", "shift-up"]),
    row(Chat, "previous_permission_mode", &[]),
    row(Chat, "next_permission_mode", &[]),
    row(Chat, "edit_queued_message", &["shift-left", "alt-up"]),
    row(Chat, "prompt_stack_back", &["shift-right", "alt-down"]),
    row(Chat, "skip_question", &["ctrl-]"]),
    row(Voice, "toggle_voice_mute", &["f9"]),
    row(Composer, "submit", &["enter"]),
    row(Composer, "queue", &["tab"]),
    row(Composer, "toggle_shortcuts", &["?", "shift-?"]),
    row(Composer, "history_search_previous", &["ctrl-r"]),
    row(Composer, "history_search_next", &["ctrl-s"]),
    row(
        Editor,
        "insert_newline",
        &["ctrl-j", "ctrl-m", "enter", "shift-enter", "alt-enter"],
    ),
    row(Editor, "move_left", &["left", "ctrl-b"]),
    row(Editor, "move_right", &["right", "ctrl-f"]),
    row(Editor, "move_up", &["up", "ctrl-p"]),
    row(Editor, "move_down", &["down", "ctrl-n"]),
    row(
        Editor,
        "move_word_left",
        &["alt-b", "alt-left", "ctrl-left"],
    ),
    row(
        Editor,
        "move_word_right",
        &["alt-f", "alt-right", "ctrl-right"],
    ),
    row(Editor, "move_line_start", &["home", "ctrl-a", "super-left"]),
    row(Editor, "move_line_end", &["end", "ctrl-e", "super-right"]),
    row(
        Editor,
        "delete_backward",
        &["backspace", "shift-backspace", "ctrl-h"],
    ),
    row(
        Editor,
        "delete_forward",
        &["delete", "shift-delete", "ctrl-d"],
    ),
    row(
        Editor,
        "delete_backward_word",
        &[
            "alt-backspace",
            "ctrl-backspace",
            "ctrl-shift-backspace",
            "ctrl-w",
            "ctrl-alt-h",
        ],
    ),
    row(
        Editor,
        "delete_forward_word",
        &["alt-delete", "ctrl-delete", "ctrl-shift-delete", "alt-d"],
    ),
    row(Editor, "kill_line_start", &["ctrl-u", "super-backspace"]),
    row(Editor, "kill_whole_line", &[]),
    row(Editor, "kill_line_end", &["ctrl-k"]),
    row(Editor, "yank", &["ctrl-y"]),
    row(VimNormal, "enter_insert", &["i", "insert"]),
    row(VimNormal, "append_after_cursor", &["a"]),
    row(VimNormal, "append_line_end", &["shift-a", "A"]),
    row(VimNormal, "insert_line_start", &["shift-i", "I"]),
    row(VimNormal, "open_line_below", &["o"]),
    row(VimNormal, "open_line_above", &["shift-o", "O"]),
    row(VimNormal, "enter_replace_mode", &["shift-r", "R"]),
    row(VimNormal, "move_left", &["h", "left"]),
    row(VimNormal, "move_right", &["l", "right"]),
    row(VimNormal, "move_up", &["k", "up"]),
    row(VimNormal, "move_down", &["j", "down"]),
    row(VimNormal, "move_word_forward", &["w"]),
    row(VimNormal, "move_word_backward", &["b"]),
    row(VimNormal, "move_word_end", &["e"]),
    row(VimNormal, "move_line_start", &["0"]),
    row(VimNormal, "move_line_end", &["$", "shift-$"]),
    row(VimNormal, "find_forward", &["f"]),
    row(VimNormal, "find_backward", &["shift-f"]),
    row(VimNormal, "till_forward", &["t"]),
    row(VimNormal, "till_backward", &["shift-t"]),
    row(VimNormal, "jump_top", &[]),
    row(VimNormal, "jump_bottom", &["shift-g", "G"]),
    row(VimNormal, "delete_char", &["x"]),
    row(VimNormal, "replace_char", &["r"]),
    row(VimNormal, "repeat_last_change", &["."]),
    row(VimNormal, "substitute_char", &["s"]),
    row(VimNormal, "delete_to_line_end", &["shift-d", "D"]),
    row(VimNormal, "change_to_line_end", &["shift-c", "C"]),
    row(VimNormal, "yank_line", &["shift-y", "Y"]),
    row(VimNormal, "paste_after", &["p"]),
    row(VimNormal, "start_delete_operator", &["d"]),
    row(VimNormal, "start_yank_operator", &["y"]),
    row(VimNormal, "start_change_operator", &["c"]),
    row(VimNormal, "undo", &["u"]),
    row(VimNormal, "redo", &["ctrl-r"]),
    row(VimNormal, "cancel_operator", &["esc"]),
    row(VimSearch, "forward", &["/"]),
    row(VimSearch, "backward", &["?", "shift-?"]),
    row(VimSearch, "next", &["n"]),
    row(VimSearch, "previous", &["N", "shift-n"]),
    row(VimOperator, "delete_line", &["d"]),
    row(VimOperator, "yank_line", &["y"]),
    row(VimOperator, "motion_left", &["h"]),
    row(VimOperator, "motion_right", &["l"]),
    row(VimOperator, "motion_up", &["k"]),
    row(VimOperator, "motion_down", &["j"]),
    row(VimOperator, "motion_word_forward", &["w"]),
    row(VimOperator, "motion_word_backward", &["b"]),
    row(VimOperator, "motion_word_end", &["e"]),
    row(VimOperator, "motion_line_start", &["0"]),
    row(VimOperator, "motion_line_end", &["$", "shift-$"]),
    row(VimOperator, "motion_find_forward", &["f"]),
    row(VimOperator, "motion_find_backward", &["shift-f"]),
    row(VimOperator, "motion_till_forward", &["t"]),
    row(VimOperator, "motion_till_backward", &["shift-t"]),
    row(VimOperator, "motion_jump_top", &[]),
    row(VimOperator, "motion_jump_bottom", &["shift-g", "G"]),
    row(VimOperator, "select_inner_text_object", &["i"]),
    row(VimOperator, "select_around_text_object", &["a"]),
    row(VimOperator, "cancel", &["esc"]),
    row(VimTextObject, "word", &["w"]),
    row(VimTextObject, "big_word", &["shift-w", "W"]),
    row(
        VimTextObject,
        "parentheses",
        &["(", "shift-(", ")", "shift-)", "b"],
    ),
    row(VimTextObject, "brackets", &["[", "]"]),
    row(
        VimTextObject,
        "braces",
        &["{", "shift-{", "}", "shift-}", "shift-b", "B"],
    ),
    row(VimTextObject, "double_quote", &["\"", "shift-\""]),
    row(VimTextObject, "single_quote", &["'"]),
    row(VimTextObject, "backtick", &["`"]),
    row(VimTextObject, "cancel", &["esc"]),
    row(Pager, "scroll_up", &["up", "k"]),
    row(Pager, "scroll_down", &["down", "j"]),
    row(Pager, "page_up", &["page-up", "shift-space", "ctrl-b"]),
    row(Pager, "page_down", &["page-down", "space", "ctrl-f"]),
    row(Pager, "half_page_up", &["ctrl-u"]),
    row(Pager, "half_page_down", &["ctrl-d"]),
    row(Pager, "jump_top", &["home"]),
    row(Pager, "jump_bottom", &["end"]),
    row(Pager, "close", &["q", "ctrl-c"]),
    row(Pager, "close_transcript", &["ctrl-t"]),
    row(Pager, "find", &["f3", "/"]),
    row(List, "move_up", &["up", "ctrl-p", "ctrl-k", "k"]),
    row(List, "move_down", &["down", "ctrl-n", "ctrl-j", "j"]),
    row(List, "move_left", &["left", "ctrl-h"]),
    row(List, "move_right", &["right", "ctrl-l"]),
    row(List, "page_up", &["page-up", "ctrl-b"]),
    row(List, "page_down", &["page-down", "ctrl-f"]),
    row(List, "jump_top", &["home"]),
    row(List, "jump_bottom", &["end"]),
    row(List, "accept", &["enter"]),
    row(List, "cancel", &["esc"]),
    row(Agents, "resume", &["o"]),
    row(Agents, "search", &["f3", "/"]),
    row(Agents, "new_task", &["n"]),
    row(Agents, "new_worktree", &["w"]),
    row(Agents, "fork", &["f"]),
    row(Agents, "rename", &["r"]),
    row(Agents, "stop", &["x"]),
    row(Agents, "archive", &["a"]),
    row(Agents, "delete", &["backspace"]),
    row(Agents, "hide", &["h"]),
    row(Agents, "toggle_pin", &["p"]),
    row(Agents, "toggle_grouping", &["g"]),
    row(Approval, "open_fullscreen", &["ctrl-a", "ctrl-shift-a"]),
    row(Approval, "open_thread", &["o"]),
    row(Approval, "approve", &["y"]),
    row(Approval, "approve_for_session", &["a"]),
    row(Approval, "approve_for_prefix", &["p"]),
    row(Approval, "deny", &["d"]),
    row(Approval, "decline", &["esc", "n"]),
    row(Approval, "cancel", &["c"]),
];

/// Actions read from `global` when their own context sets none
/// (`configured_binding_for_action`).
pub const GLOBAL_FALLBACK: &[ActionId] = &[
    ActionId::new(Composer, "submit"),
    ActionId::new(Composer, "queue"),
    ActionId::new(Composer, "toggle_shortcuts"),
];

/// Actions no config sets (`runtime_only`): the default is the binding.
pub const RUNTIME_ONLY: &[ActionId] = &[ActionId::new(Global, "open_warnings")];

/// Defaults added after the first release: they yield to any binding the
/// user set for the same key in a context that can be active together
/// (`focus_activity_defaults`, `voice_mute_default_is_shadowed`,
/// `resolve_new_default_bindings`, the vim `..._to_preserve` lists): a new
/// default never invalidates an existing config.
pub fn yields_to_configured(id: ActionId) -> bool {
    id.context.is_vim()
        || matches!(
            (id.context, id.name),
            (
                Global,
                "focus_activity" | "open_warnings" | "toggle_side_conversation"
            ) | (Voice, "toggle_voice_mute")
                | (
                    Chat,
                    "decrease_reasoning_effort" | "increase_reasoning_effort"
                )
                | (
                    List,
                    "move_left"
                        | "move_right"
                        | "page_up"
                        | "page_down"
                        | "jump_top"
                        | "jump_bottom"
                )
        )
}

/// The keys the app reserves (`MAIN_RESERVED_BINDINGS`): the name of the
/// fixed shortcut and its spec.
pub const MAIN_RESERVED: &[(&str, &str)] = &[
    ("fixed.interrupt_or_quit", "ctrl-c"),
    ("fixed.quit", "ctrl-d"),
    ("fixed.paste_image", "ctrl-v"),
    ("fixed.paste_image", "ctrl-alt-v"),
    ("fixed.cycle_collaboration_mode", "shift-tab"),
    ("fixed.backtrack", "esc"),
    ("fixed.previous_agent", "alt-left"),
    ("fixed.next_agent", "alt-right"),
    ("fixed.slash_command", "/"),
    ("fixed.shell_command", "!"),
    ("fixed.file_paths", "@"),
    ("fixed.connector_mentions", "$"),
];

/// The keys the transcript's backtrack takes
/// (`TRANSCRIPT_BACKTRACK_RESERVED_BINDINGS`).
pub const TRANSCRIPT_RESERVED: &[(&str, &str)] = &[
    ("fixed.transcript_edit_previous", "esc"),
    ("fixed.transcript_edit_previous", "left"),
    ("fixed.transcript_edit_next", "right"),
    ("fixed.transcript_confirm_edit", "enter"),
];

/// The leader's built-in prefix (`leader::resolve`).
pub const DEFAULT_LEADER: &str = "ctrl-x";

/// The shortcuts a free leader chord gets without a config
/// (`leader_defaults::register`): context, action and the completion after
/// the leader.
pub const LEADER_DEFAULTS: &[(Context, &str, &str)] = &[
    (Global, "change_host", "b"),
    (Global, "select_transcript", "["),
    (Chat, "open_presets", "p"),
    (Global, "focus_main_composer", "c"),
    (Global, "focus_sidebar_content", "s"),
    (Global, "hide_pane", "-"),
    (Global, "focus_pane_left", "left"),
    (Global, "focus_pane_left", "h"),
    (Global, "focus_pane_right", "right"),
    (Global, "focus_pane_right", "l"),
    (Global, "toggle_pane_zoom", "z"),
    (Global, "move_divider_left", "shift-h"),
    (Global, "move_divider_right", "shift-l"),
    (Global, "equalize_pane_widths", "="),
    (Global, "resize_panes", "r"),
    (Global, "minimize_pane_width", ","),
    (Global, "maximize_pane_width", "."),
    (Global, "close_sidebar_tab", "w"),
    (Global, "focus_sidebar_tabs", "t"),
    (Global, "sidebar_up", "shift-tab"),
    (Global, "sidebar_down", "tab"),
    (Global, "sidebar_up", "up"),
    (Global, "sidebar_up", "k"),
    (Global, "sidebar_down", "down"),
    (Global, "sidebar_down", "j"),
];

/// The keys a list reserves for a chord (`LIST_RESERVED_BINDINGS`).
pub const LIST_RESERVED: &[(&str, &str)] = &[
    ("cancel", "ctrl-c"),
    ("resume_picker.toggle_transcript", "ctrl-t"),
    ("resume_picker.toggle_expansion", "ctrl-e"),
    ("resume_picker.toggle_density", "ctrl-o"),
];

/// Actions whose key may not be a character a text field takes
/// (`validate_conflicts`: "printable keys are reserved for text input").
pub const PRINTABLE_RESERVED: &[ActionId] = &[
    ActionId::new(Global, "change_host"),
    ActionId::new(Chat, "open_presets"),
    ActionId::new(Chat, "toggle_voice"),
    ActionId::new(Chat, "previous_permission_mode"),
    ActionId::new(Chat, "next_permission_mode"),
    ActionId::new(Chat, "prompt_stack_back"),
    ActionId::new(Chat, "skip_question"),
];

/// Actions that may not take ctrl-z, which suspends the terminal.
pub const CTRL_Z_RESERVED: &[ActionId] = &[
    ActionId::new(Global, "change_host"),
    ActionId::new(Global, "open_agents"),
    ActionId::new(Global, "focus_main_composer"),
    ActionId::new(Global, "focus_sidebar_content"),
    ActionId::new(Global, "focus_sidebar_tabs"),
    ActionId::new(Global, "focus_pane_left"),
    ActionId::new(Global, "focus_pane_right"),
    ActionId::new(Global, "minimize_pane_width"),
    ActionId::new(Global, "maximize_pane_width"),
    ActionId::new(Global, "equalize_pane_widths"),
    ActionId::new(Global, "resize_panes"),
    ActionId::new(Global, "move_divider_left"),
    ActionId::new(Global, "move_divider_right"),
    ActionId::new(Global, "toggle_pane_zoom"),
    ActionId::new(Global, "hide_pane"),
    ActionId::new(Global, "close_sidebar_tab"),
    ActionId::new(Global, "sidebar_up"),
    ActionId::new(Global, "sidebar_down"),
    ActionId::new(Chat, "open_presets"),
    ActionId::new(Chat, "toggle_voice"),
];

/// Pairs that may share a key (`allowed_overlaps` of the oracle's passes):
/// the first action's binding wins where both are active.
pub const ALLOWED_OVERLAPS: &[(ActionId, ActionId, &str)] = &[
    (
        ActionId::new(Global, "clear_terminal"),
        ActionId::new(List, "move_right"),
        "ctrl-l",
    ),
    (
        ActionId::new(Composer, "submit"),
        ActionId::new(Editor, "insert_newline"),
        "enter",
    ),
];

/// A group of actions whose single keys the oracle checks together
/// (`validate_unique` and `validate_no_shadow_with_allowed_overlaps` of
/// `validate_conflicts`): within `primary` no two actions may share a key;
/// with `shadowed`, no primary action may take a key a shadowed one has.
pub struct Scope {
    /// The `tui.keymap.<name>` an error names.
    pub name: &'static str,
    pub primary: fn(ActionId) -> bool,
    pub shadowed: Option<fn(ActionId) -> bool>,
    /// Whether an error names actions bare (`move_up`) or with their context.
    pub bare: bool,
}

fn is_app(id: ActionId) -> bool {
    matches!(id.context, Global | Chat | Voice | Composer)
}

/// The global actions that run before an overlay reads a key.
fn is_app_over_overlay(id: ActionId) -> bool {
    id.context == Global
        && matches!(
            id.name,
            "open_agents"
                | "open_transcript"
                | "find_transcript"
                | "select_transcript"
                | "focus_activity"
                | "open_warnings"
                | "open_external_editor"
                | "copy"
                | "clear_terminal"
                | "toggle_vim_mode"
                | "toggle_fast_mode"
                | "toggle_raw_output"
                | "toggle_side_conversation"
        )
}

fn is_overlay(id: ActionId) -> bool {
    matches!(id.context, List | Approval)
}

fn is_editor_key_of_main(id: ActionId) -> bool {
    id.context == Editor
        && matches!(
            id.name,
            "insert_newline"
                | "move_word_left"
                | "move_word_right"
                | "move_line_start"
                | "delete_backward"
                | "delete_forward"
                | "delete_backward_word"
                | "delete_forward_word"
                | "kill_line_start"
                | "kill_whole_line"
        )
}

fn in_list_nav(id: ActionId, names: &[&str]) -> bool {
    id.context == List && names.contains(&id.name)
}

/// Every scope, in the oracle's order.
pub const SCOPES: &[Scope] = &[
    Scope {
        name: "app",
        primary: is_app,
        shadowed: None,
        bare: false,
    },
    Scope {
        name: "app",
        primary: is_app_over_overlay,
        shadowed: Some(is_overlay),
        bare: false,
    },
    // The request-user-input overlay consumes turn interruption before its
    // question navigation reaches the list.
    Scope {
        name: "request_user_input",
        primary: |id| id == ActionId::new(Chat, "interrupt_turn"),
        shadowed: Some(|id| in_list_nav(id, &["move_left", "move_right"])),
        bare: false,
    },
    // Async question shortcuts run before option selection and text editing.
    Scope {
        name: "async_questions",
        primary: |id| {
            id.context == Chat && matches!(id.name, "prompt_stack_back" | "skip_question")
        },
        shadowed: Some(|id| in_list_nav(id, &["move_up", "move_down", "accept"])),
        bare: false,
    },
    // While the composer is focused, the main-surface handlers consume
    // matching keys before the event reaches the textarea editor.
    Scope {
        name: "main",
        primary: is_app,
        shadowed: Some(is_editor_key_of_main),
        bare: false,
    },
    Scope {
        name: "editor",
        primary: |id| id.context == Editor,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "vim_normal",
        primary: |id| id.context == VimNormal,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "vim_operator",
        primary: |id| id.context == VimOperator,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "vim_text_object",
        primary: |id| id.context == VimTextObject,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "pager",
        primary: |id| id.context == Pager,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "transcript",
        primary: |id| {
            id == ActionId::new(Global, "select_transcript")
                || id == ActionId::new(Pager, "close_transcript")
        },
        shadowed: None,
        bare: false,
    },
    Scope {
        name: "list",
        primary: |id| id.context == List,
        shadowed: None,
        bare: true,
    },
    // Activity navigation joins the list's actions with the global Find and
    // selection.
    Scope {
        name: "activity",
        primary: |id| {
            id.context == List
                || id.context == Global
                    && matches!(id.name, "find_transcript" | "select_transcript")
        },
        shadowed: None,
        bare: false,
    },
    Scope {
        name: "agents",
        primary: |id| id.context == Agents,
        shadowed: None,
        bare: true,
    },
    Scope {
        name: "approval",
        primary: |id| id.context == Approval,
        shadowed: None,
        bare: true,
    },
];

/// Whether the oracle checks the single keys of two actions against each
/// other.
pub fn scopes_conflict(a: ActionId, b: ActionId) -> bool {
    SCOPES.iter().any(|scope| match scope.shadowed {
        None => (scope.primary)(a) && (scope.primary)(b),
        Some(shadowed) => (scope.primary)(a) && shadowed(b) || (scope.primary)(b) && shadowed(a),
    })
}
