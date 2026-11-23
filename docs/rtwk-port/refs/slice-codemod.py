#!/usr/bin/env -S python3 -I
"""The mechanical half of moving a feature into a slice (ADR-012; the steps are
docs/rtwk-port/refs/feature-slices.md). Run from anywhere; it works on codex-rs/tui-rtwk.

  slice-codemod.py new <f> <Marker>            skeleton of both halves + the list line
  slice-codemod.py move <f> <old.rs> [<name>]  git mv a core file into the slice, fix paths
  slice-codemod.py hub <f> [<word>]            the hub lines that still name the feature
  slice-codemod.py budget                      lower hub_budget.txt to what the code has

`new` is idempotent on an existing slice (it only prints what it skipped). It writes a slice
that compiles with every component `Never<Self>`; the filler then replaces the components it
has (step 3 of the recipe). `move` renames a file whose types stay as they are (a model or a
helper that never was a hub variant); the hub variants are rewritten by hand, `hub` lists them.
`budget` runs the rules test and writes the numbers it asks for into tests/rules/hub_budget.txt.
"""

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3] / "codex-rs" / "tui-rtwk"
CORE = ROOT / "core" / "src"
VIEW = ROOT / "src"
LIST = CORE / "features" / "mod.rs"
BUDGET = ROOT / "tests" / "rules" / "hub_budget.txt"
# The host's list of capabilities (`effects::Ask` / `Done`): a feature asks one through
# `AppCx::effect` and is answered in `Feature::effect_done`; the variant itself stays here.
HOST_CAPABILITIES = "core/src/app/effects.rs"
STAYS = "   [host capability: stays in effects.rs, the feature asks it through cx.effect]"

CORE_MOD = '''//! {title} as a feature slice (ADR-012; oracle: TODO the oracle file and function).

use codex_app_server_protocol::{{ClientRequest, RequestId}};

use super::{{Feature, Never}};
use crate::ids::ThreadId;
use crate::rpc::RpcError;

/// The marker type of the feature.
pub struct {marker};

impl Feature for {marker} {{
    const NAME: &'static str = "{f}";

    type State = ();
    type ThreadPart = ();
    type Intent = Never<Self>;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Never<Self>;
    type Menu = Never<Self>;
    type Pick = Never<Self>;
    type Prompt = Never<Self>;
    type Note = Never<Self>;
    type Fact = Never<Self>;

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {{
        command.absurd()
    }}

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {{
        command.absurd()
    }}
}}
'''

VIEW_MOD = '''//! The view half of {title} (ADR-012): the words of its notes and the rows of its menus
//! (`strings.rs`). The flow itself is `codex_tui_core::features::{f}`.

pub mod strings;

use codex_tui_core::features::Never;
use codex_tui_core::features::{f}::{marker};
use codex_tui_core::screens::select::list::List;

use super::{{FeatureView, PromptWords}};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for {marker} {{
    fn note_words(note: &Never<Self>, _words: &SessionStrings) -> String {{
        note.absurd()
    }}

    fn menu_list(menu: &Never<Self>, _state: &(), _words: &SelectStrings) -> List {{
        menu.absurd()
    }}

    fn prompt_words(prompt: &Never<Self>, _words: &DialogStrings) -> PromptWords {{
        prompt.absurd()
    }}
}}
'''

VIEW_STRINGS = '''//! The words of {title}: its transcript notes and the rows of its menus (the oracle's
//! English until the catalog has the entries; a typed const table, ADR-010).
'''


def die(message):
    sys.exit(f"slice-codemod: {message}")


def run(*cmd, cwd=ROOT, check=True):
    return subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, check=check)


def snake(name):
    if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
        die(f"feature name {name!r}: lower snake case")
    return name


def listed():
    text = LIST.read_text()
    body = text.split("macro_rules! feature_list {", 1)[1].split("\n}\n", 1)[0]
    return re.findall(r"^\s+(\w+) => (\w+);$", body, re.M)


def new(f, marker):
    snake(f)
    if not re.fullmatch(r"[A-Z][A-Za-z0-9]*", marker):
        die(f"marker {marker!r}: a type name")
    title = f"`/{f.replace('_', '-')}`"
    core = CORE / "features" / f
    view = VIEW / "features" / f
    for path, text in [
        (core / "mod.rs", CORE_MOD.format(f=f, marker=marker, title=title)),
        (view / "mod.rs", VIEW_MOD.format(f=f, marker=marker, title=title)),
        (view / "strings.rs", VIEW_STRINGS.format(title=title)),
    ]:
        if path.exists():
            print(f"skipped {path.relative_to(ROOT)} (exists)")
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        print(f"wrote {path.relative_to(ROOT)}")
    if any(name == f for name, _ in listed()):
        print(f"skipped the list line (`{f} => ...` is in feature_list!)")
        return
    text = LIST.read_text()
    match = re.search(r"(\$callback! \{\n)((?:\s+\w+ => \w+;\n)*)(\s+)\}", text)
    if not match:
        die("feature_list! in core/src/features/mod.rs has an unexpected shape")
    line = f"            {f} => {marker};\n"
    text = text[: match.end(2)] + line + text[match.end(2) :]
    LIST.write_text(text)
    print(f"added `{f} => {marker};` to feature_list! (the line's position is the hook order)")


def sources():
    for base in (CORE, VIEW, ROOT / "host" / "src", ROOT / "core" / "tests", ROOT / "tests"):
        yield from base.rglob("*.rs")


def module_path(old):
    """`core/src/screens/select/goal.rs` -> `screens::select::goal`."""
    rel = old.relative_to(CORE).with_suffix("")
    parts = list(rel.parts)
    if parts[-1] == "mod":
        parts.pop()
    return "::".join(parts)


