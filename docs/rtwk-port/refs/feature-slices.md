# Moving a feature into a slice: the template (from the `/goal` move)

ADR-012 (`architecture.md`, "Recipe: a feature as a slice") says what a slice is; this is
the same thing as steps, worked once on `/goal` (commit `913860d68`,
`core/src/features/goal`, `src/features/goal`). Read it with the two files open. The
mechanical parts are `docs/rtwk-port/refs/slice-codemod.py` (`new`, `move`, `hub`, `budget`);
the rest is the decisions below, and they are a few per feature.

Run commands from `codex-rs/tui-rtwk`. Checks per step: `acargo check --tests --quiet`; at the
end `acargo nextest run --test rules`, `-p codex-tui-core` for the feature's own test file,
`acargo clippy -q --all-targets -- -D warnings`, and the oracle shard of the feature.

## 0. Know what the feature is made of

`python3 -I ../../docs/rtwk-port/refs/slice-codemod.py hub <f>` lists every line of the hub
files that names the feature. Sort them into the table of step 4. The ones that are not hub
variants (a model, a helper, a note's words) move as files (step 2); the variants become
payloads of the feature's own types.

## 1. The skeleton

`slice-codemod.py new <f> <Marker>` writes `core/src/features/<f>/mod.rs` (every component
`Never<Self>`), `src/features/<f>/{mod,strings}.rs` and the line `<f> => <Marker>;` in
`feature_list!` (`core/src/features/mod.rs`). It compiles as written. **The line's position is
the hook order**: a feature whose `on_turn_end` sends the next message goes before one that
reads whether the thread is busy. A feature without RPCs keeps `wire` / `decode` as
`command.absurd()` (`Never::absurd`).

## 2. Move the files, then name the components

- A file of `core/src` that only the feature uses (`screens/select/goal.rs`,
  `app/session/goal.rs`): `slice-codemod.py move <f> <path> [<name>]` (git mv, the `mod`
  line follows, the `crate::`/`codex_tui_core::` paths are rewritten). Grouped `use` lines,
  `super::` and re-exports are the compiler's.
- Split it as it reads: `model.rs` (state and decisions, no hub types), `menu.rs`
  (`Menu`, `Pick`, `choose`), `wire.rs` (`Ask` / `Reply`, `request()` builds the
  `ClientRequest`, `reply()` decodes), `note.rs` (`Note`, its payload types), `mod.rs` (the
  marker, `Intent`, `Prompt`, `impl Feature`).
- Renames that the move allows: `GoalAction` -> `goal::Action`, `GoalHint` ->
  `goal::UsageHint`, `GoalFailure` -> `goal::Failure`, `goal::Hint` -> `goal::CommandsHint`
  (the slice's module already says "goal").

## 3. The components: which hub variant becomes what

| it was (hub) | it is (slice) | the hub keeps |
|---|---|---|
| `Intent::Goal`-style slash dispatch in `core/src/app/session.rs` `S::Goal { args }` | `type Intent = goal::Intent` (`Command { source, args }`); the router builds it: `super::features::intent(app, goal::Intent::Command { .. }.into(), fx)` | the oracle's `SessionIntent` variant (the slash list stays the oracle's) |
| `select::Command::Goal(..)` / `Ctx::Goal` / `Reply::Goal*` and `ThreadAsk::Goal*` | `type Command = goal::Ask`, `type Ctx = goal::Ctx`, `type Reply = goal::Reply`; `wire` builds the `ClientRequest`, `decode` reads the result | `Command::Feature`, `AppCtx::Feature`, `Answer::Feature` (one each, generic) |
| `Menu::Goal(..)`, its `choose` arm, its `page()` arm | `type Menu = goal::Menu`, `choose(menu, row) -> Choice::{Stay, Dismiss, Pick, Replace}`; the rows are `FeatureView::menu_list(menu, state, words)` in `src/features/goal/strings.rs`: **from the feature's state**, read at every build, so a menu whose rows change while it is open (install progress, a stale answer, an editor) is the same menu with other rows, never a reopen (a reopen replaces the stack: the cursor and the tab are lost). What a typed family had as `Model::page()` built from the select model is a read of `state` here | `Menu::Feature` |
| a fact the host sends unasked: `AppEv::Presets(..)`, `AppEv::PluginConfig(..)`, and the arm in `apply_app` that hands it to a screen's model (`select::event(.., Ev::Dials / Ev::Plugins(Config))`) | `type Fact = presets::Fact` (the payload, the feature's own type); `fn on_fact(state, fact, cx)` keeps it in the feature's `State`; the host sends `AppEv::Feature(FeatureFact::from(fact))` (`host/src/in_process.rs` builds it with the feature's type and `From`: the one file besides the feature's own and the list that may name `features::<f>::Fact`, `FACT_SENDERS` in `tests/rules/layers.rs`); the open menu reads it through `menu_list` | `AppEv::Feature` (one, generic) |
| `Picked::Goal(..)` and the arm in `core/src/app/select.rs` | `type Pick = goal::Pick`; `fn picked(state, pick, cx)` acts | `Picked::Feature` |
| `PromptKind::GoalEdit` + `Prompt::goal_edit` + the arm in `session.rs` | `type Prompt = goal::Prompt` (`Edit { .. }`); `cx.prompt(prompt, initial)` opens, `fn prompt_submitted` takes the text; the dialog's title and placeholder are `FeatureView::prompt_words` in `src/features/goal/strings.rs` | `PromptKind::Feature` |
| `NoteKey::Goal(..)` + the tone arm in `threads/note.rs` + `goal_note` in `pages/session/strings.rs` | `type Note = goal::Note`; `fn note_tone(note) -> Tone`; the words are `src/features/goal/strings.rs::note` (`FeatureView::note_words`) | `NoteKey::Feature` |
| a call in `core/src/app.rs` after a resume (`session::goal::after_resume`) | `fn on_resumed(state, thread, cx)` | `features::resumed` (already there) |
| a step of the hub that waited for the feature's answer | the feature's `answered(state, ctx, result, cx)`; an answer for a thread that is no longer the one in use is dropped there | the generic `features::answer` |
| a host ask: `Intent::Effect { request, ask }` or `fx.commands.push(effects::command(request, Ask::X))`, and the arm that words `Done::X` (`session::export::settled`, `Done::Export` / `Done::Copy`) | `cx.effect(Ask::X { .. }, ctx)` with the feature's own `Ctx` (what the ask was for); `fn effect_done(state, ctx, done, cx)` words the answer (`cx.note(..)`). A copy answers twice: `Done::Copy(Ok(Pending(_)))`, then how it ended (`done.is_final()` tells which). The request number is the app's (`FEATURE_REQUEST_BIT \| n`), never a stamp the feature chooses | `effects::Ask` / `Done` (the host's list of capabilities: copy, write a file, suggest a title; a new capability is a new variant there, counted in the budget) and the one `AppEv::Effect` arm that routes a feature's request |

Every type in the right column is the feature's own (two features sharing `String` as a note
fail to compile: the root enums' `From` impls would overlap). The part of the table a feature
does not have stays `Never<Self>`.

State that is the feature's alone goes in `type State` (app-scoped, a field of `FeatureStates`)
or `type ThreadPart` (per thread, `FeatureThreadParts`), both `Default + Clone + Eq`.
Facts of the thread that other hub code reads (the goal itself, `threads::Goal`, the status
line reads it) are not a feature's part: they stay in `ThreadCore`.

## 4. The hub lines to remove (step 5 of the codemod's `hub`)

Remove, in this order (the compiler leads): the variant, its arms (`choose`, `wire`,
`decode`, `page`, `tone`, `picked`, the prompt's submit arm), the payload types that only the
variant used, the re-exports in `threads.rs` / `screens/select.rs`, the strings of the feature
in `pages/session/strings.rs` and its `NoteKey` arm. Then
`slice-codemod.py budget` lowers `tests/rules/hub_budget.txt` to what the code has. A
number that did not go down is a variant still there. `hub_enums_only_shrink` also fails
when a count *fell* without the file being lowered, which is the point: the ratchet is only
ever tightened.

## 5. The seams every slice meets

- `Command::Feature { ticket, thread, command }`: an RPC that is about a thread must be seen
  as a thread's by the host (`host/src/in_process.rs` `asks_thread` matches it) and by
  `backend_of` (`core/src/app.rs`: `Command::Feature { thread, .. } if !thread.as_str()
  .is_empty()` goes to `agents.owner_of`; an empty thread is the app's). The fake backend
  (`src/services/backend/fake.rs`) lists it with the other `Command::Screen` arms. These are
  written already; a feature that adds a new `Command` kind meets them again.
- The notes: `cx.note(thread_or_none, note)` is carried by `session_note`, which writes the
  transcript of the thread (or the chat before any thread).
- A change of a thread's feature part from the app's side is `cx.thread(&id, intent)`: a
  queued `ThreadEv::FeatureIntent`, applied by `threads::apply` under the thread's writer.
- **The host door.** A feature reaches the server through `wire` (an RPC about a thread or the
  app, `cx.ask` / `ThreadCx::request`) and the host through `cx.effect(ask, ctx)` (`AppCx` only: a
  host effect is the app's, not a thread's). `app/features.rs` carries `Out::Effect`: the app
  mints the request number (`Effects::ask_for_feature`), keeps `ctx` against it and pushes the
  same `Command::Effect { request, ask }` a screen's `Intent::Effect` makes, so the host and
  `backend_of` see no difference; the answer (`AppEv::Effect`) is dispatched to the feature's
  `effect_done` and kept for no screen; the last answer ends the ask, a late one is dropped.
  What a feature reads to build the ask (the text to export, the thread's directory) comes
  from `AppView` / `ThreadView` methods the feature adds, each a decision. A slice never names
  `AppEv`, `Intent::Effect` or `Command::Effect` (`feature_slices_ask_only_through_the_outbox`).
- **A feature that reacts to another feature's end** (the goal was set, so the queue goes
  on) never names the other. `cx.settled(&thread)` says "what held this thread's input back
  is over"; every feature's `on_settled(part, cx)` runs, in list order (the queue resumes).
  This is the one hook step 1 added to the framework; the goal-to-queue nudge was
  `QueueResume` before. A feature that needs to nudge by another meaning adds its own
  `Out` and hook the same way (`cx.rs`, `framework.rs` probe and test, `app/features.rs`
  carrying it), one decision each.

## 6. The tests

`core/tests/<f>.rs` stays as it is, through `CodexApp`; only the helpers that matched the
removed variants change, once, in the helper functions at the top of the file:

```rust
// an RPC of the feature, as the host sees it
Command::Feature { ticket, thread, command: FeatureCommand::Goal(ask) } => ...
// the answer the host would send back
AppEv::Reply(ticket, Reply::feature(FeatureReply::Goal(reply)))
// the note the transcript got
NoteKey::Feature(FeatureNote::Goal(note))
// a menu, a prompt
Menu::Feature(FeatureMenu::Goal(goal::Menu::Replace { .. }))
PromptKind::Feature(FeaturePrompt::Goal(goal::Prompt::Edit { .. }))
```

A host ask is tested the same way: the helper that matched `Command::Effect { ask: Ask::X, .. }`
with the UI's own request number takes the request from the command and answers it with
`AppEv::Effect { request, done }`, as before; the note it ends in is `NoteKey::Feature(..)`.
The framework's own test of the door is `core/src/app/features/tests.rs` and
`framework.rs` (`a_host_answer_reaches_the_feature_that_asked`).

A wire test that lived in `core/tests/app_wiring.rs` for the feature's RPCs moves into the
feature's file (the `ClientRequest` it builds is the slice's `wire`). Other test files that
filtered the feature's asks (`core/tests/agents.rs`) match `AppCommand::Feature { command:
FeatureCommand::Goal(_), .. }`.

## 7. What the pilot found the framework lacked

1. `AppCx::settled` / `FeatureThreadIntent::Settled` / `Feature::on_settled`: the goal set
   resumes the queue without goal naming queue.
2. `Never::absurd()`: the empty match of a missing component, in one word.
3. `Command::Feature` in the host's thread routing and in `backend_of` (listed above).
4. The `#![allow(unreachable_code, unused_variables)]` header of `cx.rs` was a step-0 stand-in
   for the empty list; it is gone, a real feature uses what it declares.
5. (early review F1) A cross-feature nudge that used to name both must still do its job:
   `Out::Settled` also resumes the queue (`app/features.rs`); the regression test is
   `core/tests/goal.rs` `a_goal_set_lets_the_queue_go_on`. Move a nudge, then test the
   end-to-end behaviour the oracle has (`maybe_send_next_queued_input` after the goal is set).
6. (F2) The tickets of a step are the model's only after the whole `apply`: `ThreadView::busy`
   counts a turn start said before the hooks (`ThreadFx::start_said`); `ThreadView` no longer
   exposes `core` / `slots` (the raw `core.busy()` is stale mid-step).
7. (R1, changed by K1) `ThreadCx::new` was none unless this process writes the thread. It is now
   always one, and only the effects are the writer's (`ThreadCx::request` is `None` without it,
   `thread_intent` is not delivered): see item 14.
