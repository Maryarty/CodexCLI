//! `/presets` (oracle `chatwidget/quick_dials.rs`, `app/quick_dials.rs`): the numbered prompts and
//! slash commands of the config (`tui.presets`), as typed pages of the sheet of the presets: the
//! slot picker (save, edit, delete), the sources among the current settings, the editor and the
//! confirmation. The pages are data (the slots, the dials, what is unavailable and why); the
//! words (`Save preset to slot`, `Replace preset 3?`, the reasons) are the view's. The shortcut
//! menu on the composer (`n`, `s`, `e`, `d`, a digit) is the view's: it asks for these pages
//! with `Intent::Quick`, with the gate (`quick_dials_available`) and the composer it knows.
//!
//! Which page leads where is the pure `Menu::choose` (a row picked) and `Menu::submit` (the
//! editor's text): a page to push over this one, or a verb for the model (`Ask`). The stack of
//! the pages is the sheet's own (a nested Navigator, ADR-013 O1); the model keeps what IO needs:
//! the slots, the write and the notes.
//!
//! The slots are the config's (`Quick::slots`, told by `Ev::Dials`: the host reads them with the
//! layer each comes from); a write is one `config/batchWrite` of `tui.presets.<slot>`
//! (`select::Command::Preset`), and the slot changes only when the config took it.

use std::collections::BTreeMap;

use super::models::{Catalog, Current};
use crate::screens::chat::{DEFAULT_STATUS_LINE_ITEMS, StatusFacts};

/// The slots, in the order the oracle lists them: 1 to 9, then 0.
pub const SLOTS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];

/// `DEFAULT_TERMINAL_TITLE_ITEMS`.
const DEFAULT_TITLE_ITEMS: [&str; 3] = ["activity", "thread-name", "project-name"];

/// The most a stored input holds (`QuickDial::validate`).
pub const MAX_INPUT_BYTES: usize = 8_000;

/// The id of the one row of the confirmation (`Intent::Choose`).
pub const CONFIRM: &str = "preset.confirm";

/// The prefix of the id of a slot row (the slot's digit follows).
const SLOT_ROW: &str = "preset.slot:";

/// The ids of the rows of the sources menu (the oracle's order).
pub const MODEL: &str = "preset.model";
pub const PERMISSIONS: &str = "preset.permissions";
pub const TERMINAL_TITLE: &str = "preset.title";
pub const STATUS_LINE: &str = "preset.statusline";

/// Why an input is not a preset (`QuickDial::validate`); the words are the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    /// `Preset input cannot be empty.`
    Empty,
    /// `Presets support prompts and slash commands, not shell commands.`
    Shell,
    /// `Preset input cannot exceed 8,000 UTF-8 bytes.`
    TooLong,
}

/// One reusable input (`QuickDial`): a prompt or a slash command, with an optional short name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dial {
    pub input: String,
    pub label: Option<String>,
}

impl Dial {
    pub fn new(input: impl Into<String>) -> Self {
        Dial {
            input: input.into(),
            label: None,
        }
    }

    /// `QuickDial::validate`.
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.input.trim().is_empty() {
            return Err(Invalid::Empty);
        }
        if self.input.trim_start().starts_with('!') {
            return Err(Invalid::Shell);
        }
        if self.input.len() > MAX_INPUT_BYTES {
            return Err(Invalid::TooLong);
        }
        Ok(())
    }

    /// `quick_dial_label`: the label, or the input, as its first line.
    pub fn label_line(&self) -> String {
        self.label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or(&self.input)
            .trim_start()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    }

    /// Whether the label differs from the input line (the picker shows the input under it).
    pub fn has_label(&self) -> bool {
        self.label
            .as_deref()
            .is_some_and(|label| !label.trim().is_empty())
    }
}

