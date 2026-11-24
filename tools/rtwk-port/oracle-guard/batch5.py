"""pane_footer_1: the sparkle helpers live in tests/kit/sparkle.rs; the shard calls them as Screen methods."""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import Shard  # noqa: E402

s = Shard('pane_footer_1')
# drop the local helpers that moved to the kit
s.rx(r'/// The braille starfield dots of the oracle.s sparkle.*?\n/// The first row that contains `text`', '/// The first row that contains `text`', count=1)
s.rx(r'/// The composer.s three rows \(the blank row above.*?\nconst FIND_REASON', 'const FIND_REASON', count=1)
s.rx(r'/// `assert_help_snap` against the rows of one frame.*?\n}\n\n(?=// ORACLE)', '', count=1)
s.rx(r'/// The oracle.s sparkle palette', '/// The oracle\'s sparkle palette', count=1)
s.rx(r'fn assert_help_snap\(s: &Screen, name: &str\) \{\n    assert_help_rows\(s, &kit::snap_rows\(name, ""\), name\);\n\}\n\n', '', count=1)
# calls
s.rx(r'assert_no_dots_in\(&draft\);', 's.assert_no_dots_in_draft();', count=1)
s.rx(r'    let rows = s\.rows\(\);\n    let prompt = rows\n        \.iter\(\)\n        \.position\(\|row\| row\.starts_with\(.›.\)\)\n        \.expect\("the prompt row"\);\n    let draft: String = rows\[prompt\]\.chars\(\)\.skip\(2\)\.take\(22\)\.collect\(\);\n    s\.assert_no_dots_in_draft\(\);', '    s.assert_no_dots_in_draft();', count=1)
s.rx(r'assert_no_dots\(&(\w+)\);', r'\1.assert_no_dots();', count=None)
s.rx(r'assert_stars\(\s*&(\w+),\s*', r'\1.assert_stars(', count=None)
s.rx(r'assert_layout\(\s*&(\w+),\s*', r'\1.assert_layout(', count=None)
s.rx(r'assert_empty_draft\(&(\w+)\);', r'\1.assert_empty_draft();', count=None)
s.rx(r'assert_help_snap\(&(\w+), ', r'\1.assert_help_snap(', count=None)
s.rx(r'assert_help_rows\(&(\w+), ', r'\1.assert_help_rows(', count=None)
# lit_dots
s.rx(r'assert!\(\s*!fading\.is_empty\(\), "the fading frame has dots"\);\n', '', count=1)
s.rx(r'    for \(x, y, glyph, contrast\) in &fading \{.*?\n    \}\n    s\.wait\(800\);', '    s.assert_dots_fade_from(&last);\n    s.wait(800);', count=1)
s.rx(r'    let fading = lit_dots\(&s\);\n', '', count=1)
s.rx(r'let last = lit_dots\(&s\);', 'let last = s.dots();', count=1)
s.rx(r'assert!\(\s*!lit_dots\(&(\w+)\)\.is_empty\(\)(?:,\s*"[^"]*")?\s*\);', r'\1.assert_dots(true);', count=None)
s.rx(r'assert_eq!\(\s*lit_dots\(&s\)\.is_empty\(\),\s*!(\w+),\s*("[^"]*")\s*\);', r's.assert_dots_lit(\1, \2);', count=None)
s.rx(r'assert_eq!\(\s*lit_dots\(&s\)\.is_empty\(\),\s*!(\w+)\s*\);', r's.assert_dots_lit(\1, "");', count=None)
s.rx(r'assert_eq!\(\s*lit_dots\(&s\)\.is_empty\(\),\s*completion == "draft",\s*"\{completion\}"\s*\);', 's.assert_dots_lit(completion != "draft", completion);', count=1)
s.rx(r'assert_eq!\(\s*lit_dots\(&s\)\.is_empty\(\),\s*completion == "accept",\s*("[^"]*")\s*\);', r's.assert_dots_lit(completion != "accept", &format!(\1));', count=1)
s.rx(r'assert_eq!\(!lit_dots\(&s\)\.is_empty\(\), back, ("[^"]*")\);', r's.assert_dots_lit(back, &format!(\1));', count=1)
# cursor on last row
s.rx(r'    let \(_, row\) = s\n        \.grid\(\)\n        \.cursor\n        \.expect\("the cursor is shown in the history search"\);\n    assert_eq!\(\n        row,\n        s\.rows\(\)\.len\(\) - 1,\n        "the cursor is on the final row:\\n\{\}",\n        s\.text\(\)\n    \);', '    s.assert_cursor_on_last_row();', count=1)
s.rx(r'    let frames: Vec<&\[String\]> = oracle\.split\(\|row\| row == "---"\)\.collect\(\);\n    assert_eq!\(frames\.len\(\), 3, "the snapshot holds three renders"\);', '    let frames = kit::split_frames(&oracle, 3);', count=1)
s.rx(r's\.assert_help_rows\(frame, NAME\);', 's.assert_help_rows(&frame, NAME);', count=1)
s.save()
