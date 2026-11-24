"""Generates codex-rs/tui-rtwk/src/oracle_strings/<package>.rs: the oracle's UI string literals
(codex-rs/tui/src) that the port does not have yet, one `pub const NAME: &str` per literal with an
`// oracle <file>:<line>` comment, grouped by the W2 package that owns the oracle module.

Run from the repository root: python3 -I tools/rtwk-port/oracle-strings/gen.py [--stats]
A second run changes nothing; a literal the port gained since (in src/, core/src or host/src outside
src/oracle_strings) drops out of the tables on the next run, which is how a porter "consumes" a constant.
See README.md for the filters.
"""
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ORACLE = ROOT / "codex-rs/tui/src"
PORT = ROOT / "codex-rs/tui-rtwk"
OUT = PORT / "src/oracle_strings"
sys.path.insert(0, str(ROOT / "tools/rtwk-port/map-full"))
import modules  # noqa: E402

# W2 package -> the module owners of tools/rtwk-port/map-full/modules.py it merges (briefs.md part A).
PACKAGES = {
    "transcript": ["ui-pager-find", "ui-select-copy", "ui-transcript-chrome"],
    "composer": ["ui-composer-states", "composer-sweep"],
    "settings": ["ui-settings-feeds", "ui-config-panel", "ui-keymap-editor"],
    "cells": ["ui-exec-mcp", "ui-safety-compaction", "cells-sweep"],
    "approvals": ["ui-approvals-rest", "dialogs-sweep"],
    "commands": ["ui-openers-side", "ui-worktree-sandbox", "ui-feedback-cmds"],
    "status": ["ui-usage-limits", "footer-status-sweep"],
    "live": ["live-turn-sweep", "markdown-gaps"],
}
OWNER_TO_PACKAGE = {o: p for p, owners in PACKAGES.items() for o in owners}

TEST_FILE = re.compile(
    r"(^|/)(tests?\.rs|[^/]*_tests?\.rs|[^/]*test_support[^/]*|test_backend[^/]*|snapshots|bin|tests)(/|$)|"
    r"_tests?\.rs$|_test_support\.rs$"
)

# Calls and macros whose string arguments are not UI text (logs, panics, asserts, keys, paths, JSON).
SKIP_CALLEES = {
    "trace!", "debug!", "info!", "warn!", "error!", "event!", "span!", "trace_span!", "debug_span!",
    "info_span!", "warn_span!", "error_span!", "panic!", "unreachable!", "todo!", "unimplemented!",
    "assert!", "assert_eq!", "assert_ne!", "debug_assert!", "debug_assert_eq!", "debug_assert_ne!",
    "matches!", "env!", "option_env!", "include_str!", "include_bytes!", "stringify!", "cfg!", "json!",
    "println!", "eprintln!", "print!", "eprint!", "dbg!", "expect", "expect_err", "context",
    "with_context", "starts_with", "ends_with", "contains", "strip_prefix", "strip_suffix", "split",
    "rsplit", "splitn", "rsplitn", "split_once", "rsplit_once", "find", "rfind", "trim_start_matches",
    "trim_end_matches", "trim_matches", "eq_ignore_ascii_case", "get", "get_mut", "insert", "remove",
    "entry", "var", "var_os", "arg", "args", "env", "join", "parse", "from_str", "Path::new",
    "PathBuf::from", "Command::new", "OsStr::new", "OsString::from", "Regex::new", "Url::parse",
    "from_env", "instrument", "rename", "alias", "cfg", "target_os", "feature", "serde", "derive",
    "snapshot", "assert_snapshot!", "insta", "r#try", "concat_idents", "test", "tokio::test",
}
TEST_ATTR_WORDS = {"test"}

IDENT_LIKE = re.compile(r"[a-z0-9_./:\\\-#@=*%+,;<>~^]+")
CAMEL = re.compile(r"[a-z][a-z0-9]*([A-Z][a-z0-9]*)+")
SCREAMING = re.compile(r"[A-Z0-9_]+")


