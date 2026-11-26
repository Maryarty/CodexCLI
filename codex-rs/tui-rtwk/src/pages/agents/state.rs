//! `ScreenState`: modes, controllers, key handling, the `State` impl.

use super::keys::*;
use super::*;

impl ScreenState {
    /// The list's cursor as the highlight core knows.
    fn cursor(&self) -> Option<Sel> {
        self.selection.cursor().and_then(view::RowKey::into_sel)
    }

    /// Moves the list's cursor (the one copy of the highlight).
    fn put_cursor(&self, key: view::RowKey) {
        if self.selection.cursor().as_ref() != Some(&key) {
            self.selection.set_cursor(Some(key.clone()));
            self.selection.select(key);
        }
    }

    /// What the view shows that changes which rows count.
    fn lens(&self) -> Lens {
        Lens {
            filter: self.tabs.index(),
            searching: self.mode == CenterMode::Search,
            renaming: match &self.mode {
                CenterMode::Rename(target) => Some(target.clone()),
                _ => None,
            },
        }
    }

    /// What the widgets read of the screen's own state.
    fn ui(&self) -> view::Ui {
        view::Ui {
            filter: self.tabs.index(),
            help: self.mode == CenterMode::Help,
            searching: self.mode == CenterMode::Search,
            renaming: matches!(self.mode, CenterMode::Rename(_)),
        }
    }

    fn editing_now(&self) -> bool {
        matches!(self.mode, CenterMode::Search | CenterMode::Rename(_))
    }

    fn send(cx: &mut Cx<'_, Self>, intent: AgentsIntent) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Agents(intent));
    }

    /// The cursor goes to the task after `id` leaves (oracle:
    /// `selection_after_removal`, a pure function of the model).
    fn after_removal(&self, model: &Model, id: &str) {
        if let Some(next) = model.selection_after_removal(id, &[id.to_owned()], &self.lens()) {
            self.put_cursor(view::RowKey::Task(next));
        }
    }

    /// The answer to the confirmation: a yes asks the model to remove the
    /// task (the cursor leaves it at once).
    fn removal(&mut self, model: &Model, confirm: view::Confirm, cx: &mut Cx<'_, Self>) {
        self.after_removal(model, &confirm.thread);
        Self::send(
            cx,
            match confirm.action {
                Action::Archive => AgentsIntent::Archive(confirm.thread),
                Action::Delete => AgentsIntent::Delete(confirm.thread),
            },
        );
    }

    /// The editing line changes mode; its text is reset for the new one.
    fn set_mode(&mut self, mode: CenterMode, center: &Center) {
        if mode == self.mode {
            return;
        }
        match &mode {
            CenterMode::Search => {
                self.browse = self.selection.cursor();
                self.edit.reset("");
            }
            CenterMode::Rename(target) => {
                let name = center
                    .rows
                    .iter()
                    .find(|row| row.id() == target.as_str())
                    .and_then(|row| row.name.clone())
                    .unwrap_or_default();
                self.edit.reset(&name);
            }
            CenterMode::Browse | CenterMode::Help => {
                if self.mode == CenterMode::Search
                    && let Some(key) = self.browse.take()
                {
                    self.put_cursor(key);
                }
            }
        }
        self.mode = mode;
    }

