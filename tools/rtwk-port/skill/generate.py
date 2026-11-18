#!/usr/bin/env python3
"""Writes .claude/skills/rtwk/ (SKILL.md and references/) from the pinned rtwk.

    python3 -I tools/rtwk-port/skill/generate.py            # write
    python3 -I tools/rtwk-port/skill/generate.py --check    # exit 1 when stale

Reads: the pin's crates/rtwk/SKILL.md (copied to references/guides/getting-started.md), docs/guide,
crates/*/src (API index),
crates/rtwk-gallery (catalog, imports) and tools/rtwk-port/skill/gallery-codes.jsonl
(the gallery's Code-tab Rust and options, from refresh-codes.sh);
authored inputs: tools/rtwk-port/skill/{skill.src.md (the whole SKILL.md), widgets-by-need.src.md,
recipes.src.md, snippets/*.rs}. Python standard library only.
"""
import argparse
import glob
import hashlib
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rsscan  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
DEFAULT_PIN = os.environ.get("RTWK_PIN", os.path.expanduser("~/.cache/codexcli/rtwk-pin"))
OUT_REL = ".claude/skills/rtwk"
GEN = "tools/rtwk-port/skill/generate.py"

# crate -> the facade module a name defined there is reached by, when no `use rtwk::..` names it
FACADE_OF_CRATE = {
    "rtwk_controls": "rtwk::controls", "rtwk_data": "rtwk::data", "rtwk_editing": "rtwk::editing",
    "rtwk_overlay": "rtwk::overlay", "rtwk_provider": "rtwk::provider", "rtwk_scroll": "rtwk::scroll",
    "rtwk_cli": "rtwk::cli", "rtwk_hooks": "rtwk::hooks", "rtwk_animation": "rtwk::animation",
    "rtwk_rich": "rtwk_rich", "rtwk_test": "rtwk_test", "rtwk_tokio": "rtwk_tokio", "rtwk_term": "rtwk_term",
    "rtwk_overlay_base": "rtwk::overlay", "rtwk_state": "rtwk_state", "rtwk_core": "rtwk_core", "rtwk_render": "rtwk_render",
}
# widgets whose catalog title is not the name of their type
TITLE_ALIAS = {
    "Rich text": ["Span", "Text"], "Right to left": ["TextOrder"], "ANSI": ["AnsiStream"],
    "Window title": [], "Notifications": [], "Static": ["StaticOutput"], "Viewport": [],
    "Repaint rainbow": [], "Image": ["ImageWidget"], "Cli": ["Cli"], "Effect": ["Effect"],
    "Console": ["Console"], "Theme": ["Theme", "ThemeData"], "Table": ["Table", "TableColumn", "TableController"],
    "Navigator": ["Navigator", "PageRoute"], "Overlay": ["Overlay"], "App": ["App"],
    "Dialog": ["Dialog", "DialogOptions"], "Toast": ["ToastOptions"], "Calendar": ["Calendar", "CalendarController"],
    "TextArea": ["TextArea", "TextEditingController"], "TextField": ["TextField", "TextEditingController"],
    "Row, Column, Flex": ["Row", "Column", "Flex"], "KeyBindings, LeaderKey": ["KeyBindings", "LeaderKey"],
    "IntrinsicWidth/Height": ["IntrinsicWidth", "IntrinsicHeight"],
    "Box": ["BoxWidget"], "Gauge, LineGauge": ["Gauge", "LineGauge"],
}


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def pin_id(pin):
    try:
        return subprocess.check_output(["git", "-C", pin, "rev-parse", "--short=8", "HEAD"], text=True,
                                       stderr=subprocess.DEVNULL).strip()
    except Exception:
        return "unknown"


# ---------------------------------------------------------------- the API index

