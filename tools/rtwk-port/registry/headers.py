"""One-time migration of the cell list (fw-hubs-c5): writes the `// cell: <pattern>` head line into
each cells/<file>.rs from the entries of the old hand-written `cell_list!` (kept here), and takes the
`pub mod` lines of those files and the macro out of cells/mod.rs. Idempotent: rerunning changes
nothing. Run from the repository root: python3 -I tools/rtwk-port/registry/headers.py"""
import pathlib

CELLS = pathlib.Path("codex-rs/tui-rtwk/src/pages/session/cells")
OLD = """Item::User(_) => user;
Item::Agent { .. } => agent;
Item::Tool(_) => tool_rows;
Item::Reasoning { .. } => reasoning;
Item::Patch(_) => patch;
Item::Plan { .. } => plan;
Item::Web(_) => web;
Item::ImageView { .. } => viewed_image;
Item::Notice(_) => notice;
Item::Note(_) => note;
Item::PlanUpdate(_) => plan_update;
Item::Hook(_) => hook;
Item::Collab(_) => collab;
Item::Delegated(_) => delegated;
Item::Generated(_) => generated_image;
Item::Error { .. } | Item::HighLoad => error;
Item::Warning { .. } | Item::LocalWarning(_) | Item::StartupWarning(_) => warning;
Item::Deprecation { .. } => deprecation;
Item::Decision(_) | Item::Review(_) => decision;
Item::Status { .. } => status;"""

mods = []
for line in OLD.splitlines():
    pattern, file = line.rsplit(" => ", 1)
    file = file.rstrip(";")
    mods.append(file)
    path = CELLS / f"{file}.rs"
    if not path.is_file():
        path = CELLS / file / "mod.rs"
    text = path.read_text()
    head = f"// cell: {pattern}\n"
    if not text.startswith(head):
        path.write_text(head + text)

hub = CELLS / "mod.rs"
text = hub.read_text()
for file in mods:
    text = text.replace(f"pub mod {file};\n", "")
start = text.find("/// The list of the kinds of transcript item")
end = text.find("use cell_list;\n")
if start != -1 and end != -1:
    text = text[:start] + 'include!(concat!(env!("OUT_DIR"), "/cells.rs"));\n' + text[end + len("use cell_list;\n"):]
hub.write_text(text)
