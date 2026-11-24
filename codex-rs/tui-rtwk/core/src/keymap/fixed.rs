//! The keys Codex hardwires, the ones its `tui.keymap` has no action for (Up and Down walk the
//! prompt history, Enter and Tab of a popup, Esc, the arrows over a placeholder, the kill keys the
//! field's own editing answers to), as actions with fixed bindings and no config key. In Flutter
//! these are the `Shortcuts` entries an app writes where no user setting exists: the key is
//! still named by an `Intent` (an [`ActionId`] here), never spelled in the widget. A page asks
//! `pressed(keymap, id, key)` for them as for any action (`src/keymap`), and builds the press an
//! action stands for with `press_of`. Not rows of the catalogue: the user cannot rebind them, the
//! resolver does not see them, and a test keeps their names apart from the catalogue's.

use super::catalogue::{ActionId, Context};
use super::key::{Key, Mods};

/// Which modifiers a fixed binding takes (the oracle matches some keys by code alone, some by the
/// exact chord, `Alt+V` and `Ctrl+V` as "holds Ctrl or Alt").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModsRule {
    /// Whatever is held: the page reads the modifiers itself where they matter.
    Any,
    /// Exactly these.
    Exact(Mods),
    /// At least these.
    Has(Mods),
}

impl ModsRule {
    /// Whether the modifiers held satisfy the rule.
    pub fn accepts(self, held: Mods) -> bool {
        match self {
            ModsRule::Any => true,
            ModsRule::Exact(mods) => held == mods,
            ModsRule::Has(mods) => held.contains(mods),
        }
    }

    /// The modifiers of the press this rule stands for.
    pub const fn pressed(self) -> Mods {
        match self {
            ModsRule::Any => Mods::NONE,
            ModsRule::Exact(mods) | ModsRule::Has(mods) => mods,
        }
    }
}

/// One key of a fixed action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedBinding {
    pub key: Key,
    pub mods: ModsRule,
}

const fn any(key: Key) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Any,
    }
}

const fn exact(key: Key, mods: Mods) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Exact(mods),
    }
}

const fn has(key: Key, mods: Mods) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Has(mods),
    }
}

const fn composer(name: &'static str) -> ActionId {
    ActionId::new(Context::Composer, name)
}

const fn editor(name: &'static str) -> ActionId {
    ActionId::new(Context::Editor, name)
}

/// The field's own Enter and Tab (submit and queue are the keymap's, these are what is left).
pub const ENTER: ActionId = composer("enter");
pub const TAB: ActionId = composer("tab");
/// Esc: closes a popup, leaves shell mode or a search.
pub const DISMISS: ActionId = composer("dismiss");
/// Up and Down walk the prompt history at the draft's ends (and the remote image rows).
pub const HISTORY_PREVIOUS: ActionId = composer("history_previous");
pub const HISTORY_NEXT: ActionId = composer("history_next");
/// A popup's highlight (`@` mentions, slash commands).
pub const POPUP_PREVIOUS: ActionId = composer("popup_previous");
pub const POPUP_NEXT: ActionId = composer("popup_next");
/// Enter picks the highlighted row, Tab completes it as text.
pub const POPUP_ACCEPT: ActionId = composer("popup_accept");
pub const POPUP_COMPLETE: ActionId = composer("popup_complete");
/// Left and Right switch the `@` popup's tab while the caret is in the token.
pub const POPUP_TAB_PREVIOUS: ActionId = composer("popup_tab_previous");
pub const POPUP_TAB_NEXT: ActionId = composer("popup_tab_next");
/// Ctrl+V or Alt+V: the clipboard's image.
pub const PASTE_IMAGE: ActionId = composer("paste_image");
/// Ctrl+C on a draft with text clears it before it interrupts or quits.
pub const CLEAR_FOR_CTRL_C: ActionId = composer("clear_for_ctrl_c");
/// The keys of a history search (Ctrl+R) and of Vim's `/` query.
pub const SEARCH_ACCEPT: ActionId = composer("search_accept");
pub const SEARCH_CANCEL: ActionId = composer("search_cancel");
pub const SEARCH_ERASE: ActionId = composer("search_erase");
pub const SEARCH_CLEAR: ActionId = composer("search_clear");
/// The ETX byte (`0x03`) a terminal without the kitty protocol sends for Ctrl+C, typed as text
/// into a search: it cancels like Esc.
pub const SEARCH_CANCEL_BYTE: ActionId = composer("search_cancel_byte");
/// `!` on an empty draft turns it into a shell command (`shell_commands_enabled`), and `/` while a
/// command's name is typed completes the highlighted command as text.
pub const SHELL_PREFIX: ActionId = composer("shell_prefix");
pub const SLASH_COMPLETE: ActionId = composer("slash_complete");
/// The port's own hidden debug key: no field of the oracle's swallows it.
pub const DEBUG_KEY: ActionId = ActionId::new(Context::Global, "debug_key");