class Index:
    def __init__(self, pin):
        self.pin = pin
        self.items = {}      # name -> [Item]
        self.methods = {}    # (crate, name) -> [(sig, fname, doc, line, path)]
        self.imports = {}    # name -> 'rtwk::module::Name'
        crates = sorted(glob.glob(os.path.join(pin, "crates", "*")))
        for c in crates:
            cname = os.path.basename(c).replace("-", "_")
            if cname in ("rtwk_gallery", "rtwk_dev", "rtwk_e2e", "rtwk_web", "rtwk_ssh"):
                continue
            for f in sorted(glob.glob(os.path.join(c, "src", "**", "*.rs"), recursive=True)):
                rel = os.path.relpath(f, pin)
                if "/tests/" in rel or rel.endswith("_tests.rs") or rel.endswith("/tests.rs"):
                    continue
                its, imps = [], []
                rsscan.index_text(read(f), cname, rel, its, imps)
                for it in its:
                    self.items.setdefault(it.name, []).append(it)
                for (name, crate, path, sig, fname, doc, line, _g) in imps:
                    self.methods.setdefault((crate, name), []).append((sig, fname, doc, line, path))
        self._scan_imports()

    def _scan_imports(self):
        files = glob.glob(os.path.join(self.pin, "crates", "rtwk-gallery", "src", "**", "*.rs"), recursive=True)
        files += glob.glob(os.path.join(self.pin, "crates", "rtwk-gallery", "tests", "code", "**", "*.rs"), recursive=True)
        files += glob.glob(os.path.join(self.pin, "crates", "rtwk", "examples", "**", "*.rs"), recursive=True)
        files += [os.path.join(self.pin, "crates", "rtwk", "SKILL.md")]
        for f in sorted(files):
            text = read(f)
            for m in re.finditer(r"\buse\s+(rtwk(?:_[a-z]+)?::[^;]+);", text):
                for path in expand_use(m.group(1)):
                    segs = path.split("::")
                    name = segs[-1]
                    if name in ("self", "*") or not name[:1].isupper():
                        continue
                    old = self.imports.get(name)
                    if old is None or (path.startswith("rtwk::") and not old.startswith("rtwk::")):
                        self.imports[name] = path

    def def_of(self, name, prefer_crate=None):
        cands = [i for i in self.items.get(name, []) if i.kind in ("struct", "enum", "trait", "type")]
        if not cands:
            return None
        path = self.imports.get(name)
        if path:
            fac = path.split("::")[1] if path.startswith("rtwk::") else None
            for i in cands:
                if fac and FACADE_OF_CRATE.get(i.crate, "").endswith("::" + fac):
                    return i
        if prefer_crate:
            for i in cands:
                if i.crate == prefer_crate:
                    return i
        cands.sort(key=lambda i: (-len(self.methods.get((i.crate, i.name), [])), i.path))
        return cands[0]

    def facade_path(self, item):
        p = self.imports.get(item.name)
        if p and p.startswith("rtwk"):
            return p, True
        fac = FACADE_OF_CRATE.get(item.crate)
        if fac:
            return "%s::%s" % (fac, item.name), False
        # rtwk_widgets / rtwk_framework: the first module under src
        parts = item.path.split("/")
        mod = None
        if len(parts) >= 4:
            mod = parts[3][:-3] if parts[3].endswith(".rs") else parts[3]
        if item.crate == "rtwk_framework" and (mod in (None, "widget", "cx", "key", "handler", "obs", "strs", "element")):
            return "rtwk::%s" % item.name, False
        return "rtwk::%s::%s" % (mod or "?", item.name), False


def expand_use(s):
    """`rtwk::a::{B, c::D, E as F}` -> full paths."""
    s = re.sub(r"\s+", "", s)
    out = []

    def rec(prefix, rest):
        i = rest.find("{")
        if i < 0:
            for part in rest.split(","):
                if part:
                    out.append(prefix + part.split("as")[0] if "as" in part and part.split("as")[0] else prefix + part)
            return
        pre = rest[:i]
        depth, j = 0, i
        k = i
        while k < len(rest):
            if rest[k] == "{":
                depth += 1
            elif rest[k] == "}":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        inner = rest[i + 1:k]
        parts, d, cur = [], 0, ""
        for ch in inner:
            if ch == "{":
                d += 1
            if ch == "}":
                d -= 1
            if ch == "," and d == 0:
                parts.append(cur)
                cur = ""
            else:
                cur += ch
        if cur:
            parts.append(cur)
        for p in parts:
            rec(prefix + pre, p)

    rec("", s)
    return out


# ---------------------------------------------------------------- the gallery