def tokenize(src):
    """Rust source -> [(kind, text, line)]: kind 'str' (text = decoded value), 'raw' (byte string,
    ignored), 'id', 'num', 'p'. Comments are dropped."""
    toks, i, n, line = [], 0, len(src), 1
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif c in " \t\r":
            i += 1
        elif src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    line += src[i] == "\n"
                    i += 1
        elif c == '"' or (c in "br" and STR_PREFIX.match(src, i)):
            m = STR_PREFIX.match(src, i) or re.compile('"').match(src, i)
            prefix = m.group(0)
            start, start_line = i, line
            i += len(prefix)
            if "r" in prefix:
                end = '"' + prefix.lstrip("br").rstrip('"')
                j = src.index(end, i)
                value = src[i:j]
                i = j + len(end)
            else:
                j = i
                while src[j] != '"':
                    j += 2 if src[j] == "\\" else 1
                value = decode(src[i:j])
                i = j + 1
            line += src.count("\n", start, i)
            toks.append(("raw" if prefix.startswith("b") else "str", value, start_line))
        elif c == "'":
            if src[i + 1:i + 2] == "\\":
                i = src.index("'", i + 3) + 1
            elif src[i + 2:i + 3] == "'":
                i += 3
            else:  # lifetime
                m = re.compile(r"'[A-Za-z_][A-Za-z0-9_]*").match(src, i)
                i = m.end() if m else i + 1
        elif c.isalpha() or c == "_":
            m = re.compile(r"(r#)?[A-Za-z_][A-Za-z0-9_]*").match(src, i)
            i = m.end()
            toks.append(("id", m.group(0), line))
        elif c.isdigit():
            m = re.compile(r"[0-9][0-9A-Za-z_]*").match(src, i)
            i = m.end()
            toks.append(("num", m.group(0), line))
        else:
            two = src[i:i + 2]
            if two in ("=>", "==", "!=", "::", "||", "&&", "->", "<=", ">="):
                toks.append(("p", two, line))
                i += 2
            else:
                toks.append(("p", c, line))
                i += 1
    return toks


STR_PREFIX = re.compile(r'b?r#*"|b"')


ESC = {"n": "\n", "r": "\r", "t": "\t", "\\": "\\", "0": "\0", '"': '"', "'": "'"}


def decode(s):
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        d = s[i + 1]
        if d in ESC:
            out.append(ESC[d])
            i += 2
        elif d == "x":
            out.append(chr(int(s[i + 2:i + 4], 16)))
            i += 4
        elif d == "u":
            j = s.index("}", i)
            out.append(chr(int(s[i + 3:j].replace("_", ""), 16)))
            i = j + 1
        elif d == "\n" or d == "\r":
            i += 2
            while i < len(s) and s[i] in " \t\r\n":
                i += 1
        else:
            raise ValueError("escape " + repr(s[i:i + 4]))
    return "".join(out)


def callee_of(toks, i):
    """Name of the call or macro that the `(` / `{` / `[` at toks[i] opens ('' if none)."""
    if i == 0:
        return ""
    prev = toks[i - 1]
    if prev[1] == "!" and i >= 2 and toks[i - 2][0] == "id":
        return toks[i - 2][1] + "!"
    if prev[0] == "id":
        if i >= 3 and toks[i - 2][1] == "::" and toks[i - 3][0] == "id":
            return toks[i - 3][1] + "::" + prev[1]
        return prev[1]
    return ""


def skip_item(toks, i):
    """Index after the item that starts at toks[i] (its first `{...}` block or `;`)."""
    depth = 0
    while i < len(toks):
        t = toks[i][1]
        if toks[i][0] == "p":
            if t in "([":
                depth += 1
            elif t in ")]":
                depth -= 1
            elif t == "{" and depth == 0:
                level = 0
                while i < len(toks):
                    if toks[i][0] == "p":
                        level += toks[i][1] == "{"
                        level -= toks[i][1] == "}"
                        if level == 0:
                            return i + 1
                    i += 1
                return i
            elif t == ";" and depth == 0:
                return i + 1
        i += 1
    return i


def literals(toks, filtered):
    """The string literals of one file: [(value, line)]. `filtered` drops the ones in test items,
    attributes, logs, asserts, keys and comparisons."""
    out, stack, i, n = [], [], 0, len(toks)
    attr_words = None  # idents of the attribute being read
    skip_next = False
    while i < n:
        kind, text, line = toks[i]
        if kind == "p" and text == "#" and i + 1 < n and toks[i + 1][1] in ("[", "!"):
            j = i + 1 + (toks[i + 1][1] == "!")
            if j < n and toks[j][1] == "[":
                depth, k = 0, j
                words = []
                while k < n:
                    if toks[k][1] == "[" and toks[k][0] == "p":
                        depth += 1
                    elif toks[k][1] == "]" and toks[k][0] == "p":
                        depth -= 1
                        if depth == 0:
                            break
                    elif toks[k][0] == "id":
                        words.append(toks[k][1])
                    k += 1
                if filtered and "test" in words and "not" not in words and "feature" not in words:
                    skip_next = True
                i = k + 1
                continue
        if skip_next:
            skip_next = False
            i = skip_item(toks, i)
            continue
        if kind == "p" and text in "([{":
            stack.append(callee_of(toks, i) if text != "{" or toks[i - 1][1] == "!" else "{")
        elif kind == "p" and text in ")]}":
            if stack:
                stack.pop()
        elif kind == "str":
            if not filtered or keep_context(toks, i, stack):
                out.append((text, line))
        i += 1
    return out


def keep_context(toks, i, stack):
    if any(c in SKIP_CALLEES for c in stack):
        return False
    prev = toks[i - 1][1] if i else ""
    nxt = toks[i + 1][1] if i + 1 < len(toks) else ""
    if prev in ("==", "!=") or nxt in ("==", "!=", "=>", "|"):
        return False
    if prev == "|" and i >= 2 and toks[i - 2][0] == "str":
        return False
    return True


PLACEHOLDER = re.compile(r"\{[^{}]*\}")


