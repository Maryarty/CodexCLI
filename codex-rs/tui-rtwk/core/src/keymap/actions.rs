//! The actions as typed handles: `actions::LIST_ACCEPT` is
//! `tui.keymap.list.accept`. A view asks the `Keymap` for one of these and
//! never spells an action by string; a test keeps the table and the
//! catalogue the same set.

use super::catalogue::{ActionId, Context};

pub const GLOBAL_CHANGE_HOST: ActionId = ActionId::new(Context::Global, "change_host");
pub const GLOBAL_OPEN_AGENTS: ActionId = ActionId::new(Context::Global, "open_agents");
pub const GLOBAL_OPEN_TRANSCRIPT: ActionId = ActionId::new(Context::Global, "open_transcript");
pub const GLOBAL_FIND_TRANSCRIPT: ActionId = ActionId::new(Context::Global, "find_transcript");
pub const GLOBAL_SELECT_TRANSCRIPT: ActionId = ActionId::new(Context::Global, "select_transcript");
pub const GLOBAL_FOCUS_ACTIVITY: ActionId = ActionId::new(Context::Global, "focus_activity");
pub const GLOBAL_FOCUS_MAIN_COMPOSER: ActionId =
    ActionId::new(Context::Global, "focus_main_composer");
pub const GLOBAL_FOCUS_SIDEBAR_CONTENT: ActionId =
    ActionId::new(Context::Global, "focus_sidebar_content");
pub const GLOBAL_FOCUS_SIDEBAR_TABS: ActionId =
    ActionId::new(Context::Global, "focus_sidebar_tabs");
pub const GLOBAL_HIDE_PANE: ActionId = ActionId::new(Context::Global, "hide_pane");
pub const GLOBAL_CLOSE_SIDEBAR_TAB: ActionId = ActionId::new(Context::Global, "close_sidebar_tab");
pub const GLOBAL_SIDEBAR_UP: ActionId = ActionId::new(Context::Global, "sidebar_up");
pub const GLOBAL_SIDEBAR_DOWN: ActionId = ActionId::new(Context::Global, "sidebar_down");
pub const GLOBAL_FOCUS_PANE_LEFT: ActionId = ActionId::new(Context::Global, "focus_pane_left");
pub const GLOBAL_FOCUS_PANE_RIGHT: ActionId = ActionId::new(Context::Global, "focus_pane_right");
pub const GLOBAL_MINIMIZE_PANE_WIDTH: ActionId =
    ActionId::new(Context::Global, "minimize_pane_width");
pub const GLOBAL_MAXIMIZE_PANE_WIDTH: ActionId =
    ActionId::new(Context::Global, "maximize_pane_width");
pub const GLOBAL_EQUALIZE_PANE_WIDTHS: ActionId =
    ActionId::new(Context::Global, "equalize_pane_widths");
pub const GLOBAL_RESIZE_PANES: ActionId = ActionId::new(Context::Global, "resize_panes");
pub const GLOBAL_MOVE_DIVIDER_LEFT: ActionId = ActionId::new(Context::Global, "move_divider_left");
pub const GLOBAL_MOVE_DIVIDER_RIGHT: ActionId =
    ActionId::new(Context::Global, "move_divider_right");
pub const GLOBAL_TOGGLE_PANE_ZOOM: ActionId = ActionId::new(Context::Global, "toggle_pane_zoom");
pub const GLOBAL_OPEN_WARNINGS: ActionId = ActionId::new(Context::Global, "open_warnings");
pub const GLOBAL_OPEN_EXTERNAL_EDITOR: ActionId =
    ActionId::new(Context::Global, "open_external_editor");
