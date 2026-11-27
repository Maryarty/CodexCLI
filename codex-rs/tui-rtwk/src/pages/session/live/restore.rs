//! The composer takes back messages the model gave it (`restore_composer_state`):
//! the steers and the queue after an interrupt, a steer that failed, the
//! newest queued message for editing. The model decides what and in which
//! order (`Restore::onto`); the composer controller only holds the text. The
//! user messages of a replayed (resumed) thread seed the composer's history.

use std::collections::BTreeMap;

use codex_tui_core::data::mention::{ToolMention, uri};
use codex_tui_core::kit::history::{HistoryEntry, MentionBinding, TextElement};
use codex_tui_core::kit::mentions::codec::{PLUGIN_TEXT_MENTION_SIGIL, TOOL_MENTION_SIGIL};
use codex_tui_core::kit::mentions::rebind::find_next_mention_token_range;
use codex_tui_core::screens::backtrack::Prompt;
use codex_tui_core::threads::UserMessage;
use codex_tui_core::threads::replay::Submissions;

use crate::widgets::thread_ui::ThreadUi;

/// Puts `message` in the composer of `ui`: its remote images above the text, its text, each
/// local image's placeholder an attached element again (numbered after the remote ones), the
/// caret at the end. `image` words a placeholder from its number
/// (`restore_user_message_to_composer`).
pub(crate) fn put_back(
    ui: &ThreadUi,
    message: &UserMessage,
    replaces_draft: bool,
    image: impl Fn(usize) -> String,
) {
    ui.draft
        .restore_message(&ui.composer, recalled_entry(message), replaces_draft);
    ui.draft.relabel_images(&ui.composer, image);
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
        mention_bindings: mention_bindings(
            &prompt.text,
            prompt
                .mentions
                .iter()
                .map(|input| (input.skill, input.name.as_str(), input.path.as_str())),
        ),
        pending_pastes: Vec::new(),
    }
}

fn mention_bindings<'a>(
    text: &str,
    mentions: impl Iterator<Item = (bool, &'a str, &'a str)>,
) -> Vec<MentionBinding> {
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
    let mut bindings: Vec<MentionBinding> = mentions
        .map(|(skill, name, path)| {
            if skill {
                return MentionBinding {
                    sigil: TOOL_MENTION_SIGIL,
                    mention: name.to_owned(),
                    path: path.to_owned(),
                };
            }
            let plugin = path.strip_prefix("plugin://");
            let mention = if let Some(plugin) = plugin {
                plugin
                    .split_once('@')
                    .map_or(plugin, |(name, _)| name)
                    .to_string()
            } else if path.starts_with("app://") {
                connector_slug(name)
            } else {
                name.to_owned()
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
                path: path.to_owned(),
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

/// The history entry of a message the launch sent: its text with the placeholders of its images
/// as elements, the images' files.
pub(crate) fn recalled_entry(message: &UserMessage) -> HistoryEntry {
    let mut entry = HistoryEntry::new(message.text.clone());
    for image in &message.images {
        entry.text_elements.push(TextElement {
            start: image.range.start,
            end: image.range.end,
            placeholder: message.text.get(image.range.clone()).map(str::to_string),
        });
        entry.local_image_paths.push(image.path.clone());
    }
    let tool_bindings = mention_bindings(
        &message.text,
        message.mentions.iter().map(|mention| match mention {
            ToolMention::Skill { name, path } => (true, name.as_str(), path.as_str()),
            ToolMention::Mention { name, path } => (false, name.as_str(), path.as_str()),
        }),
    );
    let mut cursors = BTreeMap::new();
    let mut bindings: Vec<_> = tool_bindings
        .into_iter()
        .map(|binding| {
            let token = format!("{}{}", binding.sigil, binding.mention);
            let cursor = cursors.entry(token.clone()).or_default();
            let start = find_next_mention_token_range(&message.text, &token, *cursor).map_or(
                usize::MAX,
                |range| {
                    *cursor = range.end;
                    range.start
                },
            );
            (start, binding)
        })
        .collect();
    for task in &message.tasks {
        let Some(token) = message.text.get(task.range.clone()) else {
            continue;
        };
        let Some(title) = token.strip_prefix('@') else {
            continue;
        };
        entry.text_elements.push(TextElement {
            start: task.range.start,
            end: task.range.end,
            placeholder: Some(token.to_owned()),
        });
        bindings.push((
            task.range.start,
            MentionBinding {
                sigil: '@',
                mention: title.to_owned(),
                path: uri("thread", task.thread.as_str()),
            },
        ));
    }
    bindings.sort_by_key(|(start, _)| *start);
    entry.mention_bindings = bindings.into_iter().map(|(_, binding)| binding).collect();
    entry.remote_image_urls = message.remote_images.clone();
    entry
}
