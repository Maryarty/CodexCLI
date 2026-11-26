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


def turn_started(turn="turn-1"):
    return notify("turn/started", {"threadId": THREAD, "turn": dict(STARTED, id=turn)})


def turn_completed(turn="turn-1", status="completed", error=None):
    done = dict(STARTED, id=turn, status=status, itemsView="summary", durationMs=1000, error=error)
    return notify("turn/completed", {"threadId": THREAD, "turn": done})


def joined(questions):
    """The visible text: each title, its options as `- ` bullets, a blank row between."""
    parts = []
    for title, options in questions:
        rows = [title] + [f"- {option}" for option in options or []]
        parts.append("\n".join(rows))
    return "\n\n".join(parts)


def asks(message, questions, turn="turn-1"):
    item = {
        "type": "agentMessage", "moderation": None, "id": message, "text": joined(questions),
        "phase": "final_answer", "memoryCitation": None, "delivery": None,
        "questions": [{"title": t, "options": o} for t, o in questions],
        "startedAtMs": None, "completedAtMs": None,
    }
    return notify("item/completed", {"item": item, "threadId": THREAD, "turnId": turn, "completedAtMs": 0})


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
write("q_two_done", SEED, turn_completed(), note="question_turn_end_tests.rs: live completion of the first turn")
write("q_queue_cleared", {"step": "screen", "dir": "chat", "ev": {"pending": {"thread": THREAD}}},
      note="questions_tests.rs:595 the cleared preview queue")
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
# questions_tests.rs:647 open_questions(): First?, Second? with one choice, a turn running (the queue)
REVIEW = [("First?", None), ("Second?", ["Next"])]
write("q_review_running", SEED, turn_started(), asks("review", REVIEW),
      note="questions_tests.rs:647 open_questions(), turn running so that messages queue")
write("q_review", SEED, asks("review", REVIEW), note="questions_tests.rs:647 open_questions()")
write("q_buffered", asks("buffered", [("Buffered?", None)]), note="questions_tests.rs:793 the replayed question that arrives live")
# questions_tests.rs:889 a turn that runs (messages queue), the questions that arrive after the queue is on screen
write("q_run", SEED, turn_started(), note="questions_tests.rs:889 a running turn, no questions yet")
write("q_ask_two", asks("message", TWO), note="questions_tests.rs:889 add_async_questions(message)")
write("q_remapped", asks("remapped", TWO), note="questions_tests.rs:889 add_async_questions(remapped)")

# question_turn_end_tests.rs: live turn endings recover only their own questions.
ONE = [("Which way?", None)]
write("q_turn_end_one", SEED, turn_started(), asks("question", ONE),
      note="question_turn_end_tests.rs: the running turn with one question")
write("q_turn_end_open", turn_started(), asks("first", [("Which style?", None)]),
      asks("second", [("What budget?", None)]),
      note="question_turn_end_tests.rs: separate messages with ordered questions")
write("q_turn_end_large", turn_started("turn-2"), asks("large", ONE, "turn-2"),
      note="question_turn_end_tests.rs: another live turn with a new draft")
write("q_turn_end_second", turn_completed("turn-2"),
      note="question_turn_end_tests.rs: the second turn ends")
write("q_turn_end_multiline", SEED, turn_started(),
      asks("multiline", [("First line\r\n\r\n\x1b[31mLast\r line?\x1b[0m\x07", None)]),
      note="question_turn_end_tests.rs: title line breaks and terminal controls")
ERROR = {"message": "Turn failed", "codexErrorInfo": None, "additionalDetails": None, "misalignment": None}
write("q_turn_end_error",
      {"step": "screen", "dir": "chat", "ev": {"pending": {"thread": THREAD, "queued": ["queued prompt"]}}},
      notify("error", {"threadId": THREAD, "turnId": "turn-1", "error": ERROR, "willRetry": False}),
      turn_completed(status="failed", error=ERROR),
      note="question_turn_end_tests.rs: non-retry error followed by the duplicate failed completion")
write("q_turn_end_interrupted",
      {"step": "screen", "dir": "chat", "ev": {"pending": {
          "thread": THREAD, "steers": ["pending steer"], "queued": ["queued prompt"],
      }}}, turn_completed(status="interrupted"),
      note="question_turn_end_tests.rs: restore the pending steer and queued prompt before the answer")
write("q_turn_end_coalesced", turn_completed(), turn_started("turn-2"),
      asks("new-question", [("A new question?", None)], "turn-2"),
      asks("question", ONE), asks("late-question", [("An orphan question?", None)]),
      note="the ended question, the new turn, and two old items before the next frame")
write("q_turn_end_two", turn_completed(), turn_started("turn-2"),
      asks("new-question", [("A new question?", None)], "turn-2"), turn_completed("turn-2"),
      note="two separate live turns with questions end before the next frame")
write("q_turn_end_desktop_reply", reply(question_id("question", 0), "Which way?", "Desktop answer"),
      note="a desktop answers after the owner's interruption")
write("q_turn_end_in_progress", turn_completed(status="inProgress"),
      note="a partial update does not end the running turn")