pub const GLOBAL_COPY: ActionId = ActionId::new(Context::Global, "copy");
pub const GLOBAL_CLEAR_TERMINAL: ActionId = ActionId::new(Context::Global, "clear_terminal");
pub const GLOBAL_TOGGLE_VIM_MODE: ActionId = ActionId::new(Context::Global, "toggle_vim_mode");
pub const GLOBAL_TOGGLE_FAST_MODE: ActionId = ActionId::new(Context::Global, "toggle_fast_mode");
pub const GLOBAL_TOGGLE_RAW_OUTPUT: ActionId = ActionId::new(Context::Global, "toggle_raw_output");
pub const GLOBAL_TOGGLE_SIDE_CONVERSATION: ActionId =
    ActionId::new(Context::Global, "toggle_side_conversation");
pub const CHAT_OPEN_PRESETS: ActionId = ActionId::new(Context::Chat, "open_presets");
pub const CHAT_TOGGLE_VOICE: ActionId = ActionId::new(Context::Chat, "toggle_voice");
pub const CHAT_INTERRUPT_TURN: ActionId = ActionId::new(Context::Chat, "interrupt_turn");
pub const CHAT_DECREASE_REASONING_EFFORT: ActionId =
    ActionId::new(Context::Chat, "decrease_reasoning_effort");
pub const CHAT_INCREASE_REASONING_EFFORT: ActionId =
    ActionId::new(Context::Chat, "increase_reasoning_effort");
pub const CHAT_PREVIOUS_PERMISSION_MODE: ActionId =
    ActionId::new(Context::Chat, "previous_permission_mode");
pub const CHAT_NEXT_PERMISSION_MODE: ActionId =
    ActionId::new(Context::Chat, "next_permission_mode");
pub const CHAT_EDIT_QUEUED_MESSAGE: ActionId = ActionId::new(Context::Chat, "edit_queued_message");
pub const CHAT_PROMPT_STACK_BACK: ActionId = ActionId::new(Context::Chat, "prompt_stack_back");
pub const CHAT_SKIP_QUESTION: ActionId = ActionId::new(Context::Chat, "skip_question");
pub const VOICE_TOGGLE_VOICE_MUTE: ActionId = ActionId::new(Context::Voice, "toggle_voice_mute");
pub const COMPOSER_SUBMIT: ActionId = ActionId::new(Context::Composer, "submit");
pub const COMPOSER_QUEUE: ActionId = ActionId::new(Context::Composer, "queue");
pub const COMPOSER_TOGGLE_SHORTCUTS: ActionId =
    ActionId::new(Context::Composer, "toggle_shortcuts");
pub const COMPOSER_HISTORY_SEARCH_PREVIOUS: ActionId =
    ActionId::new(Context::Composer, "history_search_previous");
pub const COMPOSER_HISTORY_SEARCH_NEXT: ActionId =
    ActionId::new(Context::Composer, "history_search_next");
pub const EDITOR_INSERT_NEWLINE: ActionId = ActionId::new(Context::Editor, "insert_newline");
pub const EDITOR_MOVE_LEFT: ActionId = ActionId::new(Context::Editor, "move_left");
pub const EDITOR_MOVE_RIGHT: ActionId = ActionId::new(Context::Editor, "move_right");
pub const EDITOR_MOVE_UP: ActionId = ActionId::new(Context::Editor, "move_up");
pub const EDITOR_MOVE_DOWN: ActionId = ActionId::new(Context::Editor, "move_down");
pub const EDITOR_MOVE_WORD_LEFT: ActionId = ActionId::new(Context::Editor, "move_word_left");
pub const EDITOR_MOVE_WORD_RIGHT: ActionId = ActionId::new(Context::Editor, "move_word_right");
pub const EDITOR_MOVE_LINE_START: ActionId = ActionId::new(Context::Editor, "move_line_start");
pub const EDITOR_MOVE_LINE_END: ActionId = ActionId::new(Context::Editor, "move_line_end");
pub const EDITOR_DELETE_BACKWARD: ActionId = ActionId::new(Context::Editor, "delete_backward");
pub const EDITOR_DELETE_FORWARD: ActionId = ActionId::new(Context::Editor, "delete_forward");
pub const EDITOR_DELETE_BACKWARD_WORD: ActionId =
    ActionId::new(Context::Editor, "delete_backward_word");
