"""Packages of the full port map: owned files, rows' owners, sizes.

Paths are relative to codex-rs/tui-rtwk.  A pattern is a file, `dir/**` (everything under dir,
existing and new) or `rest:dir` (every file now under dir that no other package names).
`rest:` entries with the longest dir resolve first; the fallbacks are `rest:core/src` and
`rest:host/src` of fw-core-hubs and `rest:src` of fw-ui-registry, so every production file has one owner.
"""

# id -> dict(title, wave, runs, lines, core, ui, oracle, first, deps, tm, own)
P = {}


def pkg(id, title, wave, runs, lines, own, oracle="", core="", ui="", first="", deps="", tm="", brief=""):
    assert id not in P, id
    P[id] = dict(id=id, title=title, wave=wave, runs=runs, lines=lines, own=own, oracle=oracle,
                 core=core, ui=ui, first=first, deps=deps, tm=tm, brief=brief)


# ---------------------------------------------------------------- wave 1: landed (core halves), dc3417d92
# Their files now belong to the W2 package whose UI uses them (own-lists are empty: a landed slice owns nothing).
W1 = "W1 (влита)"
pkg("fw1-c1", "core: /diff-документ pager, /presets, /daemon, /subagents, app link, provider switch", W1, "1", "≈750",
    [],
    oracle="get_git_diff.rs, app/quick_dials.rs, chatwidget/quick_dials.rs, app/daemon_menu.rs, app/multi_agents.rs",
    core="вся (P1+P3 core)", tm="fw1-c1")
pkg("fw1-c2", "core: внешний редактор Ctrl+G, очередь команд, состояния composer, /side /btw", W1, "1", "≈1150",
    [],
    oracle="external_editor.rs, chatwidget/input_queue.rs, session_queue_commands.rs, app/side.rs",
    core="вся (P2+P13 core)", tm="fw1-c2")
pkg("fw1-c3", "core: /experimental /skills /hooks, /config /tui /keymap, аргументные формы (settings-слайс)", W1, "1-2", "≈1500",
    [],
    oracle="experimental_features.rs, hooks_rpc.rs, config_panel/, keymap_setup/, app/thread_title.rs, status_surface_commands.rs",
    core="вся (P4+P5+P15 core)", tm="fw1-c3")
pkg("fw1-c4", "core: /copy и Ctrl+O, /logout /ide /daybreak /recap /debug-config /debug-m-*", W1, "1", "≈950",
    [],
    oracle="chatwidget/copy_picker.rs, markdown_copy.rs, app/recap.rs, debug_config.rs, daybreak.rs, ide_context.rs",
    core="вся (P6+P14 core)", tm="fw1-c4")
pkg("fw1-c5", "core: exploration-группы, фоновые терминалы и /ps, инвентарь /mcp", W1, "1", "≈900",
    [],
    oracle="history_cell/exec.rs, bottom_pane/unified_exec_footer.rs, history_cell/mcp.rs, app/background_requests.rs",
    core="вся (P9 core)", tm="fw1-c5")
pkg("fw1-c6", "core: safety buffering, misalignment, guardian, compaction", W1, "1", "≈800",
    [],
    oracle="chatwidget/safety_buffering.rs, app/safety_buffering.rs, misalignment_policy.rs, chatwidget/compaction.rs",
    core="вся (P10 core)", tm="fw1-c6")
pkg("fw1-c7", "core: сброс лимита /usage, billing, /status thread usage и строка сервера, предупреждения лимитов", W1, "1", "≈900",
    [],
    oracle="chatwidget/usage.rs, reset_credits.rs, chatwidget/thread_usage.rs, rate_limits.rs, usage_notice.rs",
    core="вся (P11+P12 core)", tm="fw1-c7")
pkg("fw1-c8", "core: worktree/checkout для /new /fork /worktree, «Forked thread», Windows sandbox, security setup", W1, "1", "≈1000",
    [],
    oracle="chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, windows_sandbox_prompts.rs, security_setup.rs",
    core="вся (P16+P18 core)", tm="fw1-c8")
pkg("fw1-c9", "core: /feedback (note, upload), решение approval, вопросы (черновики, очередь)", W1, "1", "≈700",
    [],
    oracle="bottom_pane/feedback_view.rs, feedback_note_view.rs, app/feedback_upload.rs, history_cell/approvals.rs, request_user_input/",
    core="вся (P17+P19 core)", tm="fw1-c9")

