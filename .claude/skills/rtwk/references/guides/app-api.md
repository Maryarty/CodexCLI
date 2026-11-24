<!-- Written by tools/rtwk-port/skill/generate.py from rtwk c953f062; do not edit. -->

# App and component contexts

The application context types: what a view function, a handler and a `State` can call. Signatures are from the source; the one line is its doc comment.

## AppView

- `rtwk::app::AppView` (struct, `crates/rtwk/src/app.rs:151`) The root of an application: its state (taken by the element once, when it is mounted), its view and its first change.

#### `AppView`: constructors and associated functions

- `pub fn new(app: A, view: ViewFn<A>) -> AppView<A>` - The application `app`, shown by `view`.

#### `AppView`: builder methods (take and return `self`)

- `pub fn exit_on_ctrl_c(mut self, on: bool) -> AppView<A>` - Whether Ctrl+C that no focus node or `InputListener` takes quits the application (`AppBinding::quit`); by default it does, as in twk.
- `pub fn stop_on_ctrl_z(mut self, on: bool) -> AppView<A>` - Whether Ctrl+Z that nothing takes stops the application for the shell it was started from (the process's own terminal on a POSIX system only, which has the terminal as it was before the application took it until `fg` ...
- `pub fn rest_cursor(mut self, on: bool) -> AppView<A>` - Whether the terminal's hidden cursor rests on what has the focus (`Presenter::rests_cursor`: iTerm2's cursor guide keeps still on it, macOS zoom, a screen reader and a braille display follow it); by default it does.
- `pub fn screen_reader(mut self, on: bool) -> AppView<A>` - Whether the application is read out as lines of text for a screen reader (`AppBinding::screen_reader`); by default whether the environment's `TWK_SCREEN_READER` is 1.
- `pub fn start_time(mut self, time: LogTime) -> AppView<A>` - What `AppBinding::now` is when the frame loop's clock reads zero: fixed, as a test does, so that what is drawn of the time is the same on every run; by default the system's time.
- `pub fn on_error(mut self, f: impl FnMut(&RuntimeError) + 'static) -> AppView<A>` - With `f` told of every error `run` finds while it runs the application (twk's `onError`): a `RuntimeError` with its `kind` (the origin of the error), its policy, its message and, where there is one, the `path` of the ...
- `pub fn clipboard_selection(mut self, selection: ClipboardSelection) -> AppView<A>` - Where the host's OSC 52 puts what the app copies (a `SelectionArea`'s selection, a text field's Ctrl+C, `copy_to_clipboard`): the clipboard, the primary selection, or both; by default the clipboard (`c`), as in twk.
- `pub fn clipboard(mut self, clipboard: Clipboard) -> AppView<A>` - With the app's own clipboard in place of the host's OSC 52 write for every copy it makes (a `SelectionArea`'s selection, a text field's Ctrl+C, `copy_to_clipboard`): `Clipboard::Custom` gets each text once and no OSC 52 ...
- `pub fn on_copy(mut self, f: impl FnMut(&str) + 'static) -> AppView<A>` - With `f` told of each text the app copies, as the host writes it to the terminal's clipboard (OSC 52, `clipboard_selection`) or hands it to `clipboard`'s function: where the terminal drops OSC 52 (tmux without ...
- `pub fn on_start(mut self, start: StartFn<A>) -> AppView<A>` - With `start` run as the root is mounted, before the first view.
- `pub fn with_handle(mut self, handle: AppHandle) -> AppView<A>` - With `handle`, the frame loop's: `AppCx::quit` ends the loop.


## ViewCx

- `rtwk::app::ViewCx` (struct, `crates/rtwk/src/app.rs:772`) What a view is given: where its handlers send (its application's root), and the tracked reads a build may make.

#### `ViewCx`: methods

- `pub fn id(&self) -> Eid` - The application's root element.
- `pub fn on<F>(&self, f: F) -> Handler<()> where F: Fn(&mut A, &mut AppCx<'_, '_, A>) + 'static,` - A handler that changes the application: `f` is lent it and an `AppCx`; the view runs again after, unless `f` says `unchanged`.
- `pub fn on_with<E: 'static, F>(&self, f: F) -> Handler<E> where F: Fn(&mut A, E, &mut AppCx<'_, '_, A>) + 'static,` - A handler of events `E` (a component's output: a row chosen, a text typed) that changes the application with what each carries.
- `pub fn generated_route<R, D, E>(&self) -> GeneratedRoute<D, E> where D: Clone + Eq + 'static, E: 'static, R: GeneratedAction<A, D, E>,`
- `pub fn action_route_shared<D, E, F>(&self, action: Rc<F>) -> ActionRoute<F, D, E> where D: 'static, E: 'static, F: ?Sized + Fn(&mut A, D, E, &mut AppCx<'_, '_, A>) + 'static,` - Routes a retained action to this application's root for later binding.
- `pub fn on_key<F>(&self, f: F) -> KeyHandler where F: Fn(&mut A, KeyPress, &mut AppCx<'_, '_, A>) + 'static,` - A key handler taking every key that reaches it (after its `Focus`'s bindings): a modal's trap.
- `pub fn on_keys<E: 'static, M, F>(&self, map: M, f: F) -> KeyHandler where M: Fn(KeyPress) -> Option<E> + 'static, F: Fn(&mut A, E, &mut AppCx<'_, '_, A>) + 'static,` - A key handler taking the keys `map` makes an event of, handing the event to `f`; the others go on up to the `Focus` above.
- `pub fn has_focus(&mut self, id: FocusId) -> bool` - Whether the `Focus` named `id` has the focus, watched: the view runs again when the focus moves.
- `pub fn depend<T: InheritedWidget, R>(&mut self, f: impl FnOnce(&T) -> R) -> Option<R>` - Reads the nearest `T` above the application with `f`, and depends on it (`Cx::depend`).
- `pub fn watch<T: Clone + 'static>(&mut self, obs: Obs<T>) -> T` - The value of `obs`, watched (`Cx::watch`).
- `pub fn memo<D>(&mut self, deps: D, build: MemoFn<D, A>) -> AnyWidget where D: PartialEq + 'static,` - The subtree `build` makes of `deps`, kept while a later view gives equal `deps` at the same place: the same subtree then, which reconciliation skips with everything below it.
- `pub fn memo_keyed<K, D>(&mut self, key: K, deps: D, build: MemoFn<D, A>) -> AnyWidget where K: Hash + Eq + fmt::Debug + 'static, D: PartialEq + 'static,` - `memo` keyed by `key` among its siblings (twk's keys, `Key::value`): the node goes with its key wherever the row moved, so a list sorted anew, a block moved any distance, rows removed or inserted anywhere keep every row ...
- `pub fn root(&self) -> ViewRoot<A>` - The application's root as a `ViewRoot`: a `Copy` handle that a closure the view hands on (a `ListView`'s item builder, a layout builder's) captures to make kept subtrees and handlers to this application where no ...


## AppCx

- `rtwk::app::AppCx` (struct, `crates/rtwk/src/app.rs:951`) What a handler (or the first change, or a task's completion) may do besides changing the application — the effects an A+Msg `update` has.

#### `AppCx`: methods

- `pub fn id(&self) -> Eid` - The application's root element.
- `pub fn origin(&self) -> Eid` - The element in whose build the handler of this change was bound — what twk's handler has as the `context` it captured: the memo node for a handler of `MemoCx` (or of a `ViewRoot` it gave), the root for one of `ViewCx` ...
- `pub fn unchanged(&mut self)` - Suppresses the root view rebuild normally requested by this action.
- `pub fn spawn<R: 'static>(&mut self, future: impl Future<Output = R> + 'static, then: impl FnOnce(&mut A, R, &mut AppCx<'_, '_, A>) + 'static) -> TaskId` - Runs `future` on the tree's executor; `then` applies its output to the application, with a context of its own, if the root is still mounted then.
- `pub fn try_spawn<R: 'static, E: fmt::Display + 'static>(&mut self, future: impl Future<Output = Result<R, E>> + 'static, then: impl FnOnce(&mut A, R, &mut AppCx<'_, '_, A>) + 'static) -> TaskId` - As `spawn`, for a future that may fail: `then` is given what it succeeds with; an error is reported instead — a `RuntimeError` of kind `rtwk_framework::ErrorKind::Task`, recoverable: to the application's `on_error` ...
- `pub fn suspend<R, Fut>(&mut self, run: impl FnOnce() -> Fut + 'static, then: impl FnOnce(&mut A, Result<R, SuspendError>, &mut AppCx<'_, '_, A>) + 'static) -> TaskId where R: 'static, Fut: Future<Output = R> + 'static,` - Hands the terminal to the program `run` makes (`$EDITOR`, a pager, a shell with inherited stdio) once the frame this change asks for is drawn, and takes it back when the program's future completes; `then` applies what ...
- `pub fn suspend_blocking<R>(&mut self, run: impl FnOnce() -> R + Send + 'static, then: impl FnOnce(&mut A, Result<R, SuspendError>, &mut AppCx<'_, '_, A>) + 'static) -> TaskId where R: Send + 'static,` - As `suspend` for a program that blocks (`std::process::Command::status`): `run` runs on a thread of its own (`AppBinding::suspend_blocking`) while the loop goes on.
- `pub fn cancel(&mut self, task: TaskId)` - Cancels `task`: its future is dropped, its `then` never runs.
- `pub fn focus(&mut self, id: FocusId)` - Moves the focus to the `Focus` named `id`.
- `pub fn focus_node(&mut self, node: &FocusNode)` - Gives `node` the primary focus.
- `pub fn blur(&mut self)` - Takes the focus from whatever has it.
- `pub fn focused(&self) -> Option<FocusId>` - The id that has the focus.
- `pub fn emit<E: 'static>(&mut self, h: &Handler<E>, e: E)` - Sends `h` (a component's handler) the message it makes of `e`, after this change.
- `pub fn request_visible_rect(&mut self, resolve: impl FnOnce(&ViewportCx<'_>) -> Option<(Eid, (i32, i32), rtwk_render::Size)> + 'static, completed: impl Fn(&mut A, bool, &mut AppCx<'_, '_, A>) + 'static) -> bool` - Stages an owned, restricted visibility request in the current native tree transaction.
- `pub fn request_viewport(&mut self, operation: impl FnOnce(&mut ViewportCx<'_>) -> bool + 'static, completed: impl Fn(&mut A, bool, &mut AppCx<'_, '_, A>) + 'static) -> bool` - Queues a restricted viewport operation and delivers its result to this app.
- `pub fn observable<T: 'static>(&mut self, value: T) -> Obs<T>` - A new observable holding `value` (`Cx::observable`): made in a handler (or the first change, or a task's completion) it is kept like one made outside, until it is disposed of or the tree is dropped; the view watches it ...
- `pub fn write<T: 'static>(&mut self, obs: Obs<T>, f: impl FnOnce(&mut T) -> bool)` - Changes `obs` with `f` (`Cx::write`).
- `pub fn read<T: 'static, R>(&self, obs: Obs<T>, f: impl FnOnce(&T) -> R) -> R` - Reads `obs` with `f`, not watched (`Cx::read`).
- `pub fn observable_alive<T>(&self, obs: Obs<T>) -> bool` - Whether `obs` is still there (`Cx::observable_alive`).
- `pub fn dispose_observable<T>(&mut self, obs: Obs<T>)` - Disposes of `obs` (`Cx::dispose_observable`).
- `pub fn report_task_error(&mut self, error: impl fmt::Display)` - Reports `error` as the application's task finishing with an error (`Cx::report_task_error`): recoverable, as an error of `try_spawn`'s future is.
- `pub fn channel<T: Send + 'static>(&mut self, f: impl Fn(&mut A, T, &mut AppCx<'_, '_, A>) + 'static) -> rtwk_framework::Sender<T>` - A channel into the application: what any thread sends into the `Sender` is applied by `f`, item by item, at the next flush — every item sent by then, in order, and one view for all of them (ADR-017: work from elsewhere ...
- `pub fn quit(&mut self)` - Ends the frame loop the application runs in (`run`); nothing otherwise.
- `pub fn clear_history(&mut self)` - Erases the terminal's screen and scrollback in the inline mode, and the inline region starts again on the screen's first row (rtwk-core's `App::clear_history`; an rtwk extension, twk has none).
- `pub fn show_dialog<T: 'static>(&mut self, builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static, options: DialogOptions, closed: impl FnOnce(&mut A, Option<T>, &mut AppCx<'_, '_, A>) + 'static) -> AppDialog<T>` - Shows what `builder` builds as a dialog, as `rtwk_overlay::show_dialog` does — over a barrier, the focus kept in it, Escape and a click on the barrier closing it with none when `options` say it is dismissible — and runs ...
- `pub fn show_dialog_from<T: 'static>(&mut self, view_root: ViewRoot<A>, builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static, options: DialogOptions, closed: impl FnOnce(&mut A, Option<T>, &mut AppCx<'_, '_, A>) + 'static) -> AppDialog<T>` - As `show_dialog`, from the origin of `view_root` (`ViewRoot::origin`) rather than this change's: twk's `showDialog(context: stored)` with a context kept from a build.
- `pub fn root(&self) -> ViewRoot<A>` - The application's root as a `ViewRoot` (`ViewCx::root`): a `Copy` handle a dialog's builder captures to make handlers to the application, and kept subtrees of it.


## Cx

- `rtwk::Cx` (struct, `crates/rtwk-framework/src/cx.rs:39`) What a widget's code gets from the framework: the element it runs for (its `id`) and the tree around it.
  - public fields: `owner: &'a mut Owner`, `me: Eid`

#### `Cx`: constructors and associated functions

- `pub fn new(owner: &'a mut Owner, me: Eid) -> Self`

#### `Cx`: methods

- `pub fn id(&self) -> Eid` - The element this code runs for.
- `pub fn segmenter(&self) -> rtwk_core::unicode::TextSegmenter` - The segmenter the render tree measures text with (twk's `segmenterOf(context)`): the terminal's widths, its ambiguous ones among them.
- `pub fn depend<T: InheritedWidget, R>(&mut self, f: impl FnOnce(&T) -> R) -> Option<R>` - Reads the nearest `T` above this element (or its own widget) with `f`, and depends on it: this element is rebuilt (a state told in `State::did_change_dependencies` first) when it is replaced by one that ...
- `pub fn depend_on_element<T: InheritedWidget, R>(&mut self, ancestor: Eid, f: impl FnOnce(&T) -> R) -> Option<R>` - Depends on `ancestor`, the element of an inherited widget `T` above this one, as `depend` does, and reads it with `f` (twk's `dependOnInheritedElement`).
- `pub fn watch_route_paint_visibility<T: InheritedWidget>(&mut self, has_painter: bool, is_visible: impl Fn(&T) -> bool) -> bool` - Internal CustomPaint subscription.
- `pub fn route_paint_visibility_changed(&mut self)` - Internal Navigator notification for an existing route host whose coverage changed.
- `pub fn get_inherited<T: InheritedWidget, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R>` - Reads the nearest `T` above with `f`, without depending on it (twk's `getInheritedWidgetOfExactType`).
- `pub fn inherited_element<T: InheritedWidget>(&self) -> Option<Eid>` - The element of the nearest `T` above, without depending on it (twk's `getElementForInheritedWidgetOfExactType`).
- `pub fn find_ancestor_widget<W: Widget, R>(&self, f: impl FnOnce(&W) -> R) -> Option<R>` - Reads the widget of the nearest ancestor whose widget is a `W` with `f` (twk's `findAncestorWidgetOfExactType`).
- `pub fn find_ancestor_state<S2: State>(&self) -> Option<Eid>` - The element of the nearest stateful ancestor whose state is an `S2` (twk's `findAncestorStateOfType`).
- `pub fn visit_ancestors(&self, mut visit: impl FnMut(Eid, &AnyWidget) -> bool)` - Visits the parents of this element, nearest first, until `visit` returns false (twk's `visitAncestorElements`).
- `pub fn global_key_element(&self, key: &GlobalKey) -> Option<Eid>` - The mounted element in this tree whose widget has `key` (twk's `GlobalKey.currentContext`).
- `pub fn with_widget<W: Widget, R>(&self, e: Eid, f: impl FnOnce(&W) -> R) -> Option<R>` - Reads the current widget of `e` with `f`, without depending on it.
- `pub fn render_object_of(&self, e: Eid) -> Option<RenderId>` - The render object of `e`, or of the nearest element below it that attaches one (twk's `findRenderObject`).
- `pub fn watch<T: Clone + 'static>(&mut self, obs: Obs<T>) -> T` - The value of `obs`, watched: this element is rebuilt when a write changes it.
- `pub fn watch_with<T: 'static, R>(&mut self, obs: Obs<T>, f: impl FnOnce(&T) -> R) -> R` - Reads `obs` with `f`, watched as `watch` is.
- `pub fn read<T: 'static, R>(&self, obs: Obs<T>, f: impl FnOnce(&T) -> R) -> R` - Reads `obs` with `f`, not watched.
- `pub fn observable<T: 'static>(&mut self, value: T) -> Obs<T>` - A new observable holding `value`.
- `pub fn write<T: 'static>(&mut self, obs: Obs<T>, f: impl FnOnce(&mut T) -> bool)` - Changes `obs` with `f`; when `f` answers that it changed it, marks the elements that watch it dirty.
- `pub fn observable_alive<T>(&self, obs: Obs<T>) -> bool` - Whether `obs` is still there to read and write: false once it was disposed of (`dispose_observable`, `WidgetTree::dispose_observable`).
- `pub fn dispose_observable<T>(&mut self, obs: Obs<T>)` - Disposes of `obs`: its watchers are no longer rebuilt for it, its handle is refused from then on (a read or a write of it panics, as after `WidgetTree::dispose_observable`).
- `pub fn report_task_error(&mut self, error: impl fmt::Display)` - Reports `error` as this element's task finishing with an error: a recoverable `ErrorKind::Task` error naming the element by its path (to the tree's error handler, else kept for `take_errors`), as `AppCx::try_spawn` ...
- `pub fn report_debug_check(&mut self, message: impl fmt::Display)` - Reports a failed check of a debug build: a recoverable `ErrorKind::DebugCheck` error naming this element by its path (ADR-027).
- `pub fn mark_needs_build(&mut self, element: Eid)` - Rebuilds `element` in the next build scope (twk's `markNeedsBuild`, as a `setState` from elsewhere does).
- `pub fn emit<E: 'static>(&mut self, h: &Handler<E>, e: E)` - Sends `h` the message it makes of `e`, after the update or the build under way, before the next build scope.
- `pub fn request_viewport(&mut self, operation: impl FnOnce(&mut crate::tree::ViewportCx<'_>) -> bool + 'static, completion: Handler<bool>) -> bool` - Stages a viewport operation at the current native transaction boundary.
- `pub fn dispatch(&mut self, n: &dyn Notification)` - Tells the listeners above this element about `n`, the nearest first, until one of them stops it (twk's `Notification.dispatch`).
- `pub fn serial(&mut self) -> u64` - The next number of a counter the tree keeps (for an item to know the order states were made in).
- `pub fn is_mounted(&self, e: Eid) -> bool` - Whether `e` is a mounted element (active or removed this frame).
- `pub fn with_state<S2: State, R>(&self, e: Eid, f: impl FnOnce(&S2) -> R) -> Option<R>` - Reads the state of `e`, an `S2`, with `f`.
- `pub fn owner_mut(&mut self) -> &mut Owner` - The owner, for the framework's own widgets.
- `pub fn send(&self, msg: S::Msg) -> Handler<()> where S::Msg: Clone + PartialEq,` - A handler that sends this state `msg` (its `State::update`).
- `pub fn map<E: 'static>(&self, f: impl Fn(E) -> S::Msg + 'static) -> Handler<E>` - A handler that sends this state the message `f` makes of each event.
- `pub fn callback<E: 'static>(&self, f: impl Fn(&mut S, E) + 'static) -> Handler<E>` - A handler that calls `f` with this state lent and the event, then rebuilds (twk's `setState` in a callback).
- `pub fn callback_with<E: 'static>(&self, f: impl Fn(&mut S, E, &mut Cx<'_, S>) -> Rebuild + 'static) -> Handler<E>` - A handler that calls `f` with this state lent, the event and a context (to emit, spawn, write); `f` answers whether to rebuild.
- `pub fn map_static<A: handler::StaticMap<E, S::Msg>, E: 'static>(&self) -> Handler<E>` - Framework hook for a map whose behavior is selected by a marker type.
- `pub fn callback_static<A: handler::StaticCallback<S, E>, E: 'static>(&self) -> Handler<E>` - Framework hook for a callback selected by a marker type.
- `pub fn spawn<F>(&mut self, future: F) -> TaskId where F: Future<Output = Option<S::Msg>> + 'static,` - Runs `future` on the tree's executor; the message it ends with, if any, goes to this state's `State::update` — if this element is still mounted then.
- `pub fn cancel(&mut self, task: TaskId)` - Cancels `task` (one this state spawned): its future is dropped.
- `pub fn channel(&mut self) -> crate::Sender<S::Msg> where S::Msg: Send,` - A channel into this state: what any thread sends into the `Sender` comes to its `State::update` as messages — every item sent by the next flush, in order, and one build for all of them (ADR-017: work from elsewhere is ...
- `pub fn local_sender(&mut self) -> Rc<dyn Fn(S::Msg)>` - A function that sends this state a message from a callback the tree calls itself, on its own thread, inside a transaction: a `FocusNode`'s listener, which twk's focus manager calls as it flushes, or a callback a handler ...
- `pub fn channel_with<T: Send + 'static>(&mut self, f: impl Fn(&mut S, &S::Widget, T, &mut Cx<'_, S>) -> Rebuild + 'static) -> crate::Sender<T>` - A channel into this state whose items `f` applies, lent the state, its widget and a context (to write observables, emit, spawn, report an error): as `channel` delivers a message to `State::update`, this delivers an item ...
- `pub fn keys<F>(&self, f: F) -> KeyHandler where F: Fn(KeyPress) -> Option<S::Msg> + 'static,` - A key handler that sends this state the message `f` makes of each key, and lets a key go on up when `f` makes none.
- `pub fn has_focus(&mut self, id: FocusId) -> bool` - Whether the `Focus` named `id` has the focus, watched: this element is rebuilt when the focus moves.
- `pub fn focused(&self) -> Option<FocusId>` - The id that has the focus: the one asked for while no `Focus` shows it, else the name of the `Focus` with the primary focus.
- `pub fn focus(&mut self, id: Option<FocusId>)` - Moves the focus to the `Focus` named `id` (none: nothing has it): at once when it is in the tree, when it mounts when not.
- `pub fn focus_node(&mut self, node: &FocusNode)` - Gives an active, focusable node in this tree the primary focus.
- `pub fn focus_manager(&self) -> FocusManager` - The focus manager of this tree.


## BuildCx

- `rtwk::BuildCx` (type, `crates/rtwk-framework/src/cx.rs:49`) The context of a build: `Cx`.


## UpdateCx

- `rtwk::UpdateCx` [path inferred] (type, `crates/rtwk-framework/src/cx.rs:51`) The context of an update: `Cx`.


## Handler

- `rtwk::Handler` (struct, `crates/rtwk-framework/src/handler.rs:56`) A handler of events `E`: makes a message (or a closure call) for the state of the element that made it.
  - public fields: `target: Eid`, `make: Rc<dyn Maker<E>>`

#### `Handler`: methods

- `pub fn target(&self) -> Eid` - The element the handler is for.


## Obs

- `rtwk::Obs` (struct, `crates/rtwk-framework/src/obs.rs:23`) A handle on an observable value of type `T`.


## Navigator

- `rtwk::navigation::Navigator` (struct, `crates/rtwk/src/navigation/navigator.rs:1722`) Shows a stack of `Route`s, the last pushed in front: its first, which `initial_route` names (`DEFAULT_ROUTE_NAME` by default), and those pushed on it (`NavigatorState::push`, `NavigatorState::push_named`) until they are ...
  - public fields: `home: Option<AnyWidget>`, `routes: Vec<(Str`, `initial_route: Option<Str>`, `on_generate_route: Option<RouteFactory>`, `on_unknown_route: Option<RouteFactory>`, `back_key: Option<Str>`, `observers: Vec<Rc<dyn NavigatorObserver>>`

#### `Navigator`: constructors and associated functions

- `pub fn new() -> Navigator` - A navigator with no route made yet: give it a `home` or `routes`.
- `pub fn of<S>(cx: &Cx<'_, S>) -> NavigatorState` - The nearest navigator above `cx`.
- `pub fn maybe_of<S>(cx: &Cx<'_, S>) -> Option<NavigatorState>` - The navigator `of` finds, if there is one.
- `pub fn root_of<S>(cx: &Cx<'_, S>) -> NavigatorState` - The furthest navigator above `cx`, the root one.
- `pub fn maybe_root_of<S>(cx: &Cx<'_, S>) -> Option<NavigatorState>` - The navigator `root_of` finds, if there is one.
- `pub fn push<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>) -> DialogResult<T>` - `NavigatorState::push` on the navigator above `cx`.
- `pub fn push_named<T: 'static, S>(cx: &Cx<'_, S>, name: &str) -> DialogResult<T>` - `NavigatorState::push_named` on the navigator above `cx`.
- `pub fn push_named_with<T: 'static, S>(cx: &Cx<'_, S>, name: &str, arguments: impl Any + fmt::Debug) -> DialogResult<T>` - `NavigatorState::push_named_with` on the navigator above `cx`.
- `pub fn push_replacement<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>) -> DialogResult<T>` - `NavigatorState::push_replacement` on the navigator above `cx`.
- `pub fn push_replacement_with<T: 'static, R: Any + Clone, S>(cx: &Cx<'_, S>, route: impl Into<Route>, result: R) -> DialogResult<T>` - `NavigatorState::push_replacement_with` on the navigator above `cx`.
- `pub fn push_replacement_named_with<T: 'static, R: Any + Clone, S>(cx: &Cx<'_, S>, name: &str, arguments: Option<Arguments>, result: Option<R>) -> DialogResult<T>` - `NavigatorState::push_replacement_named_with` on the navigator above `cx`.
- `pub fn push_and_remove_until<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_and_remove_until` on the navigator above `cx`.
- `pub fn push_named_and_remove_until<T: 'static, S>(cx: &Cx<'_, S>, name: &str, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_named_and_remove_until` on the navigator above `cx`.
- `pub fn push_named_and_remove_until_with<T: 'static, S>(cx: &Cx<'_, S>, name: &str, arguments: impl Any + fmt::Debug, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_named_and_remove_until_with` on the navigator above `cx`.
- `pub fn pop_and_push_named<T: 'static, S>(cx: &mut Cx<'_, S>, name: &str) -> DialogResult<T>` - `NavigatorState::pop_and_push_named` on the navigator above `cx`.
- `pub fn pop_and_push_named_with<T: 'static, R: Any + Clone, S>(cx: &mut Cx<'_, S>, name: &str, arguments: Option<Arguments>, result: Option<R>) -> DialogResult<T>` - `NavigatorState::pop_and_push_named_with` on the navigator above `cx`.
- `pub fn pop<S>(cx: &mut Cx<'_, S>)` - `NavigatorState::pop` on the navigator above `cx`.
- `pub fn pop_with<S, R: Any + Clone>(cx: &mut Cx<'_, S>, result: R)` - `NavigatorState::pop_with` on the navigator above `cx`.
- `pub fn pop_until<S>(cx: &mut Cx<'_, S>, predicate: impl Fn(&Route) -> bool)` - `NavigatorState::pop_until` on the navigator above `cx`.
- `pub fn maybe_pop<S>(cx: &mut Cx<'_, S>) -> bool` - `NavigatorState::maybe_pop` on the navigator above `cx`.
- `pub fn maybe_pop_with<S, R: Any + Clone>(cx: &mut Cx<'_, S>, result: R) -> bool` - `NavigatorState::maybe_pop_with` on the navigator above `cx`.
- `pub fn can_pop<S>(cx: &Cx<'_, S>) -> bool` - `NavigatorState::can_pop` on the navigator above `cx`; false when there is none.

#### `Navigator`: builder methods (take and return `self`)

- `pub fn home(mut self, home: impl Into<AnyWidget>) -> Navigator` - With `home` as the route named `/`.
- `pub fn route(mut self, name: impl Into<Str>, builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static) -> Navigator` - With the route `name` showing what `builder` builds.
- `pub fn initial_route(mut self, name: impl Into<Str>) -> Navigator` - Starting at the route `name`.
- `pub fn on_generate_route(mut self, f: impl Fn(&RouteSettings) -> Option<Route> + 'static) -> Navigator` - Making the routes `home` and `routes` do not make with `f`.
- `pub fn on_unknown_route(mut self, f: impl Fn(&RouteSettings) -> Option<Route> + 'static) -> Navigator` - Making the routes nothing else makes with `f`.
- `pub fn back_key(mut self, key: Option<&str>) -> Navigator` - Popping the route in front on `key`, none for no key.
- `pub fn observer<O: NavigatorObserver>(mut self, observer: Rc<O>) -> Navigator` - Telling `observer` of each change.


## PageRoute

- `rtwk::navigation::PageRoute` (struct, `crates/rtwk/src/navigation/navigator.rs:716`) A screen: what its builder builds, covering the routes under it.

#### `PageRoute`: constructors and associated functions

- `pub fn new(builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static) -> PageRoute<T>` - A screen of what `builder` builds.

#### `PageRoute`: builder methods (take and return `self`)

- `pub fn settings(mut self, settings: RouteSettings) -> PageRoute<T>` - Known by `settings`.
- `pub fn transition(mut self, d: Duration) -> PageRoute<T>` - Coming in over `d`.
- `pub fn reverse_transition(mut self, d: Duration) -> PageRoute<T>` - Going out over `d` (the time to come in where none is given).
- `pub fn into_route(self) -> Route` - The route to push; a result it is popped with has to be a `T`.
- `pub fn into_route_any(self) -> Route` - The route to push, popped with a result of any type (twk's `Route<Object?>`): the type of what `push` returns is the push's alone.

#### `PageRoute`: methods

- `pub fn transition_duration(&self) -> Duration` - How long it takes to come in.
- `pub fn reverse_transition_duration(&self) -> Duration` - How long it takes to go out.
