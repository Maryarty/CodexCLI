"""Folds the seven worktree variants of `select::Note` into one, `Note::Worktree(Line::..)`
(hub_enums_only_shrink: the hub's `Note` may not grow). Run from the repo root:
python3 -I tools/rtwk-port/worktree-notes/fold.py  (idempotent: a folded file is left as it is)."""
import pathlib

CRATE = pathlib.Path("codex-rs/tui-rtwk")
FILES = [
    "core/src/screens/select.rs",
    "core/src/app/worktree.rs",
    "src/pages/session/strings.rs",
    "core/tests/worktree_popups.rs",
]
# variant -> (unit, opener)
VARIANTS = {
    "WorktreesUnavailable": "Unavailable",
    "WorktreeListFailed": "ListFailed",
    "WorktreeRefused": "Refused",
    "WorktreeFailed": "Failed",
    "WorktreeRetained": "Retained",
    "WorktreeRemoved": "Removed",
    "WorktreeRemoveFailed": "RemoveFailed",
}
CLOSE = {"(": ")", "{": "}"}


def fold(text: str) -> str:
    out, i = [], 0
    while i < len(text):
        hit = None
        if text.startswith("Note::Worktree", i) and not text.startswith("Note::Worktree(", i):
            for old, new in VARIANTS.items():
                end = i + len("Note::") + len(old)
                if text.startswith("Note::" + old, i) and not (
                    text[end : end + 1].isalnum() or text[end : end + 1] == "_"
                ):
                    if hit is None or len(old) > len(hit[0]):
                        hit = (old, new, end)
        if hit is None:
            out.append(text[i])
            i += 1
            continue
        old, new, end = hit
        j = end
        rest = text[j:]
        stripped = rest.lstrip(" ")
        opener = stripped[:1]
        if opener in CLOSE and not (opener == "{" and False):
            k = j + (len(rest) - len(stripped))
            depth, m = 0, k
            while True:
                c = text[m]
                if c in "({[":
                    depth += 1
                elif c in ")}]":
                    depth -= 1
                    if depth == 0:
                        break
                m += 1
            out.append(f"Note::Worktree(Line::{new}{text[j:m + 1]})")
            i = m + 1
        else:
            out.append(f"Note::Worktree(Line::{new})")
            i = end
    return "".join(out)


for name in FILES:
    path = CRATE / name
    text = path.read_text()
    new = fold(text)
    if new != text:
        path.write_text(new)
        print("folded", name)
