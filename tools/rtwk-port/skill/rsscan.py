"""A small Rust scanner: enough to index public items and inherent methods.

Not a parser. It masks comments and string literals so that braces and
keywords can be matched on code only, then slices the original text.
Python standard library only.
"""
import re


def mask(text):
    """Same length as text; comment and literal contents blanked (newlines kept)."""
    out = list(text)
    n = len(text)
    i = 0

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == "r" and re.match(r'r#*"', text[i:i + 12]) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r'r(#*)"', text[i:])
            end = '"' + m.group(1)
            j = text.find(end, i + m.end())
            j = n if j < 0 else j + len(end)
            blank(i + m.end(), j - len(end))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\.[^']*|[^\\'])'", text[i:i + 12])
            if m:
                blank(i + 1, i + m.end() - 1)
                i += m.end()
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def match_close(masked, i):
    """Index just after the bracket that closes masked[i]."""
    pairs = {"{": "}", "(": ")", "[": "]"}
    o = masked[i]
    c = pairs[o]
    depth = 0
    for j in range(i, len(masked)):
        ch = masked[j]
        if ch == o:
            depth += 1
        elif ch == c:
            depth -= 1
            if depth == 0:
                return j + 1
    raise ValueError("unbalanced")


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def doc_before(text, pos):
    """The `///` lines directly above the item that starts at pos (attributes skipped)."""
    lines = text[:pos].split("\n")
    # lines[-1] is the partial line holding the item; look upwards
    k = len(lines) - 2
    docs = []
    while k >= 0:
        s = lines[k].strip()
        if s.startswith("///"):
            docs.append(s[3:].strip())
        elif s.startswith("#[") or s.startswith("#!["):
            pass
        elif s == "" or not s.startswith("//"):
            if s.startswith("#[") is False and not s.startswith("///"):
                break
        k -= 1
    docs.reverse()
    return docs


def first_sentence(docs):
    """First paragraph of a doc comment as one line, cut at the first full stop."""
    para = []
    for d in docs:
        if d == "":
            if para:
                break
            continue
        para.append(d)
    s = " ".join(para)
    s = re.sub(r"\s+", " ", s).strip()
    m = re.search(r"(?<=[a-z0-9\)\]`])\.(?:\s|$)", s)
    if m:
        s = s[: m.start() + 1]
    return s


def clean_doc(s):
    """Intra-doc links [`X`](Self::x) and [`X`] become `X`."""
    s = re.sub(r"\[`([^`]+)`\]\([^)]*\)", r"`\1`", s)
    s = re.sub(r"\[`([^`]+)`\]", r"`\1`", s)
    s = re.sub(r"\[([^\]]+)\]\((?:Self|crate|super)::[^)]*\)", r"\1", s)
    return s


ITEM_RE = re.compile(r"^[ \t]*pub(?:\([a-z]+\))?[ \t]+(struct|enum|trait|type|fn|const|static)[ \t]+(?:r#)?([A-Za-z_]\w*)", re.M)
IMPL_RE = re.compile(r"^[ \t]*impl(?P<g><(?:[^<>]|<[^<>]*>)*>)?[ \t]+(?P<ty>[^{;]*?)[ \t\n]*(?:where[^{;]*)?\{", re.M)
FN_RE = re.compile(r"^[ \t]*pub[ \t]+((?:const[ \t]+|async[ \t]+|unsafe[ \t]+)*)fn[ \t]+(?:r#)?([A-Za-z_]\w*)", re.M)


def signature(text, masked, start):
    """Text of a fn signature from its `pub` up to the body or `;`, whitespace folded."""
    depth = 0
    j = start
    n = len(masked)
    while j < n:
        ch = masked[j]
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        elif ch == "<" and masked[j - 1] != "-" and False:
            pass
        elif ch in "{;" and depth == 0:
            break
        j += 1
    sig = text[start:j]
    sig = re.sub(r"\s+", " ", sig).strip()
    sig = re.sub(r"\s*,\s*\)", ")", sig)
    sig = re.sub(r"\(\s+", "(", sig)
    sig = re.sub(r"\s+where\s+", " where ", sig)
    return sig


class Item:
    __slots__ = ("kind", "name", "crate", "path", "line", "doc", "methods", "variants", "fields", "generics")

    def __init__(self, kind, name, crate, path, line, doc):
        self.kind, self.name, self.crate, self.path, self.line, self.doc = kind, name, crate, path, line, doc
        self.methods = []   # (name, signature, doc line, line)
        self.variants = []  # names
        self.fields = []    # (name, type)
        self.generics = ""