/// The keys of the field's own editing, each the one rtwk's editing and Codex's placeholder-aware
/// editing answer to for the Editor action of that name (`Edit::canonical`).
pub const FIELD_LEFT: ActionId = editor("field_left");
pub const FIELD_RIGHT: ActionId = editor("field_right");
pub const FIELD_UP: ActionId = editor("field_up");
pub const FIELD_DOWN: ActionId = editor("field_down");
pub const FIELD_WORD_LEFT: ActionId = editor("field_word_left");
pub const FIELD_WORD_RIGHT: ActionId = editor("field_word_right");
pub const FIELD_LINE_START: ActionId = editor("field_line_start");
pub const FIELD_LINE_END: ActionId = editor("field_line_end");
pub const FIELD_DELETE_BACKWARD: ActionId = editor("field_delete_backward");
pub const FIELD_DELETE_FORWARD: ActionId = editor("field_delete_forward");
pub const FIELD_DELETE_BACKWARD_WORD: ActionId = editor("field_delete_backward_word");
pub const FIELD_DELETE_FORWARD_WORD: ActionId = editor("field_delete_forward_word");
pub const FIELD_KILL_LINE_START: ActionId = editor("field_kill_line_start");
pub const FIELD_KILL_LINE_END: ActionId = editor("field_kill_line_end");
pub const FIELD_YANK: ActionId = editor("field_yank");

const fn list(name: &'static str) -> ActionId {
    ActionId::new(Context::List, name)
}

const fn chat(name: &'static str) -> ActionId {
    ActionId::new(Context::Chat, name)
}