    /// Enter on a row: the task opens (a search closes), "Show more" asks for
    /// the next rows or matches.
    fn accept(&mut self, model: &Model, sel: Sel, center: &Center, cx: &mut Cx<'_, Self>) {
        let searching = self.mode == CenterMode::Search;
        match sel {
            Sel::ShowMore if searching => Self::send(cx, AgentsIntent::MoreMatches(self.lens())),
            Sel::ShowMore => Self::send(cx, AgentsIntent::ShowMore),
            Sel::Task(_) if matches!(self.mode, CenterMode::Rename(_)) => {}
            Sel::Task(id) if !model.held() && model.failure.is_none() && model.has_task(&id) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.open_thread, codex_tui_core::ids::ThreadId::new(id));
                if searching {
                    self.set_mode(CenterMode::Browse, center);
                    Self::send(cx, AgentsIntent::ClearSearch);
                }
            }
            Sel::Task(_) => {}
        }
    }

    /// Esc or ctrl+c on an open line: a search is cleared, the line closes.
    fn cancel_editing(&mut self, _model: &Model, center: &Center, cx: &mut Cx<'_, Self>) {
        if self.mode == CenterMode::Search {
            Self::send(cx, AgentsIntent::ClearSearch);
        }
        self.set_mode(CenterMode::Browse, center);
    }

    /// Opens the confirmation of a removal (a dialog of rtwk's overlay).
    fn ask_removal(&mut self, model: &Model, confirm: view::Confirm, cx: &mut Cx<'_, Self>) {
        let ask = view::Ask {
            name: view::confirm_name(strings::of(cx), model, &confirm),
            cloud: model.cloud,
            confirm: confirm.clone(),
        };
        let answer = cx.map(move |_: bool| Msg::Confirmed(confirm.clone()));
        drop(show_dialog::<(), _>(
            cx,
            move |_| {
                dialogs::ConfirmDialog {
                    ask: ask.clone(),
                    answer: answer.clone(),
                }
                .into()
            },
            DialogOptions {
                barrier_alpha: Some(0),
                fade_in: Duration::ZERO,
                ..DialogOptions::default()
            },
        ));
    }

    /// The usage of the highlighted task is asked for when what it depends
    /// on changes: the task, its fetch, the clock (oracle: the repaint that
    /// follows a move asks).
    fn ask_usage(&mut self, model: &Model, center: &Center, cx: &mut Cx<'_, Self>) {
        let thread = center.selected_row().map(|row| row.id().to_owned());
        let usage_key = UsageKey {
            fetched_at: thread
                .as_ref()
                .and_then(|id| model.usage.get(id))
                .and_then(|usage| usage.fetched_at),
            thread,
            pending: model.pending_usage.is_some(),
            now: model.now,
        };
        if self.usage_seen.as_ref() != Some(&usage_key) {
            self.usage_seen = Some(usage_key);
            let thread = self.usage_seen.as_ref().and_then(|key| key.thread.clone());
            Self::send(cx, AgentsIntent::NeedUsage(thread));
        }
    }

    /// The cursor, the keys' focus and the usage request follow the rows the
    /// model and the tab show.
    fn sync(&mut self, model: &Model, cx: &mut Cx<'_, Self>) -> Center {
        let center = model.center_at(
            &read::<CenterThreads, _>(cx),
            self.cursor().as_ref(),
            &self.lens(),
        );
        if let Some(entry) = center.selected_entry() {
            self.put_cursor(view::key_of(&center, entry));
        } else if center.entries.is_empty() && self.selection.cursor().is_some() {
            // Oracle (`reconcile_command_center_selection`): nothing
            // selectable sets the selection to `usize::MAX`, so the first
            // row is the highlight when rows come back.
            self.selection.set_cursor(None);
        }
        self.ask_usage(model, &center, cx);
        center
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        self.listening = Some(Listening::new(cx, self.selection.listenable(), Msg::Cursor));
        self.tab_taps = (0..view::FILTERS)
            .map(|index| cx.map(move |_: PointerEvent| Msg::Tab(index)))
            .collect();
        if w.open {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::Open));
        }
    }

    fn dispose(&mut self) {
        if let Some(listening) = &self.listening {
            listening.cancel();
        }
        self.selection.dispose();
        self.tabs.dispose();
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let agents = watched(cx);
        watch::<CenterThreads, _>(cx);
        let model = &*agents;
        // A task is being opened from here: its page is pushed at once, in
        // its loading state, and goes when the reply comes (a failure shows
        // the center again with the error).
        if model.opening.is_some() && !self.opening_pushed && Route::is_current_of(cx) {
            self.opening_pushed = true;
            let popped = cx.map(|_: PopInvoked| Msg::OpeningClosed);
            drop(Navigator::of(cx).push::<()>(super::opening::route(popped)));
        }
        let answered = model.opening.is_none() && self.opening_pushed;
        if answered {
            self.opening_pushed = false;
            Navigator::maybe_pop(cx);
        }
        match self.entered {
            None => self.entered = Some(model.resumes),
            Some(entered) if model.resumes > entered => {
                // A resume was answered: the chat shows the thread now.
                Navigator::maybe_pop(cx);
            }
            Some(_) => {}
        }
        // The list's cursor is the only copy of the highlight; a cursor whose
        // row left the list moves to the default row.
        if model.search != self.search_seen {
            self.search_seen = model.search.clone();
            let center = model.center_at(&read::<CenterThreads, _>(cx), None, &self.lens());
            if let Some(index) = center
                .entries
                .iter()
                .position(|entry| matches!(entry, CenterRow::Task(_)))
            {
                self.put_cursor(view::key_of(&center, index));
            }
        }
        self.sync(model, cx);
        self.retap(cx);
        if model.failure.is_some() != self.failing {
            self.failing = model.failure.is_some();
            if self.failing {
                // The refusal's view replaces the screen's content (the
                // build draws none while it shows) at the top left.
                drop(show_dialog::<(), _>(
                    cx,
                    |_| dialogs::FailureDialog.into(),
                    DialogOptions {
                        alignment: Alignment::TOP_LEFT,
                        barrier_dismissible: false,
                        barrier_alpha: Some(0),
                        fade_in: Duration::ZERO,
                        ..DialogOptions::default()
                    },
                ));
            }
        }
    }

    fn update(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let rebuild = self.handle(msg, w, cx);
        if rebuild == Rebuild::Yes {
            self.retap(cx);
        }
        rebuild
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        self.view(w, cx)
    }
}

