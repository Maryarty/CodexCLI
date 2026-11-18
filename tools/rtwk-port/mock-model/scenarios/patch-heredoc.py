"""apply_patch typed into exec_command with a heredoc; core intercepts it as a patch."""
from events import ev_apply_patch_exec_command_call_via_heredoc
from scenario import Scenario, Turn, text_response, tool_response

TODO = "alpha\nbeta\ngamma\n"
PATCH = (
    "*** Begin Patch\n"
    "*** Update File: todo.txt\n"
    "@@\n"
    " alpha\n"
    "-beta\n"
    "+beta (heredoc patch)\n"
    " gamma\n"
    "*** End Patch"
)
SCENARIO = Scenario(
    name="patch-heredoc",
    description="apply_patch via exec_command + heredoc (update todo.txt); on-request, read-only",
    turns=[
        Turn(
            "Patch todo.txt through the shell.",
            [
                tool_response("ph1", ev_apply_patch_exec_command_call_via_heredoc("call-ph-1", PATCH), lead_text="Patching through the shell."),
                text_response("ph2", ["Done. ", "END-patch-heredoc"], pace=0.01),
            ],
        )
    ],
    sentinel="END-patch-heredoc",
    approval_policy="on-request",
    sandbox_mode="read-only",
    approvals=1,
    seed_files={"todo.txt": TODO},
    helpers="ev_apply_patch_exec_command_call_via_heredoc (responses.rs:1113)",
    suite="unified_exec.rs:630 unified_exec_intercepts_apply_patch_exec_command",
)
