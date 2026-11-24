#!/usr/bin/env python3
"""Sizes the oracle's tests of VS2's scope and cuts them into shards.

    python3 -I tests/oracle/tools/shards.py            # writes what is missing
    python3 -I tests/oracle/tools/shards.py --check    # fails when it is stale

Outputs:
  docs/rtwk-port/test-shards.md        the table of shards and the files left out
  tests/oracle/<shard>.rs              one stub test per screen-level oracle test
A shard file that exists is never overwritten (a porter may have started it);
the table is rewritten. The shard modules are the files of tests/oracle/: the roots' module
lists are written by build.rs of test-mods, so this script no longer edits tests/oracle.rs.

A test is *screen-level* when it asserts an insta snapshot (kind `snap`) or
renders / presses keys (kind `ui`); the rest is `logic` (a state test, left to
the slice that owns the behaviour). The kind is a text heuristic: the porter of
a shard decides test by test (port it, or kit::na with the reason).
"""
import collections
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.normpath(os.path.join(HERE, '..', '..', '..'))      # codex-rs/tui-rtwk
ORACLE = os.path.normpath(os.path.join(CRATE, '..', 'tui', 'src'))
REPO = os.path.normpath(os.path.join(CRATE, '..', '..'))
MAX = 60       # screen-level tests per shard, at most
CHUNK = 55     # a file bigger than MAX is cut into pieces of about this size

TEST_RE = re.compile(r'^\s*#\[(tokio::)?test(\(|\])')
FN_RE = re.compile(r'^\s*(pub(\([a-z]+\))? )?(async )?fn (\w+)')
SNAP_CALL = re.compile(r'assert(_\w+)?_snapshot!|assert_chat_snapshot|insta::')
UI_CALL = re.compile(
    r'render|handle_key|Buffer|draw\(|handle_paste|lines_to_single_string|'
    r'\.display_lines|desired_height|Terminal::')
LITERAL = re.compile(r'"([A-Za-z0-9_]+)"')