impl ScreenState {
    /// The handlers of the row clicks, kept while the rows are the same
    /// ones (a handler made anew is never equal to the last, and the list
    /// would not compare equal), made here, not in `build`.
    fn retap(&mut self, cx: &mut Cx<'_, Self>) {
        let agents = read::<ScreenStates, _>(cx).agents.clone();
        let center = agents.center_at(
            &read::<CenterThreads, _>(cx),
            self.cursor().as_ref(),
            &self.lens(),
        );
        let keys: Vec<view::RowKey> = (0..center.entries.len())
            .map(|index| view::key_of(&center, index))
            .collect();
        if self.row_taps.as_ref().is_some_and(|(was, _)| *was == keys) {
            return;
        }
        let taps = Rc::new(
            keys.iter()
                .map(|key| {
                    let key = key.clone();
                    cx.map(move |_: PointerEvent| Msg::Row(key.clone()))
                })
                .collect::<Vec<_>>(),
        );
        self.row_taps = Some((keys, taps));
    }

    /// The key (or click or answer) changes the screen's own state and asks
    /// the model.
    fn handle(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let agents = read::<ScreenStates, _>(cx).agents.clone();
        let model = &*agents;
        // The list's cursor is read here, when the key or click arrives: it
        // moves at once, the model only at the flush.
        let selected = self.selection.cursor();
        let at = selected.clone().and_then(view::RowKey::into_sel);
        let center = model.center_at(&read::<CenterThreads, _>(cx), at.as_ref(), &self.lens());
        match msg {
            Msg::Act(act) => {
                let editing = self.editing_now();
                let task = || match &at {
                    Some(Sel::Task(id)) => Some(id.clone()),
                    _ => None,
                };
                match act {
                    Act::Interrupt => {
                        // `on_ctrl_c` takes the key while a line is open (it
                        // cancels it) or a removal runs; otherwise it is the
                        // app's: quit.
                        if editing {
                            self.cancel_editing(model, &center, cx);
                            return Rebuild::Yes;
                        }
                        if !matches!(
                            model.notice,
                            Some(codex_tui_core::screens::agents::Notice::Removing(_))
                        ) {
                            let hs = read::<Hs, _>(cx);
                            cx.emit(&hs.quit, ());
                        }
                    }
                    Act::Escape if self.mode == CenterMode::Help => {
                        self.mode = CenterMode::Browse;
                        return Rebuild::Yes;
                    }
                    Act::Escape if editing => {
                        self.cancel_editing(model, &center, cx);
                        return Rebuild::Yes;
                    }
                    Act::Escape => {
                        if w.can_return {
                            Self::send(cx, AgentsIntent::CancelOpen);
                            Navigator::maybe_pop(cx);
                        }
                    }
                    Act::Move(mv) => {
                        let len = center.entries.len();
                        // A step rests on "Show more" too (not while it reads
                        // "Searching history…"); a page or an end stops at a
                        // task (oracle `move_selection` / paging).
                        let searching = model.history.as_ref().is_some_and(|h| h.loading);
                        let selectable = |at: usize| {
                            let key = view::key_of(&center, at);
                            match mv {
                                Move::Step(_) => {
                                    key.selectable()
                                        && !(searching && key == view::RowKey::ShowMore)
                                }
                                _ => matches!(key, view::RowKey::Task(_)),
                            }
                        };
                        if let Some(to) = target(
                            len,
                            center.selected_entry(),
                            &selectable,
                            mv,
                            page_rows(&self.list),
                        ) && let key = view::key_of(&center, to)
                            && key.clone().into_sel().is_some()
                        {
                            self.put_cursor(key);
                            // Going up, the heading and the gap above a task
                            // come into view with it (the oracle scrolls to
                            // the group).
                            let mut head = to;
                            if center.selected_entry().is_some_and(|from| to < from) {
                                while head > 0 && !view::key_of(&center, head - 1).selectable() {
                                    head -= 1;
                                }
                            }
                            self.list.ensure_index_visible(head);
                            return Rebuild::Yes;
                        }
                    }
                    Act::Accept => {
                        if let Some(sel) = at {
                            self.accept(model, sel, &center, cx);
                            return Rebuild::Yes;
                        }
                    }
                    Act::Filter { back } => {
                        let count = view::FILTERS;
                        self.tabs.set_index(
                            (self.tabs.index() + if back { count - 1 } else { 1 }) % count,
                        );
                        self.sync(model, cx);
                        return Rebuild::Yes;
                    }
                    Act::Help => {
                        self.mode = if self.mode == CenterMode::Help {
                            CenterMode::Browse
                        } else {
                            CenterMode::Help
                        };
                        return Rebuild::Yes;
                    }
                    // The search key toggles the line (oracle: `search.is_pressed`
                    // calls `on_ctrl_c` while searching).
                    Act::Search => {
                        if self.mode == CenterMode::Search {
                            self.cancel_editing(model, &center, cx);
                            return Rebuild::Yes;
                        }
                        if !matches!(self.mode, CenterMode::Rename(_)) {
                            self.set_mode(CenterMode::Search, &center);
                            return Rebuild::Yes;
                        }
                    }
                    // A line already renaming keeps its target (oracle:
                    // `rename_target.is_none()`).
                    Act::Rename => {
                        if !matches!(self.mode, CenterMode::Rename(_))
                            && let Some(id) = task()
                            && !model.held()
                            && model.has_task(&id)
                        {
                            Self::send(cx, AgentsIntent::ClearSearch);
                            self.set_mode(CenterMode::Rename(id), &center);
                            return Rebuild::Yes;
                        }
                    }
                    Act::Archive | Act::Delete => {
                        if let Some(thread) = task()
                            && !model.held()
                            && model.has_task(&thread)
                        {
                            let action = if act == Act::Archive {
                                Action::Archive
                            } else {
                                Action::Delete
                            };
                            self.ask_removal(model, view::Confirm { thread, action }, cx);
                        }
                    }
                    Act::Swallow => {}
                    Act::Right => {
                        if let Some(sel) = at {
                            self.accept(model, sel, &center, cx);
                        }
                    }
                    // The picker is a route over the center (`push`): a pick
                    // resumes the task, Esc pops back to the center as it was.
                    // Held back while a notice holds the center.
                    Act::Resume => {
                        if !model.held() {
                            drop(Navigator::of(cx).push::<()>(crate::pages::resume::route()));
                        }
                    }
                    Act::Grouping => Self::send(cx, AgentsIntent::CycleGrouping),
                    Act::New => Self::send(cx, AgentsIntent::New(task())),
                    Act::NewWorktree => Self::send(cx, AgentsIntent::NewWorktree(task())),
                    Act::Hide | Act::Pin | Act::Stop | Act::Fork => {
                        if let Some(id) = task() {
                            if act == Act::Hide {
                                self.after_removal(model, &id);
                            }
                            Self::send(
                                cx,
                                match act {
                                    Act::Hide => AgentsIntent::Hide(id),
                                    Act::Pin => AgentsIntent::TogglePin(id),
                                    Act::Stop => AgentsIntent::Stop(id),
                                    _ => AgentsIntent::Fork(id),
                                },
                            );
                        }
                    }
                }
            }
            Msg::Cursor => {
                if let Some(l) = &self.listening {
                    l.handled();
                }
                // The cursor is the list's own: only the rows that show it
                // (details, keys) are rebuilt; the usage of the task it
                // lands on is asked for at once.
                self.ask_usage(model, &center, cx);
                return Rebuild::Yes;
            }
            Msg::Row(key) => {
                // A click on the row that is highlighted opens it, on another
                // one highlights it (the list's cursor follows the model).
                if selected.as_ref() == Some(&key) {
                    if let Some(sel) = at {
                        self.accept(model, sel, &center, cx);
                        return Rebuild::Yes;
                    }
                } else if key.clone().into_sel().is_some()
                    && !matches!(self.mode, CenterMode::Rename(_))
                {
                    self.put_cursor(key);
                    return Rebuild::Yes;
                }
            }
            Msg::OpeningClosed => {
                // The loading page was popped while the resume is still out:
                // Esc gave the opening up. (A reply pops it after the model
                // forgot the opening: nothing to cancel then.)
                if model.opening.is_some() {
                    self.opening_pushed = false;
                    Self::send(cx, AgentsIntent::CancelOpen);
                }
            }
            Msg::Confirmed(confirm) => {
                self.removal(model, confirm, cx);
                return Rebuild::Yes;
            }
            Msg::Search(text) => {
                if self.mode == CenterMode::Search {
                    let lens = self.lens();
                    Self::send(cx, AgentsIntent::SetSearch { text, lens });
                }
            }
            Msg::Submit(text) => {
                if let CenterMode::Rename(thread) = self.mode.clone() {
                    // An empty name keeps the line open.
                    if !text.trim().is_empty() {
                        Self::send(cx, AgentsIntent::Rename { thread, name: text });
                        self.set_mode(CenterMode::Browse, &center);
                        return Rebuild::Yes;
                    }
                } else if let Some(sel) = at {
                    self.accept(model, sel, &center, cx);
                    return Rebuild::Yes;
                }
            }
            Msg::Tab(index) => {
                self.tabs.set_index(index.min(view::FILTERS - 1));
                self.sync(model, cx);
                return Rebuild::Yes;
            }
        }
        Rebuild::No
    }

