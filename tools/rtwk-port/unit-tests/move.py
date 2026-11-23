#!/usr/bin/env python3 -I
"""Move the in-src unit tests of tui-rtwk into the test roots (build-speed.md section 8).

    python3 -I tools/rtwk-port/unit-tests/move.py [--list FILE] [--no-check] [src files ...]

For every `#[cfg(test)] mod <name> { ... }` (inline) or `#[cfg(test)] mod <name>;` (file) at
the top level of a listed src file the body is written to
    codex-rs/tui-rtwk/core/tests/unit/<path below core/src>.rs   (crate codex-tui-core)
    codex-rs/tui-rtwk/tests/unit/<path below src>.rs             (crate codex-tui-rtwk)
with `use super::*` turned into `use <crate>::<module path>::*`, the parent's own private
`use` lines copied over (`crate::` -> the crate name), and the module registered by
`tests/unit/mod.rs` (generated: one `#[path]` line per moved file), itself `mod unit;` of
core/tests/all.rs and tests/screens.rs.

Then `acargo check --tests -p <crate>` runs on the whole batch; a moved module that does not
compile (a private item, a pub(crate) path, ...) is put back into its src file exactly as it
was, the check runs again, until clean. Each put-back is a line of
tools/rtwk-port/unit-tests/kept.txt: `<file>: <first compiler error>`.
No visibility is changed in src: that is a decision, not mechanics.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()
)
CRATE_DIR = REPO / "codex-rs" / "tui-rtwk"
KEPT = REPO / "tools" / "rtwk-port" / "unit-tests" / "kept.txt"
ACARGO = Path.home() / ".cache" / "rtwk" / "acargo"

# crate: (cargo package, rust name, src dir, unit dir, root file that holds `mod unit;`)
CRATES = {
    "core": (
        "codex-tui-core",
        "codex_tui_core",
        CRATE_DIR / "core" / "src",
        CRATE_DIR / "core" / "tests" / "unit",
        CRATE_DIR / "core" / "tests" / "all.rs",
    ),
    "ui": (
        "codex-tui-rtwk",
        "codex_tui_rtwk",
        CRATE_DIR / "src",
        CRATE_DIR / "tests" / "unit",
        CRATE_DIR / "tests" / "screens.rs",
    ),
}


# ---------------------------------------------------------------- lexing


def match_brace(text, open_idx):
    """Index of the `}` that closes the `{` at open_idx (strings, chars, comments aware)."""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if text.startswith("/*", i):
            level = 1
            i += 2
            while i < n and level:
                if text.startswith("/*", i):
                    level += 1
                    i += 2
                elif text.startswith("*/", i):
                    level -= 1
                    i += 2
                else:
                    i += 1
            continue
        m = re.compile(r'b?r(#*)"').match(text, i)
        if m and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            end = '"' + m.group(1)
            j = text.find(end, m.end())
            i = n if j < 0 else j + len(end)
            continue
        if c == '"':
            i += 1
            while i < n and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
            i += 1
            continue
        if c == "'":
            # a char literal ('x', '\n', '\u{..}') or a lifetime ('a)
            m = re.compile(r"'(\\u\{[0-9a-fA-F_]+\}|\\x[0-9a-fA-F]{2}|\\.|[^\\'])'").match(text, i)
            if m:
                i = m.end()
                continue
            i += 1
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced braces")


# ---------------------------------------------------------------- src files


_MODMAP = {}

DECL_RE = re.compile(
    r'^(?P<attrs>(?:\s*#\[[^\n]*\]\n)*)\s*(?:pub(?:\([a-z: ]+\))? )?mod (?P<name>\w+);', re.M
)


def module_map(src_dir):
    """src file -> module path, from the `mod x;` declarations below lib.rs (`#[path]` aware)."""
    if src_dir in _MODMAP:
        return _MODMAP[src_dir]
    out = {}

    def walk(file, parts):
        out[file] = parts
        text = file.read_text()
        is_root = file.name in ("mod.rs", "lib.rs", "main.rs")
        base = file.parent if is_root else file.parent / file.stem
        for m in DECL_RE.finditer(text):
            attrs = m.group("attrs")
            if "cfg(test)" in attrs:
                continue
            name = m.group("name")
            pm = re.search(r'#\[path = "([^"]+)"\]', attrs)
            if pm:
                cands = [file.parent / pm.group(1)]
            else:
                cands = [base / f"{name}.rs", base / name / "mod.rs"]
            f = next((c for c in cands if c.exists()), None)
            if f is not None and f.resolve() not in out:
                walk(f.resolve(), parts + [name])

    walk((src_dir / "lib.rs").resolve(), [])
    # modules a macro declares (`$(pub mod $dir;)*`) are not seen above: a file whose
    # directory's module is known is that module's child
    progress = True
    while progress:
        progress = False
        for f in sorted(src_dir.rglob("*.rs")):
            f = f.resolve()
            if f in out or f.name in ("mod.rs", "lib.rs"):
                continue
            for owner in (f.parent / "mod.rs", f.parent.with_suffix(".rs")):
                if owner in out:
                    walk(f, out[owner] + [f.stem])
                    progress = True
                    break
    _MODMAP[src_dir] = out
    return out


def module_path(src_dir, path):
    mm = module_map(src_dir)
    if path in mm:
        return mm[path]
    rel = path.relative_to(src_dir).with_suffix("")
    parts = list(rel.parts)
    if parts and parts[-1] == "mod":
        parts.pop()
    return parts


class Block:
    """A `#[cfg(test)] mod name ...` of a src file."""

    def __init__(self, name, start, end, attrs, body, file_path=None):
        self.name = name
        self.start = start  # offsets of the whole block in the src text
        self.end = end
        self.attrs = attrs  # `allow(...)` attribute texts between cfg(test) and mod
        self.body = body  # dedented module body (inline) or the tests file text
        self.file_path = file_path  # `mod name;`: the file that held the body


MOD_RE = re.compile(r"^(?:pub(?:\([a-z]+\))? )?mod (\w+)\s*([{;])", re.M)


def find_blocks(path, text):
    blocks = []
    pos = 0
    for m in re.finditer(r"^#\[cfg\(test\)\]\n", text, re.M):
        if m.start() < pos:
            continue
        i = m.end()
        attrs = []
        while True:
            am = re.compile(r"(#\[[^\n]*\])\n").match(text, i)
            if not am:
                break
            attrs.append(am.group(1))
            i = am.end()
        mm = MOD_RE.match(text, i)
        if not mm:
            continue
        name = mm.group(1)
        if mm.group(2) == ";":
            end = mm.end()
            if text[end : end + 1] == "\n":
                end += 1
            stem = path.stem
            base = path.parent if stem in ("mod", "lib") else path.parent / stem
            cands = [base / f"{name}.rs", base / name / "mod.rs"]
            fp = next((c for c in cands if c.exists()), None)
            if fp is None:
                blocks.append(Block(name, m.start(), end, attrs, None))
            else:
                blocks.append(Block(name, m.start(), end, attrs, fp.read_text(), fp))
            pos = end
            continue
        open_idx = mm.end() - 1
        close = match_brace(text, open_idx)
        end = close + 1
        if text[end : end + 1] == "\n":
            end += 1
        body = text[open_idx + 1 : close]
        body = body[1:] if body.startswith("\n") else body
        body = "\n".join(l[4:] if l.startswith("    ") else l for l in body.split("\n"))
        body = body.rstrip() + "\n"
        blocks.append(Block(name, m.start(), end, attrs, body))
        pos = end
    return blocks


# ---------------------------------------------------------------- use lines


def split_top(s):
    parts, depth, cur = [], 0, ""
    for c in s:
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts if p.strip()]


def expand(tree, prefix=""):
    """`a::{b, c::d as e}` -> [("a::b", None), ("a::c::d", "e")]."""
    tree = tree.strip()
    br = tree.find("{")
    if br >= 0 and tree.endswith("}"):
        head = tree[:br].strip()
        inner = tree[br + 1 : -1]
        out = []
        for part in split_top(inner):
            out += expand(part, prefix + head)
        return out
    alias = None
    am = re.match(r"(.*?)\s+as\s+(\w+)$", tree)
    if am:
        tree, alias = am.group(1).strip(), am.group(2)
    full = prefix + tree
    if full.endswith("::self"):
        full = full[: -len("::self")]
    return [(full, alias)]


def parent_uses(text):
    """The private top-level `use` statements of a src file as (path, alias)."""
    out = []
    for m in re.finditer(r"^use ([^;]+);", text, re.M):
        out += expand(re.sub(r"\s+", " ", m.group(1)))
    return out


def rewrite_path(p, crate_name, mod_parts):
    if p.startswith("crate::"):
        return crate_name + "::" + p[len("crate::") :]
    if p.startswith("self::"):
        return "::".join([crate_name] + mod_parts) + "::" + p[len("self::") :]
    if p.startswith("super::"):
        rest = p
        parts = list(mod_parts)
        while rest.startswith("super::"):
            parts = parts[:-1]
            rest = rest[len("super::") :]
        return "::".join([crate_name] + parts + [rest])
    return p


def uses_for(body, text, crate_name, mod_parts):
    lines = []
    for path, alias in parent_uses(text):
        if path.endswith("::*"):
            keep = True
        else:
            name = alias or path.split("::")[-1]
            internal = path.split("::")[0] in ("crate", "self", "super")
            keep = alias == "_" or not internal or re.search(rf"\b{re.escape(name)}\b", body)
        if keep and not path.endswith("::*"):
            name = alias or path.split("::")[-1]
            if re.search(rf"^\s*(?:pub )?use [^;]*\b{re.escape(name)}\b[^;]*;", body, re.M) or re.search(
                rf"^\s*(?:pub )?(?:fn|struct|enum|trait|type|const|static|mod) {re.escape(name)}\b", body, re.M
            ):
                keep = False
        if keep:
            rp = rewrite_path(path, crate_name, mod_parts)
            lines.append(f"use {rp}{' as ' + alias if alias else ''};")
    return lines


def rewrite_body(block, text, crate_name, mod_parts):
    body = block.body
    modp = "::".join([crate_name] + mod_parts)
    nested = re.search(r"^\s+(pub(\([a-z]+\))? )?mod \w+\s*\{", body, re.M) is not None
    out = []
    for line in body.split("\n"):
        if re.match(r"use super::\*;", line):
            line = f"use {modp}::*;"
        elif not nested or re.match(r"use super::", line):
            line = re.sub(
                r"(?<![\w:])((?:super::)+)",
                lambda m: "::".join(
                    [crate_name] + mod_parts[: len(mod_parts) - (m.group(1).count("super::") - 1)]
                )
                + "::",
                line,
            )
        line = re.sub(r"(?<![\w:])crate::", crate_name + "::", line)
        out.append(line)
    body = "\n".join(out)
    # inner attributes of the moved module must come before the first item
    inner = [a.replace("#[", "#![", 1) for a in block.attrs]
    m = re.match(r"((?:\s*(?://![^\n]*|#!\[[^\n]*\])\n)*)", body)
    head, rest = body[: m.end()], body[m.end() :]
    uses = uses_for(rest, text, crate_name, mod_parts)
    # `use super::*` already replaced; the parent's uses go behind it
    pieces = [head] if head.strip() else []
    if inner:
        pieces.append("\n".join(inner) + "\n")
    pieces.append(rest)
    res = "".join(pieces)
    if uses:
        marker = f"use {modp}::*;"
        if marker in res:
            res = res.replace(marker, marker + "\n" + "\n".join(uses), 1)
        else:
            res = "\n".join(uses) + "\n\n" + res
    return res


# ---------------------------------------------------------------- batch


def fix_includes(content, src_path, target):
    """include_str!("rel") is relative to the file that holds it: point it at the old place."""

    def fix(m):
        rel = m.group(2)
        if rel.startswith("/") or "CARGO" in rel:
            return m.group(0)
        old = (src_path.parent / rel).resolve()
        return f'{m.group(1)}("{os.path.relpath(old, target.parent)}")'

    return re.sub(r'\b(include_str!|include_bytes!)\(\s*"([^"]+)"\s*\)', fix, content)


def unit_name(rel_parts):
    return "_".join(rel_parts)


def regenerate_mod(crate):
    pkg, name, src_dir, unit_dir, root = CRATES[crate]
    files = sorted(
        p for p in unit_dir.rglob("*.rs") if p.name != "mod.rs"
    ) if unit_dir.exists() else []
    if not files:
        if (unit_dir / "mod.rs").exists():
            (unit_dir / "mod.rs").unlink()
        text = root.read_text()
        text = re.sub(r"\nmod unit;\n", "\n", text)
        root.write_text(text)
        return
    lines = [
        "//! The unit tests moved out of the src files by tools/rtwk-port/unit-tests/move.py",
        "//! (one file per moved module, the path below src mirrored; this list is generated).",
        "",
        "#![allow(unused_imports, clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]",
        "",
    ]
    used = set()
    for p in files:
        rel = p.relative_to(unit_dir)
        mod = unit_name(rel.with_suffix("").parts)
        while mod in used:
            mod += "_"
        used.add(mod)
        lines.append(f'#[path = "{rel.as_posix()}"]')
        lines.append(f"mod {mod};")
    unit_dir.mkdir(parents=True, exist_ok=True)
    (unit_dir / "mod.rs").write_text("\n".join(lines) + "\n")
    text = root.read_text()
    if not re.search(r"^mod unit;$", text, re.M):
        if root.name == "all.rs":
            text = text.replace("mod worked_for;\n", "mod unit;\nmod worked_for;\n", 1)
            if "mod unit;" not in text:
                text = text.rstrip("\n") + "\nmod unit;\n"
        else:
            text = re.sub(r"^(mod common;\n)", r"\1mod unit;\n", text, count=1, flags=re.M)
            if "mod unit;" not in text:
                text = text.rstrip("\n") + "\nmod unit;\n"
        root.write_text(text)


PRIV_RE = re.compile(r"error\[E0(603|624|616|451|638)\]")
ERR_RE = re.compile(r"^(\S+?\.rs):(\d+):(\d+): (error.*)$", re.M)


def check(pkg):
    r = subprocess.run(
        [str(ACARGO), "check", "-q", "--tests", "-p", pkg, "--message-format=short"],
        cwd=CRATE_DIR,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stdout + r.stderr


def main(argv):
    do_check = True
    files = []
    it = iter(argv)
    for a in it:
        if a == "--no-check":
            do_check = False
        elif a == "--list":
            for l in Path(next(it)).read_text().split("\n"):
                if l.strip():
                    files.append(l.strip())
        else:
            files.append(a)
    paths = []
    for f in files:
        p = Path(f)
        if not p.is_absolute():
            p = REPO / f if (REPO / f).exists() else CRATE_DIR / f
        paths.append(p.resolve())

    # crate -> {src path: (original text, [blocks], new text, written unit files, removed files)}
    state = {"core": {}, "ui": {}}
    for p in paths:
        crate = "core" if CRATES["core"][2] in p.parents else "ui"
        src_dir = CRATES[crate][2]
        if src_dir not in p.parents:
            print(f"skip {p}: not a src file of the crates", file=sys.stderr)
            continue
        text = p.read_text()
        blocks = [b for b in find_blocks(p, text) if b.body is not None]
        # only plain `allow` attributes can move into the file
        blocks = [b for b in blocks if all(a.startswith("#[allow(") for a in b.attrs)]
        if not blocks:
            continue
        mod_parts = module_path(src_dir, p)
        rel = p.relative_to(src_dir).with_suffix("")
        unit_dir = CRATES[crate][3]
        new_text = text
        written, removed = {}, {}
        for idx, b in enumerate(blocks):
            target = unit_dir / (rel.as_posix() + (f"__{b.name}" if idx else "") + ".rs")
            written[target] = fix_includes(
                rewrite_body(b, text, CRATES[crate][1], mod_parts), b.file_path or p, target
            )
            if b.file_path is not None:
                removed[b.file_path] = b.body
        for b in sorted(blocks, key=lambda b: -b.start):
            new_text = new_text[: b.start] + new_text[b.end :]
        new_text = new_text.rstrip("\n") + "\n"
        state[crate][p] = (text, written, removed, new_text)

    for crate in ("core", "ui"):
        files_state = state[crate]
        if not files_state:
            continue
        pkg = CRATES[crate][0]
        kept = {}  # path -> first error

        def apply(p):
            text, written, removed, new_text = files_state[p]
            for t, content in written.items():
                t.parent.mkdir(parents=True, exist_ok=True)
                t.write_text(content)
            for r in removed:
                r.unlink()
            p.write_text(new_text)

        def revert(p, why):
            text, written, removed, new_text = files_state.pop(p)
            p.write_text(text)
            for t in written:
                if t.exists():
                    t.unlink()
            for r, content in removed.items():
                r.write_text(content)
            kept[p] = why

        for p in list(files_state):
            apply(p)
        regenerate_mod(crate)
        while do_check and files_state:
            code, out = check(pkg)
            if code == 0:
                break
            by_file = {}
            for m in ERR_RE.finditer(out):
                loc = (CRATE_DIR / m.group(1)).resolve() if not Path(m.group(1)).is_absolute() else Path(m.group(1))
                msg = f"{m.group(4)} ({m.group(1)}:{m.group(2)})"
                if loc not in by_file or (
                    not PRIV_RE.match(by_file[loc]) and PRIV_RE.match(m.group(4))
                ):
                    by_file[loc] = msg
            hit = []
            for p, (text, written, removed, new_text) in files_state.items():
                for t in written:
                    if t in by_file:
                        hit.append((p, by_file[t]))
                        break
                else:
                    if p in by_file:
                        hit.append((p, by_file[p]))
            if not hit:
                print(out[-3000:], file=sys.stderr)
                print(f"{pkg}: errors outside the moved modules; stopping with the batch applied", file=sys.stderr)
                return 1
            for p, why in hit:
                revert(p, why)
            regenerate_mod(crate)
        _record_kept(kept, set(kept) | set(files_state))
        done = len(files_state)
        print(f"{pkg}: {done} files moved, {len(kept)} kept")
    return 0


def _record_kept(kept, processed):
    lines = KEPT.read_text().split("\n") if KEPT.exists() else []
    rels = {str(p.relative_to(REPO)) for p in processed}
    lines = [l for l in lines if l.strip() and l.split(":", 1)[0] not in rels]
    for p, why in kept.items():
        lines.append(f"{p.relative_to(REPO)}: {why}")
    KEPT.parent.mkdir(parents=True, exist_ok=True)
    KEPT.write_text("\n".join(sorted(lines)) + ("\n" if lines else ""))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
