"""Mechanical part of the port of core/tests/settings.rs to the feature slice.

Run from codex-rs/tui-rtwk: python3 -I ../../tools/rtwk-port/settings-slice/core_test.py
(the rest of the port is by hand: the helpers at the top, the tests of the request log and of
the write cycle).
"""
import re
from pathlib import Path

path = Path("core/tests/settings.rs")
text = path.read_text()
text = text.replace("codex_tui_core::screens::settings::", "codex_tui_core::features::settings::")
text = text.replace("../../src/sheets/settings/fixtures", "../../src/features/settings/fixtures")
text = text.replace("src/sheets/settings/draft.rs", "src/features/settings/draft.rs")
text = re.sub(r"\bSandbox\b", "Rig", text)
text = re.sub(r"\bsandbox\b", "rig", text)
text = text.replace("SettingsEv", "SettingsFact")
text = text.replace("Dialog::", "Which::")
path.write_text(text)
