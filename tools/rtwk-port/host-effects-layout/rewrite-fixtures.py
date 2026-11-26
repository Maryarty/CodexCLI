"""Move the former screen-specific command patterns into a host effect."""

from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[3] / "codex-rs/tui-rtwk"
PATHS = """
core/tests/agents_live.rs
core/tests/app.rs
core/tests/app_link.rs
core/tests/app_wiring.rs
core/tests/approvals.rs
core/tests/backend_switch.rs
core/tests/commands.rs
core/tests/import.rs
core/tests/model.rs
core/tests/onboarding.rs
core/tests/onboarding/consent.rs
core/tests/onboarding/device.rs
core/tests/onboarding/gateway.rs
core/tests/queue.rs
core/tests/select_live.rs
core/tests/usage/state.rs
core/tests/worktree_popups.rs
host/src/in_process.rs
tests/app/skeleton.rs
tests/common/mod.rs
tests/kit/seed.rs
tests/pages/chat_live.rs
tests/pages/commands.rs
tests/pages/composer_history.rs
tests/pages/composer_images.rs
tests/pages/onboarding.rs
tests/pages/onboarding_focus.rs
tests/pages/signin.rs
tests/pages/signin_wizards.rs
tests/pages/startup.rs
tests/services/shell_agents.rs
""".split()
VARIANTS = """
BindWorktree CreateWorktree Directories Import OpenUrl Popovers
ReconnectIdleThread SaveGrouping Startup Usage VerifyUser
""".split()
PATTERN = re.compile(
    rf"\b(?P<command>Command|AppCommand)::(?P<variant>{'|'.join(VARIANTS)})\s*\{{"
)


def closing_brace(source, start):
    depth = 1
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if not depth:
                return index
    raise ValueError(f"unclosed pattern near offset {start}")


def migrate(source):
    output = []
    offset = 0
    for match in PATTERN.finditer(source):
        line_start = source.rfind("\n", 0, match.start()) + 1
        if source[line_start:match.start()].lstrip().startswith("//"):
            continue
        end = closing_brace(source, match.end())
        fields = source[match.end():end]
        command, variant = match.group("command", "variant")
        output.append(source[offset:match.start()])
        output.append(
            f"{command}::Effect {{ ask: codex_tui_core::app::effects::Ask::{variant} "
            f"{{{fields}}}, .. }}"
        )
        offset = end + 1
    output.append(source[offset:])
    return "".join(output)


def main():
    for name in PATHS:
        path = ROOT / name
        current = path.read_text()
        changed = migrate(current)
        if changed != current:
            path.write_text(changed)
            print(name)


if __name__ == "__main__":
    main()
