//! The strings of the `/keymap` editor (oracle `keymap_setup/{picker,actions,capture,debug}.rs`
//! and `keymap_setup.rs`): the text of the picker's tabs, the action menu, the replace menu, the
//! conflict sheet, the capture view and the keypress inspector, byte for byte. A table per locale
//! read through `of(cx)`, as the other tables of the slice; the strings of the picker's frame
//! (title, placeholder, hint) are the settings table's.

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

use crate::l10n::{Tables, lang};

/// What a tab of the picker says of itself: its line under the title, and the row that stands
/// in for its actions when it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabText {
    pub id: &'static str,
    pub description: &'static str,
    pub empty_name: &'static str,
    pub empty_description: &'static str,
}

const fn tab(
    id: &'static str,
    description: &'static str,
    empty_name: &'static str,
    empty_description: &'static str,
) -> TabText {
    TabText {
        id,
        description,
        empty_name,
        empty_description,
    }
}

/// The strings of the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    pub num: NumSyms,
    // keymap_setup/picker.rs
    pub tabs: [TabText; 12],
    pub all_summary: Msg<3>,
    pub debug_row: &'static str,
    pub debug_row_hint: &'static str,
    pub debug_row_selected: &'static str,
    pub debug_row_search: &'static str,
    pub debug_footer: &'static str,
    pub debug_summary: &'static str,
    // keymap_setup.rs: the action menu
    pub menu_title: &'static str,
    pub menu_current: &'static str,
    pub menu_config: &'static str,
    pub menu_fallback_description: &'static str,
    pub menu_source_custom: &'static str,
    pub menu_source_default: &'static str,
    pub menu_footer_note_open: &'static str,
    pub menu_footer_note_path: &'static str,
    pub menu_footer_note_close: &'static str,
    pub set_key: &'static str,
    pub set_key_hint: &'static str,
    pub replace_binding: &'static str,
    pub replace_binding_hint: Msg<1>,
    pub add_alternate: &'static str,
    pub add_alternate_hint: Msg<1>,
    pub replace_one: &'static str,
    pub replace_one_hint: &'static str,
    pub replace_all: &'static str,
    pub replace_all_hint: Msg<1>,
    pub use_leader: &'static str,
    pub use_leader_hint: &'static str,
    pub leader_unbound: &'static str,
    pub configure_leader: &'static str,
    pub configure_leader_hint: &'static str,
    pub set_chord: &'static str,
    pub set_chord_hint: &'static str,
    pub replace_chord: &'static str,
    pub replace_chord_hint: Msg<1>,
    pub add_chord: &'static str,
    pub add_chord_hint: Msg<1>,
    pub remove_custom: &'static str,
    pub remove_custom_hint: &'static str,
    pub remove_custom_disabled: &'static str,
    pub gutter_disabled: &'static str,
    pub back: &'static str,
    pub back_hint: &'static str,
    pub unbound: &'static str,
    // the replace menu
    pub replace_menu_title: &'static str,
    pub replace_menu_pick: &'static str,
    pub chord_suffix: Msg<1>,
    pub replace_one_key_hint: Msg<1>,
    pub replace_one_chord_hint: Msg<1>,
    // the conflict sheet
    pub conflict_title: &'static str,
    pub conflict_line: Msg<3>,
    pub pick_another: &'static str,
    pub pick_another_hint: &'static str,
    pub cancel: &'static str,
    pub cancel_hint: &'static str,
    // keymap_setup/capture.rs
    pub capture_title: &'static str,
    pub capture_action: &'static str,
    pub capture_current: &'static str,
    pub capture_single: &'static str,
    pub capture_chord_first: &'static str,
    pub capture_chord_second: Msg<1>,
    pub capture_leader: &'static str,
    pub capture_error: &'static str,
    // the errors of a key that cannot be stored in the config
    pub key_modifiers: &'static str,
    pub key_function: Msg<1>,
    pub key_printable: &'static str,
    pub key_unsupported: &'static str,
    // keymap_setup/debug.rs
    pub debug_title: &'static str,
    pub debug_intro: &'static str,
    pub debug_tip: &'static str,
    pub debug_tip_delayed: &'static str,
    pub debug_waiting: &'static str,
    pub debug_detected: &'static str,
    pub debug_config_key: &'static str,
    pub debug_unsupported: Msg<1>,
    pub debug_raw: &'static str,
    pub debug_assigned: &'static str,
    pub debug_none: &'static str,
    pub debug_match: Msg<5>,
    pub source_custom: &'static str,
    pub source_custom_global: &'static str,
    pub source_default: &'static str,
    pub modifiers_none: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    num: NumSyms::ENGLISH,
    tabs: [
        tab(
            "all",
            "All configurable shortcuts.",
            "No shortcuts available",
            "No configurable shortcuts are available",
        ),
        tab(
            "common",
            "Frequently customized shortcuts.",
            "No common shortcuts",
            "No common shortcut actions are available",
        ),
        tab(
            "customized",
            "Root-level shortcut overrides.",
            "No customized shortcuts",
            "No root-level keymap overrides have been configured",
        ),
        tab(
            "unbound",
            "Actions without an active shortcut.",
            "No unbound shortcuts",
            "Every configurable action currently has a shortcut",
        ),
        tab(
            "app",
            "Global and chat-level shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "composer",
            "Composer submission and queue shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "editor",
            "Inline editor movement and editing shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "vim",
            "Vim normal-mode and operator shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "navigation",
            "Pager and selection-list navigation shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "agents",
            "Shared agents dashboard shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "approval",
            "Approval prompt shortcuts.",
            "No shortcuts in this group",
            "No configurable actions are available in this group",
        ),
        tab(
            "debug",
            "Inspect keypresses from your terminal.",
            "",
            "",
        ),
    ],
    all_summary: msg!("{0} actions, {1} customized, {2} unbound."),
    debug_row: "Inspect keypresses",
    debug_row_hint: "Press Enter, then any key to inspect it (Ctrl+C exits)",
    debug_row_selected: "Open a live inspector that shows the detected key, config key, and matching actions",
    debug_row_search: "debug inspect keypress key terminal detected actions",
    debug_footer: "enter start inspector \u{b7} esc close",
    debug_summary: "See the key Codex detects and any shortcuts assigned to it.",
    menu_title: "Edit Shortcut",
    menu_current: "Current ",
    menu_config: "Config ",
    menu_fallback_description: "Configure this shortcut.",
    menu_source_custom: "Custom root override",
    menu_source_default: "Default keymap",
    menu_footer_note_open: "Changes write the root ",
    menu_footer_note_path: "`tui.keymap.*`",
    menu_footer_note_close: " override.",
    set_key: "Set key",
    set_key_hint: "Capture a key for this unbound action",
    replace_binding: "Replace binding",
    replace_binding_hint: msg!("Capture a replacement key for `{0}`"),
    add_alternate: "Add alternate binding",
    add_alternate_hint: msg!("Keep `{0}` and add another key"),
    replace_one: "Replace one binding...",
    replace_one_hint: "Choose which existing binding to replace",
    replace_all: "Replace all bindings",
    replace_all_hint: msg!("Replace `{0}` with one key"),
    use_leader: "Use leader...",
    use_leader_hint: "Capture the key that follows the configured leader",
    leader_unbound: "The leader is unbound",
    configure_leader: "Configure leader...",
    configure_leader_hint: "Set a leader before assigning this shortcut",
    set_chord: "Set key chord",
    set_chord_hint: "Capture two consecutive keys for this action",
    replace_chord: "Replace with key chord",
    replace_chord_hint: msg!("Replace `{0}` with a two-stroke key chord"),
    add_chord: "Add alternate key chord",
    add_chord_hint: msg!("Keep `{0}` and add a two-stroke key chord"),
    remove_custom: "Remove custom binding",
    remove_custom_hint: "Restore the default keymap binding",
    remove_custom_disabled: "No custom root override to remove",
    gutter_disabled: "\u{2013}",
    back: "Back to shortcuts",
    back_hint: "Return to the shortcut list",
    unbound: "unbound",
    replace_menu_title: "Replace Binding",
    replace_menu_pick: "Choose the binding to replace.",
    chord_suffix: msg!("{0} (key chord)"),
    replace_one_key_hint: msg!("Replace `{0}` with another key"),
    replace_one_chord_hint: msg!("Replace `{0}` with a two-stroke key chord"),
    conflict_title: "Shortcut Conflict",
    conflict_line: msg!("{0}.{1} cannot use `{2}`."),
    pick_another: "Pick another key",
    pick_another_hint: "Return to key capture for this action",
    cancel: "Cancel",
    cancel_hint: "Leave keymap unchanged",
    capture_title: "Remap Shortcut",
    capture_action: "Action: ",
    capture_current: "Current: ",
    capture_single: "Press the new key now. Esc cancels.",
    capture_chord_first: "Press the first key, then the second. Esc cancels.",
    capture_chord_second: msg!("First key: {0}. Press the second key. Esc cancels."),
    capture_leader: "Press the key that follows the configured leader. Esc cancels.",
    capture_error: "Error: ",
    key_modifiers: "Only ctrl, alt, shift, and super modifiers can be stored in `tui.keymap`.",
    key_function: msg!("Only function keys F1 through F{0} can be stored in `tui.keymap`."),
    key_printable: "Only printable ASCII keys can be stored in `tui.keymap`.",
    key_unsupported: "That key is not supported by `tui.keymap`.",
    debug_title: "Keypress Inspector",
    debug_intro: "Press any key to see what Codex receives. Esc is inspected; Ctrl+C closes.",
    debug_tip: "Tip: Codex can only inspect keys your terminal sends.",
    debug_tip_delayed: "Still waiting? If nothing changes when you press a key, your terminal is not sending that key to Codex. Only received keys can be assigned as shortcuts.",
    debug_waiting: "Waiting for a keypress...",
    debug_detected: "Detected: ",
    debug_config_key: "Config key: ",
    debug_unsupported: msg!("unsupported - {0}"),
    debug_raw: "Raw event: ",
    debug_assigned: "Assigned actions:",
    debug_none: "  none",
    debug_match: msg!("{0}.{1} ({2}) - {3} [{4}]"),
    source_custom: "Custom",
    source_custom_global: "Custom global",
    source_default: "Default",
    modifiers_none: "none",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

