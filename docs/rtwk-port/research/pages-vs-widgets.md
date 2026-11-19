# Pages vs widgets: classification of the screen map (tm `pages-vs-widgets`, 2026-11-19)

The person: "there are not many pages here; distinguish, as in Flutter, what is
a page and what is a widget." Method: for each entry of architecture.md §1 the
oracle (codex-rs/tui, rust-v0.170.0) was grepped for how it is shown: its own
`run_*` loop or an `Overlay` on the alt screen (a full-screen thing), a
`BottomPaneView` shown by `show_bottom_pane_view` (in place of the composer,
transcript tail visible), a history cell, or a field/pane of `ChatWidget`.
"Unverified" marks entries where only the file list was read.

## Classes and their Flutter analogue

| class | meaning | Flutter analogue |
|---|---|---|
| Page | full screen, own layout, pushed on the `Navigator`; the oracle gives it its own loop or alt-screen overlay | a `Route` with a `Scaffold` (`MaterialPageRoute`) |
| Mode | a state of ONE page/widget, switched by that widget's controller; same widget tree, other behaviour | `TextField` with a `FocusNode`/edit mode; `ListView` in select mode; `Scaffold` with a selection `AppBar` |
| Slot view | a panel in the chat page's bottom slot, drawn in place of the composer, driven by the core view stack (ADR-002 amendment 2026-11-18) | `Scaffold.bottomSheet` / `persistentFooterButtons` content swapped by state; not a route |
| Dialog | centered, with a barrier (`show_dialog`); only where the oracle draws it centered | `showDialog` / `AlertDialog` |
| Widget | part of a page: no route, no overlay | a `StatefulWidget` composed into a page |

Rule (added to §1): **a mode of one widget is driven by its controller, never a
second copy.** The pager's transcript content and the chat transcript are one
transcript widget configuration; backtrack is a mode of the pager; `/diff` is a
content variant of the pager; none of them is a parallel screen with its own
copy of the items, the scroll offset or the selection (the analogue of one
`ScrollController` / `TextEditingController` shared, not two lists).

## Table