    /// The view of the screen's state.
    fn view(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let keymap = crate::keymap::keymap(cx);
        let hints = Hints::new(keymap.clone());
        let agents = watched(cx);
        let look = view::Look::new(&Theme::of(cx), agents.thread_colors());
        let center = Rc::new(agents.center_at(
            &watch::<CenterThreads, _>(cx),
            self.cursor().as_ref(),
            &self.lens(),
        ));
        let ui = self.ui();
        let taps = view::Taps {
            rows: self
                .row_taps
                .as_ref()
                .map_or_else(|| Rc::new(Vec::new()), |(_, taps)| taps.clone()),
            tabs: self.tab_taps.clone(),
        };
        let controls = view::Controls {
            selection: self.selection.clone(),
            list: self.list.clone(),
            edit: self.edit.clone(),
            focus: self.list_focus.clone(),
            empty_focus: self.empty_focus.clone(),
            keys: view::Keys {
                browse: table(cx, browse(&keymap)),
                search: table(cx, search(&keymap)),
                rename: table(cx, rename(&keymap)),
                help: table(cx, help(&keymap)),
            },
            edit_focus: self.edit_focus.clone(),
            scroll: self.scroll.clone(),
            search: cx.map(|text: Rc<str>| Msg::Search(text.to_string())),
            submit: cx.map(|text: Rc<str>| Msg::Submit(text.to_string())),
        };
        let can_return = w.can_return;
        let help_focus = self.help_focus.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let model = &*agents;
            if model.failure.is_some() {
                return AnyWidget::default();
            }
            // The header takes three rows from six, the footer one from two.
            let mut column = Column::new()
                .overflow_intended(true)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            if constraints.max_height >= 6 {
                column = column.child(view::header(
                    &look, strings, &hints, &num, model, &ui, &center, &taps,
                ));
            }
            // Both stay mounted (the list keeps its cursor and scroll); the
            // help takes the room and the keys while it shows.
            // Both panes stay mounted, one offstage and out of the focus
            // (the list keeps its cursor and scroll); the help takes the
            // room and the keys while it shows.
            let help = ui.help;
            let body = view::body(
                &look, strings, &num, &agents, &ui, &center, &controls, &taps,
            );
            let help_pane = KeyBindings::new(
                controls.keys.help.clone(),
                Focus::new(view::help(&look, strings, &hints, model, &center))
                    .focus_node(help_focus.clone()),
            );
            let stack = Stack::new()
                .fit(StackFit::Expand)
                .child(Offstage::new(help, ExcludeFocus::new(body).excluding(help)))
                .child(Offstage::new(
                    !help,
                    ExcludeFocus::new(help_pane).excluding(!help),
                ));
            column = column.child(Expanded::new(1, stack));
            if constraints.max_height >= 2 {
                column = column.child(view::footer_row(
                    &look, strings, &hints, model, &ui, can_return,
                ));
            }
            column.into()
        });
        FocusScope::new(body).into()
    }
}
