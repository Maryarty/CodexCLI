//! A mounted artifact reads an image snapshot through the host. The thread, item, and path
//! identify the source. The generation distinguishes a new read of that source from a reply
//! that was still in flight when its last reader went away.

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::app::Now;
use crate::app::effects::{Ask, Done, ImageRead, ImageSize, ImageSnapshot};
use crate::ids::ThreadId;
use crate::threads::ItemKey;

use super::{AppCx, FeatureStates};

pub struct Images;

const ATTACHMENT_REFRESH_MS: i64 = 1000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Source {
    pub thread: ThreadId,
    pub kind: SourceKind,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Transcript(ItemKey),
    Attachment(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interest {
    pub source: Source,
    pub preview: Option<ImageSize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    pub thread: ThreadId,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Open(Open),
    Refresh,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Read { interest: Interest, generation: u64 },
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness {
    Pending,
    Ready(ImageSnapshot),
    Unavailable,
}

impl Readiness {
    fn snapshot(&self) -> Option<&ImageSnapshot> {
        match self {
            Self::Ready(snapshot) => Some(snapshot),
            Self::Pending | Self::Unavailable => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    generation: u64,
    readiness: Readiness,
    refresh: Option<Now>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    entries: BTreeMap<Interest, Entry>,
    generation: u64,
}

impl State {
    pub fn snapshot(&self, interest: &Interest) -> Option<&ImageSnapshot> {
        self.readiness(interest).snapshot()
    }

    pub fn readiness(&self, interest: &Interest) -> &Readiness {
        self.entries
            .get(interest)
            .map_or(&Readiness::Pending, |entry| &entry.readiness)
    }
}

fn read(interest: &Interest, entry: &Entry, cx: &mut AppCx<'_>) {
    cx.effect(
        Ask::ImageSnapshot {
            thread: interest.source.thread.clone(),
            path: interest.source.path.clone(),
            preview: interest.preview,
            previous: entry
                .readiness
                .snapshot()
                .and_then(|snapshot| snapshot.revision),
        },
        Ctx::Read {
            interest: interest.clone(),
            generation: entry.generation,
        },
    );
}

pub(crate) fn retire(states: &mut Rc<FeatureStates>, owns: impl Fn(&ThreadId) -> bool) {
    if states
        .images
        .entries
        .keys()
        .any(|interest| owns(&interest.source.thread))
    {
        Rc::make_mut(states)
            .images
            .entries
            .retain(|interest, _| !owns(&interest.source.thread));
    }
}

feature! {
    impl Feature for Images {
        const NAME: &'static str = "images";
        type State = State;
        type Intent = Intent;
        type Ctx = Ctx;
        type Interest = Interest;
        {
            fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
                if !open {
                    state.entries.remove(interest);
                    return;
                }
                if state.entries.contains_key(interest)
                    || cx.view().thread(&interest.source.thread).is_none() {
                    return;
                }
                state.generation = state.generation.wrapping_add(1);
                let entry = Entry { generation: state.generation, readiness: Readiness::Pending, refresh: None };
                read(interest, &entry, cx);
                state.entries.insert(interest.clone(), entry);
            }

            fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
                if let Ctx::Read { interest, generation } = ctx
                    && let Some(entry) = state.entries.get_mut(&interest)
                    && entry.generation == generation
                    && let Done::ImageSnapshot(result) = done
                {
                    let attachment = matches!(interest.source.kind, SourceKind::Attachment(_));
                    if matches!(result, ImageRead::Unavailable) {
                        entry.readiness = Readiness::Unavailable;
                        return;
                    }
                    if let ImageRead::Snapshot(mut snapshot) = result {
                        if attachment {
                            snapshot.link = None;
                        }
                        entry.readiness = Readiness::Ready(snapshot);
                    }
                    if attachment {
                        entry.refresh = Some(cx.view().now().plus_millis(ATTACHMENT_REFRESH_MS));
                    }
                }
            }

            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Open(open) => {
                        let held = state.entries.iter().any(|(interest, entry)| {
                            matches!(interest.source.kind, SourceKind::Transcript(_))
                                && interest.source.thread == open.thread
                                && entry.readiness.snapshot().and_then(|snapshot| snapshot.link.as_ref()) == Some(&open.url)
                        });
                        if held && cx.view().thread(&open.thread).is_some() {
                            cx.effect(Ask::open_url(open.url), Ctx::Open);
                        }
                    }
                    Intent::Refresh => {
                        let now = cx.view().now();
                        for (interest, entry) in &mut state.entries {
                            if entry.refresh.is_some_and(|at| now.reached(at)) {
                                entry.refresh = None;
                                read(interest, entry, cx);
                            }
                        }
                    }
                }
            }

            fn wake_at(state: &State) -> Option<(Now, Intent)> {
                state.entries.values().filter_map(|entry| entry.refresh)
                    .reduce(Now::min).map(|at| (at, Intent::Refresh))
            }
        }
    }
}
