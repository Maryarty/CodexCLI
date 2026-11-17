#!/usr/bin/env python3
"""Render design.md (Markdown + Mermaid) to a self-contained design.html.

Usage: python3 -I docs/rtwk-port/design_html.py
Stdlib only. Handles what design.md uses: headings, paragraphs, lists,
links, inline code, bold, and ```mermaid blocks (drawn in the browser by
Mermaid from a CDN).
"""
import html
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
SRC, OUT = HERE / "design.md", HERE / "design.html"


def inline(text: str) -> str:
    t = html.escape(text, quote=False)
    t = re.sub(r"`([^`]+)`", r"<code>\1</code>", t)
    t = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", t)
    t = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", lambda m: f'<a href="{m.group(2).replace(".md", ".html") if m.group(2) == "design.md" else m.group(2)}">{m.group(1)}</a>', t)
    return t


def render(md: str) -> str:
    out, para, items, block, lang = [], [], [], None, ""

    def flush():
        nonlocal para, items
        if para:
            out.append(f"<p>{inline(' '.join(para))}</p>")
            para = []
        if items:
            tag = "ol" if items[0][0] else "ul"
            out.append(f"<{tag}>" + "".join(f"<li>{inline(t)}</li>" for _, t in items) + f"</{tag}>")
            items = []

    for line in md.splitlines():
        if block is not None:
            if line.startswith("```"):
                body = "\n".join(block)
                if lang == "mermaid":
                    out.append(f'<div class="diagram"><pre class="mermaid">{html.escape(body)}</pre></div>')
                else:
                    out.append(f"<pre><code>{html.escape(body)}</code></pre>")
                block = None
            else:
                block.append(line)
            continue
        if line.startswith("```"):
            flush()
            block, lang = [], line[3:].strip()
            continue
        m = re.match(r"^(#{1,6})\s+(.*)", line)
        if m:
            flush()
            n = len(m.group(1))
            out.append(f"<h{n}>{inline(m.group(2))}</h{n}>")
            continue
        m = re.match(r"^\s*(\d+\.|[-*])\s+(.*)", line)
        if m:
            if para:
                flush()
            items.append((m.group(1)[0].isdigit(), m.group(2)))
            continue
        if not line.strip():
            flush()
            continue
        if items and line.startswith("   "):
            ordered, text = items[-1]
            items[-1] = (ordered, text + " " + line.strip())
            continue
        para.append(line.strip())
    flush()
    return "\n".join(out)


PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Codex CLI on rtwk — target architecture</title>
<style>
body {{ font: 16px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; max-width: 1100px; margin: 0 auto; padding: 24px; color: #1d1d1f; background: #fafafa; }}
h1 {{ font-size: 28px; }} h2 {{ margin-top: 40px; border-bottom: 1px solid #ddd; padding-bottom: 4px; }}
code {{ background: #eee; padding: 1px 4px; border-radius: 4px; font-size: 90%; }}
.diagram {{ background: #fff; border: 1px solid #e3e3e3; border-radius: 10px; padding: 16px; overflow-x: auto; }}
.note {{ color: #666; font-size: 14px; }}
@media (prefers-color-scheme: dark) {{ body {{ background: #161618; color: #e8e8ea; }} .diagram {{ background: #1f1f22; border-color: #333; }} code {{ background: #2a2a2e; }} h2 {{ border-color: #333; }} a {{ color: #7ab7ff; }} }}
</style></head><body>
{body}
<p class="note">Generated from design.md by design_html.py. Diagrams drawn by Mermaid (loaded from a CDN).</p>
<script type="module">
import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
mermaid.initialize({{ startOnLoad: true, theme: dark ? "dark" : "default", flowchart: {{ useMaxWidth: true }}, securityLevel: "strict" }});
</script>
</body></html>
"""

if __name__ == "__main__":
    OUT.write_text(PAGE.format(body=render(SRC.read_text())))
    print(f"wrote {OUT}")
