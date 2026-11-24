#!/usr/bin/env python3
"""Renames the reason of the kit-seeds pending markers whose cause is a feature or an entry of another package.

    cd <worktree>
    python3 -I tools/rtwk-port/oracle-pending/rename.py --check    # the reasons it would rename, nothing written
    python3 -I tools/rtwk-port/oracle-pending/rename.py            # rewrite tests/oracle/<shard>.rs, then sync_tsv.py + map-full/build.py

`map-full/tests_rules.py` sends a reason that says `no seed` / `no entry ..` to the kit-seeds package: the kit change
has to land before the feature can. These tests are past that: the kit seeds what a script can seed (a script step, a
keymap, a fixture), and what is left is the feature, or the debug entry its slice registers. RULES says which
(shard, a regex over the reason's start) -> the words that replace the match, naming the feature and its package, so
the test goes to that package by its name. Only the first words of a string-literal reason change: the rest of the
reason (what the oracle does, which fixture is missing) is the next porter's brief. Rerunnable: a renamed reason
matches no rule.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
ORACLE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle')

# (shard, regex over the start of the literal, replacement)
RULES = [
    ('usage', r'^no debug entry', 'feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers no entry'),
    ('small_dialogs_1', r'^no app-link dialog', 'feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built'),
    ('small_dialogs_1', r'^no custom prompt', 'feature custom prompt view (w2-approvals, dialogs screen): the view is not built'),
    ('approvals_1', r'^no entry with', 'entry to register (w2-approvals, src/sheets/approvals): the sheet entry with'),
    ('approvals_1', r'^no keymap seed', 'keymap is seeded by `.keymap(..)`; the entry to register (w2-approvals) is missing; was: no keymap seed'),
    ('approvals_1', r'^no seed', 'feature composer history in the question prompt (w2-approvals): was: no seed'),
    ('chat_status_3', r'^no /(?:statusline|title)(?: or /(?:statusline|title))? (?:setup )?popup', 'feature /statusline and /title setup popups (w2-status): the chat has none'),
    ('chat_status_3', r'^the chat has no MCP tool call with an image result', 'feature image result of an MCP tool call (w2-cells): the chat has none'),
    ('chat_popups_4', r'^no (quick dials|skills menu|manage-skills summary|security-setup banner|/feedback flow)', r'feature \1 (w2-approvals, chat popups): none in the port'),
    ('chat_popups_4', r'^no entry with', 'entry to register (w2-commands, feedback consent): the entry with'),
    ('chat_commands_3', r'^(?:the )?(startup draft|startup submission|startup header|pre-session startup draft|startup blossom decoration|terminal-app-over-ssh inline startup)', r'feature \1 (w2-composer, startup): not in the port'),
]
# The rest of a renamed reason says `no entry `x`` / `no seed` / `not seedable`: the words tests_rules.py reads as a
# kit gap. The kit seeds a script, a keymap and a fixture now; what these name is the missing feature or entry.
TAIL = [
    (r'no entry (`[^`]*`)', r'entry \1 missing'),
    (r'no entry (with|for)\b', r'entry missing \1'),
    (r'no debug entry\b', 'debug entry missing'),
    (r'\bnot seedable\b', 'not reachable'),
    (r'\bno keymap seed\b', 'a keymap is read from `.keymap(..)`, and'),
    (r'\bno seed\b', 'no scripted state'),
    (r'\bthe kit cannot\b', 'the screen cannot'),
    (r'\bkit cannot\b', 'screen cannot'),
]
TESTS = os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'unpend', 'kit-seeds.txt')


def main(argv):
    check = '--check' in argv
    names = [l.strip() for l in open(TESTS, encoding='utf-8') if l.strip() and not l.startswith('#')]
    sources = {}
    renamed = 0
    for name in names:
        shard, test = name.split('::', 1)
        rules = [(rx, new) for s, rx, new in RULES if s == shard]
        if not rules:
            continue
        path = os.path.join(ORACLE, shard + '.rs')
        src = sources.get(shard) or open(path, encoding='utf-8').read()
        m = re.search(r'\bfn %s\s*\(' % re.escape(test), src)
        k = m and re.compile(r'kit::pending\(\s*Owner::\w+,\s*"').search(src, m.end())
        if not k:
            continue
        for rx, new in rules:
            hit = re.compile(rx.lstrip('^')).match(src, k.end())
            if hit:
                src = src[:k.end()] + hit.expand(new) + src[hit.end():]
                renamed += 1
                print('rename  ', name)
                break
        # a renamed (or feature-first) reason: its tail loses the words of a kit gap
        if re.compile(r'(?:feature |entry to register)').match(src, k.end()):
            end = k.end()
            while src[end] != '"':
                end += 2 if src[end] == '\\' else 1
            text = src[k.end():end]
            for rx, new in TAIL:
                text = re.sub(rx, new, text)
            src = src[:k.end()] + text + src[end:]
        sources[shard] = src
    print('%d reasons renamed' % renamed)
    if not check:
        for shard, src in sources.items():
            with open(os.path.join(ORACLE, shard + '.rs'), 'w', encoding='utf-8') as f:
                f.write(src)


if __name__ == '__main__':
    main(sys.argv[1:])