/// The /plugins browser's own keys (`handle_plugins_popup_key_event`): Tab flips the row under the
/// cursor, Ctrl+R removes and Ctrl+U upgrades the marketplace the tab shows.
pub const PLUGINS_TOGGLE: ActionId = list("plugins_toggle");
pub const PLUGINS_REMOVE: ActionId = list("plugins_remove");
pub const PLUGINS_UPGRADE: ActionId = list("plugins_upgrade");
/// The /feedback note step's own keys: PageUp and PageDown scroll the disclosure above the editor,
/// Ctrl+S saves the report to a local zip instead of sending it.
pub const FEEDBACK_SCROLL_UP: ActionId = list("feedback_scroll_up");
pub const FEEDBACK_SCROLL_DOWN: ActionId = list("feedback_scroll_down");
pub const FEEDBACK_SAVE: ActionId = list("feedback_save");
/// PageUp and PageDown turn the chat's transcript a page wherever the caret is.
pub const TRANSCRIPT_PAGE_UP: ActionId = chat("transcript_page_up");
pub const TRANSCRIPT_PAGE_DOWN: ActionId = chat("transcript_page_down");
/// Shift+Tab cycles the collaboration mode (the oracle's reserved `fixed.cycle_collaboration_mode`).
pub const CYCLE_COLLABORATION_MODE: ActionId = chat("cycle_collaboration_mode");
/// Left on an empty composer opens the agents page.
pub const OPEN_AGENTS_FROM_LEFT: ActionId = chat("open_agents_from_left");
/// Left and Right while the prompts are browsed (Esc Esc) or in the transcript sheet: the older
/// and the newer prompt.
pub const BROWSE_OLDER: ActionId = chat("browse_older");
pub const BROWSE_NEWER: ActionId = chat("browse_newer");
/// `h` and `l` while the prompts are browsed: the same two steps, vim's way.
pub const BROWSE_OLDER_VIM: ActionId = chat("browse_older_vim");
pub const BROWSE_NEWER_VIM: ActionId = chat("browse_newer_vim");
/// Esc on the chat: the page's own ladder (close the overlay, leave for another page, scroll back,
/// interrupt, prime the backtrack); the catalogue's `interrupt_turn` is only one rung of it.
pub const ESCAPE: ActionId = chat("escape");
/// Ctrl+C on the chat: interrupts a running turn, else quits (the shortcut overlay and the usage
/// page name it too).
pub const INTERRUPT_OR_QUIT: ActionId = chat("interrupt_or_quit");
/// Ctrl+D on the chat: quits when the composer is empty and no turn runs.
pub const QUIT_ON_EMPTY: ActionId = chat("quit_on_empty");
/// `?` on an empty composer: the shortcut overlay.
pub const OPEN_SHORTCUTS: ActionId = chat("open_shortcuts");
/// The keys of a thread another process writes: retry the writer, fork a copy, quit.
pub const RETRY_WRITER: ActionId = chat("retry_writer");
pub const FORK_WRITER: ActionId = chat("fork_writer");
pub const QUIT_READ_ONLY: ActionId = chat("quit_read_only");
/// The keys of the notice a model provider's refusal leaves: a fresh chat, or this one continued.
pub const PROVIDER_NEW: ActionId = chat("provider_new");
pub const PROVIDER_CONTINUE: ActionId = chat("provider_continue");
/// The keys the shortcut overlay (`?`) lists that Codex hardwires and no handler of ours reads
/// through the keymap yet (the terminal or the field's own editing answers): hint-only actions,
/// named so the overlay asks for a stroke, never spells one.
/// Alt+Left and Alt+Right: the previous and the next subagent thread (empty prompt).
pub const SUBAGENT_PREVIOUS: ActionId = chat("subagent_previous");
pub const SUBAGENT_NEXT: ActionId = chat("subagent_next");
/// The transcript's own selection and jumps: Ctrl+Space starts a selection, Ctrl+Home and Ctrl+End
/// (or Alt+Shift+, and Alt+Shift+.) go to the top and the latest.
pub const SELECTION_START: ActionId = ActionId::new(Context::Pager, "selection_start");
pub const TRANSCRIPT_TOP: ActionId = ActionId::new(Context::Pager, "transcript_top");
pub const TRANSCRIPT_LATEST: ActionId = ActionId::new(Context::Pager, "transcript_latest");
pub const TRANSCRIPT_TOP_ALT: ActionId = ActionId::new(Context::Pager, "transcript_top_alt");
pub const TRANSCRIPT_LATEST_ALT: ActionId = ActionId::new(Context::Pager, "transcript_latest_alt");
/// The agents page's Tab and Shift+Tab step the filter (`center_filter_hint`).
pub const AGENTS_FILTER_NEXT: ActionId = ActionId::new(Context::Agents, "filter_next");
pub const AGENTS_FILTER_PREVIOUS: ActionId = ActionId::new(Context::Agents, "filter_previous");
/// The usage page's Shift+Tab goes to the previous tab.
pub const USAGE_PREVIOUS_TAB: ActionId = list("usage_previous_tab");
/// The agents page's confirm dialog (`ConfirmDialog`): Up and Down move the highlighted option.
pub const CONFIRM_PREVIOUS: ActionId = list("confirm_previous");
pub const CONFIRM_NEXT: ActionId = list("confirm_next");
/// The import's source prompt (`handle_key`): j/k and the arrows walk the sources, Enter takes
/// one, Esc, Ctrl+C and Ctrl+D leave.
pub const PICK_PREVIOUS: ActionId = list("pick_previous");
pub const PICK_NEXT: ActionId = list("pick_next");
pub const IMPORT_ACCEPT: ActionId = list("import_accept");
pub const IMPORT_CANCEL: ActionId = list("import_cancel");

