"""Rewrites `impl Feature for X { .. }` of the named slices as `feature! { .. }` (fw-hubs-c5): the
constants and the associated types a slice has, in the macro's order, the absent ones left out
(`Never<Self>`, `()`), the methods in one braces group. A slice already converted is left as is;
one the script cannot convert safely (a method with a multi-line string) is named, not changed.
Run from the repository root: python3 -I tools/rtwk-port/registry/feature_macro.py copy daemon .."""
import pathlib
import re
import sys

FEATURES = pathlib.Path("codex-rs/tui-rtwk/core/src/features")
CONSTS = ["NAME", "SEED", "OPENS", "RUNS"]
TYPES = ["State", "ThreadPart", "Target", "Intent", "ThreadIntent", "Command", "Reply", "Ctx",
         "Note", "Fact", "Wish", "Interest", "Event"]
DEFAULT = {"State": "()", "ThreadPart": "()"}


def drop_absurd(methods):
    """`wire` and `decode` that only say `command.absurd()`: the macro writes them."""
    out, k = [], 0
    while k < len(methods):
        if re.match(r"    fn (wire|decode)\(", methods[k]):
            e = k
            while methods[e] != "    }":
                e += 1
            seg = "\n".join(methods[k:e + 1])
            if "&Never<Self>" in seg and "absurd()" in seg:
                k = e + 1
                if out and not out[-1].strip():
                    out.pop()
                continue
        out.append(methods[k])
        k += 1
    return out


def convert(text):
    m = re.search(r"^impl Feature for (\w+) \{\n", text, re.M)
    if not m:
        return None, "no impl (already converted?)"
    end = text.index("\n}\n", m.end()) + 3
    body = text[m.end():end - 3].split("\n")
    consts, types, methods = {}, {}, []
    seen_method = False
    for line in body:
        cm = re.match(r"    const (\w+): (.*);$", line)
        tm = re.match(r"    type (\w+) = (.*);$", line)
        if cm and not seen_method:
            consts[cm.group(1)] = line.strip()
        elif tm and not seen_method:
            types[tm.group(1)] = tm.group(2)
        elif cm and line.startswith("    const RUNS") :
            consts["RUNS"] = line.strip()
        else:
            if line.strip():
                seen_method = True
            methods.append(line)
    while methods and not methods[0].strip():
        methods.pop(0)
    while methods and not methods[-1].strip():
        methods.pop()
    if "Command" not in types or types["Command"] == "Never<Self>":
        methods = drop_absurd(methods)
    for line in methods:
        stripped = line.replace('\\"', "").replace("'\"'", "")
        if stripped.count('"') % 2:
            return None, "a method line with an odd number of quotes: " + line.strip()
    out = [f"feature! {{", f"    impl Feature for {m.group(1)} {{"]
    for name in CONSTS:
        if name in consts:
            out.append("        " + consts[name])
    unknown = set(consts) - set(CONSTS)
    if unknown or set(types) - set(TYPES):
        return None, f"unknown items {unknown} {set(types) - set(TYPES)}"
    for name in TYPES:
        value = types.get(name)
        if value is None or value == "Never<Self>" or value == DEFAULT.get(name):
            continue
        out.append(f"        type {name} = {value};")
    if methods:
        out.append("        {")
        out.extend(("        " + line) if line.strip() else "" for line in methods)
        out.append("        }")
    out += ["    }", "}"]
    return text[:m.start()] + "\n".join(out) + "\n" + text[end:], None


for name in sys.argv[1:]:
    path = FEATURES / name / "mod.rs"
    new, why = convert(path.read_text())
    if new is None:
        print(f"{name}: left as is: {why}")
    else:
        path.write_text(new)
        print(f"{name}: converted")
