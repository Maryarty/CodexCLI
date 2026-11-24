"""Rules that give every pending oracle test one package: first match on `shard::test reason` wins.

A test whose reason names a missing kit seed / entry / frame names goes to `kit-seeds` first (the
kit change has to land before the feature can turn the test green); everything else goes by the
feature the test name or reason names, then by a shard fallback.
"""
import re

KIT = (r"no keymap seed|keymap seed|no seed|not seedable|seedable|kit cannot|names the frames|Screen::feed|"
       r"no debug entry|make_elicitation|`\.keymap`|no entry (?:`|with|for)|seeded sheet entry|no fixture")

RULES = [
    ("kit-seeds", KIT, "reason"),
    ("ui-safety-compaction", r"safety_buffer|safety_recourse|misalign|compaction|compact_|guardian|provider_error|luna"),
    ("ui-usage-limits", r"^usage |^status |reset_|billing|usage_command|usage_menu|startup_check|rate_limit|limit_warning|credit|thread_usage|server_row|usage_notice|status_history|account_usage"),
    ("ui-openers-side", r"\bside_|slash_side|slash_btw|\bbtw\b|side_context|subagent|daemon|presets|quick_dial|app_link|provider_switch|agent_picker"),
    ("ui-worktree-sandbox", r"forked_thread|worktree|checkout|windows_sandbox|sandbox|security_setup|full_access"),
    ("ui-feedback-cmds", r"feedback|upload_consent|diagnostic|recap|logout|ide_context|\bide_|daybreak|debug_config|debug_m"),
    ("ui-config-panel", r"config_panel|tui_mode|tui_picker"),
    ("ui-keymap-editor", r"keymap_setup|keymap_editor|keymap_capture|keymap_debug|keymap_picker|leader|chord"),
    ("ui-settings-feeds", r"hooks_|hook_popup|hooks_browser|experimental|skills_toggle|manage_skills|current_settings|memories_settings"),
    ("footer-status-sweep", r"^pane_footer|^chat_status|statusline|status_line|title_setup|terminal_title|window_title|footer|status_indicator|working_status|shortcut_hud|effort_ignition|sparkle"),
    ("ui-pager-find", r"^pager|diff|pager_|\bfind|search_footer|offline_find|highlight_match|n_and_shift"),
    ("ui-select-copy", r"select_mode|copy_mode|copy_picker|slash_copy|clipboard|selection|double_click|triple|markdown_copy|copy_payload|^transcript_view_4"),
    ("ui-transcript-chrome", r"^transcript_view|prompt_header|turn_details|turn_tip|disclosure|follow|history_page|mouse_scroll|focus_activity"),
    ("ui-exec-mcp", r"exploration|unified_exec|background_terminal|\bps_|slash_ps|processes|mcp_inventory|inventory|slash_mcp|waited_for"),
    ("ui-approvals-rest", r"approval|elicitation|request_user_input|question|decision|write_stdin|network_host|async_"),
    ("ui-composer-states", r"queue|queued|external_editor|ctrl_g|composer_state|read_only|right_click|mouse|drain"),
    ("voice", r"voice|realtime|split_flap|session_lock"),
    ("sidebar", r"sidebar"),
    ("inline-mode", r"inline|scrollback|reflow|committed|insert_history"),
    ("ui-tree-viz", r"ui_tree|visualization"),
    ("pets-ambient", r"\bpet|pet_"),
    ("startup-rest", r"^onboarding|^signin|^startup_prompts|^import|startup_|trust|update_prompt|oss_|unarchive|cwd_prompt|migration"),
    ("resume-rest", r"^resume|resume_picker|thread_colors"),
    ("agents-center", r"agents?_(?:overview|center|focus|mouse)|agent_status|^agents"),
    ("markdown-gaps", r"^markdown|math|mermaid|soft_break|nested_list|lists_preserve|heading|table_"),
    ("cells-sweep", r"^cells|^renderer"),
    ("dialogs-sweep", r"^small_dialogs|shortcut|warnings|^picker|^chat_popups|popup|model_|permissions_"),
    ("composer-sweep", r"^textarea|^composer|paste|vim|history_search|mention|startup_draft|steer|attachment|image"),
    ("ui-approvals-rest", r"^chat_approvals|^approvals"),
    ("live-turn-sweep", r"^chat_live|^chat_commands|^chat_status|backtrack|banner|plan_|review"),
]


SHARD_FALLBACK = {"usage": "ui-usage-limits", "status": "ui-usage-limits", "pager": "ui-pager-find",
                  "picker": "dialogs-sweep", "renderer_internals": "cells-sweep", "signin": "startup-rest",
                  "onboarding": "startup-rest", "startup_prompts": "startup-rest"}


def classify(name, reason):
    """-> package id.  Pass 1: kit reason; pass 2: the test name; pass 3: the reason; pass 4: the shard."""
    if re.search(RULES[0][1], reason):
        return RULES[0][0]
    key = name.replace("::", " ").lower()
    for pkg, rx, *_ in RULES[1:]:
        if re.search(rx, key):
            return pkg
    for pkg, rx, *_ in RULES[1:]:
        if re.search(rx, reason.lower()):
            return pkg
    shard = name.split("::")[0]
    if shard in SHARD_FALLBACK:
        return SHARD_FALLBACK[shard]
    base = re.sub(r"_\d+$", "", shard)
    return {"chat_live": "live-turn-sweep", "chat_commands": "live-turn-sweep", "chat_status": "footer-status-sweep",
            "chat_approvals": "ui-approvals-rest", "approvals": "ui-approvals-rest", "cells": "cells-sweep",
            "chat_popups": "dialogs-sweep", "small_dialogs": "dialogs-sweep", "transcript_view": "ui-transcript-chrome",
            "composer": "composer-sweep", "composer_popups": "composer-sweep", "markdown_streaming": "markdown-gaps",
            "pane_footer": "footer-status-sweep", "resume": "resume-rest", "textarea": "composer-sweep"}.get(base)
