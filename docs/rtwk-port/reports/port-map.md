# Карта всего порта: oracle (codex-rs/tui) против порта (codex-rs/tui-rtwk)

Дата: 2026-11-24, база 01741db4b (батчи 10-12). Документ собран скриптом `tools/rtwk-port/map-full/build.py` из данных рядом с ним: правится дата, перезапуск. Проверка непротиворечивости — `tools/rtwk-port/map-full/check.py`. Карта экрана чата `chat-feature-map.md` остаётся (подробные ячейки, числа тестов, архитектура ADR-013) и ссылается сюда; здесь — вся функциональность oracle без исключений, пакеты работ по владению файлами, и 1228 pending-тестов по пакетам.

**Итог.** Строк-функций: 326 (экран чата — 291 из `chat-feature-map.md` + 35 вне экрана чата): ✅ 135, 🟡 86, ❌ 100, «?» 5. Все 312 строк `codex-tui-features.csv` покрыты строкой (раздел «Покрытие CSV»), все 1230 файлов `codex-rs/tui/src` — пакетом или «✅ rtwk» (`port-map-modules.tsv`), все 24 экранов карты S01-S27 (в карте нет S07-S09) — строкой. Пакетов: 42 (волна 1 — 9, идёт; волна 2 — 23; волна 3 — 10); кроме волны 1 — 53-58 прогонов порт-агента (прогон ≈150-250 ходов, ≈1500 написанных строк). Pending-тестов oracle: 1228, у каждого пакет (раздел «Тесты»).

## Как читать

- Статус: ✅ работает или заменено rtwk; 🟡 частично; ❌ нет; «?» не удалось установить (первый шаг пакета — проверить). Чего нет — слово в слово из `chat-feature-map.md` для строк чата (их числа и оговорки не перебирались), для остальных — по коду на 01741db4b.
- Oracle — файлы относительно `codex-rs/tui/src`; порт — относительно `codex-rs/tui-rtwk` (`(нет)` — файла ещё нет, его создаёт пакет). Пакет — владелец, чьими файлами строка закрывается; «ядро: fw1-cN» — ядровая половина уже идёт в волне 1 (контейнер fw1-core), её файлы здесь не переплановываются, пакет делает UI-половину и вешает проверку.
- Нет списка «не пакеты»: строка, упирающаяся в rtwk, получает пакет, первый шаг которого называет, что просить у rtwk (поле «первый шаг»).

## Правила владения

1. **Один файл — один пакет.** Каждый production-файл порта (`src`, `core/src`, `host/src`, `tests/rules`) в точности у одного пакета: явным путём, `dir/**` или `rest:dir` (все ещё не названные файлы каталога). `check.py` падает при пересечении файла или пути; вывод проверки (0 пересечений, 0 файлов без владельца) — в `task_done`.
2. **Хаб не отдаётся одному пакету на все фичи** (ADR-012). Фича входит в приложение только дверьми слайса: `feature_list!`/`target_of` (слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможность хоста, строка в `hub_budget.txt`), `ThreadModel.features` (состояние треда). Правка хаба — только СЖАТИЕ, и это работа двух framework-пакетов: **fw-core-hubs** (общий вариант `SessionIntent::Feature`, конец арма NotAvailable, одна дверь возможностей хоста) и **fw-ui-registry** (реестры действий клавиш, целей open, ячеек, режимов и тестовых корней; `session_page.rs` — один владелец с причиной). Остальные пакеты называют только файлы своего слайса, своих страниц и свои тесты.
3. **Двери.** `core/src/features/mod.rs` и `src/features/mod.rs` — по одной строке на слайс (`feature_list!`); их не владеет никто, слайс дописывает свою строку (слияние без конфликта по смыслу). Это единственное исключение из пункта 1.
4. **Тесты.** Файлы тестов (шарды `tests/oracle/*.rs`, `tests/pages/*`) не входят в проверку пересечений: шард собран из независимых функций, пакет правит только функции из своего списка `tools/rtwk-port/map-full/unpend/<пакет>.txt` (сгенерирован, по одному `шард::тест` в строке); несколько пакетов в одном шарде сливаются построчно. Новые тестовые файлы пакета — `tests/pages/<пакет>_*.rs`, строки `mod` в корнях добавляет реестр fw-ui-registry.
5. **Порядок.** Волна 1 (fw1-c1..c9) идёт. Волна 2: сначала framework-пакеты, пока они не влиты, UI-пакеты стоят; затем UI-половины и sweep-пакеты параллельно (пересечений нет). Волна 3: целые фичи (voice, sidebar, inline, ui_tree, pets, startup, …); часть упирается в rtwk (см. «первый шаг»).

## Пакеты

| Пакет | Волна | Прогонов | Строк | Строк закрыто | Pending-тестов | Зависит от |
|---|---|---|---|---|---|---|
| `fw1-c1` — core: /diff-документ pager, /presets, /daemon, /subagents, app link, provider switch | W1 (идёт) | 1 | ≈750 | ядро для 18 | 0 | — |
| `fw1-c2` — core: внешний редактор Ctrl+G, очередь команд, состояния composer, /side /btw | W1 (идёт) | 1 | ≈1150 | ядро для 24 | 0 | — |
| `fw1-c3` — core: /experimental /skills /hooks, /config /tui /keymap, аргументные формы (settings-слайс) | W1 (идёт) | 1-2 | ≈1500 | ядро для 21 | 0 | — |
| `fw1-c4` — core: /copy и Ctrl+O, /logout /ide /daybreak /recap /debug-config /debug-m-* | W1 (идёт) | 1 | ≈950 | ядро для 20 | 0 | — |
| `fw1-c5` — core: exploration-группы, фоновые терминалы и /ps, инвентарь /mcp | W1 (идёт) | 1 | ≈900 | ядро для 5 | 0 | — |
| `fw1-c6` — core: safety buffering, misalignment, guardian, compaction | W1 (идёт) | 1 | ≈800 | ядро для 5 | 0 | — |
| `fw1-c7` — core: сброс лимита /usage, billing, /status thread usage и строка сервера, предупреждения лимитов | W1 (идёт) | 1 | ≈900 | ядро для 15 | 0 | — |
| `fw1-c8` — core: worktree/checkout для /new /fork /worktree, «Forked thread», Windows sandbox, security setup | W1 (идёт) | 1 | ≈1000 | ядро для 8 | 0 | — |
| `fw1-c9` — core: /feedback (note, upload), решение approval, вопросы (черновики, очередь) | W1 (идёт) | 1 | ≈700 | ядро для 19 | 0 | — |
| `fw-core-hubs` — framework: core-хабы — сжатие (`SessionIntent::Feature`, конец арма NotAvailable) и одна дверь возможностей хоста (`effects::Ask`); с 2026-11-24 = tm fw-hubs-a/b/c + fix-recap-title, fix-stream-lost (arch-pillars P1-P9) | W2 | 3 + 2 | ≈950 | — (сжатие хабов) | 0 | после fw1-c1..c9, fw1-c2 (Ask::Editor) |
| `fw-ui-registry` — framework: сжатие UI-хабов — реестры действий, целей, ячеек и режимов; session_page.rs по заботам | W2 | 1-2 | ≈1000 | — (сжатие хабов) | 0 | после fw1-c1..c9 (их открытые sheet) |
| `ui-pager-find` — UI /diff, статический pager, поиск по транскрипту F3 и по pager | W2 | 2 | ≈1500 | 8 | 17 | fw-ui-registry, fw1-c1 |
| `ui-select-copy` — UI выбор и копирование: select mode, copy mode, /copy, Ctrl+O, Markdown-копия | W2 | 2 | ≈1350 | 11 | 103 | fw-ui-registry, fw1-c4 |
| `ui-transcript-chrome` — UI хром транскрипта: F4, закреплённая шапка prompt, раскрытие деталей, колесо, страницы истории | W2 | 1 | ≈700 | 8 | 98 | fw-ui-registry |
| `ui-composer-states` — UI composer: Ctrl+G, состояния, мышь, запуск очереди, flash, sparkle | W2 | 1 | ≈950 | 9 | 16 | fw-ui-registry, fw1-c2 |
| `composer-sweep` — composer: тела textarea/composer_4/composer_popups, steer, hidden paste, картинки из буфера, Vim в ответе, startup draft | W2 | 2 | ≈1500 | 12 | 82 | fw-ui-registry |
| `ui-exec-mcp` — UI exploration, фоновые терминалы, /ps, инвентарь /mcp | W2 | 2 | ≈1000 | 5 | 20 | fw-ui-registry, fw1-c5 |
| `ui-safety-compaction` — UI safety buffering, misalignment, guardian, compaction | W2 | 1 | ≈650 | 5 | 34 | fw-ui-registry, fw1-c6 |
| `ui-approvals-rest` — UI approvals и вопросы: решение в транскрипте, черновики, Vim в ответе, write_stdin | W2 | 2 | ≈1000 | 10 | 59 | fw-ui-registry, fw1-c9 |
| `ui-usage-limits` — UI /usage: меню и reset, billing, подсказка на старте; /status: usage треда, строка сервера, предупреждения; страница аналитики | W2 | 2 | ≈1500 | 15 | 60 | fw-ui-registry, fw1-c7 |
| `ui-settings-feeds` — UI /experimental, /skills, /hooks, аргументные формы, memories | W2 | 1 | ≈550 | 11 | 5 | fw-ui-registry, fw1-c3 |
| `ui-config-panel` — UI /config и /tui (панель настроек, выбор inline/fullscreen) | W2 | 1 | ≈800 | 3 | 0 | ui-settings-feeds, fw1-c3 |
| `ui-keymap-editor` — UI /keymap: пикер, захват клавиши, отладка, конфликты | W2 | 1 | ≈700 | 2 | 4 | ui-settings-feeds, fw1-c3 |
| `ui-openers-side` — UI /presets, /daemon, /subagents, app link, /side и /btw | W2 | 2 | ≈850 | 10 | 9 | fw-ui-registry, fw1-c1, fw1-c2 |
| `ui-worktree-sandbox` — UI worktree/checkout, «Forked thread», Windows sandbox, security setup, full-access confirm | W2 | 2 | ≈800 | 8 | 26 | fw-ui-registry, fw1-c8 |
| `ui-feedback-cmds` — UI /feedback (note, consent), /recap, /debug-config, /logout, /ide, /daybreak | W2 | 1 | ≈900 | 9 | 29 | fw-ui-registry, fw1-c4, fw1-c9 |
| `cells-sweep` — ячейки и снимки: collab, картинки, Worked for, гиперссылки, plan/patch/hook, cells_1-5 | W2 | 2 | ≈1400 | 4 | 56 | fw-ui-registry |
| `markdown-gaps` — Markdown: math, mermaid, soft break, отступ списков, ThemeExtension, inline-синтаксис, dim-стиль | W2 | 1 | ≈700 | 1 | 112 | rtwk |
| `dialogs-sweep` — малые диалоги и пикеры: шорткаты «?», /warnings, тап по строке, user verification, model/permissions/apps/review хвосты | W2 | 2 | ≈1200 | 14 | 53 | fw-ui-registry |
| `footer-status-sweep` — футер, строка статуса, /statusline, /title, настройка status line; pane_footer и chat_status | W2 | 2 | ≈1500 | 5 | 148 | fw-ui-registry, fw1-c3, fw1-c2 |
| `live-turn-sweep` — живой ход: баннеры, backtrack, plan mode, review/fork при MCP startup; chat_live_1-6 | W2 | 2 | ≈1300 | 2 | 46 | fw-ui-registry |
| `voice` — voice/realtime: /voice, F8, voice strip, настройки, split-flap, session lock | W3 | 3 | ≈1900 | 5 | 1 | fw-ui-registry, fw-core-hubs (Ask::Audio) |
| `sidebar` — sidebar: документы, файлы, ссылки, chooser, анимация | W3 | 2 | ≈1300 | 1 | 0 | fw-ui-registry |
| `inline-mode` — inline-режим S27: viewport, вставка в scrollback, reflow, временный alt-screen (rtwk уже умеет inline) | W3 | 2 | ≈1500 | 6 | 12 | fw-core-hubs (Ask::ScreenMode) |
| `ui-tree-viz` — ui_tree и inline visualization | W3 | 2 | ≈1100 | 1 | 0 | fw-core-hubs |
| `pets-ambient` — pets: ambient-питомец, asset pack, протоколы изображений | W3 | 2 | ≈1200 | 4 | 0 | rtwk |
| `startup-rest` — запуск и onboarding: хвосты S01-S06 и startup-шарды, daemon recovery, startup error, cloud environment, провайдеры | W3 | 2 | ≈1300 | 6 | 13 | rtwk для G48-G55; остальное не зависит |
| `agents-center` — agents command center (/agents): остаток и снимки | W3 | 1 | ≈800 | 1 | 0 | fw-ui-registry |
| `resume-rest` — resume/fork picker: хвосты и превью транскрипта | W3 | 1 | ≈600 | 1 | 5 | ui-pager-find, kit-seeds |
| `terminal-features` — терминал и рантайм: Ctrl+Z, Ctrl+L, screen reader, reduced motion, уведомления, тема | W3 | 1 | ≈900 | 9 | 0 | fw-ui-registry, fw-core-hubs (Ask::Suspend/Notify/Probe) |
| `app-misc` — остальное приложения: exit summary, session log, turn tips, MCP startup, reconnect, read-only, quit-confirm | W3 | 1-2 | ≈1100 | 4 | 0 | fw-core-hubs, fw-ui-registry |
| `kit-seeds` — обвязка oracle-тестов: семена kit, имена кадров, хранилище настроек | W2 | 1-2 | ≈1400 | 1 | 220 | — |

### `fw1-c1`

- **Что:** core: /diff-документ pager, /presets, /daemon, /subagents, app link, provider switch. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈750 строк.
- **Задача tm:** `fw1-c1` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/screens/transcript/diff.rs`, `core/src/screens/transcript.rs`, `core/src/screens/select/presets.rs`, `core/src/screens/select/quick_dials.rs`, `core/src/screens/select/daemon.rs`, `core/src/screens/select/agent_picker.rs`, `host/src/git_diff.rs`.
- **Oracle:** get_git_diff.rs, app/quick_dials.rs, chatwidget/quick_dials.rs, app/daemon_menu.rs, app/multi_agents.rs.
- **Ядро / шаги:** вся (P1+P3 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c2`