# ---------------------------------------------------------------- wave 2: hubs first (registry codegen)
W2 = "W2"
pkg("fw-core-hubs", "framework: core-хабы — сжатие (`SessionIntent::Feature`, конец арма NotAvailable) и одна дверь возможностей хоста (`effects::Ask`); с 2026-11-24 = tm fw-hubs-a/b/c + fix-recap-title, fix-stream-lost (arch-pillars P1-P9)", W2, "3 + 2", "≈950",
    ["core/src/app/session.rs", "core/src/commands/dispatch.rs", "core/src/commands.rs",
     "core/src/threads.rs", "core/src/threads/cells.rs", "core/src/app.rs", "tests/rules/hub_budget.txt",
     "core/src/app/effects.rs", "host/src/effects.rs", "host/src/in_process.rs", "host/src/facts.rs",
     "src/services/effects.rs",
     "tests/rules/layers.rs", "rest:core/src", "rest:host/src", "rest:tests/rules"],
    oracle="app/event_dispatch.rs, chatwidget/slash_dispatch.rs, slash_command.rs, app_event.rs",
    core="ADR-012: пакет фичи НЕ владеет хабом и не пишет в него свои строки; фича входит в приложение только через двери слайса: `feature_list!`/`target_of` (слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможности хоста), `ThreadModel.features` (состояние треда). Правка хаба бывает только СЖАТИЕМ — это и есть вся работа этого пакета. "
         "Шаг 1: общий вариант `Feature` в `SessionIntent` (то самое «Need of the ADR-012 owner» в `tests/rules/hub_budget.txt`: «слэш-список не имеет generic-варианта») — после него слэш-команда нового слайса не добавляет варианта; бюджет `SessionIntent 43` фиксируется. "
         "Шаг 2 (после слияния fw1-c1..c9): удалить остаток арма `S::Recap | S::Side | … => NotAvailable` (`app/session.rs:361-386`), `Notice::NotAvailable`/`Stub`; команды, что остались без владельца (`/model <имя>`, `/permissions` без аргумента, `/review`, `/resume <имя>`, `/apps`), переехали в слайсы пакетов w2-approvals и resume-rest — арм сжимается до пустого и исчезает, бюджет в `hub_budget.txt` опускается на столько же. "
         "Шаг 3: `threads.rs`/`cells.rs` — только вычёркивание вариантов `ThreadEv`/`ThreadAsk`, ушедших в `ThreadModel.features` (счётчики в `hub_budget.txt` падают; рост запрещён правилом). Остальное в `core/src` и `host/src`, чего никто не назвал, — тоже у этого пакета: фичи сюда не пишут. "
         "Шаг 4 (дверь возможностей хоста): `AppCx::effect` с `effects::Ask` — единственная дверь для возможностей хоста, у неё один enum на всех; поэтому пакет один раз добавляет заранее названные варианты `Audio` (voice), `Suspend` (Ctrl+Z), `Notify` (OSC 9 / BEL), `ScreenMode` (alt-screen для inline), `Probe` (цвета терминала), `SessionLock` с `Done`-ответами, строками в `hub_budget.txt` и пустыми обработчиками хоста. Причина одного владельца (и единого пакета с шагами 1-3: у них общий `hub_budget.txt`): пакеты voice, terminal-features, inline-mode идут параллельно и иначе правили бы один enum; дальше каждый пишет только свой файл хоста (`host/src/audio.rs` и т. п.) и слайс. Редактор Ctrl+G (`Ask::Editor`) делает fw1-c2.",
    deps="после fw1-c1..c9, fw1-c2 (Ask::Editor)")
pkg("fw-ui-registry", "framework: сжатие UI-хабов — реестры действий, целей, ячеек и режимов; session_page.rs по заботам", W2, "1-2", "≈1000",
    ["src/pages/session/session_page.rs", "src/pages/session/keys.rs", "src/pages/session/open.rs",
     "src/pages/session/actions.rs", "src/pages/session/mod.rs", "src/pages/session/slice.rs",
     "src/pages/session/popup.rs", "src/pages/session/strings.rs",
     "src/pages/session/cells/dispatch.rs", "src/pages/session/cells/notice.rs",
     "src/pages/mod.rs", "src/sheets/mod.rs", "src/sheets/presenter.rs",
     "src/app/presenters/mod.rs", "src/app/debug/**", "src/l10n/**", "src/keymap/**",
     "tests/pages.rs", "tests/screens.rs", "tests/oracle.rs", "tests/rules.rs", "rest:src"],
    oracle="chatwidget/slash_dispatch.rs, app/event_dispatch.rs, app/input.rs",
    ui="Тот же принцип, что у fw-core-hubs: UI-хабы — только сжатие. Где можно, хаб заменяется реестром (макрос по списку, как `feature_list!`/`screen_list!`), где нельзя — остаётся один владелец с причиной. "
       "Реестр 1, действия клавиш: `Action` в `keys.rs` и его диспатч собираются списком `action_list!` из `key_actions/<имя>.rs` (`find`, `copy`, `external_editor`, `side`, `voice`, `focus_activity`, `clear_terminal`); файл действия — привязка и `run(cx, state)`. "
       "Реестр 2, цели open: `open.rs` держит общий путь `Target::Feature` (он уже есть, `open.rs:75`) и сжимается: `pager`, `daemon`, `agent_picker`, `voice` становятся слайсами и уходят из арма. "
       "Реестр 3, ячейки и режимы: `cells/dispatch.rs`, `cells/mod.rs`, `transcript/mod.rs` собирают ячейки (`exploration`, `processes`, `mcp_inventory`, `compaction`, `misalignment`, `decision`, `forked`, `recap`) и режимы (`find`, `select_mode`, `copy_mode`, `diff_view`) списками `cell_list!`/`mode_list!` по файлам. "
       "Реестр 4, тесты: `tests/pages.rs`, `screens.rs`, `oracle.rs` получают `mod`-строки из каталога (`build.rs` пишет `mods.rs` по `tests/pages/*.rs`) — новый тестовый файл не правит корень. "
       "Без реестра, один владелец: `session_page.rs` (1218 строк, один `Component`; причина — его состояние общее, дробить нечем, кроме выноса забот в `session/*.rs`; пакет выносит и дальше правит только он) и `actions.rs` (сжатие: снять `not_yet`, `presets_action`, `daemon_action`, `action_subagents`). Двери `core/src/features/mod.rs` и `src/features/mod.rs` — одна строка слайса, их не владеют (см. «Двери»).",
    first="идёт первым в W2: пока реестры не влиты, UI-пакеты ниже стоят (иначе правят один файл)",
    deps="после fw1-c1..c9 (их открытые sheet)")