| id | entry | oracle shows it as | tui-rtwk now | target class | what changes |
|---|---|---|---|---|---|
| S01 | update prompt | own `run_update_prompt_if_needed` loop before main (update_prompt.rs) | startup | Page (startup flow) | none |
| S02 | onboarding welcome/auth | own `run_onboarding_app` loop (onboarding/onboarding_screen.rs) | screens/onboarding | Page (one route per step, or one page with a step controller: a Mode) | one page + step enum if the oracle draws them in one screen (routes-lists to decide) |
| S03 | onboarding provider/gateway/Bedrock/GCP | same loop | screens/onboarding | Page (steps of S02) | same |
| S04 | directory trust | same loop | screens/onboarding | Page (step of S02) | same |
| S05 | startup prompts: cwd, model migration, OSS, unarchive | own `run_*` loops (cwd_prompt.rs:113, model_migration.rs:142 with alt screen, oss_selection.rs:323, unarchive_prompt.rs:38) | screens/startup | Page (startup flow) | the map said "dialogs": they are pages before the home route |
| S05 | startup hooks review, daemon recovery, startup error | unverified (own files, no `run_*` found) | screens/startup | Page or Slot view: check in the owning task | verify |
| S06 | `/import` | own loops `run_external_agent_config_migration_prompt` / `_source_prompt` | screens/import | Page | none |
| S10 | main chat | the home view: `ChatWidget` + `BottomPane` | screens/chat, composer | Page (the only home page; Scaffold = Column(transcript, bottom slot)) | the chat page owns the slot, the transcript, the sidebar |
| S11 | history cells, streaming, markdown, diff | `HistoryCell`s in the transcript | screens/cells | Widget | none |
| S12 | composer popups (`/`, `@`, `$`, history search) | `CommandPopup` etc. inside the composer | screens/popovers | Widget (popover of the composer) | none |
| S13 | approvals, elicitation, request-user-input | `ApprovalOverlay` as bottom-pane view | screens/approvals | Slot view | none (already a slot view) |
| S14 | generic selection | `ListSelectionView` bottom-pane view (some `Centered`) | screens/select | Slot view (Dialog only for `Centered`) | none |
| S15 | settings: `/config`, status line, theme, keymap, ... | bottom-pane views (config_panel/, theme_picker.rs builds a `SelectionViewParams`) | screens/settings | Slot view | none |
| S16 | transcript pager (Ctrl+T), static pager (`/diff`) | `Overlay::Transcript` / `Overlay::Static` (pager_overlay.rs:44), alt screen over chat | screens/pager (one `PageRoute`), screens/transcript | Page, one pager page with two content variants | transcript content must come from the chat's transcript model, not a second copy |
| S17 | backtrack (Esc Esc) | mode of the transcript overlay (app_backtrack.rs "backtrack mode in the transcript overlay") | screens/backtrack (own screen) | Mode of the pager page (S16) | no separate screen; a controller on the pager |
| S18 | resume / fork picker | own alt-screen loop (resume_picker.rs:510, `AltScreenGuard` :753); also from a live session (app/session_picker.rs:47) | screens/resume | Page | none |
| S19 | `/status` card | a history cell added by `ChatWidget` (chatwidget/status_controls.rs:246 `new_status_output...`) | screens/status | Widget (a cell in the transcript) | not a dialog: becomes a cell kind of S11 |
| S20 | `/usage` dashboard | `Overlay::Analytics` (pager_overlay.rs:47) alt screen | screens/usage | Page (same alt-screen overlay family as the pager) | none |
| S21 | agents command center | `AgentsOverviewView` is a `BottomPaneView` (app/agents_overview.rs:137), `desired_height` 24 (app/agent_center/render.rs:88), transcript tail visible | screens/agents, pushed as a route (chat/open.rs:41 `Target::Agents`) | Slot view (24 rows) | NOT a page in the oracle; VS1 built it as a route (full screen): rows differ from the oracle by layout (view stack entry instead of `push`); change in the owning task |
| S21 | subagent switcher | multi_agents.rs: switching the active thread | agents | Mode of the chat page (active thread) | none |
| S22 | sidebar, side conversation panes | panes of `ChatWidget` (app/sidebar.rs `SidebarPane`, side_pane.rs) | screens/sidebar | Widget (pane of the chat page) | none |
| S23 | shortcuts, warnings, daemon menu, app link, custom prompt, user verification, feedback (+note, consent) | bottom_pane/* views | screens/dialogs | Slot view | none |
| S24 | pets: ambient sprite; picker | `render_ambient_pet_image` inside chat (pets/mod.rs:129); picker via the selection view | screens/pets | Widget (ambient) + Slot view (picker) | split the one "overlay" |
| S25 | voice strip, realtime settings | `voice_strip.rs` part of chat; settings views | screens/voice | Widget (strip) + Slot view (settings) | none |
| S26 | ui_tree elements, inline visualization | footer elements, a cell kind | screens/ui_tree | Widget | none |
| S27 | inline mode | screen mode | ScreenMode | Mode of the app (not a page) | none |

Pages that remain: **home (chat)**, **pager** (transcript, static, usage as
content variants or sibling alt-screen pages), **resume picker**, **onboarding**,
**startup/pre-chat prompts**, **import**. Everything else is a slot view,
widget, mode or a true dialog. The map's 23+ "screens" are ~6 pages.

## Rationale per class

- Page: the oracle gives it its own event loop or alt-screen overlay, so the
  user leaves the chat; in Flutter that is a `Route` with a `Scaffold`. Tests:
  one page route per entry, state through a controller, ids in the route.
- Mode: a second behaviour of the same content (select, search, backtrack) is
  one widget with a controller; a copy of the widget would duplicate scroll and
  selection state (Flutter: one `ScrollController`, `ListView` in select mode).
- Slot view: the chat stays visible above; a route would hide it and change
  the rows. Flutter has no better analogue than a bottom area swapped by state.
- Dialog: only what the oracle draws centered over the chat.
- Widget: reusable inside any page; nothing pushes it.

## Follow-up moves per VS2 task (scopes assumed from the task names; confirm against each brief)

- session-core: the chat page owns the slot (core view stack), the active-thread
  mode (subagent switcher) and the transcript model shared with the pager.
- ui-pane: S19 `/status` as a cell kind of the transcript (not a dialog); S26
  and S25 strips as footer widgets; no routes in this pane.
- chat-nav-pane: pager as one page over the chat's transcript model; backtrack
  (S17) folded into the pager as a mode (its controller, no second list);
  `screens/transcript` is the content widget the chat page and pager both use;
  the sidebar (S22) stays a widget of the chat page.
- dialogs-pane: S13, S14, S15, S23, S24 picker, S25 settings are slot views on
  the view stack; `show_dialog` only for the oracle's `Centered` views.
- (OVERRULED 2026-11-19 by the person: S21 stays a PAGE.) chat-live: S21 agents command center becomes a slot view of 24 rows (the
  oracle's), with its tabs, list and details inside; revisit the route built
  in VS1 (`Target::Agents`).
- routes-lists: pages: resume picker, usage, import, onboarding (steps),
  startup prompts (verify the three unverified ones); onboarding as one page
  with a step controller if the oracle draws all steps in one screen.
