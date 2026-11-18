"""config.toml pieces the scenarios share (see Scenario.config_top / config_tables)."""

# core/tests/suite/rmcp_client.rs `insert_mcp_server`: a stdio server named "rmcp"
# (its tools are the namespace `mcp__rmcp`); the server is mcp_server.py.
MCP_TABLES = """\
[mcp_servers.rmcp]
command = "{python}"
args = ["-I", "{mcp_server}"]
startup_timeout_sec = 20
"""

# Auto-compaction at 200k tokens (compact.rs:2575 `model_auto_compact_token_limit`) and a
# compaction prompt the router can recognise (compact.rs:161 `set_test_compact_prompt`).
COMPACT_PROMPT = "MOCK-COMPACT-PROMPT: write a short summary of the conversation so far."
COMPACT_TOP = f"""\
model_auto_compact_token_limit = 200000
compact_prompt = "{COMPACT_PROMPT}"
"""