def parse_catalog(pin):
    """The gallery catalog: groups and their pages. A group's `pages: &[...]` may hold one page
    (`&[CatalogPage {`) or many, so groups are cut at `CatalogGroup {` and pages found in the rest."""
    t = read(os.path.join(pin, "crates/rtwk-gallery/src/reference_catalog.rs"))
    t = t[t.index("pub(crate) const CATALOG"):]
    groups = []
    for chunk in t.split("CatalogGroup {")[1:]:
        gt = re.search(r'title: "([^"]+)"', chunk)
        gs = re.search(r'\n\s*slug: "([^"]+)",\s*pages:', chunk)
        if not gt or not gs:
            raise SystemExit("catalog: a group without title or slug; the parser needs updating")
        pages = []
        for p in re.finditer(r'title: "((?:[^"\\]|\\.)*)",\s*about: "((?:[^"\\]|\\.)*)",\s*slug: "([^"]+)",\s*route: "([^"]+)"', chunk):
            pages.append({"title": unesc(p.group(1)), "about": unesc(p.group(2)), "slug": p.group(3), "route": p.group(4)})
        groups.append({"title": gt.group(1), "slug": gs.group(1), "pages": pages})
    n_routes = len(re.findall(r'\broute: "', t))
    if sum(len(g["pages"]) for g in groups) != n_routes:
        raise SystemExit("catalog: parsed %d pages but the file has %d routes" % (sum(len(g["pages"]) for g in groups), n_routes))
    return groups


def unesc(s):
    return s.replace('\\"', '"').replace("\\\\", "\\")


def load_codes(path):
    rows = {}
    head = None
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f):
            d = json.loads(line)
            if n == 0:
                head = d
            else:
                rows[d["route"]] = d
    return head, rows


def inputs_hash(pin):
    h = hashlib.sha256()
    base = os.path.join(pin, "crates", "rtwk-gallery")
    files = sorted(glob.glob(os.path.join(base, "src", "**", "*.rs"), recursive=True))
    files += sorted(glob.glob(os.path.join(base, "tests", "code", "**", "*.rs"), recursive=True))
    for f in files:
        h.update(os.path.relpath(f, base).encode())
        h.update(b"\0")
        h.update(read(f).encode())
        h.update(b"\0")
    return h.hexdigest()[:16]


def page_sources(pin):
    """route -> 'crates/rtwk-gallery/src/pages/x.rs:LINE' (where the page is defined)."""
    out = {}
    for f in sorted(glob.glob(os.path.join(pin, "crates/rtwk-gallery/src/pages/*.rs"))):
        t = read(f)
        for m in re.finditer(r'"(#/[a-z0-9-]+/[a-z0-9-]+)"', t):
            r = m.group(1)
            if r not in out and "?" not in r:
                out[r] = "%s:%d" % (os.path.relpath(f, pin), rsscan.line_of(t, m.start()))
    return out


# ---------------------------------------------------------------- rendering

def header(pid, extra=""):
    return "<!-- Written by %s from rtwk %s; do not edit.%s -->\n" % (GEN, pid, extra)


def md_escape_cell(s):
    return s.replace("|", "\\|")


def fmt_values(values, kind):
    if kind.startswith("slider"):
        return kind.replace("slider ", "") + " (a slider)"
    if kind == "switch":
        return "on, off"
    if len(values) > 12:
        return "%s, ... %s (%d values)" % (", ".join(values[:5]), ", ".join(values[-2:]), len(values))
    return ", ".join(values)


def title_types(index, title):
    names = []
    if title in TITLE_ALIAS:
        names = list(TITLE_ALIAS[title])
    else:
        toks = [t for t in re.split(r"[,/ ]+", title) if t]
        for t in toks:
            names.append(t)
    out = []
    for n in names:
        if index.def_of(n) and n not in out:
            out.append(n)
    return out


COMMON = {"Text", "Theme", "ThemeData", "SizedBox", "Column", "Row", "AnyWidget", "Handler", "Some", "None", "Ok", "Err",
          "String", "Vec", "Rc", "Option", "Box", "Self", "Span", "Style", "Key", "Message", "Cx", "ViewCx", "BuildCx",
          "State", "StatefulWidget", "StatelessWidget", "Rebuild", "Duration", "Instant", "BoxWidget", "Border", "BorderType",
          "EdgeInsets", "MainAxisSize", "CrossAxisAlignment", "MainAxisAlignment", "Color", "Attrs", "Widget"}


