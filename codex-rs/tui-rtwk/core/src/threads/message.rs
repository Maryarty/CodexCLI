//! What the user sends: the text of the composer with the images attached to
//! it. A local image stands in the text as its `[Image #N]` placeholder; the
//! turn carries the file and the placeholder's byte range
//! (`chatwidget/input_submission.rs`: `UserInput::LocalImage` items first,
//! then the text with its `text_elements`).

use std::ops::Range;
use std::path::PathBuf;

pub use crate::data::mention::ToolMention;
pub use crate::kit::mentions::TaskRef;

/// An image file attached to a message, the placeholder it stands for in the
/// text being `range`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalImage {
    pub path: PathBuf,
    pub range: Range<usize>,
}

/// A message of the user: text, the images its placeholders stand for and the tools it names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserMessage {
    pub text: String,
    pub images: Vec<LocalImage>,
    /// The URLs of the images above the draft (`remote_image_urls`): no placeholder stands for
    /// them in the text, the turn sends them before the local ones.
    pub remote_images: Vec<String>,
    pub mentions: Vec<ToolMention>,
    /// The tasks its `@` mentions name (`@title` in the text): the turn adds their context and
    /// writes them as links (`kit::mentions::with_task_context`); the visible text keeps the title.
    pub tasks: Vec<TaskRef>,
}

impl UserMessage {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            images: Vec::new(),
            remote_images: Vec::new(),
            mentions: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// The message with the whitespace around its text cut away, the images'
    /// ranges moved with it (an image whose placeholder was cut is dropped).
    pub fn trimmed(self) -> Self {
        let start = self.text.len() - self.text.trim_start().len();
        let text = self.text.trim().to_string();
        let images = self
            .images
            .into_iter()
            .filter_map(|image| {
                let range = image.range.start.checked_sub(start)?..image.range.end - start;
                (range.end <= text.len()).then_some(LocalImage {
                    path: image.path,
                    range,
                })
            })
            .collect();
        let tasks = self
            .tasks
            .into_iter()
            .filter_map(|task| {
                let range = task.range.start.checked_sub(start)?..task.range.end - start;
                (range.end <= text.len()).then_some(TaskRef { range, ..task })
            })
            .collect();
        Self {
            text,
            images,
            remote_images: self.remote_images,
            mentions: self.mentions,
            tasks,
        }
    }

    /// Nothing to send: no text and no image, remote or local.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.images.is_empty() && self.remote_images.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(start: usize, end: usize) -> LocalImage {
        LocalImage {
            path: PathBuf::from("/tmp/a.png"),
            range: start..end,
        }
    }

    #[test]
    fn trimming_moves_the_ranges_with_the_text() {
        let message = UserMessage {
            text: "  [Image #1] hi\n".into(),
            images: vec![image(2, 12)],
            ..UserMessage::default()
        };
        let message = message.trimmed();
        assert_eq!(message.text, "[Image #1] hi");
        assert_eq!(message.images, vec![image(0, 10)]);
    }

    #[test]
    fn trimming_moves_the_task_ranges_too() {
        let message = UserMessage {
            text: "  see @Fix it\n".into(),
            tasks: vec![TaskRef {
                range: 6..13,
                title: "Fix it".into(),
                thread: "t1".into(),
            }],
            ..UserMessage::default()
        };
        let message = message.trimmed();
        assert_eq!(message.text, "see @Fix it");
        assert_eq!(message.tasks[0].range, 4..11);
    }

    #[test]
    fn an_image_alone_is_a_message() {
        let message = UserMessage {
            text: String::new(),
            images: vec![image(0, 0)],
            ..UserMessage::default()
        };
        assert!(!message.is_empty());
        assert!(UserMessage::text("").is_empty());
    }

    #[test]
    fn remote_images_alone_are_a_message_and_survive_trimming() {
        let message = UserMessage {
            remote_images: vec!["https://x/a.png".into()],
            ..UserMessage::default()
        };
        assert!(!message.is_empty());
        assert_eq!(message.trimmed().remote_images, vec!["https://x/a.png"]);
    }
}