pub const EDITOR_DELETE_FORWARD_WORD: ActionId =
    ActionId::new(Context::Editor, "delete_forward_word");
pub const EDITOR_KILL_LINE_START: ActionId = ActionId::new(Context::Editor, "kill_line_start");
pub const EDITOR_KILL_WHOLE_LINE: ActionId = ActionId::new(Context::Editor, "kill_whole_line");
pub const EDITOR_KILL_LINE_END: ActionId = ActionId::new(Context::Editor, "kill_line_end");
pub const EDITOR_YANK: ActionId = ActionId::new(Context::Editor, "yank");
pub const VIM_NORMAL_ENTER_INSERT: ActionId = ActionId::new(Context::VimNormal, "enter_insert");
pub const VIM_NORMAL_APPEND_AFTER_CURSOR: ActionId =
    ActionId::new(Context::VimNormal, "append_after_cursor");
pub const VIM_NORMAL_APPEND_LINE_END: ActionId =
    ActionId::new(Context::VimNormal, "append_line_end");
pub const VIM_NORMAL_INSERT_LINE_START: ActionId =
    ActionId::new(Context::VimNormal, "insert_line_start");
pub const VIM_NORMAL_OPEN_LINE_BELOW: ActionId =
    ActionId::new(Context::VimNormal, "open_line_below");
pub const VIM_NORMAL_OPEN_LINE_ABOVE: ActionId =
    ActionId::new(Context::VimNormal, "open_line_above");
pub const VIM_NORMAL_ENTER_REPLACE_MODE: ActionId =
    ActionId::new(Context::VimNormal, "enter_replace_mode");
pub const VIM_NORMAL_MOVE_LEFT: ActionId = ActionId::new(Context::VimNormal, "move_left");
pub const VIM_NORMAL_MOVE_RIGHT: ActionId = ActionId::new(Context::VimNormal, "move_right");
pub const VIM_NORMAL_MOVE_UP: ActionId = ActionId::new(Context::VimNormal, "move_up");
pub const VIM_NORMAL_MOVE_DOWN: ActionId = ActionId::new(Context::VimNormal, "move_down");
pub const VIM_NORMAL_MOVE_WORD_FORWARD: ActionId =
    ActionId::new(Context::VimNormal, "move_word_forward");
pub const VIM_NORMAL_MOVE_WORD_BACKWARD: ActionId =
    ActionId::new(Context::VimNormal, "move_word_backward");
pub const VIM_NORMAL_MOVE_WORD_END: ActionId = ActionId::new(Context::VimNormal, "move_word_end");
pub const VIM_NORMAL_MOVE_LINE_START: ActionId =
    ActionId::new(Context::VimNormal, "move_line_start");
pub const VIM_NORMAL_MOVE_LINE_END: ActionId = ActionId::new(Context::VimNormal, "move_line_end");
pub const VIM_NORMAL_FIND_FORWARD: ActionId = ActionId::new(Context::VimNormal, "find_forward");
pub const VIM_NORMAL_FIND_BACKWARD: ActionId = ActionId::new(Context::VimNormal, "find_backward");
pub const VIM_NORMAL_TILL_FORWARD: ActionId = ActionId::new(Context::VimNormal, "till_forward");
pub const VIM_NORMAL_TILL_BACKWARD: ActionId = ActionId::new(Context::VimNormal, "till_backward");
pub const VIM_NORMAL_JUMP_TOP: ActionId = ActionId::new(Context::VimNormal, "jump_top");
pub const VIM_NORMAL_JUMP_BOTTOM: ActionId = ActionId::new(Context::VimNormal, "jump_bottom");
pub const VIM_NORMAL_DELETE_CHAR: ActionId = ActionId::new(Context::VimNormal, "delete_char");
pub const VIM_NORMAL_REPLACE_CHAR: ActionId = ActionId::new(Context::VimNormal, "replace_char");
pub const VIM_NORMAL_REPEAT_LAST_CHANGE: ActionId =
    ActionId::new(Context::VimNormal, "repeat_last_change");