def code_types(index, code, skip):
    """Types the code names, in order of first use: (name, def or None, path)."""
    out = []
    seen = set()
    for m in re.finditer(r"\b([A-Z][A-Za-z0-9]+)\b(?=::|\s*\{|\()", code):
        n = m.group(1)
        if n in skip or n in seen:
            continue
        seen.add(n)
        # only a name the gallery imports from rtwk: a type the example declares itself is not rtwk's
        if n in index.imports:
            out.append((n, index.def_of(n)))
    return out


def method_kind(sig):
    m = re.match(r"pub (?:const |async |unsafe )*fn \w+(?:<.*?>)?\((.*)", sig)
    args = m.group(1) if m else ""
    if re.match(r"\s*(mut )?self\b", args):
        return "builder"
    if re.match(r"\s*&(mut )?self\b", args):
        return "method"
    return "constructor"


def short(doc, n=220):
    return doc if len(doc) <= n else doc[:n].rsplit(" ", 1)[0] + " ..."


def type_line(index, name, item):
    """One list line for a type: facade path, kind, source, doc; variants and fields below."""
    if item is None:
        return ["- `%s` (no API listing: defined by a macro or outside the index)" % index.imports.get(name, name)]
    fp, ok = index.facade_path(item)
    lines = []
    doc = (" " + short(item.doc)) if item.doc else ""
    lines.append("- `%s`%s (%s, `%s:%d`)%s" % (fp, "" if ok else " [path inferred]", item.kind, item.path, item.line, doc))
    if item.kind == "enum" and item.variants:
        lines.append("  - variants: %s" % ", ".join("`%s`" % v for v in item.variants))
    if item.kind == "struct" and item.fields:
        lines.append("  - public fields: %s" % ", ".join("`%s: %s`" % f for f in item.fields[:20]))
    return lines


def method_lines(index, item):
    """The public inherent methods of a type, in three groups."""
    ms = index.methods.get((item.crate, item.name), [])
    seen = set()
    groups = {"constructor": [], "builder": [], "method": []}
    for sig, _fname, mdoc, _line, _path in ms:
        if sig in seen:
            continue
        seen.add(sig)
        groups[method_kind(sig)].append((sig, mdoc))
    titles = {"constructor": "constructors and associated functions", "builder": "builder methods (take and return `self`)",
              "method": "methods"}
    lines = []
    for k in ("constructor", "builder", "method"):
        if not groups[k]:
            continue
        lines.append("#### `%s`: %s" % (item.name, titles[k]))
        lines.append("")
        for sig, mdoc in groups[k]:
            lines.append("- `%s`%s" % (sig, (" - " + short(mdoc)) if mdoc else ""))
        lines.append("")
    return lines


def render_type(index, item, full=True):
    lines = type_line(index, item.name, item)
    if full:
        lines.append("")
        lines.extend(method_lines(index, item))
    return lines


def render_page(index, page, row, src, pid):
    out = ["## %s" % page["title"], "", page["about"], ""]
    prim = title_types(index, page["title"])
    skip = set(prim)
    used = code_types(index, row["code"], skip) if row else []
    if not prim and used:
        # the catalog title names no type: the types of the example that carry the page
        picks = [(n, d) for n, d in used if n not in COMMON][:3]
        prim = [n for n, d in picks if d]
        skip |= set(prim)
        used = [(n, d) for n, d in used if n not in skip]
    out.append("API:")
    out.append("")
    for n in prim:
        out.extend(type_line(index, n, index.def_of(n)))
    out.append("")
    if row:
        out.append("### Code")
        out.append("")
        out.append("The gallery's Code tab at the initial options (route `%s`, `%s`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`." % (page["route"], src or "?"))
        out.append("")
        out.append("```rust")
        out.append(row["code"].rstrip("\n"))
        out.append("```")
        out.append("")
        if row["options"]:
            out.append("### Options")
            out.append("")
            out.append("The gallery's inspector; each value is one setting of the example above.")
            out.append("")
            out.append("| option | values | starts at |")
            out.append("| --- | --- | --- |")
            for o in row["options"]:
                vals = o["values"]
                init = vals[o["initial"]] if vals and o["initial"] < len(vals) else str(o["initial"])
                out.append("| %s | %s | %s |" % (md_escape_cell(o["key"]), md_escape_cell(fmt_values(vals, o["kind"])), md_escape_cell(init)))
            out.append("")
    ml = []
    for n in prim:
        d = index.def_of(n)
        if d:
            ml.extend(method_lines(index, d))
    if ml:
        out.append("### Methods")
        out.append("")
        out.extend(ml)
    rel = [(n, d) for n, d in used if n not in skip and n not in COMMON]
    if rel:
        out.append("### Also used in the code")
        out.append("")
        for n, d in rel:
            out.extend(type_line(index, n, d))
        out.append("")
    return "\n".join(out).rstrip("\n") + "\n"