# (path regex, screen, owner slice, group). First match wins. The owner is the
# slice that makes the rows visible; it names a pending test.
RULES = [
    # renderers replaced by rtwk-rich (ADR-004): internals are n/a
    (r'^(wrapping|wrapping_reuse_tests|text_formatting|table_detect)\.rs$|^render/', 'S11', 'chat-live', 'renderer-internals'),
    (r'^markdown_stream\.rs$', 'S11', 'chat-live', 'renderer-internals'),
    # chat widget
    (r'^chatwidget/realtime', None, None, None),
    (r'^chatwidget/tests/(exec_flow|approval_requests|guardian|questions_tests|question_notifications_tests|question_turn_end_tests)\.rs$', 'S13', 'dialogs-pane', 'chat-approvals'),
    (r'^chatwidget/tests/(popups_and_settings|permissions|permission_picker_tests|permission_shortcuts_tests|model_picker_tests|model_display_name_tests|review_picker_tests|worktree_picker_tests|copy_export_picker_tests|goal_menu|plugin_catalog_tests|bedrock_catalog_tests|collaboration_catalog_tests|session_model_selection_tests|quick_dials_tests|skills_summary_tests|security_setup_tests)\.rs$', 'S14', 'dialogs-pane', 'chat-popups'),
    (r'^chatwidget/tests/(slash_commands|composer_submission|startup_submission_tests|sparkle_submission_tests)\.rs$|^chatwidget/(slash_command_args_tests|reasoning_shortcuts)\.rs$|^startup_draft', 'S10', 'chat-nav-pane', 'chat-commands'),
    (r'^chatwidget/tests/(status_and_layout|status_command_tests|status_surface_command_tests|status_surface_previews|terminal_title|terminal_program_status|program_status|reasoning_status_tests|subagent_activity_tests|dynamic_activity_tests|tool_activity_tests|computer_activity_tests)\.rs$|^chatwidget/status_state\.rs$|^status_indicator_widget', 'S10', 'ui-pane', 'chat-status'),
    (r'^chatwidget/', 'S10', 'chat-live', 'chat-live'),
    # bottom pane
    (r'^bottom_pane/(approval_overlay|mcp_server_elicitation|request_user_input|async_questions|pending_thread_approvals|questions_tests|user_verification_tests)', 'S13', 'dialogs-pane', 'approvals'),
    (r'^bottom_pane/(list_selection_view|selection_|multi_select_picker|scroll_state|popup_consts|tests/picker_hint)', 'S14', 'ui-pane', 'picker'),
    (r'^bottom_pane/(app_link_view|custom_prompt_view|feedback|hooks_browser|experimental_features|memories_settings|skills_toggle|warnings_view|shortcut_overlay|status_line_setup|title_setup)', 'S23', 'dialogs-pane', 'small-dialogs'),
    (r'^bottom_pane/(command_popup|file_search_popup|skill_popup|mentions_v2|slash_commands|mention_usage|chat_composer/(history_search|mentions_layout|completion_target|popup_state|skill_completion|slash_input))', 'S12', 'chat-nav-pane', 'composer-popups'),
    (r'^bottom_pane/textarea', 'S10', 'chat-live', 'textarea'),
    (r'^bottom_pane/(mod\.rs|view_stack_tests|footer|pending_input_preview|unified_exec_footer|tests/|status_line_style|voice_strip|effort_|chat_composer/(status_surface|footer_state|sparkle|warning_notice)|chat_composer_effort)', 'S10', 'ui-pane', 'pane-footer'),
    (r'^bottom_pane/', 'S10', 'chat-live', 'composer'),
    # cells, markdown, streaming
    (r'^history_cell/|^exec_cell/|^diff_render\.rs$', 'S11', 'chat-live', 'cells'),
    (r'^markdown|^streaming/', 'S11', 'chat-live', 'markdown-streaming'),
    # overlays, routes, cards
    (r'^analytics(/|\.rs$|_tests\.rs$)', 'S20', 'routes-lists', 'usage'),
    (r'^pager_overlay', 'S16', 'routes-lists', 'pager'),
    (r'^transcript_view|^app/owned_transcript', 'S10', 'chat-live', 'transcript-view'),
    (r'^resume_picker', 'S18', 'routes-lists', 'resume'),
    (r'^status/', 'S19', 'dialogs-pane', 'status'),
    # startup page: welcome, update prompt (S01), directory trust (S04)
    (r'^onboarding/(welcome|trust_directory|onboarding_screen)\.rs$|^update_prompt\.rs$', 'S01-S04', 'startup-flow', 'onboarding'),
    (r'^external_agent_config_migration/', 'S06', 'routes-lists', 'import'),
    # startup prompts (S05)
    (r'^(cwd_prompt|model_migration|startup_hooks_review|unarchive_prompt_tests|oss_selection|oss_selection_tests|daemon_recovery_tests)\.rs$', 'S05', 'startup-prompts', 'startup_prompts'),
    # startup page: sign-in (S02) and provider setup (S03)
    (r'^onboarding/(auth(/\w+)?|provider_setup|gateway(_login|_tests)?|bedrock(_tests)?|gcp(_tests)?)\.rs$', 'S02-S03', 'onboarding', 'signin'),
]
RULES = [(re.compile(p), s, o, g) for p, s, o, g in RULES]

# What is left out of VS2 (a rule above with no group, or no rule at all).
OUT = [
    (r'^chatwidget/realtime', 'S25 voice / realtime (a later slice)'),
    (r'^app/', 'App-level: threads, startup, agents overview, app events (session-core, agents-live)'),
    (r'^(app_server_session|lib_tests|app_backtrack)\.rs$', 'App/backend session (session-core); backtrack S17 is logic only'),
    (r'^(keymap|key_hint)', 'Keymap runtime (ADR-006, reuse-carry)'),
    (r'^onboarding/|^update_|^cwd_prompt|^model_migration|^oss_selection|^daemon_|^startup_', 'S01-S05 startup screens (landed)'),
    (r'^pets/', 'S24 pets (landed)'),
    (r'^ui_tree|^inline_visualization', 'S26 ui_tree (landed)'),
    (r'^(tui|custom_terminal|insert_history|terminal_|transcript_reflow)', 'Terminal and inline mode (rtwk owns it, S27)'),
    (r'^(clipboard|external_editor|markdown_copy|ide_context|tooltips|mention_codec|debug_config|streaming_)', 'Carried helper logic (reuse-carry): no screen'),
]
OUT = [(re.compile(p), r) for p, r in OUT]