impl Strings {
    /// The words of a tab of the picker, by its id.
    pub fn tab(&self, id: &str) -> Option<&TabText> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    /// "153 actions, 0 customized, 5 unbound.".
    pub fn all_summary(&self, total: usize, custom: usize, unbound: usize) -> Fmt<'_, 3> {
        Fmt::new(
            self.all_summary,
            [
                Arg::Int(total as u64),
                Arg::Int(custom as u64),
                Arg::Int(unbound as u64),
            ],
            &self.num,
        )
    }

    /// A message of one argument.
    pub fn with<'a>(&'a self, msg: Msg<1>, arg: &'a str) -> Fmt<'a, 1> {
        Fmt::new(msg, [Arg::Text(arg)], &self.num)
    }

    /// "ctrl-t": the conflict sheet's line.
    pub fn conflict<'a>(&'a self, context: &'a str, action: &'a str, key: &'a str) -> Fmt<'a, 3> {
        Fmt::new(
            self.conflict_line,
            [Arg::Text(context), Arg::Text(action), Arg::Text(key)],
            &self.num,
        )
    }

    /// "Only function keys F1 through F24 can be stored in `tui.keymap`.".
    pub fn function_keys(&self, last: u8) -> Fmt<'_, 1> {
        Fmt::new(self.key_function, [Arg::Int(u64::from(last))], &self.num)
    }

    /// The line of an action that has the key under inspection.
    pub fn matched<'a>(
        &'a self,
        context: &'a str,
        action: &'a str,
        label: &'a str,
        description: &'a str,
        source: &'a str,
    ) -> Fmt<'a, 5> {
        Fmt::new(
            self.debug_match,
            [
                Arg::Text(context),
                Arg::Text(action),
                Arg::Text(label),
                Arg::Text(description),
                Arg::Text(source),
            ],
            &self.num,
        )
    }
}
