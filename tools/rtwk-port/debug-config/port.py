"""Carry the pure part of the oracle's `/debug-config` (`codex-rs/tui/src/debug_config.rs`, up to
its tests) into `codex-rs/tui-rtwk/host/src/debug_config.rs`: the layer stack and the
requirements become rows (`Row`), the history cell is the view's. Rerun after the oracle moves:
python3 -I tools/rtwk-port/debug-config/port.py"""
import pathlib

root = pathlib.Path(__file__).resolve().parents[3]
src = (root / "codex-rs/tui/src/debug_config.rs").read_text().split("\n")
# 1-based: the body from render_agents_config_lines to the end of the code before the tests,
# without session_all_proxy_url (the host has no session proxy to word).
start = next(i for i, l in enumerate(src) if l.startswith("fn render_agents_config_lines"))
tests = next(i for i, l in enumerate(src) if l.startswith("#[cfg(test)]"))
body = src[start:tests]
drop = next(i for i, l in enumerate(body) if l.startswith("fn session_all_proxy_url"))
end = next(i for i in range(drop, len(body)) if body[i] == "}")
body = body[:drop] + body[end + 2:]
text = "\n".join(body).replace("Line<'static>", "Line")
header = '''//! `/debug-config` (oracle `debug_config.rs`): the config layers and the requirements as rows. The
//! rendering is carried by `tools/rtwk-port/debug-config/port.py`; the cell that draws the rows is
//! the view's (`Done::DebugConfig`). The session's network proxy is not the host's: no such rows.

use codex_config::CONFIG_TOML_FILE;
use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerSource;
use codex_config::ConfigLayerStack;
use codex_config::ManagedHooksRequirementsToml;
use codex_config::NetworkConstraints;
use codex_config::NetworkDomainPermissionToml;
use codex_config::NetworkUnixSocketPermissionToml;
use codex_config::RequirementSource;
use codex_config::ResidencyRequirement;
use codex_config::SandboxModeRequirement;
use codex_config::TomlValue;
use codex_config::WebSearchModeRequirement;
use codex_config::format_config_layer_source;
use codex_core::config::Config;
use codex_core::config::Permissions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::permissions::NetworkSandboxPolicy;

/// One row, as the oracle's `Stylize` chain builds it; `style` is the name the slice's
/// `Fact::debug_config` reads (`plain`, `bold`, `dim`, `magenta`).
pub(crate) struct Row {
    pub(crate) text: String,
    pub(crate) style: &'static str,
}

pub(crate) struct Line(Row);

impl From<&str> for Line {
    fn from(text: &str) -> Self {
        Self(Row {
            text: text.to_owned(),
            style: "plain",
        })
    }
}

impl From<String> for Line {
    fn from(text: String) -> Self {
        Self(Row {
            text,
            style: "plain",
        })
    }
}

trait Stylize: Sized + Into<Line> {
    fn styled(self, style: &'static str) -> Line {
        let Line(row) = self.into();
        Line(Row { style, ..row })
    }
    fn bold(self) -> Line {
        self.styled("bold")
    }
    fn dim(self) -> Line {
        self.styled("dim")
    }
    fn magenta(self) -> Line {
        self.styled("magenta")
    }
}

impl Stylize for &str {}
impl Stylize for String {}

/// The rows of `/debug-config` for `config` (`new_debug_config_output` without the session proxy).
pub(crate) fn rows(config: &Config) -> Vec<Row> {
    let mut lines = render_debug_config_lines(&config.config_layer_stack, |mode| {
        sandbox_mode_is_allowed_by_permissions(&config.permissions, mode)
    });
    lines.extend(render_agents_config_lines(config));
    lines.into_iter().map(|Line(row)| row).collect()
}

'''
out = root / "codex-rs/tui-rtwk/host/src/debug_config.rs"
out.write_text(header + text + "\n")
