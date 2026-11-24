"""Shards cells_*, approvals_1, chat_commands_*, chat_live_*, composer_*: the oracle's own
expectations through the kit (assert_sole_turn, assert_reads, assert_rows_are, ...)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import Shard  # noqa: E402

# ---- cells_1
s = Shard('cells_1')
s.sub('    assert!(s.rows().iter().all(|r| r.chars().count() <= 12));', '    s.assert_rows_fit(12);')
s.rx(r'assert_eq!\(\n        cell_rows\(&s, (\d+)\),\n        \[(.*?)\],\n    \);',
     r'kit::assert_rows_are(\n        &cell_rows(&s, \1),\n        &[\2],\n    );', count=3)
s.sub('''    assert_eq!(
        rows[rows.len() - 2..],
        ["    Script error:", "    permission denied"],
        "{rows:?}"
    );''', '''    kit::assert_rows_are(
        &rows[rows.len() - 2..],
        &["    Script error:", "    permission denied"],
    );''')
s.save()

# ---- cells_2
s = Shard('cells_2')
s.sub('    assert!(!s.text().contains("OpenAI Codex"), "{}", s.text());', '    s.assert_absent("OpenAI Codex");', 3)
s.sub('    assert_eq!(s.page_requests().len(), 2);', '    s.assert_page_requests(2);', 3)
s.sub('''    let first = s.rows().into_iter().find(|row| !row.is_empty());
    assert_eq!(
        first.as_deref().map(str::trim),
        Some(">_ OpenAI Codex (v0.170.0)")
    );''', '    s.assert_first_row(">_ OpenAI Codex (v0.170.0)");')
s.save()

# ---- cells_3
s = Shard('cells_3')
s.sub('''    let at = |row: usize| rows.get(row).map_or("", |row| row.trim_end());
    assert_eq!(at(y.saturating_sub(1)), "", "a blank row above the header");
    s.assert_row_starts_with(y, "  >_ OpenAI Codex (v");
    assert!(at(y).ends_with(')'), "{}", at(y));
    assert_eq!(at(y + 1), "     /tmp/project");
    assert_eq!(at(y + 2), "");
    assert_eq!(at(y + 3), "  Tip: Model just became available");''',
'''    s.assert_row(y.saturating_sub(1), "");
    s.assert_row_starts_with(y, "  >_ OpenAI Codex (v");
    s.assert_row_ends_with(y, ")");
    s.assert_row(y + 1, "     /tmp/project");
    s.assert_row(y + 2, "");
    s.assert_row(y + 3, "  Tip: Model just became available");''')
s.save()

# ---- cells_5
s = Shard('cells_5')
s.sub('''    let bullets = s
        .rows()
        .iter()
        .filter(|row| row.trim_start().starts_with('•'))
        .count();
    assert_eq!(bullets, 1, "{}", s.text());''', '    s.assert_rows_starting("•", 1);')
s.save()

# ---- approvals_1
s = Shard('approvals_1')
s.sub('s.paste(&"x".repeat(codex_tui_core::screens::composer::MAX_INPUT_CHARS + 1));', 's.paste_over_the_input_limit();')
s.save()

# ---- chat_commands_1
s = Shard('chat_commands_1')
s.sub('use codex_tui_core::app::facts::TuiSettings;\n', '')
s.rx(r'assert!\(s\.turns\(\)\.is_empty\(\), [^;]*\);', 's.assert_no_turn();', count=4)
s.rx(r'assert!\(\n            !s\.commands\(\)\.iter\(\)\.any\(\|c\| matches!\(.*?\n        \);',
     's.assert_no_shell_or_compact();', count=1)
s.rx(r'    let said = s\n        \.rows\(\)\n        \.iter\(\)\n        \.filter\(\|row\| \{\n            row\.trim_end\(\)\n                == ("[^"]*")\n        \}\)\n        \.count\(\);\n    assert_eq!\(said, 3, "\{\}", s\.text\(\)\);',
     r'    s.assert_row_count(\1, 3);', count=1)
s.sub('assert!(s.request_params("thread/settings/update").is_empty());', 's.assert_no_request("thread/settings/update");')
s.rx(r'assert!\(\n        !s\.commands\(\)\n            \.iter\(\)\n            \.any\(\|c\| format!\("\{c:\?\}"\)\.contains\("Shell"\)\),\n        "\{:\?\}",\n        s\.commands\(\)\n    \);',
     's.assert_no_shell_command();', count=1)
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(turns[0].text, "[Image #1]!");
    assert_eq!(turns[0].images.len(), 1);
    assert_eq!(turns[0].images[0].path, path);
    assert_eq!(turns[0].images[0].range, 0.."[Image #1]".len());''',
'''    s.assert_sole_turn(kit::Turn::text("[Image #1]!").images(&[(&path, 0.."[Image #1]".len())]));''')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(
        turns[0].remote_images,
        vec!["https://example.com/remote.png"]
    );
    assert_eq!(turns[0].images.len(), 1);
    assert_eq!(turns[0].images[0].path, path);
    assert_eq!(turns[0].images[0].range, 0.."[Image #2]".len());
    assert_eq!(turns[0].text, "[Image #2] submit mixed");''',
'''    s.assert_sole_turn(
        kit::Turn::text("[Image #2] submit mixed")
            .remote_images(&["https://example.com/remote.png"])
            .images(&[(&path, 0.."[Image #2]".len())]),
    );''')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(
        turns[0].remote_images,
        vec!["https://example.com/remote-only.png"]
    );
    assert!(turns[0].images.is_empty());
    assert_eq!(turns[0].text, "");''',
'''    s.assert_sole_turn(
        kit::Turn::text("")
            .remote_images(&["https://example.com/remote-only.png"])
            .no_images(),
    );''')
s.sub('''.tui(TuiSettings {
                vim_mode_default: true,
                ..TuiSettings::default()
            })''', '.vim_mode()')
s.save()

# ---- chat_commands_2
s = Shard('chat_commands_2')
s.rx(r'/// The names the app asked the server to give the thread \(`Op::SetThreadName`\)\.\nfn set_names.*?\n}\n\n/// The host answers the title suggestion.*?\n}\n\n', '', count=1)
s.sub('answer_title(&mut s, ', 's.answer_title(', 3)
s.rx(r'assert_eq!\(set_names\(&s\), (\[[^\]]*\])\);', r's.assert_set_names(&\1);', count=2)
s.save()

# ---- chat_commands_3
s = Shard('chat_commands_3')
s.sub('use codex_tui_core::app::Command;\nuse codex_tui_core::screens::{ScreenCommand, agents};\n\n', '')
s.rx(r'assert_eq!\(archives\(&(\w)\), (\d)\);', r'\1.assert_archives(\2);', count=5)
s.rx(r'assert_eq!\(deletes\(&(\w)\), (\d)\);', r'\1.assert_deletes(\2);', count=2)
s.rx(r"/// The oracle's `AppEvent::ArchiveCurrentThread`.*?\n        \.count\(\)\n}\n\n", '', count=1)
s.rx(r'assert!\(\n        s\.commands\(\)\.iter\(\)\.any\(\|command\| matches!\(\n            command,\n            codex_tui_core::app::Command::Screen \{.*?\n        s\.commands\(\)\n    \);',
     's.assert_clean_terminals_sent();', count=1)
s.rx(r'assert_eq!\(\n        s\.text\(\)\n            \.matches\("([^"]*)"\)\n            \.count\(\),\n        1\n    \);',
     r's.assert_text_count("\1", 1);', count=1)
s.sub('/// The glyphs of the sparkle\'s dots.\nconst SPARKLE_DOTS: &str = "⠁⠂⠄⠈⠐⠠⡀⢀";\n', '')
s.rx(r'    let shown = \|s: &Screen\| s\.text\(\)\.chars\(\)\.any\(\|c\| SPARKLE_DOTS\.contains\(c\)\);\n    assert!\(\n        shown\(&s\),\n        "the dots show on a fresh Astra thread:\\n\{\}",\n        s\.text\(\)\n    \);',
     '    s.assert_dots(true);', count=1)
s.sub('    assert!(!shown(&s), "the submit consumed the sparkle:\\n{}", s.text());', '    s.assert_dots(false);')
s.sub('    assert!(!shown(&s), "and it does not come back:\\n{}", s.text());', '    s.assert_dots(false);')
s.save()

# ---- chat_live_1
s = Shard('chat_live_1')
s.rx(r'/// The screen\'s text without its white space.*?\n/// Whether the screen reads `text`.*?\nfn reads\(s: &Screen, text: &str\) -> bool \{\n.*?\n}\n\n', '', count=1)
s.rx(r'assert!\(\n        reads\(&s, ([^\n]*?)\),\n        "\{\}",\n        s\.text\(\)\n    \);', r's.assert_reads(\1);', count=2)
s.rx(r'assert!\(\n        reads\(\n            &s,\n            ("[^"]*")\n        \),\n        "\{\}",\n        s\.text\(\)\n    \);', r's.assert_reads(\1);', count=2)
s.rx(r'assert!\(reads\(&s, (\w+)\), "\{\}", s\.text\(\)\);', r's.assert_reads(\1);', count=2)
s.save()

# ---- chat_live_2
s = Shard('chat_live_2')
s.sub('assert_eq!(s.text().contains("esc to dismiss"), dismissible);', 's.assert_present("esc to dismiss", dismissible);')
s.sub('assert_eq!(s.nudges().len(), 1, "{:?}", s.nudges());', 's.assert_nudges(1);')
s.sub('assert_eq!(s.nudges().len(), 1);', 's.assert_nudges(1);')
s.sub('assert_eq!(s.composer_text(), "");', 's.assert_composer("");')
s.rx(r'assert_eq!\(\n            s\.text\(\)\.contains\("Selected model usage exhausted"\),\n            !dismissible\n        \);',
     's.assert_present("Selected model usage exhausted", !dismissible);', count=2)
s.rx(r'    // the first fallback the picker lists, test-model-c, is asked of the thread\n    let updates = \|s: &Screen\| \{.*?\n    \};\n', '    // the first fallback the picker lists, test-model-c, is asked of the thread\n', count=1)
s.sub('assert_eq!(updates(&s), ["test-model-c"]);', 's.assert_model_updates(&["test-model-c"]);')
s.sub('assert_eq!(updates(&s), ["test-model-c"], "no second switch");', 's.assert_model_updates(&["test-model-c"]);')
s.sub('''        assert_eq!(
            s.opened_urls(),
            destination
                .map(str::to_owned)
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert!(s.turns().is_empty());''', '''        s.assert_opened_urls(&destination.into_iter().collect::<Vec<_>>());
        s.assert_no_turn();''')
s.save()

# ---- chat_live_4
s = Shard('chat_live_4')
s.sub('use codex_tui_core::app::facts::TuiSettings;\n', '')
s.sub('''.tui(TuiSettings {
            vim_mode_default: true,
            ..TuiSettings::default()
        })''', '.vim_mode()')
s.save()

# ---- chat_live_5
s = Shard('chat_live_5')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1);
    assert_eq!(
        turns[0]
            .images
            .iter()
            .map(|i| i.path.clone())
            .collect::<Vec<_>>(),
        vec![path]
    );''', '    s.assert_sole_turn(kit::Turn::any().image_paths(&[path.as_path()]));')
s.save()

# ---- composer_3
s = Shard('composer_3')
s.sub('''    assert_eq!(
        s.turns()[0].mentions,
        vec![codex_tui_core::threads::ToolMention::Skill {
            name: "figma".to_string(),
            path: "/tmp/user/figma/SKILL.md".to_string(),
        }]
    );''', '    s.assert_sole_turn(kit::Turn::any().skills(&[("figma", "/tmp/user/figma/SKILL.md")]));')
s.save()

s = Shard('composer_3')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(
        turns[0].remote_images,
        vec!["https://example.com/remote.png"]
    );
    assert_eq!(turns[0].images.len(), 1);
    assert_eq!(turns[0].images[0].path, path);
    s.keys("up");''', '''    s.assert_sole_turn(
        kit::Turn::any()
            .remote_images(&["https://example.com/remote.png"])
            .image_paths(&[path.as_path()]),
    );
    s.keys("up");''')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(turns[0].images.len(), 1);
    assert_eq!(turns[0].images[0].path, second);
    assert_ne!(turns[0].images[0].path, first);''', '''    s.assert_sole_turn(kit::Turn::any().image_paths(&[second.as_path()]));''')
s.save()

s = Shard('composer_4')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert!(
        turns[0].images.is_empty(),
        "{:?} {first:?} {second:?}",
        turns[0]
    );
    assert_eq!(turns[0].text, "X\\u{e9}hello\\nworld");''', '''    s.assert_sole_turn(kit::Turn::text("X\\u{e9}hello\\nworld").no_images());''')
s.save()

s = Shard('composer_3')
s.sub('''    let turns = s.turns();
    assert_eq!(turns.len(), 1, "{:?}", s.commands());
    assert_eq!(
        turns[0].remote_images,
        vec!["https://example.com/one.png", "https://example.com/two.png"]
    );
    assert!(turns[0].text.is_empty());''', '''    s.assert_sole_turn(
        kit::Turn::text("").remote_images(&["https://example.com/one.png", "https://example.com/two.png"]),
    );''')
s.save()
