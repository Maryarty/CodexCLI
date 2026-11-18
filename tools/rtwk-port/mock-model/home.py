"""Writes a temporary CODEX_HOME and workdir for a scenario. Never touches ~/.codex.

The config is the recipe of codex-rs/tui/tests/suite/resize_reflow.rs
(`write_config`, `write_auth`) plus the scenario's approval policy and sandbox;
the provider block is the one of sdk/python/tests/app_server_harness.py
(`AppServerHarness._write_config`: wire_api = "responses", request_max_retries = 0).

    python3 -I home.py <scenario> <mock-url> <dir>     # prints the two paths
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import scenario as scenarios  # noqa: E402


def write_home(sc: scenarios.Scenario, mock_url: str, root: Path) -> tuple[Path, Path]:
    """Creates `root/codex-home` and `root/work`; returns (codex_home, workdir)."""
    codex_home = root / "codex-home"
    workdir = root / "work"
    codex_home.mkdir(parents=True, exist_ok=True)
    workdir.mkdir(parents=True, exist_ok=True)
    real_work = os.path.realpath(workdir)  # trust key: macOS /var is /private/var
    for rel, content in sc.seed_files.items():
        target = workdir / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            target.write_bytes(content)
        else:
            target.write_text(content)
    mcp_server = str(Path(__file__).resolve().parent / "mcp_server.py")

    def fill(text: str) -> str:
        return text.replace("{python}", sys.executable).replace("{mcp_server}", mcp_server)

    (codex_home / "config.toml").write_text(
        f"""model = "gpt-6-sol"
model_provider = "mock_provider"
approval_policy = "{sc.approval_policy}"
sandbox_mode = "{sc.sandbox_mode}"
suppress_unstable_features_warning = true
check_for_update_on_startup = false
{fill(sc.config_top)}

[analytics]
enabled = false

[model_providers.mock_provider]
name = "Mock provider (tools/rtwk-port/mock-model)"
base_url = "{mock_url}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = {sc.stream_max_retries}

[notice.model_migrations]
"gpt-5.4" = "gpt-6-sol"

[projects."{real_work}"]
trust_level = "trusted"

{fill(sc.config_tables)}
"""
    )
    (codex_home / "auth.json").write_text(
        '{"OPENAI_API_KEY":"dummy","tokens":null,"last_refresh":null}'
    )
    return codex_home, Path(real_work)


def temp_root() -> Path:
    return Path(tempfile.mkdtemp(prefix="rtwk-mock-model-"))


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("usage: home.py <scenario> <mock-url> <dir>")
    sc = scenarios.load(sys.argv[1])
    home, work = write_home(sc, sys.argv[2], Path(sys.argv[3]))
    print(home)
    print(work)
