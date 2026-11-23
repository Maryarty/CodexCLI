//! The debug entries of `/presets`: one per page of the sheet, on the slots a script seeded, the
//! menu of the page written out (what `Opening::Quick` carries when the model asks for the sheet).

use codex_tui_core::screens::select::quick_dials::{
    Dial, Menu, PickerMode, Source, SourceRow, Target, Unavailable,
};

use crate::app::debug::{DebugEntry, EntryOpen};

macro_rules! entry {
    ($name:literal, $description:literal, $script:literal, $menu:expr) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $script, ".jsonl")),
            open: EntryOpen::Dialog(|_| super::sheet(&$menu)),
        }
    };
}

const MODEL: &str = "/model gpt-5.5 --reasoning=high --speed=fast";

fn coding() -> Dial {
    Dial {
        input: MODEL.to_string(),
        label: Some("coding".to_string()),
    }
}

fn source(source: Source, input: &str) -> SourceRow {
    SourceRow {
        source,
        dial: Ok(Dial::new(input)),
    }
}

fn sources(model: Result<Dial, Unavailable>, title: Result<Dial, Unavailable>) -> Vec<SourceRow> {
    vec![
        SourceRow {
            source: Source::Model,
            dial: model,
        },
        source(
            Source::Permissions,
            "/permissions :workspace --approval-policy=on-request --reviewer=user",
        ),
        SourceRow {
            source: Source::TerminalTitle,
            dial: title,
        },
        source(
            Source::StatusLine,
            "/statusline model project-name model --no-colors",
        ),
    ]
}

/// What the debug menu lists for the presets.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        entry!(
            "quick_save",
            "the slot picker of a preset to save: slot 1 taken, the cursor on the first empty slot",
            "dials",
            Menu::Slots {
                mode: PickerMode::Save,
                dial: Some(Dial::new("Review the current changes")),
            }
        ),
        entry!(
            "quick_edit",
            "the slot picker of a preset to edit: the saved slots",
            "dials",
            Menu::Slots {
                mode: PickerMode::Edit,
                dial: None,
            }
        ),
        entry!(
            "quick_clear",
            "the slot picker of a preset to delete: the saved slots",
            "dials",
            Menu::Slots {
                mode: PickerMode::Clear,
                dial: None,
            }
        ),
        entry!(
            "quick_clear_empty",
            "the slot picker of a preset to delete with nothing saved",
            "dials_empty",
            Menu::Slots {
                mode: PickerMode::Clear,
                dial: None,
            }
        ),
        entry!(
            "quick_editor",
            "the editor of a new preset on two lines of text",
            "dials",
            Menu::Editor {
                target: Target::PickSlot,
                dial: Dial::new("Review the current changes\nfor safety"),
            }
        ),
        entry!(
            "quick_editor_slot",
            "the editor of the preset in slot 1",
            "dials",
            Menu::Editor {
                target: Target::Slot(1),
                dial: coding(),
            }
        ),
        entry!(
            "quick_source",
            "the current settings a preset can start from",
            "dials",
            Menu::Sources(sources(
                Ok(Dial::new("/model gpt-5.5 --speed=standard")),
                Ok(Dial::new("/title model project-name model")),
            ))
        ),
        entry!(
            "quick_source_unavailable",
            "the sources with the model and the terminal title unavailable",
            "dials",
            Menu::Sources(sources(
                Err(Unavailable::Effort),
                Err(Unavailable::TitleItems),
            ))
        ),
        entry!(
            "quick_confirm_replace",
            "the confirmation of replacing the preset in slot 1",
            "dials",
            Menu::Confirm {
                slot: 1,
                old: Some(Dial::new(MODEL)),
                new: Some(Dial::new(
                    "/model gpt-5.6-terra --reasoning=high --speed=fast"
                )),
            }
        ),
        entry!(
            "quick_confirm_save",
            "the confirmation of saving a preset in the empty slot 2",
            "dials",
            Menu::Confirm {
                slot: 2,
                old: None,
                new: Some(Dial::new(
                    "/model gpt-5.6-terra --reasoning=high --speed=fast"
                )),
            }
        ),
        entry!(
            "quick_confirm_delete",
            "the confirmation of deleting the preset in slot 1",
            "dials",
            Menu::Confirm {
                slot: 1,
                old: Some(coding()),
                new: None,
            }
        ),
    ]
}
