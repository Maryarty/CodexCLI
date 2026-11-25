#!/usr/bin/env python3
"""Un-pends the oracle tests of f-settings-lists (or the `shard::test` names given): the
kit::pending marker leaves the test, the row leaves pending-tests.tsv, map-full is regenerated.

    python3 -I tools/rtwk-port/f-settings-lists/unpend.py [shard::test ...]
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'unpend-stale'))
import unpend as u  # noqa: E402

TESTS = '''chat_commands_3::slash_export_opens_destination_picker
chat_commands_3::slash_keymap_debug_can_inspect_app_shortcuts
chat_commands_3::slash_keymap_debug_opens_keypress_inspector
chat_commands_3::slash_memories_opens_memory_menu
chat_popups_3::hooks_popup_shows_list_diagnostics
chat_popups_4::experimental_features_popup_snapshot
chat_popups_4::experimental_features_toggle_saves_on_exit
chat_popups_4::experimental_popup_available_snapshot
chat_popups_4::experimental_popup_loading_snapshot
chat_popups_4::feature_enable_prompts_snapshot
chat_popups_4::manage_skills_summary_snapshot
chat_popups_4::skills_menu_default_mentions_shortcut_snapshot
small_dialogs_2::footer_hint_uses_list_keymap_accept_and_cancel'''.split()


def main():
    names = sys.argv[1:] or TESTS
    done = []
    for name in names:
        shard, test = name.split('::')
        f = os.path.join(u.CRATE, 'tests', 'oracle', shard + '.rs')
        src = open(f, encoding='utf-8').read()
        new = u.remove_marker(src, test)
        if new is None:
            print('no marker: ' + name)
            continue
        open(f, 'w', encoding='utf-8').write(u.tidy_consts(u.tidy_imports(new)))
        done.append(name)
    lines = open(u.TSV, encoding='utf-8').read().split('\n')
    open(u.TSV, 'w', encoding='utf-8').write('\n'.join(l for l in lines if l.split('\t')[0] not in set(done)))
    subprocess.run([sys.executable, '-I', os.path.join(u.ROOT, 'tools', 'rtwk-port', 'map-full', 'build.py')], check=True)
    print('un-pended: %d' % len(done))


if __name__ == '__main__':
    main()
