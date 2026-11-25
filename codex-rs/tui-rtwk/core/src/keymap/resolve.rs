//! Config to `Keymap`: precedence, chords, the leader key, new defaults that
//! yield to the user's keys, and the validation that keeps one key from
//! meaning two things.
//!
//! The oracle (`tui/src/keymap.rs`, `keymap/chords.rs`, `keymap/leader.rs`)
//! writes each of these passes once per context by hand; here the passes run
//! over the catalogue's rows, with the same rules and the same error texts.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use super::catalogue::{
    ALLOWED_OVERLAPS, ActionId, CTRL_Z_RESERVED, Context, DEFAULT_LEADER, GLOBAL_FALLBACK,
    LEADER_DEFAULTS, LIST_RESERVED, MAIN_RESERVED, PRINTABLE_RESERVED, ROWS, RUNTIME_ONLY, SCOPES,
    SIDE_PANE, Scope, TRANSCRIPT_RESERVED, scopes_conflict, yields_to_configured,
};
use super::key::{Binding, Key, Mods, Spec, Stroke, parse_spec};
use super::spec::KeymapSpec;

/// The most chords the leader keys expand to (`MAX_LEADER_CHORD_BINDINGS`).
const MAX_LEADER_CHORDS: usize = 1024;

/// The config's keyword for a leader chord: `leader c`.
const LEADER: &str = "leader ";

const DOCS: &str = "See the Codex keymap documentation for supported actions and examples.";

/// What was wrong with a binding written in the config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// `leader ...` that is not `leader` and one key.
    Leader,
    /// Two words that are not a two-stroke chord.
    Chord,
    /// One word that is not a key.
    Single,
}