# ---------------------------------------------------------------- wave 2: UI packages (8 merged, areas = the tm tasks w2-*)
# Own-lists are the `area` of the tm tasks (briefs: docs/rtwk-port/research/w2-packages/briefs.md §A); a directory is `dir/**`,
# and `dir.rs` beside it joins its package. The 20 packages of the first plan (ui-*, *-sweep) are merged into them.
pkg("w2-transcript", "UI транскрипт: Ctrl+T pager с поиском, select и copy mode, детали хода, /diff, хром транскрипта", W2, "4", "≈3550",
    ["core/src/features/copy/**", "core/src/features/diff/**",
     "src/features/copy/**", "src/features/diff/**",
     "core/src/screens/transcript/**", "core/src/screens/transcript.rs",
     "src/pages/session/activity.rs", "src/pages/session/transcript_activity.rs",
     "src/pages/session/transcript_header.rs", "src/pages/session/transcript_pane.rs",
     "src/pages/session/key_actions/find.rs", "src/pages/session/key_actions/copy.rs",
     "src/pages/session/key_actions/focus_activity.rs", "src/pages/session/open/pager.rs",
     "src/pages/session/transcript/**"],
    oracle="pager_overlay.rs, pager_overlay/{scrolling,transcript}.rs, diff_render.rs, diff_model.rs, transcript_view/{search,search_footer,search_presentation}.rs; transcript_view/{select_mode,copy_mode,selection}.rs, chatwidget/copy_picker.rs, markdown_copy.rs, clipboard_copy.rs; transcript_view/{prompt_header,turn_details,turn_tip,disclosure,activity,follow_control,footer}.rs, turn_tip.rs",
    ui="`StaticPager` с документом из fw1-c1 и live-tail sync; порт `diff_render.rs`; поиск (подсветка, счётчик, n/N, футер поиска) над `ItemTexts`; F3; лист copy picker, Ctrl+O, payload plain/HTML/Markdown, режим выделения с клавиатуры, copy mode (лидер Ctrl+X), двойной/тройной клик; F4 focus activity, pinned header, TurnTip, ширинно-зависимый счёт скрытых строк, mouse_scroll_speed, ctrl+home / alt+<",
    first="rtwk: `ItemTexts` (текст по элементу транскрипта, равный нарисованному) — без него поиск и выделение считаются по исходникам; rtwk: чтение start/end у `SelectionAreaController`, жесты двойного/тройного клика, `ItemTexts`; rtwk: `ListController::visible_range()` (шапка prompt, TurnTip, 50+50 строк вывода)",
    deps="fw-ui-registry, fw1-c1, fw1-c4", tm="w2-transcript", brief="§A.1")
pkg("w2-composer", "UI composer: состояния, очередь, Ctrl+G, мышь, Vim, burst, mentions и popovers", W2, "3", "≈2450",
    ["core/src/features/external_editor/**", "core/src/kit/**",
     "core/src/screens/composer/**", "core/src/screens/composer.rs",
     "core/src/screens/popovers.rs", "core/src/threads/composer.rs",
     "core/src/threads/queue.rs", "src/features/external_editor/**",
     "src/pages/session/composer/**", "src/pages/session/footer/flash.rs",
     "src/pages/session/footer/sparkle.rs", "src/pages/session/key_actions/external_editor.rs",
     "src/pages/session/popovers/**", "src/widgets/composer_draft/**",
     "src/widgets/composer_draft.rs", "src/widgets/composer_sparkle.rs"],
    oracle="external_editor.rs, chatwidget/input_queue.rs, bottom_pane/chat_composer/{mouse,sparkle,agents_navigation,warning_notice}.rs, bottom_pane/chat_composer.rs (состояния); bottom_pane/{chat_composer,textarea,paste_burst,command_popup,file_search_popup,mentions_v2,skill_popup}*, chat_composer/{history_search,edit_history,draft_state,paste_input,attachment_state}.rs, startup_draft*.rs",
    ui="состояния disabled/shutdown/parent-owned, подсказка agents-nav, мышь (курсор, выделение, правый клик), Tab-очередь команд, запуск при drain, Ctrl+G; пересчёт 100 pending composer-шардов; Vim replace/yank/dot и Vim в поле ответа; steer и hidden shell paste; `$`-приложения до /apps; блокировка отправки при недоступной модели; startup draft",
    first="rtwk G67: приостановка терминала на время `$EDITOR` из обработчика (`App::suspend` доступен только host); rtwk G65 (каретка TextArea по метрикам строк), G66 (Ctrl+J), G78 (каретка isearch), G-new-1 (стиль по диапазону), G68 (таблица Autocomplete)",
    deps="fw-ui-registry, fw1-c2", tm="w2-composer", brief="§A.2")
