//! What the `@` and `$` popups list from the server (oracle `refresh_startup_skills`,
//! `fetch_plugin_mentions`, `on_connectors_loaded`, then `mention_items`): the skills and the
//! installed plugins of the thread in use, asked for once when its directory is known (the
//! popovers' per-thread part, a thread RPC through `ThreadFx`), and the installed apps of the
//! directory cache (`/apps`). The three make one catalog; the popovers model takes it when the
//! lists change. Run after every event, as the composer's asks of the host are
//! (`drain_popover_asks`): comparing the lists is cheap, the catalog is built when they differ.

use std::rc::Rc;

use super::{AppFx, CodexApp, apply_thread};
use crate::screens::ScreenThreadEv;
use crate::screens::composer::mentions::sources;
use crate::screens::popovers::{self, Ev, Listed, ThreadEv};

pub(super) fn sync(app: &mut CodexApp, fx: &mut AppFx) {
    let Some(thread) = app.session.active.clone() else {
        return;
    };
    let Some(model) = app.threads.get(&thread) else {
        return;
    };
    let slots = model.screen_slots();
    // The lists are the writer's to ask for: a thread another process holds, or the server
    // refused for its provider, is no one's composer to feed.
    if !slots.popovers.asked
        && model.view().has_writer()
        && let Some(cwd) = slots.chat.cwd.clone()
    {
        let load = ThreadEv::Load { cwd };
        apply_thread(
            app,
            &thread,
            crate::threads::ThreadEv::Screen(ScreenThreadEv::Popovers(load)),
            fx,
        );
    }
    let Some(model) = app.threads.get(&thread) else {
        return;
    };
    let part = &model.screen_slots().popovers;
    let apps = app.screens.select.apps.ready();
    let listed = &app.screens.popovers.listed;
    let nothing = part.skills.is_none() && part.plugins.is_none() && apps.is_none();
    let same = listed.thread.as_ref() == Some(&thread)
        && listed.skills == part.skills
        && listed.plugins == part.plugins
        && listed.apps.as_deref() == apps;
    // Nothing listed and nothing made of it before: what a script seeded is not the lists' to
    // replace.
    if same || (nothing && listed.is_empty()) {
        return;
    }
    let listed = Listed {
        thread: Some(thread),
        skills: part.skills.clone(),
        plugins: part.plugins.clone(),
        apps: apps.map(<[_]>::to_vec),
    };
    let mentions = sources::catalog(
        listed.skills.as_deref().unwrap_or_default(),
        listed.plugins.as_deref().unwrap_or_default(),
        &listed.apps.as_deref().map(sources::apps).unwrap_or_default(),
    );
    popovers::apply(
        &mut Rc::make_mut(&mut app.screens).popovers,
        Ev::Catalog { listed, mentions },
        fx,
    );
}
