"""Consecutive search and read commands share one exploration row."""

from events import exec_command_call
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="exploration",
    description="Search one file and read two files before replying",
    seed_files={"alpha.txt": "needle\n", "beta.txt": "other text\n"},
    turns=[
        Turn(
            "Search for needle and read both files.",
            [
                tool_response("ex1", exec_command_call("call-explore-1", "rg needle alpha.txt")),
                tool_response("ex2", exec_command_call("call-explore-2", "cat alpha.txt")),
                tool_response("ex3", exec_command_call("call-explore-3", "cat beta.txt")),
                text_response("ex4", ["Done. ", "END-exploration"], pace=0.01),
            ],
        )
    ],
    sentinel="END-exploration",
    suite="history_cell/tests.rs:2017 coalesces_reads_across_multiple_calls",
)
