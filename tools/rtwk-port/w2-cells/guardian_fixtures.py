"""Hand-written fixtures of the guardian tests of `chat_approvals_2` that the recorder left partial.

The oracle's tests call `on_guardian_assessment`, `on_task_started` and `on_mcp_server_status_updated` on the
widget directly; the recorded scripts carry only the turn notifications. This writes the notifications those calls
stand for, as the denied test's recorded script shapes them (`item/autoApprovalReview/*`, `guardianWarning`).

    python3 -I tools/rtwk-port/w2-cells/guardian_fixtures.py
"""

import json
import pathlib

OUT = pathlib.Path(__file__).resolve().parents[3] / "codex-rs/tui-rtwk/tests/oracle/fixtures/chat_approvals_2"


def note(method, params):
    return json.dumps({"step": "notify", "message": {"method": method, "params": params}}, separators=(",", ":"))


def turn(method, turn_id, status):
    return note(
        method,
        {
            "threadId": "thread-1",
            "turn": {
                "id": turn_id,
                "rootTurnId": None,
                "items": [],
                "itemsView": "full",
                "status": status,
                "error": None,
                "startedAt": None,
                "completedAt": None,
                "durationMs": None,
            },
        },
    )


def action(command):
    return {"type": "command", "source": "shell", "command": command, "cwd": "/tmp"}


def permissions(reason):
    return {"type": "requestPermissions", "reason": reason, "permissions": {}}


def review(kind, review_id, turn_id, command, status, risk=None, auth=None, rationale=None):
    """`command` is a shell command, or a ready action object."""
    params = {
        "threadId": "thread-1",
        "turnId": turn_id,
        "startedAtMs": 0,
        "reviewId": review_id,
        "targetItemId": review_id + "-target",
        "review": {"status": status, "riskLevel": risk, "userAuthorization": auth, "rationale": rationale},
        "action": command if isinstance(command, dict) else action(command),
    }
    if kind == "completed":
        params["completedAtMs"] = 1
        params["decisionSource"] = "agent"
    return note(f"item/autoApprovalReview/{kind}", params)


def mcp(name, status):
    return note(
        "mcpServer/startupStatus/updated",
        {"threadId": "thread-1", "name": name, "status": status, "error": None, "failureReason": None},
    )


SEED = json.dumps({"step": "seed", "seed": "session", "thread": "thread-1"})


def write(name, origin, lines, seed=True):
    """A script of its own (seeded session) or a fragment for `Screen::feed` (`seed=False`)."""
    body = f"# oracle chatwidget/tests/guardian.rs:{origin} {name} (hand-written: guardian_fixtures.py)\n"
    body += "\n".join([SEED, *lines] if seed else lines) + "\n"
    (OUT / f"{name}.jsonl").write_text(body)
    print("wrote", name)


def parallel(turn_id):
    return [
        review("started", "guardian-1", turn_id, "rm -rf '/tmp/guardian target 1'", "inProgress"),
        review("started", "guardian-2", turn_id, "rm -rf '/tmp/guardian target 2'", "inProgress"),
    ]


write(
    "guardian_parallel_reviews_render_aggregate_status_snapshot",
    744,
    [turn("turn/started", "turn-1", "inProgress"), *parallel("turn-1")],
)
write(
    "guardian_approved_exec_is_hidden_from_history",
    328,
    [
        note(
            "guardianWarning",
            {
                "threadId": "thread-1",
                "message": "Automatic approval review approved (risk: low, authorization: high): Narrowly scoped to the requested file.",
            },
        ),
        review(
            "completed",
            "guardian-approved",
            "turn-1",
            "rm -f /tmp/guardian-approved.sqlite",
            "approved",
            "low",
            "high",
            "Narrowly scoped to the requested file.",
        ),
    ],
)
write(
    "guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_1",
    802,
    [
        turn("turn/started", "turn-1", "inProgress"),
        mcp("alpha", "starting"),
        review("started", "stale-review", "turn-1", "rm -rf '/tmp/stale-review'", "inProgress"),
        turn("turn/completed", "turn-1", "interrupted"),
    ],
)
write(
    "guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_2",
    802,
    [
        turn("turn/started", "turn-2", "inProgress"),
        review("started", "current-review", "turn-2", "rm -rf '/tmp/current-review'", "inProgress"),
    ],
    seed=False,
)
write(
    "guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_3",
    802,
    [turn("turn/completed", "turn-2", "completed")],
    seed=False,
)
REASON = "Need write access for generated report assets."
write(
    "guardian_approved_request_permissions_clears_status_without_history_1",
    384,
    [
        turn("turn/started", "turn-1", "inProgress"),
        review("started", "guardian-request-permissions", "turn-1", permissions(REASON), "inProgress"),
    ],
)
write(
    "guardian_approved_request_permissions_clears_status_without_history_2",
    384,
    [
        review(
            "completed",
            "guardian-request-permissions",
            "turn-1",
            permissions(REASON),
            "approved",
            "low",
            "high",
            "Request is scoped to report output.",
        ),
    ],
    seed=False,
)