/// Which configuration layers define a slot (`save_quick_dial`: a slot is the user config's to
/// replace when its highest layer is the user's, and to clear when no other layer has it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Only the user config has it.
    User,
    /// The user config has it, and so does a layer below.
    Shadowing,
    /// A layer above the user config has it, or the user config does not.
    Outside,
}

/// A saved slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub slot: u8,
    pub dial: Dial,
    pub origin: Origin,
}

/// What a slot picker does with the slot picked (`QuickDialPickerMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerMode {
    /// `Save preset to slot`: all ten slots; a saved one asks to confirm the replacement.
    Save,
    /// `Edit a preset`: the saved slots.
    Edit,
    /// `Delete a preset`: the saved slots.
    Clear,
}

/// Where an editor's text goes (`QuickDialEditorTarget`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A new preset: the slot is chosen after the text.
    PickSlot,
    /// The text of this slot.
    Slot(u8),
}

/// What one of the confirmation's actions is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Replace,
    Delete,
    Save,
}

/// A menu of the presets family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// The slot picker; `dial` is what `PickerMode::Save` keeps (the saved slot's own dial otherwise).
    Slots {
        mode: PickerMode,
        dial: Option<Dial>,
    },
    /// `Start preset from current settings`: the four sources.
    Sources(Vec<SourceRow>),
    /// The text prompt (`New preset`, `Edit preset {slot}`): its answer is `Menu::submit`.
    Editor { target: Target, dial: Dial },
    /// `{Replace|Delete|Save} preset {slot}?`: one row, Enter confirms; `old` is what the slot
    /// holds, `new` what it will (none: deleted).
    Confirm {
        slot: u8,
        old: Option<Dial>,
        new: Option<Dial>,
    },
}

/// A row of the slot picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotRow {
    pub id: String,
    pub slot: u8,
    pub saved: Option<Dial>,
    /// `PickerMode::Save` with nothing to save: every row is disabled.
    pub disabled: bool,
}

/// The slot pickers' rows and the row the cursor opens on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub rows: Vec<SlotRow>,
    /// `PickerMode::Save` opens on the first empty slot.
    pub initial: Option<usize>,
}

/// Which current setting a source row takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Model,
    Permissions,
    TerminalTitle,
    StatusLine,
}

impl Source {
    fn id(self) -> &'static str {
        match self {
            Source::Model => MODEL,
            Source::Permissions => PERMISSIONS,
            Source::TerminalTitle => TERMINAL_TITLE,
            Source::StatusLine => STATUS_LINE,
        }
    }
}

/// Why a source cannot be saved as a preset; the words are the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// `The current model cannot be recalled with /model.`
    Model,
    /// `Choose an available reasoning effort with /model before saving.`
    Effort,
    /// `Choose an available speed with /model before saving.`
    Speed,
    /// `Select a named permission profile with /permissions first.`
    NoProfile,
    /// `The approval policy cannot be recalled.`
    ApprovalPolicy,
    /// `The permission profile name cannot be recalled.`
    ProfileName,
    /// `Choose available terminal-title items with /title before saving.`
    TitleItems,
    /// `Choose available status-line items with /statusline before saving.`
    StatusItems,
    /// The command did not validate as a preset.
    Invalid(Invalid),
}

/// A row of the sources menu: the dial the current settings make, or why there is none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRow {
    pub source: Source,
    pub dial: Result<Dial, Unavailable>,
}

impl SourceRow {
    pub fn id(&self) -> &'static str {
        self.source.id()
    }
}

/// The settings the sources read (`current_*_quick_dial`), as the view gathers them: the thread's
/// model (`Current`), the config's speed, permissions and status layouts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The model and effort of the thread (`reserve`: only the Reserve may be picked).
    pub current: Current,
    pub service_tier: Option<String>,
    pub fast_mode: bool,
    pub ultrafast_mode: bool,
    /// The active permission profile's id.
    pub profile: Option<String>,
    /// `untrusted`, `on-request`, `never`, or the granular policy as JSON.
    pub approval: Option<String>,
    pub auto_review: bool,
    pub title: Vec<String>,
    pub status: Vec<String>,
    pub colors: bool,
}

