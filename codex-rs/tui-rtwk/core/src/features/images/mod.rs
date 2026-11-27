//! A mounted artifact reads an image snapshot through the host. The thread, item, and path
//! identify the source. The generation distinguishes a new read of that source from a reply
//! that was still in flight when its last reader went away.

use std::collections::BTreeMap;

use crate::app::effects::{Ask, Done, ImageSize, ImageSnapshot};
use crate::ids::ThreadId;
use crate::threads::ItemKey;

use super::AppCx;

pub struct Images;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Source {
    pub thread: ThreadId,
    pub item: ItemKey,
    pub path: String,
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
pub enum Ctx {
    Read { interest: Interest, generation: u64 },
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    generation: u64,
    snapshot: Option<ImageSnapshot>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    entries: BTreeMap<Interest, Entry>,
    generation: u64,
}

impl State {
    pub fn snapshot(&self, interest: &Interest) -> Option<&ImageSnapshot> {
        self.entries.get(interest)?.snapshot.as_ref()
    }
}

feature! {
    impl Feature for Images {
        const NAME: &'static str = "images";
        type State = State;
        type Intent = Open;
        type Ctx = Ctx;
        type Interest = Interest;
        {
            fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
                if !open {
                    state.entries.remove(interest);
                    return;
                }
                state.generation = state.generation.wrapping_add(1);
                let generation = state.generation;
                state.entries.insert(interest.clone(), Entry { generation, snapshot: None });
                cx.effect(
                    Ask::ImageSnapshot {
                        thread: interest.source.thread.clone(),
                        path: interest.source.path.clone(),
                        preview: interest.preview,
                    },
                    Ctx::Read { interest: interest.clone(), generation },
                );
            }

            fn effect_done(state: &mut State, ctx: Ctx, done: Done, _: &mut AppCx<'_>) {
                if let Ctx::Read { interest, generation } = ctx
                    && let Some(entry) = state.entries.get_mut(&interest)
                    && entry.generation == generation
                    && let Done::ImageSnapshot(snapshot) = done
                {
                    entry.snapshot = Some(snapshot);
                }
            }

            fn intent(state: &mut State, open: Open, cx: &mut AppCx<'_>) {
                let held = state.entries.iter().any(|(interest, entry)| {
                    interest.source.thread == open.thread &&
                        entry.snapshot.as_ref().and_then(|snapshot| snapshot.link.as_ref()) == Some(&open.url)
                });
                if held && cx.view().thread(&open.thread).is_some() {
                    cx.effect(Ask::open_url(open.url), Ctx::Open);
                }
            }
        }
    }
}
