#!/usr/bin/env python3
"""Writes the fixtures of the `/ps` snapshot tests of cells_3 and un-pends them.

    python3 -I tools/rtwk-port/w2-cells/ps_fixtures.py

The oracle builds `new_unified_exec_processes_output(Vec<UnifiedExecProcessDetails>)` by hand
(history_cell/tests.rs:751..1022); here the thread is told of the same processes: one `item/started` of a
unified-exec command per process and one output delta per recent chunk (the thread keeps the last three
non-empty lines). Each test's fixture is tests/oracle/fixtures/cells_3/<test>.jsonl; its body opens that script,
types `/ps` and compares the screen from row 0 with the oracle's snapshot (the shard keeps its size). A test the
script already drives is left alone, and the `pending-tests.tsv` rows go (`unpend.py`).
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
CRATE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk')
SHARD = os.path.join(CRATE, 'tests', 'oracle', 'cells_3.rs')
FIXTURES = os.path.join(CRATE, 'tests', 'oracle', 'fixtures', 'cells_3')

HALFWIDTH = 'ｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟ'
# (test, [(command, [recent chunks])])
TESTS = [
    ('ps_output_multiline_snapshot', [
        ('echo hello\nand then some extra text', ['hello', 'done']),
        ('(\n  sleep 120\n)', []),
        ('sleep 1\r\nsleep 120', []),
        ('rg "foo" src', ['src/main.rs:12:foo']),
    ]),
    ('ps_output_multiline_long_command_snapshot', [('(\n  %s\n)' % ('x' * 100), [])]),
    ('ps_output_long_command_snapshot', [(
        'rg "foo" src --glob \'**/*.rs\' --max-count 1000 --no-ignore --hidden --follow --glob \'!target/**\'',
        ['searching...'])]),
    ('ps_output_halfwidth_sound_marks_snapshot', [('echo ' + HALFWIDTH, ['output ' + HALFWIDTH])]),
    ('ps_output_many_sessions_snapshot', [('command %d' % i, []) for i in range(20)]),
    ('ps_output_chunk_leading_whitespace_snapshot',
     [('just fix', ['  indented first', '    more indented'])]),
]


def step(method, params):
    return json.dumps({'step': 'notify', 'message': {'method': method, 'params': params}},
                      ensure_ascii=False)


def script(test, processes):
    lines = ['# oracle history_cell/tests.rs %s: the processes of `new_unified_exec_processes_output`' % test,
             json.dumps({'step': 'seed', 'seed': 'session', 'thread': 'thread-1'})]
    for index, (command, chunks) in enumerate(processes, 1):
        call = 'call-%d' % index
        lines.append(step('item/started', {
            'item': {'type': 'commandExecution', 'id': call, 'pluginId': None, 'scriptPath': None,
                     'command': command, 'cwd': '/tmp/project', 'processId': 'process-%d' % index,
                     'source': 'unifiedExecStartup', 'status': 'inProgress', 'commandActions': [],
                     'aggregatedOutput': None, 'exitCode': None, 'durationMs': None,
                     'startedAtMs': 0, 'completedAtMs': None},
            'threadId': 'thread-1', 'turnId': 'turn-1', 'startedAtMs': 0}))
        for chunk in chunks:
            lines.append(step('item/commandExecution/outputDelta', {
                'threadId': 'thread-1', 'turnId': 'turn-1', 'itemId': call, 'delta': chunk + '\n'}))
    return '\n'.join(lines) + '\n'


# a terminal that clusters nothing: rtwk's own text engine takes a halfwidth katakana and its sound mark as
# one cell, the oracle counts the mark too (G44, closed under `TextMode::Legacy`)
LEGACY = {'ps_output_halfwidth_sound_marks_snapshot'}
# rows the screen has at least: the cells of the started processes stand above the `/ps` cell
MIN_ROWS = 24


def body(src, test):
    """The test's screen opened on its script (and the legacy text engine where G44 needs it)."""
    pattern = re.compile(
        r'(fn %s\(\) \{\n)(?:    kit::pending\(\n.*?\n    \);\n)?'
        r'    let mut s = Screen::build\(Owner::ChatLive\)(?:\.size\((\d+), (\d+)\)\.live\(\)\.open\(\);'
        r'|\s*\.size\((\d+), (\d+)\)\s*\.script\(include_str!\(\s*"[^"]*"\s*\)\)\s*'
        r'(?:\.legacy_text\(\)\s*)?\.open\(\);)\n' % test, re.S)
    match = pattern.search(src)
    if not match:
        return src, False
    width = match.group(2) or match.group(4)
    height = max(int(match.group(3) or match.group(5)), MIN_ROWS)
    legacy = '        .legacy_text()\n' if test in LEGACY else ''
    new = ('%s    let mut s = Screen::build(Owner::ChatLive)\n        .size(%s, %d)\n'
           '        .script(include_str!("fixtures/cells_3/%s.jsonl"))\n%s        .open();\n'
           % (match.group(1), width, height, test, legacy))
    return src[:match.start()] + new + src[match.end():], True


HELPER = """/// The screen row of the `/ps` cell's first line: the processes the fixtures start leave their own
/// cells above it (the oracle builds the cell alone), so the snapshot is compared from there.
fn ps_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|row| row.trim_end() == "/ps")
        .unwrap_or_else(|| panic!("the /ps cell is not on the screen:\\n{}", s.text()))
}

"""


def at_ps_row(src, test):
    """The snapshot compared from the cell's row, not from the screen's top."""
    old = '"%s", 0, 0);' % test
    new = '"%s", ps_row(&s), 0);' % test
    return src.replace(old, new)


def main():
    os.makedirs(FIXTURES, exist_ok=True)
    src = open(SHARD, encoding='utf-8').read()
    done = []
    for test, processes in TESTS:
        open(os.path.join(FIXTURES, test + '.jsonl'), 'w', encoding='utf-8').write(script(test, processes))
        src, changed = body(src, test)
        print('script', test, 'test rewritten' if changed else 'test left alone')
        done.append('cells_3::' + test)
    for test in ['ps_output_empty_snapshot'] + [name for name, _ in TESTS]:
        src = at_ps_row(src, test)
    marker = '// ORACLE history_cell/tests.rs:751 ps_output_empty_snapshot'
    if 'fn ps_row' not in src:
        src = src.replace(marker, HELPER + marker, 1)
    open(SHARD, 'w', encoding='utf-8').write(src)
    rows = open(os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'pending-tests.tsv'),
                encoding='utf-8').read().splitlines(keepends=True)
    keep = [row for row in rows if row.split('\t')[0] not in set(done)]
    open(os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'pending-tests.tsv'), 'w',
         encoding='utf-8').write(''.join(keep))


if __name__ == '__main__':
    if len(sys.argv) != 1:
        sys.exit(__doc__)
    main()
