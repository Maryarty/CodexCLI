//! The reasoning shortcuts (`Alt+,` / `Alt+.`, `Shift+Down` / `Shift+Up`): one step of the
//! active model's reasoning effort, on the efforts the model advertises, with the advanced
//! ones last and Ultra never reached by a raise (oracle `chatwidget/reasoning_shortcuts.rs`).

use super::models::{Catalog, Current, Effort, Preset, is_advanced};

/// The way a shortcut steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Lower,
    Raise,
}

/// What a shortcut came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The thread takes this effort (in Plan mode its override only).
    Set(Effort),
    /// Nothing changes; the history says why.
    Say(super::Note),
}

/// The efforts the model advertises, the advanced ones after the standard ones and Ultra last
/// (`reasoning_choices`).
fn choices(preset: &Preset) -> Vec<Effort> {
    let (mut standard, mut advanced): (Vec<Effort>, Vec<Effort>) = preset
        .efforts
        .iter()
        .map(|option| option.effort.clone())
        .partition(|effort| !is_advanced(effort));
    advanced.sort_by_key(|effort| matches!(effort, Effort::Ultra));
    standard.extend(advanced);
    if standard.is_empty() {
        standard.push(preset.default_effort.clone());
    }
    standard
}

/// The place the shortcut starts from: the configured effort when the model advertises it,
/// else the model's default, else its first effort.
fn anchor(choices: &[Effort], preset: &Preset, configured: Option<&Effort>) -> Effort {
    let configured = configured.unwrap_or(&preset.default_effort);
    if choices.contains(configured) {
        configured.clone()
    } else if choices.contains(&preset.default_effort) {
        preset.default_effort.clone()
    } else {
        choices.first().cloned().unwrap_or_else(|| preset.default_effort.clone())
    }
}

/// One step of the shortcut on the thread `current`, the catalog held. `started` is the
/// session being configured (`is_session_configured`).
pub fn step(catalog: &Catalog, current: &Current, direction: Direction, started: bool) -> Outcome {
    use super::Note;
    if !started {
        return Outcome::Say(Note::ReasoningDisabled);
    }
    let Some(preset) = catalog.presets.iter().find(|preset| preset.model == current.model) else {
        return Outcome::Say(Note::ReasoningUnavailable(current.model.clone()));
    };
    let choices = choices(preset);
    let at = anchor(&choices, preset, current.effort.as_ref());
    let place = choices.iter().position(|choice| choice == &at);
    let next = place.and_then(|index| match direction {
        Direction::Lower => index.checked_sub(1).and_then(|index| choices.get(index)),
        Direction::Raise => choices.get(index + 1),
    });
    match next {
        None => Outcome::Say(Note::ReasoningBound { direction, effort: at }),
        Some(Effort::Ultra) if direction == Direction::Raise => {
            Outcome::Say(Note::UltraAvailable(current.model.clone()))
        }
        Some(effort) => Outcome::Set(effort.clone()),
    }
}
