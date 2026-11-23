"""/rename: the name prompt opens (Name thread), a typed name is saved."""
from scenario import Scenario, Turn, text_response

SCENARIO = Scenario(
    name="rename-prompt",
    description="a turn, then a bare /rename (the prompt asks for a name; Enter saves what was typed), then a turn",
    turns=[
        Turn("hello", [text_response("rn1", ["Hello! How can I help?"], pace=0.01)]),
        Turn("/rename\rBetter title", []),
        Turn("and now?", [text_response("rn3", ["After the rename. ", "END-rename-prompt"], pace=0.01)]),
    ],
    sentinel="END-rename-prompt",
    pty_only=True,
    helpers="ev_assistant_message",
    suite="no suite twin (a slash prompt)",
    notes="The second prompt is bare /rename, Enter, a name, and the turn's closing Enter submits it; it never reaches the model.",
)
