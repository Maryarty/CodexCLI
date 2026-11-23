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
| `Menu::Goal(..)`, its `choose` arm, its `page()` arm | `type Menu = goal::Menu`, `choose(menu, row) -> Choice::{Stay, Dismiss, Pick, Replace}`; the rows are `FeatureView::menu_list` in `src/features/goal/strings.rs` | `Menu::Feature` |
| `Picked::Goal(..)` and the arm in `core/src/app/select.rs` | `type Pick = goal::Pick`; `fn picked(state, pick, cx)` acts | `Picked::Feature` |
| `PromptKind::GoalEdit` + `Prompt::goal_edit` + the arm in `session.rs` | `type Prompt = goal::Prompt` (`Edit { .. }`); `cx.prompt(prompt, initial)` opens, `fn prompt_submitted` takes the text | `PromptKind::Feature` |
| `NoteKey::Goal(..)` + the tone arm in `threads/note.rs` + `goal_note` in `pages/session/strings.rs` | `type Note = goal::Note`; `fn note_tone(note) -> Tone`; the words are `src/features/goal/strings.rs::note` (`FeatureView::note_words`) | `NoteKey::Feature` |
| a call in `core/src/app.rs` after a resume (`session::goal::after_resume`) | `fn on_resumed(state, thread, cx)` | `features::resumed` (already there) |
| a step of the hub that waited for the feature's answer | the feature's `answered(state, ctx, result, cx)`; an answer for a thread that is no longer the one in use is dropped there | the generic `features::answer` |

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

## 8. Done when

`hub` lists no line of the feature outside `core/src/features/<f>`, `src/features/<f>` and
the one list line; the oracle shard of the feature and the rules are green; `hub_budget.txt`
is lower by what moved; `slice-codemod.py hub <f>` prints only the oracle's `SessionIntent`
and `SlashCommand` rows.
