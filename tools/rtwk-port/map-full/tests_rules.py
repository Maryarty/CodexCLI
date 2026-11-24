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
    ("w2-cells", r"safety_buffer|safety_recourse|misalign|compaction|compact_|guardian|provider_error|luna"),
    ("w2-status", r"^usage |^status |reset_|billing|usage_command|usage_menu|startup_check|rate_limit|limit_warning|credit|thread_usage|server_row|usage_notice|status_history|account_usage"),
    ("w2-commands", r"\bside_|slash_side|slash_btw|\bbtw\b|side_context|subagent|daemon|presets|quick_dial|app_link|provider_switch|agent_picker"),
    ("w2-commands", r"forked_thread|worktree|checkout|windows_sandbox|sandbox|security_setup|full_access"),
    ("w2-commands", r"feedback|upload_consent|diagnostic|recap|logout|ide_context|\bide_|daybreak|debug_config|debug_m"),
    ("w2-settings", r"config_panel|tui_mode|tui_picker"),
    ("w2-settings", r"keymap_setup|keymap_editor|keymap_capture|keymap_debug|keymap_picker|leader|chord"),
    ("w2-settings", r"hooks_|hook_popup|hooks_browser|experimental|skills_toggle|manage_skills|current_settings|memories_settings"),
    ("w2-status", r"^pane_footer|^chat_status|statusline|status_line|title_setup|terminal_title|window_title|footer|status_indicator|working_status|shortcut_hud|effort_ignition|sparkle"),
    ("w2-transcript", r"^pager|diff|pager_|\bfind|search_footer|offline_find|highlight_match|n_and_shift"),
    ("w2-transcript", r"select_mode|copy_mode|copy_picker|slash_copy|clipboard|selection|double_click|triple|markdown_copy|copy_payload|^transcript_view_4"),
    ("w2-transcript", r"^transcript_view|prompt_header|turn_details|turn_tip|disclosure|follow|history_page|mouse_scroll|focus_activity"),
    ("w2-cells", r"exploration|unified_exec|background_terminal|\bps_|slash_ps|processes|mcp_inventory|inventory|slash_mcp|waited_for"),
    ("w2-approvals", r"approval|elicitation|request_user_input|question|decision|write_stdin|network_host|async_"),
    ("w2-composer", r"queue|queued|external_editor|ctrl_g|composer_state|read_only|right_click|mouse|drain"),
    ("voice", r"voice|realtime|split_flap|session_lock"),
    ("sidebar", r"sidebar"),
    ("inline-mode", r"inline|scrollback|reflow|committed|insert_history"),
    ("ui-tree-viz", r"ui_tree|visualization"),
    ("pets-ambient", r"\bpet|pet_"),
    ("startup-rest", r"^onboarding|^signin|^startup_prompts|^import|startup_|trust|update_prompt|oss_|unarchive|cwd_prompt|migration"),
    ("resume-rest", r"^resume|resume_picker|thread_colors"),
    ("agents-center", r"agents?_(?:overview|center|focus|mouse)|agent_status|^agents"),
    ("w2-live", r"^markdown|math|mermaid|soft_break|nested_list|lists_preserve|heading|table_"),
    ("w2-cells", r"^cells|^renderer"),
    ("w2-approvals", r"^small_dialogs|shortcut|warnings|^picker|^chat_popups|popup|model_|permissions_"),
    ("w2-composer", r"^textarea|^composer|paste|vim|history_search|mention|startup_draft|steer|attachment|image"),
    ("w2-approvals", r"^chat_approvals|^approvals"),
    ("w2-live", r"^chat_live|^chat_commands|^chat_status|backtrack|banner|plan_|review"),
]


SHARD_FALLBACK = {"usage": "w2-status", "status": "w2-status", "pager": "w2-transcript",
                  "picker": "w2-approvals", "renderer_internals": "w2-cells", "signin": "startup-rest",
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
    return {"chat_live": "w2-live", "chat_commands": "w2-live", "chat_status": "w2-status",
            "chat_approvals": "w2-approvals", "approvals": "w2-approvals", "cells": "w2-cells",
            "chat_popups": "w2-approvals", "small_dialogs": "w2-approvals", "transcript_view": "w2-transcript",
            "composer": "w2-composer", "composer_popups": "w2-composer", "markdown_streaming": "w2-live",
            "pane_footer": "w2-status", "resume": "resume-rest", "textarea": "w2-composer"}.get(base)