def gen_widgets(index, groups, codes, srcs, pid, report):
    files = {}
    top = [header(pid), "# rtwk widgets", "",
           "Every widget of the gallery catalog, one line each: its name and what it is, by group. A group's file has each widget's API paths, the Rust that builds it, the inspector options and the builder methods. The catalog is twk's, ported: `ported` means a page of `crates/rtwk-gallery`.", ""]
    for g in groups:
        top.append("## %s" % g["title"])
        top.append("")
        top.append("File: `widgets/%s.md`." % g["slug"])
        top.append("")
        for p in g["pages"]:
            top.append("- %s: %s" % (p["title"], p["about"]))
        top.append("")
        body = [header(pid), "# %s" % g["title"], "", "[All widgets](../widgets.md)", ""]
        for p in g["pages"]:
            row = codes.get(p["route"])
            if not row:
                report.append("no code row: %s" % p["route"])
            prim = title_types(index, p["title"])
            if not prim:
                report.append("no API type for the title: %s" % p["title"])
            body.append(render_page(index, p, row, srcs.get(p["route"]), pid))
        files["references/widgets/%s.md" % g["slug"]] = "\n".join(body)
    files["references/widgets.md"] = "\n".join(top).rstrip("\n") + "\n"
    return files


def gen_guides(pin, pid):
    files = {}
    for f in sorted(glob.glob(os.path.join(pin, "docs", "guide", "*.md"))):
        name = os.path.basename(f)
        files["references/guides/%s" % name] = header(pid, " Copied from docs/guide/%s." % name) + read(f)
    return files


def gen_api(index, pid, report):
    """guides/app-api.md and guides/testing-api.md: the public methods of the app and test contexts."""
    def section(names, intro, title):
        out = [header(pid), "# %s" % title, "", intro, ""]
        for n in names:
            d = index.def_of(n)
            if not d:
                report.append("api guide: type not found: %s" % n)
                continue
            out.append("## %s" % n)
            out.append("")
            out.extend(render_type(index, d, full=True))
            out.append("")
        return "\n".join(out).rstrip("\n") + "\n"
    files = {}
    files["references/guides/app-api.md"] = section(
        ["AppView", "ViewCx", "AppCx", "Cx", "BuildCx", "UpdateCx", "Handler", "Obs", "Navigator", "PageRoute"],
        "The application context types: what a view function, a handler and a `State` can call. Signatures are from the source; the one line is its doc comment.",
        "App and component contexts")
    files["references/guides/testing-api.md"] = section(
        ["Tester", "Options"],
        "`rtwk_test`: drive an app headless and read the screen. `find::*` finders are free functions in `rtwk_test::find`.",
        "Testing API: rtwk_test")
    # finders
    fl = [d for d in index.items.get("by_type", [])]
    return files


STD_NAMES = {"Rc", "Arc", "Vec", "String", "Option", "Some", "None", "Ok", "Err", "Result", "Box", "Cell",
             "RefCell", "Duration", "ExitCode", "Default", "Clone", "PartialEq", "Eq", "Fn", "FnOnce", "Self"}