def scan():
    files = {}
    for root, _, names in os.walk(ORACLE):
        for n in sorted(names):
            if not n.endswith('.rs'):
                continue
            p = os.path.join(root, n)
            rel = os.path.relpath(p, ORACLE)
            lines = open(p, encoding='utf-8').read().split('\n')
            tests, i = [], 0
            while i < len(lines):
                if TEST_RE.match(lines[i]):
                    j = i + 1
                    while j < len(lines) and not FN_RE.match(lines[j]):
                        j += 1
                    if j >= len(lines):
                        break
                    name = FN_RE.match(lines[j]).group(4)
                    depth, started, k, body = 0, False, j, []
                    while k < len(lines):
                        line = lines[k]
                        body.append(line)
                        depth += line.count('{') - line.count('}')
                        if '{' in line:
                            started = True
                        if started and depth <= 0:
                            break
                        k += 1
                    tests.append({'name': name, 'line': j + 1, 'body': '\n'.join(body)})
                    i = k
                i += 1
            if tests:
                files[rel] = tests
    snaps = collections.defaultdict(list)   # tail -> [stems]
    count = 0
    for root, _, names in os.walk(ORACLE):
        for n in names:
            if n.endswith('.snap'):
                count += 1
                stem = n[:-5]
                tail = stem.split('__tests__')[-1] if '__tests__' in stem else stem.split('__')[-1]
                snaps[tail].append(stem)
    return files, snaps, count


def snap_ref(tail, stems):
    """The name the kit's `find` accepts for the snapshot with this tail."""
    if len(stems) == 1:
        return tail if '__tests__' + tail in stems[0] else stems[0]
    return None


def classify(test, snaps):
    body = test['body']
    tokens = set(LITERAL.findall(body)) | {test['name']}
    matched = sorted(t for t in tokens if t in snaps)
    if matched or SNAP_CALL.search(body):
        kind = 'snap'
    elif UI_CALL.search(body):
        kind = 'ui'
    else:
        kind = 'logic'
    test['kind'] = kind
    test['snaps'] = matched


def rule_for(rel):
    for rx, screen, owner, group in RULES:
        if rx.search(rel):
            return (screen, owner, group) if group else None
    return None


def out_reason(rel):
    for rx, reason in OUT:
        if rx.search(rel):
            return reason
    return 'not a VS2 file'