pub const VIM_NORMAL_SUBSTITUTE_CHAR: ActionId =
    ActionId::new(Context::VimNormal, "substitute_char");
pub const VIM_NORMAL_DELETE_TO_LINE_END: ActionId =
    ActionId::new(Context::VimNormal, "delete_to_line_end");
pub const VIM_NORMAL_CHANGE_TO_LINE_END: ActionId =
    ActionId::new(Context::VimNormal, "change_to_line_end");
pub const VIM_NORMAL_YANK_LINE: ActionId = ActionId::new(Context::VimNormal, "yank_line");
pub const VIM_NORMAL_PASTE_AFTER: ActionId = ActionId::new(Context::VimNormal, "paste_after");
pub const VIM_NORMAL_START_DELETE_OPERATOR: ActionId =
    ActionId::new(Context::VimNormal, "start_delete_operator");
pub const VIM_NORMAL_START_YANK_OPERATOR: ActionId =
    ActionId::new(Context::VimNormal, "start_yank_operator");
pub const VIM_NORMAL_START_CHANGE_OPERATOR: ActionId =
    ActionId::new(Context::VimNormal, "start_change_operator");
pub const VIM_NORMAL_UNDO: ActionId = ActionId::new(Context::VimNormal, "undo");
pub const VIM_NORMAL_REDO: ActionId = ActionId::new(Context::VimNormal, "redo");
pub const VIM_NORMAL_CANCEL_OPERATOR: ActionId =
    ActionId::new(Context::VimNormal, "cancel_operator");
pub const VIM_SEARCH_FORWARD: ActionId = ActionId::new(Context::VimSearch, "forward");
pub const VIM_SEARCH_BACKWARD: ActionId = ActionId::new(Context::VimSearch, "backward");
pub const VIM_SEARCH_NEXT: ActionId = ActionId::new(Context::VimSearch, "next");
pub const VIM_SEARCH_PREVIOUS: ActionId = ActionId::new(Context::VimSearch, "previous");
pub const VIM_OPERATOR_DELETE_LINE: ActionId = ActionId::new(Context::VimOperator, "delete_line");
pub const VIM_OPERATOR_YANK_LINE: ActionId = ActionId::new(Context::VimOperator, "yank_line");
pub const VIM_OPERATOR_MOTION_LEFT: ActionId = ActionId::new(Context::VimOperator, "motion_left");
pub const VIM_OPERATOR_MOTION_RIGHT: ActionId = ActionId::new(Context::VimOperator, "motion_right");
pub const VIM_OPERATOR_MOTION_UP: ActionId = ActionId::new(Context::VimOperator, "motion_up");
pub const VIM_OPERATOR_MOTION_DOWN: ActionId = ActionId::new(Context::VimOperator, "motion_down");
pub const VIM_OPERATOR_MOTION_WORD_FORWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_word_forward");
pub const VIM_OPERATOR_MOTION_WORD_BACKWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_word_backward");
pub const VIM_OPERATOR_MOTION_WORD_END: ActionId =
    ActionId::new(Context::VimOperator, "motion_word_end");
pub const VIM_OPERATOR_MOTION_LINE_START: ActionId =
    ActionId::new(Context::VimOperator, "motion_line_start");
pub const VIM_OPERATOR_MOTION_LINE_END: ActionId =
    ActionId::new(Context::VimOperator, "motion_line_end");
pub const VIM_OPERATOR_MOTION_FIND_FORWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_find_forward");
pub const VIM_OPERATOR_MOTION_FIND_BACKWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_find_backward");
pub const VIM_OPERATOR_MOTION_TILL_FORWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_till_forward");
pub const VIM_OPERATOR_MOTION_TILL_BACKWARD: ActionId =
    ActionId::new(Context::VimOperator, "motion_till_backward");
pub const VIM_OPERATOR_MOTION_JUMP_TOP: ActionId =
    ActionId::new(Context::VimOperator, "motion_jump_top");