/// Why a config does not make a keymap; [`Display`](fmt::Display) names the config path and
/// the next step, as the oracle's text does. The variants carry what the text is made of (the
/// action, the key, the action that holds it) so a caller can tell the causes apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeymapError {
    /// A binding that does not parse (`parse_configured`).
    Invalid {
        id: ActionId,
        raw: String,
        shape: Shape,
    },
    /// `tui.keymap.global.leader` with more than one key.
    LeaderNotSingle,
    /// A leader that does not parse.
    LeaderInvalid { raw: String },
    /// A leader that would take plain text input.
    LeaderPlainText { label: String },
    /// A leader of ctrl-z.
    LeaderCtrlZ { label: String },
    /// A leader the app reserves.
    LeaderReserved {
        label: String,
        reserved: &'static str,
    },
    /// More leader chords than `MAX_LEADER_CHORDS`.
    TooManyLeaderChords,
    /// A printable key on an action that sits where text is typed.
    PrintableReserved(ActionId),
    /// Ctrl-z on an action, which suspends the terminal.
    CtrlZReserved(ActionId),
    /// An AltGr character on `global.open_agents`.
    OpenAgentsAltGr,
    /// A key of the agents list that is for editing.
    AgentsEditing { name: &'static str },
    /// Ctrl-z on an action of the agents list.
    AgentsCtrlZ { name: &'static str },
    /// A key the app or the transcript takes for itself.
    ReservedKey {
        scope: &'static str,
        id: ActionId,
        reserved: &'static str,
    },
    /// Two actions of a scope on one key.
    Ambiguous {
        scope: &'static str,
        bare: bool,
        shadows: bool,
        first: ActionId,
        second: ActionId,
    },
    /// A chord whose prefix is AltGr text input.
    ChordAltGr { id: ActionId, text: String },
    /// A chord whose prefix is plain text input.
    ChordPlainPrefix { id: ActionId, text: String },
    /// A chord that uses plain `esc`.
    ChordEsc { id: ActionId, text: String },
    /// A chord of the command center that starts with backspace.
    ChordBackspace { id: ActionId, text: String },
    /// A chord that uses ctrl-z.
    ChordCtrlZ { id: ActionId, text: String },
    /// A chord that uses a key the app reserves.
    ChordReserved {
        id: ActionId,
        text: String,
        reserved: &'static str,
    },
    /// A chord that repeats the leader.
    ChordLeaderRepeat { id: ActionId, text: String },
    /// A chord whose prefix is a single key of another action.
    ChordShadows {
        id: ActionId,
        text: String,
        shadowed: ActionId,
    },
    /// A chord another action already has.
    ChordTaken {
        id: ActionId,
        text: String,
        previous: ActionId,
    },
}

impl KeymapError {
    /// The text the user is shown.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeymapError::Invalid { id, raw, shape } => {
                let path = id.config_path();
                match shape {
                    Shape::Leader => write!(
                        f,
                        "Invalid `{path}` = `{raw}`. Use `leader` followed by one key, such as \
`leader c`."
                    ),
                    Shape::Chord => write!(
                        f,
                        "Invalid `{path}` = `{raw}`. Use a single key such as `ctrl-a` or a \
two-stroke chord such as `ctrl-x ctrl-t`."
                    ),
                    Shape::Single => write!(
                        f,
                        "Invalid `{path}` = `{raw}`. Use values like `ctrl-a`, `shift-enter`, \
or `page-down`. {DOCS}"
                    ),
                }
            }
            KeymapError::LeaderNotSingle => {
                f.write_str("Invalid `tui.keymap.global.leader`: use a single prefix key.")
            }
            KeymapError::LeaderInvalid { raw } => write!(
                f,
                "Invalid `tui.keymap.global.leader` = `{raw}`. Use values like \
`ctrl-a`, `shift-enter`, or `page-down`. {DOCS}"
            ),
            KeymapError::LeaderPlainText { label } => write!(
                f,
                "Invalid `tui.keymap.global.leader` = `{label}`: use ctrl, alt, super, or a \
non-character key so ordinary text input is not intercepted. Ctrl-alt characters may be AltGr \
input on Windows."
            ),
            KeymapError::LeaderCtrlZ { label } => write!(
                f,
                "Invalid `tui.keymap.global.leader` = `{label}`: ctrl-z is reserved for \
suspending the terminal on Unix."
            ),
            KeymapError::LeaderReserved { label, reserved } => write!(
                f,
                "Invalid `tui.keymap.global.leader` = `{label}`: this key is reserved by \
`{reserved}`."
            ),
            KeymapError::TooManyLeaderChords => write!(
                f,
                "Too many leader shortcut combinations (maximum {MAX_LEADER_CHORDS}). \
Reduce `tui.keymap.global.leader` alternatives or `leader ...` bindings."
            ),
            KeymapError::PrintableReserved(id) => write!(
                f,
                "{}: printable keys are reserved for text input",
                id.config_path()
            ),
            KeymapError::CtrlZReserved(id) => {
                write!(f, "{}: ctrl-z is reserved for suspend", id.config_path())
            }
            KeymapError::OpenAgentsAltGr => f.write_str(
                "tui.keymap.global.open_agents: AltGr characters are reserved for text input",
            ),
            KeymapError::AgentsEditing { name } => write!(
                f,
                "tui.keymap.agents.{name}: AltGr and backspace are reserved for editing"
            ),
            KeymapError::AgentsCtrlZ { name } => write!(
                f,
                "tui.keymap.agents.{name}: ctrl-z is reserved for suspend"
            ),
            KeymapError::ReservedKey {
                scope,
                id,
                reserved,
            } => write!(
                f,
                "Ambiguous `tui.keymap.{scope}` bindings: `{}` uses a key reserved by \
`{reserved}`. Set a different key in `~/.codex/config.toml` and retry. {DOCS}",
                if *scope == "agents" || *scope == "pager" {
                    id.name.to_owned()
                } else {
                    short_name(*id)
                }
            ),
            KeymapError::Ambiguous {
                scope,
                bare,
                shadows,
                first,
                second,
            } => {
                let name = |id: ActionId| {
                    if *bare {
                        id.name.to_owned()
                    } else {
                        short_name(id)
                    }
                };
                let (first, second) = (name(*first), name(*second));
                write!(f, "Ambiguous `tui.keymap.{scope}` bindings: ")?;
                if *shadows {
                    write!(f, "`{first}` shadows `{second}` with the same key")?;
                } else {
                    write!(f, "`{first}` and `{second}` use the same key")?;
                }
                write!(
                    f,
                    ". Set unique keys in `~/.codex/config.toml` and retry. {DOCS}"
                )
            }
            KeymapError::ChordAltGr { id, text } => write!(
                f,
                "Invalid `{}` = `{text}`: a ctrl-alt character prefix may be AltGr text \
input on Windows. Choose a different chord and retry.",
                id.config_path()
            ),
            KeymapError::ChordPlainPrefix { id, text } => write!(
                f,
                "Invalid `{}` = `{text}`: a chord prefix outside Vim or the command center \
must use ctrl, alt, super, or a non-character key so ordinary text input is not intercepted.",
                id.config_path()
            ),
            KeymapError::ChordEsc { id, text } => write!(
                f,
                "Ambiguous `{}` = `{text}`: plain `esc` is reserved for cancelling a pending chord.",
                id.config_path()
            ),
            KeymapError::ChordBackspace { id, text } => write!(
                f,
                "Invalid `{}` = `{text}`: `backspace` is reserved for editing task input.",
                id.config_path()
            ),
            KeymapError::ChordCtrlZ { id, text } => write!(
                f,
                "Ambiguous `{}` = `{text}`: `ctrl-z` is reserved for suspending the terminal \
on Unix. Choose a different chord and retry.",
                id.config_path()
            ),
            KeymapError::ChordReserved { id, text, reserved } => write!(
                f,
                "Ambiguous `{}` = `{text}`: the chord uses the key reserved by \
`{reserved}`. Choose a different chord and retry.",
                id.config_path()
            ),
            KeymapError::ChordLeaderRepeat { id, text } => write!(
                f,
                "Ambiguous `{}` = `{text}`: repeating the leader is reserved for cancellation.",
                id.config_path()
            ),
            KeymapError::ChordShadows { id, text, shadowed } => write!(
                f,
                "Ambiguous `{}` = `{text}`: its prefix shadows `{}`. Unbind or remap the \
existing shortcut before using it as a chord prefix.",
                id.config_path(),
                shadowed.config_path()
            ),
            KeymapError::ChordTaken { id, text, previous } => write!(
                f,
                "Ambiguous `{}` = `{text}`: the same chord is already assigned to \
`{}`. Choose a unique chord and retry.",
                id.config_path(),
                previous.config_path()
            ),
        }
    }
}