def check_names(index, pin, text, not_rtwk):
    """The rtwk names skill.src.md writes that the pin lacks: CamelCase names (and `Type::member`) in
    inline code, in the `+` lines of diff fences and in table cells after the first. Rust fences come
    from snippets, which compile-snippets.sh compiles; a `-` line or a table's first cell is the
    Flutter or ratatui side; `<!-- not rtwk: ... -->` lists the other names written on purpose."""
    spans = []
    fence = None
    for ln in text.split("\n"):
        if ln.startswith("```"):
            fence = None if fence is not None else ln[3:].strip()
            continue
        if fence is not None:
            if fence == "diff" and ln.startswith("+"):
                spans.append(ln[1:])
            continue
        if ln.startswith("|"):
            cells = ln.strip("|").split("|")
            ln = "|".join(cells[1:])
        spans.extend(re.findall(r"`([^`]+)`", ln))
    unknown = []
    file_text = {}
    def has_member(items, member):
        for it in items:
            if member in it.variants or any(mm[0] == member for mm in it.methods):
                return True
            if any(mm[1] == member for mm in index.methods.get((it.crate, it.name), [])):
                return True
            t = file_text.setdefault(it.path, read(os.path.join(pin, it.path)))
            if re.search(r"\b%s\b" % re.escape(member), t):
                return True
        return False
    seen = set()
    for sp in spans:
        if "/" in sp or sp.startswith("#"):  # a path or a heading, not code
            continue
        for mm in re.finditer(r"(?<![\w:])([A-Z][A-Za-z0-9]*[a-z][A-Za-z0-9]*|[A-Z]{2,}[a-z]\w*)(?:::([A-Za-z_]\w*))?", sp):
            name, member = mm.group(1), mm.group(2)
            if name in STD_NAMES or name in not_rtwk or (name, member) in seen:
                continue
            seen.add((name, member))
            items = index.items.get(name, [])
            if not items:
                unknown.append("unknown name %s (in `%s`)" % (name, sp.strip()))
            elif member and not has_member(items, member):
                unknown.append("unknown member %s::%s (in `%s`)" % (name, member, sp.strip()))
    return unknown


def expand_directives(text, base, index, report, snippets_dir):
    def snip(m):
        name, part = m.group(1), m.group(2)
        p = os.path.join(snippets_dir, name + ".rs")
        if not os.path.exists(p):
            report.append("missing snippet %s" % name)
            return "(missing snippet %s)" % name
        code = read(p)
        if part:  # the lines after `// skill: <part>` up to the next `// skill:` marker
            parts = re.split(r"^// skill: (\w+)\n", code, flags=re.M)
            code = dict(zip(parts[1::2], parts[2::2])).get(part)
            if code is None:
                report.append("missing part %s of snippet %s" % (part, name))
                return "(missing part %s of snippet %s)" % (part, name)
        return "```rust\n%s\n```" % code.rstrip("\n")
    text = re.sub(r"\{\{snippet ([\w-]+)(?:#(\w+))?\}\}", snip, text)

    def ref(m):
        name = m.group(1)
        d = index.def_of(name)
        if not d:
            report.append("by-need: unknown widget %s" % name)
            return "`%s`" % name
        fp, ok = index.facade_path(d)
        return "`%s`" % fp if fp.endswith("::" + name) or fp == name else "`%s`" % name
    text = re.sub(r"\[\[([A-Za-z0-9_]+)\]\]", ref, text)
    return text


