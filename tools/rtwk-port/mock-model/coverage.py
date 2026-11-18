"""Prints the coverage tables of docs/rtwk-port/parity/mock-model.md.

    python3 -I coverage.py [--results FILE ...]

`--results` takes the JSON files `check.py --json` writes (one per run; a result is keyed
by binary label and scenario). Without them the "checked" columns say "not run".
Table 1: the Rust builders of core/tests/common/responses.rs and the scenarios that use
them (a builder no scenario uses is shown as such). Table 2: the response types, one row
per scenario: Rust helpers, the core suite test it mirrors, whether the installed codex
accepted it (check.py) and whether codex-rtwk served it.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import scenario as scenarios  # noqa: E402

HERE = Path(__file__).resolve().parent
RESPONSES_RS = HERE.parents[2] / "codex-rs/core/tests/common/responses.rs"

# Rust builder -> the Python name it is written under (events.py or the harness).
BUILDERS = [
    ("ev_response_created", "ev_response_created"),
    ("ev_completed", "ev_completed"),
    ("ev_completed_with_tokens", "ev_completed_with_tokens"),
    ("ev_model_verification_metadata", "ev_model_verification_metadata"),
    ("ev_assistant_message", "ev_assistant_message"),
    ("ev_message_item_added", "ev_message_item_added"),
    ("ev_output_text_delta", "ev_output_text_delta"),
    ("ev_reasoning_item", "ev_reasoning_item("),
    ("ev_reasoning_item_added", "ev_reasoning_item_added"),
    ("ev_reasoning_summary_text_delta", "ev_reasoning_summary_text_delta"),
    ("ev_reasoning_text_delta", "ev_reasoning_text_delta"),
    ("ev_web_search_call_added_partial", "ev_web_search_call_added_partial"),
    ("ev_web_search_call_done", "ev_web_search_call_done"),
    ("ev_image_generation_call", "ev_image_generation_call"),
    ("ev_function_call", "ev_function_call("),
    ("ev_function_call_with_namespace", "ev_function_call_with_namespace"),
    ("ev_tool_search_call", "ev_tool_search_call"),
    ("ev_custom_tool_call", "ev_custom_tool_call("),
    ("ev_custom_tool_call_with_namespace", "ev_custom_tool_call_with_namespace"),
    ("ev_apply_patch_custom_tool_call", "ev_apply_patch_custom_tool_call"),
    ("ev_exec_command_call", "exec_command_call("),
    ("ev_exec_command_call_with_args", "yield_time_ms="),  # exec_command_call with arguments
    ("ev_apply_patch_exec_command_call_via_heredoc", "ev_apply_patch_exec_command_call_via_heredoc"),
]
EXTRA_HELPERS = [("sse_failed", "ev_failed_with_code")]


def rust_builders() -> list[str]:
    if not RESPONSES_RS.is_file():
        return []
    return re.findall(r"^pub fn (ev_\w+)\(", RESPONSES_RS.read_text(), re.M)


def users(needle: str) -> list[str]:
    out = []
    for name in scenarios.names():
        text = (scenarios.SCENARIO_DIR / f"{name}.py").read_text()
        if needle in text:
            out.append(name)
    return out


def table_builders() -> str:
    rows = ["| Rust builder (responses.rs) | Python | scenarios |", "|---|---|---|"]
    known = {b for b, _ in BUILDERS}
    for rust, needle in BUILDERS:
        used = users(needle)
        # the harness/events builders may be used through scenario.py's text_response
        if rust in ("ev_response_created", "ev_completed", "ev_assistant_message", "ev_message_item_added", "ev_output_text_delta"):
            used = ["every scenario (through `text_response`)"]
        rows.append(f"| `{rust}` | `{needle.rstrip('(')}` | {', '.join(used) if used else '**none**'} |")
    for rust, needle in EXTRA_HELPERS:
        rows.append(f"| `{rust}` (not an `ev_*`) | `{needle}` | {', '.join(users(needle))} |")
    missing = [b for b in rust_builders() if b not in known]
    if missing:
        rows.append(f"| **not mapped** | {', '.join(missing)} | |")
    return "\n".join(rows)


def load_results(paths: list[str]) -> dict[tuple[str, str], dict]:
    out: dict[tuple[str, str], dict] = {}
    for p in paths:
        for r in json.loads(Path(p).read_text()):
            out[(r["binary"], r["scenario"])] = r
    return out


def cell(results: dict, label: str, name: str, *, pass_word: str = "yes") -> str:
    r = results.get((label, name))
    if r is None:
        return "not run"
    if r["status"] == "PASS":
        return pass_word
    return "no: " + r["detail"][:90].replace("|", "/")


def table_scenarios(results: dict) -> str:
    rows = [
        "| scenario | response type shown | Rust helper | core suite test it mirrors | installed codex 0.167 (check.py) | codex-rtwk served it | codex-rtwk renders the type |",
        "|---|---|---|---|---|---|---|",
    ]
    for name in scenarios.names():
        sc = scenarios.load(name)
        mode = results.get(("oracle", name), {}).get("mode", "")
        oracle = cell(results, "oracle", name)
        if oracle == "yes" and mode:
            oracle = f"yes ({mode})"
        rows.append(
            f"| `{name}` | {sc.description.replace('|', '/')} | {sc.helpers.replace('|', '/')} | {sc.suite.replace('|', '/')} | {oracle} | "
            f"{cell(results, 'rtwk', name)} | n.a. until its slice |"
        )
    return "\n".join(rows)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--results", action="append", default=[])
    args = ap.parse_args()
    results = load_results(args.results)
    print(table_builders())
    print()
    print(table_scenarios(results))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