pkg("w2-settings", "UI настройки: /config, /keymap, /experimental /skills /hooks, лист по заботам", W2, "2", "≈2050",
    ["core/src/features/settings/**", "core/src/keymap/**",
     "core/src/keymap.rs", "src/features/settings/**"],
    oracle="bottom_pane/{experimental_features_view,skills_toggle_view,hooks_browser_view,memories_settings_view,status_line_setup,title_setup}.rs, chatwidget/hooks.rs; config_panel/, chatwidget/config_panel.rs, bottom_pane/config_panel.rs, chatwidget/tui_mode_picker.rs, app/tui_mode_picker.rs; keymap_setup/{picker,capture,debug}.rs, chatwidget/keymap_picker.rs, keymap.rs (runtime: chords, leader, conflicts)",
    ui="подключить готовые sheet features/setup/hooks к фактам fw1-c3; split `src/features/settings/mod.rs` на файл по `Which` (заглушки `config_panel.rs`, `tui_mode.rs`, `keymap_editor/`); панель с вкладками и под-пикерами, выбор режима TUI на следующий запуск; редактор клавиш; leader Ctrl+X, зарезервированные клавиши, конфликты",
    first="шаг 1 — расщепить `mod.rs` настроек: без этого панель /config и редактор /keymap правят его вместе",
    deps="fw-ui-registry, fw1-c3", tm="w2-settings", brief="§A.3")
pkg("w2-cells", "UI ячейки: exec, MCP, exploration, /ps, compaction, safety, guardian, ошибки провайдера и остальной cells-sweep", W2, "3", "≈3050",
    ["core/src/features/mcp_inventory/**", "core/src/features/processes/**",
     "src/features/mcp_inventory/**", "src/features/processes/**",
     "core/src/screens/cells/**", "core/src/screens/cells.rs",
     "core/src/threads/activity.rs", "core/src/threads/calls.rs",
     "core/src/threads/citations.rs", "core/src/threads/collab.rs",
     "core/src/threads/compaction.rs", "core/src/threads/daybreak.rs",
     "core/src/threads/decision.rs", "core/src/threads/directives.rs",
     "core/src/threads/exploration.rs", "core/src/threads/followups.rs",
     "core/src/threads/goal.rs", "core/src/threads/local_links.rs",
     "core/src/threads/message.rs", "core/src/threads/note.rs",
     "core/src/threads/previews.rs", "core/src/threads/processes.rs",
     "core/src/threads/replay.rs", "core/src/threads/safety/**",
     "core/src/threads/safety.rs", "core/src/threads/status.rs",
     "core/src/threads/stream_preview.rs", "core/src/threads/user_prompt.rs",
     "src/pages/session/cells/agent.rs", "src/pages/session/cells/citation.rs",
     "src/pages/session/cells/collab.rs", "src/pages/session/cells/computer_activity.rs",
     "src/pages/session/cells/delegated.rs", "src/pages/session/cells/deprecation.rs",
     "src/pages/session/cells/disclosure.rs", "src/pages/session/cells/error.rs",
     "src/pages/session/cells/exec.rs", "src/pages/session/cells/generated_image.rs",
     "src/pages/session/cells/hook.rs", "src/pages/session/cells/interaction.rs",
     "src/pages/session/cells/markdown.rs", "src/pages/session/cells/note.rs",
     "src/pages/session/cells/one_row.rs", "src/pages/session/cells/patch.rs",
     "src/pages/session/cells/plan.rs", "src/pages/session/cells/plan_update.rs",
     "src/pages/session/cells/raw.rs", "src/pages/session/cells/reasoning.rs",
     "src/pages/session/cells/safety.rs", "src/pages/session/cells/separator.rs",
     "src/pages/session/cells/session_info.rs", "src/pages/session/cells/strings.rs",
     "src/pages/session/cells/text.rs", "src/pages/session/cells/tool_call.rs",
     "src/pages/session/cells/tool_rows.rs", "src/pages/session/cells/update_available.rs",
     "src/pages/session/cells/user.rs", "src/pages/session/cells/viewed_image.rs",
     "src/pages/session/cells/warning.rs", "src/pages/session/cells/web.rs",
     "src/pages/session/cells/exploration.rs", "src/pages/session/cells/processes.rs",
     "src/pages/session/cells/compaction.rs", "src/pages/session/cells/misalignment.rs",
     "src/pages/session/cells/mcp_inventory.rs", "src/pages/session/live/provider_error.rs",
     "src/pages/session/status/terminals.rs", "src/sheets/approvals/guardian.rs"],
    oracle="history_cell/exec.rs, exec_cell/, history_cell/mcp.rs, bottom_pane/unified_exec_footer.rs; chatwidget/{safety_buffering,compaction,misalignment_policy,provider_error}.rs, safety_recourse/; history_cell/{base,messages,patches,plans,search,separators,session,notices,hook_cell,delegated_message,dynamic}.rs, exec_cell/",
    ui="группа exploration, ячейка взаимодействия «waited for», список /ps, футер unified exec, ячейка инвентаря MCP; повтор/подтверждение safety, остановка misalignment, ячейки guardian, таймер и ячейка compaction; 210 pending cells_* и renderer_internals; ячейки collab, image, separator, user, plan; resume-replay",
    first="rtwk G72 (автоссылка в Text), G-tool-bullet-pulse, G-exec-user-shell",
    deps="fw-ui-registry, fw1-c5, fw1-c6", tm="w2-cells", brief="§A.4")