impl Settings {
    /// What the chat's status facts hold; `current` is the thread's model as the opener knows it
    /// (the Reserve restriction is not among the facts).
    pub fn from_facts(facts: &StatusFacts, current: Current) -> Self {
        let approval = match facts.approval_policy.as_deref() {
            Some(name @ ("untrusted" | "on-request" | "never")) => Some(name.to_owned()),
            _ => facts.approval_granular.clone(),
        };
        Settings {
            current,
            service_tier: facts.service_tier.clone(),
            fast_mode: facts.fast_mode,
            ultrafast_mode: facts.ultrafast_mode,
            profile: facts.permission_profile.clone(),
            approval,
            auto_review: facts.auto_review,
            title: facts.terminal_title.clone().unwrap_or_else(|| {
                DEFAULT_TITLE_ITEMS
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            }),
            status: facts.items.clone().unwrap_or_else(|| {
                DEFAULT_STATUS_LINE_ITEMS
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            }),
            colors: facts.colors,
        }
    }
}

/// What the view's composer holds when an editor opens on it (`composer_quick_dial_text`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composer {
    pub text: String,
    /// Local or remote images are attached.
    pub images: bool,
    /// Linked mentions are bound.
    pub mentions: bool,
}

/// `quick_dials_available`: whether presets can be used where the user is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Ready,
    /// A conversation that takes no direct input: a side conversation, a blocked composer.
    Blocked,
    /// The session is not configured yet.
    Starting,
}

/// What `Intent::Quick` asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Open {
    /// `/presets`: the shortcut menu over the composer (`open_quick_dial_menu`). The menu is the
    /// composer's drawn state: the model says only whether it may open (the gate's notes), and
    /// opens no sheet.
    Menu,
    /// `n`, a digit with no preset: the editor on the composer's text, on `dial`, or on the
    /// slot's own preset (`OpenQuickDialEditor`).
    Editor {
        target: Target,
        dial: Option<Dial>,
        composer: Composer,
    },
    /// `s`: the sources among the current settings.
    Prefills(Box<Settings>),
    /// `e`, `d`.
    Picker(PickerMode),
    /// The slot picker for `dial` (`OpenQuickDialSavePicker`).
    Save(Dial),
    /// A digit with a preset: its input is submitted as typed (`RecallQuickDial`).
    Recall(u8),
}

/// What a pick of a row, or the text of the editor, leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// A page opens over this one (Esc shows this one again).
    Push(Menu),
    /// The model is told (`Intent::Preset`).
    Ask(Ask),
}

/// What a page of the sheet asks of the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The change is settled: the sheet closes and the config is asked to write it
    /// (`save_quick_dial`).
    Write(Write),
    /// The text typed is not a preset: the model says why in the history and the editor stays.
    Invalid(Invalid),
}

impl Menu {
    /// The slot picker's rows over the slots saved.
    pub fn page(&self, quick: &Quick) -> Option<Page> {
        let Menu::Slots { mode, dial } = self else {
            return None;
        };
        let rows = SLOTS
            .iter()
            .filter_map(|&slot| {
                let saved = quick.get(slot).map(|saved| saved.dial.clone());
                if *mode != PickerMode::Save && saved.is_none() {
                    return None;
                }
                Some(SlotRow {
                    id: slot_id(slot),
                    slot,
                    disabled: *mode == PickerMode::Save && dial.is_none(),
                    saved,
                })
            })
            .collect::<Vec<_>>();
        let initial = (*mode == PickerMode::Save).then(|| {
            SLOTS
                .iter()
                .position(|&slot| quick.get(slot).is_none())
                .unwrap_or(0)
        });
        Some(Page { rows, initial })
    }