def main(check):
    files, snaps, nsnaps = scan()
    groups = collections.OrderedDict()
    core_only = []
    out = collections.Counter()
    out_tests = collections.Counter()
    for rel in sorted(files):
        for t in files[rel]:
            classify(t, snaps)
        rule = rule_for(rel)
        if rule is None:
            top = rel.split('/')[0]
            key = (top, out_reason(rel))
            out[key] += 1
            out_tests[key] += len(files[rel])
            continue
        screen, owner, group = rule
        cands = [t for t in files[rel] if t['kind'] != 'logic']
        logic = len(files[rel]) - len(cands)
        if not cands:
            why = ('renderer internals, replaced by rtwk-rich (ADR-004)'
                   if group == 'renderer-internals' else 'logic only: state tests')
            core_only.append((rel, len(files[rel]), screen, owner, why))
            continue
        groups.setdefault(group, []).append((rel, screen, owner, cands, logic))
    shards = []
    order = []
    for _rx, _s, _o, g in RULES:
        if g and g not in order:
            order.append(g)
    for group, entries in sorted(groups.items(), key=lambda kv: order.index(kv[0])):
        pieces = []
        for rel, screen, owner, cands, logic in entries:
            if len(cands) > MAX:
                n = -(-len(cands) // CHUNK)
                size = -(-len(cands) // n)
                for k in range(n):
                    part = cands[k * size:(k + 1) * size]
                    pieces.append((rel, screen, owner, part, round(logic * len(part) / len(cands))))
            else:
                pieces.append((rel, screen, owner, cands, logic))
        packs, cur, count = [], [], 0
        for piece in pieces:
            if cur and count + len(piece[3]) > MAX:
                packs.append(cur)
                cur, count = [], 0
            cur.append(piece)
            count += len(piece[3])
        if cur:
            packs.append(cur)
        # a small tail joins the shard before it (up to 75 tests)
        if len(packs) > 1 and sum(len(p[3]) for p in packs[-1]) < 20 and \
                sum(len(p[3]) for q in packs[-2:] for p in q) <= 75:
            packs[-2].extend(packs.pop())
        for k, pack in enumerate(packs, 1):
            sid = group.replace('-', '_') + ('_%d' % k if len(packs) > 1 else '')
            shards.append({'id': sid, 'group': group, 'pieces': pack})
    render(shards, core_only, out, out_tests, files, snaps, nsnaps, check)


def unique_names(shard):
    seen = collections.Counter()
    for _rel, _s, _o, tests, _l in shard['pieces']:
        for t in tests:
            seen[t['name']] += 1
    names, used = {}, set()
    for rel, _s, _o, tests, _l in shard['pieces']:
        stem = os.path.basename(os.path.splitext(rel)[0])
        for t in tests:
            name = t['name']
            if seen[name] > 1 or name in used:
                name = '%s_%s' % (stem, t['name'])
            while name in used:
                name += '_'
            used.add(name)
            names[(rel, t['line'])] = name
    return names


def shard_text(shard, names, snaps):
    owners = sorted({p[2] for p in shard['pieces']})
    screens = sorted({p[1] for p in shard['pieces']})
    out = [
        '//! Shard `%s` (screens %s, owner %s): the oracle tests of the manifest below,' % (
            shard['id'], '/'.join(screens), '/'.join(owners)),
        '//! ported as black-box screen tests with the kit. One test per manifest line,',
        '//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.',
        '//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by',
        '//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).',
        '',
        'use crate::kit;',
        '',
    ]
    for rel, _screen, owner, tests, logic in shard['pieces']:
        out.append('// ---- oracle: %s (%d screen-level tests here; %d state tests left to %s)' % (
            rel, len(tests), logic, owner))
        for t in tests:
            refs = []
            for tail in t['snaps']:
                r = snap_ref(tail, snaps[tail])
                refs.append('snap=%s' % (r if r else '%s(several:%s)' % (tail, '|'.join(snaps[tail]))))
            line = '// ORACLE %s:%d %s kind=%s owner=%s %s' % (
                rel, t['line'], names[(rel, t['line'])], t['kind'], owner, ' '.join(refs))
            out.extend([line.rstrip(), '#[test]', 'fn %s() {' % names[(rel, t['line'])],
                        '    kit::todo();', '}', ''])
    return '\n'.join(out).rstrip() + '\n'


def render(shards, core_only, out, out_tests, files, snaps, nsnaps, check):
    results = {}
    rows = []
    total_c = total_l = total_s = 0
    for sh in shards:
        names = unique_names(sh)
        results[os.path.join(CRATE, 'tests/oracle/%s.rs' % sh['id'])] = shard_text(sh, names, snaps)
        cands = sum(len(p[3]) for p in sh['pieces'])
        logic = sum(p[4] for p in sh['pieces'])
        nsn = len({s for p in sh['pieces'] for t in p[3] for s in t['snaps']})
        total_c += cands
        total_l += logic
        total_s += nsn
        parts = []
        for p in sh['pieces']:
            whole = len([t for t in files[p[0]] if t['kind'] != 'logic']) == len(p[3])
            parts.append('`%s`%s' % (p[0], '' if whole else ' (`%s` .. `%s`)' % (p[3][0]['name'], p[3][-1]['name'])))
        rows.append((sh['id'], '/'.join(sorted({p[1] for p in sh['pieces']})),
                     '/'.join(sorted({p[2] for p in sh['pieces']})), cands, logic, nsn, '; '.join(parts)))
    md = ['# Oracle test shards (VS2: the working chat)', '']
    md.append('Generated by `codex-rs/tui-rtwk/tests/oracle/tools/shards.py` from the oracle '
              '(`codex-rs/tui` at `docs/rtwk-port/ORACLE_REF`); rerun the script, do not edit by hand. '
              'The recipe a porter follows: `docs/rtwk-port/refs/screen-tests.md`. The kit: '
              '`codex-rs/tui-rtwk/tests/kit/`. The suite: `tests/oracle.rs`, run with '
              '`python3 -I tests/oracle/tools/summary.py --run <shard>` (nextest, own junit, a line per shard).')
    md.append('')
    md.append('**Totals.** %d shards, %d screen-level oracle tests to port (asserting %d snapshots of the '
              'oracle\'s %d), %d state tests of the same files left to the owning slices, %d files with '
              'nothing visible (core-only, below).' % (len(shards), total_c, total_s, nsnaps, total_l, len(core_only)))
    md.append('')
    md.append('Columns: `tests` = screen-level tests of the shard (one stub each in the shard file); '
              '`state` = logic tests of the same oracle files, not ported by the swarm (the slice porter '
              'ports them as state tests); `snaps` = oracle snapshots the shard\'s tests assert. The kind '
              '(snapshot / render-or-keys / logic) is a text heuristic: the shard\'s porter decides per '
              'test, port it or `kit::na(owner, reason)`. Slice = the owner of the rows the shard shows '
              '(it names the pending tests): ui-pane, routes-lists, dialogs-pane, chat-nav-pane, chat-live '
              '(tm `vs2` children). Screens: architecture.md section 1.')
    md.append('')
    md.append('## Shards')
    md.append('')
    md.append('| shard | screen | slice | tests | state | snaps | target file | oracle files |')
    md.append('|---|---|---|---:|---:|---:|---|---|')
    for r in rows:
        md.append('| %s | %s | %s | %d | %d | %d | `tests/oracle/%s.rs` | %s |' % (r[0], r[1], r[2], r[3], r[4], r[5], r[0], r[6]))
    md.append('')
    md.append('## By slice')
    md.append('')
    by = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        for o in r[2].split('/'):
            by[o][0] += 1
            by[o][1] += r[3]
    md.append('| slice | shards | tests |')
    md.append('|---|---:|---:|')
    for o in sorted(by):
        md.append('| %s | %d | %d |' % (o, by[o][0], by[o][1]))
    md.append('')
    md.append('## Core-only: nothing visible (left to the slice porters)')
    md.append('')
    md.append('Every test of these files is logic (no snapshot, render or key press by the heuristic). '
              'The slice named ports them as state tests; renderer internals are n/a (ADR-004).')
    md.append('')
    md.append('| oracle file | tests | screen | slice | why |')
    md.append('|---|---:|---|---|---|')
    for rel, n, screen, owner, why in sorted(core_only):
        md.append('| `%s` | %d | %s | %s | %s |' % (rel, n, screen, owner, why))
    md.append('')
    md.append('## Not VS2: left out of the shards')
    md.append('')
    md.append('| top-level | oracle files | tests | why |')
    md.append('|---|---:|---:|---|')
    for key, n in sorted(out.items(), key=lambda kv: -out_tests[kv[0]]):
        md.append('| `%s` | %d | %d | %s |' % (key[0], n, out_tests[key], key[1]))
    md.append('')
    results[os.path.join(REPO, 'docs/rtwk-port/test-shards.md')] = '\n'.join(md) + '\n'
    stale = []
    for path, content in results.items():
        is_shard = '/tests/oracle/' in path and path.endswith('.rs')
        if os.path.exists(path) and (is_shard or open(path).read() == content):
            continue
        stale.append(path)
        if not check:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'w') as f:
                f.write(content)
    print('%d shards, %d tests, %d snaps, %d state tests, %d core-only files; %d files %s' % (
        len(shards), total_c, total_s, total_l, len(core_only), len(stale), 'stale' if check else 'written'))
    if check and stale:
        for s in stale:
            print('stale:', os.path.relpath(s, REPO))
        sys.exit(1)


if __name__ == '__main__':
    main('--check' in sys.argv)