pkg("w2-approvals", "UI approvals, request-user-input и все select/dialog-листы; bare /review и /permissions", W2, "3", "≈2200",
    ["core/src/data/files.rs", "core/src/data/history.rs",
     "core/src/data/links.rs", "core/src/data/mention.rs",
     "core/src/data/task.rs", "core/src/features/goal/**",
     "core/src/features/plugins/**", "src/features/goal/**",
     "src/features/plugins/**", "core/src/screens/approvals/**",
     "core/src/screens/approvals.rs", "core/src/screens/dialogs.rs",
     "core/src/screens/select.rs", "core/src/screens/select/apps.rs",
     "core/src/screens/select/collaboration.rs", "core/src/screens/select/models.rs",
     "core/src/screens/select/permission_args.rs", "core/src/screens/select/permission_shortcuts.rs",
     "core/src/screens/select/permissions.rs", "core/src/screens/select/persist.rs",
     "core/src/screens/select/reasoning.rs", "core/src/screens/select/review.rs",
     "src/pages/session/cells/decision.rs", "src/pages/session/cells/request_input.rs",
     "src/sheets/approvals/auto_review.rs", "src/sheets/approvals/banner.rs",
     "src/sheets/approvals/clip.rs", "src/sheets/approvals/decision_cell.rs",
     "src/sheets/approvals/dialog.rs", "src/sheets/approvals/dismiss.rs",
     "src/sheets/approvals/fixtures/**", "src/sheets/approvals/form.rs",
     "src/sheets/approvals/keys.rs", "src/sheets/approvals/present.rs",
     "src/sheets/approvals/strings.rs", "src/sheets/approvals/tips.rs",
     "src/sheets/approvals/user_input.rs", "src/sheets/dialogs/dialog.rs",
     "src/sheets/dialogs/fixtures/**", "src/sheets/dialogs/shortcuts.rs",
     "src/sheets/dialogs/strings.rs", "src/sheets/dialogs/warnings.rs",
     "src/sheets/dialogs/user_verification.rs", "src/sheets/select/apps.rs",
     "src/sheets/select/generic.rs", "src/sheets/select/lists.rs",
     "src/sheets/select/models/**", "src/sheets/select/models.rs",
     "src/sheets/select/prompt.rs", "src/sheets/select/review/**",
     "src/sheets/select/review.rs", "src/sheets/select/strings.rs",
     "src/sheets/select/fixtures/**", "src/widgets/age.rs",
     "src/widgets/directives.rs", "src/widgets/dots.rs",
     "src/widgets/interest.rs", "src/widgets/key_guard.rs",
     "src/widgets/measure.rs", "src/widgets/mod.rs",
     "src/widgets/options.rs", "src/widgets/page_keys.rs",
     "src/widgets/picker/**", "src/widgets/sheet.rs",
     "src/widgets/tab_strip.rs", "src/widgets/thread_ui.rs"],
    oracle="bottom_pane/{approval_overlay,request_user_input,async_questions,mcp_server_elicitation}*, history_cell/{approvals,request_user_input,async_questions}.rs; bottom_pane/{list_selection_view,shortcut_overlay,warnings_view,user_verification,multi_select_picker,custom_prompt_view}.rs, chatwidget/{model_popups,permission_popups,connectors,review_popups}.rs",
    ui="ячейка решения (Approved/Denied, усечение, network-allow), apply_patch и write_stdin, черновики вопросов, пустые ответы, очередь вопросов; small_dialogs_1/2 (58 pending), picker/chat_popups хвосты, refresh каталога моделей, reasoning submenu",
    first="rtwk G8 (сброс typeahead перед approval), G72 (автоссылка в обёрнутом URL); rtwk G44 (halfwidth katakana), G56 (имя+описание одним абзацем)",
    deps="fw-ui-registry, fw1-c9", tm="w2-approvals", brief="§A.5")