- **Что:** core: внешний редактор Ctrl+G, очередь команд, состояния composer, /side /btw. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈1150 строк.
- **Задача tm:** `fw1-c2` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/threads/queue.rs`, `core/src/threads/chat.rs`, `core/src/threads/side.rs`, `core/src/features/external_editor/**`, `core/src/features/side/**`, `host/src/editor.rs`.
- **Oracle:** external_editor.rs, chatwidget/input_queue.rs, session_queue_commands.rs, app/side.rs.
- **Ядро / шаги:** вся (P2+P13 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c3`

- **Что:** core: /experimental /skills /hooks, /config /tui /keymap, аргументные формы (settings-слайс). **Волна:** W1 (идёт); **размер:** 1-2 прогонов, ≈1500 строк.
- **Задача tm:** `fw1-c3` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/features/settings/**`, `src/features/settings/strings.rs`, `host/src/settings.rs`.
- **Oracle:** experimental_features.rs, hooks_rpc.rs, config_panel/, keymap_setup/, app/thread_title.rs, status_surface_commands.rs.
- **Ядро / шаги:** вся (P4+P5+P15 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c4`

- **Что:** core: /copy и Ctrl+O, /logout /ide /daybreak /recap /debug-config /debug-m-*. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈950 строк.
- **Задача tm:** `fw1-c4` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/screens/select/copy.rs`, `core/src/features/copy/**`, `core/src/features/recap/**`, `core/src/features/small_cmds/**`, `host/src/ide.rs`, `src/features/copy/strings.rs`, `src/features/recap/strings.rs`, `src/features/small_cmds/strings.rs`.
- **Oracle:** chatwidget/copy_picker.rs, markdown_copy.rs, app/recap.rs, debug_config.rs, daybreak.rs, ide_context.rs.
- **Ядро / шаги:** вся (P6+P14 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c5`

- **Что:** core: exploration-группы, фоновые терминалы и /ps, инвентарь /mcp. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈900 строк.
- **Задача tm:** `fw1-c5` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/threads/processes.rs`, `core/src/threads/mcp_inventory.rs`, `core/src/threads/exploration.rs`.
- **Oracle:** history_cell/exec.rs, bottom_pane/unified_exec_footer.rs, history_cell/mcp.rs, app/background_requests.rs.
- **Ядро / шаги:** вся (P9 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c6`

- **Что:** core: safety buffering, misalignment, guardian, compaction. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈800 строк.
- **Задача tm:** `fw1-c6` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/threads/safety.rs`, `core/src/threads/compaction.rs`, `core/src/threads/guardian.rs`.
- **Oracle:** chatwidget/safety_buffering.rs, app/safety_buffering.rs, misalignment_policy.rs, chatwidget/compaction.rs.
- **Ядро / шаги:** вся (P10 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c7`

- **Что:** core: сброс лимита /usage, billing, /status thread usage и строка сервера, предупреждения лимитов. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈900 строк.
- **Задача tm:** `fw1-c7` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/screens/usage/**`, `core/src/screens/usage.rs`, `core/src/threads/limits.rs`, `host/src/usage.rs`.
- **Oracle:** chatwidget/usage.rs, reset_credits.rs, chatwidget/thread_usage.rs, rate_limits.rs, usage_notice.rs.
- **Ядро / шаги:** вся (P11+P12 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c8`

- **Что:** core: worktree/checkout для /new /fork /worktree, «Forked thread», Windows sandbox, security setup. **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈1000 строк.
- **Задача tm:** `fw1-c8` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/screens/select/worktree.rs`, `core/src/screens/select/worktree/**`, `core/src/screens/select/permission_sandbox.rs`, `core/src/features/security_setup/**`, `host/src/workspace.rs`.
- **Oracle:** chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, windows_sandbox_prompts.rs, security_setup.rs.
- **Ядро / шаги:** вся (P16+P18 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c9`

- **Что:** core: /feedback (note, upload), решение approval, вопросы (черновики, очередь). **Волна:** W1 (идёт); **размер:** 1 прогонов, ≈700 строк.
- **Задача tm:** `fw1-c9` (идёт; только ядровая половина).
- **Файлы (владеет):** `core/src/features/feedback/**`, `src/features/feedback/strings.rs`, `core/src/threads/decisions.rs`, `core/src/threads/questions.rs`, `host/src/feedback.rs`.
- **Oracle:** bottom_pane/feedback_view.rs, feedback_note_view.rs, app/feedback_upload.rs, history_cell/approvals.rs, request_user_input/.
- **Ядро / шаги:** вся (P17+P19 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw-core-hubs`

- **Что:** framework: core-хабы — сжатие (`SessionIntent::Feature`, конец арма NotAvailable) и одна дверь возможностей хоста (`effects::Ask`). **Волна:** W2; **размер:** 1-2 прогонов, ≈950 строк.
- **Файлы (владеет):** `core/src/app/session.rs`, `core/src/commands/dispatch.rs`, `core/src/commands.rs`, `core/src/threads.rs`, `core/src/threads/cells.rs`, `core/src/app.rs`, `tests/rules/hub_budget.txt`, `core/src/app/effects.rs`, `host/src/effects.rs`, `host/src/in_process.rs`, `host/src/facts.rs`, `src/services/effects.rs`, `tests/rules/layers.rs`, `rest:core/src`, `rest:host/src`, `rest:tests/rules`.
- **Oracle:** app/event_dispatch.rs, chatwidget/slash_dispatch.rs, slash_command.rs, app_event.rs.
- **Ядро / шаги:** ADR-012: пакет фичи НЕ владеет хабом и не пишет в него свои строки; фича входит в приложение только через двери слайса: `feature_list!`/`target_of` (слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможности хоста), `ThreadModel.features` (состояние треда). Правка хаба бывает только СЖАТИЕМ — это и есть вся работа этого пакета. Шаг 1: общий вариант `Feature` в `SessionIntent` (то самое «Need of the ADR-012 owner» в `tests/rules/hub_budget.txt`: «слэш-список не имеет generic-варианта») — после него слэш-команда нового слайса не добавляет варианта; бюджет `SessionIntent 43` фиксируется. Шаг 2 (после слияния fw1-c1..c9): удалить остаток арма `S::Recap | S::Side | … => NotAvailable` (`app/session.rs:361-386`), `Notice::NotAvailable`/`Stub`; команды, что остались без владельца (`/model <имя>`, `/permissions` без аргумента, `/review`, `/resume <имя>`, `/apps`), переехали в слайсы пакетов dialogs-sweep и resume-rest — арм сжимается до пустого и исчезает, бюджет в `hub_budget.txt` опускается на столько же. Шаг 3: `threads.rs`/`cells.rs` — только вычёркивание вариантов `ThreadEv`/`ThreadAsk`, ушедших в `ThreadModel.features` (счётчики в `hub_budget.txt` падают; рост запрещён правилом). Остальное в `core/src` и `host/src`, чего никто не назвал, — тоже у этого пакета: фичи сюда не пишут. Шаг 4 (дверь возможностей хоста): `AppCx::effect` с `effects::Ask` — единственная дверь для возможностей хоста, у неё один enum на всех; поэтому пакет один раз добавляет заранее названные варианты `Audio` (voice), `Suspend` (Ctrl+Z), `Notify` (OSC 9 / BEL), `ScreenMode` (alt-screen для inline), `Probe` (цвета терминала), `SessionLock` с `Done`-ответами, строками в `hub_budget.txt` и пустыми обработчиками хоста. Причина одного владельца (и единого пакета с шагами 1-3: у них общий `hub_budget.txt`): пакеты voice, terminal-features, inline-mode идут параллельно и иначе правили бы один enum; дальше каждый пишет только свой файл хоста (`host/src/audio.rs` и т. п.) и слайс. Редактор Ctrl+G (`Ask::Editor`) делает fw1-c2..
- **Зависит:** после fw1-c1..c9, fw1-c2 (Ask::Editor).
- **Строки (0):** —.
- **Тесты:** —.

### `fw-ui-registry`

- **Что:** framework: сжатие UI-хабов — реестры действий, целей, ячеек и режимов; session_page.rs по заботам. **Волна:** W2; **размер:** 1-2 прогонов, ≈1000 строк.
- **Файлы (владеет):** `src/pages/session/session_page.rs`, `src/pages/session/keys.rs`, `src/pages/session/open.rs`, `src/pages/session/actions.rs`, `src/pages/session/mod.rs`, `src/pages/session/slice.rs`, `src/pages/session/popup.rs`, `src/pages/session/strings.rs`, `src/pages/session/key_actions/mod.rs`, `src/pages/session/transcript/mod.rs`, `src/pages/session/cells/mod.rs`, `src/pages/session/cells/dispatch.rs`, `src/pages/session/cells/notice.rs`, `src/pages/session/composer/mod.rs`, `src/pages/mod.rs`, `src/sheets/mod.rs`, `src/sheets/presenter.rs`, `src/app/presenters/mod.rs`, `src/app/debug/**`, `src/l10n/**`, `src/keymap/**`, `tests/pages.rs`, `tests/screens.rs`, `tests/oracle.rs`, `tests/rules.rs`, `rest:src`.
- **Oracle:** chatwidget/slash_dispatch.rs, app/event_dispatch.rs, app/input.rs.
- **UI:** Тот же принцип, что у fw-core-hubs: UI-хабы — только сжатие. Где можно, хаб заменяется реестром (макрос по списку, как `feature_list!`/`screen_list!`), где нельзя — остаётся один владелец с причиной. Реестр 1, действия клавиш: `Action` в `keys.rs` и его диспатч собираются списком `action_list!` из `key_actions/<имя>.rs` (`find`, `copy`, `external_editor`, `side`, `voice`, `focus_activity`, `clear_terminal`); файл действия — привязка и `run(cx, state)`. Реестр 2, цели open: `open.rs` держит общий путь `Target::Feature` (он уже есть, `open.rs:75`) и сжимается: `pager`, `daemon`, `agent_picker`, `voice` становятся слайсами и уходят из арма. Реестр 3, ячейки и режимы: `cells/dispatch.rs`, `cells/mod.rs`, `transcript/mod.rs` собирают ячейки (`exploration`, `processes`, `mcp_inventory`, `compaction`, `misalignment`, `decision`, `forked`, `recap`) и режимы (`find`, `select_mode`, `copy_mode`, `diff_view`) списками `cell_list!`/`mode_list!` по файлам. Реестр 4, тесты: `tests/pages.rs`, `screens.rs`, `oracle.rs` получают `mod`-строки из каталога (`build.rs` пишет `mods.rs` по `tests/pages/*.rs`) — новый тестовый файл не правит корень. Без реестра, один владелец: `session_page.rs` (1218 строк, один `Component`; причина — его состояние общее, дробить нечем, кроме выноса забот в `session/*.rs`; пакет выносит и дальше правит только он) и `actions.rs` (сжатие: снять `not_yet`, `presets_action`, `daemon_action`, `action_subagents`). Двери `core/src/features/mod.rs` и `src/features/mod.rs` — одна строка слайса, их не владеют (см. «Двери»)..
- **Первый шаг:** идёт первым в W2: пока реестры не влиты, UI-пакеты ниже стоят (иначе правят один файл).
- **Зависит:** после fw1-c1..c9 (их открытые sheet).
- **Строки (0):** —.
- **Тесты:** —.

### `ui-pager-find`

- **Что:** UI /diff, статический pager, поиск по транскрипту F3 и по pager. **Волна:** W2; **размер:** 2 прогонов, ≈1500 строк.
- **Файлы (владеет):** `src/pages/session/transcript/pager.rs`, `src/pages/session/transcript/sheet.rs`, `src/pages/session/transcript/diff_view.rs`, `src/pages/session/transcript/find.rs`, `src/pages/session/transcript/find_footer.rs`, `src/pages/session/transcript/strings_find.rs`, `src/pages/session/open/pager.rs`, `src/pages/session/key_actions/find.rs`.
- **Oracle:** pager_overlay.rs, pager_overlay/{scrolling,transcript}.rs, diff_render.rs, diff_model.rs, transcript_view/{search,search_footer,search_presentation}.rs.
- **UI:** `StaticPager` с документом из fw1-c1 и live-tail sync; порт `diff_render.rs`; поиск (подсветка, счётчик, n/N, футер поиска) над `ItemTexts`; F3.
- **Первый шаг:** rtwk: `ItemTexts` (текст по элементу транскрипта, равный нарисованному) — без него поиск и выделение считаются по исходникам.
- **Зависит:** fw-ui-registry, fw1-c1.
- **Строки (8):** /diff; Статический pager (/diff, вывод): строки, страницы, Home/End; Pager: поиск /, n/N; Pager: live tail sync при росте транскрипта; Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N); Подсказки поиска и футер поиска; F3 — поиск; F4 — focus activity.
- **Тесты:** 17 pending (список — `tools/rtwk-port/map-full/unpend/ui-pager-find.txt`).

### `ui-select-copy`

- **Что:** UI выбор и копирование: select mode, copy mode, /copy, Ctrl+O, Markdown-копия. **Волна:** W2; **размер:** 2 прогонов, ≈1350 строк.
- **Файлы (владеет):** `src/pages/session/transcript/selection.rs`, `src/pages/session/transcript/copy.rs`, `src/pages/session/transcript/copy_feedback.rs`, `src/pages/session/transcript/select_mode.rs`, `src/pages/session/transcript/copy_mode.rs`, `src/pages/session/transcript/strings_select.rs`, `rest:src/features/copy`, `src/pages/session/key_actions/copy.rs`.
- **Oracle:** transcript_view/{select_mode,copy_mode,selection}.rs, chatwidget/copy_picker.rs, markdown_copy.rs, clipboard_copy.rs.
- **UI:** лист copy picker, Ctrl+O, payload plain/HTML/Markdown, режим выделения с клавиатуры, copy mode (лидер Ctrl+X), двойной/тройной клик.
- **Первый шаг:** rtwk: чтение start/end у `SelectionAreaController`, жесты двойного/тройного клика, `ItemTexts`.
- **Зависит:** fw-ui-registry, fw1-c4.
- **Строки (11):** Гиперссылки в тексте (клик открывает); /copy — копировать ответ; Copy/export picker (кастомные клавиши, payload); Копирование выделения (Ctrl+C / по отпусканию); Двойной/тройной клик: слово/строка/абзац; Shift-клик расширяет, правая кнопка копирует; Копия как Markdown/plain (payload); Режим выделения с клавиатуры (select mode); Режим копирования (copy mode, Ctrl+X-лидер); Клик по ссылке/файлу в overlay; Ctrl+O — копировать ответ.
- **Тесты:** 103 pending (список — `tools/rtwk-port/map-full/unpend/ui-select-copy.txt`).

### `ui-transcript-chrome`

- **Что:** UI хром транскрипта: F4, закреплённая шапка prompt, раскрытие деталей, колесо, страницы истории. **Волна:** W2; **размер:** 1 прогонов, ≈700 строк.
- **Файлы (владеет):** `src/pages/session/transcript/follow.rs`, `src/pages/session/transcript/history.rs`, `src/pages/session/transcript/turn_details.rs`, `src/pages/session/transcript/strings.rs`, `src/pages/session/transcript_header.rs`, `src/pages/session/transcript_pane.rs`, `src/pages/session/transcript_activity.rs`, `src/pages/session/activity.rs`, `src/pages/session/key_actions/focus_activity.rs`.
- **Oracle:** transcript_view/{prompt_header,turn_details,turn_tip,disclosure,activity,follow_control,footer}.rs, turn_tip.rs.
- **UI:** F4 focus activity, pinned header, TurnTip, ширинно-зависимый счёт скрытых строк, mouse_scroll_speed, ctrl+home / alt+<.
- **Первый шаг:** rtwk: `ListController::visible_range()` (шапка prompt, TurnTip, 50+50 строк вывода).
- **Зависит:** fw-ui-registry.
- **Строки (8):** Сворачивание деталей хода (collapse_turn_details), раскрытие; Фокус на activity-группах (F4) и подсказка раскрытия; PageUp/PageDown, колесо мыши, Home/End в транскрипте; Подгрузка старых страниц истории при прокрутке вверх; Закреплённая шапка prompt над транскриптом (pinned header); Скорость колеса (mouse_scroll_speed); Старые страницы в overlay (history pages); Turn tips и tooltips.
- **Тесты:** 98 pending (список — `tools/rtwk-port/map-full/unpend/ui-transcript-chrome.txt`).

### `ui-composer-states`

- **Что:** UI composer: Ctrl+G, состояния, мышь, запуск очереди, flash, sparkle. **Волна:** W2; **размер:** 1 прогонов, ≈950 строк.
- **Файлы (владеет):** `src/pages/session/composer/states.rs`, `src/pages/session/composer/mouse.rs`, `src/pages/session/composer/drain.rs`, `src/pages/session/footer/flash.rs`, `src/pages/session/footer/sparkle.rs`, `src/widgets/composer_sparkle.rs`, `src/features/external_editor/**`, `src/pages/session/key_actions/external_editor.rs`.
- **Oracle:** external_editor.rs, chatwidget/input_queue.rs, bottom_pane/chat_composer/{mouse,sparkle,agents_navigation,warning_notice}.rs, bottom_pane/chat_composer.rs (состояния).
- **UI:** состояния disabled/shutdown/parent-owned, подсказка agents-nav, мышь (курсор, выделение, правый клик), Tab-очередь команд, запуск при drain, Ctrl+G.
- **Первый шаг:** rtwk G67: приостановка терминала на время `$EDITOR` из обработчика (`App::suspend` доступен только host).
- **Зависит:** fw-ui-registry, fw1-c2.
- **Строки (9):** Фоновая «starfield»/sparkle эффекты композера; Очередь slash- и shell-команд во время хода; Состояния composer: disabled, shutdown, parent-owned, agents; Внешний редактор Ctrl+G; Мышь в composer (курсор, выделение, вставка правой кнопкой); Sparkle/«искры» над composer; Flash в футере и временный override подсказки; Tab — в очередь (сообщения); Ctrl+G — внешний редактор.
- **Тесты:** 16 pending (список — `tools/rtwk-port/map-full/unpend/ui-composer-states.txt`).

### `composer-sweep`

- **Что:** composer: тела textarea/composer_4/composer_popups, steer, hidden paste, картинки из буфера, Vim в ответе, startup draft. **Волна:** W2; **размер:** 2 прогонов, ≈1500 строк.
- **Файлы (владеет):** `rest:src/pages/session/composer`, `src/pages/session/popovers/**`, `src/widgets/composer_draft/**`, `src/widgets/composer_draft.rs`, `core/src/screens/composer/**`, `core/src/screens/composer.rs`, `core/src/screens/popovers.rs`, `core/src/kit/**`.
- **Oracle:** bottom_pane/{chat_composer,textarea,paste_burst,command_popup,file_search_popup,mentions_v2,skill_popup}*, chat_composer/{history_search,edit_history,draft_state,paste_input,attachment_state}.rs, startup_draft*.rs.
- **UI:** пересчёт 100 pending composer-шардов; Vim replace/yank/dot и Vim в поле ответа; steer и hidden shell paste; `$`-приложения до /apps; блокировка отправки при недоступной модели; startup draft.
- **Первый шаг:** rtwk G65 (каретка TextArea по метрикам строк), G66 (Ctrl+J), G78 (каретка isearch), G-new-1 (стиль по диапазону), G68 (таблица Autocomplete).
- **Зависит:** fw-ui-registry.
- **Строки (12):** Steer: Enter во время хода отправляет уточнение; Блокировка отправки при недоступной модели; Вставка пути к файлу превращается в элемент/картинку; Скрытая shell-вставка с /plan (hidden shell paste); $ skills / плагины / apps в упоминаниях; Картинки из буфера Ctrl+V / Alt+V; Vim: нормальный/вставка/операторы/текст-объекты/поиск (движо; Vim: replace/yank/dot-repeat; Startup draft (черновик из аргументов, file-search startup); Предупреждения о лимитах в карточке и ленте; Предупреждения (/warnings, F2): страницы, wrap; Ctrl+V / Alt+V — вставка картинки.
- **Тесты:** 82 pending (список — `tools/rtwk-port/map-full/unpend/composer-sweep.txt`).

### `ui-exec-mcp`

- **Что:** UI exploration, фоновые терминалы, /ps, инвентарь /mcp. **Волна:** W2; **размер:** 2 прогонов, ≈1000 строк.
- **Файлы (владеет):** `src/pages/session/cells/exec.rs`, `src/pages/session/cells/interaction.rs`, `src/pages/session/cells/tool_call.rs`, `src/pages/session/cells/tool_rows.rs`, `src/pages/session/cells/exploration.rs`, `src/pages/session/cells/processes.rs`, `src/pages/session/cells/mcp_inventory.rs`, `src/pages/session/status/terminals.rs`.
- **Oracle:** history_cell/exec.rs, exec_cell/, history_cell/mcp.rs, bottom_pane/unified_exec_footer.rs.
- **UI:** группа exploration, ячейка взаимодействия «waited for», список /ps, футер unified exec, ячейка инвентаря MCP.
- **Зависит:** fw-ui-registry, fw1-c5.
- **Строки (5):** Группы exploration (Read/List/Search свёрнуты в одну); Unified exec: фоновые терминалы, wait-streak, interaction-яч; Инвентарь MCP (вывод /mcp); /mcp; /ps — фоновые терминалы.
- **Тесты:** 20 pending (список — `tools/rtwk-port/map-full/unpend/ui-exec-mcp.txt`).

### `ui-safety-compaction`

- **Что:** UI safety buffering, misalignment, guardian, compaction. **Волна:** W2; **размер:** 1 прогонов, ≈650 строк.
- **Файлы (владеет):** `src/pages/session/cells/safety.rs`, `src/pages/session/cells/misalignment.rs`, `src/pages/session/cells/compaction.rs`, `src/sheets/approvals/guardian.rs`, `src/pages/session/live/provider_error.rs`.
- **Oracle:** chatwidget/{safety_buffering,compaction,misalignment_policy,provider_error}.rs, safety_recourse/.
- **UI:** повтор/подтверждение safety, остановка misalignment, ячейки guardian, таймер и ячейка compaction.
- **Зависит:** fw-ui-registry, fw1-c6.
- **Строки (5):** Compaction: статус, таймер, ячейка завершения; Safety buffering: «повторить», подтверждение, короткое сообщ; Misalignment/guardian stop треда; /compact; Guardian review (approved/denied/timed out, aggregate status.
- **Тесты:** 34 pending (список — `tools/rtwk-port/map-full/unpend/ui-safety-compaction.txt`).

### `ui-approvals-rest`

- **Что:** UI approvals и вопросы: решение в транскрипте, черновики, Vim в ответе, write_stdin. **Волна:** W2; **размер:** 2 прогонов, ≈1000 строк.
- **Файлы (владеет):** `rest:src/sheets/approvals`, `src/pages/session/cells/request_input.rs`, `src/pages/session/cells/decision.rs`, `core/src/screens/approvals/**`, `core/src/screens/approvals.rs`.
- **Oracle:** bottom_pane/{approval_overlay,request_user_input,async_questions,mcp_server_elicitation}*, history_cell/{approvals,request_user_input,async_questions}.rs.
- **UI:** ячейка решения (Approved/Denied, усечение, network-allow), apply_patch и write_stdin, черновики вопросов, пустые ответы, очередь вопросов.
- **Первый шаг:** rtwk G8 (сброс typeahead перед approval), G72 (автоссылка в обёрнутом URL).
- **Зависит:** fw-ui-registry, fw1-c9.
- **Строки (10):** Решение approval в транскрипте (Approved/Denied + команда); Approval apply_patch: модал, call_id, заголовок; Approval network host (разово/сессия); Approval write_stdin; Вопросы: черновики, навигация, replay черновиков; Вопросы: поиск по истории, Vim в поле ответа; Асинхронные вопросы (desktop answer, уведомления); Отклонение пустых ответов, явный skip; Разворот вопроса уступает мышь терминалу для ссылок; Очередь вопросов и queued messages делят один shortcut.
- **Тесты:** 59 pending (список — `tools/rtwk-port/map-full/unpend/ui-approvals-rest.txt`).

### `ui-usage-limits`

- **Что:** UI /usage: меню и reset, billing, подсказка на старте; /status: usage треда, строка сервера, предупреждения; страница аналитики. **Волна:** W2; **размер:** 2 прогонов, ≈1500 строк.
- **Файлы (владеет):** `src/pages/usage/**`, `src/pages/session/live/usage_menu.rs`, `src/pages/session/live/limits.rs`, `src/pages/session/cells/status/**`, `core/src/screens/status/**`, `core/src/screens/status.rs`, `host/src/analytics/**`, `host/src/analytics.rs`, `core/src/data/usage.rs`, `src/services/history/usage.rs`.
- **Oracle:** chatwidget/{usage,reset_credits,usage_notice,thread_usage,rate_limits}.rs, status/{card,rate_limits}.rs, analytics/.
- **UI:** пункты меню Analytics/Refresh/Reset, picker и confirm, подсказка на старте, строки карточки, ленточные предупреждения; 43 теста usage-шарда.
- **Первый шаг:** rtwk G-new-usage-1 (`ensure_visible` в `State`), G-new-usage-2 (`visible_range`), G75/G76 (перенос UAX #14 и optimal-fit).
- **Зависит:** fw-ui-registry, fw1-c7.
- **Строки (15):** Rate-limit подсказки/предупреждения во время хода; /status — карточка статуса; /usage; /statusline; Reset-limit picker и подтверждение (usage menu); Карточка /status: usage треда; Карточка /status: строка сервера; Подсказка reset-лимита на старте; Страница аналитики: бары, вкладки, часы плана, narrow-график; Меню /usage: Analytics, Refresh, Reset; Reset лимита: пикер, подтверждение, idempotency-ключ, in-fli; Billing и enterprise-планы; Подсказка reset на старте; Статус после billing retries; Ранний redraw и повтор thread usage.
- **Тесты:** 60 pending (список — `tools/rtwk-port/map-full/unpend/ui-usage-limits.txt`).

### `ui-settings-feeds`

- **Что:** UI /experimental, /skills, /hooks, аргументные формы, memories. **Волна:** W2; **размер:** 1 прогонов, ≈550 строк.
- **Файлы (владеет):** `rest:src/features/settings`.
- **Oracle:** bottom_pane/{experimental_features_view,skills_toggle_view,hooks_browser_view,memories_settings_view,status_line_setup,title_setup}.rs, chatwidget/hooks.rs.
- **UI:** подключить готовые sheet features/setup/hooks к фактам fw1-c3; split `src/features/settings/mod.rs` на файл по `Which` (заглушки `config_panel.rs`, `tui_mode.rs`, `keymap_editor/`).
- **Первый шаг:** шаг 1 — расщепить `mod.rs` настроек: без этого ui-config-panel и ui-keymap-editor правят его вместе.
- **Зависит:** fw-ui-registry, fw1-c3.
- **Строки (11):** Команды с inline-аргументами (/plan x, /goal x, /review x, /; /experimental — feature-флаги; /memories; /skills; /hooks; Hooks popup: список, диагностика; Experimental features popup (toggle сохраняется на выходе); Memories settings и reset; Manage skills summary; Current settings (model/permissions/status layout); Settings: Theme, Memories (из чата).
- **Тесты:** 5 pending (список — `tools/rtwk-port/map-full/unpend/ui-settings-feeds.txt`).

### `ui-config-panel`

- **Что:** UI /config и /tui (панель настроек, выбор inline/fullscreen). **Волна:** W2; **размер:** 1 прогонов, ≈800 строк.
- **Файлы (владеет):** `src/features/settings/config_panel/**`, `src/features/settings/config_panel.rs`, `src/features/settings/tui_mode.rs`.
- **Oracle:** config_panel/, chatwidget/config_panel.rs, bottom_pane/config_panel.rs, chatwidget/tui_mode_picker.rs, app/tui_mode_picker.rs.
- **UI:** панель с вкладками и под-пикерами, выбор режима TUI на следующий запуск.
- **Зависит:** ui-settings-feeds, fw1-c3.
- **Строки (3):** /config; /tui — быстрые настройки TUI; Settings: Config / Statusline / Title / Experimental / Skill.
- **Тесты:** —.

### `ui-keymap-editor`

- **Что:** UI /keymap: пикер, захват клавиши, отладка, конфликты. **Волна:** W2; **размер:** 1 прогонов, ≈700 строк.
- **Файлы (владеет):** `src/features/settings/keymap_editor/**`, `core/src/keymap/**`, `core/src/keymap.rs`.
- **Oracle:** keymap_setup/{picker,capture,debug}.rs, chatwidget/keymap_picker.rs, keymap.rs (runtime: chords, leader, conflicts).
- **UI:** редактор клавиш; leader Ctrl+X, зарезервированные клавиши, конфликты.
- **Зависит:** ui-settings-feeds, fw1-c3.
- **Строки (2):** /keymap — редактор клавиш; Ctrl+X — leader (Ctrl+X b — смена бэкенда).
- **Тесты:** 4 pending (список — `tools/rtwk-port/map-full/unpend/ui-keymap-editor.txt`).

### `ui-openers-side`

- **Что:** UI /presets, /daemon, /subagents, app link, /side и /btw. **Волна:** W2; **размер:** 2 прогонов, ≈850 строк.
- **Файлы (владеет):** `src/sheets/select/daemon.rs`, `src/sheets/select/agent_picker.rs`, `rest:src/sheets/select/quick`, `src/sheets/select/quick.rs`, `src/sheets/select/app_link/**`, `src/sheets/select/app_link.rs`, `src/pages/session/open/daemon.rs`, `src/pages/session/open/agent_picker.rs`, `src/features/side/**`, `src/pages/session/key_actions/side.rs`.
- **Oracle:** app/daemon_menu.rs, app/multi_agents.rs, chatwidget/quick_dials.rs, bottom_pane/app_link_view.rs, app/side*.rs.
- **UI:** листы daemon и agent picker, открыть quick, side-тред и его подпись; Ctrl+/.
- **Зависит:** fw-ui-registry, fw1-c1, fw1-c2.
- **Строки (10):** Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись ро; /presets — выбор пресета (Daybreak presets); /side, /btw; /daemon; /subagents (multi-agents picker); Quick dials (saved dials: save/clear picker); Daemon (подключение к daemon); Ссылка на приложение (app link) диалог; Model provider switch (блокирует кэш выбора); Ctrl+/ (Ctrl+_, Ctrl+7) — side.
- **Тесты:** 9 pending (список — `tools/rtwk-port/map-full/unpend/ui-openers-side.txt`).

### `ui-worktree-sandbox`

- **Что:** UI worktree/checkout, «Forked thread», Windows sandbox, security setup, full-access confirm. **Волна:** W2; **размер:** 2 прогонов, ≈800 строк.
- **Файлы (владеет):** `rest:src/sheets/select/worktree`, `src/sheets/select/worktree.rs`, `src/sheets/select/permissions.rs`, `src/sheets/select/confirm.rs`, `src/features/security_setup/**`, `src/pages/session/cells/forked.rs`.
- **Oracle:** chatwidget/worktree_picker.rs, windows_sandbox_prompts.rs, security_setup.rs, history_cell/session.rs (forked).
- **UI:** вид checkout-выбора и цепочка старта, строка «Forked thread», подтверждения sandbox.
- **Зависит:** fw-ui-registry, fw1-c8.
- **Строки (8):** Строка «Forked thread» / имя треда в истории; /setup-default-sandbox (Windows); /new; /fork; /worktree; Windows sandbox: prompts, help-ссылки, fallback; Worktree/checkout picker; Security setup / security prompts.
- **Тесты:** 26 pending (список — `tools/rtwk-port/map-full/unpend/ui-worktree-sandbox.txt`).

### `ui-feedback-cmds`

- **Что:** UI /feedback (note, consent), /recap, /debug-config, /logout, /ide, /daybreak. **Волна:** W2; **размер:** 1 прогонов, ≈900 строк.
- **Файлы (владеет):** `src/sheets/dialogs/feedback.rs`, `rest:src/features/feedback`, `rest:src/features/recap`, `rest:src/features/small_cmds`, `src/pages/session/cells/recap.rs`, `src/pages/session/cells/debug_config.rs`.
- **Oracle:** bottom_pane/{feedback_view,feedback_note_view}.rs, app/feedback_upload.rs, history_cell/notices.rs (recap), debug_config.rs.
- **UI:** страницы note/consent, ячейки recap и debug-config, заметки logout/ide/daybreak.
- **Зависит:** fw-ui-registry, fw1-c4, fw1-c9.
- **Строки (9):** /daybreak; /ide — контекст IDE; /recap; /debug-config; /logout; /feedback; /debug-m-drop, /debug-m-update; Feedback: выбор, consent upload, диагностика, note; Feedback (диалог).
- **Тесты:** 29 pending (список — `tools/rtwk-port/map-full/unpend/ui-feedback-cmds.txt`).

### `cells-sweep`

- **Что:** ячейки и снимки: collab, картинки, Worked for, гиперссылки, plan/patch/hook, cells_1-5. **Волна:** W2; **размер:** 2 прогонов, ≈1400 строк.
- **Файлы (владеет):** `rest:src/pages/session/cells`, `rest:core/src/threads`, `core/src/screens/cells/**`, `core/src/screens/cells.rs`.
- **Oracle:** history_cell/{base,messages,patches,plans,search,separators,session,notices,hook_cell,delegated_message,dynamic}.rs, exec_cell/.
- **UI:** 210 pending cells_* и renderer_internals; ячейки collab, image, separator, user, plan; resume-replay.
- **Первый шаг:** rtwk G72 (автоссылка в Text), G-tool-bullet-pulse, G-exec-user-shell.
- **Зависит:** fw-ui-registry.
- **Строки (4):** Ячейка exec: команда, вывод, статус, длительность; Ячейка просмотра/генерации изображений; Ячейка collab/sub-agent (spawn, wait, send, модель/effort); Final «Worked for …» разделитель с длительностью.
- **Тесты:** 56 pending (список — `tools/rtwk-port/map-full/unpend/cells-sweep.txt`).

### `markdown-gaps`

- **Что:** Markdown: math, mermaid, soft break, отступ списков, ThemeExtension, inline-синтаксис, dim-стиль. **Волна:** W2; **размер:** 1 прогонов, ≈700 строк.
- **Файлы (владеет):** `src/widgets/markdown.rs`, `src/widgets/markdown/**`.
- **Oracle:** markdown_render.rs, markdown_render/{math,mermaid}.rs, markdown_stream.rs, streaming/.
- **UI:** после rtwk: настройки `MarkdownView` и рендереры; снять 112 pending markdown_streaming_1-5.
- **Первый шаг:** rtwk: G61 (math), G62 (mermaid), G63 (soft break), G64 (отступ вложенных списков), G71 (унаследованный стиль), G73 (ThemeExtension / Markdown layout options), G60 (inline-синтаксис).
- **Зависит:** rtwk.
- **Строки (1):** Ответ агента: стриминг markdown, списки, таблицы, код, ссылк.
- **Тесты:** 112 pending (список — `tools/rtwk-port/map-full/unpend/markdown-gaps.txt`).

### `dialogs-sweep`

- **Что:** малые диалоги и пикеры: шорткаты «?», /warnings, тап по строке, user verification, model/permissions/apps/review хвосты. **Волна:** W2; **размер:** 2 прогонов, ≈1200 строк.
- **Файлы (владеет):** `rest:src/sheets/dialogs`, `rest:src/widgets`, `rest:core/src/screens/select`, `core/src/screens/select.rs`, `core/src/screens/dialogs.rs`, `src/sheets/dialogs/user_verification.rs`, `rest:core/src/data`.
- **Oracle:** bottom_pane/{list_selection_view,shortcut_overlay,warnings_view,user_verification,multi_select_picker,custom_prompt_view}.rs, chatwidget/{model_popups,permission_popups,connectors,review_popups}.rs.
- **UI:** small_dialogs_1/2 (58 pending), picker/chat_popups хвосты, refresh каталога моделей, reasoning submenu.
- **Первый шаг:** rtwk G44 (halfwidth katakana), G56 (имя+описание одним абзацем).
- **Зависит:** fw-ui-registry.
- **Строки (14):** /model — выбор модели и effort; /permissions — approvals/профили; /goal [цель/pause/resume/clear]; /plugins; Model picker: каталог, discovery, bedrock/custom display, se; Permissions popup: пресеты, профили, retry discovery; Goal: меню и edit prompt, resume-paused prompt; Collaboration / mode catalog; Plugins: список, вкладки, поиск, деталь, install, marketplac; Apps/connectors: список, статус, install-only; Sweep: прочие мелкие popup'ы; Шорткаты: оверлей «?» со всеми действиями; Верификация пользователя (user verification); Список выбора (SelectionList): тап по строке.
- **Тесты:** 53 pending (список — `tools/rtwk-port/map-full/unpend/dialogs-sweep.txt`).

### `footer-status-sweep`

- **Что:** футер, строка статуса, /statusline, /title, настройка status line; pane_footer и chat_status. **Волна:** W2; **размер:** 2 прогонов, ≈1500 строк.
- **Файлы (владеет):** `rest:src/pages/session/footer`, `rest:src/pages/session/status`, `src/pages/session/status_surfaces.rs`, `src/pages/session/window_title.rs`, `src/widgets/status_line.rs`.
- **Oracle:** bottom_pane/{footer,status_line,status_line_setup,title_setup,effort_ignition,shortcut_hud}*.rs, status_indicator_widget.rs, chatwidget/status_surfaces.rs, terminal_title.rs.
- **UI:** 183 pending статус-шардов (pane_footer_1/2, chat_status_1-3); Cwd/Clock/git/headline от host; подпись side-контекста.
- **Зависит:** fw-ui-registry, fw1-c3, fw1-c2.
- **Строки (5):** Часы/время в уведомлениях (clock facts); /title — терминальный title; Настройка статус-строки (/statusline, порядок элементов); Терминальный заголовок (title) с элементами; Подпись контекста side-разговора.
- **Тесты:** 148 pending (список — `tools/rtwk-port/map-full/unpend/footer-status-sweep.txt`).

### `live-turn-sweep`

- **Что:** живой ход: баннеры, backtrack, plan mode, review/fork при MCP startup; chat_live_1-6. **Волна:** W2; **размер:** 2 прогонов, ≈1300 строк.
- **Файлы (владеет):** `rest:src/pages/session/live`, `src/pages/session/banners.rs`, `src/pages/session/backtrack/**`, `src/pages/session/blossom/**`, `core/src/screens/backtrack.rs`.
- **Oracle:** chatwidget/{turn_lifecycle,interrupts,reconnect,mcp_startup,plan_implementation,review}.rs, app_backtrack.rs, empty_state_animation.rs.
- **UI:** остаток chat_live-шардов (≈250 pending вне пакетов фич); steer/Esc, plan reasoning scope, luna reserve.
- **Зависит:** fw-ui-registry.
- **Строки (2):** Резерв Luna: селектор и статус; Empty-state animation.
- **Тесты:** 46 pending (список — `tools/rtwk-port/map-full/unpend/live-turn-sweep.txt`).

### `voice`

- **Что:** voice/realtime: /voice, F8, voice strip, настройки, split-flap, session lock. **Волна:** W3; **размер:** 3 прогонов, ≈1900 строк.
- **Файлы (владеет):** `src/sheets/voice/**`, `core/src/screens/voice.rs`, `core/src/features/voice/**`, `src/features/voice/**`, `host/src/audio.rs`, `src/pages/session/open/voice.rs`, `src/pages/session/key_actions/voice.rs`, `src/pages/session/footer/voice.rs`, `src/pages/session/cells/split_flap.rs`.
- **Oracle:** bottom_pane/voice_strip.rs, chatwidget/realtime*.rs, realtime_call.rs, app/realtime_*.rs, session_lock.rs.
- **Ядро / шаги:** состояние realtime, `VoiceControl`, факты звука.
- **UI:** strip, лист настроек, split-flap ячейка.
- **Первый шаг:** шаг host-audio: решение человека о крейте захвата/воспроизведения и о доступе к realtime API; потом host/src/audio.rs.
- **Зависит:** fw-ui-registry, fw-core-hubs (Ask::Audio).
- **Строки (5):** /voice; Индикатор голоса (voice strip), F8; Voice (диалог, F8); F8 — голос; OS session lock.
- **Тесты:** 1 pending (список — `tools/rtwk-port/map-full/unpend/voice.txt`).

### `sidebar`

- **Что:** sidebar: документы, файлы, ссылки, chooser, анимация. **Волна:** W3; **размер:** 2 прогонов, ≈1300 строк.
- **Файлы (владеет):** `core/src/features/sidebar/**`, `src/features/sidebar/**`, `src/pages/session/sidebar/**`, `src/sheets/select/sidebar_chooser.rs`, `src/pages/session/cells/file.rs`.
- **Oracle:** app/sidebar*.rs, sidebar_chooser.rs, sidebar_tab_strip.rs, app/sidebar_document.rs.
- **Ядро / шаги:** факты sidebar (документы, загрузка).
- **UI:** панель рядом с транскриптом, chooser, ячейка file.
- **Первый шаг:** rtwk: раскладка панели рядом с транскриптом со скользящей анимацией (SplitPane/AnimatedSize), если её нет.
- **Зависит:** fw-ui-registry.
- **Строки (1):** Сайдбар (панель агентов/тредов).
- **Тесты:** —.

### `inline-mode`

- **Что:** inline-режим S27: viewport, вставка в scrollback, reflow, временный alt-screen. **Волна:** W3; **размер:** 2 прогона, ≈1500 строк (человек 2026-11-24: rtwk уже поддерживает inline).
- **Файлы (владеет):** `src/app/run.rs`, `src/app/mod.rs`, `src/app/screen_mode/**`, `src/app/scrollback/**`, `src/services/transcript_file.rs`.
- **Oracle:** tui.rs, insert_history.rs, app/resize_reflow.rs, custom_terminal*, transcript_reflow.rs, app/native_history.rs.
- **UI:** ScreenMode из `tui.fullscreen_transcript`, committed scrollback, снимки cells_4/5 и transcript_view_3 (34 n/a).
- **Первый шаг:** взять готовое из rtwk: `ScreenMode::Inline` (rtwk-core/src/presenter/modes.rs:63), `binding::Static` — готовый вывод над приложением, в истории терминала (rtwk docs/widgets/app/static.md), образец inline-агента `rtwk-e2e/src/agent.rs:624 inline_agent_app` и `rtwk-e2e/src/modes.rs:74`. Чего в rtwk нет (бывшие G1/G2/G5/G7: смена alt-screen на ходу, reflow committed-строк, OSC 8/картинки в них) — проверить по этим образцам одной строкой разрыва, без rtwk-работы вперёд.
- **Зависит:** fw-core-hubs (Ask::ScreenMode).
- **Строки (6):** Ячейка MCP-вызова (running/ok/failed, результат); Inline-режим (без alt-screen); Inline viewport; Вставка завершённых строк в scrollback; Resize reflow; Временный alt-screen для оверлеев.
- **Тесты:** 12 pending (список — `tools/rtwk-port/map-full/unpend/inline-mode.txt`).

### `ui-tree-viz`

- **Что:** ui_tree и inline visualization. **Волна:** W3; **размер:** 2 прогонов, ≈1100 строк.
- **Файлы (владеет):** `core/src/features/ui_tree/**`, `src/features/ui_tree/**`, `src/widgets/ui_tree/**`, `core/src/threads/inline_vis.rs`.
- **Oracle:** ui_tree.rs, ui_tree/, inline_visualization.rs, inline_visualization/.
- **Ядро / шаги:** разбор модельного декларативного UI, директивы inline-vis (сейчас курсив-заметка).
- **UI:** элементы status line и просмотр visualization.
- **Первый шаг:** решение человека: что делать с viewer-HTML (host пишет файл, ссылка file://).
- **Зависит:** fw-core-hubs.
- **Строки (1):** Дерево UI (ui-tree).
- **Тесты:** —.

### `pets-ambient`

- **Что:** pets: ambient-питомец, asset pack, протоколы изображений. **Волна:** W3; **размер:** 2 прогонов, ≈1200 строк.
- **Файлы (владеет):** `src/sheets/pets/**`, `core/src/screens/pets.rs`, `src/widgets/pets/**`, `host/src/pets.rs`.
- **Oracle:** pets/{ambient,asset_pack,catalog,frames,image_protocol,sixel,preview,picker}.rs.
- **Ядро / шаги:** каталог и состояние питомца как факты host.
- **UI:** ambient в футере и транскрипте, превью.
- **Первый шаг:** rtwk G4 (картинки внутри региона, kitty/sixel/iTerm2), G10 (tmux pane-local graphics).
- **Зависит:** rtwk.
- **Строки (4):** Ambient pet в транскрипте/футере; /pets, /pet; Pets (диалог выбора); Inline images (kitty / sixel / iTerm2).
- **Тесты:** —.

### `startup-rest`

- **Что:** запуск и onboarding: хвосты S01-S06 и startup-шарды, daemon recovery, startup error, cloud environment, провайдеры. **Волна:** W3; **размер:** 2 прогонов, ≈1300 строк.
- **Файлы (владеет):** `src/pages/startup/**`, `core/src/screens/startup.rs`, `core/src/screens/startup/**`, `core/src/screens/onboarding.rs`, `core/src/screens/onboarding/**`, `core/src/screens/import/**`, `core/src/protocol/onboarding.rs`, `core/src/protocol/import.rs`, `src/app/start_page.rs`, `host/src/startup.rs`, `host/src/launch.rs`, `host/src/gateway.rs`, `host/src/providers.rs`, `host/src/update.rs`, `host/src/import.rs`, `host/src/cloud.rs`, `host/src/target.rs`, `src/services/startup.rs`, `src/services/backend/**`, `src/services/remote.rs`.
- **Oracle:** onboarding/, startup_*.rs, cwd_prompt.rs, model_migration.rs, oss_selection.rs, daemon_recovery.rs, update_prompt.rs, unarchive_prompt.rs, external_agent_config_migration/.
- **UI:** 6 pending startup-шардов; startup error screen; cloud environment picker; provider editor.
- **Первый шаг:** rtwk: G48 (перенос только по пробелам), G49 (после `/`), G50/G51 (приоритет flex-детей `Column`), G52 (путь по сегментам), G55 (последний символ секрета).
- **Зависит:** rtwk для G48-G55; остальное не зависит.
- **Строки (6):** Trust on open (запрос доверия к каталогу); Startup error и daemon recovery; Multi-backend; Update check; Provider editor и cloud environment; Onboarding: sign in, gateway, Bedrock, GCP.
- **Тесты:** 13 pending (список — `tools/rtwk-port/map-full/unpend/startup-rest.txt`).

### `agents-center`

- **Что:** agents command center (/agents): остаток и снимки. **Волна:** W3; **размер:** 1 прогонов, ≈800 строк.
- **Файлы (владеет):** `src/pages/agents/**`, `core/src/screens/agents/**`, `src/services/agents/**`, `core/src/protocol/agents.rs`.
- **Oracle:** app/agents_overview*.rs, app/agent_center/, app/agent_navigation.rs, app/agent_status_feed.rs.
- **UI:** страница есть; ячейка agent status feed; навигация из composer (Left); подпись `⌥↑` — сверить с oracle.
- **Зависит:** fw-ui-registry.
- **Строки (1):** Agents command center.
- **Тесты:** —.

### `resume-rest`

- **Что:** resume/fork picker: хвосты и превью транскрипта. **Волна:** W3; **размер:** 1 прогонов, ≈600 строк.
- **Файлы (владеет):** `src/pages/resume/**`, `core/src/screens/resume/**`, `core/src/screens/resume.rs`, `src/widgets/session_list.rs`.
- **Oracle:** resume_picker.rs, resume_picker/, resume_picker_transcript_preview.rs, named_session_lookup.rs.
- **UI:** 12 pending resume-шардов: хранилище плотности, thread_to_transcript_cells, превью через pager.
- **Первый шаг:** rtwk: шаблон имён кадров в kit (см. kit-seeds); pager из ui-pager-find.
- **Зависит:** ui-pager-find, kit-seeds.
- **Строки (1):** /resume.
- **Тесты:** 5 pending (список — `tools/rtwk-port/map-full/unpend/resume-rest.txt`).

### `terminal-features`

- **Что:** терминал и рантайм: Ctrl+Z, Ctrl+L, screen reader, reduced motion, уведомления, тема. **Волна:** W3; **размер:** 1 прогонов, ≈900 строк.
- **Файлы (владеет):** `src/app/terminal_effects.rs`, `src/app/view.rs`, `src/app/theme.rs`, `src/app/theme/**`, `src/services/clipboard/**`, `src/services/clipboard.rs`, `src/services/open_url.rs`, `host/src/main.rs`, `host/src/viewer.rs`, `src/pages/session/key_actions/clear_terminal.rs`.
- **Oracle:** tui/job_control.rs, screen_reader*.rs, system_motion.rs, motion.rs, notifications/, terminal_probe/, clipboard_copy/, clipboard_paste/.
- **UI:** Ctrl+Z suspend, Ctrl+L, режим screen reader, reduced motion, настройка уведомлений (OSC 9 / BEL), probe цветов.
- **Первый шаг:** rtwk: suspend/SIGTSTP и возврат (`App::suspend`), событие reduced-motion.
- **Зависит:** fw-ui-registry, fw-core-hubs (Ask::Suspend/Notify/Probe).
- **Строки (9):** Уведомления терминала о завершении хода / approval; Ctrl+L — очистить терминал; Ctrl+Z — suspend; Suspend / resume; Focus, synchronized output, Windows console, panic hook; Terminal probe (OSC 10/11, DA1); tmux / WSL / screen; Screen reader; Reduced motion.
- **Тесты:** —.

### `app-misc`

- **Что:** остальное приложения: exit summary, session log, turn tips, MCP startup, reconnect, read-only, quit-confirm. **Волна:** W3; **размер:** 1-2 прогонов, ≈1100 строк.
- **Файлы (владеет):** `core/src/screens/chat.rs`, `core/src/screens/chat/**`, `core/src/screens/tips.rs`, `core/src/features/exit_summary/**`, `src/features/exit_summary/**`, `host/src/session_log.rs`.
- **Oracle:** app/exit_summary.rs, session_log.rs, turn_tip.rs, chatwidget/{mcp_startup,reconnect,read_only,provider_error}.rs, tooltips.rs, app/server_version_notice.rs.
- **Ядро / шаги:** exit summary (токены + подсказка resume), session log.
- **UI:** диалоги quit-confirm и task-running.
- **Зависит:** fw-core-hubs, fw-ui-registry.
- **Строки (4):** Exit summary; Server version notice; Quit-confirm и task-running; Session log.
- **Тесты:** —.

### `kit-seeds`

- **Что:** обвязка oracle-тестов: семена kit, имена кадров, хранилище настроек. **Волна:** W2; **размер:** 1-2 прогонов, ≈1400 строк.
- **Файлы (владеет):** `tests/kit/**`, `tests/oracle/fixtures/**`, `tests/support/**`.
- **Oracle:** —.
- **UI:** семена: keymap-семя, хранилище view-mode, entry для make_elicitation_request, имена кадров склеенного снимка, Screen::feed в entry-экраны; снимает pending «no seed»/«kit names the frames» (≈160 тестов).
- **Зависит:** —.
- **Строки (1):** Keymap-seeds для approval.
- **Тесты:** 220 pending (список — `tools/rtwk-port/map-full/unpend/kit-seeds.txt`).

## Строки

### Транскрипт и ячейки

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Сообщение пользователя (блок с фоном, пути, картинки-плейсхолдеры) | ✅ | Фон блока prompt по теме (?) | history_cell/messages.rs | src/pages/session/cells/user.rs | — |
| Ответ агента: стриминг markdown, списки, таблицы, код, ссылки | 🟡 | landed c4c2af4e0: настройки `[tui.rendering]` (таблицы, mermaid, списки) читаются там, где строится сообщение; при выключении источник сохраняется (5 ключевых тестов markdown_streaming_2/5 зелёные). Нет: math off только флаг (G-math-off, G61), маркер списка `•` вместо `- ` (G-list-marker), `user.rs`/reasoning/resume рисуют markdown без настройки, lists_preserve_layout (G73) и MATH-тесты (G61/G62) — sweep; часть markdown-рендера ждёт (счёт не пересчитан) | markdown_render.rs, streaming/ | src/widgets/markdown.rs, src/pages/session/cells/markdown.rs | `markdown-gaps` |
| Подсветка кода и diff в ответах | ✅ | — | markdown_render.rs, render/highlight.rs | src/widgets/markdown.rs | — |
| Гиперссылки в тексте (клик открывает) | 🟡 | обычный клик по ссылке красный (ordinary_click_opens_links), OSC-8 в активной ячейке ждёт | terminal_hyperlinks.rs, transcript_view/selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | landed a15c35bf8: сводка live и в истории, raw reasoning по show_raw_agent_reasoning, пустые блоки (7 тестов зелёные) | history_cell/messages.rs | src/pages/session/cells/reasoning.rs | — |
| Ячейка exec: команда, вывод, статус, длительность | 🟡 | компактная форма как у oracle (afd840c6e; non-zero exit — bold-буллет); нет: пульс буллета running (G-tool-bullet-pulse), форма `!cmd` с 50 строками (G-exec-user-shell), scrollback-снимки ждут inline-режима S27 | history_cell/exec.rs, exec_cell/ | src/pages/session/cells/exec.rs | `cells-sweep` |
| Группы exploration (Read/List/Search свёрнуты в одну) | ❌ | группировка, прерывание, нулевой exit, порядок с reasoning (≈20 тестов) | history_cell/exec.rs, exec_cell/model.rs | src/pages/session/cells/exploration.rs (нет) | ядро: fw1-c5 (идёт); `ui-exec-mcp` |
| Unified exec: фоновые терминалы, wait-streak, interaction-ячейка | ❌ | ячейка взаимодействия, «waited for», сохранение при interrupt | history_cell/exec.rs, bottom_pane/unified_exec_footer.rs | src/pages/session/cells/interaction.rs | ядро: fw1-c5 (идёт); `ui-exec-mcp` |
| Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | пустой/failed-patch формат, ручное approval-заголовок (тесты ждут) | history_cell/patches.rs, diff_render.rs | src/pages/session/cells/patch.rs | — |
| Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | в чате совпадает с oracle (pty, afd840c6e); снимки cells_4/5 — scrollback-форма, ждут inline-режима S27; пульс буллета ❌ | history_cell/mcp.rs | src/pages/session/cells/tool_call.rs | `inline-mode` |
| Инвентарь MCP (вывод /mcp) | ❌ | ячейка и данные инвентаря | history_cell/mcp.rs | src/pages/session/cells/mcp_inventory.rs (нет) | ядро: fw1-c5 (идёт); `ui-exec-mcp` |
| Ячейка web search (запрос, детали, усечение) | ✅ | строка в чате и обёрнутая форма в Ctrl+T (afd840c6e) | history_cell/dynamic.rs | src/pages/session/cells/web.rs | — |
| Ячейка просмотра/генерации изображений | 🟡 | image_generation_call_adds_history_cell и просмотр: рендер есть (cells/viewed_image, generated_image), снимки ждут | history_cell/base.rs, transcript_image.rs | src/pages/session/cells/viewed_image.rs, src/pages/session/cells/generated_image.rs | `cells-sweep` |
| Ячейка плана (update_plan) и предложенный план | ✅ | — | history_cell/plans.rs | src/pages/session/cells/plan.rs, src/pages/session/cells/plan_update.rs | — |
| Ячейка collab/sub-agent (spawn, wait, send, модель/effort) | 🟡 | spawn_end с моделью, wait-элементы, повторный рендер sub-agent | history_cell/dynamic.rs, multi_agents.rs | src/pages/session/cells/collab.rs | `cells-sweep` |
| Ячейка delegated (делегированный вывод tool) | ✅ | — | history_cell/delegated_message.rs | src/pages/session/cells/delegated.rs | — |
| Ячейка hook (запущен/завершён) | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Решение approval в транскрипте (Approved/Denied + команда) | ❌ | decision cell: усечение многострочных команд, network-allow, canceled host | history_cell/approvals.rs | src/pages/session/cells/decision.rs (нет) | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Результат вопроса request_user_input в транскрипте | ✅ | landed a15c35bf8: ячейка ответа/прерывания (completed_and_interrupted_results); markdown из истории — sweep | history_cell/request_user_input.rs | src/pages/session/cells/request_input.rs | — |
| Ячейка ошибки (upstream, cyber policy, overloaded, safety access) | ✅ | chained config error wrap (pending) | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Предупреждения (warning, sqlite, config, guardian, model verification) | ✅ | wrap уведомления в narrow, notice_details_fit | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Deprecation / update-available / version-notice | ✅ | update-available и server-version от host (e263606f0); server-version дедуп в пределах запуска (oracle хранит last_shown) | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Часы/время в уведомлениях (clock facts) | 🟡 | Clock и LinkDisplay от host (host/src/viewer.rs, e263606f0); RuntimeMetrics без источника (G-runtime-metrics → host-счётчики, core-live-debts) | clock_format.rs | core/src/format.rs | ядро: fw1-c3, fw1-c2 (идёт); `footer-status-sweep` |
| Session info: шапка сессии (модель, cwd, подсказки, tooltips) | ✅ | landed 87ad7733c: карточка шапки, каталог подсказок и выбор tip-строки, model-changed notes, стартовые факты от host | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Строка «Forked thread» / имя треда в истории | ❌ | fork-history line с именем/id, prompt-edit history line | history_cell/session.rs | src/pages/session/cells/forked.rs (нет) | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| Review-ячейки (баннер Current changes, request hint, prompt скрыт) | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Compaction: статус, таймер, ячейка завершения | ❌ | timer, snapshot restore, clear on turn end | chatwidget/compaction.rs | src/pages/session/cells/compaction.rs (нет) | ядро: fw1-c6 (идёт); `ui-safety-compaction` |
| Final «Worked for …» разделитель с длительностью | 🟡 | cumulative duration snapshot ждёт; разделитель есть (cells/separator) | history_cell/separators.rs | src/pages/session/cells/separator.rs | `cells-sweep` |
| Переключатель плотности (compact/detailed) ячеек | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Сворачивание деталей хода (collapse_turn_details), раскрытие групп | 🟡 | сворачивание, сводки групп, раскрытие кликом/Enter/Space (16ed6526f); нет: ширинно-зависимый счёт скрытых строк exec, хранение 50+50 строк вывода, TurnTip, закреплённая шапка prompt (G-new-usage-2) | transcript_view/turn_details.rs, disclosure.rs | src/pages/session/transcript/turn_details.rs | `ui-transcript-chrome` |
| Фокус на activity-группах (F4) и подсказка раскрытия | ❌ | focus_activity целиком | transcript_view/activity.rs, history_cell/activity_group.rs | src/pages/session/transcript_activity.rs | `ui-transcript-chrome` |
| Автопрокрутка за хвостом, «вернуться вниз», строка новой активности | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | Home/End в ленте; скорость колеса (mouse_scroll_speed) не применяется | transcript_view/input.rs, tui/event_stream.rs | src/pages/session/transcript/follow.rs | `ui-transcript-chrome` |
| Подгрузка старых страниц истории при прокрутке вверх | 🟡 | landed ee4c83bd9: статус-строка (loading / refused с retry / loadable, 4 ширины) из `transcript/history.rs`, join страницы не двигает читателя, 9 page-тестов, 4 oracle-теста зелёные; pty: тот же финальный экран. Нет: клавиши ctrl+home / alt+< (идти к началу, retry) и ctrl+end / alt+> (Need: session_page.rs); склейка групп oracle при join (page_join_* в transcript_view_3, sweep); join при читателе в 1-3 строках ниже первой сдвигает вид на строку; один тик колеса двигает много больше строк, чем у oracle | app/history_pagination.rs, transcript_view/mutations.rs | src/pages/session/transcript/history.rs | `ui-transcript-chrome` |
| Закреплённая шапка prompt над транскриптом (pinned header) | ❌ | нет модуля prompt_header | transcript_view/prompt_header.rs | src/pages/session/transcript_header.rs | `ui-transcript-chrome` |
| Replay истории при resume/fork: сообщения, tool, reasoning, review | ✅ | resumed_initial_messages_render_history, nested review prompts — ждут | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Ambient pet в транскрипте/футере | 🟡 | pets sheet есть; ambient-тесты chat_status_1 не включены | pets/ambient.rs | src/sheets/pets/ | `pets-ambient` |
| Фоновая «starfield»/sparkle эффекты композера | 🟡 | задача занята; 12 тестов не зелёные | bottom_pane/chat_composer/sparkle.rs, sparkle_field.rs | src/widgets/composer_sparkle.rs | ядро: fw1-c2 (идёт); `ui-composer-states` |

### Живой ход (стриминг, прерывание, очередь, steer)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Отправка сообщения Enter, индикатор working/«Thinking», таймер хода | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Прерывание хода Esc | ✅ | Esc с ожидающими steer: отправка всех сразу и сохранение (тест ждёт) | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Ctrl+C: прерывает ход, второй раз выходит; «armed quit» подсказка | ✅ | Ctrl+C при caps lock (n/a) | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+D на пустом composer выходит (когда idle) | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Очередь сообщений (Tab / Enter в ходе) над composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Очередь slash- и shell-команд во время хода | ❌ | ядро готово (`core/src/threads/queue.rs`: `Queued` Message/Literal/Command/Shell, `DueCommand`, `QueueRan`/`QueueResume`); нет вида: Tab ставит строку команды, запуск при drain, !shell в очереди, отклонение при MCP startup | chatwidget/input_queue.rs, session_queue_commands.rs | core/src/threads/queue.rs | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Правка очереди (Alt+Up, вернуть в composer) | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Steer: Enter во время хода отправляет уточнение | 🟡 | pending steers: восстановление при interrupt ок; ordering/mention-bindings/failed steer не включены | chatwidget/input_submission.rs | src/pages/session/composer/ | `composer-sweep` |
| Предпросмотр pending steers над composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Отправка во время review/plan stream (очередь вместо steer) | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Блокировка отправки при недоступной модели | ❌ | submit_user_message_blocks_when_thread_model_is_unavailable | chatwidget/input_submission.rs, session_model_selection.rs | src/pages/session/composer/ | `composer-sweep` |
| Подтверждение Esc/Enter при диалогах поверх composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Баннер бэкенда (нумерованные строки, CTA, отклонение по occurrence) | ✅ | landed c4fff1a2b: авто-переключение fallback и `{time}`, F6 (обрыв соединения отменяет запросы в полёте), 2 ключевых теста chat_live_2 зелёные; в pty баннер не гонялся (в mock нет endpoint). Нет (sweep): три теста восстановления (нужен on_rate_limit_error из chunk-status) и снимок backend_banner_notification; has_pending_recovery не портирован, fallback только при чтении аккаунта, notice_model не сбрасывается при смене треда | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Резерв Luna: селектор и статус | 🟡 | селектор работает; модель возврата резерва живёт только в памяти (нужен Ask::ConfigWrite); backend_auth заменяет has_chatgpt_account; патч переключения = модель + effort; статус по активной модели, переживание закрытия баннера — ждут | chatwidget/luna_reserve_*.rs, app/backend_banner_fallback.rs | src/pages/session/banners.rs | `live-turn-sweep` |
| Safety buffering: «повторить», подтверждение, короткое сообщение | ❌ | весь блок safety_buffering (9 тестов) | chatwidget/safety_buffering.rs, safety_recourse.rs | src/pages/session/cells/safety.rs | ядро: fw1-c6 (идёт); `ui-safety-compaction` |
| Экран ошибки провайдера (сохраняет черновик, выход в Resume) | ✅ | сохранение между потоками, provider_id sync — ждут | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Read-only внешний writer: заметка вместо composer, переход в Agents | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Уведомления терминала о завершении хода / approval | 🟡 | настройки notify_when; async-question уведомления n/a | notifications/, chatwidget/notifications.rs | src/app/terminal_effects.rs | `terminal-features` |
| Stream recovery: восстановление статуса после обрыва | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| MCP startup: статус, lag, отмена, отказ как warning | ✅ | mcp_startup_failure_renders_warning_history, booting snapshot — ждут | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Rate-limit подсказки/предупреждения во время хода | ❌ | limit warnings (90%/95%), prompt по лимиту | chatwidget/rate_limits.rs, status/rate_limits.rs | src/pages/session/live/limits.rs | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Misalignment/guardian stop треда | ❌ | misalignment_* (3), guardian parallel reviews, aggregate status | chatwidget/misalignment_policy.rs, app/misalignment_policy.rs | src/pages/session/cells/misalignment.rs (нет) | ядро: fw1-c6 (идёт); `ui-safety-compaction` |
| Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись родителя | ❌ | нет вообще: Ctrl+/ даёт `Notice::NotAvailable(Side)` (session_page.rs:683), /side и /btw — NotAvailable (core/src/app/session.rs); тесты отказов в side-треде тоже ждут | app/side*.rs, chatwidget/side.rs | src/features/side/ (нет) | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Режим Plan: popup «Implement this plan?», scope рассуждений | ✅ | plan_mode reasoning-scope popup (тесты n/a), reasoning override metka | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Shift+Tab переключение режима (collab mode) только в idle | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Бюджет цели: сообщения budget_limited / interrupted | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Replay незавершённого хода помечает task running | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |

### Composer (ввод, история, вставка, картинки, @/$ упоминания, Vim)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Многострочный ввод, Shift+Enter, курсор, wrap, прокрутка по высоте | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, undo | ✅ | часть textarea-тестов ждёт (39) | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Placeholder-подсказка пустого composer | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Slash-popup: фильтр, стрелки, Tab/Enter, описание, скрытие недоступных | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Недоступная команда во время хода: «disabled» текст, черновик цел | ✅ | disabled_slash_command_while_task_running_snapshot ждёт | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Команды с inline-аргументами (/plan x, /goal x, /review x, /rename x) | 🟡 | /model x, /permissions-profile без args; submission-цепочка не вся; после батчей 10-11: `/permissions x` ✅ (`PermissionsArgs`); `/model x`, `/title x`, `/statusline x`, `/resume x`, `/review` без args — NotAvailable или не все (см. раздел «Заглушки») | chatwidget/slash_command_args.rs, bottom_pane/prompt_args.rs | core/src/commands/dispatch.rs | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Вставка: paste burst, placeholder большой вставки [Pasted Content N] | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Вставка пути к файлу превращается в элемент/картинку | 🟡 | путь→картинка есть, hidden shell paste очередь/восстановление не включены | bottom_pane/chat_composer/paste_input.rs, attachment_files.rs | src/pages/session/composer/ | `composer-sweep` |
| Скрытая shell-вставка с /plan (hidden shell paste) | 🟡 | работает, тест не включён (4 из 5 stale), 1 красный: submits_literal | chatwidget/input_flow.rs | src/pages/session/composer/ | `composer-sweep` |
| История Up/Down в сессии | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| История между сессиями (persistent history) | ✅ | landed 2054483f3: Up после перезапуска вспоминает последнее сообщение (pty: rtwk = oracle), slash-команды записываются как у oracle (`/di` из popup → `/diff`), история засевается из replay resume (3 oracle-теста зелёные, 2 page-теста). Нет (sweep, ждут картинок и Vim): восстановление картинок и vim-навигация по истории (4 теста composer_3); история текстовая; `/diff` до pager закрывается Esc | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Поиск по истории Ctrl+R / Ctrl+S | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Восстановление очищенного Ctrl+C prompt из истории | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| @-упоминания: popup, вставка, Tab, множественные | ✅ | landed 2054483f3: живой file search (через in-process поиск хоста, `Ev::FileMatches`; app-server fuzzyFileSearch не портирован, не нужен) и server skills/plugins в списке, строки как у oracle (pty: `@sr` совпадает построчно); n/a при MentionsV2 off (решение 2026-11-23): 4 теста composer_1. Нет (sweep): adjacent_image_completion_inserts_leading_separator (ждёт картинок и Vim) | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Семена mention (при Esc/восстановление steer возвращают биндинги) | ✅ | биндинги доходят до отправки и восстанавливаются (mention-seeds, влит) | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| $ skills / плагины / apps в упоминаниях | 🟡 | landed 2054483f3: каталог skills (`skills/list`) и plugins (`plugin/installed`) грузится раз на тред, когда известен cwd; установленные apps берутся из кэша /apps (pty: `$sk` совпадает построчно). Нет: кэш apps загружается только при открытии /apps, поэтому `$`-приложения появляются после /apps, oracle грузит коннекторы при старте (Need владельцу select/apps); предупреждения о ошибках загрузки skill (invalid SKILL.md) не портированы; legacy MentionsV2-off — n/a (решение 2026-11-23) | bottom_pane/mentions_v2/, skill_popup.rs | src/pages/session/popovers/ | `composer-sweep` |
| Картинки: локальные [Image #N], нумерация, удаление | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Картинки из буфера Ctrl+V / Alt+V | 🟡 | поток есть: Ctrl+V/Alt+V → `Intent::PasteImage` → `Ask::ClipboardImage`, ответ в `src/services/effects.rs` (`paste_image`) → `Ev::ImagePasted`; в тестах ответ хоста подаёт kit (`clipboard_image`, батч 5: снята причина pending у теста с картинкой из буфера после удержанного `!`, chat_commands_1). Нет: прогона с настоящим буфером в бинаре; `adjacent_image_completion_inserts_leading_separator` (composer_3) ждёт (sweep) | clipboard_paste/, chatwidget/image_submission.rs | src/services/clipboard/ | `composer-sweep` |
| Remote-картинки (URL): submit и нумерация | ✅ | landed cc0bbabd5 (батч 5): `UserMessage.remote_images` идут в turn_input перед локальными, через merge_messages, `Draft::message` и put_back; Enter и Tab на одних remote-картинках отправляют; вызванная из истории запись снова рисует remote-строки; выделение, режущее элемент, расширяется до него; kit-семена `Open::remote_images` и `clipboard_image`. Тесты с этими семенами включены в chat_commands_1, composer_2, composer_3, composer_4. Нет (sweep): тела composer_4 без семян Vim/remote-картинок (удаление выбранной remote-строки как одна правка Vim, часть снимков) | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Состояния composer: disabled, shutdown, parent-owned, agents-nav hint | ❌ | нет понятия состояний; read-only есть частично | bottom_pane/chat_composer.rs, warning_notice.rs | src/pages/session/composer/states.rs (нет) | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Bang-shell режим: !команда, Enter в ходе → run_user_shell | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Vim: нормальный/вставка/операторы/текст-объекты/поиск (движок) | 🟡 | landed fdadb68ff (батч 5): `/vim` включает и выключает с заметкой (`NoteKey::Toggled`), F9 (`global.toggle_vim_mode`) тоже, `tui.vim_mode_default` начинает черновик нового треда в Insert (TuiSettings, config host), подпись футера следует режиму (страница перестраивается, когда клавиша меняет режим), `!` в Normal входит в Insert; vim-тесты composer_1, composer_3, chat_live_4, chat_commands_1 включены. Нет (sweep): composer_3 `left_respects_editor_and_vim_history_remaps` (нет keymap-семени), тела composer_4 без Vim-семян (поиск `/`, Ctrl+R на пустой истории, redo, Replace); Vim в поле ответа на вопрос (см. «Вопросы») | bottom_pane/textarea/vim*.rs, chat_composer/vim_*.rs | src/widgets/composer_draft/ | `composer-sweep` |
| Vim: replace/yank/dot-repeat | 🟡 | dot-повтор теперь записывает удержанный или сброшенный paste-burst символ (fdadb68ff); тела composer_4 ждут (Vim Replace, kit не включает Vim в этих телах) | bottom_pane/textarea/vim*.rs, chat_composer/vim_*.rs | src/widgets/composer_draft/ | `composer-sweep` |
| Внешний редактор Ctrl+G | ❌ | нет ни обработчика, ни запуска $EDITOR | external_editor.rs | src/features/external_editor/ (нет) | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Мышь в composer (курсор, выделение, вставка правой кнопкой) | ❌ | right_click_paste и мышь не выполняются | bottom_pane/chat_composer/mouse.rs, textarea/mouse.rs | src/pages/session/composer/mouse.rs (нет) | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Startup draft (черновик из аргументов, file-search startup) | 🟡 | работает, тест не включён | startup_draft*.rs | src/pages/startup/ | `composer-sweep` |
| Подсказки «? shortcuts» / Esc-Esc / «Ctrl+C to quit» в футере composer | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Элементы composer: [Image], [Pasted], mention — атомарное удаление | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Расширение (collapse) длинной вставки при submit | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Отправка с пустым / только пробелы / только плейсхолдер | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Тема-цвет курсора и подсказки; narrow-терминал | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |

### Слэш-команды (каждая — своя строка; порядок как в slash_command.rs)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| /model — выбор модели и effort | 🟡 | bare ✅ (OpenModels), landed 87ad7733c (chunk-popups-sweep): 13 тестов chat_popups_4 на живом пикере, курсор по форме страницы, скрытые модели; /model <имя> NotAvailable (provider-write не портирован); advanced effort, refresh каталога не включены; батчи 10-11: лист models на вложенном Navigator (advanced-страница, скрытые модели); `/model <имя>` — `Intent::Model{..}` → NotAvailable (app/session.rs) | chatwidget/model_command.rs, model_popups.rs | core/src/screens/select/model.rs | `dialogs-sweep` |
| /presets — выбор пресета (Daybreak presets) | ❌ | лист quick-dials (`Opening::Quick`, `select/quick_dials.rs`) и `select::presets` есть, debug-вход есть; страница `Intent::Quick` не шлёт: `SlashCommand::Presets` показывает попап «not built» (actions.rs, b11-fixes) | chatwidget/quick_dials.rs, app/quick_dials.rs | src/sheets/select/quick.rs | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| /daybreak | ❌ | NotAvailable; отказ Daybreak | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| /ide — контекст IDE | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| /permissions — approvals/профили | 🟡 | bare ✅ (OpenPermissions), args через PermissionsArgs ✅; подтверждение full-access, Windows, history cell не все | chatwidget/permissions_menu.rs, permission_popups.rs | src/sheets/select/permissions.rs | `dialogs-sweep` |
| /keymap — редактор клавиш | ❌ | `Which::Keymap` → `return` (core/src/features/settings/mod.rs), пустой слот «settings (S15)» без вида; ленты keymap-настроек нет | keymap_setup/ | src/features/settings/ (нет keymap_editor) | ядро: fw1-c3 (идёт); `ui-keymap-editor` |
| /vim — переключить Vim composer | ✅ | fdadb68ff (батч 5): включает и выключает черновик, пишет заметку; сброс режима в Insert после submit (composer_1 `vim_mode_resets_to_insert_after_submission` включён); pty-прогона в этом пересмотре нет | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /setup-default-sandbox (Windows) | ❌ | ElevateSandbox NotAvailable; только Windows | chatwidget/windows_sandbox_prompts.rs, windows_sandbox.rs | src/sheets/select/permissions.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| /experimental — feature-флаги | ❌ | `Which::Experimental` → `return`; sheet `src/features/settings/features.rs` есть, нет открывателя и ленты experimentalFeature/list | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| /approve — Auto-review | ✅ | открывает sheet denials (`sheets::approvals::open_auto_review`), без denials — заметка; снимки ждут | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /memories | 🟡 | `host/src/settings.rs` подключён (`settings_fact` в in_process.rs, батч 10): /memories открывается, когда host выдал факты memory tool (`facts.memories`), иначе молча ничего; reset-confirmation ✅ как UI-состояние. Pty при обновлении карты не перегонялся | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| /skills | ❌ | `Which::Skills` → `return`; sheet `src/features/settings/setup.rs` есть, нет открывателя и ленты skills/list; manage_skills_summary pending | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| /import — перенос из других агентов | ✅ | route Import; shard import 16/16 | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /hooks | ❌ | NotAvailable (`S::Hooks` в core/src/app/session.rs); sheet `src/features/settings/hooks.rs` есть, нет ленты hooks/list; hooks_popup_shows_list_diagnostics | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| /review — выбор ветки/коммита/custom | ✅ | branch/commit picker snapshots ждут (review_*_picker, 2 pending) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /rename — имя треда | ✅ | landed a15c35bf8 (95826e191): с аргументом и bare prompt (slice features/rename); батч 9 (features-trait): prompt — UI-половина `src/features/rename/prompt.rs` на общем виде `features/prompt.rs` (Menu/Pick/Prompt у Feature убраны), pty rename-prompt в land-check батча; 29abc1aa5: вид prompt `src/sheets/dialogs/prompt.rs`, чат показывает любой открытый моделью prompt (так же файловый prompt /export и edit /goal), 5 тестов chat_commands_2 зелёные, pty rename-prompt проходит в обоих бинарях. Нет: предложение имени — хост отвечает Done::Title(None), oracle генерирует через ephemeral thread (thread_title.rs); нужен backend SuggestTitle в services/rpc | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /new | 🟡 | новый тред ✅; выбор checkout (`Intent::OpenCheckout`) — модель в core есть, приложение его не шлёт (select.rs:706) | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| /archive | ✅ | confirm sheet + RPC | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /delete | ✅ | confirm sheet + RPC | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /resume | 🟡 | route Resume ✅ (resume_1/2: 48 из 72 зелёные); `/resume <имя>` (`ResumeNamed`) — NotAvailable | resume_picker.rs, session_resume.rs, named_session_lookup.rs | src/pages/resume/ | `resume-rest` |
| /fork | 🟡 | новый тред ✅; ошибка fork привязана к исходному треду (b11-fixes); пикер checkout (`OpenCheckout` не шлётся) и строка «Forked thread» ❌ | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| /worktree | 🟡 | sheet открывается (`S::Worktree` → `OpenWorktrees`); список не грузится: у `Ask::Worktrees` и `Ask::WorktreeRemove` нет исполнителя (services/effects.rs → None, host/src/effects.rs не обслуживает); Continue/Start-цепочка ждёт `OpenCheckout` и старт | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| /app — открыть в desktop-приложении | ✅ | OpenUrl codex://threads/<id> | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /init | ✅ | отправляет prompt AGENTS.md | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /compact | 🟡 | команда ✅; статус/таймер compaction не включены | chatwidget/compaction.rs | src/pages/session/cells/compaction.rs (нет) | ядро: fw1-c6 (идёт); `ui-safety-compaction` |
| /config | ❌ | `Which::Config` → пустой слот «settings (S15)» (`Shown::Other`), вида нет; то же для `/tui` | config_panel/, chatwidget/tui_mode_picker.rs | src/features/settings/ (нет config_panel) | ядро: fw1-c3 (идёт); `ui-config-panel` |
| /recap | ✅ | fix-recap-title 4131311a7: AppCx::structured_turn (ephemeral read-only thread, outputSchema), recap cell / failure line; остаток: отказ для облачной беседы -> w2-commands | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| Пробел: /recap и предложение имени треда — заглушки | ✅ | fix-recap-title 4131311a7: заглушки Ask::StructuredTurn/SuggestTitle удалены; /recap и предложение имени идут через AppCx::structured_turn; остаток (авто-имя по первому сообщению, отмена устаревшего запуска) -> w2-commands | temporary_structured_request.rs, app/recap.rs, app/thread_title.rs | core/src/features/recap, core/src/features/rename, src/services/effects.rs | `fix-recap-title` (после fw-hubs-b) |
| Пробел: потеря/восстановление потока треда | ✅ | fix-stream-lost 78e957dcf: host/src/cloud.rs шлёт BackendEv::StreamLost{thread,error}/StreamRestored; простой тред — Command::ReconnectIdleThread, в ходе — строка Error, очередь держится; восстановление: history_may_be_incomplete + отправка очереди; открытие потерянного треда перечитывает его. Остаток: restore_reconnected_input (steer до эха) -> live-turn; AppEv::ShuttingDown без отправителя (нет on_stop в rtwk, потребителя нет) | app/app_server_events.rs:130-170, chatwidget/input_flow.rs | host/src/in_process.rs, host/src/cloud.rs, core/src/threads | `fix-stream-lost` (после fw-hubs-b) |
| /plan [prompt] | ✅ | маска из каталога, уведомления disabled/unavailable | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /voice | ❌ | bare /voice, mute, stop → NotAvailable (`VoiceControl`); F8 и fallback-ветка open.rs → пустой sheet voice | chatwidget/realtime*.rs, bottom_pane/voice_strip.rs | src/sheets/voice/ | `voice` |
| /goal [цель/pause/resume/clear] | 🟡 | команда и модель (features/goal) ✅; батч 9 (features-trait): у цели своя UI-половина `src/features/goal/` (sheet.rs: меню цели и edit prompt, общий вид prompt `features/prompt.rs`), `Target::Feature`; снимки меню и edit prompt не включены | chatwidget/goal_menu.rs, goal_status.rs, goal_display.rs | src/features/goal/ | `dialogs-sweep` |
| /agents — центр агентов | ✅ | Target::Agents route | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /side, /btw | ❌ | нет; тесты отказов в side-треде тоже ждут | app/side*.rs | src/features/side/ (нет) | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| /copy — копировать ответ | ❌ | NotAvailable(Copy); copy picker и Ctrl+O не готовы; в чате есть только копия выделения мышью | chatwidget/copy_picker.rs, markdown_copy.rs | src/features/copy/ (нет) | ядро: fw1-c4 (идёт); `ui-select-copy` |
| /export | ✅ | landed a15c35bf8 (95826e191): путь, picker назначения, prompt имени (slice features/export); батч 9: UI-половина `src/features/export/` (sheet.rs), `Target::Feature` | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /raw | ✅ | режим + заметка, Alt+R | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /tui — быстрые настройки TUI | ❌ | `Which::Config` → пустой слот (dispatch_bare шлёт Tui в `Target::Settings`; вида нет) | config_panel/, chatwidget/tui_mode_picker.rs | src/features/settings/ (нет config_panel) | ядро: fw1-c3 (идёт); `ui-config-panel` |
| /diff | ❌ | `Target::Pager` показывает «not yet» (open.rs). Готово: core `DiffStep` (screens/transcript/diff.rs), `git_diff` + `WorkspaceExec` (src/services/shell.rs), `Ask::GitDiff` → `Done::GitDiff` (services/effects.rs, host-тесты). Нет: ответ → документ pager, проверка stale cwd, открыватель | get_git_diff.rs, diff_render.rs | src/pages/session/transcript/diff_view.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| /mention | ✅ | вставляет «@» | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /status — карточка статуса | 🟡 | карточка ✅ — ячейка транскрипта треда, как у oracle (status-card-adr003); landed e1795b779 (батч 3): живая карточка из фактов треда, config, аккаунта, каталога моделей и удалённого подключения; лимиты с обновлением по `RateLimitOrigin::Status(request)`, строки Model (display name, reasoning, summaries), Collaboration mode, Agents.md; pty: строки равны oracle. Нет: Thread name (oracle делает title-запрос, порт нет), версия в заголовке `(v0.0.0)` вместо `(v0.170.0)` (версия сборки); thread usage, server row, limit warnings ❌ (дети); лента фактов статус-строки (Cwd/Clock/SideLabel/AgentLabel, git-пробы) не построена | status/card.rs, chatwidget/thread_usage.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| /daemon | ❌ | `Target::Select` (остаток /daemon) показывает «not yet» (`daemon_action`); модель `select::daemon` в core есть | app/daemon_menu.rs | src/sheets/select/daemon.rs (нет) | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| /warnings (и F2) | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /cd <путь> | ✅ | с отказами | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /pwd, /cwd | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /usage | 🟡 | нужен backend auth, иначе заметка UsageNeedsLogin; страница есть (43 теста не включены); меню reset ❌ | chatwidget/usage.rs, analytics/ | src/pages/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| /debug-config | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| /title — терминальный title | 🟡 | bare /title открывается и сохраняет через `Ask::ConfigWrite` (`items::title`, батч 10); `/title <args>` — NotAvailable | chatwidget/status_surface_commands.rs, bottom_pane/title_setup.rs | src/features/settings/ | ядро: fw1-c3, fw1-c2 (идёт); `footer-status-sweep` |
| /statusline | 🟡 | bare /statusline открывается и сохраняет (`items::status_line`); `/statusline <args>` — NotAvailable | status/card.rs, chatwidget/thread_usage.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| /theme | ✅ | settings Theme; 790cc44a6 (батч 6): без темы в config пикер открывается на теме терминала по умолчанию (строка «(current)» и курсор), превью по центру под строкой отступа панели — строки пикера равны live oracle (pty settings-theme). Нет: подсказка футера (у нас `Press enter to confirm or esc to go back`, у live oracle `enter select · esc back`; снимки oracle-шардов совпадают со стандартной); лента фактов host подключена (`settings_fact`, батч 10), темы из config и `{codex_home}/themes` должны быть в бинаре (pty не перегонялся); клик по барьеру не возвращает тему превью (как было) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /pets, /pet | 🟡 | sheet pets открывается; ambient-часть и тесты не включены | pets/, chatwidget/pets.rs | src/sheets/pets/ | `pets-ambient` |
| /mcp | ❌ | NotAvailable; ячейка инвентаря | history_cell/mcp.rs | src/pages/session/cells/mcp_inventory.rs (нет) | ядро: fw1-c5 (идёт); `ui-exec-mcp` |
| /apps | ✅ | диалог ссылки приложения, вкл/выкл, сброс при смене аккаунта, обновление каталога (b2eeacb7c); partial-snapshot тесты n/a (нет is_final:false в протоколе) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /plugins | 🟡 | влит 8c492bcbc: список, установка, marketplaces. Пилот ADR-013 (батч 8, 454b13f11): Esc как у oracle (закрывает на карточке, loading и failure; возвращает только на подтверждении удаления и prompt add-marketplace), `Back to plugins` — строка карточки (курсор, вкладка и запрос сохраняются), setup-ссылка через `Ask::OpenUrl`, страницы — маршруты вложенного Navigator; pty рядом с oracle: браузер, вкладки, поиск, карточка, add-marketplace равны. Различия: усечение `…` после пробела (общий TextOverflow::Ellipsis пикера), переход prompt → failure add-marketplace не гонялся; красный на tip chat_popups_2 `plugins_popup_preserves…` (debt-b9-session) | chatwidget/plugins.rs, plugin_catalog.rs | src/sheets/select/ | `dialogs-sweep` |
| /logout | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| /quit, /exit | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /feedback | 🟡 | батч 9 (features-trait, d776d1a43): `/feedback` открывает страницу Feedback листа dialogs (категория, consent) — раньше открывал пустой лист; заметка и загрузка логов не написаны; upload-consent/diagnostics снимки ждут (27 тестов) | bottom_pane/feedback_view.rs, feedback_note_view.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| /rollout | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /ps — фоновые терминалы | ❌ | NotAvailable | bottom_pane/unified_exec_footer.rs, history_cell/exec.rs | src/pages/session/cells/processes.rs (нет) | ядро: fw1-c5 (идёт); `ui-exec-mcp` |
| /stop, /clean | ✅ | чистит терминалы + заметка | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /clear | ✅ | новый тред/экран | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /test-approval | ✅ | Target::Approvals | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /subagents (multi-agents picker) | ❌ | `Target::AgentPicker` показывает «not yet» (`action_subagents` в open.rs); `app/multi_agents.rs` не портирован | app/multi_agents.rs, agent_navigation.rs | src/sheets/select/agent_picker.rs (нет) | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| /debug-m-drop, /debug-m-update | 🟡 | заглушка-заметка `NoteKey::Stub(MemoryMaintenance)` (debug) | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |

### Попапы и селекторы

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Список выбора: поиск, стрелки, цифры, disabled-строки, hint по биндингам | ✅ | — | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | — |
| Model picker: каталог, discovery, bedrock/custom display, service tier | 🟡 | landed 87ad7733c: скрытые модели, seed провайдера, лимит потоков агента; осталось: refresh каталога, reasoning submenu (extra high, advanced), снимки | chatwidget/model_popups.rs, model_catalog.rs | core/src/screens/select/model.rs | `dialogs-sweep` |
| Reasoning picker и Alt+,/. , Shift+Up/Down | ✅ | reach_max (красный), ultra, advanced-only ждут | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Permissions popup: пресеты, профили, retry discovery | 🟡 | full access confirm, Windows sandbox prompts, history cell решения | chatwidget/permission_popups.rs, permission_discovery.rs | src/sheets/select/permissions.rs | `dialogs-sweep` |
| Windows sandbox: prompts, help-ссылки, fallback | ❌ | весь блок (≈10 тестов); только Windows | chatwidget/windows_sandbox_prompts.rs | src/sheets/select/permissions.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| Goal: меню и edit prompt, resume-paused prompt | 🟡 | меню и edit prompt — лист фичи `features/goal/sheet.rs` (батч 9); снимки не включены; resume-paused работает, тест не включён | chatwidget/goal_menu.rs | src/features/goal/ | `dialogs-sweep` |
| Plan: popup реализации и scope reasoning | ✅ | — | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Collaboration / mode catalog | 🟡 | каталог недоступен (тест не включён) | collaboration_modes.rs, chatwidget/plan_implementation.rs | core/src/screens/select/ | `dialogs-sweep` |
| Worktree/checkout picker | 🟡 | лист worktree (`select/worktree.rs`, `Opening::Worktree`) открывается из /worktree; нет исполнителя `Ask::Worktrees`/`WorktreeRemove` и шага старта; `OpenCheckout` не шлётся (/new, /fork) | chatwidget/worktree_picker.rs | src/sheets/select/worktree.rs | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| Quick dials (saved dials: save/clear picker) | ❌ | лист `Opening::Quick` и debug-вход есть, входа из чата нет (`Intent::Quick` не шлётся, /presets) | chatwidget/quick_dials.rs | src/sheets/select/quick.rs | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Plugins: список, вкладки, поиск, деталь, install, marketplaces | 🟡 | влит 8c492bcbc и пилот ADR-013 (см. строку /plugins); осталось: усечение после пробела, add-marketplace failure не гонялся, красный plugins_popup_preserves… | chatwidget/plugins.rs, plugin_catalog.rs | src/features/plugins/ | `dialogs-sweep` |
| Apps/connectors: список, статус, install-only | 🟡 | простые состояния ✅; partial refresh ❌ | chatwidget/connectors.rs, bottom_pane/app_link_view.rs | src/sheets/select/app_link/ | `dialogs-sweep` |
| Hooks popup: список, диагностика | ❌ | sheet `features/settings/hooks.rs` есть; `Which::Hooks` → `return`, ленты hooks/list нет | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Experimental features popup (toggle сохраняется на выходе) | ❌ | sheet `features/settings/features.rs` есть; `Which::Experimental` → `return`, ленты experimentalFeature/list нет (toggle сохраняется на выходе — не проверено) | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Feedback: выбор, consent upload, диагностика, note | 🟡 | страница Feedback открывается из `/feedback` (батч 9); note и логи нет; снимки ждут | bottom_pane/feedback_view.rs, app/feedback_upload.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| Review picker (ветка/коммит/custom) | ✅ | — | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Copy/export picker (кастомные клавиши, payload) | ❌ | export-назначение есть (`features/export`, см. /export); `/copy` picker и payload нет | chatwidget/copy_picker.rs, transcript_export.rs | src/features/export/, src/features/copy/ (нет) | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Reset-limit picker и подтверждение (usage menu) | ❌ | ≈30 тестов chat_live_6 | chatwidget/reset_credits.rs, usage.rs | src/pages/session/live/usage_menu.rs | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Security setup / security prompts | ❌ | security_setup_* (≥4) | security_setup.rs, chatwidget/security_setup.rs | src/features/security_setup/ (нет) | ядро: fw1-c8 (идёт); `ui-worktree-sandbox` |
| Memories settings и reset | 🟡 | экран и reset готовы; открывается, когда host выдал факты memory tool (см. /memories) | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Manage skills summary | ❌ | sheet `features/settings/setup.rs` есть; `Which::Skills` → `return`, ленты skills/list нет | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Current settings (model/permissions/status layout) | ❌ | current_settings_offer_* | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Sweep: прочие мелкие popup'ы | ? | состав «sweep» не установлен по коду; задача created | bottom_pane/list_selection_view.rs | src/sheets/dialogs/ | `dialogs-sweep` |

### Approvals и вопросы

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Approval exec: модал, варианты, execpolicy-префикс, причина | ✅ | multiline-prefix скрытие execpolicy ждёт | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | — |
| Approval apply_patch: модал, call_id, заголовок | 🟡 | untrusted модал, manual flow, patch snapshot ждут | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Approval network host (разово/сессия) | 🟡 | history decision cell ждёт; запрос ✅ | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Approval write_stdin | ❌ | render terminal input | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Permissions request approval, MCP elicitation форма | ✅ | — | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| Guardian review (approved/denied/timed out, aggregate status) | ❌ | 0 из 11 | auto_review_denials.rs, history_cell/approvals.rs | src/sheets/approvals/guardian.rs (нет) | ядро: fw1-c6 (идёт); `ui-safety-compaction` |
| Auto-review denials popup | ✅ | sheet denials (`open_auto_review`); снимок ждёт | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| request_user_input: вопросы с вариантами, цифры/буквы | ✅ | батч 7 (b3ae82940): клавиши диалогов вопросов через keymap (`keys.rs`), `Draft.selected` → `choice`; поведение то же | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| Вопросы: черновики, навигация, replay черновиков | ❌ | question_drafts_survive_*, turn_end drafts (≈10) | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Вопросы: поиск по истории, Vim в поле ответа | ❌ | history search; Vim в поле ответа не подключён (approvals_1, chat_approvals_2 ждут, батч 5 их не трогал) | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Асинхронные вопросы (desktop answer, уведомления) | 🟡 | уведомления n/a; многое n/a — другой клиент | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Отклонение пустых ответов, явный skip | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Разворот вопроса уступает мышь терминалу для ссылок | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Очередь вопросов и queued messages делят один shortcut | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (идёт); `ui-approvals-rest` |
| Keymap-seeds для approval | ❌ | — | keymap.rs | tests/kit/ | `kit-seeds` |

### Футер и статус-строка

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режимы | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Подсказка очереди «Alt+Up edit» | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Строка статуса: модель, effort, cwd, context %, токены | ✅ | git-ветка, PR со ссылкой, изменения ветки, headline, raw output (ea69879b9); нет: Cwd/Clock-факты и пробы git/headline от host, заголовок треда в строке | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Настройка статус-строки (/statusline, порядок элементов) | 🟡 | bare /statusline открывает и сохраняет (`Ask::ConfigWrite`); аргументы NotAvailable | bottom_pane/status_line_setup.rs, title_setup.rs, terminal_title.rs | src/features/settings/, src/pages/session/window_title.rs | ядро: fw1-c3, fw1-c2 (идёт); `footer-status-sweep` |
| Терминальный заголовок (title) с элементами | 🟡 | рендер ✅; настройка bare /title ✅ (`/title <args>` NotAvailable) | bottom_pane/status_line_setup.rs, title_setup.rs, terminal_title.rs | src/features/settings/, src/pages/session/window_title.rs | ядро: fw1-c3, fw1-c2 (идёт); `footer-status-sweep` |
| Индикаторы режимов: Plan, goal, side, agent, Vim | ✅ | рисуются (ea69879b9); подпись Vim следует режиму (fdadb68ff); метки side/agent ждут фактов от host | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Индикатор голоса (voice strip), F8 | ❌ | — | bottom_pane/voice_strip.rs | src/pages/session/footer/voice.rs (нет) | `voice` |
| Индикатор эффекта effort ignition | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Sparkle/«искры» над composer | 🟡 | задача занята | bottom_pane/chat_composer/sparkle.rs | src/pages/session/footer/sparkle.rs | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Счётчик предупреждений (F2) в футере | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Flash в футере и временный override подсказки | 🟡 | таймер flash и override есть (ea69879b9); на странице нет источника вспышки | bottom_pane/footer.rs, footer_hint.rs | src/pages/session/footer/flash.rs | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Строка follow («вернуться вниз») над composer | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Карточка /status: основное | ✅ | ячейка транскрипта (ADR-003), e1795b779: Model с reasoning и summaries, Collaboration mode, Permissions, Agents.md, Session, Token usage, Context window, Limits; нет Thread name (title не генерируется) | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | — |
| Карточка /status: usage треда | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Карточка /status: строка сервера | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Предупреждения о лимитах в карточке и ленте | ❌ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | `composer-sweep` |
| Подпись контекста side-разговора | ❌ | — | bottom_pane/chat_composer/status_surface.rs, app/side_companion.rs | src/pages/session/status/ | ядро: fw1-c3, fw1-c2 (идёт); `footer-status-sweep` |
| Подсказка reset-лимита на старте | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |

### Транскрипт-оверлей / pager / backtrack / выделение-копирование-поиск

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Ctrl+T: подробный транскрипт (sheet) с reasoning и warnings | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Esc-Esc: backtrack — выбор прошлого prompt, Enter откат | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | ?: часть тестов backtrack всего 4 | pager_overlay.rs, transcript_view/ | src/pages/session/transcript/ | — |
| Статический pager (/diff, вывод): строки, страницы, Home/End, q, проценты | 🟡 | есть как режим страницы (`StaticPager`, core/src/screens/transcript.rs); /diff «not yet» (см. /diff), live-tail sync и поиск ❌ | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| Pager: поиск /, n/N | ❌ | — | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| Pager: live tail sync при росте транскрипта | ❌ | — | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| Мышью выделить текст в транскрипте (подсветка) | ✅ | SelectionArea рисует выделение | pager_overlay.rs, transcript_view/ | src/pages/session/transcript/ | — |
| Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | landed 542959815: копия по отпусканию, Ctrl+C / Cmd+C / Ctrl+Shift+C, Enter = копия и вниз, Esc снимает, подсказка в футере, строка «Copied N chars to host clipboard», выделение снимается после подтверждённой копии (pty text-100: рядом с oracle совпадает); сервис копирования clipboard-service (5026ad24c). Расхождения: конец диапазона у oracle до ячейки под курсором, у rtwk SelectionArea включает её («Copied 12 chars» вместо 11, rtwk gap); OSC 52 при копии по отпусканию пишется дважды (clipboard-provider в rtwk). Sweep: X11 primary, deselect сравнивает текст, не диапазон, oracle-снимки на 3-4 строках (kit::pending) | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Двойной/тройной клик: слово/строка/абзац | ❌ | по паку это жесты rtwk SelectionArea; тестов и проверки в бинаре у порта нет | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Shift-клик расширяет, правая кнопка копирует | 🟡 | правая кнопка копирует (Listener в SelectionHost, 542959815, тест в selection_copy.rs); Shift-клик и расширение с клавиатуры не построить: у SelectionAreaController есть `select(target, item, start, end)`, но нет чтения start/end (rtwk gap) | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Копия как Markdown/plain (payload) | ❌ | HTML-вариант Markdown-копии в сервисе готов (5026ad24c); выбора формата в UI нет; копия выделения мышью только plain text (CopyFormat::MarkdownSelection ждёт тексты элементов); transcript_view_3 в основном n/a | markdown_copy.rs, clipboard_html.rs | src/pages/session/transcript/copy.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Режим выделения с клавиатуры (select mode) | ❌ | нет модуля | transcript_view/select_mode.rs | src/pages/session/transcript/select_mode.rs (нет) | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Режим копирования (copy mode, Ctrl+X-лидер) | ❌ | нет модуля | transcript_view/copy_mode.rs | src/pages/session/transcript/copy_mode.rs (нет) | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | F3 только открывает sheet | transcript_view/search*.rs | src/pages/session/transcript/find.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| Подсказки поиска и футер поиска | ❌ | — | transcript_view/search*.rs | src/pages/session/transcript/find.rs (нет) | ядро: fw1-c1 (идёт); `ui-pager-find` |
| Скорость колеса (mouse_scroll_speed) | ❌ | настройка есть, не применяется | tui/scrollback.rs, app/history_pagination.rs | src/pages/session/transcript/follow.rs, src/pages/session/transcript/history.rs | `ui-transcript-chrome` |
| Старые страницы в overlay (history pages) | 🟡 | ee4c83bd9: статус-строка sheet транскрипта берёт спеллинги из `transcript/history.rs` (`sheet::status`); нет ctrl+home/alt+< и склейки групп (см. строку выше) | tui/scrollback.rs, app/history_pagination.rs | src/pages/session/transcript/follow.rs, src/pages/session/transcript/history.rs | `ui-transcript-chrome` |
| Сайдбар (панель агентов/тредов) | ❌ | ADR-закрытая задача sidebar | app/sidebar*.rs, sidebar_tab_strip.rs | src/pages/session/sidebar/ (нет) | `sidebar` |
| Inline-режим (без alt-screen) | ❌ | задача created | tui.rs, insert_history.rs, app/resize_reflow.rs | src/app/screen_mode/ (нет) | `inline-mode` |
| Дерево UI (ui-tree) | ? | не относится к ленте; задача created | ui_tree.rs, ui_tree/ | src/widgets/ui_tree/ (нет) | `ui-tree-viz` |
| Клик по ссылке/файлу в overlay | 🟡 | обычный клик красный | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c4 (идёт); `ui-select-copy` |

### Диалоги

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Предупреждения (/warnings, F2): страницы, wrap | 🟡 | warning_pages_wrap красный | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | `composer-sweep` |
| Шорткаты: оверлей «?» со всеми действиями | 🟡 | ab78ae687 (батч 6): страница шорткатов — состояние sheet на границе ADR-013, F2 и копирование через keymap; gap: из чата оверлей достижим только из debug-меню | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | `dialogs-sweep` |
| Rename thread prompt (bare /rename) | ✅ | slice features/rename (a15c35bf8), вид prompt 29abc1aa5; с батча 9 prompt — UI-половина фичи (`features/rename/prompt.rs`); нет предложения имени (Done::Title(None), нужен SuggestTitle; gap из hand-in батча 9: предложение имени не в поле ввода — нужно покрытие pty/oracle) | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Confirm: archive/delete треда | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Daemon (подключение к daemon) | ❌ | попап «not yet» (см. /daemon) | app/daemon_menu.rs, bottom_pane/app_link_view.rs | src/sheets/select/ | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Ссылка на приложение (app link) диалог | ❌ | — | app/daemon_menu.rs, bottom_pane/app_link_view.rs | src/sheets/select/ | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Custom review prompt | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Верификация пользователя (user verification) | ❌ | — | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | `dialogs-sweep` |
| Settings: Theme, Memories (из чата) | 🟡 | Theme ✅ (лента фактов host подключена, батч 10); Memories открывается только при фактах memory tool (`facts.memories`); строки Theme и настройки принимают тап (`GestureDetector`) | theme_picker.rs, bottom_pane/memories_settings_view.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-settings-feeds` |
| Settings: Config / Statusline / Title / Experimental / Skills / Hooks / Keymap | 🟡 | Statusline и Title ✅ (bare; см. строки команд); Config и Keymap — пустой слот без вида; Experimental, Skills, Hooks — sheet есть (features.rs, setup.rs, hooks.rs), открыватель `Which::*` → `return`, ленты host нет. `Fact::Offered` приходит только из seed/тестов/debug-меню, host его не шлёт | config_panel/, bottom_pane/status_line_setup.rs | src/features/settings/ | ядро: fw1-c3 (идёт); `ui-config-panel` |
| Feedback (диалог) | 🟡 | ab78ae687: шаг feedback — состояние sheet; с батча 9 из чата достижим командой `/feedback` (страница Feedback); снимки consent/diagnostics ждут | bottom_pane/feedback_view.rs, app/feedback_upload.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c4, fw1-c9 (идёт); `ui-feedback-cmds` |
| Pets (диалог выбора) | 🟡 | ab78ae687: курсор и превью — состояние sheet; глаголы LoadPreview, ReleasePreview; ambient-часть и тесты не включены | pets/picker.rs | src/sheets/pets/ | `pets-ambient` |
| Voice (диалог, F8) | ❌ | — | chatwidget/realtime_settings.rs | src/sheets/voice/ | `voice` |
| Trust on open (запрос доверия к каталогу) | ? | в startup-шарде 29/32 зелёные; из чата не вызывается | onboarding/trust_directory.rs | src/pages/startup/ | `startup-rest` |
| Model provider switch (блокирует кэш выбора) | ❌ | provider_switch_* | app/provider/, chatwidget/model_popups.rs | core/src/screens/select/model.rs | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Debug menu F12 | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Список выбора (SelectionList): тап по строке | 🟡 | gap (2f68c2c75): тап по строке только двигает курсор, как у rtwk-виджета; выбор строки — клавишей | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | `dialogs-sweep` |
| Resume picker: клавиши oracle (ctrl-p/n/k/j/h/l/b/f), ctrl+e на пустом id | ✅ | landed 656a4d753 (батч 6, adr013-resume): список и курсор — через keymap (`LIST_*`), плотность и открытая строка — состояние страницы, дозагрузка — один verb `LoadMore`; ctrl+e на пустом id игнорируется как у oracle. На ee6bad4e8 resume_2 `page_and_jump_navigation` больше не красный (resume_1 + resume_2: 49 зелёные, 12 pending, 0 red) | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |

### /usage

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Страница аналитики: бары, вкладки, часы плана, narrow-графики | 🟡 | рендер работает, 43 теста не включены; батч 7 (usage-agents, f1675bfa8, ecdfcbed1): usage live/Leave, группы не в Pos, нет whole-state watch, диалог ошибки по привязанным ключам, помощь закрывается, когда секция не меняется; поведение то же | analytics/ | src/pages/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Меню /usage: Analytics, Refresh, Reset | 🟡 | в чате 0 из 25: пункты меню и reset не подключены | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Reset лимита: пикер, подтверждение, idempotency-ключ, in-flight | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Billing и enterprise-планы | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Подсказка reset на старте | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Статус после billing retries | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Ранний redraw и повтор thread usage | ❌ | n/a в harness | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7 (идёт); `ui-usage-limits` |
| Нужен вход (UsageNeedsLogin) | ✅ | — | analytics/, chatwidget/usage.rs | src/pages/usage/ | — |

### Горячие клавиши (дефолты oracle)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Ctrl+T — подробный транскрипт | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Esc — прервать ход | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Esc Esc (пустой composer) — backtrack | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+C — прервать/выход | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+D — выход на пустом | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Up/Down — история | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+R / Ctrl+S — поиск по истории | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Shift+Tab — режим | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Tab — в очередь (сообщения) | 🟡 | команды и shell в очереди ❌ | keymap.rs, chatwidget/input_queue.rs | src/pages/session/composer/drain.rs | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Alt+Up — вернуть из очереди | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Alt+, / Alt+. — reasoning | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Shift+Up/Down — reasoning | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Alt+R — raw | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| F2 — warnings | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| F3 — поиск | 🟡 | открывает sheet без поиска | keymap.rs, transcript_view/search.rs | src/pages/session/keys.rs | ядро: fw1-c1 (идёт); `ui-pager-find` |
| F4 — focus activity | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | keymap.rs, transcript_view/search.rs | src/pages/session/keys.rs | ядро: fw1-c1 (идёт); `ui-pager-find` |
| F8 — голос | 🟡 | открывает пустой sheet | keymap.rs, bottom_pane/voice_strip.rs | src/pages/session/key_actions/voice.rs (нет) | `voice` |
| F12 — debug menu | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+/ (Ctrl+_, Ctrl+7) — side | ❌ | `Notice::NotAvailable(Side)` (session_page.rs:683) | keymap.rs, app/side.rs | src/pages/session/key_actions/side.rs (нет) | ядро: fw1-c1, fw1-c2 (идёт); `ui-openers-side` |
| Ctrl+O — копировать ответ | ❌ | в keymap/catalogue.rs есть, обработчика нет | keymap.rs, chatwidget/copy_picker.rs | src/pages/session/key_actions/copy.rs (нет) | ядро: fw1-c4 (идёт); `ui-select-copy` |
| Ctrl+G — внешний редактор | ❌ | в keymap/catalogue.rs есть, обработчика и запуска $EDITOR нет | keymap.rs, external_editor.rs | src/pages/session/key_actions/external_editor.rs (нет) | ядро: fw1-c2 (идёт); `ui-composer-states` |
| Ctrl+V / Alt+V — вставка картинки | 🟡 | `Ask::ClipboardImage` обслуживается (см. строку composer); с настоящим буфером в бинаре не гонялось | clipboard_paste/ | src/services/clipboard/ | `composer-sweep` |
| Ctrl+L — очистить терминал | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | tui/job_control.rs, keymap.rs | src/app/terminal_effects.rs | `terminal-features` |
| Ctrl+Z — suspend | ? | в keymap зарезервирован; обработчик не найден | tui/job_control.rs, keymap.rs | src/app/terminal_effects.rs | `terminal-features` |
| Ctrl+X — leader (Ctrl+X b — смена бэкенда) | 🟡 | leader живёт только для баннера | keymap.rs (leader) | core/src/keymap/ | ядро: fw1-c3 (идёт); `ui-keymap-editor` |
| ? — оверлей шорткатов | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Left на пустом composer — Agents | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Enter / Shift+Enter — отправка / перенос | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| PgUp/PgDn, колесо — прокрутка ленты | ✅ | — | keymap.rs, keymap/ | core/src/keymap/, src/pages/session/keys.rs | — |
| Vim-клавиши (69 действий) — normal/operator/text-object/search | ✅ | включаются /vim и F9 (батч 5); sweep: см. строки Vim в Composer; keys-5 (батч 12): `/` поиск и `!` идут через keymap (SHELL_PREFIX, SLASH_COMPLETE); KeyPress-строк не осталось | keymap.rs, keymap/ | core/src/keymap/, src/pages/session/keys.rs | — |

### Вне экрана чата

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Inline viewport | ❌ | S27: нижний viewport со своим scrollback; rtwk ScreenMode::Inline + Static | tui.rs, custom_terminal.rs, tui/frame_requester.rs | — | `inline-mode` |
| Вставка завершённых строк в scrollback | ❌ | DECSTBM/native scrollback, OSC 8 и картинки в committed-строках (G7) | insert_history.rs, tui/scrollback.rs, tui/history_tail.rs | — | `inline-mode` |
| Resize reflow | ❌ | пересборка committed-строк из исходных ячеек при resize (G2) | app/resize_reflow.rs, transcript_reflow.rs, resize_reflow_cap.rs | — | `inline-mode` |
| Fullscreen (alt-screen + мышь) | ✅ | — | tui/alternate_screen.rs | rtwk host (`App::run`), src/app/run.rs | — |
| Временный alt-screen для оверлеев | ❌ | вход/выход alt-screen во время работы (G1) для pager в inline | tui/alternate_screen.rs, pager_overlay.rs | — | `inline-mode` |
| Suspend / resume | ❌ | Ctrl+Z: SIGTSTP и возврат; в core есть только запись каталога keymap | tui/job_control.rs | core/src/keymap/catalogue.rs (запись) | `terminal-features` |
| Bracketed paste и kitty keyboard | ✅ | — | tui/keyboard_modes.rs, tui/input_boundary.rs | rtwk (ввод), src/pages/session/composer/burst.rs | — |
| Focus, synchronized output, Windows console, panic hook | ? | в port не нашлось собственных проверок: первый шаг пакета — проверить, что rtwk host делает это (focus-in/out, DEC 2026, консоль Windows, восстановление терминала при panic) | tui/event_stream.rs, tui/frame_rate_limiter.rs, tui/windows_console.rs, tui/windows_key_sequence.rs, tui_panic_tests.rs | rtwk-core platform/console_input.rs | `terminal-features` |
| Clipboard copy (native, tmux, WSL, OSC 52) | ✅ | — | clipboard_copy/, tui/selection_clipboard.rs | src/services/clipboard/ (machine, tmux, osc52) | — |
| Мышь | ✅ | двойной/тройной клик и Shift-клик — в строках выделения | tui/link_pointer.rs, app/link_hover.rs | rtwk (hit-testing), src/pages/session/transcript/selection.rs | — |
| Terminal probe (OSC 10/11, DA1) | ❌ | цвета фона и тема по ответу терминала; запрос и разбор | terminal_probe/, terminal_palette.rs, color.rs | — | `terminal-features` |
| Inline images (kitty / sixel / iTerm2) | ❌ | картинки внутри региона (G4), tmux pane-local (G10) | pets/image_protocol.rs, pets/sixel.rs, transcript_image.rs | — | `pets-ambient` |
| tmux / WSL / screen | 🟡 | буфер обмена умеет tmux; нет проверки passthrough для OSC 9/8 и графики | tui/terminal_stderr.rs, terminal_program_status.rs | src/services/clipboard/tmux.rs | `terminal-features` |
| Screen reader | ❌ | режим доступности: линейный вывод без анимаций | screen_reader.rs, screen_reader_windows.rs | — | `terminal-features` |
| Reduced motion | 🟡 | порт читает флаг только в футере effort; нет системной настройки (macOS/Windows) и общего отключения анимаций | motion.rs, system_motion.rs | src/pages/session/footer/effort.rs | `terminal-features` |
| OS session lock | ❌ | блокировка сессии ОС завершает голос | session_lock.rs, session_lock/ | — | `voice` |
| App shell | ✅ | маршрутизация потоков и событий — один `CodexApp`, один `ThreadModel` на тред (ADR-003) | app.rs, app/event_dispatch.rs, app/thread_routing.rs, app/thread_events.rs, app_event.rs | core/src/app.rs, core/src/threads.rs | — |
| Startup error и daemon recovery | 🟡 | экраны есть; 6 pending startup-шардов (prompts, flow) | startup_error.rs, daemon_recovery.rs, daemon_startup.rs, startup_recovery.rs | src/pages/startup/prompts/, host/src/startup.rs | `startup-rest` |
| Multi-backend | 🟡 | локальный in-process и fake; облачный и remote (выбор бэкенда, ожидание переключения) не прошли | app/backend_selection.rs, backend_registry.rs, backend_sessions.rs, cloud_switch_wait.rs, backend_id.rs | host/src/target.rs, host/src/cloud.rs, src/services/backend/ | `startup-rest` |
| Exit summary | ❌ | токены и подсказка resume при выходе | app/exit_summary.rs, token_usage.rs | — | `app-misc` |
| Update check | 🟡 | ячейка update-available и host/update.rs; нет действия обновления и кэша версий | updates.rs, updates_cache.rs, update_action.rs, update_versions.rs, npm_registry.rs | host/src/update.rs, src/pages/session/cells/update_available.rs | `startup-rest` |
| Server version notice | 🟡 | заметка есть; нет сравнения версий сервера и проверки на старте | app/server_version_notice.rs, version.rs | src/pages/session/cells/notice.rs | `app-misc` |
| Turn tips и tooltips | 🟡 | ячейка session_info показывает подсказки; нет `turn_tip` в ходе и каталога tooltips | turn_tip.rs, tooltips.rs, tooltips/, app/turn_tips.rs | src/pages/session/cells/session_info.rs | `ui-transcript-chrome` |
| Empty-state animation | 🟡 | порт: `blossom`; нет процедурного освещения и политики пустого состояния | empty_state_animation.rs, empty_state_animation/, app/empty_state_policy.rs, bottom_pane/empty_state_policy.rs | src/pages/session/blossom/ | `live-turn-sweep` |
| Assistant directives и git action directives | ✅ | директивы разобраны (`core/src/threads/directives.rs`); git-action-директивы показываются как текст | assistant_directives.rs, git_action_directives.rs | core/src/threads/directives.rs | — |
| Agents command center | 🟡 | страница `/agents` есть (src/pages/agents); остаток: agent status feed, навигация из composer, снимки | app/agents_overview*.rs, app/agent_center/, agent_navigation.rs, app/agent_status_feed.rs | src/pages/agents/, core/src/screens/agents/ | `agents-center` |
| Provider editor и cloud environment | ❌ | список/редактор провайдера и выбор cloud environment | app/provider/, app/cloud_switch_wait.rs, app_server_session/cloud_project.rs | host/src/providers.rs (факты) | `startup-rest` |
| ViewStack, CenteredView, DialogOverlay | ✅ | заменено rtwk: Navigator, sheet/dialog из rtwk | bottom_pane/view_stack.rs, bottom_pane/bottom_pane_view.rs | src/sheets/presenter.rs | — |
| Quit-confirm и task-running | ❌ | диалоги выхода при запущенных сессиях и при активной задаче | app/session_lifecycle.rs, chatwidget/interrupts.rs | — | `app-misc` |
| Базовые ячейки | ✅ | Plain/Labeled/Prefixed/Composite, tooltip, clear-screen header — как ячейки cells/* | history_cell/base.rs, history_cell/notices.rs | src/pages/session/cells/{text,one_row,note,notice}.rs | — |
| Onboarding: хост и welcome | ✅ | стек шагов — маршруты rtwk; анимация welcome | onboarding/mod.rs, onboarding/welcome.rs | src/pages/startup/onboarding/ | — |
| Onboarding: sign in, gateway, Bedrock, GCP | 🟡 | формы есть; хвосты G48 (перенос только по пробелам), G49, G50/G51 (приоритет flex-детей), G52 (путь по сегментам), G55 (последний символ секрета) | onboarding/auth*.rs, gateway*.rs, bedrock.rs, gcp.rs, device_login.rs, local_chatgpt_auth.rs | src/pages/startup/onboarding/signin/ | `startup-rest` |
| Onboarding: подтверждения и миграции | ✅ | trust, cwd, model migration, update, unarchive, OSS, startup hooks, перенос из других агентов — страницы S04-S06 со снимками | onboarding/trust_directory.rs, cwd_prompt.rs, model_migration.rs, update_prompt.rs, unarchive_prompt.rs, oss_selection.rs, startup_hooks_review.rs, external_agent_config_migration/ | src/pages/startup/prompts/, src/pages/startup/import/ | — |
| Публичные виджеты и Renderable | ✅ | заменено rtwk (Widget/Element) | public_widgets/, render/, ui_consts.rs | rtwk | — |
| Session log | ❌ | журнал событий сессии в файл для отладки | session_log.rs | — | `app-misc` |

### Карта экранов S01-S27

| Экран | Что | Статус | Чего нет | Пакет |
|---|---|---|---|---|
| S01 | update prompt | ✅ | — | — |
| S02 | onboarding: welcome, sign in | ✅ | — | — |
| S03 | onboarding: provider, gateway, Bedrock, GCP | 🟡 | хвосты G48-G55 | `startup-rest` |
| S04 | directory trust | ✅ | — | — |
| S05 | startup prompts (cwd, migration, hooks, unarchive, OSS, daemon recovery, startup error, Windows sandbox) | 🟡 | Windows sandbox (fw1-c8), startup error | `startup-rest` |
| S06 | import from other agents | ✅ | — | — |
| S10 | main chat | 🟡 | см. разделы «Транскрипт», «Живой ход», «Composer», «Футер» | `live-turn-sweep` |
| S11 | streaming and history cells, markdown, diff | 🟡 | см. «Транскрипт и ячейки» | `cells-sweep` |
| S12 | composer popovers | ✅ | — | — |
| S13 | approvals and questions | 🟡 | решение в транскрипте, write_stdin, черновики вопросов | `ui-approvals-rest` |
| S14 | generic selection dialog | 🟡 | model/permissions хвосты, small_dialogs | `dialogs-sweep` |
| S15 | settings dialogs | 🟡 | /config, /experimental, /skills, /hooks, keymap editor, провайдер | `ui-settings-feeds` |
| S16 | transcript pager, static pager, search | 🟡 | статический pager, поиск | `ui-pager-find` |
| S17 | backtrack | ✅ | — | — |
| S18 | resume / fork picker | 🟡 | 12 pending, превью транскрипта | `resume-rest` |
| S19 | /status card | 🟡 | usage треда, строка сервера, лимиты | `ui-usage-limits` |
| S20 | /usage dashboard | 🟡 | reset-flow, billing, подсказка | `ui-usage-limits` |
| S21 | agents command center, subagent switcher | 🟡 | agent status feed; switcher (/subagents) | `agents-center` |
| S22 | sidebar and side conversation | ❌ | обе панели | `sidebar` |
| S23 | small dialogs | 🟡 | daemon menu, app link, user verification, feedback | `dialogs-sweep` |
| S24 | pets | 🟡 | ambient, asset packs, графика | `pets-ambient` |
| S25 | voice / realtime | ❌ | вся функция | `voice` |
| S26 | ui_tree and inline visualization | ❌ | обе части | `ui-tree-viz` |
| S27 | inline mode | ❌ | режим целиком | `inline-mode` |

## Покрытие CSV (`research/codex-tui-features.csv`)

Каждая из 312 строк — в строке этого документа; ✅ — готово, иначе пакет закрывает строку.

| id | Название | Строка | Статус | Пакет |
|---|---|---|---|---|
| term.inline-viewport | Inline viewport (default non-fullscreen mode) | Inline viewport | ❌ | `inline-mode` |
| term.history-insertion | Insert finalized history lines into scrollback | Вставка завершённых строк в scrollback | ❌ | `inline-mode` |
| term.resize-reflow | Resize reflow: rebuild scrollback from source cells | Resize reflow | ❌ | `inline-mode` |
| term.alt-screen-fullscreen | Owned/fullscreen screen mode (alt screen + mouse) | Fullscreen (alt-screen + мышь) | ✅ | — |
| term.alt-screen-overlay | Temporary alt screen for pager overlays (inline mode) | Временный alt-screen для оверлеев | ❌ | `inline-mode` |
| term.suspend-ctrl-z | Suspend / resume (Ctrl+Z, SIGTSTP) | Suspend / resume | ❌ | `terminal-features` |
| term.bracketed-paste | Bracketed paste | Bracketed paste и kitty keyboard | ✅ | — |
| term.keyboard-enhancement | Kitty keyboard enhancement flags + key mode detection | Bracketed paste и kitty keyboard | ✅ | — |
| term.focus-events | Focus change events | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.sync-update | Synchronized output frames + frame scheduling | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.osc8-hyperlinks | OSC 8 hyperlinks | Гиперссылки в тексте (клик открывает) | 🟡 | `ui-select-copy` |
| term.osc9-notifications | Desktop notifications (OSC 9 / BEL) | Уведомления терминала о завершении хода / approval | 🟡 | `terminal-features` |
| term.osc52-clipboard | Clipboard copy (native, tmux, WSL, OSC 52) | Clipboard copy (native, tmux, WSL, OSC 52) | ✅ | — |
| term.clipboard-paste-image | Clipboard image paste + right-click paste | Картинки из буфера Ctrl+V / Alt+V | 🟡 | `composer-sweep` |
| term.mouse | Mouse support (wheel, click, drag, hover, right-click) | Мышь | ✅ | — |
| term.probe-colors | Terminal probing (OSC 10/11 colors, DA1, cursor pos) | Terminal probe (OSC 10/11, DA1) | ❌ | `terminal-features` |
| term.osc-title | Terminal title (OSC 0) with configurable items | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `footer-status-sweep` |
| term.images-kitty-sixel-iterm | Inline images (kitty graphics / sixel / iTerm2) | Inline images (kitty / sixel / iTerm2) | ❌ | `pets-ambient` |
| term.tmux-wsl-screen | tmux / WSL / screen adaptations | tmux / WSL / screen | 🟡 | `terminal-features` |
| term.windows-console | Windows console specifics | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.panic-restore | Panic hook and terminal restore | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.screen-reader | Screen reader / accessibility mode | Screen reader | ❌ | `terminal-features` |
| term.motion-reduce | Reduced-motion preference | Reduced motion | 🟡 | `terminal-features` |
| term.session-lock | OS session lock detection (voice ends) | OS session lock | ❌ | `voice` |
| term.external-editor | External editor handoff ($EDITOR) | Внешний редактор Ctrl+G | ❌ | `ui-composer-states` |
| app.shell | App event loop, thread routing, event dispatch | App shell | ✅ | — |
| app.startup-draft | Startup draft (composer before app-server ready) | Startup draft (черновик из аргументов, file-search star | 🟡 | `composer-sweep` |
| app.startup-error | Startup error / daemon recovery screens | Startup error и daemon recovery | 🟡 | `startup-rest` |
| app.multi-backend | Multi-backend sessions (local/cloud/remote) and banners | Multi-backend | 🟡 | `startup-rest` |
| app.exit-summary | Exit summary (token usage + resume hint) | Exit summary | ❌ | `app-misc` |
| app.update-check | Update checks / update action | Update check | 🟡 | `startup-rest` |
| app.server-version-notice | Server version mismatch notice | Server version notice | 🟡 | `app-misc` |
| onboarding.screen | Onboarding host (step stack, own mini event loop) | Onboarding: хост и welcome | ✅ | — |
| onboarding.welcome | Welcome step (animated) | Onboarding: хост и welcome | ✅ | — |
| onboarding.auth | Sign in (ChatGPT browser / device code / API key) | Onboarding: sign in, gateway, Bedrock, GCP | 🟡 | `startup-rest` |
| onboarding.gateway | Gateway picker + OAuth device prompt | Onboarding: sign in, gateway, Bedrock, GCP | 🟡 | `startup-rest` |
| onboarding.bedrock | Amazon Bedrock credentials + GovCloud guidance | Onboarding: sign in, gateway, Bedrock, GCP | 🟡 | `startup-rest` |
| onboarding.gcp | GCP (Vertex) setup | Onboarding: sign in, gateway, Bedrock, GCP | 🟡 | `startup-rest` |
| onboarding.trust-directory | Directory trust prompt | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.cwd-prompt | Resume/fork working directory prompt | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.model-migration | Model migration notice screen | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.update-prompt | Update prompt (update/skip) | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.unarchive-prompt | Unarchive session prompt | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.oss-selection | OSS provider selection (LM Studio / Ollama) | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.hooks-review | Startup hooks review | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.external-config-migration | Import from other agent (Claude Code) migration flow | Onboarding: подтверждения и миграции | ✅ | — |
| onboarding.windows-sandbox | Windows sandbox setup prompts | Windows sandbox: prompts, help-ссылки, fallback | ❌ | `ui-worktree-sandbox` |
| onboarding.security-setup | Security setup flow | Security setup / security prompts | ❌ | `ui-worktree-sandbox` |
| chat.widget | ChatWidget: transcript tail + bottom pane | Отправка сообщения Enter, индикатор working/«Thinking», | ✅ | — |
| chat.rendering-layout | Chat vertical layout | Автопрокрутка за хвостом, «вернуться вниз», строка ново | ✅ | — |
| chat.session-header | Session header card | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| chat.protocol-events | Protocol event handling | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `cells-sweep` |
| chat.interrupt | Interrupt turn (Esc / Ctrl+C) | Прерывание хода Esc | ✅ | — |
| chat.queued-input | Queued user messages while turn runs | Очередь сообщений (Tab / Enter в ходе) над composer | ✅ | — |
| chat.status-indicator | Working indicator (spinner, shimmer, elapsed, hint) | Отправка сообщения Enter, индикатор working/«Thinking», | ✅ | — |
| chat.status-line | Configurable status line | Строка статуса: модель, effort, cwd, context %, токены | ✅ | — |
| chat.footer | Composer footer (hints, quit reminder, esc hint) | Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режим | ✅ | — |
| chat.shortcut-overlay | Shortcut help ('?') | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `dialogs-sweep` |
| chat.effort-ignition | Reasoning effort ignition animation + shortcuts | Индикатор эффекта effort ignition | ✅ | — |
| chat.turn-tips | Turn tips / tooltips | Turn tips и tooltips | 🟡 | `ui-transcript-chrome` |
| chat.empty-state-animation | Empty-state animation (procedural lighting) + Daybreak | Empty-state animation | 🟡 | `live-turn-sweep` |
| chat.goal | Thread goal display + menu | Goal: меню и edit prompt, resume-paused prompt | 🟡 | `dialogs-sweep` |
| chat.plan-mode | Plan mode + implement-plan prompt | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| chat.review-mode | Review flows | Review-ячейки (баннер Current changes, request hint, pr | ✅ | — |
| chat.compaction | Context compaction progress | Compaction: статус, таймер, ячейка завершения | ❌ | `ui-safety-compaction` |
| chat.recap | Thread recap | /daybreak | ❌ | `ui-feedback-cmds` |
| chat.side-conversation | Side conversation docked pane | Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпи | ❌ | `ui-openers-side` |
| chat.sidebar | Sidebar (documents, files, links) with slide animation | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| chat.read-only-thread | Read-only thread notice | Read-only внешний writer: заметка вместо composer, пере | ✅ | — |
| chat.provider-error | Provider error + safety buffering/recourse/misalignment | Safety buffering: «повторить», подтверждение, короткое  | ❌ | `ui-safety-compaction` |
| chat.mcp-startup | MCP startup status | MCP startup: статус, lag, отмена, отказ как warning | ✅ | — |
| chat.reconnect | Reconnect/retry banners | Stream recovery: восстановление статуса после обрыва | ✅ | — |
| chat.rate-limits | Rate limit warnings / reset credits | Rate-limit подсказки/предупреждения во время хода | ❌ | `ui-usage-limits` |
| chat.copy-last | Copy last response (Ctrl+O, /copy picker) | Copy/export picker (кастомные клавиши, payload) | ❌ | `ui-select-copy` |
| chat.transcript-export | Export transcript as markdown | /export | ✅ | — |
| chat.reasoning-replay | Reasoning summary and replay | Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | — |
| chat.user-verification | User verification flow | Верификация пользователя (user verification) | ❌ | `dialogs-sweep` |
| chat.tool-lifecycle | Tool call lifecycle to active cells | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `cells-sweep` |
| composer.chat-composer | ChatComposer (multiline input + popups + history) | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.textarea | TextArea editor (emacs bindings, kill ring, undo) | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.vim-mode | Vim mode (normal/insert/visual, operators, text objects, sea | Vim: нормальный/вставка/операторы/текст-объекты/поиск ( | 🟡 | `composer-sweep` |
| composer.paste-burst | Paste-burst detection | Вставка: paste burst, placeholder большой вставки [Past | ✅ | — |
| composer.large-paste-placeholder | Large paste placeholders | Вставка: paste burst, placeholder большой вставки [Past | ✅ | — |
| composer.image-attachments | Image attachments ([Image #N]) | Картинки: локальные [Image #N], нумерация, удаление | ✅ | — |
| composer.history | Composer history | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.history-search | Reverse history search | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.undo-redo | Edit history undo/redo | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.draft-restore | Draft state and restore | Восстановление очищенного Ctrl+C prompt из истории | ✅ | — |
| composer.sparkle | Composer sparkle effect | Sparkle/«искры» над composer | 🟡 | `ui-composer-states` |
| composer.warning-notice | Inline composer warning | Состояния composer: disabled, shutdown, parent-owned, a | ❌ | `ui-composer-states` |
| composer.mouse | Composer mouse handling | Мышь в composer (курсор, выделение, вставка правой кноп | ❌ | `ui-composer-states` |
| composer.agents-navigation | Subagent navigation from composer | Left на пустом composer — Agents | ✅ | — |
| composer.slash-popup | Slash command popup (fuzzy filter) | Slash-popup: фильтр, стрелки, Tab/Enter, описание, скры | ✅ | — |
| composer.file-mention-popup | @ file search popup | $ skills / плагины / apps в упоминаниях | 🟡 | `composer-sweep` |
| composer.mentions-v2 | Mentions v2 popup (files, skills, plugins, apps, tasks; tabs | $ skills / плагины / apps в упоминаниях | 🟡 | `composer-sweep` |
| composer.skill-popup | Skill popup ($skill) | $ skills / плагины / apps в упоминаниях | 🟡 | `composer-sweep` |
| composer.voice-strip | Voice strip + realtime voice | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| composer.split-flap | Realtime split-flap display | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| composer.ide-context | IDE context integration | /daybreak | ❌ | `ui-feedback-cmds` |
| composer.quick-dials | Quick dials / saved presets | Quick dials (saved dials: save/clear picker) | ❌ | `ui-openers-side` |
| composer.assistant-directives | Assistant directives parsing | Assistant directives и git action directives | ✅ | — |
| view.list-selection | ListSelectionView (generic picker: search, tabs, columns, si | Список выбора: поиск, стрелки, цифры, disabled-строки,  | ✅ | — |
| view.approval-overlay | ApprovalOverlay (exec / patch / permissions / MCP) | Permissions request approval, MCP elicitation форма | ✅ | — |
| view.mcp-elicitation | McpServerElicitationOverlay (MCP form) | Permissions request approval, MCP elicitation форма | ✅ | — |
| view.request-user-input | RequestUserInputOverlay (model questions) | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| view.async-questions | AsyncQuestions (non-blocking question stack) | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| view.app-link | AppLinkView (connect ChatGPT app) | Apps/connectors: список, статус, install-only | 🟡 | `dialogs-sweep` |
| view.custom-prompt | CustomPromptView (text prompt) | Custom review prompt | ✅ | — |
| view.feedback-note | FeedbackNoteView | Feedback: выбор, consent upload, диагностика, note | 🟡 | `ui-feedback-cmds` |
| view.multi-select | MultiSelectPicker | Custom review prompt | ✅ | — |
| view.status-line-setup | StatusLineSetupView (/statusline) | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `footer-status-sweep` |
| view.title-setup | TerminalTitleSetupView (/title) | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `footer-status-sweep` |
| view.skills-toggle | SkillsToggleView | Hooks popup: список, диагностика | ❌ | `ui-settings-feeds` |
| view.experimental-features | ExperimentalFeaturesView | Hooks popup: список, диагностика | ❌ | `ui-settings-feeds` |
| view.hooks-browser | HooksBrowserView (/hooks) | Hooks popup: список, диагностика | ❌ | `ui-settings-feeds` |
| view.memories-settings | MemoriesSettingsView | Hooks popup: список, диагностика | ❌ | `ui-settings-feeds` |
| view.user-verification | UserVerificationView | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `dialogs-sweep` |
| view.warnings | WarningsView (/warnings) | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `dialogs-sweep` |
| view.config-panel | ConfigPanel (/config, tabbed settings with sub-pickers) | Settings: Config / Statusline / Title / Experimental /  | 🟡 | `ui-config-panel` |
| view.keymap-capture | KeymapCaptureView | /keymap — редактор клавиш | ❌ | `ui-keymap-editor` |
| view.keymap-debug | KeymapDebugView | /keymap — редактор клавиш | ❌ | `ui-keymap-editor` |
| view.keymap-picker | Keymap setup picker | /keymap — редактор клавиш | ❌ | `ui-keymap-editor` |
| view.agents-overview | AgentsOverviewView (/agents command center) | Agents command center | 🟡 | `agents-center` |
| view.provider-editor | Provider editor / list | Provider editor и cloud environment | ❌ | `startup-rest` |
| view.daemon-menu | Daemon menu (/daemon) | Quick dials (saved dials: save/clear picker) | ❌ | `ui-openers-side` |
| view.view-stack | ViewStack / CenteredView / DialogOverlay | ViewStack, CenteredView, DialogOverlay | ✅ | — |
| view.actionable-banner | InlineBanner above composer | Баннер бэкенда (нумерованные строки, CTA, отклонение по | ✅ | — |
| view.unified-exec-footer | Unified exec footer (background terminals) | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `ui-exec-mcp` |
| popup.model-picker | Model & reasoning effort picker | Model picker: каталог, discovery, bedrock/custom displa | 🟡 | `dialogs-sweep` |
| popup.permissions | Permissions / approval mode popup + full access confirm | Permissions popup: пресеты, профили, retry discovery | 🟡 | `dialogs-sweep` |
| popup.theme-picker | Syntax theme picker with live preview | Settings: Theme, Memories (из чата) | 🟡 | `ui-settings-feeds` |
| popup.pets | Pet picker with animated preview | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| popup.skills-menu | Skills action menu | Hooks popup: список, диагностика | ❌ | `ui-settings-feeds` |
| popup.plugins | Plugins browser (marketplaces, detail, install/upgrade) | Plugins: список, вкладки, поиск, деталь, install, marke | 🟡 | `dialogs-sweep` |
| popup.apps | Apps / connectors list | Apps/connectors: список, статус, install-only | 🟡 | `dialogs-sweep` |
| popup.usage | Usage popup + reset flow | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `ui-usage-limits` |
| popup.worktrees | Worktree browser / managed worktree actions | Worktree/checkout picker | 🟡 | `ui-worktree-sandbox` |
| popup.copy-picker | Copy to clipboard picker | Copy/export picker (кастомные клавиши, payload) | ❌ | `ui-select-copy` |
| popup.voice-settings | Voice / sound device / channel pickers | Voice (диалог, F8) | ❌ | `voice` |
| popup.tui-mode | TUI mode (Inline / Fullscreen) | Settings: Config / Statusline / Title / Experimental /  | 🟡 | `ui-config-panel` |
| popup.archive-delete-confirm | Archive / Delete session confirmations | Confirm: archive/delete треда | ✅ | — |
| popup.quit-confirm | Quit and stop running sessions | Quit-confirm и task-running | ❌ | `app-misc` |
| popup.task-running | Task is still running choice | Quit-confirm и task-running | ❌ | `app-misc` |
| popup.goal-replace | Goal replace / resume prompts | Goal: меню и edit prompt, resume-paused prompt | 🟡 | `dialogs-sweep` |
| popup.daybreak-turn-on | Turn on Daybreak for next request | /daybreak | ❌ | `ui-feedback-cmds` |
| popup.cloud-environment | Cloud environment picker | Provider editor и cloud environment | ❌ | `startup-rest` |
| popup.subagents | Subagents switcher | /subagents (multi-agents picker) | ❌ | `ui-openers-side` |
| popup.statusline-provider | Status line provider picker | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `footer-status-sweep` |
| popup.feedback | Feedback rating + consent | Feedback: выбор, consent upload, диагностика, note | 🟡 | `ui-feedback-cmds` |
| popup.session-unavailable | Unable to resume session error | /resume | 🟡 | `resume-rest` |
| popup.windows-sandbox | Windows sandbox prompts | Windows sandbox: prompts, help-ссылки, fallback | ❌ | `ui-worktree-sandbox` |
| popup.misalignment-policy | Chat paused as a precaution | Misalignment/guardian stop треда | ❌ | `ui-safety-compaction` |
| popup.debug-config | /debug-config layers | /daybreak | ❌ | `ui-feedback-cmds` |
| overlay.transcript-pager | Transcript pager overlay (Ctrl+T, inline mode) | Статический pager (/diff, вывод): строки, страницы, Hom | 🟡 | `ui-pager-find` |
| overlay.static-pager | Static pager overlay (diff, long output) | Статический pager (/diff, вывод): строки, страницы, Hom | 🟡 | `ui-pager-find` |
| overlay.analytics | Analytics dashboard (usage charts, tasks, plan, chats, model | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `ui-usage-limits` |
| overlay.backtrack | Backtrack (Esc Esc): pick earlier prompt, revert, edit | Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | — |
| overlay.diff-view | /diff git diff viewer | /diff | ❌ | `ui-pager-find` |
| overlay.inline-visualization | Inline visualization viewer | Дерево UI (ui-tree) | ? | `ui-tree-viz` |
| overlay.ui-tree | UI tree renderer (model-emitted declarative UI) | Дерево UI (ui-tree) | ? | `ui-tree-viz` |
| overlay.sidebar-chooser | Sidebar chooser | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| transcript.viewport | Owned-screen transcript viewport | PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | `ui-transcript-chrome` |
| transcript.selection-copy | Mouse selection + copy-on-select | Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | `ui-select-copy` |
| transcript.select-mode | Keyboard select mode | Режим выделения с клавиатуры (select mode) | ❌ | `ui-select-copy` |
| transcript.search | Transcript find | Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | `ui-pager-find` |
| transcript.chrome | Disclosure, turn details, sticky prompt header, follow butto | Закреплённая шапка prompt над транскриптом (pinned head | ❌ | `ui-transcript-chrome` |
| transcript.thread-transcript | Thread transcript projection | Replay истории при resume/fork: сообщения, tool, reason | ✅ | — |
| transcript.resume-preview | Transcript preview in resume picker | /resume | 🟡 | `resume-rest` |
| resume.picker | Resume / fork session picker | /resume | 🟡 | `resume-rest` |
| status.card | /status card | /status — карточка статуса | 🟡 | `ui-usage-limits` |
| status.rate-limits | Rate limit bars | Rate-limit подсказки/предупреждения во время хода | ❌ | `ui-usage-limits` |
| streaming.controller | Streaming controller (deltas to commit ticks) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| streaming.markdown-stream | Markdown streaming (fences, table holdback, prose preview) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| markdown.renderer | Markdown renderer | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| markdown.math | Math rendering | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| markdown.mermaid | Mermaid diagram as text | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| markdown.syntax-highlight | Syntax highlighting + themes + wrapping utils | Подсветка кода и diff в ответах | ✅ | — |
| diff.render | Diff rendering | /diff | ❌ | `ui-pager-find` |
| keymap.ctx-app | Keymap context: App (panes, sidebar, transcript open/find/se | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-chat | Keymap context: Chat (presets, voice, interrupt, effort, per | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-composer | Keymap context: Composer (submit, queue, shortcuts, history  | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-editor | Keymap context: Editor textarea | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-vim | Keymap context: Vim normal/operator/text-object | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-pager | Keymap context: Pager | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-list | Keymap context: List navigation | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.ctx-agents | Keymap context: Agents command center | Agents command center | 🟡 | `agents-center` |
| keymap.ctx-approval | Keymap context: Approval | Tab — в очередь (сообщения) | 🟡 | `ui-composer-states` |
| keymap.runtime | Runtime keymap: config, chords, leader, conflicts, reserved  | /keymap — редактор клавиш | ❌ | `ui-keymap-editor` |
| cell.plain | Plain lines cell | Базовые ячейки | ✅ | — |
| cell.labeled-tool | Labeled tool cell | Базовые ячейки | ✅ | — |
| cell.web-hyperlink | Web hyperlink cell | Гиперссылки в тексте (клик открывает) | 🟡 | `ui-select-copy` |
| cell.prefixed-wrapped | Prefixed wrapped text cell | Базовые ячейки | ✅ | — |
| cell.composite | Composite cell | Базовые ячейки | ✅ | — |
| cell.user | User message cell | Сообщение пользователя (блок с фоном, пути, картинки-пл | ✅ | — |
| cell.agent-message | Agent message cell | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| cell.agent-markdown | Agent markdown cell (source-backed reflow) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| cell.streaming-agent-tail | Streaming agent tail | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `markdown-gaps` |
| cell.reasoning-summary | Reasoning summary cell | Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | — |
| cell.exec | Exec cell (command, output preview, spinner, exploring group | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `cells-sweep` |
| cell.unified-exec-interaction | Unified exec interaction cell | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `ui-exec-mcp` |
| cell.unified-exec-processes | Background terminals list | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `ui-exec-mcp` |
| cell.patch | Patch cell (edited files with diffs) | Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | — |
| cell.patch-failure | Patch failure cell | Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | — |
| cell.view-image | View image cell | Ячейка просмотра/генерации изображений | 🟡 | `cells-sweep` |
| cell.image-generation | Image generation cell | Ячейка просмотра/генерации изображений | 🟡 | `cells-sweep` |
| cell.mcp-tool-call | MCP tool call cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.mcp-inventory-loading | MCP inventory loading cell | Инвентарь MCP (вывод /mcp) | ❌ | `ui-exec-mcp` |
| cell.dynamic-tool-call | Dynamic tool call cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.web-search | Web search cell | Ячейка web search (запрос, детали, усечение) | ✅ | — |
| cell.computer-activity | Computer-use activity cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.hook | Hook run cell | Ячейка hook (запущен/завершён) | ✅ | — |
| cell.proposed-plan | Proposed plan cell | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.proposed-plan-stream | Proposed plan stream cell | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.streaming-plan-tail | Streaming plan tail | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.plan-update | Plan update checklist cell | Ячейка плана (update_plan) и предложенный план | ✅ | — |
| cell.final-message-separator | Final message separator | Final «Worked for …» разделитель с длительностью | 🟡 | `cells-sweep` |
| cell.session-header | Session header card | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| cell.session-info | Session info cell | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| cell.session-notice | Session notice cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.forked-thread | Forked thread notice | Строка «Forked thread» / имя треда в истории | ❌ | `ui-worktree-sandbox` |
| cell.clear-screen-header | Clear-screen header | Базовые ячейки | ✅ | — |
| cell.tooltip | Tooltip cell | Базовые ячейки | ✅ | — |
| cell.update-available | Update available notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.safety-access-block | Safety access block notice | Safety buffering: «повторить», подтверждение, короткое  | ❌ | `ui-safety-compaction` |
| cell.deprecation-notice | Deprecation notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.interruption-notice | Interruption notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.thread-recap-loading | Recap loading cell | /daybreak | ❌ | `ui-feedback-cmds` |
| cell.thread-recap | Thread recap cell | /daybreak | ❌ | `ui-feedback-cmds` |
| cell.startup-warnings | Startup warnings cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.warning | Warning cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.delegated-message | Delegated (subagent) message cell | Ячейка delegated (делегированный вывод tool) | ✅ | — |
| cell.request-user-input-result | Request-user-input result cell | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| cell.agent-status | Agent status feed cell | Ячейка delegated (делегированный вывод tool) | ✅ | — |
| cell.file | Sidebar file/document cell | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| cell.split-flap-transcript | Split-flap transcript cell | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| cell.status | Status card cell | /status — карточка статуса | 🟡 | `ui-usage-limits` |
| cell.approval-decision | Approval decision cells (approved/denied/guardian) | Решение approval в транскрипте (Approved/Denied + коман | ❌ | `ui-approvals-rest` |
| cell.activity-group | Activity groups (collapsed tool activity) | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `cells-sweep` |
| cell.spoken-artifacts | Spoken artifacts / local and web link annotation | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| slash.model | /model - choose what model and reasoning effort to use | /model — выбор модели и effort | 🟡 | `dialogs-sweep` |
| slash.presets | /presets - run or save reusable prompts and slash commands | /presets — выбор пресета (Daybreak presets) | ❌ | `ui-openers-side` |
| slash.daybreak | /daybreak - turn Daybreak on or off | /daybreak | ❌ | `ui-feedback-cmds` |
| slash.ide | /ide - include current selection, open files, and other cont | /ide — контекст IDE | ❌ | `ui-feedback-cmds` |
| slash.permissions | /permissions - choose what Codex is allowed to do | /permissions — approvals/профили | 🟡 | `dialogs-sweep` |
| slash.keymap | /keymap - remap TUI shortcuts | /keymap — редактор клавиш | ❌ | `ui-keymap-editor` |
| slash.vim | /vim - toggle Vim mode for the composer | /vim — переключить Vim composer | ✅ | — |
| slash.setup-default-sandbox | /setup-default-sandbox - set up elevated agent sandbox | /setup-default-sandbox (Windows) | ❌ | `ui-worktree-sandbox` |
| slash.experimental | /experimental - toggle experimental features | /experimental — feature-флаги | ❌ | `ui-settings-feeds` |
| slash.approve | /approve - approve one retry of a recent auto-review denial | /approve — Auto-review | ✅ | — |
| slash.memories | /memories - configure memory use and generation | /memories | 🟡 | `ui-settings-feeds` |
| slash.skills | /skills - use skills to improve how Codex performs specific  | /skills | ❌ | `ui-settings-feeds` |
| slash.import | /import - import setup, this project, and recent chats from  | /import — перенос из других агентов | ✅ | — |
| slash.hooks | /hooks - view and manage lifecycle hooks | /hooks | ❌ | `ui-settings-feeds` |
| slash.review | /review - review my current changes and find issues | /review — выбор ветки/коммита/custom | ✅ | — |
| slash.rename | /rename - rename the current thread | /rename — имя треда | ✅ | — |
| slash.new | /new - start a new chat during a conversation | /new | 🟡 | `ui-worktree-sandbox` |
| slash.archive | /archive - archive this session | /archive | ✅ | — |
| slash.delete | /delete - permanently delete this session | /delete | ✅ | — |
| slash.resume | /resume - resume a saved chat | /resume | 🟡 | `resume-rest` |
| slash.fork | /fork - fork the current chat | /fork | 🟡 | `ui-worktree-sandbox` |
| slash.worktree | /worktree - start or continue a conversation in a new worktr | /worktree | 🟡 | `ui-worktree-sandbox` |
| slash.app | /app - continue this session in the Desktop app | /app — открыть в desktop-приложении | ✅ | — |
| slash.init | /init - create an AGENTS.md file with instructions for Codex | /init | ✅ | — |
| slash.compact | /compact - summarize conversation to prevent hitting the con | /compact | 🟡 | `ui-safety-compaction` |
| slash.config | /config - configure Codex preferences | /config | ❌ | `ui-config-panel` |
| slash.recap | /recap - summarize the current conversation now | /recap | ❌ | `ui-feedback-cmds` |
| slash.plan | /plan - switch to Plan mode | /plan [prompt] | ✅ | — |
| slash.voice | /voice - start or stop voice; use /voice settings to choose  | /voice | ❌ | `voice` |
| slash.goal | /goal - set or view the goal for a long-running task | /goal [цель/pause/resume/clear] | 🟡 | `dialogs-sweep` |
| slash.agents | /agents - open the agent command center | /agents — центр агентов | ✅ | — |
| slash.side | /side - start a side conversation | /side, /btw | ❌ | `ui-openers-side` |
| slash.btw | /btw - start a side conversation | /side, /btw | ❌ | `ui-openers-side` |
| slash.copy | /copy - copy the last response or part of it | /copy — копировать ответ | ❌ | `ui-select-copy` |
| slash.export | /export - export the conversation as markdown | /export | ✅ | — |
| slash.raw | /raw - toggle raw scrollback mode for copy-friendly terminal | /raw | ✅ | — |
| slash.tui | /tui - choose the TUI mode for the next launch | /tui — быстрые настройки TUI | ❌ | `ui-config-panel` |
| slash.diff | /diff - show git diff (including untracked files) | /diff | ❌ | `ui-pager-find` |
| slash.mention | /mention - mention a file | /mention | ✅ | — |
| slash.status | /status - show current session configuration and token usage | /status — карточка статуса | 🟡 | `ui-usage-limits` |
| slash.daemon | /daemon - Manage the local background server | /daemon | ❌ | `ui-openers-side` |
| slash.warnings | /warnings - view retained warnings and diagnostic details | /warnings (и F2) | ✅ | — |
| slash.cd | /cd - change the current working directory | /cd <путь> | ✅ | — |
| slash.pwd | /pwd - show the current working directory | /pwd, /cwd | ✅ | — |
| slash.usage | /usage - view account usage or use a usage limit reset | /usage | 🟡 | `ui-usage-limits` |
| slash.debug-config | /debug-config - show config layers and requirement sources f | /debug-config | ❌ | `ui-feedback-cmds` |
| slash.title | /title - configure which items appear in the terminal title | /title — терминальный title | 🟡 | `footer-status-sweep` |
| slash.statusline | /statusline - configure which items appear in the status lin | /statusline | 🟡 | `ui-usage-limits` |
| slash.theme | /theme - choose a syntax highlighting theme | /theme | ✅ | — |
| slash.pets | /pets - choose or hide the terminal pet | /pets, /pet | 🟡 | `pets-ambient` |
| slash.mcp | /mcp - list MCP tools; /mcp verbose or /mcp login <name> [-- | /mcp | ❌ | `ui-exec-mcp` |
| slash.apps | /apps - manage apps | /apps | ✅ | — |
| slash.plugins | /plugins - browse plugins | /plugins | 🟡 | `dialogs-sweep` |
| slash.logout | /logout - log out of Codex | /logout | ❌ | `ui-feedback-cmds` |
| slash.quit | /quit - exit Codex | /quit, /exit | ✅ | — |
| slash.exit | /exit - exit Codex | /quit, /exit | ✅ | — |
| slash.feedback | /feedback - send logs to maintainers or save a ZIP | /feedback | 🟡 | `ui-feedback-cmds` |
| slash.rollout | /rollout - print the rollout file path | /rollout | ✅ | — |
| slash.ps | /ps - list background terminals | /ps — фоновые терминалы | ❌ | `ui-exec-mcp` |
| slash.stop | /stop - stop all background terminals | /stop, /clean | ✅ | — |
| slash.clear | /clear - clear the terminal and start a new chat | /goal [цель/pause/resume/clear] | 🟡 | `dialogs-sweep` |
| slash.test-approval | /test-approval - test approval request | /test-approval | ✅ | — |
| slash.subagents | /subagents - switch between this session's subagents | /subagents (multi-agents picker) | ❌ | `ui-openers-side` |
| slash.debug-m-drop | /debug-m-drop - DO NOT USE | /debug-m-drop, /debug-m-update | 🟡 | `ui-feedback-cmds` |
| slash.debug-m-update | /debug-m-update - DO NOT USE | /debug-m-drop, /debug-m-update | 🟡 | `ui-feedback-cmds` |
| pets.ambient | Terminal pet (animated companion via kitty/sixel) | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| pets.asset-pack | Pet asset packs / catalog | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| analytics.panels | Analytics sections | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `ui-usage-limits` |
| analytics.plot | Plot engine | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `ui-usage-limits` |
| other.tooltips | Startup tooltips / announcements | Turn tips и tooltips | 🟡 | `ui-transcript-chrome` |
| other.public-widgets | Public widgets (composer input) | Публичные виджеты и Renderable | ✅ | — |
| other.session-log | Session log | Session log | ❌ | `app-misc` |
| other.renderable-trait | Renderable trait + UI consts | Публичные виджеты и Renderable | ✅ | — |

## Покрытие модулей oracle

Все 1230 файлов `codex-rs/tui/src` (с тестами) — по строке в `docs/rtwk-port/reports/port-map-modules.tsv` (путь, статус, пакет, место в порте). Сводка:

| Пакет | Файлов oracle | ✅ | 🟡 | ❌ |
|---|---|---|---|---|
| `live-turn-sweep` | 243 | 0 | 243 | 0 |
| `startup-rest` | 125 | 0 | 125 | 0 |
| `composer-sweep` | 112 | 0 | 112 | 0 |
| `ui-usage-limits` | 74 | 0 | 74 | 0 |
| — (заменено rtwk / готово) | 54 | 54 | 0 | 0 |
| `dialogs-sweep` | 53 | 0 | 53 | 0 |
| `ui-settings-feeds` | 49 | 0 | 49 | 0 |
| `ui-select-copy` | 48 | 0 | 33 | 15 |
| `ui-transcript-chrome` | 46 | 0 | 46 | 0 |
| `markdown-gaps` | 44 | 0 | 44 | 0 |
| `ui-exec-mcp` | 39 | 0 | 39 | 0 |
| `ui-approvals-rest` | 39 | 0 | 39 | 0 |
| `terminal-features` | 33 | 0 | 0 | 33 |
| `footer-status-sweep` | 31 | 0 | 31 | 0 |
| `voice` | 31 | 0 | 0 | 31 |
| `agents-center` | 29 | 0 | 29 | 0 |
| `cells-sweep` | 26 | 0 | 26 | 0 |
| `inline-mode` | 25 | 0 | 0 | 25 |
| `ui-feedback-cmds` | 18 | 0 | 4 | 14 |
| `ui-worktree-sandbox` | 16 | 0 | 16 | 0 |
| `ui-safety-compaction` | 15 | 0 | 15 | 0 |
| `resume-rest` | 14 | 0 | 14 | 0 |
| `pets-ambient` | 13 | 0 | 13 | 0 |
| `ui-openers-side` | 12 | 0 | 0 | 12 |
| `ui-pager-find` | 12 | 0 | 0 | 12 |
| `ui-tree-viz` | 12 | 0 | 0 | 12 |
| `ui-keymap-editor` | 11 | 0 | 11 | 0 |
| `ui-composer-states` | 2 | 0 | 0 | 2 |
| `app-misc` | 2 | 0 | 0 | 2 |
| `sidebar` | 2 | 0 | 0 | 2 |

## Тесты: 1228 pending по пакетам

Правило отнесения (`tests_rules.py`): тест, чья причина названа семенем/входом/именами кадров kit, — в `kit-seeds` (это обвязка, пока она не влита, функция не зазеленит тест); остальные — по имени теста, затем по причине, затем по шарду. Неточность правил — не пробел: пакет получает свой список, перенос теста между пакетами — правка одной строки в `unpend/`.

| Пакет | Pending | Шарды (число) |
|---|---|---|
| `ui-pager-find` | 17 | pager 7, transcript_view_4 7, approvals_2 1, transcript_view_1 1, transcript_view_2 1 |
| `ui-select-copy` | 103 | transcript_view_4 50, chat_commands_2 14, chat_popups_2 7, transcript_view_3 6, transcript_view_5 6, transcript_view_2 5 |
| `ui-transcript-chrome` | 98 | transcript_view_2 28, transcript_view_5 27, transcript_view_1 22, transcript_view_3 8, textarea 4, markdown_streaming_5 2 |
| `ui-composer-states` | 16 | chat_commands_2 4, composer_4 3, chat_live_1 2, textarea 2, chat_commands_3 1, chat_live_4 1 |
| `composer-sweep` | 82 | composer_4 33, textarea 29, chat_live_5 6, composer_1 3, composer_3 3, chat_commands_1 2 |
| `ui-exec-mcp` | 20 | chat_approvals_1 15, cells_3 3, cells_4 2 |
| `ui-safety-compaction` | 34 | chat_approvals_2 10, chat_live_1 8, chat_live_2 7, chat_live_3 3, markdown_streaming_4 2, cells_2 1 |
| `ui-approvals-rest` | 59 | chat_approvals_1 29, chat_approvals_2 17, approvals_2 7, approvals_1 6 |
| `ui-usage-limits` | 60 | chat_live_6 23, usage 14, chat_status_1 12, chat_status_2 6, status 3, chat_status_3 2 |
| `ui-settings-feeds` | 5 | chat_popups_4 4, chat_status_2 1 |
| `ui-keymap-editor` | 4 | chat_commands_3 3, transcript_view_1 1 |
| `ui-openers-side` | 9 | chat_live_5 6, chat_status_3 2, transcript_view_2 1 |
| `ui-worktree-sandbox` | 26 | chat_popups_2 14, chat_popups_4 7, chat_live_2 4, resume_1 1 |
| `ui-feedback-cmds` | 29 | small_dialogs_1 14, transcript_view_1 3, transcript_view_5 3, chat_status_1 2, pane_footer_1 2, cells_4 1 |
| `cells-sweep` | 56 | cells_1 14, cells_4 14, cells_5 13, renderer_internals 6, cells_2 5, cells_3 4 |
| `markdown-gaps` | 112 | markdown_streaming_2 34, markdown_streaming_1 30, markdown_streaming_4 26, markdown_streaming_5 18, markdown_streaming_3 4 |
| `dialogs-sweep` | 53 | chat_popups_1 9, composer_popups 9, small_dialogs_2 6, chat_live_4 5, chat_popups_2 5, small_dialogs_1 5 |
| `footer-status-sweep` | 148 | pane_footer_2 44, chat_status_1 33, chat_status_2 30, pane_footer_1 22, chat_status_3 3, composer_popups 3 |
| `live-turn-sweep` | 46 | chat_live_1 18, chat_commands_3 9, chat_live_2 8, chat_live_5 6, chat_commands_2 2, chat_live_6 2 |
| `voice` | 1 | composer_popups 1 |
| `inline-mode` | 12 | markdown_streaming_2 4, textarea 3, markdown_streaming_5 2, cells_4 1, chat_popups_2 1, composer_1 1 |
| `startup-rest` | 13 | chat_live_3 4, startup_prompts 3, chat_commands_3 2, signin 2, cells_1 1, cells_2 1 |
| `resume-rest` | 5 | resume_2 3, resume_1 2 |
| `kit-seeds` | 220 | usage 29, small_dialogs_1 25, chat_commands_1 22, approvals_1 20, chat_commands_3 16, chat_status_3 14 |

По тегам владельца шарда: chat-live 627, dialogs-pane 248, ui-pane 189, chat-nav-pane 103, routes-lists 55, startup-prompts 3, onboarding 2, startup-flow 1.

## Связь с прежней картой

Пакеты P1-P21 из `chat-feature-map.md` («Пакеты для портеров») заменены пакетами этого документа: ядровые половины P1-P19 — fw1-c1..c9, UI-половины и sweep — `ui-*`, `*-sweep`; P20/P21 — framework-пакеты. Строки `chat-feature-map.md` с задачей «устарела» теперь имеют пакет здесь.