/// The startup prompts, the sign-in wizards and the trust and update prompts (each the oracle's own
/// `handle_key`): the list's two moves (Up and `k`, Down and `j`), Ctrl+C and Ctrl+D leaving, the
/// number keys (in order, `1` to `9`), a `y` and an `n` beside `1` and `2`, a lone `q`.
pub const PICK_QUIT: ActionId = list("pick_quit");
pub const PICK_NUMBERS: ActionId = list("pick_numbers");
pub const PICK_YES: ActionId = list("pick_yes");
pub const PICK_NO: ActionId = list("pick_no");
pub const PICK_LEAVE: ActionId = list("pick_leave");
/// The unarchive confirmation (capitals too) and the open-source-model prompt (`1`/`l`, `2`/`o`,
/// and Left/Up/Ctrl+H/`k` against Right/Down/Ctrl+L/`j`; Ctrl+C alone cancels).
pub const UNARCHIVE_YES: ActionId = list("unarchive_yes");
pub const UNARCHIVE_NO: ActionId = list("unarchive_no");
pub const OSS_PREVIOUS: ActionId = list("oss_previous");
pub const OSS_NEXT: ActionId = list("oss_next");
pub const OSS_LMSTUDIO: ActionId = list("oss_lmstudio");
pub const OSS_OLLAMA: ActionId = list("oss_ollama");
pub const OSS_CANCEL: ActionId = list("oss_cancel");
/// The onboarding's plain keys (`onboarding/keys.rs`, `key_hint::plain`): Up/`k` and Down/`j` move,
/// Enter confirms, each with no modifier held (Shift+Enter is none of them).
pub const ONBOARD_PREVIOUS: ActionId = list("onboard_previous");
pub const ONBOARD_NEXT: ActionId = list("onboard_next");
pub const ONBOARD_CONFIRM: ActionId = list("onboard_confirm");
/// A sign-in wizard's form: Up and Shift+Tab, Down and Tab move between the fields; the gateway's
/// model list also walks with Ctrl+K/P and Ctrl+J/N, jumps to its ends, pages with PageUp/Ctrl+B
/// and PageDown/Ctrl+F, takes a row on Enter or `1`; Ctrl+Space completes a variable.
pub const FORM_PREVIOUS: ActionId = list("form_previous");
pub const FORM_NEXT: ActionId = list("form_next");
pub const GATEWAY_PREVIOUS: ActionId = list("gateway_previous");
pub const GATEWAY_NEXT: ActionId = list("gateway_next");
pub const GATEWAY_TOP: ActionId = list("gateway_top");
pub const GATEWAY_BOTTOM: ActionId = list("gateway_bottom");
pub const GATEWAY_PAGE_UP: ActionId = list("gateway_page_up");
pub const GATEWAY_PAGE_DOWN: ActionId = list("gateway_page_down");
pub const GATEWAY_CHOOSE: ActionId = list("gateway_choose");
pub const GATEWAY_COMPLETE: ActionId = list("gateway_complete");
/// `c` copies the sign-in link; Ctrl+. and Ctrl+Shift+. replay the welcome logo's animation.
pub const COPY_LINK: ActionId = list("copy_link");
pub const REPLAY_LOGO: ActionId = list("replay_logo");
/// The usage page's Tab goes to the next tab (Shift+Tab is `USAGE_PREVIOUS_TAB`).
pub const USAGE_NEXT_TAB: ActionId = list("usage_next_tab");
/// Left and Right turn the warnings dialog's page.
pub const TURN_PREVIOUS: ActionId = list("turn_previous");
pub const TURN_NEXT: ActionId = list("turn_next");
/// `k` keeps the warning on the page and turns to the next.
pub const TURN_KEEP: ActionId = list("turn_keep");
/// Tab and Shift+Tab step the cursor of a dialog's action rows on and back (the app link dialog).
pub const STEP_NEXT: ActionId = list("step_next");
pub const STEP_PREVIOUS: ActionId = list("step_previous");

/// The bytes a terminal without the kitty protocol sends for Ctrl+/ (`0x1f`, read as Ctrl+_, and
/// Ctrl+7 shares it): the side conversation's key where `toggle_side_conversation` is Ctrl+/.
pub const SIDE_CONVERSATION_BYTES: ActionId =
    ActionId::new(Context::Global, "side_conversation_bytes");

/// A fixed action and its keys; the first is the one `press_of` builds.
#[derive(Clone, Copy, Debug)]
pub struct Fixed {
    pub id: ActionId,
    pub bindings: &'static [FixedBinding],
}

const CTRL: Mods = Mods::CTRL;
const ALT: Mods = Mods::ALT;
const ALT_SHIFT: Mods = Mods::ALT.with(Mods::SHIFT);