def macro_instances(text, masked):
    """For a `macro_rules! m { ($name:ident, ..., $doc:literal) => { body } }` defined in this file and
    invoked as `m!(Name, ..., "doc")`, the body with `$name` and the doc put in, and the invocation's line."""
    out = []
    for md in re.finditer(r"macro_rules!\s*(\w+)\s*\{", masked):
        mname = md.group(1)
        try:
            mend = match_close(masked, md.end() - 1)
        except ValueError:
            continue
        arrow = masked.find("=>", md.end(), mend)
        if arrow < 0:
            continue
        ob = masked.find("{", arrow, mend)
        if ob < 0:
            continue
        try:
            bend = match_close(masked, ob)
        except ValueError:
            continue
        body = text[ob + 1:bend - 1]
        if "$name" not in body:
            continue
        for inv in re.finditer(r"(?m)^[ \t]*" + re.escape(mname) + r"!\s*[\(\{]", masked):
            try:
                close = match_close(masked, inv.end() - 1)
            except ValueError:
                continue
            args = text[inv.end():close - 1]
            fm = re.match(r"\s*([A-Za-z_]\w*)", args)
            if not fm:
                continue
            lits = re.findall(r'"((?:[^"\\]|\\.)*)"', args, re.S)
            doc = ""
            if lits:
                doc = lits[-1].replace("\\\n", "").replace("\\n", "\n")
                doc = re.sub(r"\n[ \t]*", "\n", doc)
            inst = body.replace("$name", fm.group(1))
            inst = re.sub(r"#\[doc = \$doc\]", "\n".join("/// " + l for l in doc.split("\n")), inst)
            out.append((inst, line_of(text, inv.start())))
    return out


def index_text(text, crate, path, items, impls, _inner=False, _line=None):
    """Collect public items (anywhere) and inherent impl methods of this file."""
    masked = mask(text)
    if not _inner:
        for inst, line in macro_instances(text, masked):
            sub_items, sub_impls = [], []
            index_text(inst, crate, path, sub_items, sub_impls, True, line)
            items.extend(sub_items)
            impls.extend(sub_impls)
    for m in ITEM_RE.finditer(masked):
        kind, name = m.group(1), m.group(2)
        if kind in ("fn", "const", "static"):
            # free items only when at column 0 (module level)
            if not re.match(r"pub", masked[m.start():m.start() + 3]):
                continue
        pos = m.start() + (len(masked[m.start():]) - len(masked[m.start():].lstrip()))
        it = Item(kind, name, crate, path, _line or line_of(text, pos), clean_doc(first_sentence(doc_before(text, pos))))
        if kind in ("struct", "enum"):
            body_start = None
            k = m.end()
            # skip generics
            g = re.match(r"\s*(<(?:[^<>]|<[^<>]*>)*>)?", masked[k:])
            it.generics = (g.group(1) or "") if g else ""
            k += g.end() if g else 0
            tail = masked[k:k + 400]
            b = re.search(r"[{(;]", tail)
            if b and tail[b.start()] == "{":
                body_start = k + b.start()
            if body_start is not None:
                try:
                    end = match_close(masked, body_start)
                except ValueError:
                    end = body_start
                body = masked[body_start + 1:end - 1]
                obody = text[body_start + 1:end - 1]
                if kind == "enum":
                    depth = 0
                    for lm, mm in enumerate(re.finditer(r"[^\n]*\n?", body)):
                        pass
                    # variants: identifiers at depth 0 beginning an entry
                    d = 0
                    entry = True
                    i = 0
                    while i < len(body):
                        ch = body[i]
                        if ch in "{([<":
                            d += 1
                        elif ch in "})]>":
                            d -= 1
                        elif ch == "," and d == 0:
                            entry = True
                        elif entry and d == 0 and (ch.isalpha() or ch == "_"):
                            vm = re.match(r"[A-Za-z_]\w*", body[i:])
                            it.variants.append(vm.group(0))
                            i += vm.end()
                            entry = False
                            continue
                        i += 1
                else:
                    for fm in re.finditer(r"(?m)^[ \t]*pub[ \t]+(\w+)[ \t]*:[ \t]*([^,\n]+)", body):
                        it.fields.append((fm.group(1), re.sub(r"\s+", " ", obody[fm.start(2) - 0:fm.end(2)].strip()) if False else fm.group(2).strip()))
        items.append(it)
    for m in IMPL_RE.finditer(masked):
        ty = m.group("ty").strip()
        if " for " in " " + ty + " " or ty.startswith("!"):
            continue
        tm = re.match(r"(?:[\w:]+::)?([A-Za-z_]\w*)", ty)
        if not tm:
            continue
        name = tm.group(1)
        open_i = m.end() - 1
        try:
            end = match_close(masked, open_i)
        except ValueError:
            continue
        body_m = masked[open_i:end]
        for fm in FN_RE.finditer(body_m):
            # direct children of the impl only: depth 1
            before = body_m[:fm.start()]
            if before.count("{") - before.count("}") != 1:
                continue
            abs_start = open_i + fm.start()
            abs_start += len(masked[abs_start:]) - len(masked[abs_start:].lstrip())
            sig = signature(text, masked, abs_start)
            impls.append((name, crate, path, sig, fm.group(2), clean_doc(first_sentence(doc_before(text, abs_start))), _line or line_of(text, abs_start), m.group("g") or ""))