def move(f, old, name):
    snake(f)
    old = (ROOT / old).resolve() if not pathlib.Path(old).is_absolute() else pathlib.Path(old)
    if not old.exists() or CORE not in old.parents:
        die(f"{old}: not a file of core/src")
    name = name or old.stem
    new_file = CORE / "features" / f / f"{name}.rs"
    if not (CORE / "features" / f / "mod.rs").exists():
        die(f"core/src/features/{f}/mod.rs: run `new {f} <Marker>` first")
    old_path = module_path(old)
    new_path = f"features::{f}::{name}"
    new_file.parent.mkdir(parents=True, exist_ok=True)
    if run("git", "mv", str(old), str(new_file), check=False).returncode != 0:
        old.rename(new_file)  # not tracked yet
    # the declaration in the old parent and in the slice
    parent = old.parent.with_suffix(".rs")
    for decl in (parent, old.parent / "mod.rs"):
        if decl.exists():
            text = decl.read_text()
            text = re.sub(rf"^\s*(pub(\([a-z: ]+\))? )?mod {old.stem};\n", "", text, flags=re.M)
            decl.write_text(text)
    mod_rs = CORE / "features" / f / "mod.rs"
    text = mod_rs.read_text()
    if f"mod {name};" not in text:
        text = text.replace("\nuse ", f"\nmod {name};\n\nuse ", 1) if "\nuse " in text else text + f"\nmod {name};\n"
        mod_rs.write_text(text)
    # the paths, as `crate::a::b` and `codex_tui_core::a::b`; a grouped `use crate::a::{b, c}`,
    # `super::` and a re-export are the compiler's to name
    changed = 0
    for path in sources():
        if path == new_file:
            continue
        text = path.read_text()
        fixed = text
        for prefix in ("crate::", "codex_tui_core::"):
            fixed = fixed.replace(f"{prefix}{old_path}", f"{prefix}{new_path}")
        if fixed != text:
            path.write_text(fixed)
            changed += 1
    print(f"moved {old.relative_to(ROOT)} -> {new_file.relative_to(ROOT)}; paths fixed in {changed} files")
    print("next: cargo check; `super::` and relative uses inside the moved file are the compiler's to name")


def hub(f, word):
    word = (word or f).replace("_", "")
    pattern = re.compile(word, re.I)
    budget_files = {
        line.split()[1]
        for line in BUDGET.read_text().splitlines()
        if line.startswith(("enum ", "arms "))
    }
    budget_files |= {"src/pages/session/strings.rs", "core/src/commands/dispatch.rs"}
    seen = 0
    for rel in sorted(budget_files):
        path = ROOT / rel
        if not path.exists():
            continue
        for number, line in enumerate(path.read_text().splitlines(), 1):
            if pattern.search(line) and not line.lstrip().startswith("//"):
                if rel == HOST_CAPABILITIES:
                    print(f"{rel}:{number}: {line.strip()}{STAYS}")
                    continue
                print(f"{rel}:{number}: {line.strip()}")
                seen += 1
    # a feature leaves no variant of its own behind in the other core files either
    for path in sorted(CORE.rglob("*.rs")):
        rel = str(path.relative_to(ROOT))
        if rel in budget_files or "/features/" in rel:
            continue
        for number, line in enumerate(path.read_text().splitlines(), 1):
            if re.search(rf"\b(Menu|Picked|NoteKey|PromptKind|Command|Ctx|Reply|ThreadAsk|Intent)::\w*{word}", line, re.I):
                print(f"{rel}:{number}: {line.strip()}")
                seen += 1
            elif re.search(rf"\b(Ask|Done)::\w*{word}", line, re.I) and not line.lstrip().startswith("//"):
                # the host's capability: the call site becomes `cx.effect(Ask::X, ctx)` and an
                # arm of `effect_done`; the variants stay in effects.rs
                print(f"{rel}:{number}: {line.strip()}   [host ask/answer: cx.effect / effect_done]")
                seen += 1
    print(f"{seen} lines name {word!r}: each is a hub variant or arm to move (recipe step 5)")


def budget():
    out = run(
        "bash", "-c",
        "~/.cache/rtwk/acargo nextest run --test rules --status-level fail --final-status-level fail "
        "--hide-progress-bar --cargo-quiet -E 'test(hub_enums_only_shrink) | test(threads_apply_matches)' 2>&1",
        check=False,
    ).stdout
    text = BUDGET.read_text()
    changed = 0
    for file, name, count in re.findall(r"(\S+\.rs) (\w+): (\d+) < the budget \d+;", out):
        text, n = re.subn(rf"^(enum {re.escape(file)} {name} )\d+$", rf"\g<1>{count}", text, flags=re.M)
        changed += n
    for file, fn, pattern, count in re.findall(r"([^\s\"]+\.rs) (\w+) names (\S+): (\d+) < the budget \d+", out):
        text, n = re.subn(rf"^(arms {re.escape(file)} {fn} {re.escape(pattern)} )\d+$", rf"\g<1>{count}", text, flags=re.M)
        changed += n
    BUDGET.write_text(text)
    print(f"lowered {changed} budget lines" if changed else "nothing to lower")
    if "FAIL" in out and not changed:
        print(out[-2000:])


def main(argv):
    if len(argv) >= 3 and argv[0] == "new" and len(argv) == 3:
        new(argv[1], argv[2])
    elif argv and argv[0] == "move" and len(argv) in (3, 4):
        move(argv[1], argv[2], argv[3] if len(argv) == 4 else None)
    elif argv and argv[0] == "hub" and len(argv) in (2, 3):
        hub(argv[1], argv[2] if len(argv) == 3 else None)
    elif argv == ["budget"]:
        budget()
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == "__main__":
    main(sys.argv[1:])
