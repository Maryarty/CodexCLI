"""Markdown with a heading, list, rust code block, table, quote and inline code."""
from scenario import Scenario, Turn, chunks, text_response

TEXT = """# Mock markdown

A paragraph with **bold**, *italic*, `inline code` and a [link](https://example.com).

## A list

- first item
- second item with `code`
  - nested item
1. one
2. two

## Code

```rust
fn main() {
    let items = vec![1, 2, 3];
    for item in &items {
        println!("item = {item}");
    }
}
```

## Table

| name  | kind    | count |
|-------|---------|------:|
| alpha | letter  |     1 |
| beta  | letter  |    22 |
| gamma | letter  |   333 |

> a quote that runs long enough to wrap in a narrow terminal, so the wrapping of quoted text is visible too.

END-markdown
"""
SCENARIO = Scenario(
    name="markdown",
    description="markdown answer (heading, lists, code block, table, quote) streamed 16 chars per delta at 200/s",
    turns=[Turn("Show me some markdown.", [text_response("md", chunks(TEXT, 16), pace=0.005)])],
    sentinel="END-markdown",
    helpers='ev_output_text_delta, ev_assistant_message',
    suite='items.rs (agent message items); markdown content has no suite twin',
)