def ui_text(s):
    """Does the literal read like words shown to the user (not a key, path, id, glyph or a bare
    format placeholder)? Placeholders are looked past: `tui.keymap.{action}` is an id, `{} hooks` is text."""
    if len(s) < 2 or not any(c.isalpha() for c in s):
        return False
    bare = PLACEHOLDER.sub("x", s)
    if not any(len(w) >= 2 for w in re.findall(r"[^\W\d_]+", PLACEHOLDER.sub(" ", s))):
        return False
    if not any(c.isspace() for c in s):
        if IDENT_LIKE.fullmatch(bare) or CAMEL.fullmatch(bare):
            return False
        if "_" in s and SCREAMING.fullmatch(bare):
            return False
    if s.startswith(("--", "http://", "https://")) and " " not in s:
        return False
    return True


def rust_literal(s):
    out = ['"']
    for ch in s:
        cat = unicodedata.category(ch)
        if ch == '"' or ch == "\\":
            out.append("\\" + ch)
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif cat in ("Cc", "Cf", "Zl", "Zp", "Cs", "Co") or (cat == "Zs" and ch != " "):
            out.append("\\u{%x}" % ord(ch))
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def package_of(rel):
    for pattern, _status, owner, _where in modules.MODULES:
        if re.search(pattern, rel):
            return OWNER_TO_PACKAGE.get(owner)
    return None


def port_literals():
    have = set()
    for sub in ("src", "core/src", "host/src"):
        for path in sorted((PORT / sub).rglob("*.rs")):
            if "oracle_strings" in path.parts:
                continue
            for value, _ in literals(tokenize(path.read_text()), False):
                have.add(value)
                have.add(value.strip())
    return have


def module_word(rel):
    p = Path(rel)
    stem = p.stem if p.stem != "mod" else p.parent.name
    return re.sub(r"[^A-Za-z0-9]+", "_", stem).strip("_").upper() or "MOD"


def const_name(rel, value, used):
    plain = re.sub(r"\{[^{}]*\}", " ", value)
    words = re.findall(r"[A-Za-z0-9]+", plain)[:6]
    name = module_word(rel)
    if words:
        name += "_" + "_".join(w.upper() for w in words)
    while len(name) > 72 and "_" in name:
        name = name.rsplit("_", 1)[0]
    if name[0].isdigit():
        name = "S_" + name
    base, k = name, 1
    while name in used:
        k += 1
        name = f"{base}_{k}"
    used.add(name)
    return name


def main():
    have = port_literals()
    rows = {p: {} for p in PACKAGES}  # package -> value -> [(rel, line)]
    stats = {"files": 0, "raw": 0, "unmapped": 0, "ported": 0}
    for path in sorted(ORACLE.rglob("*.rs")):
        rel = path.relative_to(ORACLE).as_posix()
        if TEST_FILE.search(rel):
            continue
        pkg = package_of(rel)
        if pkg is None:
            stats["unmapped"] += 1
            continue
        stats["files"] += 1
        for value, line in literals(tokenize(path.read_text()), True):
            stats["raw"] += 1
            if not ui_text(value):
                continue
            if value in have or value.strip() in have:
                stats["ported"] += 1
                continue
            rows[pkg].setdefault(value, []).append((rel, line))
    OUT.mkdir(parents=True, exist_ok=True)
    counts = {}
    for pkg in PACKAGES:
        items = sorted(((sites[0], value, len(sites)) for value, sites in rows[pkg].items()))
        counts[pkg] = len(items)
        used = set()
        lines = [
            "//! generated by tools/rtwk-port/oracle-strings/gen.py, do not edit",
            f"//! The oracle's UI literals the `{pkg}` package still misses ({len(items)}); a porter",
            "//! uses a constant from the view's `strings.rs` and the next run drops it here.",
            "",
        ]
        current = None
        for (rel, line), value, extra in items:
            if rel != current:
                if current is not None:
                    lines.append("")
                lines.append(f"// ---- {rel}")
                current = rel
            more = f" (+{extra - 1} more)" if extra > 1 else ""
            lines.append(
                f"pub const {const_name(rel, value, used)}: &str = {rust_literal(value)};  // oracle {rel}:{line}{more}"
            )
        (OUT / f"{pkg}.rs").write_text("\n".join(lines) + "\n")
    mod = [
        "//! generated by tools/rtwk-port/oracle-strings/gen.py, do not edit",
        "//! Oracle UI literals per W2 package, waiting to be wired into the views' `strings.rs`.",
        "",
    ]
    for pkg in sorted(PACKAGES):
        mod += ["#[rustfmt::skip]", f"pub mod {pkg};"]
    (OUT / "mod.rs").write_text("\n".join(mod) + "\n")
    print("oracle files:", stats["files"], "literals:", stats["raw"], "already in the port:", stats["ported"])
    for pkg in PACKAGES:
        print(f"{pkg}: {counts[pkg]}")
    print("total:", sum(counts.values()))


if __name__ == "__main__":
    main()