pub const VIM_OPERATOR_MOTION_JUMP_BOTTOM: ActionId =
    ActionId::new(Context::VimOperator, "motion_jump_bottom");
pub const VIM_OPERATOR_SELECT_INNER_TEXT_OBJECT: ActionId =
    ActionId::new(Context::VimOperator, "select_inner_text_object");
pub const VIM_OPERATOR_SELECT_AROUND_TEXT_OBJECT: ActionId =
    ActionId::new(Context::VimOperator, "select_around_text_object");
pub const VIM_OPERATOR_CANCEL: ActionId = ActionId::new(Context::VimOperator, "cancel");
pub const VIM_TEXT_OBJECT_WORD: ActionId = ActionId::new(Context::VimTextObject, "word");
pub const VIM_TEXT_OBJECT_BIG_WORD: ActionId = ActionId::new(Context::VimTextObject, "big_word");
pub const VIM_TEXT_OBJECT_PARENTHESES: ActionId =
    ActionId::new(Context::VimTextObject, "parentheses");
pub const VIM_TEXT_OBJECT_BRACKETS: ActionId = ActionId::new(Context::VimTextObject, "brackets");
pub const VIM_TEXT_OBJECT_BRACES: ActionId = ActionId::new(Context::VimTextObject, "braces");
pub const VIM_TEXT_OBJECT_DOUBLE_QUOTE: ActionId =
    ActionId::new(Context::VimTextObject, "double_quote");
pub const VIM_TEXT_OBJECT_SINGLE_QUOTE: ActionId =
    ActionId::new(Context::VimTextObject, "single_quote");
pub const VIM_TEXT_OBJECT_BACKTICK: ActionId = ActionId::new(Context::VimTextObject, "backtick");
pub const VIM_TEXT_OBJECT_CANCEL: ActionId = ActionId::new(Context::VimTextObject, "cancel");
pub const PAGER_SCROLL_UP: ActionId = ActionId::new(Context::Pager, "scroll_up");
pub const PAGER_SCROLL_DOWN: ActionId = ActionId::new(Context::Pager, "scroll_down");
pub const PAGER_PAGE_UP: ActionId = ActionId::new(Context::Pager, "page_up");
pub const PAGER_PAGE_DOWN: ActionId = ActionId::new(Context::Pager, "page_down");
pub const PAGER_HALF_PAGE_UP: ActionId = ActionId::new(Context::Pager, "half_page_up");
pub const PAGER_HALF_PAGE_DOWN: ActionId = ActionId::new(Context::Pager, "half_page_down");
pub const PAGER_JUMP_TOP: ActionId = ActionId::new(Context::Pager, "jump_top");
pub const PAGER_JUMP_BOTTOM: ActionId = ActionId::new(Context::Pager, "jump_bottom");
pub const PAGER_CLOSE: ActionId = ActionId::new(Context::Pager, "close");
pub const PAGER_CLOSE_TRANSCRIPT: ActionId = ActionId::new(Context::Pager, "close_transcript");
pub const PAGER_FIND: ActionId = ActionId::new(Context::Pager, "find");
pub const LIST_MOVE_UP: ActionId = ActionId::new(Context::List, "move_up");
pub const LIST_MOVE_DOWN: ActionId = ActionId::new(Context::List, "move_down");
pub const LIST_MOVE_LEFT: ActionId = ActionId::new(Context::List, "move_left");
pub const LIST_MOVE_RIGHT: ActionId = ActionId::new(Context::List, "move_right");
pub const LIST_PAGE_UP: ActionId = ActionId::new(Context::List, "page_up");
pub const LIST_PAGE_DOWN: ActionId = ActionId::new(Context::List, "page_down");
pub const LIST_JUMP_TOP: ActionId = ActionId::new(Context::List, "jump_top");
pub const LIST_JUMP_BOTTOM: ActionId = ActionId::new(Context::List, "jump_bottom");
pub const LIST_ACCEPT: ActionId = ActionId::new(Context::List, "accept");
pub const LIST_CANCEL: ActionId = ActionId::new(Context::List, "cancel");
pub const AGENTS_RESUME: ActionId = ActionId::new(Context::Agents, "resume");
pub const AGENTS_SEARCH: ActionId = ActionId::new(Context::Agents, "search");
pub const AGENTS_NEW_TASK: ActionId = ActionId::new(Context::Agents, "new_task");
pub const AGENTS_NEW_WORKTREE: ActionId = ActionId::new(Context::Agents, "new_worktree");
pub const AGENTS_FORK: ActionId = ActionId::new(Context::Agents, "fork");
pub const AGENTS_RENAME: ActionId = ActionId::new(Context::Agents, "rename");
pub const AGENTS_STOP: ActionId = ActionId::new(Context::Agents, "stop");
pub const AGENTS_ARCHIVE: ActionId = ActionId::new(Context::Agents, "archive");
pub const AGENTS_DELETE: ActionId = ActionId::new(Context::Agents, "delete");
pub const AGENTS_HIDE: ActionId = ActionId::new(Context::Agents, "hide");
pub const AGENTS_TOGGLE_PIN: ActionId = ActionId::new(Context::Agents, "toggle_pin");
pub const AGENTS_TOGGLE_GROUPING: ActionId = ActionId::new(Context::Agents, "toggle_grouping");
pub const APPROVAL_OPEN_FULLSCREEN: ActionId = ActionId::new(Context::Approval, "open_fullscreen");
pub const APPROVAL_OPEN_THREAD: ActionId = ActionId::new(Context::Approval, "open_thread");
pub const APPROVAL_APPROVE: ActionId = ActionId::new(Context::Approval, "approve");
pub const APPROVAL_APPROVE_FOR_SESSION: ActionId =
    ActionId::new(Context::Approval, "approve_for_session");