pkg("w2-commands", "UI /side, /subagents, daemon, presets, worktree, Windows sandbox, security setup, /feedback и малые команды", W2, "3", "≈2550",
    ["core/src/features/daemon/**", "core/src/features/feedback/**",
     "core/src/features/recap/**", "core/src/features/security_setup/**",
     "core/src/features/side/**", "core/src/features/small_cmds/**",
     "core/src/features/windows_sandbox/**", "core/src/screens/select/agent_picker.rs",
     "core/src/screens/select/quick_dials.rs", "core/src/screens/select/worktree/**",
     "core/src/screens/select/worktree.rs", "src/features/daemon/**",
     "src/features/feedback/**", "src/features/recap/**",
     "src/features/security_setup/**", "src/features/side/**",
     "src/features/small_cmds/**", "src/features/windows_sandbox/**",
     "src/pages/session/cells/debug_config.rs", "src/pages/session/cells/forked.rs",
     "src/pages/session/cells/recap.rs", "src/pages/session/key_actions/side.rs",
     "src/pages/session/open/agent_picker.rs", "src/pages/session/open/daemon.rs",
     "src/sheets/dialogs/feedback.rs", "src/sheets/select/agent_picker/**",
     "src/sheets/select/app_link/**", "src/sheets/select/app_link.rs",
     "src/sheets/select/confirm.rs", "src/sheets/select/daemon/**",
     "src/sheets/select/permissions.rs", "src/sheets/select/quick/**",
     "src/sheets/select/quick.rs", "src/sheets/select/worktree/**",
     "src/sheets/select/worktree.rs"],
    oracle="app/daemon_menu.rs, app/multi_agents.rs, chatwidget/quick_dials.rs, bottom_pane/app_link_view.rs, app/side*.rs; chatwidget/worktree_picker.rs, windows_sandbox_prompts.rs, security_setup.rs, history_cell/session.rs (forked); bottom_pane/{feedback_view,feedback_note_view}.rs, app/feedback_upload.rs, history_cell/notices.rs (recap), debug_config.rs",
    ui="листы daemon и agent picker, открыть quick, side-тред и его подпись; Ctrl+/; вид checkout-выбора и цепочка старта, строка «Forked thread», подтверждения sandbox; страницы note/consent, ячейки recap и debug-config, заметки logout/ide/daybreak",
    deps="fw-ui-registry, fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9", tm="w2-commands", brief="§A.6")
pkg("w2-status", "UI /usage, thread usage, лимиты, карточка /status, футер и строка статуса", W2, "4", "≈3000",
    ["core/src/data/usage.rs", "core/src/features/thread_usage/**",
     "core/src/screens/status/**", "core/src/screens/status.rs",
     "core/src/screens/usage/**", "core/src/threads/limits.rs",
     "host/src/analytics/**", "host/src/analytics.rs",
     "src/pages/session/cells/status/**", "src/pages/session/footer/effort.rs",
     "src/pages/session/footer/effort_frames.rs", "src/pages/session/footer/entries.rs",
     "src/pages/session/footer/fixtures/**", "src/pages/session/footer/hints.rs",
     "src/pages/session/footer/indicators.rs", "src/pages/session/footer/lead.rs",
     "src/pages/session/footer/mod.rs", "src/pages/session/footer/status_rows.rs",
     "src/pages/session/footer/strings.rs", "src/pages/session/live/limits.rs",
     "src/pages/session/live/usage_menu.rs", "src/pages/session/status/entries.rs",
     "src/pages/session/status/fixtures/**", "src/pages/session/status/mod.rs",
     "src/pages/session/status/pending.rs", "src/pages/session/status/strings.rs",
     "src/pages/session/status_surfaces.rs", "src/pages/session/window_title.rs",
     "src/pages/usage/**", "src/services/history/usage.rs",
     "src/widgets/status_line.rs"],
    oracle="chatwidget/{usage,reset_credits,usage_notice,thread_usage,rate_limits}.rs, status/{card,rate_limits}.rs, analytics/; bottom_pane/{footer,status_line,status_line_setup,title_setup,effort_ignition,shortcut_hud}*.rs, status_indicator_widget.rs, chatwidget/status_surfaces.rs, terminal_title.rs",
    ui="пункты меню Analytics/Refresh/Reset, picker и confirm, подсказка на старте, строки карточки, ленточные предупреждения; 43 теста usage-шарда; 183 pending статус-шардов (pane_footer_1/2, chat_status_1-3); Cwd/Clock/git/headline от host; подпись side-контекста",
    first="rtwk G-new-usage-1 (`ensure_visible` в `State`), G-new-usage-2 (`visible_range`), G75/G76 (перенос UAX #14 и optimal-fit)",
    deps="fw-ui-registry, fw1-c7, fw1-c3, fw1-c2", tm="w2-status", brief="§A.7")