    /// The action a confirmation asks for.
    pub fn action(&self) -> Option<Action> {
        match self {
            Menu::Confirm {
                old: Some(_),
                new: Some(_),
                ..
            } => Some(Action::Replace),
            Menu::Confirm {
                old: Some(_),
                new: None,
                ..
            } => Some(Action::Delete),
            Menu::Confirm {
                old: None,
                new: Some(_),
                ..
            } => Some(Action::Save),
            _ => None,
        }
    }

    /// Where a row of this page leads (`None`: nothing, a disabled or unknown row).
    pub fn choose(&self, id: &str, quick: &Quick) -> Option<Chosen> {
        match self {
            Menu::Slots { mode, dial } => {
                let slot = slot_of(id)?;
                let saved = quick.get(slot).map(|saved| saved.dial.clone());
                match mode {
                    PickerMode::Save => {
                        let new = dial.clone()?;
                        Some(match saved {
                            Some(old) => Chosen::Push(Menu::Confirm {
                                slot,
                                old: Some(old),
                                new: Some(new),
                            }),
                            None => Chosen::Ask(Ask::Write(Write {
                                slot,
                                dial: Some(new),
                            })),
                        })
                    }
                    PickerMode::Edit => Some(Chosen::Push(Menu::Editor {
                        target: Target::Slot(slot),
                        dial: saved?,
                    })),
                    PickerMode::Clear => Some(Chosen::Push(Menu::Confirm {
                        slot,
                        old: Some(saved?),
                        new: None,
                    })),
                }
            }
            Menu::Sources(rows) => {
                let row = rows.iter().find(|row| row.id() == id)?;
                let dial = row.dial.clone().ok()?;
                Some(Chosen::Push(Menu::Editor {
                    target: Target::PickSlot,
                    dial,
                }))
            }
            Menu::Confirm { slot, new, .. } if id == CONFIRM => {
                Some(Chosen::Ask(Ask::Write(Write {
                    slot: *slot,
                    dial: new.clone(),
                })))
            }
            Menu::Confirm { .. } | Menu::Editor { .. } => None,
        }
    }

    /// Where the text of the editor leads (`None`: this is not an editor, or the text is blank:
    /// nothing is submitted): the slot picker for a new preset, the confirmation of the change of
    /// a slot, or the reason the text is not a preset (`open_quick_dial_save_picker`,
    /// `confirm_quick_dial`: the editor stays under what opens).
    pub fn submit(&self, text: &str, quick: &Quick) -> Option<Chosen> {
        let Menu::Editor { target, dial } = self else {
            return None;
        };
        if text.trim().is_empty() {
            return None;
        }
        let new = Dial {
            input: text.to_string(),
            label: dial.label.clone(),
        };
        if let Err(reason) = new.validate() {
            return Some(Chosen::Ask(Ask::Invalid(reason)));
        }
        Some(Chosen::Push(match target {
            Target::PickSlot => Menu::Slots {
                mode: PickerMode::Save,
                dial: Some(new),
            },
            Target::Slot(slot) => Menu::Confirm {
                slot: *slot,
                old: quick.get(*slot).map(|saved| saved.dial.clone()),
                new: Some(new),
            },
        }))
    }
}

/// The id of the row of `slot`.
pub fn slot_id(slot: u8) -> String {
    format!("{SLOT_ROW}{slot}")
}

fn slot_of(id: &str) -> Option<u8> {
    let slot = id.strip_prefix(SLOT_ROW)?.parse().ok()?;
    SLOTS.contains(&slot).then_some(slot)
}

/// The saved slots, the writes asked of the config and their answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quick {
    slots: BTreeMap<u8, Slot>,
    /// Writes the app has not yet sent.
    unsent: Vec<Write>,
}

/// One write of a slot (`None`: the slot is deleted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    pub slot: u8,
    pub dial: Option<Dial>,
}

