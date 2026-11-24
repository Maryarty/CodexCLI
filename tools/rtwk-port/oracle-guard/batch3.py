"""chat_popups_3: the plugin backend's asks through the plugin seed module (fixtures/chat_popups_3/seed.rs)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import Shard  # noqa: E402

s = Shard('chat_popups_3')
s.sub('use codex_tui_core::features::plugins::{Location, Target, Want};\n', '')
s.rx(r'/// The screen\'s text with every run of white space one space.*?\nfn flat.*?\n}\n\n', '', count=1) if False else None
s.rx(r"/// The screen's text with every run of white space one space[^\n]*\nfn flat\(s: &Screen\) -> String \{\n[^\n]*\n\}\n\n", '', count=1)
s.rx(r'/// The config of a user whose `names` are git marketplaces\.\nfn config_with\(.*?\n}\n\n', '', count=1)
s.sub('config_with(', 'seed::config_with(', 0) if False else None
s.rx(r'(?<![:\w])config_with\(', 'seed::config_with(', count=None)
# the adds
s.rx(r'    let adds: Vec<_> = seed::wants\(&s\)\n        \.into_iter\(\)\n        \.filter\(\|want\| matches!\(want, Want::Add \{ \.\. \}\)\)\n        \.collect\(\);\n    assert_eq!\(\n        adds,\n        vec!\[Want::Add \{\n            source: "owner/repo"\.to_string\(\)\n        \}\]\n    \);',
     '    seed::assert_added(&s, &["owner/repo"]);', count=1)
s.rx(r'    let upgrades: Vec<_> = seed::wants\(&s\)\n        \.into_iter\(\)\n        \.filter\(\|want\| matches!\(want, Want::Upgrade \{ \.\. \}\)\)\n        \.collect\(\);\n    assert_eq!\(\n        upgrades,\n        vec!\[Want::Upgrade \{\n            name: Some\("repo"\.to_string\(\)\)\n        \}\]\n    \);',
     '    seed::assert_upgraded(&s, &[Some("repo")]);', count=1)
s.sub('seed::last(&s, |want| matches!(want, Want::Add { .. }))', 'seed::last_of(&s, seed::Kind::Add)')
s.sub('seed::last(&s, |want| matches!(want, Want::Remove { .. }))', 'seed::last_of(&s, seed::Kind::Remove)')
s.sub('seed::last(&s, |want| matches!(want, Want::Enable { .. }))', 'seed::last_of(&s, seed::Kind::Enable)')
s.rx(r'    assert_eq!\(\n        dialog,\n        \[(.*?)\n        \],\n        "plugins_popup_marketplace_remove_confirmation, from the dialog\'s column"\n    \);',
     r'    kit::assert_rows_are(\n        &dialog,\n        &[\1\n        ],\n    );', count=1) if False else None
s.rx(r'    assert_eq!\(\n        seed::wants\(&s\)\n            \.into_iter\(\)\n            \.filter\(\|want\| matches!\(want, Want::Remove \{ \.\. \}\)\)\n            \.collect::<Vec<_>>\(\),\n        vec!\[Want::Remove \{\n            name: "repo"\.to_string\(\)\n        \}\]\n    \);',
     '    seed::assert_removed(&s, &["repo"]);', count=1)
# reads / installs by target
s.rx(r'    let target = Target \{\n        location: Location::Remote \{\n            marketplace: "workspace-directory"\.to_string\(\),\n        \},\n        plugin: "plugins~Plugin_calendar"\.to_string\(\),\n    \};\n    assert!\(\n        seed::wants\(&s\)\.contains\(&Want::Read\(target\)\),\n        "\{:\?\}",\n        seed::wants\(&s\)\n    \);',
     '    seed::assert_read_remote(&s, "workspace-directory", "plugins~Plugin_calendar", 1);', count=1)
s.rx(r'    let target = Target \{\n        location: Location::Remote \{\n            marketplace: "workspace-shared-with-me-private"\.to_string\(\),\n        \},\n        plugin: "plugins~Plugin_linear"\.to_string\(\),\n    \};\n    assert!\(\n        seed::wants\(&s\)\.contains\(&Want::Install\(target\)\),\n        "\{:\?\}",\n        seed::wants\(&s\)\n    \);',
     '    seed::assert_install_remote(&s, "workspace-shared-with-me-private", "plugins~Plugin_linear");', count=1)
s.rx(r'    assert!\(\n        !seed::wants\(&s\)\n            \.iter\(\)\n            \.any\(\|want\| matches!\(want, Want::Install\(_\)\)\)\n    \);', '    seed::assert_no_install(&s);', count=1)
s.rx(r'    assert!\(\n        !seed::wants\(&s\)\n            \.iter\(\)\n            \.any\(\|want\| matches!\(want, Want::Enable \{ \.\. \}\)\)\n    \);', '    seed::assert_no_enable(&s);', count=2)
s.rx(r'    // the detail is asked of the remote marketplace, by the remote id\n    let target = \|\| Target \{\n        location: Location::Remote \{\n            marketplace: market\.to_string\(\),\n        \},\n        plugin: id\.to_string\(\),\n    \};\n    assert!\(\n        seed::wants\(&s\)\.contains\(&Want::Read\(target\(\)\)\),\n        "\{:\?\}",\n        seed::wants\(&s\)\n    \);',
     '    // the detail is asked of the remote marketplace, by the remote id\n    seed::assert_read_remote(&s, market, id, 1);', count=1)
s.rx(r'    let reads = seed::wants\(&s\)\n        \.into_iter\(\)\n        \.filter\(\|want\| \*want == Want::Read\(target\(\)\)\)\n        \.count\(\);\n    assert_eq!\(reads, 2\);',
     '    seed::assert_read_remote(&s, market, id, 2);', count=1)
# enables
s.rx(r'    let enables = \|s: &Screen\| -> Vec<Want> \{\n        seed::wants\(s\)\n            \.into_iter\(\)\n            \.filter\(\|want\| matches!\(want, Want::Enable \{ \.\. \}\)\)\n            \.collect\(\)\n    \};\n    assert_eq!\(\n        enables\(&s\),\n        vec!\[Want::Enable \{\n            id: "plugin-drive"\.to_string\(\),\n            enabled: false\n        \}\]\n    \);',
     '    seed::assert_enables(&s, &[("plugin-drive", false)]);', count=1)
s.rx(r'    assert_eq!\(\n        enables\(&s\),\n        vec!\[\n            Want::Enable \{\n                id: "plugin-drive"\.to_string\(\),\n                enabled: false\n            \},\n            Want::Enable \{\n                id: "plugin-drive"\.to_string\(\),\n                enabled: true\n            \},\n        \]\n    \);',
     '    seed::assert_enables(&s, &[("plugin-drive", false), ("plugin-drive", true)]);', count=1)
s.save()
