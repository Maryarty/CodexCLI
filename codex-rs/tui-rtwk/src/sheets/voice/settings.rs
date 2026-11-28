use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU16;
use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::voice::settings::{
    Audio, Change, Device, DeviceKind, Intent, Operation, Snapshot,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::close_dialog;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use super::strings::{self, EN as S};
use crate::app::view::Hs;
use crate::widgets::picker::list::{Item, List, Role, Span};
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Panel {
    Home,
    Voices,
    Sound,
    Devices(DeviceKind),
    Channels {
        device: Device,
        draft: BTreeSet<NonZeroU16>,
        microphone: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Page {
    thread: Option<ThreadId>,
    panel: Panel,
}

impl Page {
    pub(super) fn home(thread: Option<ThreadId>) -> Self {
        Self {
            thread,
            panel: Panel::Home,
        }
    }

    fn load(&self) -> Option<Operation> {
        match &self.panel {
            Panel::Voices => Some(Operation::ReadCatalog),
            Panel::Sound => Some(Operation::ReadAudio),
            Panel::Devices(kind) => Some(Operation::ListDevices(*kind)),
            Panel::Home | Panel::Channels { .. } => None,
        }
    }
}

pub(super) struct PageState {
    loaded: bool,
    draft: BTreeSet<NonZeroU16>,
    empty: bool,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            loaded: false,
            draft: match &self.panel {
                Panel::Channels { draft, .. } => draft.clone(),
                _ => BTreeSet::new(),
            },
            empty: false,
        }
    }
}

impl_widget!(stateful eq Page);

fn tell(cx: &mut Cx<'_, PageState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Voice(intent));
}

fn push(cx: &mut Cx<'_, PageState>, from: &Page, panel: Panel) {
    let page = Page {
        thread: from.thread.clone(),
        panel,
    };
    drop(Navigator::of(cx).push::<()>(PageRoute::<()>::new(move |_| bottom(page.clone()))));
}

fn back(cx: &mut Cx<'_, PageState>) {
    let nav = Navigator::of(cx);
    if nav.can_pop() {
        nav.pop(cx);
    } else {
        close_dialog::<_, ()>(cx, None);
    }
}

fn item(id: impl Into<String>, name: impl Into<String>) -> Item {
    Item {
        id: id.into(),
        name: name.into(),
        ..Item::default()
    }
}

fn basic(title: &str) -> List {
    List {
        title: Some(title.into()),
        standard_hint: true,
        ..List::default()
    }
}

fn apply_next(title: &str) -> List {
    List {
        subtitle: Some(S.applies_to_next.into()),
        ..basic(title)
    }
}

fn microphone(audio: &Audio, devices: &[Device]) -> Option<Device> {
    let mut selected = devices
        .iter()
        .filter(|device| audio.selection(DeviceKind::Input, device));
    let first = selected.next()?;
    (selected.next().is_none() && first.channels >= 3).then(|| first.clone())
}

fn devices_list(kind: DeviceKind, seen: Option<&Snapshot>) -> List {
    let mut list = apply_next(strings::device_label(kind));
    let Some((audio, devices)) =
        seen.and_then(|seen| Some((seen.audio.as_ref()?, seen.devices(kind)?)))
    else {
        return list;
    };
    let current = audio.device(kind);
    let mut occurrences = HashMap::new();
    for device in devices {
        *occurrences.entry(device.name.as_str()).or_insert(0) += 1;
    }
    list.items.push(Item {
        current: current.is_none(),
        ..item("default", S.system_default)
    });
    list.items
        .extend(devices.iter().enumerate().map(|(index, device)| Item {
            current: current == Some(device.name.as_str()),
            disabled_reason:
                (occurrences[device.name.as_str()] > 1).then(|| S.duplicate_device.into()),
            ..item(index.to_string(), &device.name)
        }));
    list.initial = list
        .items
        .iter()
        .position(|item| item.current && !item.is_disabled());
    if kind == DeviceKind::Input && microphone(audio, devices).is_some() {
        list.items.insert(
            list.initial.unwrap_or(0) + 1,
            Item {
                child_label: Some('a'),
                description: Some(strings::describe_channels(audio.channels.as_deref())),
                ..item("channels", S.input_channels)
            },
        );
    }
    list.items.push(item("back", S.back));
    list
}

impl PageState {
    fn list(&self, panel: &Panel, seen: Option<&Snapshot>) -> List {
        match panel {
            Panel::Home => List {
                items: vec![
                    item("sound", S.set_sound),
                    item("voice", S.select_voice_action),
                ],
                ..basic(S.settings)
            },
            Panel::Voices => {
                let mut list = apply_next(S.select_voice);
                if let Some(catalog) = seen.and_then(|seen| seen.catalog.as_ref()) {
                    list.items.extend(catalog.voices.iter().map(|voice| Item {
                        current: Some(*voice) == catalog.current,
                        ..item(voice.wire_name(), voice.wire_name())
                    }));
                    list.initial = list.items.iter().position(|item| item.current);
                    list.items.push(item("back", S.back));
                }
                list
            }
            Panel::Sound => {
                let mut list = apply_next(S.sound_devices);
                if let Some(audio) = seen.and_then(|seen| seen.audio.as_ref()) {
                    list.items.extend(
                        [(DeviceKind::Input, "input"), (DeviceKind::Output, "output")].map(
                            |(kind, id)| Item {
                                description: Some(
                                    audio.device(kind).unwrap_or(S.system_default).into(),
                                ),
                                ..item(id, strings::device_label(kind))
                            },
                        ),
                    );
                    list.items.push(item("back", S.back));
                }
                list
            }
            Panel::Devices(kind) => devices_list(*kind, seen),
            Panel::Channels { device, .. } => List {
                title: Some(strings::microphone(&device.name)),
                subtitle: Some(S.mix_channels.into()),
                footer_hint: Some(S.channel_hint.into()),
                footer_note_spans: if self.empty {
                    vec![Span::new(S.at_least_one, Role::Danger)]
                } else {
                    Vec::new()
                },
                searchable: true,
                numbered: false,
                multi: true,
                panel: true,
                items: (1..=device.channels)
                    .filter_map(NonZeroU16::new)
                    .map(|number| Item {
                        toggle: Some(self.draft.contains(&number)),
                        search_value: Some(strings::input(number)),
                        ..item(number.to_string(), strings::input(number))
                    })
                    .collect(),
                ..List::default()
            },
        }
    }

    fn choose(&mut self, w: &Page, id: &str, cx: &mut Cx<'_, Self>) {
        if id == "back" {
            return back(cx);
        }
        let Some(thread) = &w.thread else { return };
        let seen = read::<ScreenStates, _>(cx);
        let model = &seen.voice.settings;
        if model.pending(thread) {
            return;
        }
        let seen = model.snapshot(thread);
        match &w.panel {
            Panel::Home if id == "sound" => push(cx, w, Panel::Sound),
            Panel::Home if id == "voice" => push(cx, w, Panel::Voices),
            Panel::Sound if id == "input" => push(cx, w, Panel::Devices(DeviceKind::Input)),
            Panel::Sound if id == "output" => push(cx, w, Panel::Devices(DeviceKind::Output)),
            Panel::Voices => {
                if let Some((seen, voice)) = seen.and_then(|seen| {
                    Some((
                        seen,
                        seen.catalog
                            .as_ref()?
                            .voices
                            .iter()
                            .copied()
                            .find(|voice| voice.wire_name() == id)?,
                    ))
                }) {
                    tell(
                        cx,
                        Intent::Persist {
                            origin: seen.origin.clone(),
                            change: Change::Voice(voice),
                        },
                    );
                    close_dialog::<_, ()>(cx, None);
                }
            }
            Panel::Devices(kind) => {
                let Some((seen, audio, devices)) =
                    seen.and_then(|seen| Some((seen, seen.audio.as_ref()?, seen.devices(*kind)?)))
                else {
                    return;
                };
                if id == "channels" && *kind == DeviceKind::Input {
                    if let Some(device) = microphone(audio, devices) {
                        let draft = audio.channels.as_ref().map_or_else(
                            || (1..=device.channels).filter_map(NonZeroU16::new).collect(),
                            |channels| channels.iter().copied().collect(),
                        );
                        push(
                            cx,
                            w,
                            Panel::Channels {
                                device,
                                draft,
                                microphone: audio.microphone.clone(),
                            },
                        );
                    }
                    return;
                }
                let name = if id == "default" {
                    None
                } else {
                    let Some(device) = id
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| devices.get(index))
                    else {
                        return;
                    };
                    Some(device.name.clone())
                };
                tell(
                    cx,
                    Intent::Persist {
                        origin: seen.origin.clone(),
                        change: Change::Device { kind: *kind, name },
                    },
                );
            }
            _ => {}
        }
    }
}