impl Write {
    pub(crate) fn check_against(&self, quick: &Quick) -> Result<(), super::Note> {
        quick
            .check(self.slot, self.dial.as_ref())
            .map_err(|refused| match refused {
                Refused::Outside(slot) => super::Note::PresetOutside(slot),
                Refused::Invalid(reason) => super::Note::PresetInvalid(reason),
            })
    }

    pub(crate) fn retry_editor(self) -> Option<Menu> {
        self.dial.map(|dial| Menu::Editor {
            target: Target::Slot(self.slot),
            dial,
        })
    }
}

impl Quick {
    /// The slot's preset, if saved.
    pub fn get(&self, slot: u8) -> Option<&Slot> {
        self.slots.get(&slot)
    }

    /// The saved presets in slot order (1 to 9, then 0): what the shortcut menu lists.
    pub fn saved(&self) -> Vec<&Slot> {
        SLOTS
            .iter()
            .filter_map(|slot| self.slots.get(slot))
            .collect()
    }

    pub(super) fn set(&mut self, slots: Vec<Slot>) {
        self.slots = slots.into_iter().map(|slot| (slot.slot, slot)).collect();
    }

    /// `save_quick_dial` before it writes: a slot of another layer is not the user config's to
    /// change, and the input must be a preset.
    pub(super) fn check(&self, slot: u8, dial: Option<&Dial>) -> Result<(), Refused> {
        let origin = self.get(slot).map(|saved| saved.origin);
        let outside = match dial {
            Some(_) => origin == Some(Origin::Outside),
            None => matches!(origin, Some(Origin::Outside | Origin::Shadowing)),
        };
        if outside {
            return Err(Refused::Outside(slot));
        }
        if let Some(Err(reason)) = dial.map(Dial::validate) {
            return Err(Refused::Invalid(reason));
        }
        Ok(())
    }

    pub(super) fn want(&mut self, write: Write) {
        self.unsent.push(write);
    }

    /// The writes the app has still to send, once each.
    pub fn take_wanted(&mut self) -> Vec<Write> {
        std::mem::take(&mut self.unsent)
    }

    pub(crate) fn write_result(
        &mut self,
        write: &Write,
        result: Result<(), String>,
    ) -> super::Note {
        if let Err(error) = result {
            return super::Note::PresetFailed {
                slot: write.slot,
                error,
            };
        }
        self.written(write.clone());
        if write.dial.is_some() {
            super::Note::PresetSaved(write.slot)
        } else {
            super::Note::PresetDeleted(write.slot)
        }
    }

    /// The config took the write: the slot follows it (a saved slot has a label, empty when it
    /// has none).
    pub(super) fn written(&mut self, write: Write) {
        match write.dial {
            Some(mut dial) => {
                dial.label.get_or_insert_default();
                self.slots.insert(
                    write.slot,
                    Slot {
                        slot: write.slot,
                        dial,
                        origin: Origin::User,
                    },
                );
            }
            None => {
                self.slots.remove(&write.slot);
            }
        }
    }
}

/// Why a write is not asked for.
pub(super) enum Refused {
    Outside(u8),
    Invalid(Invalid),
}

/// The four sources of the current settings, in the oracle's order.
pub(super) fn sources(settings: &Settings, catalog: &Catalog) -> Vec<SourceRow> {
    vec![
        SourceRow {
            source: Source::Model,
            dial: model_dial(settings, catalog),
        },
        SourceRow {
            source: Source::Permissions,
            dial: permissions_dial(settings),
        },
        SourceRow {
            source: Source::TerminalTitle,
            dial: items_dial(
                "/title",
                &settings.title,
                TERMINAL_TITLE_ITEMS,
                None,
                Unavailable::TitleItems,
            ),
        },
        SourceRow {
            source: Source::StatusLine,
            dial: items_dial(
                "/statusline",
                &settings.status,
                STATUS_LINE_ITEMS,
                Some(if settings.colors {
                    "--colors"
                } else {
                    "--no-colors"
                }),
                Unavailable::StatusItems,
            ),
        },
    ]
}