impl std::error::Error for KeymapError {}

/// One resolved binding and the text it was written as (`leader c`, `g g`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bound {
    binding: Binding,
    spec: String,
}

impl Bound {
    fn is_leader(&self) -> bool {
        self.spec.starts_with(LEADER)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Slot {
    /// Whether the user set this action (an empty list counts).
    configured: bool,
    bound: Vec<Bound>,
}

/// The resolved key bindings of every action, per context: what Flutter
/// would hold as the `Shortcuts` map the app provides above its widgets.
/// Built from the config by [`Keymap::resolve`]; changed only by building a
/// new one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    /// Parallel to the catalogue's rows.
    slots: Vec<Slot>,
    leaders: Vec<Stroke>,
}

fn index_of(id: ActionId) -> Option<usize> {
    static INDEX: OnceLock<HashMap<ActionId, usize>> = OnceLock::new();
    INDEX
        .get_or_init(|| {
            ROWS.iter()
                .enumerate()
                .map(|(index, row)| (row.id, index))
                .collect()
        })
        .get(&id)
        .copied()
}

impl Keymap {
    /// The built-in defaults, with no config.
    pub fn defaults() -> Keymap {
        match Keymap::resolve(&KeymapSpec::default()) {
            Ok(keymap) => keymap,
            Err(error) => panic!("built-in keymap defaults must be valid: {error}"),
        }
    }

    /// The keymap a config makes, validated: a key may not mean two things
    /// where both actions can be active, a chord's prefix may not hide a
    /// single key, and the reserved keys stay reserved.
    pub fn resolve(spec: &KeymapSpec) -> Result<Keymap, KeymapError> {
        let keymap = Keymap::build(spec, true)?;
        keymap.validate()?;
        Ok(keymap)
    }

    /// The keymap without the conflict passes, for tests that set bindings
    /// the oracle's tests set field by field. A spec that does not parse is
    /// skipped.
    pub fn resolve_unchecked(spec: &KeymapSpec) -> Keymap {
        match Keymap::build(spec, false) {
            Ok(keymap) => keymap,
            Err(_) => Keymap::defaults(),
        }
    }

    fn slot(&self, id: ActionId) -> Option<&Slot> {
        index_of(id).and_then(|index| self.slots.get(index))
    }

    /// Every binding of an action in the order a hint lists them: single keys
    /// first, then chords.
    pub fn bindings(&self, id: ActionId) -> Vec<Binding> {
        self.slot(id)
            .map(|slot| slot.bound.iter().map(|bound| bound.binding).collect())
            .unwrap_or_default()
    }