8. (R3) `FeatureView::prompt_words` (title, placeholder) for the prompt dialog; the codemod's
   view skeleton has it.
9. (R4/R5) `Out`, the non-feature variants of `FeatureThreadIntent` and the `Feature` trait's
   methods are ratchets in `hub_budget.txt`; a hook is for a thread or app lifecycle fact, never
   for one producer and one consumer (ADR-012 "What is not a slice").
10. The queue is not a slice (option B of the early review): thread lifecycle state stays in
   `core/src/threads/`; its five intents moved out of `threads::apply_ev` into
   `threads::queue::apply` (it gives back, as `Err`, an intent that is not the queue's).

11. (R2) `AppCx::effect(ask, ctx)` and `Feature::effect_done(state, ctx, done, cx)` (step 5,
   "The host door"): a host effect such as `/export` is `effects::Ask` / `Done`, no
   `ClientRequest`. The hub keeps `Ask` / `Done` as the host's capabilities; the filler moves
   the call site and the arm that words the answer. `slice-codemod.py hub` lists the `Ask::X` /
   `Done::X` sites of the feature, and the variants in `effects.rs` it marks as staying. The
   hook was added for an app lifecycle fact (the host answered), raising `Out` to 7 and the
   `Feature` trait to 15 in `hub_budget.txt` (ADR-012).

12. (M1) `Feature::on_fact(state, fact, cx)` and `type Fact`, through `feature_list!`
    (`FeatureFact`, `AppEv::Feature`): the door for what the host sends unasked. Probe test:
    `framework.rs` `a_host_fact_reaches_its_feature_and_only_its_feature`. Raised `AppEv` to 18 and
    `Feature` to 16 in `hub_budget.txt`; `AppEv::Presets` and `AppEv::PluginConfig` go down when
    /presets and /plugins move (the codemod's `hub` lists them).
13. (M2) A live menu: `FeatureView::menu_list(menu, &State, words)`. The app provides
    `FeatureStates` to the views (`src/app/view.rs`, `src/app/debug/seeded.rs`), the select dialog
    watches it, and the picker keeps its cursor, tab and query across a change of the state
    (its key is the menu's depth). Test: `tests/pages/feature_menu_live.rs` (the menu of the real
    list that exists, `/goal`'s `Replace goal?`, rebuilt by a step of a feature keeps its cursor);
    the first slice whose rows come from its state adds the row-level test beside it.
14. (K1) A reply or a lifecycle fact reaches a feature's part even when the thread lost its
    writer (a provider refusal, a lost connection, a foreign writer); only the effects are shut.
    A feature that waits for a reply (a spinner, a pending flag) is never stuck. Test:
    `framework.rs` `a_thread_this_process_does_not_write_hears_the_news_and_has_no_effects`.
15. (K2) `ThreadFx::start_turn` returns `Result<Ticket, UserMessage>`: a second start in one
    step is refused and the message comes back (`a_second_turn_start_in_one_step_is_refused`).
    The thread's own callers keep the message (the queue, `rejected_steers`).
16. (K3) `ClientRequest::ReviewStart` / `ReviewStartParams` are lifecycle RPCs
    (`LIFECYCLE_RPCS`): a slice cannot build `review/start`.

17. (migrate, /rename) **A prompt that awaits a generated suggestion.** `AppCx::prompt_suggesting(
    prompt, initial)` opens the prompt with its loading line; the feature asks the host as always
    (`cx.effect(Ask::SuggestTitle { .. }, ctx)`) and, in `effect_done`, `cx.suggest(prompt, title)`
    (`Out::Suggest`). The app fills the field only if that prompt is still the one open, still
    waiting and the user has not typed: the dialog's own model knows what was typed
    (`dialogs::Intent::Typed`), so the feature never hears a keystroke and never updates an open
    dialog itself. Test: `core/src/app/features/tests.rs`
    `a_suggestion_fills_the_prompt_it_is_for_once_and_never_over_the_users_typing`. Raised `Out`
    to 8 in `hub_budget.txt`.
18. (migrate, /rename) **A command that asks something of the thread it starts** (`/new <name>`).
    `AppCtx::NewConversation` carries `then: Option<FeatureThreadIntent>`, built by the slash
    router (`SLASH_ROUTERS` may now name `features::<f>::ThreadIntent` too, nothing else), and the
    app applies it to the new thread once it is attached: the generic route for a feature's thread
    intent, no hook. A feature's `thread_intent` is the writer's, so it runs after the attach.
19. (migrate, /rename) **A feature RPC on a thread that fails is the thread's last failure.**
    `threads::feature_reply` sets `last_error` from the `Err` before the feature hears it, as a
    turn start's failure does, so a feature that has nothing more to say writes no error line of
    its own (the activity row reads `last_error` first). A feature that words the failure as a
    note (`/goal`) is untouched: it uses `AppCx::ask`, not a thread RPC.
20. `AppView::screens()` reads the facts a feature decides by (the side-conversation flag,
    `chat.flags.side_conversation_active`); `ThreadView::name()` the thread's name.
21. (migrate, /export) **The app's renderers and directory are facts a feature reads.**
    `AppView::transcript_text(thread)` (the UI's renderer of the conversation as Markdown, handed
    to the app once with `CodexApp::with_transcript_text`; `core/src/app/transcript_text.rs`) and
    `AppView::working_directory(thread)` (`config.cwd`: the thread's own, else the status line's;
    also read by `/cd`). The app keeps one renderer and a feature never names `ThreadModel`.
    `AppView::new` takes the renderer as its last argument; `app/features.rs` `view(app)` builds
    the read-only view for the app's own code.
22. (migrate, /export) **An ask of the host answers once or twice, and the ask ends with the
    last answer.** A copy says `Pending` and then how it ended: `effect_done` hears both (the
    export notes both), a third answer for the same request is dropped (`Done::is_final`,
    `effects::feature_answered`). A feature keeps nothing to tell a pending copy from a
    final one: `Done` says it.

## 8. A feature that is half a slice: the menu half and the thread half

Some features write `ThreadModel` fields or sit in `threads::apply_ev` (`/review`,
`/permissions <profile>`, collaboration modes). **Stop and split before moving**: the menu half
moves, the thread half stays in `core/src/threads/` as thread lifecycle state (ADR-012 "What
is not a slice", "A feature that is half a slice"). Never invent a `ThreadCx` write to a
`ThreadCore` field to make the thread half fit; if the slice needs the thread to do something,
it says it (`cx.thread(&id, intent)`, a `ThreadIntent` the thread half applies) or the app
asks the thread as it did.

Sort every line `slice-codemod.py hub <f>` prints into one of the two halves:

| half | what goes in it | /review | /permissions args | collaboration modes |
|---|---|---|---|---|
| slice | the menu and its rows, its picks, what a pick says in the transcript, the prompt, the host facts and effects it reads | `Menu::Review` (presets, branches, commits, instructions), `Ask::GitBranches` / `GitLog`, `Picked::Review(ReviewTarget)` | the profile menu and the arg parsing | the collaboration menu and its pick |
| thread (stays) | a field of `ThreadModel` / `ThreadCore` or `slots.chat`, a turn start, a lifecycle RPC, an interlock with the queue, the steers or `provider_error` | `ThreadAsk::Review` (`review/start`: starts a turn and switches review mode, a lifecycle RPC), `review_mode()`, the review turn's transcript handling | `HoldQueue` / `queue_held` (the queue's) | `SetMask`, `CycleMode`, `switch_mode`, `ThreadFx::modes` (turn settings) |

The seam: a pick is the slice's `Pick`; `picked(state, pick, cx)` ends in one thing the thread
half already has as an event (`cx.thread(..)` with a `ThreadIntent` the thread half applies, or
the app's existing `Command` for it). **Provider error is not a slice at all**: `provider_error`
is in `has_writer` and `send_next`, so it is thread state; leave it out of a migration list.
Budget lines: only the menu half's variants go down in `hub_budget.txt`; the thread half's
`ThreadAsk` / `ThreadIntent` arms stay counted.

## 9. Done when

`hub` lists no line of the feature outside `core/src/features/<f>`, `src/features/<f>` and
the one list line (the `Ask` / `Done` variants it marks as host capabilities stay); the oracle shard of the feature and the rules are green; `hub_budget.txt`
is lower by what moved; `slice-codemod.py hub <f>` prints only the oracle's `SessionIntent`
and `SlashCommand` rows and the host capabilities it marks.
