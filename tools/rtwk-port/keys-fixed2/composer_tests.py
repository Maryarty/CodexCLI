"""adr013-keys-2: the composer's editing tests name their keys as fixed actions, not KeyPress literals.

Run from the repo root: python3 -I tools/rtwk-port/keys-fixed2/composer_tests.py
Rewrites the test module of codex-rs/tui-rtwk/src/pages/session/composer/editing.rs.
"""
import re

P = "codex-rs/tui-rtwk/src/pages/session/composer/editing.rs"
s = open(P).read()
head, tests = s.split("#[cfg(test)]\nmod tests {", 1)

pairs = [
    ("KeyPress::BACKSPACE.with_mods(Mods::ALT)", "nth(FIELD_DELETE_BACKWARD_WORD, 1)"),
    ("KeyPress::char('v').with_mods(Mods::ALT)", "nth(PASTE_IMAGE, 1)"),
    ("KeyPress::BACKSPACE", "of(FIELD_DELETE_BACKWARD)"),
    ("KeyPress::DELETE", "of(FIELD_DELETE_FORWARD)"),
    ("KeyPress::LEFT", "of(FIELD_LEFT)"),
    ("KeyPress::RIGHT", "of(FIELD_RIGHT)"),
    ("KeyPress::UP", "of(FIELD_UP)"),
    ("KeyPress::DOWN", "of(FIELD_DOWN)"),
    ("KeyPress::TAB", "of(TAB)"),
    ("KeyPress::ENTER", "of(ENTER)"),
]
for a, b in pairs:
    tests = tests.replace(a, b)
tests = re.sub(r"KeyPress::char\('(\w)'\)", r"crate::keymap::typed('\1')", tests)
ctrl = {
    "w": "FIELD_DELETE_BACKWARD_WORD",
    "k": "FIELD_KILL_LINE_END",
    "u": "FIELD_KILL_LINE_START",
    "y": "FIELD_YANK",
    "v": "PASTE_IMAGE",
}
tests = re.sub(r"ctrl\('(\w)'\)", lambda m: f"of({ctrl[m.group(1)]})" if m.group(1) in ctrl else "of_keyed(&d, EDITOR_KILL_WHOLE_LINE)", tests)
tests = tests.replace(
    "    fn ctrl(ch: char) -> KeyPress {\n        KeyPress::char(ch).with_mods(Mods::CTRL)\n    }\n",
    "    /// The press of a hardwired action's first key (the defaults' for a configurable one).\n"
    "    fn of(id: ActionId) -> KeyPress {\n        nth(id, 0)\n    }\n\n"
    "    fn nth(id: ActionId, n: usize) -> KeyPress {\n"
    "        crate::keymap::presses(&Keymap::defaults(), id)\n            .into_iter()\n            .nth(n)\n            .expect(\"the action has the key\")\n    }\n\n"
    "    /// The press of the first key the test's own keymap gives an action.\n"
    "    fn of_keyed(d: &KeyData, id: ActionId) -> KeyPress {\n"
    "        crate::keymap::presses(&d.keymap, id)\n            .into_iter()\n            .next()\n            .expect(\"the keymap binds the action\")\n    }\n",
)
open(P, "w").write(head + "#[cfg(test)]\nmod tests {" + tests)