pkg("w2-live", "UI живой ход: backtrack, plan mode, interrupts, баннеры, empty-state animation, Markdown", W2, "2", "≈2000",
    ["core/src/screens/backtrack.rs", "src/pages/session/backtrack/**",
     "src/pages/session/banners.rs", "src/pages/session/blossom/**",
     "src/pages/session/live/browse.rs", "src/pages/session/live/mod.rs",
     "src/pages/session/live/plan_mode.rs", "src/pages/session/live/restore.rs",
     "src/widgets/markdown/**", "src/widgets/markdown.rs"],
    oracle="chatwidget/{turn_lifecycle,interrupts,reconnect,mcp_startup,plan_implementation,review}.rs, app_backtrack.rs, empty_state_animation.rs; markdown_render.rs, markdown_render/{math,mermaid}.rs, markdown_stream.rs, streaming/",
    ui="остаток chat_live-шардов (≈250 pending вне пакетов фич); steer/Esc, plan reasoning scope, luna reserve; после rtwk: настройки `MarkdownView` и рендереры; снять 112 pending markdown_streaming_1-5",
    first="rtwk: G61 (math), G62 (mermaid), G63 (soft break), G64 (отступ вложенных списков), G71 (унаследованный стиль), G73 (ThemeExtension / Markdown layout options), G60 (inline-синтаксис)",
    deps="fw-ui-registry, rtwk", tm="w2-live", brief="§A.8")

# ---------------------------------------------------------------- wave 3: whole features
W3 = "W3"
pkg("voice", "voice/realtime: /voice, F8, voice strip, настройки, split-flap, session lock", W3, "3", "≈1900",
    ["src/sheets/voice/**", "core/src/screens/voice.rs", "core/src/features/voice/**", "src/features/voice/**",
     "host/src/audio.rs", "src/pages/session/open/voice.rs", "src/pages/session/key_actions/voice.rs",
     "src/pages/session/footer/voice.rs", "src/pages/session/cells/split_flap.rs"],
    oracle="bottom_pane/voice_strip.rs, chatwidget/realtime*.rs, realtime_call.rs, app/realtime_*.rs, session_lock.rs",
    core="состояние realtime, `VoiceControl`, факты звука", ui="strip, лист настроек, split-flap ячейка",
    first="шаг host-audio: решение человека о крейте захвата/воспроизведения и о доступе к realtime API; потом host/src/audio.rs",
    deps="fw-ui-registry, fw-core-hubs (Ask::Audio)")
pkg("sidebar", "sidebar: документы, файлы, ссылки, chooser, анимация", W3, "2", "≈1300",
    ["core/src/features/sidebar/**", "src/features/sidebar/**", "src/pages/session/sidebar/**",
     "src/sheets/select/sidebar_chooser.rs", "src/pages/session/cells/file.rs"],
    oracle="app/sidebar*.rs, sidebar_chooser.rs, sidebar_tab_strip.rs, app/sidebar_document.rs",
    core="факты sidebar (документы, загрузка)", ui="панель рядом с транскриптом, chooser, ячейка file",
    first="rtwk: раскладка панели рядом с транскриптом со скользящей анимацией (SplitPane/AnimatedSize), если её нет",
    deps="fw-ui-registry")
pkg("inline-mode", "inline-режим S27: viewport, вставка в scrollback, reflow, временный alt-screen (rtwk уже умеет inline)", W3, "2", "≈1500",
    ["src/app/run.rs", "src/app/mod.rs", "src/app/screen_mode/**", "src/app/scrollback/**",
     "src/services/transcript_file.rs"],
    oracle="tui.rs, insert_history.rs, app/resize_reflow.rs, custom_terminal*, transcript_reflow.rs, app/native_history.rs",
    ui="ScreenMode из `tui.fullscreen_transcript`, committed scrollback, снимки cells_4/5 и transcript_view_3 (34 n/a)",
    first="взять готовое из rtwk: `ScreenMode::Inline` (rtwk-core/src/presenter/modes.rs:63), `binding::Static` — готовый вывод над приложением, в истории терминала (rtwk docs/widgets/app/static.md), образец inline-агента `rtwk-e2e/src/agent.rs:624 inline_agent_app` и `rtwk-e2e/src/modes.rs:74`. Чего в rtwk нет (бывшие G1/G2/G5/G7: смена alt-screen на ходу, reflow committed-строк, OSC 8/картинки в них) — проверить по этим образцам одной строкой разрыва, без rtwk-работы вперёд",
    deps="fw-core-hubs (Ask::ScreenMode)")
pkg("ui-tree-viz", "ui_tree и inline visualization", W3, "2", "≈1100",
    ["core/src/features/ui_tree/**", "src/features/ui_tree/**", "src/widgets/ui_tree/**",
     "core/src/threads/inline_vis.rs"],
    oracle="ui_tree.rs, ui_tree/, inline_visualization.rs, inline_visualization/",
    core="разбор модельного декларативного UI, директивы inline-vis (сейчас курсив-заметка)",
    ui="элементы status line и просмотр visualization", first="решение человека: что делать с viewer-HTML (host пишет файл, ссылка file://)",
    deps="fw-core-hubs")
pkg("pets-ambient", "pets: ambient-питомец, asset pack, протоколы изображений", W3, "2", "≈1200",
    ["src/sheets/pets/**", "core/src/screens/pets.rs", "src/widgets/pets/**", "host/src/pets.rs"],
    oracle="pets/{ambient,asset_pack,catalog,frames,image_protocol,sixel,preview,picker}.rs",
    core="каталог и состояние питомца как факты host", ui="ambient в футере и транскрипте, превью",
    first="rtwk G4 (картинки внутри региона, kitty/sixel/iTerm2), G10 (tmux pane-local graphics)", deps="rtwk")
