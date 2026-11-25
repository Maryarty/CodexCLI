"""Writes the fixtures of the async-question tests of tests/oracle/chat_approvals_2.rs
(codex-rs/tui-rtwk/tests/oracle/fixtures/chat_approvals_2/q_*.jsonl): an agent message that
asks questions (the desktop's `questions`, with the text core joins of titles and options), the
turn that runs, a reply of another client.  Run: python3 -I tools/rtwk-port/async-questions/fixtures.py
"""
import json
import pathlib

OUT = pathlib.Path(__file__).resolve().parents[3] / "codex-rs/tui-rtwk/tests/oracle/fixtures/chat_approvals_2"
THREAD = "thread-1"
SEED = {"step": "seed", "seed": "session", "thread": THREAD}
STARTED = {
    "id": "turn-1", "rootTurnId": None, "items": [], "itemsView": "full", "status": "inProgress",
    "error": None, "startedAt": None, "completedAt": None, "durationMs": None,
}


def notify(method, params):
    return {"step": "notify", "message": {"method": method, "params": params}}


def turn_started():
    return notify("turn/started", {"threadId": THREAD, "turn": STARTED})


def turn_completed():
    done = dict(STARTED, status="completed", itemsView="summary", durationMs=1000)
    return notify("turn/completed", {"threadId": THREAD, "turn": done})


def joined(questions):
    """The visible text: each title, its options as `- ` bullets, a blank row between."""
    parts = []
    for title, options in questions:
        rows = [title] + [f"- {option}" for option in options or []]
        parts.append("\n".join(rows))
    return "\n\n".join(parts)


def asks(message, questions):
    item = {
        "type": "agentMessage", "moderation": None, "id": message, "text": joined(questions),
        "phase": "final_answer", "memoryCitation": None, "delivery": None,
        "questions": [{"title": t, "options": o} for t, o in questions],
        "startedAtMs": None, "completedAtMs": None,
    }
    return notify("item/completed", {"item": item, "threadId": THREAD, "turnId": "turn-1", "completedAtMs": 0})


IDE = "# Context from my IDE setup:\n\n## Open tabs:\n- lib.rs: src/lib.rs\n\n## My request for Codex:\n"


def reply(item_id, title, answer, user="desktop-1"):
    envelope = json.dumps([{"answer": answer, "question": title, "questionItemId": item_id}])
    text = IDE + f"<send_user_message_question_reply>\n{envelope}\n</send_user_message_question_reply>"
    item = {
        "type": "userMessage", "id": user, "clientId": None,
        "content": [{"type": "text", "text": text, "text_elements": []}],
        "startedAtMs": None, "completedAtMs": None,
    }
    return notify("item/completed", {"item": item, "threadId": THREAD, "turnId": "turn-1", "completedAtMs": 0})


def question_id(message, index):
    return json.dumps(["request_user_input_async", message, index], separators=(",", ":"))


def write(name, *steps, note):
    lines = [f"# {note} (tools/rtwk-port/async-questions/fixtures.py)"]
    lines += [json.dumps(step, separators=(",", ":"), ensure_ascii=False) for step in steps]
    (OUT / f"{name}.jsonl").write_text("\n".join(lines) + "\n")


TWO = [("Which way?", None), ("Any details?", None)]
SAME = [("Same title?", None), ("Same title?", None)]
write("q_two", SEED, asks("message", TWO), note="questions_tests.rs: questions()")
write("q_two_running", SEED, turn_started(), asks("message", TWO), note="questions_tests.rs:595 turn running, questions()")
write("q_two_done", SEED, turn_completed(), note="questions_tests.rs:595 the turn ends")
write("q_single_running", SEED, turn_started(), asks("single", [("Only question?", None)]),
      note="questions_tests.rs:845 single question, turn running")
write("q_named", SEED, asks("message", [("What next?", ["x" * 300 + " but do not deploy"])]),
      note="questions_tests.rs:432 one option of 300 characters")
write("q_oversized", SEED, asks("message", [("What next?", ["x" * 600 + " but do not deploy"])]),
      note="questions_tests.rs:432 an option over the 512 bytes the model's suggestions are bounded to is dropped")
write("q_plain", SEED, asks("message", [("Which way?", None)]), note="questions_tests.rs:624 one question, no choices")
write("q_numbered", SEED, asks("message", [("Which way?", ["Named"])]), note="questions_tests.rs:624 one question, one choice")
write("q_url", SEED, asks("message", [("https://example.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/r2/overview", None)]),
      note="questions_tests.rs:673 a question titled with a long URL")
write("q_same", SEED, asks("message", SAME), note="questions_tests.rs:985 two questions with one title")
write("q_same_answered", SEED, asks("message", SAME), reply(question_id("message", 0), "Same title?", "Answer from **desktop**"),
      note="questions_tests.rs:985 the second client answers question 0 before the first looks")
write("q_desktop_reply", reply(question_id("message", 0), "Same title?", "Answer from **desktop**"),
      note="questions_tests.rs:985 the reply of the other client")
MARKDOWN = [
    ("Keep `--verbose` for **failed** runs?", ["Only for **CI**"]),
    ("See the [guide](https://example.test/guide)?", None),
]
write("q_markdown", SEED, asks("ask", MARKDOWN), note="questions_tests.rs:1068 titles with Markdown")
# state_tests.rs `editor()`: First (no choices), Second; an arrival while the first is typed
write("q_first_second", SEED, asks("message", [("First", None), ("Second", ["Named"])]),
      note="state_tests.rs:11 editor(): First, Second with one choice")
write("q_new", asks("new", [("New", None)]), note="state_tests.rs:95 append(new, [New])")
write("q_history", SEED, asks("message", [("First", None), ("Second", [f"Option {i}" for i in range(1, 13)])]),
      note="state_tests.rs:329 Second with twelve choices")
write("q_next", asks("next", [("Which way?", None)]), note="questions_tests.rs:7 push_async_questions(next)")
