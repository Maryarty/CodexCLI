//! The composer takes back messages the model gave it (`restore_composer_state`):
//! the steers and the queue after an interrupt, a steer that failed, the
//! newest queued message for editing. The model decides what and in which
//! order (`Restore::onto`); the composer controller only holds the text. The
//! user messages of a replayed (resumed) thread seed the composer's history.

use std::path::PathBuf;

use codex_tui_core::screens::backtrack::Prompt;
use codex_tui_core::screens::composer::history::{HistoryEntry, MentionBinding, TextElement};
use codex_tui_core::screens::composer::mentions::codec::{
    PLUGIN_TEXT_MENTION_SIGIL, TOOL_MENTION_SIGIL,
};
use codex_tui_core::threads::UserMessage;
use codex_tui_core::threads::replay::Submissions;

use crate::widgets::thread_ui::ThreadUi;

/// Puts `message` in the composer of `ui`: its remote images above the text, its text, each
/// local image's placeholder an attached element again (numbered after the remote ones), the
/// caret at the end. `image` words a placeholder from its number
/// (`restore_user_message_to_composer`).
pub(crate) fn put_back(ui: &ThreadUi, message: &UserMessage, image: impl Fn(usize) -> String) {
    ui.draft.clear();
    ui.draft.set_shell(false);
    ui.composer.reset("");
    ui.draft
        .set_remote_images(&ui.composer, message.remote_images.clone(), &image);
    let mut images: Vec<(&std::ops::Range<usize>, &PathBuf)> = message
        .images
        .iter()
        .map(|image| (&image.range, &image.path))
        .collect();
    images.sort_by_key(|(range, _)| range.start);
    let mut cursor = 0;
    for (range, path) in images {
        if range.start < cursor || range.end > message.text.len() {
            continue;
        }
        ui.composer.insert(&message.text[cursor..range.start]);
        ui.draft.attach_image(&ui.composer, path.clone(), &image);
        cursor = range.end;
    }
    ui.composer.insert(&message.text[cursor..]);
    ui.draft.sync(&ui.composer);
}

/// Seeds the composer's history of `ui` from the user messages of the replayed
/// turns, oldest first (`record_replayed_user_message_history`): Up walks them.
/// The model already left out review prompts and hidden nested review turns.
pub(crate) fn seed_history(ui: &ThreadUi, submissions: &Submissions) {
    for submission in submissions.entries() {
        ui.draft.replayed(history_entry(&submission.prompt));
    }
}

/// The history entry of a stored prompt: its text with the placeholder
/// elements, the images, and the bindings of its skills and mentions
/// (`mention_bindings_from_user_inputs`).
fn history_entry(prompt: &Prompt) -> HistoryEntry {
    HistoryEntry {
        text: prompt.text.clone(),
        text_elements: prompt
            .elements
            .iter()
            .map(|element| TextElement {
                start: element.range.start,
                end: element.range.end,
                placeholder: element.placeholder.clone(),
            })
            .collect(),
        local_image_paths: prompt.local_images.clone(),
        remote_image_urls: prompt.remote_images.clone(),
        mention_bindings: mention_bindings(prompt),
        pending_pastes: Vec::new(),
    }
}

fn mention_bindings(prompt: &Prompt) -> Vec<MentionBinding> {
    let text = prompt.text.as_str();
    // the token of `sigil` + `mention` in the text, not followed by a name character
    let start_of = |sigil: char, mention: &str| {
        let token = format!("{sigil}{mention}");
        text.match_indices(&token).find_map(|(start, _)| {
            text.as_bytes()
                .get(start + token.len())
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'-'))
                .then_some(start)
        })
    };
    let mut bindings: Vec<MentionBinding> = prompt
        .mentions
        .iter()
        .map(|input| {
            if input.skill {
                return MentionBinding {
                    sigil: TOOL_MENTION_SIGIL,
                    mention: input.name.clone(),
                    path: input.path.clone(),
                };
            }
            let plugin = input.path.strip_prefix("plugin://");
            let mention = if let Some(plugin) = plugin {
                plugin
                    .split_once('@')
                    .map_or(plugin, |(name, _)| name)
                    .to_string()
            } else if input.path.starts_with("app://") {
                connector_slug(&input.name)
            } else {
                input.name.clone()
            };
            let sigil =
                if plugin.is_some() && start_of(PLUGIN_TEXT_MENTION_SIGIL, &mention).is_some() {
                    PLUGIN_TEXT_MENTION_SIGIL
                } else {
                    TOOL_MENTION_SIGIL
                };
            MentionBinding {
                sigil,
                mention,
                path: input.path.clone(),
            }
        })
        .collect();
    bindings.sort_by_key(|binding| start_of(binding.sigil, &binding.mention).unwrap_or(usize::MAX));
    bindings
}

/// The `$` token of an app (`connector_mention_slug_from_name`): ASCII
/// letters and digits lowercased, every other character a dash.
fn connector_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "app".to_string()
    } else {
        slug.to_string()
    }
}
