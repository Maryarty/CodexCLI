//! Local catalog reads and confirmed pet selection for the live `/pets` opener.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::screens::pets::{Ambient, AssetAsk, AssetDone, DISABLED, Picker, Preview, Unavailable};

use super::AppCx;

pub struct PetSelection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visit(u64);

impl Visit {
    pub fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    FetchAmbient,
    Request {
        visit: Visit,
        selector: Option<String>,
        graphics: bool,
    },
    ReadPreview {
        visit: Visit,
        selector: Option<String>,
    },
    Apply {
        visit: Visit,
        selector: String,
    },
    RetirePreview(Visit),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Ambient(u64),
    Catalog {
        visit: Visit,
        selector: Option<String>,
    },
    Preview {
        visit: Visit,
        revision: u64,
    },
    Validate {
        visit: Visit,
        selector: String,
    },
    Save(Visit),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Loading,
    Ready(Picker),
    Unavailable(Unavailable),
    Applying,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    LoadFailed(String),
    SaveFailed { disabling: bool, error: String },
    SaveInProgress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Operation {
    visit: Visit,
    thread: Option<ThreadId>,
    phase: Phase,
    preview: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Write {
    visit: Visit,
    thread: Option<ThreadId>,
    selector: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    operation: Option<Operation>,
    saving: Option<Write>,
    selected: Option<String>,
    ambient_requested: bool,
    ambient_revision: u64,
    ambient: Option<Arc<Ambient>>,
}

impl State {
    pub fn ambient(&self) -> Option<&Arc<Ambient>> {
        self.ambient.as_ref()
    }

    fn fetch_ambient(&mut self, cx: &mut AppCx<'_>) {
        self.ambient_revision += 1;
        self.ambient = None;
        if self.selected.as_deref() != Some(DISABLED) {
            cx.effect(
                Ask::Pet(AssetAsk::Ambient {
                    selector: self.selected.clone(),
                }),
                Ctx::Ambient(self.ambient_revision),
            );
        }
    }

    pub fn phase(&self, visit: Visit) -> Option<&Phase> {
        self.operation
            .as_ref()
            .filter(|active| active.visit == visit)
            .map(|active| &active.phase)
    }

    /// The boot configuration and any subsequently confirmed local save.
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    fn current(&mut self, visit: Visit, cx: &AppCx<'_>) -> Option<&mut Operation> {
        self.operation
            .as_mut()
            .filter(|active| active.visit == visit && active.thread.as_ref() == cx.view().active())
    }
}

pub fn is_disable(selector: &str) -> bool {
    matches!(
        selector.trim().to_ascii_lowercase().as_str(),
        "disable" | DISABLED | "hide" | "hidden" | "off" | "none"
    )
}

fn request(
    state: &mut State,
    visit: Visit,
    selector: Option<String>,
    graphics: bool,
    cx: &mut AppCx<'_>,
) {
    state.operation = Some(Operation {
        visit,
        thread: cx.view().active().cloned(),
        phase: Phase::Loading,
        preview: 0,
    });
    if selector.as_deref().is_some_and(is_disable) {
        save(state, visit, DISABLED.to_owned(), cx);
    } else {
        cx.effect(
            Ask::Pet(AssetAsk::Catalog { graphics }),
            Ctx::Catalog { visit, selector },
        );
    }
}

fn catalog(
    state: &mut State,
    visit: Visit,
    selector: Option<String>,
    read: Result<Picker, Unavailable>,
    cx: &mut AppCx<'_>,
) {
    if state
        .current(visit, cx)
        .is_none_or(|active| !matches!(active.phase, Phase::Loading))
    {
        return;
    }
    match read {
        Ok(mut picker) => {
            if let Some(selected) = &state.selected {
                picker.current = Some(selected.clone());
            } else {
                state.selected = picker.current.clone();
            }
            if let Some(selector) = selector {
                validate(state, visit, selector, cx);
            } else if let Some(active) = state.current(visit, cx) {
                active.phase = Phase::Ready(picker);
            }
        }
        Err(reason) => {
            if let Some(active) = state.current(visit, cx) {
                active.phase = Phase::Unavailable(reason);
            }
        }
    }
}

fn read_preview(state: &mut State, visit: Visit, selector: Option<String>, cx: &mut AppCx<'_>) {
    let Some(active) = state.current(visit, cx) else {
        return;
    };
    let Phase::Ready(picker) = &mut active.phase else {
        return;
    };
    if picker.preview.as_ref().map(|(selector, _)| selector) == selector.as_ref() {
        return;
    }
    active.preview += 1;
    picker.preview = None;
    if selector.as_deref() == Some(DISABLED) {
        picker.preview = Some((DISABLED.to_owned(), Preview::Disabled));
        return;
    }
    let Some(selector) =
        selector.filter(|selector| picker.pets.iter().any(|pet| &pet.selector == selector))
    else {
        return;
    };
    picker.preview = Some((selector.clone(), Preview::Loading));
    cx.effect(
        Ask::Pet(AssetAsk::Preview { selector }),
        Ctx::Preview {
            visit,
            revision: active.preview,
        },
    );
}

fn apply(state: &mut State, visit: Visit, selector: String, cx: &mut AppCx<'_>) {
    let Some(active) = state.current(visit, cx) else {
        return;
    };
    let Phase::Ready(picker) = &active.phase else {
        return;
    };
    if !picker.pets.iter().any(|pet| pet.selector == selector) {
        return;
    }
    // A displayed image proves the host has already read and validated this exact asset.
    if selector == DISABLED
        || matches!(&picker.preview, Some((current, Preview::Image(_))) if current == &selector)
    {
        save(state, visit, selector, cx);
    } else {
        validate(state, visit, selector, cx);
    }
}

fn validate(state: &mut State, visit: Visit, selector: String, cx: &mut AppCx<'_>) {
    let Some(active) = state.current(visit, cx) else {
        return;
    };
    active.phase = Phase::Applying;
    cx.effect(
        Ask::Pet(AssetAsk::Preview {
            selector: selector.clone(),
        }),
        Ctx::Validate { visit, selector },
    );
}

fn save(state: &mut State, visit: Visit, selector: String, cx: &mut AppCx<'_>) {
    let Some(active) = state.current(visit, cx) else {
        return;
    };
    active.phase = Phase::Applying;
    let thread = active.thread.clone();
    if state.saving.is_some() {
        finish(state, visit, Some(Note::SaveInProgress), cx);
        return;
    }
    state.saving = Some(Write {
        visit,
        thread,
        selector: selector.clone(),
    });
    cx.effect(Ask::Pet(AssetAsk::Save { selector }), Ctx::Save(visit));
}

fn finish(state: &mut State, visit: Visit, note: Option<Note>, cx: &mut AppCx<'_>) {
    if let Some(active) = state.current(visit, cx) {
        active.phase = Phase::Finished;
        if let Some(note) = note {
            cx.note(active.thread.clone(), note);
        }
    }
}

fn saved(state: &mut State, visit: Visit, result: Result<(), String>, cx: &mut AppCx<'_>) {
    if !state
        .saving
        .as_ref()
        .is_some_and(|save| save.visit == visit)
    {
        return;
    }
    let Some(write) = state.saving.take() else {
        return;
    };
    match result {
        Ok(()) => {
            let changed = state.selected.as_deref() != Some(write.selector.as_str());
            if let Some(Operation {
                phase: Phase::Ready(picker),
                ..
            }) = &mut state.operation
            {
                picker.current = Some(write.selector.clone());
            }
            state.selected = Some(write.selector);
            if changed && state.ambient_requested {
                state.fetch_ambient(cx);
            }
        }
        Err(error) => {
            if write.thread.is_some() || cx.view().active().is_none() {
                cx.note(
                    write.thread,
                    Note::SaveFailed {
                        disabling: write.selector == DISABLED,
                        error,
                    },
                );
            }
        }
    }
    finish(state, visit, None, cx);
}

feature! {
    impl Feature for PetSelection {
        const NAME: &'static str = "pets";
        type State = State;
        type Intent = Intent;
        type Ctx = Ctx;
        type Note = Note;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::FetchAmbient => {
                        if !state.ambient_requested {
                            state.ambient_requested = true;
                            state.fetch_ambient(cx);
                        }
                    }
                    Intent::Request { visit, selector, graphics } => request(state, visit, selector, graphics, cx),
                    Intent::ReadPreview { visit, selector } => read_preview(state, visit, selector, cx),
                    Intent::Apply { visit, selector } => apply(state, visit, selector, cx),
                    Intent::RetirePreview(visit) => {
                        if state.operation.as_ref().is_some_and(|active| active.visit == visit) {
                            state.operation = None;
                        }
                    }
                }
            }

            fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
                match (ctx, done) {
                    (Ctx::Ambient(revision), Done::Pet(AssetDone::Ambient(ambient))) if revision == state.ambient_revision => state.ambient = ambient,
                    (Ctx::Catalog { visit, selector }, Done::Pet(AssetDone::Catalog(read))) => catalog(state, visit, selector, read, cx),
                    (Ctx::Preview { visit, revision }, Done::Pet(AssetDone::Preview(read))) => {
                        if let Some(active) = state.current(visit, cx)
                            && active.preview == revision
                            && let Phase::Ready(picker) = &mut active.phase
                            && let Some((_, preview)) = &mut picker.preview
                        {
                            *preview = match read {
                                Ok(image) => Preview::Image(image),
                                Err(error) => Preview::Failed(error),
                            };
                        }
                    }
                    (Ctx::Validate { visit, selector }, Done::Pet(AssetDone::Preview(read))) => {
                        if state.current(visit, cx).is_some_and(|active| matches!(active.phase, Phase::Applying)) {
                            match read {
                                Ok(_) => save(state, visit, selector, cx),
                                Err(error) => finish(state, visit, Some(Note::LoadFailed(error)), cx),
                            }
                        }
                    }
                    (Ctx::Save(visit), Done::Pet(AssetDone::Saved(result))) => saved(state, visit, result, cx),
                    _ => {}
                }
            }

            fn on_shown(state: &mut State, thread: &ThreadId, _cx: &mut AppCx<'_>) {
                if state.operation.as_ref().is_some_and(|active| active.thread.as_ref() != Some(thread)) {
                    state.operation = None;
                }
            }
        }
    }
}