fn validated(input: String) -> Result<Dial, Unavailable> {
    let dial = Dial::new(input);
    dial.validate().map_err(Unavailable::Invalid)?;
    Ok(dial)
}

/// `current_model_quick_dial`: `/model <model> [--reasoning=<effort>] --speed=<tier>`.
fn model_dial(settings: &Settings, catalog: &Catalog) -> Result<Dial, Unavailable> {
    let current = &settings.current;
    let preset = catalog
        .presets
        .iter()
        .find(|preset| preset.model == current.model && preset.show_in_picker)
        .filter(|_| current.reserve.is_none())
        .ok_or(Unavailable::Model)?;
    let mut input = ["/model ", current.model.as_str()].concat();
    if let Some(effort) = &current.effort {
        let supported = if preset.efforts.is_empty() {
            *effort == preset.default_effort
        } else {
            preset.efforts.iter().any(|option| option.effort == *effort)
        };
        if !supported {
            return Err(Unavailable::Effort);
        }
        input.push_str(" --reasoning=");
        input.push_str(&effort.to_string());
    }
    let speed = settings
        .service_tier
        .as_deref()
        .filter(|tier| *tier != codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE)
        .unwrap_or("standard");
    let enabled = |tier: &str| match tier {
        "flex" => true,
        "ultrafast" => settings.ultrafast_mode,
        _ => settings.fast_mode,
    };
    if !matches!(speed, "standard" | "flex")
        && !preset
            .service_tiers
            .iter()
            .any(|tier| tier.id == speed && enabled(&tier.id))
    {
        return Err(Unavailable::Speed);
    }
    input.push_str(" --speed=");
    input.push_str(speed);
    validated(input)
}

/// The current permissions as `/permissions <profile> --approval-policy=<policy> --reviewer=<who>`.
fn permissions_dial(settings: &Settings) -> Result<Dial, Unavailable> {
    let profile = settings.profile.as_deref().ok_or(Unavailable::NoProfile)?;
    let approval = settings
        .approval
        .as_deref()
        .ok_or(Unavailable::ApprovalPolicy)?;
    let profile = quote(profile).ok_or(Unavailable::ProfileName)?;
    let approval = quote(approval).ok_or(Unavailable::ApprovalPolicy)?;
    let reviewer = if settings.auto_review {
        "auto_review"
    } else {
        "user"
    };
    validated(format!(
        "/permissions {profile} --approval-policy={approval} --reviewer={reviewer}"
    ))
}

/// `status_surface_quick_dial`: `<command> <item>... [suffix]`, `--clear` when no item is set;
/// every configured id must be an item (an alias is written as the item's own name).
fn items_dial(
    command: &str,
    ids: &[String],
    table: &[(&str, &[&str])],
    suffix: Option<&str>,
    invalid: Unavailable,
) -> Result<Dial, Unavailable> {
    let mut items = Vec::new();
    for id in ids {
        let (name, _) = table
            .iter()
            .find(|(name, aliases)| *name == id || aliases.contains(&id.as_str()))
            .ok_or(invalid)?;
        items.push(*name);
    }
    let mut input = command.to_string();
    if items.is_empty() {
        input.push_str(" --clear");
    } else {
        for item in items {
            input.push(' ');
            input.push_str(item);
        }
    }
    if let Some(suffix) = suffix {
        input.push(' ');
        input.push_str(suffix);
    }
    validated(input)
}

