"""f-select-mode: un-pend the select-mode tests of tests/oracle/transcript_view_4.rs on the view_only door.

For each named test fn: drops its `kit::pending(..);` statement when it opens the body and puts `.view_only()` before every
`.open()` of a `Screen::build(..)` chain that lacks it. Rerunnable (a second run changes nothing).
Run from the repo root:  python3 -I tools/rtwk-port/f-select-mode/unpend.py
"""
import re
import sys

PATH = "codex-rs/tui-rtwk/tests/oracle/transcript_view_4.rs"
# (accepted_find_pins_an_expanded_match_before_select_mode_exits stays pending: G-new-find-room-text)
TESTS = """accepted_find_repeats_into_the_select_cursor
character_motion_crosses_line_and_entry_boundaries_without_invisible_stops
configured_pager_page_motion_moves_the_select_cursor
confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme
enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows
entry_starts_on_the_last_selectable_visible_line
find_accepts_at_the_match_and_cancel_restores_the_saved_selection
logical_line_end_handles_empty_lines_and_shifted_dollar
passive_mouse_events_do_not_reanchor_a_reverse_line_mark
reverse_line_selection_copies_complete_lines_in_document_order
select_mode_renders_cursor_and_scannable_footer_hints
swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint
transcript_end_character_selection_includes_the_last_grapheme
vertical_motion_keeps_the_preferred_display_column
word_motion_crosses_entries_and_word_end_selects_the_destination_word""".split()

src = open(PATH).read()
for name in TESTS:
    m = re.search(r"\nfn %s\(\) \{\n(.*?)\n\}\n" % name, src, re.S)
    if not m:
        sys.exit("no fn " + name)
    body = m.group(1)
    # only the marker that opens the body: a marker later in it is a gap of a case that is written
    new = re.sub(r"\A    kit::pending\(\n.*?\n    \);\n", "", body, count=1, flags=re.S)
    # a `.view_only()` before each `.open()` that has none right above it
    lines = new.split("\n")
    out = []
    for line in lines:
        if line.strip() == ".open();" and not (out and out[-1].strip() == ".view_only()"):
            out.append(line.replace(".open();", ".view_only()"))
        out.append(line)
    src = src[: m.start(1)] + "\n".join(out) + src[m.end(1) :]
open(PATH, "w").write(src)
