"""Shards chat_popups_*: requests, rows and asks through the kit."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import Shard  # noqa: E402

# ---- chat_popups_1
s = Shard('chat_popups_1')
s.rx(r'let patches = s\.request_params\("thread/settings/update"\);\n        assert!\(\n            patches\.iter\(\)\.any\(\|p\| p\.to_string\(\)\.contains\("ultrafast"\)\),\n            "\{name\}: \{patches:\?\}"\n        \);',
     's.assert_request_mentions("thread/settings/update", "ultrafast");', count=1)
s.rx(r'let writes = s\.request_params\("config/batchWrite"\);\n        assert!\(\n            writes\.iter\(\)\.any\(\|p\| p\.to_string\(\)\.contains\("ultrafast"\)\),\n            "\{name\}: \{writes:\?\}"\n        \);',
     's.assert_request_mentions("config/batchWrite", "ultrafast");', count=2)
s.rx(r'let patches = s\.request_params\("thread/settings/update"\);\n    assert!\(\n        patches\n            \.iter\(\)\n            \.any\(\|p\| p\.to_string\(\)\.contains\("us\.openai\.gpt-5\.6-luna"\)\),\n        "\{patches:\?\}"\n    \);',
     's.assert_request_mentions("thread/settings/update", "us.openai.gpt-5.6-luna");', count=1)
s.rx(r'let line = \|s: &Screen, text: &str\| \{\n        assert!\(\n            s\.rows\(\)\.iter\(\)\.any\(\|row\| row\.trim\(\) == text\),\n            "the status line is `\{text\}`\\n\{\}",\n            s\.text\(\)\n        \);\n    \};',
     'let line = |s: &Screen, text: &str| s.assert_has_row_trimmed(text);', count=1)
s.save()

# ---- chat_popups_2
s = Shard('chat_popups_2')
s.rx(r'    let collapsed = s\n        \.rows\(\)\n        \.join\(" "\)\n        \.split_whitespace\(\)\n        \.collect::<Vec<_>>\(\)\n        \.join\(" "\);\n    assert!\(collapsed\.contains\("\(disabled\)"\), "\{collapsed\}"\);\n    assert!\(\n        collapsed\.contains\("this message should be printed in the description"\),\n        "\{collapsed\}"\n    \);',
     '    s.assert_reads("(disabled)");\n    s.assert_reads("this message should be printed in the description");', count=1)
s.rx(r'assert_eq!\(\n        rows,\n        \[(.*?)\n        \]\n', r'kit::assert_rows_are(\n        &rows,\n        &[\1\n        ]\n', count=1)
s.rx(r'    let row = s\n        \.rows\(\)\n        \.into_iter\(\)\n        \.find\(\|row\| row\.contains\("Docs"\)\)\n        \.expect\("the all-plugins row"\);\n    assert!\(row\.contains\("Installed"\), "\{row\}"\);\n    assert!\(!row\.contains\("Local curated docs plugin\."\), "\{row\}"\);',
     '    s.assert_row_of("Docs", &["Installed"], &["Local curated docs plugin."]);', count=1)
s.rx(r'    let text = s\.text\(\);\n    let install_row = text\n        \.lines\(\)\n        \.find\(\|line\| line\.contains\("Install plugin"\)\)\n        \.expect\("the install row"\);\n    assert!\(\n        install_row\.contains\("This plugin is not installable from this marketplace"\),\n        "\{install_row\}"\n    \);',
     '    s.assert_row_of(\n        "Install plugin",\n        &["This plugin is not installable from this marketplace"],\n        &[],\n    );', count=1)
s.save()

# ---- chat_popups_3 (the rows; the plugin wants are in batch3)
s = Shard('chat_popups_3')
s.rx(r'    let text = flat\(&s\);\n    for fragment in \[(.*?)\] \{\n        assert!\(text\.contains\(fragment\), "\{fragment\}\\n\{\}", s\.text\(\)\);\n    \}',
     r'    for fragment in [\1] {\n        s.assert_reads(fragment);\n    }', count=1)
s.rx(r'    let asked = apps_asked\(&s\);\n    assert_eq!\(asked\.len\(\), (\d), "\{asked:\?\}"\);',
     r'    s.assert_apps_asked(\1);\n    let asked = s.asked_apps();', count=5)
s.rx(r'/// The `app/list` requests the app has made, oldest first\.\npub\(crate\) fn apps_asked.*?\n}\n\n', '', count=1)
s.save()

# ---- chat_popups_4
s = Shard('chat_popups_4')
s.rx(r'    let asked = apps_asked\(&s\);\n    assert_eq!\(asked\.len\(\), (\d), "\{asked:\?\}"\);',
     r'    s.assert_apps_asked(\1);\n    let asked = s.asked_apps();', count=1)
s.sub('chat_popups_3::apps_asked', 'XX', 0) if False else None
s.rx(r'/// The `model/list` the app has asked last, still unanswered\.\nfn last_model_list\(s: &Screen\) -> codex_tui_core::app::Command \{.*?\n}\n',
     '/// The `model/list` the app has asked last, still unanswered.\nfn last_model_list(s: &Screen) -> kit::Ask {\n    s.asked_models()\n        .pop()\n        .expect("the app asked for the model list")\n}\n', count=1)
s.rx(r'            let sent = s\.request_params\("thread/settings/update"\);\n            let sent = sent\n                \.last\(\)\n                \.expect\("the shortcut asks the server for the effort"\)\n                \.to_string\(\);\n            assert!\(\n                sent\.contains\(r#""max""#\),\n                "\{key\} in plan=\{plan_mode\} sent \{sent\}"\n            \);\n            // Plan mode steps the Plan override: the mode rides with the effort\n            assert_eq!\(sent\.contains\(r#""plan""#\), plan_mode, "\{key\}: \{sent\}"\);',
     '            s.assert_last_request_has("thread/settings/update", r#""max""#);\n            // Plan mode steps the Plan override: the mode rides with the effort\n            s.assert_last_request_mentions("thread/settings/update", r#""plan""#, plan_mode);', count=1)
s.sub('        let before = s.request_params("thread/settings/update").len();', '        let before = s.requests_of("thread/settings/update");')
s.sub('        assert_eq!(s.request_params("thread/settings/update").len(), before);', '        s.assert_requests_of("thread/settings/update", before);')
s.rx(r'    assert!\(\n        s\.request_params\("thread/settings/update"\)\.is_empty\(\),\n        "nothing is applied before the selection"\n    \);',
     '    s.assert_no_request("thread/settings/update");', count=1)
s.sub('    let asked = s.commands().len();', '    let asked = s.command_count();')
s.rx(r'    let new = s\.commands\(\)\.split_off\(asked\);\n    assert!\(\n        new\.iter\(\)\.all\(.*?\n    \);\n',
     '    s.assert_no_models_asked_since(asked);\n', count=1)
s.save()

# ---- chat_popups_4: the review picker
s = Shard('chat_popups_4')
s.rx(r'    use codex_tui_core::app::Command;\n    use codex_tui_core::app::effects::\{Ask, Done\};\n    use codex_tui_core::screens::ScreenCommand;\n    use codex_tui_core::screens::chat;\n    use codex_tui_core::screens::select::review::Commit;\n    use codex_tui_core::threads::ReviewTarget;\n\n', '', count=1)
s.rx(r'    let commits = \|\| \{\n        Done::GitLog\(vec!\[.*?\n        \]\)\n    \};\n',
     '    let commits = [("1111111", "First commit"), ("abcdef2", "Selected commit")];\n', count=1)
s.rx(r'    let asked = \|s: &Screen\| \{\n        s\.commands\(\)\n            \.into_iter\(\)\n            \.filter\(.*?\n            \.collect::<Vec<_>>\(\)\n    \};\n', '', count=1)
s.rx(r'    let asks = asked\(&s\);\n    assert_eq!\(asks\.len\(\), (\d), "\{asks:\?\}"\);\n    s\.answer_effect\(&asks\[(\d)\], commits\(\)\);',
     r'    s.assert_git_logs_asked(\1);\n    let asks = s.asked_git_logs();\n    s.answer_git_log(&asks[\2], &commits);', count=2)
s.rx(r'    let parent = s\.text\(\);\n    assert!\(parent\.contains\("› 3\. Review a commit"\), "\{parent\}"\);\n    assert!\(parent\.contains\("f3 select · f2 back"\), "\{parent\}"\);',
     '    s.assert_contains("› 3. Review a commit");\n    s.assert_contains("f3 select · f2 back");', count=1)
s.rx(r'    let started: Vec<_> = s\n        \.commands\(\)\n        \.into_iter\(\)\n        \.filter_map\(.*?\n    assert_eq!\(\n        started,\n        \[ReviewTarget::Commit \{\n            sha: "abcdef2"\.into\(\),\n            title: Some\("Selected commit"\.into\(\)\)\n        \}\]\n    \);',
     '    s.assert_review_of_commit("abcdef2", "Selected commit");', count=1)
s.save()