    /// The single keys of an action.
    pub fn singles(&self, id: ActionId) -> Vec<Stroke> {
        self.slot(id)
            .map(|slot| {
                slot.bound
                    .iter()
                    .filter_map(|bound| bound.binding.single())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether a key is one of the action's single keys.
    pub fn is_single(&self, id: ActionId, stroke: &Stroke) -> bool {
        self.slot(id).is_some_and(|slot| {
            slot.bound
                .iter()
                .filter_map(|bound| bound.binding.single())
                .any(|single| single.same_key(stroke))
        })
    }

    /// The label of the action's first binding (`ShortcutHint` of the
    /// oracle's `primary_hint`): `⌃t`, `enter`, `⌃x b`.
    pub fn label(&self, id: ActionId) -> Option<String> {
        self.slot(id)
            .and_then(|slot| slot.bound.first())
            .map(|bound| bound.binding.label())
    }

    /// The labels of all bindings, in order.
    pub fn labels(&self, id: ActionId) -> Vec<String> {
        self.slot(id)
            .map(|slot| slot.bound.iter().map(|b| b.binding.label()).collect())
            .unwrap_or_default()
    }

    /// The bindings of an action as the config writes them, each once (the oracle's
    /// `active_binding_specs`): the text the user wrote for a configured one (`leader p`),
    /// else the canonical spec of each default single key, and the chords as they were
    /// declared. A leader chord some leader key cannot complete (another action holds it) is
    /// written as the chord it is, so the spec means what it is bound to.
    pub fn specs(&self, id: ActionId) -> Vec<String> {
        let Some(slot) = self.slot(id) else {
            return Vec::new();
        };
        let mut specs: Vec<String> = Vec::new();
        for bound in &slot.bound {
            let spec = match bound.binding {
                Binding::Single(_) if slot.configured => bound.spec.clone(),
                Binding::Single(stroke) => stroke.spec(),
                Binding::Chord { prefix, completion } if bound.is_leader() => {
                    let whole = self.leaders.iter().all(|leader| {
                        slot.bound.iter().any(|other| {
                            other.binding
                                == Binding::Chord {
                                    prefix: *leader,
                                    completion,
                                }
                        })
                    });
                    if whole {
                        bound.spec.clone()
                    } else {
                        format!("{} {}", prefix.spec(), completion.spec())
                    }
                }
                Binding::Chord { .. } => bound.spec.clone(),
            };
            if !specs.contains(&spec) {
                specs.push(spec);
            }
        }
        specs
    }

    /// Whether the user set the action (an empty list is a setting: unbound).
    pub fn is_configured(&self, id: ActionId) -> bool {
        self.slot(id).is_some_and(|slot| slot.configured)
    }

    /// The leader keys; each completes the `leader <key>` chords.
    pub fn leaders(&self) -> &[Stroke] {
        &self.leaders
    }

    /// Every action with its bindings, in catalogue order.
    pub fn actions(&self) -> impl Iterator<Item = (ActionId, Vec<Binding>)> + '_ {
        ROWS.iter().zip(&self.slots).map(|(row, slot)| {
            (
                row.id,
                slot.bound.iter().map(|bound| bound.binding).collect(),
            )
        })
    }

    fn build(spec: &KeymapSpec, strict: bool) -> Result<Keymap, KeymapError> {
        // Pane shortcuts have no pane to act on (classic mode): they must not
        // claim keys or chord prefixes.
        let spec = &spec.without("global", SIDE_PANE);
        let parsed = parse_configured(spec, strict)?;
        let mut slots = vec![Slot::default(); ROWS.len()];

        // Explicit single keys and chords first: the leader's default and the
        // new defaults yield to them.
        let mut leader_items: Vec<(usize, Stroke, String)> = Vec::new();
        for entry in &parsed {
            let slot = &mut slots[entry.row];
            slot.configured = true;
            for (item, text) in &entry.items {
                match item {
                    Item::Single(stroke) => {
                        let binding = Binding::Single(*stroke);
                        if !slot.bound.iter().any(|b| b.binding.same_key(&binding)) {
                            slot.bound.push(Bound {
                                binding,
                                spec: text.clone(),
                            });
                        }
                    }
                    Item::Chord(prefix, completion) => {
                        slot.bound.push(Bound {
                            binding: Binding::Chord {
                                prefix: prefix.canonical(),
                                completion: completion.canonical(),
                            },
                            spec: text.clone(),
                        });
                    }
                    Item::Leader(completion) => {
                        leader_items.push((entry.row, completion.canonical(), text.clone()));
                    }
                }
            }
        }
        let explicit_prefixes: Vec<Stroke> = slots
            .iter()
            .flat_map(|slot| &slot.bound)
            .filter_map(|bound| match bound.binding {
                Binding::Chord { prefix, .. } => Some(prefix),
                Binding::Single(_) => None,
            })
            .collect();
        let leaders = resolve_leaders(spec, &explicit_prefixes, strict)?;
        if leader_items.len().saturating_mul(leaders.len()) > MAX_LEADER_CHORDS {
            return Err(KeymapError::TooManyLeaderChords);
        }
        for (row, completion, text) in leader_items {
            for prefix in &leaders {
                let binding = Binding::Chord {
                    prefix: *prefix,
                    completion,
                };
                let slot = &mut slots[row];
                match slot.bound.iter_mut().find(|b| b.binding == binding) {
                    Some(existing) => existing.spec.clone_from(&text),
                    None => slot.bound.push(Bound {
                        binding,
                        spec: text.clone(),
                    }),
                }
            }
        }

        // Built-in defaults for what the user left alone.
        let used = UsedKeys::of(&slots, spec);
        for (index, row) in ROWS.iter().enumerate() {
            if slots[index].configured {
                continue;
            }
            for default in row.defaults {
                let Some(stroke) = Stroke::parse(default) else {
                    continue;
                };
                if yields_to_configured(row.id) && used.claims(row.id, &stroke) {
                    continue;
                }
                slots[index].bound.push(Bound {
                    binding: Binding::Single(stroke),
                    spec: (*default).to_owned(),
                });
            }
        }

        let mut keymap = Keymap { slots, leaders };
        keymap.add_implicit_chords(spec);
        // Single keys before chords, as a hint lists them; what the user set stays in the order
        // they wrote it (`primary_hint`: the first configured spec is the hint, a chord or not).
        for slot in keymap.slots.iter_mut().filter(|slot| !slot.configured) {
            slot.bound
                .sort_by_key(|bound| matches!(bound.binding, Binding::Chord { .. }));
        }
        Ok(keymap)
    }

    /// The chords no config sets: Vim's `g g`, and the leader shortcuts of
    /// the backend, presets and selection (`leader_defaults::register`).
    fn add_implicit_chords(&mut self, spec: &KeymapSpec) {
        let g = Stroke::parse("g").unwrap_or(Stroke::new(Key::Char('g'), Mods::NONE));
        let jump_top = Binding::Chord {
            prefix: g,
            completion: g,
        };
        for id in [
            ActionId::new(Context::VimNormal, "jump_top"),
            ActionId::new(Context::VimOperator, "motion_jump_top"),
        ] {
            let Some(index) = index_of(id) else { continue };
            let taken = self.slots[index].configured
                || self.chords().any(|(other, bound)| {
                    id.context.overlaps(other.context) && bound.binding == jump_top
                })
                || ROWS.iter().any(|row| {
                    id.context.overlaps(row.id.context)
                        && configured_specs(spec, row.id).is_some_and(|specs| {
                            specs
                                .iter()
                                .any(|s| Stroke::parse(s).is_some_and(|k| k.same_key(&g)))
                        })
                });
            if !taken {
                self.slots[index].bound.push(Bound {
                    binding: jump_top,
                    spec: "g g".to_owned(),
                });
            }
        }

        let mut remaining = MAX_LEADER_CHORDS
            .saturating_sub(self.chords().filter(|(_, bound)| bound.is_leader()).count());
        for (context, name, completion) in LEADER_DEFAULTS {
            if remaining == 0 {
                break;
            }
            let id = ActionId::new(*context, name);
            let Some(index) = index_of(id) else { continue };
            let Some(completion_stroke) = Stroke::parse(completion) else {
                continue;
            };
            // The port has no panes, and a configured action keeps its keys.
            if SIDE_PANE.contains(name) || self.slots[index].configured {
                continue;
            }
            for prefix in self.leaders.clone() {
                let binding = Binding::Chord {
                    prefix,
                    completion: completion_stroke.canonical(),
                };
                let taken = self.chords().any(|(other, bound)| {
                    chord_overlaps(true, id, other) && bound.binding == binding
                }) || self
                    .singles_overlapping(id, true)
                    .any(|key| key.same_key(&prefix));
                if taken {
                    continue;
                }
                self.slots[index].bound.push(Bound {
                    binding,
                    spec: format!("{LEADER}{completion}"),
                });
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        }
    }

    fn chords(&self) -> impl Iterator<Item = (ActionId, &Bound)> {
        ROWS.iter().zip(&self.slots).flat_map(|(row, slot)| {
            slot.bound
                .iter()
                .filter(|bound| matches!(bound.binding, Binding::Chord { .. }))
                .map(move |bound| (row.id, bound))
        })
    }

    /// The single keys of the actions active beside `id` (and of every pager
    /// action when the chord is a leader's: transcript navigation runs beside
    /// the composer).
    fn singles_overlapping(&self, id: ActionId, leader: bool) -> impl Iterator<Item = Stroke> + '_ {
        ROWS.iter().zip(&self.slots).flat_map(move |(row, slot)| {
            let active =
                chord_overlaps(leader, id, row.id) || leader && row.id.context == Context::Pager;
            slot.bound
                .iter()
                .filter_map(move |bound| bound.binding.single().filter(|_| active))
        })
    }

    fn validate(&self) -> Result<(), KeymapError> {
        self.validate_singles()?;
        self.validate_chords()
    }

    fn singles_of(&self, id: ActionId) -> Vec<Stroke> {
        self.singles(id)
    }

    fn validate_singles(&self) -> Result<(), KeymapError> {
        for id in PRINTABLE_RESERVED {
            if self.singles_of(*id).iter().any(|key| {
                key.is_plain_text() || matches!(key.key, Key::Char(_)) && key.mods.is_altgr()
            }) {
                return Err(KeymapError::PrintableReserved(*id));
            }
        }
        let ctrl_z = Stroke::new(Key::Char('z'), Mods::CTRL);
        for id in CTRL_Z_RESERVED {
            if self.singles_of(*id).contains(&ctrl_z) {
                return Err(KeymapError::CtrlZReserved(*id));
            }
        }
        let open_agents = ActionId::new(Context::Global, "open_agents");
        if self
            .singles_of(open_agents)
            .iter()
            .any(|key| matches!(key.key, Key::Char(_)) && key.mods.is_altgr())
        {
            return Err(KeymapError::OpenAgentsAltGr);
        }

        // The keys the app and the transcript's backtrack take for themselves.
        for (row, _) in ROWS.iter().zip(&self.slots) {
            let id = row.id;
            let (scope, reserved) = match id.context {
                Context::Global | Context::Chat | Context::Voice | Context::Composer => {
                    ("main", MAIN_RESERVED)
                }
                Context::Pager => ("pager", TRANSCRIPT_RESERVED),
                Context::Agents => ("agents", MAIN_RESERVED),
                _ => continue,
            };
            for key in self.singles_of(id) {
                let Some((reserved_action, _)) = reserved
                    .iter()
                    .find(|(_, spec)| Stroke::parse(spec) == Some(key))
                else {
                    continue;
                };
                // The oracle's allowed overlaps: interrupting a turn may keep
                // Esc (it backtracks only on an empty composer), and the
                // agents search takes `/` while the command center is open.
                let allowed = id == ActionId::new(Context::Chat, "interrupt_turn")
                    && *reserved_action == "fixed.backtrack"
                    || id == ActionId::new(Context::Agents, "search")
                        && *reserved_action == "fixed.slash_command";
                if !allowed {
                    return Err(KeymapError::ReservedKey {
                        scope,
                        id,
                        reserved: reserved_action,
                    });
                }
            }
        }
        for id in ROWS
            .iter()
            .map(|row| row.id)
            .filter(|id| id.context == Context::Agents)
        {
            let name = id.name;
            if self.singles_of(id).iter().any(|key| {
                key.key == Key::Backspace && key.mods.is_empty() && name != "delete"
                    || matches!(key.key, Key::Char(_)) && key.mods.is_altgr()
            }) {
                return Err(KeymapError::AgentsEditing { name });
            }
            if self.singles_of(id).contains(&ctrl_z) {
                return Err(KeymapError::AgentsCtrlZ { name });
            }
        }

        // One key, one action, wherever the oracle checks both together.
        for scope in SCOPES {
            let mut seen: HashMap<(Key, Mods), ActionId> = HashMap::new();
            for row in ROWS.iter().filter(|row| (scope.primary)(row.id)) {
                for key in self.singles_of(row.id) {
                    if let Some(previous) = seen.insert(key.normalized(), row.id)
                        && scope.shadowed.is_none()
                        && previous != row.id
                    {
                        return Err(ambiguous(scope, previous, row.id));
                    }
                }
            }
            let Some(shadowed) = scope.shadowed else {
                continue;
            };
            for row in ROWS.iter().filter(|row| shadowed(row.id)) {
                for key in self.singles_of(row.id) {
                    if let Some(previous) = seen.get(&key.normalized())
                        && !overlap_allowed(*previous, row.id, &key)
                    {
                        return Err(ambiguous(scope, *previous, row.id));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_chords(&self) -> Result<(), KeymapError> {
        let all: Vec<(ActionId, &Bound)> = self.chords().collect();
        for (index, (id, bound)) in all.iter().enumerate() {
            let Binding::Chord { prefix, completion } = bound.binding else {
                continue;
            };
            let text = || bound.spec.clone();
            if matches!(prefix.key, Key::Char(_))
                && prefix.mods.is_altgr()
                && !id.context.allows_plain_chord_prefix()
            {
                return Err(KeymapError::ChordAltGr {
                    id: *id,
                    text: text(),
                });
            }
            if matches!(prefix.key, Key::Char(_))
                && prefix.is_plain_text()
                && !id.context.allows_plain_chord_prefix()
                && id.context != Context::Agents
            {
                return Err(KeymapError::ChordPlainPrefix {
                    id: *id,
                    text: text(),
                });
            }
            let esc = Stroke::new(Key::Esc, Mods::NONE);
            if prefix == esc || completion == esc {
                return Err(KeymapError::ChordEsc {
                    id: *id,
                    text: text(),
                });
            }
            if id.context == Context::Agents && prefix == Stroke::new(Key::Backspace, Mods::NONE) {
                return Err(KeymapError::ChordBackspace {
                    id: *id,
                    text: text(),
                });
            }
            let ctrl_z = Stroke::new(Key::Char('z'), Mods::CTRL);
            if prefix == ctrl_z || completion == ctrl_z {
                return Err(KeymapError::ChordCtrlZ {
                    id: *id,
                    text: text(),
                });
            }
            let (reserved, list) = match id.context {
                Context::Pager => (TRANSCRIPT_RESERVED, false),
                Context::List => (LIST_RESERVED, true),
                Context::Agents | Context::Approval => (&LIST_RESERVED[..1], false),
                _ => (MAIN_RESERVED, false),
            };
            let ctrl_c = Stroke::new(Key::Char('c'), Mods::CTRL);
            let hit = reserved.iter().find(|(_, spec)| {
                let key = Stroke::parse(spec);
                key == Some(prefix) || key == Some(completion) && (!list || key == Some(ctrl_c))
            });
            if let Some((reserved_action, _)) = hit {
                return Err(KeymapError::ChordReserved {
                    id: *id,
                    text: text(),
                    reserved: reserved_action,
                });
            }
            if completion == prefix
                && (self.leaders.contains(&prefix) && id.context.overlaps(Context::Global)
                    || all.iter().any(|(other, leader)| {
                        leader.is_leader()
                            && leader.binding.first() == prefix
                            && chord_overlaps(true, *other, *id)
                    }))
            {
                return Err(KeymapError::ChordLeaderRepeat {
                    id: *id,
                    text: text(),
                });
            }
            let leader = bound.is_leader();
            if let Some(shadowed) = ROWS.iter().zip(&self.slots).find(|(row, slot)| {
                chord_overlaps(leader, *id, row.id)
                    && slot.bound.iter().any(|b| {
                        b.binding
                            .single()
                            .is_some_and(|single| single.same_key(&prefix))
                    })
            }) {
                return Err(KeymapError::ChordShadows {
                    id: *id,
                    text: text(),
                    shadowed: shadowed.0.id,
                });
            }
            for (previous_id, previous) in &all[..index] {
                let find_pair = [*previous_id, *id]
                    .contains(&ActionId::new(Context::Global, "find_transcript"))
                    && [*previous_id, *id].contains(&ActionId::new(Context::Pager, "find"));
                if previous_id != id
                    && (chord_overlaps(previous.is_leader(), *previous_id, *id)
                        || chord_overlaps(leader, *id, *previous_id))
                    && previous.binding == bound.binding
                    && !find_pair
                {
                    return Err(KeymapError::ChordTaken {
                        id: *id,
                        text: text(),
                        previous: *previous_id,
                    });
                }
            }
        }
        Ok(())
    }
}

/// How an action is named in an error of the app scope: global actions bare.
fn short_name(id: ActionId) -> String {
    if id.context == Context::Global {
        id.name.to_owned()
    } else {
        format!("{}.{}", id.context.config_name(), id.name)
    }
}

fn ambiguous(scope: &Scope, first: ActionId, second: ActionId) -> KeymapError {
    KeymapError::Ambiguous {
        scope: scope.name,
        bare: scope.bare,
        shadows: scope.shadowed.is_some(),
        first,
        second,
    }
}

fn overlap_allowed(a: ActionId, b: ActionId, key: &Stroke) -> bool {
    ALLOWED_OVERLAPS.iter().any(|(first, second, spec)| {
        (*first == a && *second == b || *first == b && *second == a)
            && Stroke::parse(spec).is_some_and(|allowed| allowed.same_key(key))
    })
}

/// Whether a chord of `id` and an action `other` can both be active. A
/// leader's chords also run beside the pager and chat handlers of the
/// transcript.
fn chord_overlaps(leader: bool, id: ActionId, other: ActionId) -> bool {
    let scope = |c: Context| matches!(c, Context::Global | Context::Chat | Context::Pager);
    id.overlaps(other) || leader && scope(id.context) && scope(other.context)
}

/// One spec of an action, read.
enum Item {
    Single(Stroke),
    Chord(Stroke, Stroke),
    Leader(Stroke),
}

struct Parsed {
    row: usize,
    items: Vec<(Item, String)>,
}

/// What the user set for an action: its own entry, or for the composer's
/// submit, queue and shortcuts toggle the `global` one.
fn configured_specs(spec: &KeymapSpec, id: ActionId) -> Option<&[String]> {
    if RUNTIME_ONLY.contains(&id) {
        return None;
    }
    spec.get(id.context.config_name(), id.name).or_else(|| {
        GLOBAL_FALLBACK
            .contains(&id)
            .then(|| spec.get("global", id.name))
            .flatten()
    })
}

fn parse_configured(spec: &KeymapSpec, strict: bool) -> Result<Vec<Parsed>, KeymapError> {
    let mut out = Vec::new();
    for (row, entry) in ROWS.iter().enumerate() {
        let Some(specs) = configured_specs(spec, entry.id) else {
            continue;
        };
        let mut items: Vec<(Item, String)> = Vec::new();
        for raw in specs {
            if items.iter().any(|(_, text)| text == raw) {
                continue;
            }
            let item = match parse_spec(raw) {
                Ok(Spec::Binding(Binding::Single(stroke))) => Item::Single(stroke),
                Ok(Spec::Binding(Binding::Chord { prefix, completion })) => {
                    Item::Chord(prefix, completion)
                }
                Ok(Spec::Leader(completion)) => Item::Leader(completion),
                Err(_) if !strict => continue,
                Err(_) => {
                    return Err(KeymapError::Invalid {
                        id: entry.id,
                        raw: raw.clone(),
                        shape: if raw.starts_with(LEADER) {
                            Shape::Leader
                        } else if raw.contains(' ') {
                            Shape::Chord
                        } else {
                            Shape::Single
                        },
                    });
                }
            };
            items.push((item, raw.clone()));
        }
        out.push(Parsed { row, items });
    }
    Ok(out)
}

/// The shared prefix keys (`leader::resolve`): `global.leader`, or `ctrl-x`
/// unless the user already uses it.
fn resolve_leaders(
    spec: &KeymapSpec,
    explicit_prefixes: &[Stroke],
    strict: bool,
) -> Result<Vec<Stroke>, KeymapError> {
    let default = Stroke::parse(DEFAULT_LEADER).unwrap_or(Stroke::new(Key::Char('x'), Mods::CTRL));
    let configured = spec.get("global", "leader");
    if configured.is_some_and(|specs| specs.iter().any(|s| s.contains(' '))) {
        return Err(KeymapError::LeaderNotSingle);
    }
    let leaders: Vec<Stroke> = match configured {
        Some(specs) => {
            let mut out: Vec<Stroke> = Vec::new();
            for raw in specs {
                let Some(stroke) = Stroke::parse(raw) else {
                    if strict {
                        return Err(KeymapError::LeaderInvalid { raw: raw.clone() });
                    }
                    continue;
                };
                let stroke = stroke.canonical();
                if !out.contains(&stroke) {
                    out.push(stroke);
                }
            }
            out
        }
        None => {
            let shadowed = spec
                .all_specs()
                .any(|s| Stroke::parse(s).is_some_and(|key| key.same_key(&default)))
                || explicit_prefixes.contains(&default);
            if shadowed { Vec::new() } else { vec![default] }
        }
    };
    if strict {
        for prefix in &leaders {
            let label = prefix.label();
            if matches!(prefix.key, Key::Char(_))
                && (prefix.is_plain_text() || prefix.mods.is_altgr())
            {
                return Err(KeymapError::LeaderPlainText { label });
            }
            if *prefix == Stroke::new(Key::Char('z'), Mods::CTRL) {
                return Err(KeymapError::LeaderCtrlZ { label });
            }
            if let Some((reserved_action, _)) = MAIN_RESERVED
                .iter()
                .find(|(_, reserved)| Stroke::parse(reserved) == Some(*prefix))
            {
                return Err(KeymapError::LeaderReserved {
                    label,
                    reserved: reserved_action,
                });
            }
        }
    }
    Ok(leaders)
}

/// The keys the user's config claims, for a new default to yield to
/// (`configured_context_binding_is_used`, `chord.prefix == binding`).
struct UsedKeys {
    singles: Vec<(ActionId, Stroke)>,
    prefixes: Vec<(ActionId, Stroke)>,
    anywhere: Vec<Stroke>,
}

impl UsedKeys {
    fn of(slots: &[Slot], spec: &KeymapSpec) -> UsedKeys {
        let mut used = UsedKeys {
            singles: Vec::new(),
            prefixes: Vec::new(),
            anywhere: spec.all_specs().filter_map(Stroke::parse).collect(),
        };
        for (row, slot) in ROWS.iter().zip(slots) {
            if !slot.configured {
                continue;
            }
            for bound in &slot.bound {
                match bound.binding {
                    Binding::Single(stroke) => used.singles.push((row.id, stroke)),
                    Binding::Chord { prefix, .. } => used.prefixes.push((row.id, prefix)),
                }
            }
        }
        used
    }

    /// Whether the user's config already uses `key` where the default of `id`
    /// would be active. The activity and warnings keys yield to any use.
    fn claims(&self, id: ActionId, key: &Stroke) -> bool {
        let broad =
            id.context == Context::Global && matches!(id.name, "focus_activity" | "open_warnings");
        let active = |other: ActionId| broad || id.overlaps(other) || scopes_conflict(id, other);
        self.singles
            .iter()
            .any(|(other, stroke)| active(*other) && stroke.same_key(key))
            || self
                .prefixes
                .iter()
                .any(|(other, prefix)| (broad || id.overlaps(*other)) && prefix.same_key(key))
            || broad && self.anywhere.iter().any(|stroke| stroke.same_key(key))
    }
}
