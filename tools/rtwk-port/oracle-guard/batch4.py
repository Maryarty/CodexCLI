"""chat_status_*, composer_popups, markdown_*, pager, pane_footer_2, status, transcript_view_*: rows, links, titles through the kit."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import Shard  # noqa: E402

# ---- chat_status_2
s = Shard('chat_status_2')
s.rx(r'assert!\(\s*s\s*\.rows\(\)\s*\.iter\(\)\s*\.any\(\|row\| row\.trim\(\) == ("[^"]*")\),\s*"[^"]*",\s*s\.text\(\)\s*\);',
     r's.assert_has_row_trimmed(\1);', count=None)
s.rx(r'assert!\(\n            s\.rows\(\)\.iter\(\)\.any\(\|row\| row\.trim\(\) == text\),\n            "the status line is `\{text\}`\\n\{\}",\n            s\.text\(\)\n        \);',
     's.assert_has_row_trimmed(text);', count=None)
s.rx(r'fn rows_with\(s: &Screen, text: &str\) -> usize \{\n    s\.rows\(\)\.iter\(\)\.filter\(\|row\| row\.contains\(text\)\)\.count\(\)\n\}\n\n', '', count=1)
s.rx(r'assert_eq!\(rows_with\(&s, ("[^"]*")\), (\d)\);', r's.assert_rows_with(\1, \2);', count=2)
s.rx(r'assert_eq!\(s\.requests_of\("account/rateLimits/read"\) - startup, (\d)\);', r's.assert_requests_of("account/rateLimits/read", startup + \1);', count=2)
s.sub('.size(crate::common::WIDTH, 60)', '.height(60)')
s.save()

# ---- chat_status_3
s = Shard('chat_status_3')
s.sub('''    let before = s.title().expect("a title while the turn runs");
    assert!(before.ends_with("project"), "the running title: {before:?}");''', '    s.assert_title_ends_with("project");')
s.sub('''    let after = s.title().expect("a title after the approval");
    assert!(after.contains("project"), "restored title: {after:?}");
    assert!(
        !after.contains("Action Required"),
        "restored title: {after:?}"
    );''', '''    s.assert_title_has("project");
    s.assert_title_lacks("Action Required");''')
s.rx(r'assert!\(\n        s\.rows\(\)\n            \.iter\(\)\n            \.any\(\|row\| row\.trim\(\) == (format!\([^\n]*\))\),\n        "[^"]*",\n        s\.text\(\)\n    \);',
     r's.assert_has_row_trimmed(&\1);', count=2)
s.save()

# ---- composer_popups
s = Shard('composer_popups')
s.rx(r'/// The rows of the oracle snapshot `name` equal ours from screen row `row`, trailing blanks\n.*?\nfn assert_rows_at\(.*?\n}\n\n', '', count=1)
s.rx(r'(?<![\w.])assert_rows_at\(\s*&(\w+),\s*([^,]+),\s*([^;]+?),?\s*\);', r'\1.assert_snap_rows_at(\2, \3);', count=None)
s.rx(r'    let mut seen = 0;\n    for \(y, row\) in s\.rows\(\)\.iter\(\)\.enumerate\(\) \{\n        if row\.trim_end\(\)\.ends_with\("File"\) \{\n            s\.assert_text_at\(12, y, "src/"\);\n            seen \+= 1;\n        \}\n    \}\n    assert_eq!\(seen, 2, "both file rows"\);',
     '    s.assert_rows_ending_have_text_at("File", 12, "src/", 2);', count=1)
s.save()

# ---- pane_footer_2
s = Shard('pane_footer_2')
s.rx(r'fn assert_rows_of\(s: &Screen, snap: &str\) \{\n.*?\n\}\n', 'fn assert_rows_of(s: &Screen, snap: &str) {\n    s.assert_snap_head(snap);\n}\n', count=1)
s.save()

# ---- pager
s = Shard('pager')
s.sub('    let initial = s.text();\n    s.keys("ctrl+f");\n    assert_ne!(s.text(), initial, "Ctrl+F moves a page");\n    s.keys("pageup");\n    assert_eq!(s.text(), initial);',
      '    let initial = s.shot();\n    s.keys("ctrl+f");\n    s.assert_rows_differ(&initial);\n    s.keys("pageup");\n    s.assert_same_rows(&initial);')
s.save()

# ---- transcript_view_1 / 5
s = Shard('transcript_view_1')
s.sub('assert_eq!(s.page_requests().len(), 2, "{width} columns · {page:?}");', 's.assert_page_requests(2);')
s.save()
s = Shard('transcript_view_5')
s.rx(r'        let rows = s\.rows\(\);\n        let got: Vec<&str> = rows\n            \.iter\(\)\n            \.skip\(top\)\n            \.take\(frame\.len\(\)\)\n            \.map\(String::as_str\)\n            \.collect\(\);\n        if got != frame \{\n            wrong\.push\(format!\("\{name\}: \{got:\?\} != \{frame:\?\}"\)\);\n        \}',
     '        let got: Vec<String> = s.rows().into_iter().skip(top).take(frame.len()).collect();\n        kit::assert_rows_are(&got, frame);', count=1)
s.rx(r'    assert!\(\n        wrong\.is_empty\(\),\n        "frames that differ:\\n\{\}",\n        wrong\.join\("\\n"\)\n    \);\n', '', count=1)
s.sub('    let mut wrong = Vec::new();\n', '')
s.save()

# ---- markdown
s = Shard('markdown_streaming_1')
s.rx(r'    let rows = s\.grid\(\)\.rows\(false\);\n    let at = rows\n        \.iter\(\)\n        \.position\(\|row\| row == "first \(first\.txt\)"\)\n        \.unwrap_or_else\(\|\| panic!\("no `first \(first\.txt\)` row:\\n\{\}", s\.text\(\)\)\);\n    assert_eq!\(rows\.get\(at \+ 1\)\.map\(String::as_str\), Some\("second\.txt"\)\);',
     '    s.assert_row_then("first (first.txt)", &["second.txt"]);', count=1)
s.save()
s = Shard('markdown_streaming_2')
s.rx(r'    let rows = s\.grid\(\)\.rows\(false\);\n    let at = rows\n        \.iter\(\)\n        \.position\(\|row\| row == "Hello"\)\n        \.unwrap_or_else\(\|\| panic!\("no `Hello` row:\\n\{\}", s\.text\(\)\)\);\n    assert_eq!\(rows\.get\(at \+ 1\)\.map\(String::as_str\), Some\("World"\)\);',
     '    s.assert_row_then("Hello", &["World"]);', count=1)
s.rx(r'            let grid = s\.grid\(\);\n            let targets: std::collections::BTreeSet<String> = .*?\n                \.collect\(\);\n            assert_eq!\(\n                targets\.iter\(\)\.map\(String::as_str\)\.collect::<Vec<_>>\(\),\n                (\[[^\]]*\]),\n                "width \{width\}, \{markdown\}:\\n\{\}",\n                s\.text\(\)\n            \);',
     r'            s.assert_link_targets(&\1);', count=1)
s.rx(r'    let grid = s\.grid\(\);\n    let \(x, _, y\) = grid\n        \.locate\("plain plain code code"\)\n        \.unwrap_or_else\(\|\| panic!\("no label-only row:\\n\{\}", s\.text\(\)\)\);\n    let linked = \|from: usize, len: usize\| \{\n        \(from\.\.from \+ len\)\.all\(\|c\| grid\.cell\(c, y\)\.link\.as_deref\(\) == Some\("https://example\.com"\)\)\n    \};\n    assert!\(!linked\(x, 5\), "the prose `plain` links"\);\n    assert!\(linked\(x \+ 6, 5\), "the label `plain` has no link"\);\n    assert!\(!linked\(x \+ 12, 4\), "the prose `code` links"\);\n    assert!\(linked\(x \+ 17, 4\), "the code label has no link"\);',
     '    let row = "plain plain code code";\n    let url = "https://example.com";\n    s.assert_span_linked(row, 0, 5, url, false);\n    s.assert_span_linked(row, 6, 5, url, true);\n    s.assert_span_linked(row, 12, 4, url, false);\n    s.assert_span_linked(row, 17, 4, url, true);', count=1)
s.rx(r'    let grid = s\.grid\(\);\n    let \(x, _, y\) = grid\n        \.locate\("plain plain"\)\n        \.expect\("the two words on the screen"\);\n    for x in x \+ "plain "\.len\(\)\.\.x \+ "plain plain"\.len\(\) \{\n        assert_eq!\(grid\.cell\(x, y\)\.link\.as_deref\(\), Some\("https://example\.com"\)\);\n    \}',
     '    s.assert_span_linked("plain plain", "plain ".len(), "plain".len(), "https://example.com", true);', count=1)
s.save()
s = Shard('markdown_streaming_3')
s.rx(r'    let rows = s\.grid\(\)\.rows\(false\);\n    let at = rows\n        \.iter\(\)\n        \.position\(\|row\| row == "• item line1"\)\n        \.unwrap_or_else\(\|\| panic!\("no `• item line1` row:\\n\{\}", s\.text\(\)\)\);\n    assert_eq!\(rows\.get\(at \+ 1\)\.map\(String::as_str\), Some\("  item line2"\)\);',
     '    s.assert_row_then("• item line1", &["  item line2"]);', count=1)
s.save()
s = Shard('markdown_streaming_5')
s.rx(r'    let rows: Vec<String> = s\.rows\(\)\.iter\(\)\.map\(\|r\| r\.trim_end\(\)\.to_string\(\)\)\.collect\(\);\n    let at = rows\.iter\(\)\.position\(\|r\| r == "alpha"\)\.expect\("alpha row"\);\n    assert_eq!\(rows\[at \+ 1\], ""\);\n    assert_eq!\(rows\[at \+ 2\], "beta"\);',
     '    s.assert_row_then("alpha", &["", "beta"]);', count=1)
s.save()

# ---- status
s = Shard('status')
s.rx(r'        let grid = s\.grid\(\);\n        let mut fragments = String::new\(\);.*?\n        assert!\(\n            destinations\n                \.iter\(\)\n                \.all\(\|destination\| destination == "https://chatgpt\.com/settings/usage"\)\n        \);',
     '        s.assert_links_spell("https://chatgpt.com/settings/usage");', count=1)
s.rx(r'        let joined = s\n            \.grid\(\)\n            \.rows\(false\)\n            \.iter\(\)\n            \.map\(\|row\| row\.trim\(\)\.to_string\(\)\)\n            \.collect::<String>\(\);\n        assert!\(\n            joined\.contains\(("[^"]*")\),\n            "path lost at width \{width\}: \{joined\}"\n        \);\n        assert!\(\n            joined\.contains\(("[^"]*")\),\n            "session lost at width \{width\}: \{joined\}"\n        \);',
     r'        s.assert_joined_rows_hold(\1);\n        s.assert_joined_rows_hold(\2);', count=1)
s.save()