def build(pin, ref_file=None):
    pid = pin_id(pin)
    report = []
    files = {}
    index = Index(pin)
    groups = parse_catalog(pin)
    head, codes = load_codes(os.path.join(HERE, "gallery-codes.jsonl"))
    srcs = page_sources(pin)
    files.update(gen_widgets(index, groups, codes, srcs, pid, report))
    files.update(gen_guides(pin, pid))
    files.update(gen_api(index, pid, report))
    snippets_dir = os.path.join(HERE, "snippets")
    for src_name, dest in (("recipes.src.md", "references/guides/recipes.md"), ("widgets-by-need.src.md", "references/guides/widgets-by-need.md")):
        p = os.path.join(HERE, src_name)
        if os.path.exists(p):
            files[dest] = header(pid, " Source: tools/rtwk-port/skill/%s." % src_name) + expand_directives(read(p), HERE, index, report, snippets_dir)
    # SKILL.md: authored whole in skill.src.md (twk's skill's shape); rtwk's own guide goes to
    # references/guides/getting-started.md, verbatim but for its frontmatter
    skill = read(os.path.join(pin, "crates/rtwk/SKILL.md"))
    m = re.match(r"(---\n.*?\n---\n)(.*)", skill, re.S)
    files["references/guides/getting-started.md"] = (
        header(pid, " Copied from crates/rtwk/SKILL.md, rtwk's own guide for agents.") + m.group(2).lstrip("\n"))
    problems = []
    src = read(os.path.join(HERE, "skill.src.md"))
    m = re.match(r"(---\n.*?\n---\n)(.*)", src, re.S)
    front, body = m.group(1), m.group(2)
    not_rtwk = set()
    def take_not_rtwk(mm):
        not_rtwk.update(mm.group(1).split())
        return ""
    body = re.sub(r"<!-- not rtwk: (.*?) -->\n", take_not_rtwk, body, flags=re.S)
    n_pages = sum(len(g["pages"]) for g in groups)
    n_heads = 0  # page headings, outside code fences (a Code tab may hold Markdown with "## ")
    for k, t in files.items():
        if k.startswith("references/widgets/"):
            fenced = False
            for ln in t.split("\n"):
                if ln.startswith("```"):
                    fenced = not fenced
                elif not fenced and ln.startswith("## "):
                    n_heads += 1
    if n_heads != n_pages:
        problems.append("widgets/*.md has %d page headings for %d catalog pages" % (n_heads, n_pages))
    body = body.replace("{{pages}}", str(n_pages)).replace("{{groups}}", ", ".join(g["slug"] for g in groups))
    body = body.replace("{{n_groups}}", str(len(groups)))
    problems.extend("skill.src.md: %s" % u for u in check_names(index, pin, body, not_rtwk))
    body = expand_directives(body, HERE, index, report, snippets_dir)
    problems.extend("skill.src.md: %s" % ln for ln in re.findall(r"\(missing [^)]*\)", body))
    files["SKILL.md"] = (front + "<!-- rtwk %s. Written by %s from tools/rtwk-port/skill/skill.src.md and snippets/; do not edit. -->\n" % (pid, GEN)
                         + body.rstrip("\n") + "\n")
    # freshness inputs
    if head["pin"] != pid:
        problems.append("gallery-codes.jsonl is from pin %s, the pin is %s: run refresh-codes.sh" % (head["pin"], pid))
    elif head.get("inputs") != inputs_hash(pin):
        problems.append("gallery-codes.jsonl is stale against the pin's gallery sources: run refresh-codes.sh")
    ref_path = ref_file or os.path.join(ROOT, "docs/rtwk-port/RTWK_REF")
    if os.path.exists(ref_path):
        ref = read(ref_path).strip()
        if not ref.startswith(pid) and not pid.startswith(ref[:8]):
            problems.append("docs/rtwk-port/RTWK_REF names %s, the pin is %s" % (ref[:8], pid))
    return files, report, problems, pid


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pin", default=DEFAULT_PIN)
    ap.add_argument("--out", default=os.path.join(ROOT, OUT_REL))
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--ref-file", default=None, help="the RTWK_REF to compare with the pin (default docs/rtwk-port/RTWK_REF)")
    ap.add_argument("--print-inputs-hash", action="store_true")
    ap.add_argument("-v", "--verbose", action="store_true")
    a = ap.parse_args()
    if a.print_inputs_hash:
        print(inputs_hash(a.pin))
        return 0
    files, report, problems, pid = build(a.pin, a.ref_file)
    if a.verbose:
        for r in report:
            print("note:", r)
    if a.check:
        bad = []
        for rel, text in sorted(files.items()):
            p = os.path.join(a.out, rel)
            if not os.path.exists(p) or read(p) != text:
                bad.append(rel)
        existing = set()
        for dp, _dn, fn in os.walk(a.out):
            for f in fn:
                existing.add(os.path.relpath(os.path.join(dp, f), a.out))
        for rel in sorted(existing - set(files)):
            bad.append(rel + " (not generated)")
        for p in problems:
            print("STALE:", p)
        for b in bad:
            print("DIFFERS:", b)
        if problems or bad:
            print("run: python3 -I %s" % GEN)
            return 1
        print("rtwk skill: %d files match the pin %s" % (len(files), pid))
        return 0
    for p in problems:
        print("warning:", p)
    for rel, text in files.items():
        p = os.path.join(a.out, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8") as f:
            f.write(text)
    # drop files a former run wrote and this one does not
    for dp, _dn, fn in os.walk(a.out):
        for f in fn:
            rel = os.path.relpath(os.path.join(dp, f), a.out)
            if rel not in files:
                os.remove(os.path.join(dp, f))
    print("wrote %d files to %s (pin %s); %d notes" % (len(files), a.out, pid, len(report)))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
