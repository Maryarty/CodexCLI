"""Rows that are not in chat-feature-map.md: terminal, app, onboarding, misc; and the screen map S01-S27.

(title, status, gap, oracle, port, owner); owner is a package id, `—` only for ✅.  The title is the key
that cover.py names (`x:<title>`)."""
EXTRA = [
    # ---- terminal and runtime
    ("Inline viewport", "❌", "S27: нижний viewport со своим scrollback; rtwk ScreenMode::Inline + Static", "tui.rs, custom_terminal.rs, tui/frame_requester.rs", "—", "inline-mode"),
    ("Вставка завершённых строк в scrollback", "❌", "DECSTBM/native scrollback, OSC 8 и картинки в committed-строках (G7)", "insert_history.rs, tui/scrollback.rs, tui/history_tail.rs", "—", "inline-mode"),
    ("Resize reflow", "❌", "пересборка committed-строк из исходных ячеек при resize (G2)", "app/resize_reflow.rs, transcript_reflow.rs, resize_reflow_cap.rs", "—", "inline-mode"),
    ("Fullscreen (alt-screen + мышь)", "✅", "", "tui/alternate_screen.rs", "rtwk host (`App::run`), src/app/run.rs", "—"),
    ("Временный alt-screen для оверлеев", "❌", "вход/выход alt-screen во время работы (G1) для pager в inline", "tui/alternate_screen.rs, pager_overlay.rs", "—", "inline-mode"),
    ("Suspend / resume", "❌", "Ctrl+Z: SIGTSTP и возврат; в core есть только запись каталога keymap", "tui/job_control.rs", "core/src/keymap/catalogue.rs (запись)", "terminal-features"),
    ("Bracketed paste и kitty keyboard", "✅", "", "tui/keyboard_modes.rs, tui/input_boundary.rs", "rtwk (ввод), src/pages/session/composer/burst.rs", "—"),
    ("Focus, synchronized output, Windows console, panic hook", "?", "в port не нашлось собственных проверок: первый шаг пакета — проверить, что rtwk host делает это (focus-in/out, DEC 2026, консоль Windows, восстановление терминала при panic)", "tui/event_stream.rs, tui/frame_rate_limiter.rs, tui/windows_console.rs, tui/windows_key_sequence.rs, tui_panic_tests.rs", "rtwk-core platform/console_input.rs", "terminal-features"),
    ("Clipboard copy (native, tmux, WSL, OSC 52)", "✅", "", "clipboard_copy/, tui/selection_clipboard.rs", "src/services/clipboard/ (machine, tmux, osc52)", "—"),
    ("Мышь", "✅", "двойной/тройной клик и Shift-клик — в строках выделения", "tui/link_pointer.rs, app/link_hover.rs", "rtwk (hit-testing), src/pages/session/transcript/selection.rs", "—"),
    ("Terminal probe (OSC 10/11, DA1)", "❌", "цвета фона и тема по ответу терминала; запрос и разбор", "terminal_probe/, terminal_palette.rs, color.rs", "—", "terminal-features"),
    ("Inline images (kitty / sixel / iTerm2)", "❌", "картинки внутри региона (G4), tmux pane-local (G10)", "pets/image_protocol.rs, pets/sixel.rs, transcript_image.rs", "—", "pets-ambient"),
    ("tmux / WSL / screen", "🟡", "буфер обмена умеет tmux; нет проверки passthrough для OSC 9/8 и графики", "tui/terminal_stderr.rs, terminal_program_status.rs", "src/services/clipboard/tmux.rs", "terminal-features"),
    ("Screen reader", "❌", "режим доступности: линейный вывод без анимаций", "screen_reader.rs, screen_reader_windows.rs", "—", "terminal-features"),
    ("Reduced motion", "🟡", "порт читает флаг только в футере effort; нет системной настройки (macOS/Windows) и общего отключения анимаций", "motion.rs, system_motion.rs", "src/pages/session/footer/effort.rs", "terminal-features"),
    ("OS session lock", "❌", "блокировка сессии ОС завершает голос", "session_lock.rs, session_lock/", "—", "voice"),
    # ---- app
    ("App shell", "✅", "маршрутизация потоков и событий — один `CodexApp`, один `ThreadModel` на тред (ADR-003)", "app.rs, app/event_dispatch.rs, app/thread_routing.rs, app/thread_events.rs, app_event.rs", "core/src/app.rs, core/src/threads.rs", "—"),
    ("Startup error и daemon recovery", "🟡", "экраны есть; 6 pending startup-шардов (prompts, flow)", "startup_error.rs, daemon_recovery.rs, daemon_startup.rs, startup_recovery.rs", "src/pages/startup/prompts/, host/src/startup.rs", "startup-rest"),
    ("Multi-backend", "🟡", "локальный in-process и fake; облачный и remote (выбор бэкенда, ожидание переключения) не прошли", "app/backend_selection.rs, backend_registry.rs, backend_sessions.rs, cloud_switch_wait.rs, backend_id.rs", "host/src/target.rs, host/src/cloud.rs, src/services/backend/", "startup-rest"),
    ("Exit summary", "❌", "токены и подсказка resume при выходе", "app/exit_summary.rs, token_usage.rs", "—", "app-misc"),
    ("Update check", "🟡", "ячейка update-available и host/update.rs; нет действия обновления и кэша версий", "updates.rs, updates_cache.rs, update_action.rs, update_versions.rs, npm_registry.rs", "host/src/update.rs, " + "src/pages/session/cells/update_available.rs", "startup-rest"),
    ("Server version notice", "🟡", "заметка есть; нет сравнения версий сервера и проверки на старте", "app/server_version_notice.rs, version.rs", "src/pages/session/cells/notice.rs", "app-misc"),
    ("Turn tips и tooltips", "🟡", "ячейка session_info показывает подсказки; нет `turn_tip` в ходе и каталога tooltips", "turn_tip.rs, tooltips.rs, tooltips/, app/turn_tips.rs", "src/pages/session/cells/session_info.rs", "w2-transcript"),
    ("Empty-state animation", "🟡", "порт: `blossom`; нет процедурного освещения и политики пустого состояния", "empty_state_animation.rs, empty_state_animation/, app/empty_state_policy.rs, bottom_pane/empty_state_policy.rs", "src/pages/session/blossom/", "w2-live"),
    ("Assistant directives и git action directives", "✅", "директивы разобраны (`core/src/threads/directives.rs`); git-action-директивы показываются как текст", "assistant_directives.rs, git_action_directives.rs", "core/src/threads/directives.rs", "—"),
    ("Agents command center", "🟡", "страница `/agents` есть (src/pages/agents); остаток: agent status feed, навигация из composer, снимки", "app/agents_overview*.rs, app/agent_center/, agent_navigation.rs, app/agent_status_feed.rs", "src/pages/agents/, core/src/screens/agents/", "agents-center"),
    ("Provider editor и cloud environment", "❌", "список/редактор провайдера и выбор cloud environment", "app/provider/, app/cloud_switch_wait.rs, app_server_session/cloud_project.rs", "host/src/providers.rs (факты)", "startup-rest"),
    ("ViewStack, CenteredView, DialogOverlay", "✅", "заменено rtwk: Navigator, sheet/dialog из rtwk", "bottom_pane/view_stack.rs, bottom_pane/bottom_pane_view.rs", "src/sheets/presenter.rs", "—"),
    ("Quit-confirm и task-running", "❌", "диалоги выхода при запущенных сессиях и при активной задаче", "app/session_lifecycle.rs, chatwidget/interrupts.rs", "—", "app-misc"),
    ("Базовые ячейки", "✅", "Plain/Labeled/Prefixed/Composite, tooltip, clear-screen header — как ячейки cells/*", "history_cell/base.rs, history_cell/notices.rs", "src/pages/session/cells/{text,one_row,note,notice}.rs", "—"),
    ("Onboarding: хост и welcome", "✅", "стек шагов — маршруты rtwk; анимация welcome", "onboarding/mod.rs, onboarding/welcome.rs", "src/pages/startup/onboarding/", "—"),
    ("Onboarding: sign in, gateway, Bedrock, GCP", "🟡", "формы есть; хвосты G48 (перенос только по пробелам), G49, G50/G51 (приоритет flex-детей), G52 (путь по сегментам), G55 (последний символ секрета)", "onboarding/auth*.rs, gateway*.rs, bedrock.rs, gcp.rs, device_login.rs, local_chatgpt_auth.rs", "src/pages/startup/onboarding/signin/", "startup-rest"),
    ("Onboarding: подтверждения и миграции", "✅", "trust, cwd, model migration, update, unarchive, OSS, startup hooks, перенос из других агентов — страницы S04-S06 со снимками", "onboarding/trust_directory.rs, cwd_prompt.rs, model_migration.rs, update_prompt.rs, unarchive_prompt.rs, oss_selection.rs, startup_hooks_review.rs, external_agent_config_migration/", "src/pages/startup/prompts/, src/pages/startup/import/", "—"),
    ("Публичные виджеты и Renderable", "✅", "заменено rtwk (Widget/Element)", "public_widgets/, render/, ui_consts.rs", "rtwk", "—"),
    ("Session log", "❌", "журнал событий сессии в файл для отладки", "session_log.rs", "—", "app-misc"),
]
SCREENS = [
    ("S01", "update prompt", "✅", "—", "—"),
    ("S02", "onboarding: welcome, sign in", "✅", "—", "—"),
    ("S03", "onboarding: provider, gateway, Bedrock, GCP", "🟡", "хвосты G48-G55", "startup-rest"),
    ("S04", "directory trust", "✅", "—", "—"),
    ("S05", "startup prompts (cwd, migration, hooks, unarchive, OSS, daemon recovery, startup error, Windows sandbox)", "🟡", "Windows sandbox (fw1-c8), startup error", "startup-rest"),
    ("S06", "import from other agents", "✅", "—", "—"),
    ("S10", "main chat", "🟡", "см. разделы «Транскрипт», «Живой ход», «Composer», «Футер»", "w2-live"),
    ("S11", "streaming and history cells, markdown, diff", "🟡", "см. «Транскрипт и ячейки»", "w2-cells"),
    ("S12", "composer popovers", "✅", "—", "—"),
    ("S13", "approvals and questions", "🟡", "решение в транскрипте, write_stdin, черновики вопросов", "w2-approvals"),
    ("S14", "generic selection dialog", "🟡", "model/permissions хвосты, small_dialogs", "w2-approvals"),
    ("S15", "settings dialogs", "🟡", "/config, /experimental, /skills, /hooks, keymap editor, провайдер", "w2-settings"),
    ("S16", "transcript pager, static pager, search", "🟡", "статический pager, поиск", "w2-transcript"),
    ("S17", "backtrack", "✅", "—", "—"),
    ("S18", "resume / fork picker", "🟡", "12 pending, превью транскрипта", "resume-rest"),
    ("S19", "/status card", "🟡", "usage треда, строка сервера, лимиты", "w2-status"),
    ("S20", "/usage dashboard", "🟡", "reset-flow, billing, подсказка", "w2-status"),
    ("S21", "agents command center, subagent switcher", "🟡", "agent status feed; switcher (/subagents)", "agents-center"),
    ("S22", "sidebar and side conversation", "❌", "обе панели", "sidebar"),
    ("S23", "small dialogs", "🟡", "daemon menu, app link, user verification, feedback", "w2-approvals"),
    ("S24", "pets", "🟡", "ambient, asset packs, графика", "pets-ambient"),
    ("S25", "voice / realtime", "❌", "вся функция", "voice"),
    ("S26", "ui_tree and inline visualization", "❌", "обе части", "ui-tree-viz"),
    ("S27", "inline mode", "❌", "режим целиком", "inline-mode"),
]


