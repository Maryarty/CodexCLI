//! The tip a new conversation's startup card shows (oracle `tooltips.rs`, `assets/tooltips.txt`):
//! the catalog of Markdown tips, the `{key:context.action}` placeholders filled from the keymap
//! (a tip whose shortcut is unbound is left out), the desktop-app tip, and the pick.
//!
//! The oracle draws a random one with the thread's `rand`; the core has no clock and no random
//! source, so the pick is a hash of the thread's id: the same conversation shows the same tip,
//! another conversation another. The announcement tip (fetched from the web by the oracle's
//! prewarm) and the tips of experimental features (`FEATURES`) are not in the pool: both are
//! host facts the core does not hold yet.

use crate::ids::ThreadId;
use crate::keymap::Keymap;
use crate::keymap::catalogue::{ActionId, ROWS};

/// The catalog: one tip per line, `#` lines are comments.
const CATALOG: &str = include_str!("../../assets/tooltips.txt");

const MACOS_APP_TIP: &str =
    "Use the **desktop app**. Run `codex app` to open it. It installs automatically if needed.";
const WINDOWS_APP_TIP: &str = "Use the **desktop app**. Install it from https://chatgpt.com/codex?app-landing-page=true and run `codex app`.";

/// The desktop-app tip of this machine (oracle `app_tooltip`). The Linux one depends on a display
/// being present, which the core cannot look at.
fn app_tip() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some(MACOS_APP_TIP)
    } else if cfg!(target_os = "windows") {
        Some(WINDOWS_APP_TIP)
    } else {
        None
    }
}

/// The catalog's templates, in file order, then the desktop-app tip (oracle `TOOLTIPS`).
pub fn templates() -> impl Iterator<Item = &'static str> {
    CATALOG
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .chain(app_tip())
}

/// The action a `{key:context.action}` placeholder names.
fn action(context: &str, name: &str) -> Option<ActionId> {
    ROWS.iter()
        .map(|row| row.id)
        .find(|id| id.context.config_name() == context && id.name == name)
}

/// Fills the placeholders of `template` with the primary shortcut of each action in a Markdown
/// code span (padded: a key may be a backtick). A placeholder that is malformed, names no action
/// or whose action has no binding leaves the tip out (oracle `render_tooltip`).
pub fn render(mut template: &str, keymap: &Keymap) -> Option<String> {
    let mut rendered = String::new();
    while let Some((prefix, rest)) = template.split_once("{key:") {
        let (path, suffix) = rest.split_once('}')?;
        let (context, name) = path.split_once('.')?;
        let label = keymap.label(action(context, name)?)?;
        rendered.push_str(prefix);
        rendered.push_str(&format!("`` {label} ``"));
        template = suffix;
    }
    rendered.push_str(template);
    Some(rendered)
}

/// The tips this keymap can show, in catalog order (oracle `resolved_tooltips`).
pub fn resolved(keymap: &Keymap) -> Vec<String> {
    templates()
        .filter_map(|template| render(template, keymap))
        .collect()
}

/// FNV-1a: the same bytes give the same number on every machine.
fn hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The tip `thread` opens with (oracle `get_tooltip`): the desktop-app tip in eight of ten
/// conversations on a machine that has one, else one of the resolved tips.
pub fn pick(thread: &ThreadId, keymap: &Keymap) -> Option<String> {
    let seed = hash(thread.as_str());
    if seed % 10 < 8
        && let Some(tip) = app_tip()
    {
        return Some(tip.to_owned());
    }
    let mut tips = resolved(keymap);
    if tips.is_empty() {
        return None;
    }
    let index = usize::try_from(seed / 10).unwrap_or_default() % tips.len();
    Some(tips.swap_remove(index))
}

/// How many times the startup shows one model's availability notice (oracle
/// `MODEL_AVAILABILITY_NUX_MAX_SHOW_COUNT`).
pub const MAX_NUX_SHOWS: u32 = 4;

/// The model-availability notice the startup shows in place of a tip (oracle
/// `select_model_availability_nux`), and how many times it will have been shown after this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nux {
    pub model: String,
    pub message: String,
    /// The count to write back to `tui.model_availability_nux.shown_count`.
    pub shown: u32,
}

/// The notice of the first listed model that has one, while it was shown fewer than
/// [`MAX_NUX_SHOWS`] times (a model whose count ran out ends the search: the oracle does not look
/// at the next). The host asks only on a later run with tooltips on, and persists `shown`.
pub fn availability_nux(
    models: &[codex_app_server_protocol::Model],
    shown: &std::collections::BTreeMap<String, u32>,
) -> Option<Nux> {
    let (model, nux) = models
        .iter()
        .find_map(|model| model.availability_nux.as_ref().map(|nux| (model, nux)))?;
    let count = shown.get(&model.model).copied().unwrap_or_default();
    (count < MAX_NUX_SHOWS).then(|| Nux {
        model: model.model.clone(),
        message: nux.message.clone(),
        shown: count.saturating_add(1),
    })
}
