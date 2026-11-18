"""The plan tool: update_plan twice (in progress, then completed), then the answer."""
import json

import harness
from events import ev_reasoning_item
from scenario import Scenario, Turn, Resp, text_response, tool_response


def plan(*steps: tuple[str, str], explanation: str | None = None) -> str:
    args: dict = {"plan": [{"step": s, "status": st} for s, st in steps]}
    if explanation:
        args["explanation"] = explanation
    return json.dumps(args)


STEPS = ["Inspect the workspace", "Write the change", "Run the checks"]
FIRST = plan((STEPS[0], "in_progress"), (STEPS[1], "pending"), (STEPS[2], "pending"), explanation="Three small steps")
SECOND = plan((STEPS[0], "completed"), (STEPS[1], "in_progress"), (STEPS[2], "pending"))
THIRD = plan((STEPS[0], "completed"), (STEPS[1], "completed"), (STEPS[2], "completed"))
SCENARIO = Scenario(
    name="plan-tool",
    description="update_plan three times (steps move pending -> in progress -> completed), then an answer",
    turns=[
        Turn(
            "Make a plan and work through it.",
            [
                Resp(
                    [
                        harness.ev_response_created("pl1"),
                        ev_reasoning_item("pl1-think", ["inspect the workspace"], []),
                        harness.ev_function_call("call-plan-1", "update_plan", FIRST),
                        harness.ev_completed("pl1"),
                    ]
                ),
                tool_response("pl2", harness.ev_function_call("call-plan-2", "update_plan", SECOND), lead_text="First step done."),
                tool_response("pl3", harness.ev_function_call("call-plan-3", "update_plan", THIRD), lead_text="All steps done."),
                text_response("pl4", ["The plan is complete. ", "END-plan-tool"], pace=0.01),
            ],
        )
    ],
    sentinel="END-plan-tool",
    config_tables="[tools.update_plan]\nenabled = true\n",
    helpers="ev_function_call (name update_plan), ev_reasoning_item",
    suite="tool_lifecycle.rs:134 (update_plan x2 with a reasoning item), items.rs:570",
)