pkg("startup-rest", "запуск и onboarding: хвосты S01-S06 и startup-шарды, daemon recovery, startup error, cloud environment, провайдеры", W3, "2", "≈1300",
    ["src/pages/startup/**", "core/src/screens/startup.rs", "core/src/screens/startup/**",
     "core/src/screens/onboarding.rs", "core/src/screens/onboarding/**", "core/src/screens/import/**",
     "core/src/protocol/onboarding.rs", "core/src/protocol/import.rs", "src/app/start_page.rs",
     "host/src/startup.rs", "host/src/launch.rs", "host/src/gateway.rs", "host/src/providers.rs",
     "host/src/update.rs", "host/src/import.rs", "host/src/cloud.rs", "host/src/target.rs",
     "src/services/startup.rs", "src/services/backend/**", "src/services/remote.rs"],
    oracle="onboarding/, startup_*.rs, cwd_prompt.rs, model_migration.rs, oss_selection.rs, daemon_recovery.rs, update_prompt.rs, unarchive_prompt.rs, external_agent_config_migration/",
    ui="6 pending startup-шардов; startup error screen; cloud environment picker; provider editor",
    first="rtwk: G48 (перенос только по пробелам), G49 (после `/`), G50/G51 (приоритет flex-детей `Column`), G52 (путь по сегментам), G55 (последний символ секрета)",
    deps="rtwk для G48-G55; остальное не зависит")
pkg("agents-center", "agents command center (/agents): остаток и снимки", W3, "1", "≈800",
    ["src/pages/agents/**", "core/src/screens/agents/**", "src/services/agents/**", "core/src/protocol/agents.rs"],
    oracle="app/agents_overview*.rs, app/agent_center/, app/agent_navigation.rs, app/agent_status_feed.rs",
    ui="страница есть; ячейка agent status feed; навигация из composer (Left); подпись `⌥↑` — сверить с oracle", deps="fw-ui-registry")
pkg("resume-rest", "resume/fork picker: хвосты и превью транскрипта", W3, "1", "≈600",
    ["src/pages/resume/**", "core/src/screens/resume/**", "core/src/screens/resume.rs", "src/widgets/session_list.rs"],
    oracle="resume_picker.rs, resume_picker/, resume_picker_transcript_preview.rs, named_session_lookup.rs",
    ui="12 pending resume-шардов: хранилище плотности, thread_to_transcript_cells, превью через pager",
    first="rtwk: шаблон имён кадров в kit (см. kit-seeds); pager из w2-transcript", deps="w2-transcript, kit-seeds")
pkg("terminal-features", "терминал и рантайм: Ctrl+Z, Ctrl+L, screen reader, reduced motion, уведомления, тема", W3, "1", "≈900",
    ["src/app/terminal_effects.rs", "src/app/view.rs", "src/app/theme.rs", "src/app/theme/**",
     "src/services/clipboard/**", "src/services/clipboard.rs", "src/services/open_url.rs", "host/src/main.rs",
     "host/src/viewer.rs", "src/pages/session/key_actions/clear_terminal.rs"],
    oracle="tui/job_control.rs, screen_reader*.rs, system_motion.rs, motion.rs, notifications/, terminal_probe/, clipboard_copy/, clipboard_paste/",
    ui="Ctrl+Z suspend, Ctrl+L, режим screen reader, reduced motion, настройка уведомлений (OSC 9 / BEL), probe цветов",
    first="rtwk: suspend/SIGTSTP и возврат (`App::suspend`), событие reduced-motion", deps="fw-ui-registry, fw-core-hubs (Ask::Suspend/Notify/Probe)")
pkg("app-misc", "остальное приложения: exit summary, session log, turn tips, MCP startup, reconnect, read-only, quit-confirm", W3, "1-2", "≈1100",
    ["core/src/screens/chat.rs", "core/src/screens/chat/**", "core/src/screens/tips.rs",
     "core/src/features/exit_summary/**", "src/features/exit_summary/**", "host/src/session_log.rs"],
    oracle="app/exit_summary.rs, session_log.rs, turn_tip.rs, chatwidget/{mcp_startup,reconnect,read_only,provider_error}.rs, tooltips.rs, app/server_version_notice.rs",
    core="exit summary (токены + подсказка resume), session log", ui="диалоги quit-confirm и task-running",
    deps="fw-core-hubs, fw-ui-registry")
pkg("kit-seeds", "обвязка oracle-тестов: семена kit, имена кадров, хранилище настроек", W2, "1-2", "≈1400",
    ["tests/kit/**", "tests/oracle/fixtures/**", "tests/support/**"],
    oracle="—",
    ui="семена: keymap-семя, хранилище view-mode, entry для make_elicitation_request, имена кадров склеенного снимка, Screen::feed в entry-экраны; снимает pending «no seed»/«kit names the frames» (≈160 тестов)",
    deps="—")