pub const APPROVAL_APPROVE_FOR_PREFIX: ActionId =
    ActionId::new(Context::Approval, "approve_for_prefix");
pub const APPROVAL_DENY: ActionId = ActionId::new(Context::Approval, "deny");
pub const APPROVAL_DECLINE: ActionId = ActionId::new(Context::Approval, "decline");
pub const APPROVAL_CANCEL: ActionId = ActionId::new(Context::Approval, "cancel");

/// Every handle above, for the test that compares them with the catalogue.
pub const ALL: &[ActionId] = &[
    GLOBAL_CHANGE_HOST,
    GLOBAL_OPEN_AGENTS,
    GLOBAL_OPEN_TRANSCRIPT,
    GLOBAL_FIND_TRANSCRIPT,
    GLOBAL_SELECT_TRANSCRIPT,
    GLOBAL_FOCUS_ACTIVITY,
    GLOBAL_FOCUS_MAIN_COMPOSER,
    GLOBAL_FOCUS_SIDEBAR_CONTENT,
    GLOBAL_FOCUS_SIDEBAR_TABS,
    GLOBAL_HIDE_PANE,
    GLOBAL_CLOSE_SIDEBAR_TAB,
    GLOBAL_SIDEBAR_UP,
    GLOBAL_SIDEBAR_DOWN,
    GLOBAL_FOCUS_PANE_LEFT,
    GLOBAL_FOCUS_PANE_RIGHT,
    GLOBAL_MINIMIZE_PANE_WIDTH,
    GLOBAL_MAXIMIZE_PANE_WIDTH,
    GLOBAL_EQUALIZE_PANE_WIDTHS,
    GLOBAL_RESIZE_PANES,
    GLOBAL_MOVE_DIVIDER_LEFT,
    GLOBAL_MOVE_DIVIDER_RIGHT,
    GLOBAL_TOGGLE_PANE_ZOOM,
    GLOBAL_OPEN_WARNINGS,
    GLOBAL_OPEN_EXTERNAL_EDITOR,
    GLOBAL_COPY,
    GLOBAL_CLEAR_TERMINAL,
    GLOBAL_TOGGLE_VIM_MODE,
    GLOBAL_TOGGLE_FAST_MODE,
    GLOBAL_TOGGLE_RAW_OUTPUT,
    GLOBAL_TOGGLE_SIDE_CONVERSATION,
    CHAT_OPEN_PRESETS,
    CHAT_TOGGLE_VOICE,
    CHAT_INTERRUPT_TURN,
    CHAT_DECREASE_REASONING_EFFORT,
    CHAT_INCREASE_REASONING_EFFORT,
    CHAT_PREVIOUS_PERMISSION_MODE,
    CHAT_NEXT_PERMISSION_MODE,
    CHAT_EDIT_QUEUED_MESSAGE,
    CHAT_PROMPT_STACK_BACK,
    CHAT_SKIP_QUESTION,
    VOICE_TOGGLE_VOICE_MUTE,
    COMPOSER_SUBMIT,
    COMPOSER_QUEUE,
    COMPOSER_TOGGLE_SHORTCUTS,
    COMPOSER_HISTORY_SEARCH_PREVIOUS,
    COMPOSER_HISTORY_SEARCH_NEXT,
    EDITOR_INSERT_NEWLINE,
    EDITOR_MOVE_LEFT,
    EDITOR_MOVE_RIGHT,
    EDITOR_MOVE_UP,
    EDITOR_MOVE_DOWN,
    EDITOR_MOVE_WORD_LEFT,
    EDITOR_MOVE_WORD_RIGHT,
    EDITOR_MOVE_LINE_START,
    EDITOR_MOVE_LINE_END,
    EDITOR_DELETE_BACKWARD,
    EDITOR_DELETE_FORWARD,
    EDITOR_DELETE_BACKWARD_WORD,
    EDITOR_DELETE_FORWARD_WORD,
    EDITOR_KILL_LINE_START,
    EDITOR_KILL_WHOLE_LINE,
    EDITOR_KILL_LINE_END,
    EDITOR_YANK,
    VIM_NORMAL_ENTER_INSERT,
    VIM_NORMAL_APPEND_AFTER_CURSOR,
    VIM_NORMAL_APPEND_LINE_END,
    VIM_NORMAL_INSERT_LINE_START,
    VIM_NORMAL_OPEN_LINE_BELOW,
    VIM_NORMAL_OPEN_LINE_ABOVE,
    VIM_NORMAL_ENTER_REPLACE_MODE,
    VIM_NORMAL_MOVE_LEFT,
    VIM_NORMAL_MOVE_RIGHT,
    VIM_NORMAL_MOVE_UP,
    VIM_NORMAL_MOVE_DOWN,
    VIM_NORMAL_MOVE_WORD_FORWARD,
    VIM_NORMAL_MOVE_WORD_BACKWARD,
    VIM_NORMAL_MOVE_WORD_END,
    VIM_NORMAL_MOVE_LINE_START,
    VIM_NORMAL_MOVE_LINE_END,
    VIM_NORMAL_FIND_FORWARD,
    VIM_NORMAL_FIND_BACKWARD,
    VIM_NORMAL_TILL_FORWARD,
    VIM_NORMAL_TILL_BACKWARD,
    VIM_NORMAL_JUMP_TOP,
    VIM_NORMAL_JUMP_BOTTOM,
    VIM_NORMAL_DELETE_CHAR,
    VIM_NORMAL_REPLACE_CHAR,
    VIM_NORMAL_REPEAT_LAST_CHANGE,
    VIM_NORMAL_SUBSTITUTE_CHAR,
    VIM_NORMAL_DELETE_TO_LINE_END,
    VIM_NORMAL_CHANGE_TO_LINE_END,
    VIM_NORMAL_YANK_LINE,
    VIM_NORMAL_PASTE_AFTER,
    VIM_NORMAL_START_DELETE_OPERATOR,
    VIM_NORMAL_START_YANK_OPERATOR,
    VIM_NORMAL_START_CHANGE_OPERATOR,
    VIM_NORMAL_UNDO,
    VIM_NORMAL_REDO,
    VIM_NORMAL_CANCEL_OPERATOR,
    VIM_SEARCH_FORWARD,
    VIM_SEARCH_BACKWARD,
    VIM_SEARCH_NEXT,
    VIM_SEARCH_PREVIOUS,
    VIM_OPERATOR_DELETE_LINE,
    VIM_OPERATOR_YANK_LINE,
    VIM_OPERATOR_MOTION_LEFT,
    VIM_OPERATOR_MOTION_RIGHT,
    VIM_OPERATOR_MOTION_UP,
    VIM_OPERATOR_MOTION_DOWN,
    VIM_OPERATOR_MOTION_WORD_FORWARD,
    VIM_OPERATOR_MOTION_WORD_BACKWARD,
    VIM_OPERATOR_MOTION_WORD_END,
    VIM_OPERATOR_MOTION_LINE_START,
    VIM_OPERATOR_MOTION_LINE_END,
    VIM_OPERATOR_MOTION_FIND_FORWARD,
    VIM_OPERATOR_MOTION_FIND_BACKWARD,
    VIM_OPERATOR_MOTION_TILL_FORWARD,
    VIM_OPERATOR_MOTION_TILL_BACKWARD,
    VIM_OPERATOR_MOTION_JUMP_TOP,
    VIM_OPERATOR_MOTION_JUMP_BOTTOM,
    VIM_OPERATOR_SELECT_INNER_TEXT_OBJECT,
    VIM_OPERATOR_SELECT_AROUND_TEXT_OBJECT,
    VIM_OPERATOR_CANCEL,
    VIM_TEXT_OBJECT_WORD,
    VIM_TEXT_OBJECT_BIG_WORD,
    VIM_TEXT_OBJECT_PARENTHESES,
    VIM_TEXT_OBJECT_BRACKETS,
    VIM_TEXT_OBJECT_BRACES,
    VIM_TEXT_OBJECT_DOUBLE_QUOTE,
    VIM_TEXT_OBJECT_SINGLE_QUOTE,
    VIM_TEXT_OBJECT_BACKTICK,
    VIM_TEXT_OBJECT_CANCEL,
    PAGER_SCROLL_UP,
    PAGER_SCROLL_DOWN,
    PAGER_PAGE_UP,
    PAGER_PAGE_DOWN,
    PAGER_HALF_PAGE_UP,
    PAGER_HALF_PAGE_DOWN,
    PAGER_JUMP_TOP,
    PAGER_JUMP_BOTTOM,
    PAGER_CLOSE,
    PAGER_CLOSE_TRANSCRIPT,
    PAGER_FIND,
    LIST_MOVE_UP,
    LIST_MOVE_DOWN,
    LIST_MOVE_LEFT,
    LIST_MOVE_RIGHT,
    LIST_PAGE_UP,
    LIST_PAGE_DOWN,
    LIST_JUMP_TOP,
    LIST_JUMP_BOTTOM,
    LIST_ACCEPT,
    LIST_CANCEL,
    AGENTS_RESUME,
    AGENTS_SEARCH,
    AGENTS_NEW_TASK,
    AGENTS_NEW_WORKTREE,
    AGENTS_FORK,
    AGENTS_RENAME,
    AGENTS_STOP,
    AGENTS_ARCHIVE,
    AGENTS_DELETE,
    AGENTS_HIDE,
    AGENTS_TOGGLE_PIN,
    AGENTS_TOGGLE_GROUPING,
    APPROVAL_OPEN_FULLSCREEN,
    APPROVAL_OPEN_THREAD,
    APPROVAL_APPROVE,
    APPROVAL_APPROVE_FOR_SESSION,
    APPROVAL_APPROVE_FOR_PREFIX,
    APPROVAL_DENY,
    APPROVAL_DECLINE,
    APPROVAL_CANCEL,
];