/// `shlex::try_quote`: the word as a shell takes it back, none for a word with a NUL.
fn quote(word: &str) -> Option<String> {
    if word.contains('\0') {
        return None;
    }
    if word.is_empty() {
        return Some("''".to_owned());
    }
    let plain = |c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | '/' | ':' | '@' | ']' | '_')
    };
    if word.chars().all(plain) {
        return Some(word.to_owned());
    }
    if !word.contains('\'') {
        return Some(format!("'{word}'"));
    }
    let mut out = String::from("\"");
    for c in word.chars() {
        if matches!(c, '$' | '`' | '"' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    Some(out)
}

/// `TerminalTitleItem`: the name each item is written with, and the aliases it is read from.
const TERMINAL_TITLE_ITEMS: &[(&str, &[&str])] = &[
    ("app-name", &[]),
    ("project-name", &["project"]),
    ("current-dir", &[]),
    ("activity", &["spinner"]),
    ("run-state", &["status"]),
    ("thread-name", &[]),
    ("thread-title", &["thread"]),
    ("git-branch", &[]),
    ("context-remaining", &[]),
    ("context-used", &["context-usage"]),
    ("five-hour-limit", &[]),
    ("weekly-limit", &[]),
    ("codex-version", &[]),
    ("used-tokens", &[]),
    ("total-input-tokens", &[]),
    ("total-output-tokens", &[]),
    ("thread-credits", &[]),
    ("estimated-thread-cost", &[]),
    ("thread-id", &["session-id"]),
    ("fast-mode", &[]),
    ("daybreak", &[]),
    ("model", &["model-name"]),
    ("model-with-reasoning", &[]),
    ("reasoning", &[]),
    ("task-progress", &[]),
];

/// `StatusLineItem`: the name each item is written with, and the aliases it is read from.
const STATUS_LINE_ITEMS: &[(&str, &[&str])] = &[
    ("model", &["model-name"]),
    ("model-with-reasoning", &[]),
    ("reasoning", &[]),
    ("current-dir", &[]),
    ("project-name", &["project", "project-root"]),
    ("hostname", &[]),
    ("git-branch", &[]),
    ("pull-request-number", &[]),
    ("branch-changes", &[]),
    ("run-state", &["status"]),
    ("permissions", &[]),
    ("approval-mode", &["approval"]),
    ("context-remaining", &[]),
    ("context-used", &["context-usage"]),
    ("five-hour-limit", &[]),
    ("weekly-limit", &[]),
    ("codex-version", &[]),
    ("context-window-size", &[]),
    ("used-tokens", &[]),
    ("total-input-tokens", &[]),
    ("total-output-tokens", &[]),
    ("thread-credits", &[]),
    ("estimated-thread-cost", &[]),
    ("thread-id", &["session-id"]),
    ("fast-mode", &[]),
    ("daybreak", &[]),
    ("raw-output", &[]),
    ("thread-name", &[]),
    ("thread-title", &[]),
    ("workspace-headline", &[]),
    ("task-progress", &[]),
    ("profile", &[]),
    ("provider", &["model-provider"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_words_are_quoted_as_shlex_does() {
        assert_eq!(quote(":workspace").as_deref(), Some(":workspace"));
        assert_eq!(quote("team review").as_deref(), Some("'team review'"));
        assert_eq!(quote("on-request").as_deref(), Some("on-request"));
        assert_eq!(quote("it's").as_deref(), Some("\"it's\""));
        assert_eq!(quote("").as_deref(), Some("''"));
        assert_eq!(quote("a\0b"), None);
    }

    #[test]
    fn a_preset_is_a_nonempty_prompt_or_slash_command() {
        assert_eq!(Dial::new("  ").validate(), Err(Invalid::Empty));
        assert_eq!(Dial::new(" !ls").validate(), Err(Invalid::Shell));
        assert_eq!(
            Dial::new("x".repeat(MAX_INPUT_BYTES + 1)).validate(),
            Err(Invalid::TooLong)
        );
        assert_eq!(Dial::new("/compact").validate(), Ok(()));
    }

    #[test]
    fn the_label_line_is_the_first_line_of_the_label_or_the_input() {
        let mut dial = Dial::new("  first\nsecond");
        assert_eq!(dial.label_line(), "first");
        dial.label = Some("Name".into());
        assert_eq!(dial.label_line(), "Name");
        dial.label = Some("  ".into());
        assert_eq!(dial.label_line(), "first");
    }
}