pub const TABLE: &[Fixed] = &[
    Fixed {
        id: ENTER,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: TAB,
        bindings: &[any(Key::Tab)],
    },
    Fixed {
        id: DISMISS,
        bindings: &[any(Key::Esc)],
    },
    Fixed {
        id: HISTORY_PREVIOUS,
        bindings: &[any(Key::Up)],
    },
    Fixed {
        id: HISTORY_NEXT,
        bindings: &[any(Key::Down)],
    },
    Fixed {
        id: POPUP_PREVIOUS,
        bindings: &[any(Key::Up), exact(Key::Char('p'), CTRL)],
    },
    Fixed {
        id: POPUP_NEXT,
        bindings: &[any(Key::Down), exact(Key::Char('n'), CTRL)],
    },
    Fixed {
        id: POPUP_ACCEPT,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: POPUP_COMPLETE,
        bindings: &[any(Key::Tab)],
    },
    Fixed {
        id: POPUP_TAB_PREVIOUS,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: POPUP_TAB_NEXT,
        bindings: &[any(Key::Right)],
    },
    Fixed {
        id: PASTE_IMAGE,
        bindings: &[
            has(Key::Char('v'), CTRL),
            has(Key::Char('v'), ALT),
            has(Key::Char('V'), CTRL),
            has(Key::Char('V'), ALT),
        ],
    },
    Fixed {
        id: CLEAR_FOR_CTRL_C,
        bindings: &[exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: SHELL_PREFIX,
        bindings: &[exact(Key::Char('!'), Mods::NONE)],
    },
    Fixed {
        id: SLASH_COMPLETE,
        bindings: &[exact(Key::Char('/'), Mods::NONE)],
    },
    Fixed {
        id: SEARCH_ACCEPT,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: SEARCH_CANCEL,
        bindings: &[any(Key::Esc), exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: SEARCH_CANCEL_BYTE,
        bindings: &[exact(Key::Char('\u{3}'), Mods::NONE)],
    },
    Fixed {
        id: SEARCH_ERASE,
        bindings: &[any(Key::Backspace), exact(Key::Char('h'), CTRL)],
    },
    Fixed {
        id: SEARCH_CLEAR,
        bindings: &[exact(Key::Char('u'), CTRL)],
    },
    Fixed {
        id: DEBUG_KEY,
        bindings: &[any(Key::F(12))],
    },
    Fixed {
        id: TRANSCRIPT_PAGE_UP,
        bindings: &[exact(Key::PageUp, Mods::NONE)],
    },
    Fixed {
        id: TRANSCRIPT_PAGE_DOWN,
        bindings: &[exact(Key::PageDown, Mods::NONE)],
    },
    Fixed {
        id: CYCLE_COLLABORATION_MODE,
        bindings: &[exact(Key::Tab, Mods::SHIFT)],
    },
    Fixed {
        id: OPEN_AGENTS_FROM_LEFT,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: BROWSE_OLDER,
        bindings: &[exact(Key::Left, Mods::NONE)],
    },
    Fixed {
        id: BROWSE_NEWER,
        bindings: &[exact(Key::Right, Mods::NONE)],
    },
    Fixed {
        id: BROWSE_OLDER_VIM,
        bindings: &[exact(Key::Char('h'), Mods::NONE)],
    },
    Fixed {
        id: BROWSE_NEWER_VIM,
        bindings: &[exact(Key::Char('l'), Mods::NONE)],
    },
    Fixed {
        id: ESCAPE,
        bindings: &[exact(Key::Esc, Mods::NONE)],
    },
    Fixed {
        id: INTERRUPT_OR_QUIT,
        bindings: &[exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: QUIT_ON_EMPTY,
        bindings: &[exact(Key::Char('d'), CTRL)],
    },
    Fixed {
        id: OPEN_SHORTCUTS,
        bindings: &[any(Key::Char('?'))],
    },
    Fixed {
        id: RETRY_WRITER,
        bindings: &[exact(Key::Char('r'), Mods::NONE)],
    },
    Fixed {
        id: FORK_WRITER,
        bindings: &[exact(Key::Char('f'), Mods::NONE)],
    },
    Fixed {
        id: QUIT_READ_ONLY,
        bindings: &[exact(Key::Char('q'), Mods::NONE)],
    },
    Fixed {
        id: PROVIDER_NEW,
        bindings: &[exact(Key::Char('n'), Mods::NONE)],
    },
    Fixed {
        id: PROVIDER_CONTINUE,
        bindings: &[exact(Key::Char('c'), Mods::NONE)],
    },
    Fixed {
        id: SIDE_CONVERSATION_BYTES,
        bindings: &[exact(Key::Char('_'), CTRL), exact(Key::Char('7'), CTRL)],
    },
    Fixed {
        id: SUBAGENT_PREVIOUS,
        bindings: &[exact(Key::Left, ALT)],
    },
    Fixed {
        id: SUBAGENT_NEXT,
        bindings: &[exact(Key::Right, ALT)],
    },
    Fixed {
        id: SELECTION_START,
        bindings: &[exact(Key::Char(' '), CTRL)],
    },
    Fixed {
        id: TRANSCRIPT_TOP,
        bindings: &[exact(Key::Home, CTRL)],
    },
    Fixed {
        id: TRANSCRIPT_LATEST,
        bindings: &[exact(Key::End, CTRL)],
    },
    Fixed {
        id: TRANSCRIPT_TOP_ALT,
        bindings: &[exact(Key::Char(','), ALT_SHIFT)],
    },
    Fixed {
        id: TRANSCRIPT_LATEST_ALT,
        bindings: &[exact(Key::Char('.'), ALT_SHIFT)],
    },
    Fixed {
        id: AGENTS_FILTER_NEXT,
        bindings: &[exact(Key::Tab, Mods::NONE)],
    },
    Fixed {
        id: AGENTS_FILTER_PREVIOUS,
        bindings: &[exact(Key::BackTab, Mods::NONE)],
    },
    Fixed {
        id: USAGE_PREVIOUS_TAB,
        bindings: &[exact(Key::Tab, Mods::SHIFT)],
    },
    Fixed {
        id: CONFIRM_PREVIOUS,
        bindings: &[any(Key::Up)],
    },
    Fixed {
        id: CONFIRM_NEXT,
        bindings: &[any(Key::Down)],
    },
    Fixed {
        id: PICK_PREVIOUS,
        bindings: &[any(Key::Up), exact(Key::Char('k'), Mods::NONE)],
    },
    Fixed {
        id: PICK_NEXT,
        bindings: &[any(Key::Down), exact(Key::Char('j'), Mods::NONE)],
    },
    Fixed {
        id: IMPORT_ACCEPT,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: IMPORT_CANCEL,
        bindings: &[
            any(Key::Esc),
            exact(Key::Char('c'), CTRL),
            exact(Key::Char('d'), CTRL),
        ],
    },
    Fixed {
        id: PICK_QUIT,
        bindings: &[exact(Key::Char('c'), CTRL), exact(Key::Char('d'), CTRL)],
    },
    Fixed {
        id: PICK_NUMBERS,
        bindings: &[
            exact(Key::Char('1'), Mods::NONE),
            exact(Key::Char('2'), Mods::NONE),
            exact(Key::Char('3'), Mods::NONE),
            exact(Key::Char('4'), Mods::NONE),
            exact(Key::Char('5'), Mods::NONE),
            exact(Key::Char('6'), Mods::NONE),
            exact(Key::Char('7'), Mods::NONE),
            exact(Key::Char('8'), Mods::NONE),
            exact(Key::Char('9'), Mods::NONE),
        ],
    },
    Fixed {
        id: PICK_YES,
        bindings: &[
            exact(Key::Char('1'), Mods::NONE),
            exact(Key::Char('y'), Mods::NONE),
        ],
    },
    Fixed {
        id: PICK_NO,
        bindings: &[
            exact(Key::Char('2'), Mods::NONE),
            exact(Key::Char('n'), Mods::NONE),
        ],
    },
    Fixed {
        id: PICK_LEAVE,
        bindings: &[exact(Key::Char('q'), Mods::NONE)],
    },
    Fixed {
        id: UNARCHIVE_YES,
        bindings: &[
            exact(Key::Char('1'), Mods::NONE),
            exact(Key::Char('y'), Mods::NONE),
            exact(Key::Char('Y'), Mods::NONE),
        ],
    },
    Fixed {
        id: UNARCHIVE_NO,
        bindings: &[
            exact(Key::Char('2'), Mods::NONE),
            exact(Key::Char('n'), Mods::NONE),
            exact(Key::Char('N'), Mods::NONE),
        ],
    },
    Fixed {
        id: OSS_PREVIOUS,
        bindings: &[
            any(Key::Left),
            any(Key::Up),
            exact(Key::Backspace, CTRL),
            exact(Key::Char('k'), Mods::NONE),
        ],
    },
    Fixed {
        id: OSS_NEXT,
        bindings: &[
            any(Key::Right),
            any(Key::Down),
            exact(Key::Char('l'), CTRL),
            exact(Key::Char('j'), Mods::NONE),
        ],
    },
    Fixed {
        id: OSS_LMSTUDIO,
        bindings: &[
            exact(Key::Char('1'), Mods::NONE),
            exact(Key::Char('l'), Mods::NONE),
            exact(Key::Char('L'), Mods::NONE),
        ],
    },
    Fixed {
        id: OSS_OLLAMA,
        bindings: &[
            exact(Key::Char('2'), Mods::NONE),
            exact(Key::Char('o'), Mods::NONE),
            exact(Key::Char('O'), Mods::NONE),
        ],
    },
    Fixed {
        id: OSS_CANCEL,
        bindings: &[exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: ONBOARD_PREVIOUS,
        bindings: &[
            exact(Key::Up, Mods::NONE),
            exact(Key::Char('k'), Mods::NONE),
        ],
    },
    Fixed {
        id: ONBOARD_NEXT,
        bindings: &[
            exact(Key::Down, Mods::NONE),
            exact(Key::Char('j'), Mods::NONE),
        ],
    },
    Fixed {
        id: ONBOARD_CONFIRM,
        bindings: &[exact(Key::Enter, Mods::NONE)],
    },
    Fixed {
        id: FORM_PREVIOUS,
        bindings: &[exact(Key::Up, Mods::NONE), exact(Key::Tab, Mods::SHIFT)],
    },
    Fixed {
        id: FORM_NEXT,
        bindings: &[exact(Key::Down, Mods::NONE), exact(Key::Tab, Mods::NONE)],
    },
    Fixed {
        id: GATEWAY_PREVIOUS,
        bindings: &[
            exact(Key::Up, Mods::NONE),
            exact(Key::Char('k'), Mods::NONE),
            exact(Key::Char('k'), CTRL),
            exact(Key::Char('p'), CTRL),
        ],
    },
    Fixed {
        id: GATEWAY_NEXT,
        bindings: &[
            exact(Key::Down, Mods::NONE),
            exact(Key::Char('j'), Mods::NONE),
            exact(Key::Char('j'), CTRL),
            exact(Key::Char('n'), CTRL),
        ],
    },
    Fixed {
        id: GATEWAY_TOP,
        bindings: &[exact(Key::Home, Mods::NONE)],
    },
    Fixed {
        id: GATEWAY_BOTTOM,
        bindings: &[exact(Key::End, Mods::NONE)],
    },
    Fixed {
        id: GATEWAY_PAGE_UP,
        bindings: &[exact(Key::PageUp, Mods::NONE), exact(Key::Char('b'), CTRL)],
    },
    Fixed {
        id: GATEWAY_PAGE_DOWN,
        bindings: &[
            exact(Key::PageDown, Mods::NONE),
            exact(Key::Char('f'), CTRL),
        ],
    },
    Fixed {
        id: GATEWAY_CHOOSE,
        bindings: &[
            exact(Key::Enter, Mods::NONE),
            exact(Key::Char('1'), Mods::NONE),
        ],
    },
    Fixed {
        id: GATEWAY_COMPLETE,
        bindings: &[exact(Key::Char(' '), CTRL)],
    },
    Fixed {
        id: COPY_LINK,
        bindings: &[exact(Key::Char('c'), Mods::NONE)],
    },
    Fixed {
        id: REPLAY_LOGO,
        bindings: &[
            exact(Key::Char('.'), CTRL),
            exact(Key::Char('.'), CTRL.with(Mods::SHIFT)),
        ],
    },
    Fixed {
        id: USAGE_NEXT_TAB,
        bindings: &[exact(Key::Tab, Mods::NONE)],
    },
    Fixed {
        id: STEP_NEXT,
        bindings: &[exact(Key::Tab, Mods::NONE)],
    },
    Fixed {
        id: STEP_PREVIOUS,
        bindings: &[exact(Key::Tab, Mods::SHIFT)],
    },
    Fixed {
        id: TURN_KEEP,
        bindings: &[exact(Key::Char('k'), Mods::NONE)],
    },
    Fixed {
        id: TURN_PREVIOUS,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: TURN_NEXT,
        bindings: &[any(Key::Right)],
    },
    Fixed {
        id: FIELD_LEFT,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: FIELD_RIGHT,
        bindings: &[any(Key::Right)],
    },
    Fixed {
        id: FIELD_UP,
        bindings: &[any(Key::Up)],
    },
    Fixed {
        id: FIELD_DOWN,
        bindings: &[any(Key::Down)],
    },
    Fixed {
        id: FIELD_WORD_LEFT,
        bindings: &[exact(Key::Left, ALT)],
    },
    Fixed {
        id: FIELD_WORD_RIGHT,
        bindings: &[exact(Key::Right, ALT)],
    },
    Fixed {
        id: FIELD_LINE_START,
        bindings: &[any(Key::Home)],
    },
    Fixed {
        id: FIELD_LINE_END,
        bindings: &[any(Key::End)],
    },
    Fixed {
        id: FIELD_DELETE_BACKWARD,
        bindings: &[any(Key::Backspace)],
    },
    Fixed {
        id: FIELD_DELETE_FORWARD,
        bindings: &[any(Key::Delete)],
    },
    Fixed {
        id: FIELD_DELETE_BACKWARD_WORD,
        bindings: &[
            exact(Key::Char('w'), CTRL),
            exact(Key::Backspace, ALT),
            exact(Key::Backspace, CTRL),
        ],
    },
    Fixed {
        id: FIELD_DELETE_FORWARD_WORD,
        bindings: &[
            exact(Key::Char('d'), ALT),
            exact(Key::Delete, ALT),
            exact(Key::Delete, CTRL),
        ],
    },
    Fixed {
        id: FIELD_KILL_LINE_START,
        bindings: &[exact(Key::Char('u'), CTRL)],
    },
    Fixed {
        id: FIELD_KILL_LINE_END,
        bindings: &[exact(Key::Char('k'), CTRL)],
    },
    Fixed {
        id: FIELD_YANK,
        bindings: &[exact(Key::Char('y'), CTRL)],
    },
    Fixed {
        id: PLUGINS_TOGGLE,
        bindings: &[exact(Key::Tab, Mods::NONE)],
    },
    Fixed {
        id: PLUGINS_REMOVE,
        bindings: &[exact(Key::Char('r'), CTRL)],
    },
    Fixed {
        id: PLUGINS_UPGRADE,
        bindings: &[exact(Key::Char('u'), CTRL)],
    },
    Fixed {
        id: FEEDBACK_SCROLL_UP,
        bindings: &[exact(Key::PageUp, Mods::NONE)],
    },
    Fixed {
        id: FEEDBACK_SCROLL_DOWN,
        bindings: &[exact(Key::PageDown, Mods::NONE)],
    },
    Fixed {
        id: FEEDBACK_SAVE,
        bindings: &[exact(Key::Char('s'), CTRL)],
    },
];

/// The keys of a fixed action; none for an action of the catalogue.
pub fn bindings_of(id: ActionId) -> Option<&'static [FixedBinding]> {
    TABLE
        .iter()
        .find(|fixed| fixed.id == id)
        .map(|fixed| fixed.bindings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::catalogue::ROWS;

    #[test]
    fn a_fixed_action_is_no_row_of_the_catalogue_and_named_once() {
        for (at, fixed) in TABLE.iter().enumerate() {
            assert!(
                ROWS.iter().all(|row| row.id != fixed.id),
                "{} is a catalogue action",
                fixed.id.config_path()
            );
            assert!(
                TABLE[..at].iter().all(|other| other.id != fixed.id),
                "{} twice",
                fixed.id.config_path()
            );
            assert!(!fixed.bindings.is_empty());
        }
    }

    #[test]
    fn the_rules_read_the_modifiers() {
        assert!(ModsRule::Any.accepts(Mods::CTRL));
        assert!(ModsRule::Exact(Mods::CTRL).accepts(Mods::CTRL));
        assert!(!ModsRule::Exact(Mods::CTRL).accepts(Mods::CTRL.with(Mods::SHIFT)));
        assert!(ModsRule::Has(Mods::ALT).accepts(Mods::ALT.with(Mods::SHIFT)));
        assert!(!ModsRule::Has(Mods::ALT).accepts(Mods::NONE));
    }
}
