#!/usr/bin/env python3 -I
"""One-off: put the test files of the `pages` root under the marker scan (fw-ui-reg-2).

    python3 -I tools/rtwk-port/test-roots/adopt.py [--root tests/pages.rs]

The hand-written `tests/pages.rs` named its modules (`#[path = "pages/x.rs"] mod x;` and the
top-level `mod rtwk_g9;` probes). The generated lists (`codex-rs/tui-rtwk/test-mods/scan.rs`)
find a file's root from a marker line on its first lines, so this writes `// test root: pages` on
line 1 of every file the old root named, and `git mv`s the top-level `tests/rtwk_*.rs` probes
into `tests/pages/`. Rerunnable (a marked file is skipped). Run it BEFORE the root is rewritten
to `tui_test_mods::pages!();`; afterwards a file of the pages root is made by writing
the marker as its first line.
"""

import re
import subprocess
import sys
from pathlib import Path

CRATE = Path(__file__).resolve().parents[3] / "codex-rs" / "tui-rtwk"
MARKER = "// test root: pages"


def main() -> None:
    root = CRATE / (sys.argv[sys.argv.index("--root") + 1] if "--root" in sys.argv else "tests/pages.rs")
    text = root.read_text()
    files = [CRATE / "tests" / m for m in re.findall(r'#\[path = "((?:pages|sheets|app)/[a-z_0-9]+\.rs)"\]', text)]
    for probe in re.findall(r"^mod (rtwk_\w+);$", text, flags=re.M):
        old, new = CRATE / "tests" / f"{probe}.rs", CRATE / "tests" / "pages" / f"{probe}.rs"
        if old.exists():
            subprocess.run(["git", "mv", str(old), str(new)], check=True)
        files.append(new)
    marked = 0
    for path in files:
        body = path.read_text()
        if any(line.startswith(MARKER) for line in body.splitlines()[:10]):
            continue
        path.write_text(MARKER + "\n" + body)
        marked += 1
    print(f"{len(files)} files named by {root.name}, {marked} marked")


if __name__ == "__main__":
    main()
