#!/usr/bin/env python3
"""Writes the scripts of the /mcp inventory tests of cells_1 and cells_4.

    python3 -I tools/rtwk-port/w2-cells/mcp_fixtures.py

Each oracle test builds `McpServerStatus` values by hand and renders `new_mcp_tools_output_from_statuses`;
ours answers the `mcpServerStatus/list` ask of `/mcp` with the same statuses (a `respond` step), in
tests/oracle/fixtures/<shard>/<test>.jsonl.
"""
import json
import os

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
OUT = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle', 'fixtures')


def server(name, runtime='unset', auth='unknown', tools=None, resources=(), templates=()):
    value = {
        'name': name,
        'tools': tools or {},
        'resources': list(resources),
        'resourceTemplates': list(templates),
        'authStatus': auth,
    }
    if runtime != 'unset':
        value['runtimeStatus'] = runtime
    return value


LOOKUP = {'lookup': {'name': 'lookup', 'inputSchema': {'type': 'object', 'properties': {}}}}

CASES = {
    ('cells_1', 'mcp_inventory_connection_states'): [
        server(name, runtime)
        for name, runtime in [
            ('unknown', None),
            ('starting', 'starting'),
            ('failed', 'failed'),
            ('disabled', 'disabled'),
            ('deferred', 'notStarted'),
            ('connected-empty', 'connected'),
            ('cancelled', 'cancelled'),
            ('auth', 'authenticationRequired'),
        ]
    ],
    ('cells_1', 'mcp_inventory_older_app_server_authentication'): [
        server('legacy-auth', auth='notLoggedIn'),
        server('legacy-healthy', auth='oAuth', tools={'lookup': {'name': 'lookup', 'inputSchema': {'type': 'object'}}}),
        server('runtime-disabled', runtime='disabled', auth='notLoggedIn'),
    ],
    ('cells_4', 'mcp_tools_output_from_statuses_renders_status_only_servers'): [
        server('plugin_docs', runtime=None, tools=LOOKUP),
    ],
    ('cells_4', 'mcp_tools_output_from_statuses_renders_verbose_inventory'): [
        server(
            'plugin_docs',
            runtime=None,
            auth='unsupported',
            tools=LOOKUP,
            resources=[{'name': 'docs', 'title': 'Docs', 'uri': 'file:///docs'}],
            templates=[{'uriTemplate': 'file:///docs/{id}', 'name': 'doc-template', 'title': 'Doc Template'}],
        ),
    ],
}

for (shard, test), servers in CASES.items():
    step = {'step': 'respond', 'method': 'mcpServerStatus/list', 'result': {'data': servers, 'nextCursor': None}}
    os.makedirs(os.path.join(OUT, shard), exist_ok=True)
    path = os.path.join(OUT, shard, test + '.jsonl')
    with open(path, 'w', encoding='utf-8') as f:
        f.write(json.dumps(step, separators=(',', ':')) + '\n')
    print('wrote', os.path.relpath(path, ROOT))
