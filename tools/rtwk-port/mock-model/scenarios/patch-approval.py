"""An apply_patch with a diff (one file updated, one added) that needs approval."""
from events import ev_apply_patch_custom_tool_call
from scenario import Scenario, Turn, text_response, tool_response

NOTES = "alpha\nbeta\ngamma\n"
PATCH = (
    "*** Begin Patch\n"
    "*** Update File: notes.txt\n"
    "@@\n"
    " alpha\n"
    "-beta\n"
    "+beta (patched by the mock model)\n"
    "+an extra line\n"
    " gamma\n"
    "*** Add File: added-by-mock.txt\n"
    "+hello from the mock model\n"
    "+second line\n"
    "*** End Patch\n"
)
SCENARIO = Scenario(
    name="patch-approval",
    description="one freeform apply_patch call (update notes.txt, add added-by-mock.txt); on-request, read-only",
    turns=[
        Turn(
            "Patch notes.txt and add a file.",
            [
                tool_response("pa1", ev_apply_patch_custom_tool_call("call-patch-1", PATCH), lead_text="Applying the patch."),
                text_response("pa2", ["The patch step is over. ", "END-patch-approval"], pace=0.01),
            ],
        )
    ],
    sentinel="END-patch-approval",
    approval_policy="on-request",
    sandbox_mode="read-only",
    approvals=1,
    seed_files={"notes.txt": NOTES},
    notes="notes.txt is seeded in the workdir so the Update hunk has a file to change.",
)
