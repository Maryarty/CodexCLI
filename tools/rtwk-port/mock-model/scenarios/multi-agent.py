"""Multi-agent: the model spawns a sub-agent; the child answers; the parent finishes."""
import json
import os

import harness
from events import ev_function_call_with_namespace
from scenario import Resp, Scenario, Turn

CHILD_PROMPT = "[mock-child] Reply with one short line."
SPAWN_ID = "call-spawn-1"
# The v1 collab tools are namespace multi_agent_v1 (subagent_notifications.rs:108, args
# {message}); the v2 tools are namespace `collaboration` (code_mode_agent_message_tests.rs:62,
# args {task_name, message}). Both current binaries use v2. Use MOCK_AGENT_API=v1 for older builds.
API = os.environ.get("MOCK_AGENT_API", "v2")
if API == "v2":
    SPAWN_CALL = ev_function_call_with_namespace(
        SPAWN_ID, "collaboration", "spawn_agent", json.dumps({"task_name": "helper", "message": CHILD_PROMPT})
    )
else:
    SPAWN_CALL = ev_function_call_with_namespace(SPAWN_ID, "multi_agent_v1", "spawn_agent", json.dumps({"message": CHILD_PROMPT}))


def is_child(req: dict, text: str) -> bool:
    """The child's request ends with the message that started it (it forks the parent's
    history, so it may carry the spawn call id too; the tree's test matches
    `body_contains(CHILD_PROMPT) && !body_contains(SPAWN_CALL_ID)` for v1)."""
    items = req.get("input") or []
    return bool(items) and CHILD_PROMPT in json.dumps(items[-1])


parent_1 = Resp(
    [
        harness.ev_response_created("ma1"),
        harness.ev_assistant_message("msg-ma1", "I will spawn a helper."),
        SPAWN_CALL,
        harness.ev_completed("ma1"),
    ],
    pick=lambda req, text: not is_child(req, text) and SPAWN_ID not in text,
)
child = Resp(
    [
        harness.ev_response_created("ma-child"),
        harness.ev_assistant_message("msg-ma-child", "child done"),
        harness.ev_completed("ma-child"),
    ],
    pick=is_child,
    child=True,
)
WAIT_ID = "call-wait-1"
parent_2 = Resp(
    [
        harness.ev_response_created("ma2"),
        harness.ev_assistant_message("msg-ma2", "Waiting for the helper."),
        ev_function_call_with_namespace(WAIT_ID, "collaboration", "wait_agent", json.dumps({"timeout_ms": 10000})),
        harness.ev_completed("ma2"),
    ],
    pick=lambda req, text: not is_child(req, text) and SPAWN_ID in text and WAIT_ID not in text,
)
parent_3 = Resp(
    [
        harness.ev_response_created("ma3"),
        harness.ev_assistant_message("msg-ma3", "The helper is done. END-multi-agent"),
        harness.ev_completed("ma3"),
    ],
    pick=lambda req, text: not is_child(req, text) and (WAIT_ID if API == "v2" else SPAWN_ID) in text,
)
RESPONSES = [parent_1, child, parent_2, parent_3] if API == "v2" else [parent_1, child, parent_3]
SCENARIO = Scenario(
    name="multi-agent",
    description="spawn_agent: the child request is matched by its prompt, then the parent reports its result",
    turns=[Turn("Spawn a helper and wait for it.", RESPONSES)],
    sentinel="END-multi-agent",
    helpers="ev_function_call_with_namespace (responses.rs:1028), mount_sse_once_match (the child is matched by body)",
    suite="subagent_notifications.rs:440-540 setup_turn_one_with_custom_spawned_child",
    notes="The responses are served by body match (Resp.pick), not in queue order: the child's request races the parent's.",
)