# Hand edits of the generated port-map, kept as data (a rebuild must not lose them): a chat-map row whose status
# or note changed since chat-feature-map.md was cut, and rows that chat-feature-map.md does not have (inserted after
# the row named).  Closed rows (✅) get no package.
OVERRIDES = {'/recap': dict(status='✅', note='fix-recap-title 4131311a7: AppCx::structured_turn (ephemeral read-only thread, outputSchema), recap cell / failure line; остаток: отказ для облачной беседы -> w2-commands')}
INSERTS = [('/recap', [
    dict(title='Пробел: /recap и предложение имени треда — заглушки', status='✅', note='fix-recap-title 4131311a7: заглушки Ask::StructuredTurn/SuggestTitle удалены; /recap и предложение имени идут через AppCx::structured_turn; остаток (авто-имя по первому сообщению, отмена устаревшего запуска) -> w2-commands', oracle='temporary_structured_request.rs, app/recap.rs, app/thread_title.rs', port='core/src/features/recap, core/src/features/rename, src/services/effects.rs'),
    dict(title='Пробел: потеря/восстановление потока треда', status='✅', note='fix-stream-lost 78e957dcf: host/src/cloud.rs шлёт BackendEv::StreamLost{thread,error}/StreamRestored; простой тред — Command::ReconnectIdleThread, в ходе — строка Error, очередь держится; восстановление: history_may_be_incomplete + отправка очереди; открытие потерянного треда перечитывает его. Остаток: restore_reconnected_input (steer до эха) -> live-turn; AppEv::ShuttingDown без отправителя (нет on_stop в rtwk, потребителя нет)', oracle='app/app_server_events.rs:130-170, chatwidget/input_flow.rs', port='host/src/in_process.rs, host/src/cloud.rs, core/src/threads'),
])]
