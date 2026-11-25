use codex_app_server_protocol::AddCreditsNudgeCreditType;
use codex_app_server_protocol::AddCreditsNudgeEmailStatus;
use codex_app_server_protocol::ByteRange;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountRateLimitsResponse;
use codex_app_server_protocol::ImageReference;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SendAddCreditsNudgeEmailParams;
use codex_app_server_protocol::SendAddCreditsNudgeEmailResponse;
use codex_app_server_protocol::TextElement;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnInterruptResponse;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_tui_core::protocol::*;
use codex_tui_core::threads::LocalImage;
use codex_tui_core::threads::ToolMention;
use codex_tui_core::threads::UserMessage;
use std::path::PathBuf;

#[test]
fn the_images_go_first_and_the_text_names_their_placeholders() {
    let message = UserMessage {
        text: "see [Image #1]".into(),
        images: vec![LocalImage {
            path: "/tmp/a.png".into(),
            range: 4..14,
        }],
        ..UserMessage::default()
    };
    let input = turn_input_in(&message, None);
    assert!(matches!(&input[0], UserInput::LocalImage { path, .. } if path.ends_with("a.png")));
    let UserInput::Text {
        text,
        text_elements,
    } = &input[1]
    else {
        panic!("the text second: {input:?}");
    };
    assert_eq!(text, "see [Image #1]");
    assert_eq!(
        text_elements,
        &vec![TextElement::new(
            ByteRange { start: 4, end: 14 },
            Some("[Image #1]".into())
        )]
    );
}

#[test]
fn an_image_alone_sends_no_text() {
    let message = UserMessage {
        text: String::new(),
        images: vec![LocalImage {
            path: "/tmp/a.png".into(),
            range: 0..0,
        }],
        ..UserMessage::default()
    };
    assert_eq!(turn_input_in(&message, None).len(), 1);
}

#[test]
fn the_remote_images_go_before_the_local_ones() {
    let message = UserMessage {
        text: "see [Image #2]".into(),
        images: vec![LocalImage {
            path: "/tmp/a.png".into(),
            range: 4..14,
        }],
        remote_images: vec!["https://x/a.png".into(), "https://x/b.png".into()],
        ..UserMessage::default()
    };
    let input = turn_input_in(&message, None);
    assert_eq!(input.len(), 4);
    assert!(matches!(
        &input[0],
        UserInput::Image { image: ImageReference::Inline { url }, .. } if url == "https://x/a.png"
    ));
    assert!(matches!(
        &input[1],
        UserInput::Image { image: ImageReference::Inline { url }, .. } if url == "https://x/b.png"
    ));
    assert!(matches!(&input[2], UserInput::LocalImage { .. }));
    assert!(matches!(&input[3], UserInput::Text { .. }));
}

#[test]
fn remote_images_alone_send_no_text() {
    let message = UserMessage {
        remote_images: vec!["https://x/a.png".into()],
        ..UserMessage::default()
    };
    assert_eq!(turn_input_in(&message, None).len(), 1);
}

#[test]
fn the_named_tools_follow_the_text() {
    let message = UserMessage {
        text: "use $deploy and $Figma".into(),
        images: Vec::new(),
        remote_images: Vec::new(),
        mentions: vec![
            ToolMention::Skill {
                name: "deploy".into(),
                path: "/skills/deploy/SKILL.md".into(),
            },
            ToolMention::Mention {
                name: "Figma".into(),
                path: "plugin://figma@openai".into(),
            },
        ],
    };
    let input = turn_input_in(&message, None);
    assert_eq!(input.len(), 3);
    assert!(matches!(&input[0], UserInput::Text { .. }));
    assert!(matches!(
        &input[1],
        UserInput::Skill { name, path } if name == "deploy" && path.ends_with("SKILL.md")
    ));
    assert!(matches!(
        &input[2],
        UserInput::Mention { name, path } if name == "Figma" && path == "plugin://figma@openai"
    ));
}

#[test]
fn a_task_mention_adds_its_context_to_the_text_sent_and_keeps_its_element() {
    use codex_tui_core::threads::TaskRef;
    let message = UserMessage {
        text: "Inspect @Review it".into(),
        tasks: vec![TaskRef {
            range: 8..18,
            title: "Review it".into(),
            thread: codex_tui_core::ids::ThreadId::new("task-123"),
        }],
        ..UserMessage::default()
    };
    let input = turn_input_in(&message, Some("current"));
    let UserInput::Text {
        text,
        text_elements,
    } = &input[0]
    else {
        panic!("the text first: {input:?}");
    };
    assert!(text.contains("\"threadId\":\"task-123\""));
    assert!(text.ends_with("Inspect [@Review it](thread://task-123)"));
    let range = &text_elements[0].byte_range;
    assert_eq!(
        &text[range.start..range.end],
        "[@Review it](thread://task-123)"
    );
    // the task of the thread itself is no reference
    let own = turn_input_in(&message, Some("task-123"));
    assert!(matches!(&own[0], UserInput::Text { text, .. } if text == "Inspect @Review it"));
}
