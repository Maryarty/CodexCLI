#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The inline forms of `/title`, `/statusline` and `/model` (oracle
//! `status_surface_commands.rs`, `model_command.rs`): the arguments checked with no IO.

use codex_tui_core::commands::args::ArgError;
use codex_tui_core::features::settings::args::{Refusal, Surface, model, status_line, title};
use codex_tui_core::screens::chat::StatusOverride;
use codex_tui_core::screens::select::models::{Apply, Current, EffortOption, Preset};
use pretty_assertions::assert_eq;

fn preset(name: &str, efforts: &[&str]) -> Preset {
    serde_json::from_value(serde_json::json!({
        "model": name,
        "name": name,
        "show_in_picker": true,
        "default_effort": "medium",
        "efforts": efforts.iter().map(|e| serde_json::json!({"effort": e})).collect::<Vec<_>>(),
    }))
    .expect("preset")
}

#[test]
fn title_items_and_clear() {
    assert_eq!(
        title("project-name thread-name").unwrap(),
        StatusOverride {
            terminal_title: Some(vec!["project-name".into(), "thread-name".into()]),
            ..StatusOverride::default()
        }
    );
    assert_eq!(title("--clear").unwrap().terminal_title, Some(Vec::new()));
    assert_eq!(
        title("--clear project-name"),
        Err(Refusal::ClearWithItems(Surface::Title))
    );
    assert_eq!(
        title("nope"),
        Err(Refusal::UnknownItem {
            surface: Surface::Title,
            token: "nope".into()
        })
    );
}

#[test]
fn statusline_items_colors_and_clear() {
    let set = status_line("model --no-colors").unwrap();
    assert_eq!(set.status_line, Some(vec!["model".into()]));
    assert_eq!(set.status_line_colors, Some(false));
    assert_eq!(status_line("--colors").unwrap().status_line, None);
    assert_eq!(
        status_line("--clear").unwrap().status_line,
        Some(Vec::new())
    );
    assert_eq!(status_line("'model"), Err(Refusal::Args(ArgError::Quoting)));
}

#[test]
fn model_names_effort_and_refusals() {
    let presets = vec![preset("gpt-a", &["low", "high"]), preset("gpt-b", &[])];
    let current = Current::new("gpt-a");
    let pick = model("gpt-b", true, &current, &presets).unwrap();
    assert_eq!(pick.selection.model, "gpt-b");
    assert_eq!(pick.selection.apply, Apply::Session);
    let pick = model("gpt-a --reasoning=high", true, &current, &presets).unwrap();
    assert_eq!(pick.selection.effort.unwrap().to_string(), "high");
    assert!(matches!(
        model("gpt-a --reasoning=ultra", true, &current, &presets),
        Err(Refusal::EffortUnavailable { .. })
    ));
    assert_eq!(
        model("gpt-z", true, &current, &presets),
        Err(Refusal::ModelUnavailable("gpt-z".into()))
    );
    assert_eq!(
        model("gpt-a extra", true, &current, &presets),
        Err(Refusal::Unexpected("extra".into()))
    );
    assert_eq!(
        model("gpt-a", false, &current, &presets),
        Err(Refusal::Starting)
    );
    let _ = EffortOption {
        effort: current.effort.clone().unwrap_or_default(),
        description: String::new(),
    };
}