impl State for PageState {
    type Widget = Page;
    type Msg = Event;

    fn did_change_dependencies(&mut self, w: &Page, cx: &mut Cx<'_, Self>) {
        if !self.loaded {
            self.loaded = true;
            if let (Some(thread), Some(operation)) = (&w.thread, w.load()) {
                tell(
                    cx,
                    Intent::Read {
                        thread: thread.clone(),
                        operation,
                    },
                );
            }
        }
    }

    fn update(&mut self, event: Event, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match event {
            Event::Choose(id) => self.choose(w, &id, cx),
            Event::Cancel => back(cx),
            Event::Toggle(id) if matches!(&w.panel, Panel::Channels { .. }) => {
                if let Ok(number) = id.parse::<NonZeroU16>() {
                    if !self.draft.remove(&number) {
                        self.draft.insert(number);
                    }
                    self.empty = self.draft.is_empty();
                    return Rebuild::Yes;
                }
            }
            Event::Confirm
                if let (
                    Some(thread),
                    Panel::Channels {
                        device, microphone, ..
                    },
                ) = (&w.thread, &w.panel) =>
            {
                self.empty = self.draft.is_empty();
                if !self.empty {
                    let states = read::<ScreenStates, _>(cx);
                    if !states.voice.settings.pending(thread)
                        && let Some(seen) = states.voice.settings.snapshot(thread)
                    {
                        let channels = (self.draft.len() != usize::from(device.channels))
                            .then(|| self.draft.iter().copied().collect());
                        tell(
                            cx,
                            Intent::Persist {
                                origin: seen.origin.clone(),
                                change: Change::InputChannels {
                                    microphone: microphone.clone(),
                                    channels,
                                },
                            },
                        );
                        close_dialog::<_, ()>(cx, None);
                    }
                }
                return Rebuild::Yes;
            }
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let thread = w.thread.clone();
        let (seen, pending) = select::<ScreenStates, _, _>(cx, move |states| {
            thread.as_ref().map_or((None, false), |thread| {
                (
                    states.voice.settings.snapshot(thread).cloned(),
                    states.voice.settings.pending(thread),
                )
            })
        });
        let mut list = self.list(&w.panel, seen.as_deref());
        if !matches!(w.panel, Panel::Home) {
            if pending {
                list.footer_note = Some(S.pending.into());
            } else if let Some(feedback) = seen.as_ref().and_then(|seen| seen.feedback.as_ref()) {
                list.footer_note_spans.push(strings::feedback(feedback));
            }
        }
        Picker::new(Rc::new(list), cx.map(|event| event)).into()
    }
}
