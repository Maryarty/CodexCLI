//! Ownership of the IDE read requested by `/ide`.

use crate::app::effects::Ask;
use crate::features::{AppCx, AppView};
use crate::ids::{BackendId, ThreadId};

use super::{Ctx, IdeFailure, IdeReading, Note};

/// The displayed conversation's IDE opt-in and latest successful reading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ide {
    /// A read for this visit succeeded. This is the footer's indicator.
    pub enabled: bool,
    generation: u64,
    owner: Option<Owner>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Owner {
    thread: ThreadId,
    cwd: String,
}

impl Ide {
    pub(crate) fn retire(&mut self) {
        self.reset();
    }

    pub(super) fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.enabled = false;
        self.owner = None;
    }

    pub(super) fn command(
        &mut self,
        args: Option<String>,
        source: Option<ThreadId>,
        cx: &mut AppCx<'_>,
    ) {
        let source = source.or_else(|| cx.view().active().cloned());
        let args = args.unwrap_or_default();
        let opted_in = self
            .owner
            .as_ref()
            .is_some_and(|owner| Some(&owner.thread) == source.as_ref());
        match args.trim().to_ascii_lowercase().as_str() {
            "" if opted_in => self.off(source, cx),
            "" | "on" => self.probe(source, cx),
            "off" => self.off(source, cx),
            "status" if opted_in => self.probe(source, cx),
            "status" => cx.note(source, Note::IdeOff),
            _ => cx.note(source, Note::IdeUsage),
        }
    }

    fn off(&mut self, source: Option<ThreadId>, cx: &mut AppCx<'_>) {
        if self
            .owner
            .as_ref()
            .is_none_or(|owner| Some(&owner.thread) == source.as_ref())
        {
            self.reset();
        }
        cx.note(source, Note::IdeOff);
    }

    fn probe(&mut self, source: Option<ThreadId>, cx: &mut AppCx<'_>) {
        let view = cx.view();
        let Some(thread) = source else {
            return cx.note(None, Note::IdeNoLocalWorkspace);
        };
        let Some(cwd) = workspace(view, &thread) else {
            if view.active() == Some(&thread) {
                self.reset();
            }
            return cx.note(Some(thread), Note::IdeNoLocalWorkspace);
        };

        self.reset();
        self.owner = Some(Owner {
            thread: thread.clone(),
            cwd: cwd.clone(),
        });
        cx.effect(
            Ask::IdeContext {
                thread: thread.clone(),
                cwd,
            },
            Ctx::IdeProbe {
                thread,
                generation: self.generation,
            },
        );
    }

    pub(super) fn probed(
        &mut self,
        thread: ThreadId,
        generation: u64,
        reading: Result<IdeReading, IdeFailure>,
        cx: &mut AppCx<'_>,
    ) {
        let view = cx.view();
        let Some(owner) = self
            .owner
            .as_ref()
            .filter(|owner| self.generation == generation && owner.thread == thread)
        else {
            return;
        };
        if workspace(view, &thread).as_deref() != Some(owner.cwd.as_str()) {
            self.reset();
            return;
        }
        match reading {
            Ok(reading) => {
                self.enabled = true;
                cx.note(
                    Some(thread),
                    Note::IdeOn {
                        has_context: reading.has_context,
                    },
                );
            }
            Err(error) => {
                self.reset();
                cx.note(Some(thread), Note::IdeCouldNotEnable { hint: error.hint });
            }
        }
    }
}

fn workspace(view: AppView<'_>, thread: &ThreadId) -> Option<String> {
    let agents = &view.screens().agents;
    let backend = agents.owner_of(thread.as_str());
    if view.active() != Some(thread)
        || view.thread(thread).is_none()
        || backend != BackendId::default()
        || agents.is_cloud(backend)
    {
        return None;
    }
    let cwd = view.working_directory(Some(thread));
    (!cwd.is_empty()).then_some(cwd)
}
