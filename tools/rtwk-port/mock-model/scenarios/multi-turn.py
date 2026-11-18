"""A conversation of three prompts: greeting, code, summary."""
from scenario import Scenario, Turn, chunks, text_response

CODE = """Here is a small function:

```python
def greet(name: str) -> str:
    return f"hello, {name}"
```

It returns a greeting.
"""
SCENARIO = Scenario(
    name="multi-turn",
    description="three turns, one response each: a greeting, a code answer, a closing summary",
    turns=[
        Turn("Hello there.", [text_response("mt1", chunks("Hello! How can I help today?", 5), pace=0.01)]),
        Turn("Show me a python function.", [text_response("mt2", chunks(CODE, 12), pace=0.01)]),
        Turn("Thanks, summarize.", [text_response("mt3", chunks("Summary: a greeting and a function. END-multi-turn", 8), pace=0.01)]),
    ],
    sentinel="END-multi-turn",
)
