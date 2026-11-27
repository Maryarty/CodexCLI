//! Model context rendering carried from `tui/src/ide_context/prompt.rs`.

use super::IdeContext;

const MAX_ACTIVE_SELECTION_CHARS: usize = 40_000;
const MAX_ACTIVE_FILE_PATH_BYTES: usize = 8_192;
const MAX_SELECTION_RANGE_BYTES: usize = 20_000;
const MAX_OPEN_TABS: usize = 100;
const MAX_OPEN_TABS_CHARS: usize = 20_000;

pub(super) fn render_prompt_context(context: &IdeContext) -> Option<String> {
    let mut ide_context_section = String::new();

    if let Some(active_file) = &context.active_file {
        let path = &active_file.descriptor.path;
        let path = &path[..path.floor_char_boundary(MAX_ACTIVE_FILE_PATH_BYTES)];
        ide_context_section.push_str(&format!("\n## Active file: {path}\n"));
        let selected_ranges = if active_file.selections.is_empty() {
            std::slice::from_ref(&active_file.selection)
        } else {
            active_file.selections.as_slice()
        }
        .iter()
        .filter(|range| range.start != range.end);

        let first_two = selected_ranges.clone().take(2).count();
        if first_two > 0 && (active_file.active_selection_content.is_empty() || first_two == 2) {
            if first_two == 1 {
                ide_context_section.push_str("\n## Active selection range:\n");
            } else {
                ide_context_section.push_str("\n## Active selection ranges:\n");
            }
            let start = ide_context_section.len();
            for range in selected_ranges {
                // Bound repeated paths even if the extension sends many ranges.
                let start_line = u64::from(range.start.line) + 1;
                let start_column = u64::from(range.start.character) + 1;
                let end_line = u64::from(range.end.line) + 1;
                let end_column = u64::from(range.end.character) + 1;
                let line = format!(
                    "- {path}: line {start_line}, column {start_column} to line {end_line}, column {end_column}\n"
                );
                if ide_context_section.len() - start + line.len() > MAX_SELECTION_RANGE_BYTES {
                    ide_context_section.push_str("[Additional selection ranges omitted.]\n");
                    break;
                }
                ide_context_section.push_str(&line);
            }
        }
    }

    if let Some(active_file) = &context.active_file
        && !active_file.active_selection_content.is_empty()
    {
        ide_context_section.push_str("\n## Active selection of the file:\n");
        let selection = active_file.active_selection_content.as_str();
        if let Some((truncate_at, _)) = selection.char_indices().nth(MAX_ACTIVE_SELECTION_CHARS) {
            ide_context_section.push_str(&selection[..truncate_at]);
            ide_context_section.push_str(&format!(
                "\n[Selection truncated to {MAX_ACTIVE_SELECTION_CHARS} characters.]\n"
            ));
        } else {
            ide_context_section.push_str(selection);
        }
    }

    if !context.open_tabs.is_empty() {
        ide_context_section.push_str("\n## Open tabs:\n");
        let mut rendered_tabs = 0;
        let mut rendered_tab_chars = 0;
        for tab in &context.open_tabs {
            if rendered_tabs >= MAX_OPEN_TABS {
                break;
            }

            // A single tab description may consume the remaining budget.
            let bytes = tab
                .label
                .len()
                .saturating_add(tab.path.len())
                .saturating_add(5);
            if bytes > MAX_OPEN_TABS_CHARS - rendered_tab_chars {
                break;
            }

            ide_context_section.push_str(&format!("- {}: {}\n", tab.label, tab.path));
            rendered_tabs += 1;
            rendered_tab_chars += bytes;
        }

        let omitted_tabs = context.open_tabs.len() - rendered_tabs;
        if omitted_tabs > 0 {
            ide_context_section.push_str(&format!("[{omitted_tabs} open tabs omitted.]\n"));
        }
    }

    if ide_context_section.is_empty() {
        None
    } else {
        Some(format!(
            "# Context from my IDE setup:\n{ide_context_section}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ide_context_prompt_bounds_extension_text_and_selection_ranges() -> anyhow::Result<()> {
        let range = json!({
            "start": { "line": u32::MAX, "character": 0 },
            "end": { "line": u32::MAX, "character": 5 },
        });
        let context = serde_json::from_value(json!({
            "activeFile": {
                "label": "main.rs",
                "path": "雪".repeat(MAX_ACTIVE_FILE_PATH_BYTES),
                "selection": range.clone(),
                "selections": vec![range; 100],
                "activeSelectionContent": "a".repeat(MAX_ACTIVE_SELECTION_CHARS + 10),
            },
            "openTabs": [{ "label": "large", "path": "b".repeat(MAX_OPEN_TABS_CHARS) }],
        }))?;
        let prompt = render_prompt_context(&context).unwrap_or_else(|| panic!("IDE context"));
        assert!(prompt.contains("line 4294967296, column 1"));
        assert!(prompt.contains("[Additional selection ranges omitted.]"));
        assert!(prompt.contains("[Selection truncated to 40000 characters.]"));
        assert!(prompt.contains("[1 open tabs omitted.]"));
        assert!(prompt.len() < 70_000);
        Ok(())
    }
}
