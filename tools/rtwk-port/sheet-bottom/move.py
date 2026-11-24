"""One shared `widgets::sheet::bottom`: removes the six local copies and imports the shared one.
Run from codex-rs/tui-rtwk with `python3 -I ../../tools/rtwk-port/sheet-bottom/move.py`; rerunnable."""
import re

FILES = [
    "src/sheets/select/apps.rs",
    "src/sheets/select/permissions.rs",
    "src/sheets/select/quick.rs",
    "src/sheets/select/models.rs",
    "src/sheets/select/review_sheet.rs",
    "src/features/plugins/pages.rs",
]
COPY = re.compile(
    r"(?:///[^\n]*\n)*(?:pub )?fn bottom\(page: impl Into<AnyWidget>\) -> AnyWidget \{\n"
    r"    Align::new\(Alignment::BOTTOM_CENTER, page\)\.into\(\)\n\}\n\n?"
)
for path in FILES:
    text = open(path).read()
    new = COPY.sub("", text)
    if new == text:
        continue
    # the layout import: drop Align/Alignment when nothing else uses them
    if "Align::" not in new and "Alignment::" not in new:
        new = re.sub(r"use rtwk::layout::\{Align, Alignment\};\n", "", new)
        new = new.replace("use rtwk::layout::{Align, Alignment, EdgeInsets, Padding};",
                          "use rtwk::layout::{EdgeInsets, Padding};")
    new = re.sub(r"(\nuse rtwk::[^\n]*\n)", r"\1", new, count=1)
    if "widgets::sheet::bottom" not in new:
        idx = new.index("\nuse crate::") if "\nuse crate::" in new else None
        line = "use crate::widgets::sheet::bottom;\n"
        if idx is None:
            m = list(re.finditer(r"^use [^\n]*;\n", new, re.M))[-1]
            new = new[:m.end()] + line + new[m.end():]
        else:
            new = new[:idx + 1] + line + new[idx + 1:]
    open(path, "w").write(new)
    print("rewrote", path)
