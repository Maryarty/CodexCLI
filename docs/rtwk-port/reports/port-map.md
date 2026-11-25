# Карта всего порта: oracle (codex-rs/tui) против порта (codex-rs/tui-rtwk)

Дата: 2026-11-24, база 01741db4b (батчи 10-12). Документ собран скриптом `tools/rtwk-port/map-full/build.py` из данных рядом с ним: правится дата, перезапуск. Проверка непротиворечивости — `tools/rtwk-port/map-full/check.py`. Карта экрана чата `chat-feature-map.md` остаётся (подробные ячейки, числа тестов, архитектура ADR-013) и ссылается сюда; здесь — вся функциональность oracle без исключений, пакеты работ по владению файлами, и 935 pending-тестов по пакетам.

**Итог.** Строк-функций: 328 (экран чата — 293 из `chat-feature-map.md` + 35 вне экрана чата): ✅ 138, 🟡 86, ❌ 99, «?» 5. Все 312 строк `codex-tui-features.csv` покрыты строкой (раздел «Покрытие CSV»), все 1230 файлов `codex-rs/tui/src` — пакетом или «✅ rtwk» (`port-map-modules.tsv`), все 24 экранов карты S01-S27 (в карте нет S07-S09) — строкой. Пакетов: 30 (волна 1 — 9, влита; волна 2 — 11; волна 3 — 10); кроме волны 1 — 46-49 прогонов порт-агента (прогон ≈150-250 ходов, ≈1500 написанных строк). Pending-тестов oracle: 935, у каждого пакет (раздел «Тесты»).

## Как читать

- Статус: ✅ работает или заменено rtwk; 🟡 частично; ❌ нет; «?» не удалось установить (первый шаг пакета — проверить). Чего нет — слово в слово из `chat-feature-map.md` для строк чата (их числа и оговорки не перебирались), для остальных — по коду на 01741db4b.
- Oracle — файлы относительно `codex-rs/tui/src`; порт — относительно `codex-rs/tui-rtwk` (`(нет)` — файла ещё нет, его создаёт пакет). Пакет — владелец, чьими файлами строка закрывается; «ядро: fw1-cN» — ядровая половина влита в волне 1 (`dc3417d92`), её файлы здесь не переплановываются, пакет делает UI-половину и вешает проверку.
- Нет списка «не пакеты»: строка, упирающаяся в rtwk, получает пакет, первый шаг которого называет, что просить у rtwk (поле «первый шаг»).

## Правила владения

1. **Один файл — один пакет.** Каждый production-файл порта (`src`, `core/src`, `host/src`, `tests/rules`) в точности у одного пакета: явным путём, `dir/**` или `rest:dir` (все ещё не названные файлы каталога). `check.py` падает при пересечении файла или пути; вывод проверки (0 пересечений, 0 файлов без владельца) — в `task_done`.
2. **Хаб не отдаётся одному пакету на все фичи** (ADR-012). Фича входит в приложение только дверьми слайса: `feature_list!`/`target_of` (слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможность хоста, строка в `hub_budget.txt`), `ThreadModel.features` (состояние треда). Правка хаба — только СЖАТИЕ, и это работа двух framework-пакетов: **fw-core-hubs** (общий вариант `SessionIntent::Feature`, конец арма NotAvailable, одна дверь возможностей хоста) и **fw-ui-registry** (реестры действий клавиш, целей open, ячеек, режимов и тестовых корней; `session_page.rs` — один владелец с причиной). Остальные пакеты называют только файлы своего слайса, своих страниц и свои тесты.
3. **Двери.** `core/src/features/mod.rs` и `src/features/mod.rs` (`feature_list!`, `sheet_list!`), `key_actions/mod.rs` (`action_list!`), `cells/mod.rs` (`cell_list!`), `transcript/mod.rs` (`mode_list!`) — по одной строке на слайс; их не владеет никто, слайс дописывает свою строку (слияние без конфликта по смыслу). Это единственное исключение из пункта 1.
4. **Тесты.** Файлы тестов (шарды `tests/oracle/*.rs`, `tests/pages/*`) не входят в проверку пересечений: шард собран из независимых функций, пакет правит только функции из своего списка `tools/rtwk-port/map-full/unpend/<пакет>.txt` (сгенерирован, по одному `шард::тест` в строке); несколько пакетов в одном шарде сливаются построчно. Новые тестовые файлы пакета — `tests/pages/<пакет>_*.rs`, строки `mod` в корнях добавляет реестр fw-ui-registry.
5. **Порядок.** Волна 1 (fw1-c1..c9) влита (`dc3417d92`), её файлы — у пакетов W2. Волна 2: сначала framework-пакеты, пока они не влиты, UI-пакеты стоят; затем восемь UI-пакетов `w2-*` параллельно (пересечений нет). Волна 3: целые фичи (voice, sidebar, inline, ui_tree, pets, startup, …); часть упирается в rtwk (см. «первый шаг»).

## Пакеты

| Пакет | Волна | Прогонов | Строк | Строк закрыто | Pending-тестов | Зависит от |
|---|---|---|---|---|---|---|
| `fw1-c1` — core: /diff-документ pager, /presets, /daemon, /subagents, app link, provider switch | W1 (влита) | 1 | ≈750 | ядро для 53 | 0 | — |
| `fw1-c2` — core: внешний редактор Ctrl+G, очередь команд, состояния composer, /side /btw | W1 (влита) | 1 | ≈1150 | ядро для 67 | 0 | — |
| `fw1-c3` — core: /experimental /skills /hooks, /config /tui /keymap, аргументные формы (settings-слайс) | W1 (влита) | 1-2 | ≈1500 | ядро для 36 | 0 | — |
| `fw1-c4` — core: /copy и Ctrl+O, /logout /ide /daybreak /recap /debug-config /debug-m-* | W1 (влита) | 1 | ≈950 | ядро для 53 | 0 | — |
| `fw1-c5` — core: exploration-группы, фоновые терминалы и /ps, инвентарь /mcp | W1 (влита) | 1 | ≈900 | ядро для 14 | 0 | — |
| `fw1-c6` — core: safety buffering, misalignment, guardian, compaction | W1 (влита) | 1 | ≈800 | ядро для 14 | 0 | — |
| `fw1-c7` — core: сброс лимита /usage, billing, /status thread usage и строка сервера, предупреждения лимитов | W1 (влита) | 1 | ≈900 | ядро для 20 | 0 | — |
| `fw1-c8` — core: worktree/checkout для /new /fork /worktree, «Forked thread», Windows sandbox, security setup | W1 (влита) | 1 | ≈1000 | ядро для 26 | 0 | — |
| `fw1-c9` — core: /feedback (note, upload), решение approval, вопросы (черновики, очередь) | W1 (влита) | 1 | ≈700 | ядро для 50 | 0 | — |
| `fw-core-hubs` — framework: core-хабы — сжатие (`SessionIntent::Feature`, конец арма NotAvailable) и одна дверь возможностей хоста (`effects::Ask`); с 2026-11-24 = tm fw-hubs-a/b/c + fix-recap-title, fix-stream-lost (arch-pillars P1-P9) | W2 | 3 + 2 | ≈950 | — (сжатие хабов) | 0 | после fw1-c1..c9, fw1-c2 (Ask::Editor) |
| `fw-ui-registry` — framework: сжатие UI-хабов — реестры действий, целей, ячеек и режимов; session_page.rs по заботам | W2 | 1-2 | ≈1000 | — (сжатие хабов) | 0 | после fw1-c1..c9 (их открытые sheet) |
| `w2-transcript` — UI транскрипт: Ctrl+T pager с поиском, select и copy mode, детали хода, /diff, хром транскрипта | W2 | 4 | ≈3550 | 27 | 194 | fw-ui-registry, fw1-c1, fw1-c4 |
| `w2-composer` — UI composer: состояния, очередь, Ctrl+G, мышь, Vim, burst, mentions и popovers | W2 | 3 | ≈2450 | 21 | 81 | fw-ui-registry, fw1-c2 |
| `w2-settings` — UI настройки: /config, /keymap, /experimental /skills /hooks, лист по заботам | W2 | 2 | ≈2050 | 16 | 9 | fw-ui-registry, fw1-c3 |
| `w2-cells` — UI ячейки: exec, MCP, exploration, /ps, compaction, safety, guardian, ошибки провайдера и остальной cells-sweep | W2 | 3 | ≈3050 | 14 | 95 | fw-ui-registry, fw1-c5, fw1-c6 |
| `w2-approvals` — UI approvals, request-user-input и все select/dialog-листы; bare /review и /permissions | W2 | 3 | ≈2200 | 24 | 121 | fw-ui-registry, fw1-c9 |
| `w2-commands` — UI /side, /subagents, daemon, presets, worktree, Windows sandbox, security setup, /feedback и малые команды | W2 | 3 | ≈2550 | 26 | 30 | fw-ui-registry, fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 |
| `w2-status` — UI /usage, thread usage, лимиты, карточка /status, футер и строка статуса | W2 | 4 | ≈3000 | 20 | 148 | fw-ui-registry, fw1-c7, fw1-c3, fw1-c2 |
| `w2-live` — UI живой ход: backtrack, plan mode, interrupts, баннеры, empty-state animation, Markdown | W2 | 2 | ≈2000 | 3 | 152 | fw-ui-registry, rtwk |
| `voice` — voice/realtime: /voice, F8, voice strip, настройки, split-flap, session lock | W3 | 3 | ≈1900 | 5 | 1 | fw-ui-registry, fw-core-hubs (Ask::Audio) |
| `sidebar` — sidebar: документы, файлы, ссылки, chooser, анимация | W3 | 2 | ≈1300 | 1 | 0 | fw-ui-registry |
| `inline-mode` — inline-режим S27: viewport, вставка в scrollback, reflow, временный alt-screen (rtwk уже умеет inline) | W3 | 2 | ≈1500 | 6 | 12 | fw-core-hubs (Ask::ScreenMode) |
| `ui-tree-viz` — ui_tree и inline visualization | W3 | 2 | ≈1100 | 1 | 0 | fw-core-hubs |
| `pets-ambient` — pets: ambient-питомец, asset pack, протоколы изображений | W3 | 2 | ≈1200 | 4 | 0 | rtwk |
| `startup-rest` — запуск и onboarding: хвосты S01-S06 и startup-шарды, daemon recovery, startup error, cloud environment, провайдеры | W3 | 2 | ≈1300 | 6 | 20 | rtwk для G48-G55; остальное не зависит |
| `agents-center` — agents command center (/agents): остаток и снимки | W3 | 1 | ≈800 | 1 | 0 | fw-ui-registry |
| `resume-rest` — resume/fork picker: хвосты и превью транскрипта | W3 | 1 | ≈600 | 1 | 5 | w2-transcript, kit-seeds |
| `terminal-features` — терминал и рантайм: Ctrl+Z, Ctrl+L, screen reader, reduced motion, уведомления, тема | W3 | 1 | ≈900 | 9 | 0 | fw-ui-registry, fw-core-hubs (Ask::Suspend/Notify/Probe) |
| `app-misc` — остальное приложения: exit summary, session log, turn tips, MCP startup, reconnect, read-only, quit-confirm | W3 | 1-2 | ≈1100 | 4 | 0 | fw-core-hubs, fw-ui-registry |
| `kit-seeds` — обвязка oracle-тестов: семена kit, имена кадров, хранилище настроек | W2 | 1-2 | ≈1400 | 1 | 67 | — |

### `fw1-c1`

- **Что:** core: /diff-документ pager, /presets, /daemon, /subagents, app link, provider switch. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈750 строк.
- **Задача tm:** `fw1-c1` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** get_git_diff.rs, app/quick_dials.rs, chatwidget/quick_dials.rs, app/daemon_menu.rs, app/multi_agents.rs.
- **Ядро / шаги:** вся (P1+P3 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c2`

- **Что:** core: внешний редактор Ctrl+G, очередь команд, состояния composer, /side /btw. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈1150 строк.
- **Задача tm:** `fw1-c2` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** external_editor.rs, chatwidget/input_queue.rs, session_queue_commands.rs, app/side.rs.
- **Ядро / шаги:** вся (P2+P13 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c3`

- **Что:** core: /experimental /skills /hooks, /config /tui /keymap, аргументные формы (settings-слайс). **Волна:** W1 (влита); **размер:** 1-2 прогонов, ≈1500 строк.
- **Задача tm:** `fw1-c3` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** experimental_features.rs, hooks_rpc.rs, config_panel/, keymap_setup/, app/thread_title.rs, status_surface_commands.rs.
- **Ядро / шаги:** вся (P4+P5+P15 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c4`

- **Что:** core: /copy и Ctrl+O, /logout /ide /daybreak /recap /debug-config /debug-m-*. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈950 строк.
- **Задача tm:** `fw1-c4` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** chatwidget/copy_picker.rs, markdown_copy.rs, app/recap.rs, debug_config.rs, daybreak.rs, ide_context.rs.
- **Ядро / шаги:** вся (P6+P14 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c5`

- **Что:** core: exploration-группы, фоновые терминалы и /ps, инвентарь /mcp. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈900 строк.
- **Задача tm:** `fw1-c5` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** history_cell/exec.rs, bottom_pane/unified_exec_footer.rs, history_cell/mcp.rs, app/background_requests.rs.
- **Ядро / шаги:** вся (P9 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c6`

- **Что:** core: safety buffering, misalignment, guardian, compaction. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈800 строк.
- **Задача tm:** `fw1-c6` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** chatwidget/safety_buffering.rs, app/safety_buffering.rs, misalignment_policy.rs, chatwidget/compaction.rs.
- **Ядро / шаги:** вся (P10 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c7`

- **Что:** core: сброс лимита /usage, billing, /status thread usage и строка сервера, предупреждения лимитов. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈900 строк.
- **Задача tm:** `fw1-c7` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** chatwidget/usage.rs, reset_credits.rs, chatwidget/thread_usage.rs, rate_limits.rs, usage_notice.rs.
- **Ядро / шаги:** вся (P11+P12 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c8`

- **Что:** core: worktree/checkout для /new /fork /worktree, «Forked thread», Windows sandbox, security setup. **Волна:** W1 (влита); **размер:** 1 прогонов, ≈1000 строк.
- **Задача tm:** `fw1-c8` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, windows_sandbox_prompts.rs, security_setup.rs.
- **Ядро / шаги:** вся (P16+P18 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw1-c9`

- **Что:** core: /feedback (note, upload), решение approval, вопросы (черновики, очередь). **Волна:** W1 (влита); **размер:** 1 прогонов, ≈700 строк.
- **Задача tm:** `fw1-c9` (влита; ядровая половина, файлы — у пакетов W2).
- **Файлы (владеет):** .
- **Oracle:** bottom_pane/feedback_view.rs, feedback_note_view.rs, app/feedback_upload.rs, history_cell/approvals.rs, request_user_input/.
- **Ядро / шаги:** вся (P17+P19 core).
- **Строки (0):** —.
- **Тесты:** —.

### `fw-core-hubs`

- **Что:** framework: core-хабы — сжатие (`SessionIntent::Feature`, конец арма NotAvailable) и одна дверь возможностей хоста (`effects::Ask`); с 2026-11-24 = tm fw-hubs-a/b/c + fix-recap-title, fix-stream-lost (arch-pillars P1-P9). **Волна:** W2; **размер:** 3 + 2 прогонов, ≈950 строк.
- **Файлы (владеет):** `core/src/app/session.rs`, `core/src/commands/dispatch.rs`, `core/src/commands.rs`, `core/src/threads.rs`, `core/src/threads/cells.rs`, `core/src/app.rs`, `tests/rules/hub_budget.txt`, `core/src/app/effects.rs`, `host/src/effects.rs`, `host/src/in_process.rs`, `host/src/facts.rs`, `src/services/effects.rs`, `tests/rules/layers.rs`, `rest:core/src`, `rest:host/src`, `rest:tests/rules`.
- **Oracle:** app/event_dispatch.rs, chatwidget/slash_dispatch.rs, slash_command.rs, app_event.rs.
- **Ядро / шаги:** ADR-012: пакет фичи НЕ владеет хабом и не пишет в него свои строки; фича входит в приложение только через двери слайса: `feature_list!`/`target_of` (слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможности хоста), `ThreadModel.features` (состояние треда). Правка хаба бывает только СЖАТИЕМ — это и есть вся работа этого пакета. Шаг 1: общий вариант `Feature` в `SessionIntent` (то самое «Need of the ADR-012 owner» в `tests/rules/hub_budget.txt`: «слэш-список не имеет generic-варианта») — после него слэш-команда нового слайса не добавляет варианта; бюджет `SessionIntent 43` фиксируется. Шаг 2 (после слияния fw1-c1..c9): удалить остаток арма `S::Recap | S::Side | … => NotAvailable` (`app/session.rs:361-386`), `Notice::NotAvailable`/`Stub`; команды, что остались без владельца (`/model <имя>`, `/permissions` без аргумента, `/review`, `/resume <имя>`, `/apps`), переехали в слайсы пакетов w2-approvals и resume-rest — арм сжимается до пустого и исчезает, бюджет в `hub_budget.txt` опускается на столько же. Шаг 3: `threads.rs`/`cells.rs` — только вычёркивание вариантов `ThreadEv`/`ThreadAsk`, ушедших в `ThreadModel.features` (счётчики в `hub_budget.txt` падают; рост запрещён правилом). Остальное в `core/src` и `host/src`, чего никто не назвал, — тоже у этого пакета: фичи сюда не пишут. Шаг 4 (дверь возможностей хоста): `AppCx::effect` с `effects::Ask` — единственная дверь для возможностей хоста, у неё один enum на всех; поэтому пакет один раз добавляет заранее названные варианты `Audio` (voice), `Suspend` (Ctrl+Z), `Notify` (OSC 9 / BEL), `ScreenMode` (alt-screen для inline), `Probe` (цвета терминала), `SessionLock` с `Done`-ответами, строками в `hub_budget.txt` и пустыми обработчиками хоста. Причина одного владельца (и единого пакета с шагами 1-3: у них общий `hub_budget.txt`): пакеты voice, terminal-features, inline-mode идут параллельно и иначе правили бы один enum; дальше каждый пишет только свой файл хоста (`host/src/audio.rs` и т. п.) и слайс. Редактор Ctrl+G (`Ask::Editor`) делает fw1-c2..
- **Зависит:** после fw1-c1..c9, fw1-c2 (Ask::Editor).
- **Строки (0):** —.
- **Тесты:** —.

### `fw-ui-registry`

- **Что:** framework: сжатие UI-хабов — реестры действий, целей, ячеек и режимов; session_page.rs по заботам. **Волна:** W2; **размер:** 1-2 прогонов, ≈1000 строк.
- **Файлы (владеет):** `src/pages/session/session_page.rs`, `src/pages/session/keys.rs`, `src/pages/session/open.rs`, `src/pages/session/actions.rs`, `src/pages/session/mod.rs`, `src/pages/session/slice.rs`, `src/pages/session/popup.rs`, `src/pages/session/strings.rs`, `src/pages/session/cells/dispatch.rs`, `src/pages/session/cells/notice.rs`, `src/pages/mod.rs`, `src/sheets/mod.rs`, `src/sheets/presenter.rs`, `src/app/presenters/mod.rs`, `src/app/debug/**`, `src/l10n/**`, `src/keymap/**`, `tests/pages.rs`, `tests/screens.rs`, `tests/oracle.rs`, `tests/rules.rs`, `rest:src`.
- **Oracle:** chatwidget/slash_dispatch.rs, app/event_dispatch.rs, app/input.rs.
- **UI:** Тот же принцип, что у fw-core-hubs: UI-хабы — только сжатие. Где можно, хаб заменяется реестром (макрос по списку, как `feature_list!`/`screen_list!`), где нельзя — остаётся один владелец с причиной. Реестр 1, действия клавиш: `Action` в `keys.rs` и его диспатч собираются списком `action_list!` из `key_actions/<имя>.rs` (`find`, `copy`, `external_editor`, `side`, `voice`, `focus_activity`, `clear_terminal`); файл действия — привязка и `run(cx, state)`. Реестр 2, цели open: `open.rs` держит общий путь `Target::Feature` (он уже есть, `open.rs:75`) и сжимается: `pager`, `daemon`, `agent_picker`, `voice` становятся слайсами и уходят из арма. Реестр 3, ячейки и режимы: `cells/dispatch.rs`, `cells/mod.rs`, `transcript/mod.rs` собирают ячейки (`exploration`, `processes`, `mcp_inventory`, `compaction`, `misalignment`, `decision`, `forked`, `recap`) и режимы (`find`, `select_mode`, `copy_mode`, `diff_view`) списками `cell_list!`/`mode_list!` по файлам. Реестр 4, тесты: `tests/pages.rs`, `screens.rs`, `oracle.rs` получают `mod`-строки из каталога (`build.rs` пишет `mods.rs` по `tests/pages/*.rs`) — новый тестовый файл не правит корень. Без реестра, один владелец: `session_page.rs` (1218 строк, один `Component`; причина — его состояние общее, дробить нечем, кроме выноса забот в `session/*.rs`; пакет выносит и дальше правит только он) и `actions.rs` (сжатие: снять `not_yet`, `presets_action`, `daemon_action`, `action_subagents`). Двери `core/src/features/mod.rs` и `src/features/mod.rs` — одна строка слайса, их не владеют (см. «Двери»)..
- **Первый шаг:** идёт первым в W2: пока реестры не влиты, UI-пакеты ниже стоят (иначе правят один файл).
- **Зависит:** после fw1-c1..c9 (их открытые sheet).
- **Строки (0):** —.
- **Тесты:** —.

### `w2-transcript`

- **Что:** UI транскрипт: Ctrl+T pager с поиском, select и copy mode, детали хода, /diff, хром транскрипта. **Волна:** W2; **размер:** 4 прогонов, ≈3550 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.1; задача tm `w2-transcript`, её `area` — список владения (15 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c1, fw1-c4.
- **Строки (27):** Гиперссылки в тексте (клик открывает); Сворачивание деталей хода (collapse_turn_details), раскрытие; Фокус на activity-группах (F4) и подсказка раскрытия; PageUp/PageDown, колесо мыши, Home/End в транскрипте; Подгрузка старых страниц истории при прокрутке вверх; Закреплённая шапка prompt над транскриптом (pinned header); /copy — копировать ответ; /diff; Copy/export picker (кастомные клавиши, payload); Статический pager (/diff, вывод): строки, страницы, Home/End; Pager: поиск /, n/N; Pager: live tail sync при росте транскрипта; Копирование выделения (Ctrl+C / по отпусканию); Двойной/тройной клик: слово/строка/абзац; Shift-клик расширяет, правая кнопка копирует; Копия как Markdown/plain (payload); Режим выделения с клавиатуры (select mode); Режим копирования (copy mode, Ctrl+X-лидер); Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N); Подсказки поиска и футер поиска; Скорость колеса (mouse_scroll_speed); Старые страницы в overlay (history pages); Клик по ссылке/файлу в overlay; F3 — поиск; F4 — focus activity; Ctrl+O — копировать ответ; Turn tips и tooltips.
- **Тесты:** 194 pending (список — `tools/rtwk-port/map-full/unpend/w2-transcript.txt`).

### `w2-composer`

- **Что:** UI composer: состояния, очередь, Ctrl+G, мышь, Vim, burst, mentions и popovers. **Волна:** W2; **размер:** 3 прогонов, ≈2450 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.2; задача tm `w2-composer`, её `area` — список владения (16 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c2.
- **Строки (21):** Фоновая «starfield»/sparkle эффекты композера; Очередь slash- и shell-команд во время хода; Steer: Enter во время хода отправляет уточнение; Блокировка отправки при недоступной модели; Вставка пути к файлу превращается в элемент/картинку; Скрытая shell-вставка с /plan (hidden shell paste); $ skills / плагины / apps в упоминаниях; Картинки из буфера Ctrl+V / Alt+V; Состояния composer: disabled, shutdown, parent-owned, agents; Vim: нормальный/вставка/операторы/текст-объекты/поиск (движо; Vim: replace/yank/dot-repeat; Внешний редактор Ctrl+G; Мышь в composer (курсор, выделение, вставка правой кнопкой); Startup draft (черновик из аргументов, file-search startup); Sparkle/«искры» над composer; Flash в футере и временный override подсказки; Предупреждения о лимитах в карточке и ленте; Предупреждения (/warnings, F2): страницы, wrap; Tab — в очередь (сообщения); Ctrl+G — внешний редактор; Ctrl+V / Alt+V — вставка картинки.
- **Тесты:** 81 pending (список — `tools/rtwk-port/map-full/unpend/w2-composer.txt`).

### `w2-settings`

- **Что:** UI настройки: /config, /keymap, /experimental /skills /hooks, лист по заботам. **Волна:** W2; **размер:** 2 прогонов, ≈2050 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.3; задача tm `w2-settings`, её `area` — список владения (4 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c3.
- **Строки (16):** Команды с inline-аргументами (/plan x, /goal x, /review x, /; /keymap — редактор клавиш; /experimental — feature-флаги; /memories; /skills; /hooks; /config; /tui — быстрые настройки TUI; Hooks popup: список, диагностика; Experimental features popup (toggle сохраняется на выходе); Memories settings и reset; Manage skills summary; Current settings (model/permissions/status layout); Settings: Theme, Memories (из чата); Settings: Config / Statusline / Title / Experimental / Skill; Ctrl+X — leader (Ctrl+X b — смена бэкенда).
- **Тесты:** 9 pending (список — `tools/rtwk-port/map-full/unpend/w2-settings.txt`).

### `w2-cells`

- **Что:** UI ячейки: exec, MCP, exploration, /ps, compaction, safety, guardian, ошибки провайдера и остальной cells-sweep. **Волна:** W2; **размер:** 3 прогонов, ≈3050 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.4; задача tm `w2-cells`, её `area` — список владения (68 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c5, fw1-c6.
- **Строки (14):** Ячейка exec: команда, вывод, статус, длительность; Группы exploration (Read/List/Search свёрнуты в одну); Unified exec: фоновые терминалы, wait-streak, interaction-яч; Инвентарь MCP (вывод /mcp); Ячейка просмотра/генерации изображений; Ячейка collab/sub-agent (spawn, wait, send, модель/effort); Compaction: статус, таймер, ячейка завершения; Final «Worked for …» разделитель с длительностью; Safety buffering: «повторить», подтверждение, короткое сообщ; Misalignment/guardian stop треда; /compact; /mcp; /ps — фоновые терминалы; Guardian review (approved/denied/timed out, aggregate status.
- **Тесты:** 95 pending (список — `tools/rtwk-port/map-full/unpend/w2-cells.txt`).

### `w2-approvals`

- **Что:** UI approvals, request-user-input и все select/dialog-листы; bare /review и /permissions. **Волна:** W2; **размер:** 3 прогонов, ≈2200 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.5; задача tm `w2-approvals`, её `area` — список владения (66 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c9.
- **Строки (24):** Решение approval в транскрипте (Approved/Denied + команда); /model — выбор модели и effort; /permissions — approvals/профили; /goal [цель/pause/resume/clear]; /plugins; Model picker: каталог, discovery, bedrock/custom display, se; Permissions popup: пресеты, профили, retry discovery; Goal: меню и edit prompt, resume-paused prompt; Collaboration / mode catalog; Plugins: список, вкладки, поиск, деталь, install, marketplac; Apps/connectors: список, статус, install-only; Sweep: прочие мелкие popup'ы; Approval apply_patch: модал, call_id, заголовок; Approval network host (разово/сессия); Approval write_stdin; Вопросы: черновики, навигация, replay черновиков; Вопросы: поиск по истории, Vim в поле ответа; Асинхронные вопросы (desktop answer, уведомления); Отклонение пустых ответов, явный skip; Разворот вопроса уступает мышь терминалу для ссылок; Очередь вопросов и queued messages делят один shortcut; Шорткаты: оверлей «?» со всеми действиями; Верификация пользователя (user verification); Список выбора (SelectionList): тап по строке.
- **Тесты:** 121 pending (список — `tools/rtwk-port/map-full/unpend/w2-approvals.txt`).

### `w2-commands`

- **Что:** UI /side, /subagents, daemon, presets, worktree, Windows sandbox, security setup, /feedback и малые команды. **Волна:** W2; **размер:** 3 прогонов, ≈2550 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.6; задача tm `w2-commands`, её `area` — список владения (35 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9.
- **Строки (26):** Строка «Forked thread» / имя треда в истории; Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись ро; /presets — выбор пресета (Daybreak presets); /daybreak; /ide — контекст IDE; /setup-default-sandbox (Windows); /new; /fork; /worktree; /side, /btw; /daemon; /debug-config; /logout; /feedback; /subagents (multi-agents picker); /debug-m-drop, /debug-m-update; Windows sandbox: prompts, help-ссылки, fallback; Worktree/checkout picker; Quick dials (saved dials: save/clear picker); Feedback: выбор, consent upload, диагностика, note; Security setup / security prompts; Daemon (подключение к daemon); Ссылка на приложение (app link) диалог; Feedback (диалог); Model provider switch (блокирует кэш выбора); Ctrl+/ (Ctrl+_, Ctrl+7) — side.
- **Тесты:** 30 pending (список — `tools/rtwk-port/map-full/unpend/w2-commands.txt`).

### `w2-status`

- **Что:** UI /usage, thread usage, лимиты, карточка /status, футер и строка статуса. **Волна:** W2; **размер:** 4 прогонов, ≈3000 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.7; задача tm `w2-status`, её `area` — список владения (31 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, fw1-c7, fw1-c3, fw1-c2.
- **Строки (20):** Часы/время в уведомлениях (clock facts); Rate-limit подсказки/предупреждения во время хода; /status — карточка статуса; /usage; /title — терминальный title; /statusline; Reset-limit picker и подтверждение (usage menu); Настройка статус-строки (/statusline, порядок элементов); Терминальный заголовок (title) с элементами; Карточка /status: usage треда; Карточка /status: строка сервера; Подпись контекста side-разговора; Подсказка reset-лимита на старте; Страница аналитики: бары, вкладки, часы плана, narrow-график; Меню /usage: Analytics, Refresh, Reset; Reset лимита: пикер, подтверждение, idempotency-ключ, in-fli; Billing и enterprise-планы; Подсказка reset на старте; Статус после billing retries; Ранний redraw и повтор thread usage.
- **Тесты:** 148 pending (список — `tools/rtwk-port/map-full/unpend/w2-status.txt`).

### `w2-live`

- **Что:** UI живой ход: backtrack, plan mode, interrupts, баннеры, empty-state animation, Markdown. **Волна:** W2; **размер:** 2 прогонов, ≈2000 строк.
- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` §A.8; задача tm `w2-live`, её `area` — список владения (10 путей, `tools/rtwk-port/map-full/packages.py`).
- **Зависит:** fw-ui-registry, rtwk.
- **Строки (3):** Ответ агента: стриминг markdown, списки, таблицы, код, ссылк; Резерв Luna: селектор и статус; Empty-state animation.
- **Тесты:** 152 pending (список — `tools/rtwk-port/map-full/unpend/w2-live.txt`).

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

- **Что:** inline-режим S27: viewport, вставка в scrollback, reflow, временный alt-screen (rtwk уже умеет inline). **Волна:** W3; **размер:** 2 прогонов, ≈1500 строк.
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
- **Тесты:** 20 pending (список — `tools/rtwk-port/map-full/unpend/startup-rest.txt`).

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
- **Первый шаг:** rtwk: шаблон имён кадров в kit (см. kit-seeds); pager из w2-transcript.
- **Зависит:** w2-transcript, kit-seeds.
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
- **Тесты:** 67 pending (список — `tools/rtwk-port/map-full/unpend/kit-seeds.txt`).

## Строки

### Транскрипт и ячейки

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Сообщение пользователя (блок с фоном, пути, картинки-плейсхолдеры) | ✅ | Фон блока prompt по теме (?) | history_cell/messages.rs | src/pages/session/cells/user.rs | — |
| Ответ агента: стриминг markdown, списки, таблицы, код, ссылки | 🟡 | landed c4c2af4e0: настройки `[tui.rendering]` (таблицы, mermaid, списки) читаются там, где строится сообщение; при выключении источник сохраняется (5 ключевых тестов markdown_streaming_2/5 зелёные). Нет: math off только флаг (G-math-off, G61), маркер списка `•` вместо `- ` (G-list-marker), `user.rs`/reasoning/resume рисуют markdown без настройки, lists_preserve_layout (G73) и MATH-тесты (G61/G62) — sweep; часть markdown-рендера ждёт (счёт не пересчитан) | markdown_render.rs, streaming/ | src/widgets/markdown.rs, src/pages/session/cells/markdown.rs | `w2-live` |
| Подсветка кода и diff в ответах | ✅ | — | markdown_render.rs, render/highlight.rs | src/widgets/markdown.rs | — |
| Гиперссылки в тексте (клик открывает) | 🟡 | обычный клик по ссылке красный (ordinary_click_opens_links), OSC-8 в активной ячейке ждёт | terminal_hyperlinks.rs, transcript_view/selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | landed a15c35bf8: сводка live и в истории, raw reasoning по show_raw_agent_reasoning, пустые блоки (7 тестов зелёные) | history_cell/messages.rs | src/pages/session/cells/reasoning.rs | — |
| Ячейка exec: команда, вывод, статус, длительность | 🟡 | компактная форма как у oracle (afd840c6e; non-zero exit — bold-буллет); нет: пульс буллета running (G-tool-bullet-pulse), форма `!cmd` с 50 строками (G-exec-user-shell), scrollback-снимки ждут inline-режима S27 | history_cell/exec.rs, exec_cell/ | src/pages/session/cells/exec.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Группы exploration (Read/List/Search свёрнуты в одну) | ❌ | группировка, прерывание, нулевой exit, порядок с reasoning (≈20 тестов) | history_cell/exec.rs, exec_cell/model.rs | src/pages/session/cells/exploration.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Unified exec: фоновые терминалы, wait-streak, interaction-ячейка | ❌ | ячейка взаимодействия, «waited for», сохранение при interrupt | history_cell/exec.rs, bottom_pane/unified_exec_footer.rs | src/pages/session/cells/interaction.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | пустой/failed-patch формат, ручное approval-заголовок (тесты ждут) | history_cell/patches.rs, diff_render.rs | src/pages/session/cells/patch.rs | — |
| Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | в чате совпадает с oracle (pty, afd840c6e); снимки cells_4/5 — scrollback-форма, ждут inline-режима S27; пульс буллета ❌ | history_cell/mcp.rs | src/pages/session/cells/tool_call.rs | `inline-mode` |
| Инвентарь MCP (вывод /mcp) | ❌ | ячейка и данные инвентаря | history_cell/mcp.rs | src/pages/session/cells/mcp_inventory.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Ячейка web search (запрос, детали, усечение) | ✅ | строка в чате и обёрнутая форма в Ctrl+T (afd840c6e) | history_cell/dynamic.rs | src/pages/session/cells/web.rs | — |
| Ячейка просмотра/генерации изображений | 🟡 | image_generation_call_adds_history_cell и просмотр: рендер есть (cells/viewed_image, generated_image), снимки ждут | history_cell/base.rs, transcript_image.rs | src/pages/session/cells/viewed_image.rs, src/pages/session/cells/generated_image.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Ячейка плана (update_plan) и предложенный план | ✅ | — | history_cell/plans.rs | src/pages/session/cells/plan.rs, src/pages/session/cells/plan_update.rs | — |
| Ячейка collab/sub-agent (spawn, wait, send, модель/effort) | 🟡 | spawn_end с моделью, wait-элементы, повторный рендер sub-agent | history_cell/dynamic.rs, multi_agents.rs | src/pages/session/cells/collab.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Ячейка delegated (делегированный вывод tool) | ✅ | — | history_cell/delegated_message.rs | src/pages/session/cells/delegated.rs | — |
| Ячейка hook (запущен/завершён) | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Решение approval в транскрипте (Approved/Denied + команда) | ❌ | decision cell: усечение многострочных команд, network-allow, canceled host | history_cell/approvals.rs | src/pages/session/cells/decision.rs (нет) | ядро: fw1-c9 (влито); `w2-approvals` |
| Результат вопроса request_user_input в транскрипте | ✅ | landed a15c35bf8: ячейка ответа/прерывания (completed_and_interrupted_results); markdown из истории — sweep | history_cell/request_user_input.rs | src/pages/session/cells/request_input.rs | — |
| Ячейка ошибки (upstream, cyber policy, overloaded, safety access) | ✅ | chained config error wrap (pending) | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Предупреждения (warning, sqlite, config, guardian, model verification) | ✅ | wrap уведомления в narrow, notice_details_fit | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Deprecation / update-available / version-notice | ✅ | update-available и server-version от host (e263606f0); server-version дедуп в пределах запуска (oracle хранит last_shown) | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Часы/время в уведомлениях (clock facts) | 🟡 | Clock и LinkDisplay от host (host/src/viewer.rs, e263606f0); RuntimeMetrics без источника (G-runtime-metrics → host-счётчики, core-live-debts) | clock_format.rs | core/src/format.rs | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Session info: шапка сессии (модель, cwd, подсказки, tooltips) | ✅ | landed 87ad7733c: карточка шапки, каталог подсказок и выбор tip-строки, model-changed notes, стартовые факты от host | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Строка «Forked thread» / имя треда в истории | ❌ | fork-history line с именем/id, prompt-edit history line | history_cell/session.rs | src/pages/session/cells/forked.rs (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Review-ячейки (баннер Current changes, request hint, prompt скрыт) | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Compaction: статус, таймер, ячейка завершения | ❌ | timer, snapshot restore, clear on turn end | chatwidget/compaction.rs | src/pages/session/cells/compaction.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Final «Worked for …» разделитель с длительностью | 🟡 | cumulative duration snapshot ждёт; разделитель есть (cells/separator) | history_cell/separators.rs | src/pages/session/cells/separator.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Переключатель плотности (compact/detailed) ячеек | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Сворачивание деталей хода (collapse_turn_details), раскрытие групп | 🟡 | сворачивание, сводки групп, раскрытие кликом/Enter/Space (16ed6526f); нет: ширинно-зависимый счёт скрытых строк exec, хранение 50+50 строк вывода, TurnTip, закреплённая шапка prompt (G-new-usage-2) | transcript_view/turn_details.rs, disclosure.rs | src/pages/session/transcript/turn_details.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Фокус на activity-группах (F4) и подсказка раскрытия | ❌ | focus_activity целиком | transcript_view/activity.rs, history_cell/activity_group.rs | src/pages/session/transcript_activity.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Автопрокрутка за хвостом, «вернуться вниз», строка новой активности | ✅ | — | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | Home/End в ленте; скорость колеса (mouse_scroll_speed) не применяется | transcript_view/input.rs, tui/event_stream.rs | src/pages/session/transcript/follow.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Подгрузка старых страниц истории при прокрутке вверх | 🟡 | landed ee4c83bd9: статус-строка (loading / refused с retry / loadable, 4 ширины) из `transcript/history.rs`, join страницы не двигает читателя, 9 page-тестов, 4 oracle-теста зелёные; pty: тот же финальный экран. Нет: клавиши ctrl+home / alt+< (идти к началу, retry) и ctrl+end / alt+> (Need: session_page.rs); склейка групп oracle при join (page_join_* в transcript_view_3, sweep); join при читателе в 1-3 строках ниже первой сдвигает вид на строку; один тик колеса двигает много больше строк, чем у oracle | app/history_pagination.rs, transcript_view/mutations.rs | src/pages/session/transcript/history.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Закреплённая шапка prompt над транскриптом (pinned header) | ❌ | нет модуля prompt_header | transcript_view/prompt_header.rs | src/pages/session/transcript_header.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Replay истории при resume/fork: сообщения, tool, reasoning, review | ✅ | resumed_initial_messages_render_history, nested review prompts — ждут | history_cell/, transcript_view/, markdown_render.rs | src/pages/session/cells/, src/pages/session/transcript/ | — |
| Ambient pet в транскрипте/футере | 🟡 | pets sheet есть; ambient-тесты chat_status_1 не включены | pets/ambient.rs | src/sheets/pets/ | `pets-ambient` |
| Фоновая «starfield»/sparkle эффекты композера | 🟡 | задача занята; 12 тестов не зелёные | bottom_pane/chat_composer/sparkle.rs, sparkle_field.rs | src/widgets/composer_sparkle.rs | ядро: fw1-c2 (влито); `w2-composer` |

### Живой ход (стриминг, прерывание, очередь, steer)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Отправка сообщения Enter, индикатор working/«Thinking», таймер хода | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Прерывание хода Esc | ✅ | Esc с ожидающими steer: отправка всех сразу и сохранение (тест ждёт) | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Ctrl+C: прерывает ход, второй раз выходит; «armed quit» подсказка | ✅ | Ctrl+C при caps lock (n/a) | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+D на пустом composer выходит (когда idle) | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Очередь сообщений (Tab / Enter в ходе) над composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Очередь slash- и shell-команд во время хода | ❌ | ядро готово (`core/src/threads/queue.rs`: `Queued` Message/Literal/Command/Shell, `DueCommand`, `QueueRan`/`QueueResume`); нет вида: Tab ставит строку команды, запуск при drain, !shell в очереди, отклонение при MCP startup | chatwidget/input_queue.rs, session_queue_commands.rs | core/src/threads/queue.rs | ядро: fw1-c2 (влито); `w2-composer` |
| Правка очереди (Alt+Up, вернуть в composer) | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Steer: Enter во время хода отправляет уточнение | 🟡 | pending steers: восстановление при interrupt ок; ordering/mention-bindings/failed steer не включены | chatwidget/input_submission.rs | src/pages/session/composer/ | ядро: fw1-c2 (влито); `w2-composer` |
| Предпросмотр pending steers над composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Отправка во время review/plan stream (очередь вместо steer) | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Блокировка отправки при недоступной модели | ❌ | submit_user_message_blocks_when_thread_model_is_unavailable | chatwidget/input_submission.rs, session_model_selection.rs | src/pages/session/composer/ | ядро: fw1-c2 (влито); `w2-composer` |
| Подтверждение Esc/Enter при диалогах поверх composer | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Баннер бэкенда (нумерованные строки, CTA, отклонение по occurrence) | ✅ | landed c4fff1a2b: авто-переключение fallback и `{time}`, F6 (обрыв соединения отменяет запросы в полёте), 2 ключевых теста chat_live_2 зелёные; в pty баннер не гонялся (в mock нет endpoint). Нет (sweep): три теста восстановления (нужен on_rate_limit_error из chunk-status) и снимок backend_banner_notification; has_pending_recovery не портирован, fallback только при чтении аккаунта, notice_model не сбрасывается при смене треда | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Резерв Luna: селектор и статус | 🟡 | селектор работает; модель возврата резерва живёт только в памяти (нужен Ask::ConfigWrite); backend_auth заменяет has_chatgpt_account; патч переключения = модель + effort; статус по активной модели, переживание закрытия баннера — ждут | chatwidget/luna_reserve_*.rs, app/backend_banner_fallback.rs | src/pages/session/banners.rs | `w2-live` |
| Safety buffering: «повторить», подтверждение, короткое сообщение | ❌ | весь блок safety_buffering (9 тестов) | chatwidget/safety_buffering.rs, safety_recourse.rs | src/pages/session/cells/safety.rs | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Экран ошибки провайдера (сохраняет черновик, выход в Resume) | ✅ | сохранение между потоками, provider_id sync — ждут | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Read-only внешний writer: заметка вместо composer, переход в Agents | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Уведомления терминала о завершении хода / approval | 🟡 | настройки notify_when; async-question уведомления n/a | notifications/, chatwidget/notifications.rs | src/app/terminal_effects.rs | `terminal-features` |
| Stream recovery: восстановление статуса после обрыва | ✅ | — | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| MCP startup: статус, lag, отмена, отказ как warning | ✅ | mcp_startup_failure_renders_warning_history, booting snapshot — ждут | chatwidget/turn_lifecycle.rs, interrupts.rs, protocol.rs | src/pages/session/live/ | — |
| Rate-limit подсказки/предупреждения во время хода | ❌ | limit warnings (90%/95%), prompt по лимиту | chatwidget/rate_limits.rs, status/rate_limits.rs | src/pages/session/live/limits.rs | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Misalignment/guardian stop треда | ❌ | misalignment_* (3), guardian parallel reviews, aggregate status | chatwidget/misalignment_policy.rs, app/misalignment_policy.rs | src/pages/session/cells/misalignment.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись родителя | ❌ | нет вообще: Ctrl+/ даёт `Notice::NotAvailable(Side)` (session_page.rs:683), /side и /btw — NotAvailable (core/src/app/session.rs); тесты отказов в side-треде тоже ждут | app/side*.rs, chatwidget/side.rs | src/features/side/ (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
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
| Команды с inline-аргументами (/plan x, /goal x, /review x, /rename x) | 🟡 | /model x, /permissions-profile без args; submission-цепочка не вся; после батчей 10-11: `/permissions x` ✅ (`PermissionsArgs`); `/model x`, `/title x`, `/statusline x`, `/resume x`, `/review` без args — NotAvailable или не все (см. раздел «Заглушки») | chatwidget/slash_command_args.rs, bottom_pane/prompt_args.rs | core/src/commands/dispatch.rs | ядро: fw1-c3 (влито); `w2-settings` |
| Вставка: paste burst, placeholder большой вставки [Pasted Content N] | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Вставка пути к файлу превращается в элемент/картинку | 🟡 | путь→картинка есть, hidden shell paste очередь/восстановление не включены | bottom_pane/chat_composer/paste_input.rs, attachment_files.rs | src/pages/session/composer/ | ядро: fw1-c2 (влито); `w2-composer` |
| Скрытая shell-вставка с /plan (hidden shell paste) | 🟡 | работает, тест не включён (4 из 5 stale), 1 красный: submits_literal | chatwidget/input_flow.rs | src/pages/session/composer/ | ядро: fw1-c2 (влито); `w2-composer` |
| История Up/Down в сессии | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| История между сессиями (persistent history) | ✅ | landed 2054483f3: Up после перезапуска вспоминает последнее сообщение (pty: rtwk = oracle), slash-команды записываются как у oracle (`/di` из popup → `/diff`), история засевается из replay resume (3 oracle-теста зелёные, 2 page-теста). Нет (sweep, ждут картинок и Vim): восстановление картинок и vim-навигация по истории (4 теста composer_3); история текстовая; `/diff` до pager закрывается Esc | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Поиск по истории Ctrl+R / Ctrl+S | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Восстановление очищенного Ctrl+C prompt из истории | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| @-упоминания: popup, вставка, Tab, множественные | ✅ | landed 2054483f3: живой file search (через in-process поиск хоста, `Ev::FileMatches`; app-server fuzzyFileSearch не портирован, не нужен) и server skills/plugins в списке, строки как у oracle (pty: `@sr` совпадает построчно); n/a при MentionsV2 off (решение 2026-11-23): 4 теста composer_1. Нет (sweep): adjacent_image_completion_inserts_leading_separator (ждёт картинок и Vim) | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | — |
| Семена mention (при Esc/восстановление steer возвращают биндинги) | ✅ | биндинги доходят до отправки и восстанавливаются (mention-seeds, влит) | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| $ skills / плагины / apps в упоминаниях | 🟡 | landed 2054483f3: каталог skills (`skills/list`) и plugins (`plugin/installed`) грузится раз на тред, когда известен cwd; установленные apps берутся из кэша /apps (pty: `$sk` совпадает построчно). Нет: кэш apps загружается только при открытии /apps, поэтому `$`-приложения появляются после /apps, oracle грузит коннекторы при старте (Need владельцу select/apps); предупреждения о ошибках загрузки skill (invalid SKILL.md) не портированы; legacy MentionsV2-off — n/a (решение 2026-11-23) | bottom_pane/mentions_v2/, skill_popup.rs | src/pages/session/popovers/ | ядро: fw1-c2 (влито); `w2-composer` |
| Картинки: локальные [Image #N], нумерация, удаление | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Картинки из буфера Ctrl+V / Alt+V | 🟡 | поток есть: Ctrl+V/Alt+V → `Intent::PasteImage` → `Ask::ClipboardImage`, ответ в `src/services/effects.rs` (`paste_image`) → `Ev::ImagePasted`; в тестах ответ хоста подаёт kit (`clipboard_image`, батч 5: снята причина pending у теста с картинкой из буфера после удержанного `!`, chat_commands_1). Нет: прогона с настоящим буфером в бинаре; `adjacent_image_completion_inserts_leading_separator` (composer_3) ждёт (sweep) | clipboard_paste/, chatwidget/image_submission.rs | src/services/clipboard/ | ядро: fw1-c2 (влито); `w2-composer` |
| Remote-картинки (URL): submit и нумерация | ✅ | landed cc0bbabd5 (батч 5): `UserMessage.remote_images` идут в turn_input перед локальными, через merge_messages, `Draft::message` и put_back; Enter и Tab на одних remote-картинках отправляют; вызванная из истории запись снова рисует remote-строки; выделение, режущее элемент, расширяется до него; kit-семена `Open::remote_images` и `clipboard_image`. Тесты с этими семенами включены в chat_commands_1, composer_2, composer_3, composer_4. Нет (sweep): тела composer_4 без семян Vim/remote-картинок (удаление выбранной remote-строки как одна правка Vim, часть снимков) | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Состояния composer: disabled, shutdown, parent-owned, agents-nav hint | ❌ | нет понятия состояний; read-only есть частично | bottom_pane/chat_composer.rs, warning_notice.rs | src/pages/session/composer/states.rs (нет) | ядро: fw1-c2 (влито); `w2-composer` |
| Bang-shell режим: !команда, Enter в ходе → run_user_shell | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Vim: нормальный/вставка/операторы/текст-объекты/поиск (движок) | 🟡 | landed fdadb68ff (батч 5): `/vim` включает и выключает с заметкой (`NoteKey::Toggled`), F9 (`global.toggle_vim_mode`) тоже, `tui.vim_mode_default` начинает черновик нового треда в Insert (TuiSettings, config host), подпись футера следует режиму (страница перестраивается, когда клавиша меняет режим), `!` в Normal входит в Insert; vim-тесты composer_1, composer_3, chat_live_4, chat_commands_1 включены. Нет (sweep): composer_3 `left_respects_editor_and_vim_history_remaps` (нет keymap-семени), тела composer_4 без Vim-семян (поиск `/`, Ctrl+R на пустой истории, redo, Replace); Vim в поле ответа на вопрос (см. «Вопросы») | bottom_pane/textarea/vim*.rs, chat_composer/vim_*.rs | src/widgets/composer_draft/ | ядро: fw1-c2 (влито); `w2-composer` |
| Vim: replace/yank/dot-repeat | 🟡 | dot-повтор теперь записывает удержанный или сброшенный paste-burst символ (fdadb68ff); тела composer_4 ждут (Vim Replace, kit не включает Vim в этих телах) | bottom_pane/textarea/vim*.rs, chat_composer/vim_*.rs | src/widgets/composer_draft/ | ядро: fw1-c2 (влито); `w2-composer` |
| Внешний редактор Ctrl+G | ❌ | нет ни обработчика, ни запуска $EDITOR | external_editor.rs | src/features/external_editor/ (нет) | ядро: fw1-c2 (влито); `w2-composer` |
| Мышь в composer (курсор, выделение, вставка правой кнопкой) | ❌ | right_click_paste и мышь не выполняются | bottom_pane/chat_composer/mouse.rs, textarea/mouse.rs | src/pages/session/composer/mouse.rs (нет) | ядро: fw1-c2 (влито); `w2-composer` |
| Startup draft (черновик из аргументов, file-search startup) | 🟡 | работает, тест не включён | startup_draft*.rs | src/pages/startup/ | ядро: fw1-c2 (влито); `w2-composer` |
| Подсказки «? shortcuts» / Esc-Esc / «Ctrl+C to quit» в футере composer | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Элементы composer: [Image], [Pasted], mention — атомарное удаление | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Расширение (collapse) длинной вставки при submit | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Отправка с пустым / только пробелы / только плейсхолдер | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |
| Тема-цвет курсора и подсказки; narrow-терминал | ✅ | — | bottom_pane/chat_composer.rs, textarea.rs | src/pages/session/composer/ | — |

### Слэш-команды (каждая — своя строка; порядок как в slash_command.rs)

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| /model — выбор модели и effort | 🟡 | bare ✅ (OpenModels), landed 87ad7733c (chunk-popups-sweep): 13 тестов chat_popups_4 на живом пикере, курсор по форме страницы, скрытые модели; /model <имя> NotAvailable (provider-write не портирован); advanced effort, refresh каталога не включены; батчи 10-11: лист models на вложенном Navigator (advanced-страница, скрытые модели); `/model <имя>` — `Intent::Model{..}` → NotAvailable (app/session.rs) | chatwidget/model_command.rs, model_popups.rs | core/src/screens/select/model.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| /presets — выбор пресета (Daybreak presets) | ❌ | лист quick-dials (`Opening::Quick`, `select/quick_dials.rs`) и `select::presets` есть, debug-вход есть; страница `Intent::Quick` не шлёт: `SlashCommand::Presets` показывает попап «not built» (actions.rs, b11-fixes) | chatwidget/quick_dials.rs, app/quick_dials.rs | src/sheets/select/quick.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /daybreak | ❌ | NotAvailable; отказ Daybreak | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /ide — контекст IDE | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /permissions — approvals/профили | 🟡 | bare ✅ (OpenPermissions), args через PermissionsArgs ✅; подтверждение full-access, Windows, history cell не все | chatwidget/permissions_menu.rs, permission_popups.rs | src/sheets/select/permissions.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| /keymap — редактор клавиш | ❌ | `Which::Keymap` → `return` (core/src/features/settings/mod.rs), пустой слот «settings (S15)» без вида; ленты keymap-настроек нет | keymap_setup/ | src/features/settings/ (нет keymap_editor) | ядро: fw1-c3 (влито); `w2-settings` |
| /vim — переключить Vim composer | ✅ | fdadb68ff (батч 5): включает и выключает черновик, пишет заметку; сброс режима в Insert после submit (composer_1 `vim_mode_resets_to_insert_after_submission` включён); pty-прогона в этом пересмотре нет | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /setup-default-sandbox (Windows) | ❌ | ElevateSandbox NotAvailable; только Windows | chatwidget/windows_sandbox_prompts.rs, windows_sandbox.rs | src/sheets/select/permissions.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /experimental — feature-флаги | ❌ | `Which::Experimental` → `return`; sheet `src/features/settings/features.rs` есть, нет открывателя и ленты experimentalFeature/list | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| /approve — Auto-review | ✅ | открывает sheet denials (`sheets::approvals::open_auto_review`), без denials — заметка; снимки ждут | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /memories | 🟡 | `host/src/settings.rs` подключён (`settings_fact` в in_process.rs, батч 10): /memories открывается, когда host выдал факты memory tool (`facts.memories`), иначе молча ничего; reset-confirmation ✅ как UI-состояние. Pty при обновлении карты не перегонялся | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| /skills | ❌ | `Which::Skills` → `return`; sheet `src/features/settings/setup.rs` есть, нет открывателя и ленты skills/list; manage_skills_summary pending | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| /import — перенос из других агентов | ✅ | route Import; shard import 16/16 | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /hooks | ❌ | NotAvailable (`S::Hooks` в core/src/app/session.rs); sheet `src/features/settings/hooks.rs` есть, нет ленты hooks/list; hooks_popup_shows_list_diagnostics | experimental_features.rs, hooks_rpc.rs, skills_helpers.rs, chatwidget/settings_popups.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| /review — выбор ветки/коммита/custom | ✅ | branch/commit picker snapshots ждут (review_*_picker, 2 pending) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /rename — имя треда | ✅ | landed a15c35bf8 (95826e191): с аргументом и bare prompt (slice features/rename); батч 9 (features-trait): prompt — UI-половина `src/features/rename/prompt.rs` на общем виде `features/prompt.rs` (Menu/Pick/Prompt у Feature убраны), pty rename-prompt в land-check батча; 29abc1aa5: вид prompt `src/sheets/dialogs/prompt.rs`, чат показывает любой открытый моделью prompt (так же файловый prompt /export и edit /goal), 5 тестов chat_commands_2 зелёные, pty rename-prompt проходит в обоих бинарях. Нет: предложение имени — хост отвечает Done::Title(None), oracle генерирует через ephemeral thread (thread_title.rs); нужен backend SuggestTitle в services/rpc | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /new | 🟡 | новый тред ✅; выбор checkout (`Intent::OpenCheckout`) — модель в core есть, приложение его не шлёт (select.rs:706) | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /archive | ✅ | confirm sheet + RPC | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /delete | ✅ | confirm sheet + RPC | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /resume | 🟡 | route Resume ✅ (resume_1/2: 48 из 72 зелёные); `/resume <имя>` (`ResumeNamed`) — NotAvailable | resume_picker.rs, session_resume.rs, named_session_lookup.rs | src/pages/resume/ | `resume-rest` |
| /fork | 🟡 | новый тред ✅; ошибка fork привязана к исходному треду (b11-fixes); пикер checkout (`OpenCheckout` не шлётся) и строка «Forked thread» ❌ | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /worktree | 🟡 | sheet открывается (`S::Worktree` → `OpenWorktrees`); список не грузится: у `Ask::Worktrees` и `Ask::WorktreeRemove` нет исполнителя (services/effects.rs → None, host/src/effects.rs не обслуживает); Continue/Start-цепочка ждёт `OpenCheckout` и старт | chatwidget/worktree_picker.rs, app/managed_worktree_creation.rs, worktree_browser.rs | src/sheets/select/worktree.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /app — открыть в desktop-приложении | ✅ | OpenUrl codex://threads/<id> | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /init | ✅ | отправляет prompt AGENTS.md | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /compact | 🟡 | команда ✅; статус/таймер compaction не включены | chatwidget/compaction.rs | src/pages/session/cells/compaction.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| /config | ❌ | `Which::Config` → пустой слот «settings (S15)» (`Shown::Other`), вида нет; то же для `/tui` | config_panel/, chatwidget/tui_mode_picker.rs | src/features/settings/ (нет config_panel) | ядро: fw1-c3 (влито); `w2-settings` |
| /recap | ✅ | fix-recap-title 4131311a7: AppCx::structured_turn (ephemeral read-only thread, outputSchema), recap cell / failure line; остаток: отказ для облачной беседы -> w2-commands | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | — |
| Пробел: /recap и предложение имени треда — заглушки | ✅ | fix-recap-title 4131311a7: заглушки Ask::StructuredTurn/SuggestTitle удалены; /recap и предложение имени идут через AppCx::structured_turn; остаток (авто-имя по первому сообщению, отмена устаревшего запуска) -> w2-commands | temporary_structured_request.rs, app/recap.rs, app/thread_title.rs | core/src/features/recap, core/src/features/rename, src/services/effects.rs | — |
| Пробел: потеря/восстановление потока треда | ✅ | fix-stream-lost 78e957dcf: host/src/cloud.rs шлёт BackendEv::StreamLost{thread,error}/StreamRestored; простой тред — Command::ReconnectIdleThread, в ходе — строка Error, очередь держится; восстановление: history_may_be_incomplete + отправка очереди; открытие потерянного треда перечитывает его. Остаток: restore_reconnected_input (steer до эха) -> live-turn; AppEv::ShuttingDown без отправителя (нет on_stop в rtwk, потребителя нет) | app/app_server_events.rs:130-170, chatwidget/input_flow.rs | host/src/in_process.rs, host/src/cloud.rs, core/src/threads | — |
| /plan [prompt] | ✅ | маска из каталога, уведомления disabled/unavailable | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /voice | ❌ | bare /voice, mute, stop → NotAvailable (`VoiceControl`); F8 и fallback-ветка open.rs → пустой sheet voice | chatwidget/realtime*.rs, bottom_pane/voice_strip.rs | src/sheets/voice/ | `voice` |
| /goal [цель/pause/resume/clear] | 🟡 | команда и модель (features/goal) ✅; батч 9 (features-trait): у цели своя UI-половина `src/features/goal/` (sheet.rs: меню цели и edit prompt, общий вид prompt `features/prompt.rs`), `Target::Feature`; снимки меню и edit prompt не включены | chatwidget/goal_menu.rs, goal_status.rs, goal_display.rs | src/features/goal/ | ядро: fw1-c9 (влито); `w2-approvals` |
| /agents — центр агентов | ✅ | Target::Agents route | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /side, /btw | ❌ | нет; тесты отказов в side-треде тоже ждут | app/side*.rs | src/features/side/ (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /copy — копировать ответ | ❌ | NotAvailable(Copy); copy picker и Ctrl+O не готовы; в чате есть только копия выделения мышью | chatwidget/copy_picker.rs, markdown_copy.rs | src/features/copy/ (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| /export | ✅ | landed a15c35bf8 (95826e191): путь, picker назначения, prompt имени (slice features/export); батч 9: UI-половина `src/features/export/` (sheet.rs), `Target::Feature` | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /raw | ✅ | режим + заметка, Alt+R | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /tui — быстрые настройки TUI | ❌ | `Which::Config` → пустой слот (dispatch_bare шлёт Tui в `Target::Settings`; вида нет) | config_panel/, chatwidget/tui_mode_picker.rs | src/features/settings/ (нет config_panel) | ядро: fw1-c3 (влито); `w2-settings` |
| /diff | ❌ | `Target::Pager` показывает «not yet» (open.rs). Готово: core `DiffStep` (screens/transcript/diff.rs), `git_diff` + `WorkspaceExec` (src/services/shell.rs), `Ask::GitDiff` → `Done::GitDiff` (services/effects.rs, host-тесты). Нет: ответ → документ pager, проверка stale cwd, открыватель | get_git_diff.rs, diff_render.rs | src/pages/session/transcript/diff_view.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| /mention | ✅ | вставляет «@» | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /status — карточка статуса | 🟡 | карточка ✅ — ячейка транскрипта треда, как у oracle (status-card-adr003); landed e1795b779 (батч 3): живая карточка из фактов треда, config, аккаунта, каталога моделей и удалённого подключения; лимиты с обновлением по `RateLimitOrigin::Status(request)`, строки Model (display name, reasoning, summaries), Collaboration mode, Agents.md; pty: строки равны oracle. Нет: Thread name (oracle делает title-запрос, порт нет), версия в заголовке `(v0.0.0)` вместо `(v0.170.0)` (версия сборки); thread usage, server row, limit warnings ❌ (дети); лента фактов статус-строки (Cwd/Clock/SideLabel/AgentLabel, git-пробы) не построена | status/card.rs, chatwidget/thread_usage.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| /daemon | ❌ | `Target::Select` (остаток /daemon) показывает «not yet» (`daemon_action`); модель `select::daemon` в core есть | app/daemon_menu.rs | src/sheets/select/daemon.rs (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /warnings (и F2) | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /cd <путь> | ✅ | с отказами | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /pwd, /cwd | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /usage | 🟡 | нужен backend auth, иначе заметка UsageNeedsLogin; страница есть (43 теста не включены); меню reset ❌ | chatwidget/usage.rs, analytics/ | src/pages/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| /debug-config | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /title — терминальный title | 🟡 | bare /title открывается и сохраняет через `Ask::ConfigWrite` (`items::title`, батч 10); `/title <args>` — NotAvailable | chatwidget/status_surface_commands.rs, bottom_pane/title_setup.rs | src/features/settings/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| /statusline | 🟡 | bare /statusline открывается и сохраняет (`items::status_line`); `/statusline <args>` — NotAvailable | status/card.rs, chatwidget/thread_usage.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| /theme | ✅ | settings Theme; 790cc44a6 (батч 6): без темы в config пикер открывается на теме терминала по умолчанию (строка «(current)» и курсор), превью по центру под строкой отступа панели — строки пикера равны live oracle (pty settings-theme). Нет: подсказка футера (у нас `Press enter to confirm or esc to go back`, у live oracle `enter select · esc back`; снимки oracle-шардов совпадают со стандартной); лента фактов host подключена (`settings_fact`, батч 10), темы из config и `{codex_home}/themes` должны быть в бинаре (pty не перегонялся); клик по барьеру не возвращает тему превью (как было) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /pets, /pet | 🟡 | sheet pets открывается; ambient-часть и тесты не включены | pets/, chatwidget/pets.rs | src/sheets/pets/ | `pets-ambient` |
| /mcp | ❌ | NotAvailable; ячейка инвентаря | history_cell/mcp.rs | src/pages/session/cells/mcp_inventory.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| /apps | ✅ | диалог ссылки приложения, вкл/выкл, сброс при смене аккаунта, обновление каталога (b2eeacb7c); partial-snapshot тесты n/a (нет is_final:false в протоколе) | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /plugins | 🟡 | влит 8c492bcbc: список, установка, marketplaces. Пилот ADR-013 (батч 8, 454b13f11): Esc как у oracle (закрывает на карточке, loading и failure; возвращает только на подтверждении удаления и prompt add-marketplace), `Back to plugins` — строка карточки (курсор, вкладка и запрос сохраняются), setup-ссылка через `Ask::OpenUrl`, страницы — маршруты вложенного Navigator; pty рядом с oracle: браузер, вкладки, поиск, карточка, add-marketplace равны. Различия: усечение `…` после пробела (общий TextOverflow::Ellipsis пикера), переход prompt → failure add-marketplace не гонялся; красный на tip chat_popups_2 `plugins_popup_preserves…` (debt-b9-session) | chatwidget/plugins.rs, plugin_catalog.rs | src/sheets/select/ | ядро: fw1-c9 (влито); `w2-approvals` |
| /logout | ❌ | NotAvailable | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /quit, /exit | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /feedback | 🟡 | батч 9 (features-trait, d776d1a43): `/feedback` открывает страницу Feedback листа dialogs (категория, consent) — раньше открывал пустой лист; заметка и загрузка логов не написаны; upload-consent/diagnostics снимки ждут (27 тестов) | bottom_pane/feedback_view.rs, feedback_note_view.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /rollout | ✅ | — | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /ps — фоновые терминалы | ❌ | NotAvailable | bottom_pane/unified_exec_footer.rs, history_cell/exec.rs | src/pages/session/cells/processes.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| /stop, /clean | ✅ | чистит терминалы + заметка | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /clear | ✅ | новый тред/экран | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /test-approval | ✅ | Target::Approvals | slash_command.rs, chatwidget/slash_dispatch.rs | core/src/commands/dispatch.rs, core/src/app/session.rs | — |
| /subagents (multi-agents picker) | ❌ | `Target::AgentPicker` показывает «not yet» (`action_subagents` в open.rs); `app/multi_agents.rs` не портирован | app/multi_agents.rs, agent_navigation.rs | src/sheets/select/agent_picker.rs (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| /debug-m-drop, /debug-m-update | 🟡 | заглушка-заметка `NoteKey::Stub(MemoryMaintenance)` (debug) | daybreak.rs, ide_context.rs, app/recap.rs, debug_config.rs | core/src/commands/dispatch.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |

### Попапы и селекторы

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Список выбора: поиск, стрелки, цифры, disabled-строки, hint по биндингам | ✅ | — | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | — |
| Model picker: каталог, discovery, bedrock/custom display, service tier | 🟡 | landed 87ad7733c: скрытые модели, seed провайдера, лимит потоков агента; осталось: refresh каталога, reasoning submenu (extra high, advanced), снимки | chatwidget/model_popups.rs, model_catalog.rs | core/src/screens/select/model.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Reasoning picker и Alt+,/. , Shift+Up/Down | ✅ | reach_max (красный), ultra, advanced-only ждут | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Permissions popup: пресеты, профили, retry discovery | 🟡 | full access confirm, Windows sandbox prompts, history cell решения | chatwidget/permission_popups.rs, permission_discovery.rs | src/sheets/select/permissions.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Windows sandbox: prompts, help-ссылки, fallback | ❌ | весь блок (≈10 тестов); только Windows | chatwidget/windows_sandbox_prompts.rs | src/sheets/select/permissions.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Goal: меню и edit prompt, resume-paused prompt | 🟡 | меню и edit prompt — лист фичи `features/goal/sheet.rs` (батч 9); снимки не включены; resume-paused работает, тест не включён | chatwidget/goal_menu.rs | src/features/goal/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Plan: popup реализации и scope reasoning | ✅ | — | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Collaboration / mode catalog | 🟡 | каталог недоступен (тест не включён) | collaboration_modes.rs, chatwidget/plan_implementation.rs | core/src/screens/select/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Worktree/checkout picker | 🟡 | лист worktree (`select/worktree.rs`, `Opening::Worktree`) открывается из /worktree; нет исполнителя `Ask::Worktrees`/`WorktreeRemove` и шага старта; `OpenCheckout` не шлётся (/new, /fork) | chatwidget/worktree_picker.rs | src/sheets/select/worktree.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Quick dials (saved dials: save/clear picker) | ❌ | лист `Opening::Quick` и debug-вход есть, входа из чата нет (`Intent::Quick` не шлётся, /presets) | chatwidget/quick_dials.rs | src/sheets/select/quick.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Plugins: список, вкладки, поиск, деталь, install, marketplaces | 🟡 | влит 8c492bcbc и пилот ADR-013 (см. строку /plugins); осталось: усечение после пробела, add-marketplace failure не гонялся, красный plugins_popup_preserves… | chatwidget/plugins.rs, plugin_catalog.rs | src/features/plugins/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Apps/connectors: список, статус, install-only | 🟡 | простые состояния ✅; partial refresh ❌ | chatwidget/connectors.rs, bottom_pane/app_link_view.rs | src/sheets/select/app_link/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Hooks popup: список, диагностика | ❌ | sheet `features/settings/hooks.rs` есть; `Which::Hooks` → `return`, ленты hooks/list нет | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Experimental features popup (toggle сохраняется на выходе) | ❌ | sheet `features/settings/features.rs` есть; `Which::Experimental` → `return`, ленты experimentalFeature/list нет (toggle сохраняется на выходе — не проверено) | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Feedback: выбор, consent upload, диагностика, note | 🟡 | страница Feedback открывается из `/feedback` (батч 9); note и логи нет; снимки ждут | bottom_pane/feedback_view.rs, app/feedback_upload.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Review picker (ветка/коммит/custom) | ✅ | — | chatwidget/*_popups.rs, bottom_pane/list_selection_view.rs | src/sheets/select/ | — |
| Copy/export picker (кастомные клавиши, payload) | ❌ | export-назначение есть (`features/export`, см. /export); `/copy` picker и payload нет | chatwidget/copy_picker.rs, transcript_export.rs | src/features/export/, src/features/copy/ (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Reset-limit picker и подтверждение (usage menu) | ❌ | ≈30 тестов chat_live_6 | chatwidget/reset_credits.rs, usage.rs | src/pages/session/live/usage_menu.rs | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Security setup / security prompts | ❌ | security_setup_* (≥4) | security_setup.rs, chatwidget/security_setup.rs | src/features/security_setup/ (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Memories settings и reset | 🟡 | экран и reset готовы; открывается, когда host выдал факты memory tool (см. /memories) | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Manage skills summary | ❌ | sheet `features/settings/setup.rs` есть; `Which::Skills` → `return`, ленты skills/list нет | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Current settings (model/permissions/status layout) | ❌ | current_settings_offer_* | bottom_pane/{hooks_browser_view,experimental_features_view,memories_settings_view,skills_toggle_view}.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Sweep: прочие мелкие popup'ы | ? | состав «sweep» не установлен по коду; задача created | bottom_pane/list_selection_view.rs | src/sheets/dialogs/ | ядро: fw1-c9 (влито); `w2-approvals` |

### Approvals и вопросы

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Approval exec: модал, варианты, execpolicy-префикс, причина | ✅ | multiline-prefix скрытие execpolicy ждёт | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | — |
| Approval apply_patch: модал, call_id, заголовок | 🟡 | untrusted модал, manual flow, patch snapshot ждут | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Approval network host (разово/сессия) | 🟡 | history decision cell ждёт; запрос ✅ | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Approval write_stdin | ❌ | render terminal input | bottom_pane/approval_overlay.rs, history_cell/approvals.rs | src/sheets/approvals/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Permissions request approval, MCP elicitation форма | ✅ | — | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| Guardian review (approved/denied/timed out, aggregate status) | ❌ | 0 из 11 | auto_review_denials.rs, history_cell/approvals.rs | src/sheets/approvals/guardian.rs (нет) | ядро: fw1-c5, fw1-c6 (влито); `w2-cells` |
| Auto-review denials popup | ✅ | sheet denials (`open_auto_review`); снимок ждёт | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| request_user_input: вопросы с вариантами, цифры/буквы | ✅ | батч 7 (b3ae82940): клавиши диалогов вопросов через keymap (`keys.rs`), `Draft.selected` → `choice`; поведение то же | bottom_pane/approval_overlay.rs, request_user_input/ | src/sheets/approvals/ | — |
| Вопросы: черновики, навигация, replay черновиков | ❌ | question_drafts_survive_*, turn_end drafts (≈10) | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Вопросы: поиск по истории, Vim в поле ответа | ❌ | history search; Vim в поле ответа не подключён (approvals_1, chat_approvals_2 ждут, батч 5 их не трогал) | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Асинхронные вопросы (desktop answer, уведомления) | 🟡 | уведомления n/a; многое n/a — другой клиент | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Отклонение пустых ответов, явный skip | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Разворот вопроса уступает мышь терминалу для ссылок | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Очередь вопросов и queued messages делят один shortcut | ❌ | — | bottom_pane/request_user_input/, async_questions/ | src/sheets/approvals/, src/pages/session/cells/request_input.rs | ядро: fw1-c9 (влито); `w2-approvals` |
| Keymap-seeds для approval | ❌ | — | keymap.rs | tests/kit/ | `kit-seeds` |

### Футер и статус-строка

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режимы | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Подсказка очереди «Alt+Up edit» | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Строка статуса: модель, effort, cwd, context %, токены | ✅ | git-ветка, PR со ссылкой, изменения ветки, headline, raw output (ea69879b9); нет: Cwd/Clock-факты и пробы git/headline от host, заголовок треда в строке | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Настройка статус-строки (/statusline, порядок элементов) | 🟡 | bare /statusline открывает и сохраняет (`Ask::ConfigWrite`); аргументы NotAvailable | bottom_pane/status_line_setup.rs, title_setup.rs, terminal_title.rs | src/features/settings/, src/pages/session/window_title.rs | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Терминальный заголовок (title) с элементами | 🟡 | рендер ✅; настройка bare /title ✅ (`/title <args>` NotAvailable) | bottom_pane/status_line_setup.rs, title_setup.rs, terminal_title.rs | src/features/settings/, src/pages/session/window_title.rs | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Индикаторы режимов: Plan, goal, side, agent, Vim | ✅ | рисуются (ea69879b9); подпись Vim следует режиму (fdadb68ff); метки side/agent ждут фактов от host | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Индикатор голоса (voice strip), F8 | ❌ | — | bottom_pane/voice_strip.rs | src/pages/session/footer/voice.rs (нет) | `voice` |
| Индикатор эффекта effort ignition | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Sparkle/«искры» над composer | 🟡 | задача занята | bottom_pane/chat_composer/sparkle.rs | src/pages/session/footer/sparkle.rs | ядро: fw1-c2 (влито); `w2-composer` |
| Счётчик предупреждений (F2) в футере | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Flash в футере и временный override подсказки | 🟡 | таймер flash и override есть (ea69879b9); на странице нет источника вспышки | bottom_pane/footer.rs, footer_hint.rs | src/pages/session/footer/flash.rs | ядро: fw1-c2 (влито); `w2-composer` |
| Строка follow («вернуться вниз») над composer | ✅ | — | bottom_pane/footer.rs, status_line.rs | src/pages/session/footer/, status/ | — |
| Карточка /status: основное | ✅ | ячейка транскрипта (ADR-003), e1795b779: Model с reasoning и summaries, Collaboration mode, Permissions, Agents.md, Session, Token usage, Context window, Limits; нет Thread name (title не генерируется) | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | — |
| Карточка /status: usage треда | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Карточка /status: строка сервера | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Предупреждения о лимитах в карточке и ленте | ❌ | — | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | ядро: fw1-c2 (влито); `w2-composer` |
| Подпись контекста side-разговора | ❌ | — | bottom_pane/chat_composer/status_surface.rs, app/side_companion.rs | src/pages/session/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Подсказка reset-лимита на старте | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |

### Транскрипт-оверлей / pager / backtrack / выделение-копирование-поиск

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Ctrl+T: подробный транскрипт (sheet) с reasoning и warnings | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Esc-Esc: backtrack — выбор прошлого prompt, Enter откат | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | ?: часть тестов backtrack всего 4 | pager_overlay.rs, transcript_view/ | src/pages/session/transcript/ | — |
| Статический pager (/diff, вывод): строки, страницы, Home/End, q, проценты | 🟡 | есть как режим страницы (`StaticPager`, core/src/screens/transcript.rs); /diff «not yet» (см. /diff), live-tail sync и поиск ❌ | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Pager: поиск /, n/N | ❌ | — | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Pager: live tail sync при росте транскрипта | ❌ | — | pager_overlay.rs, pager_overlay/ | src/pages/session/transcript/pager.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Мышью выделить текст в транскрипте (подсветка) | ✅ | SelectionArea рисует выделение | pager_overlay.rs, transcript_view/ | src/pages/session/transcript/ | — |
| Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | landed 542959815: копия по отпусканию, Ctrl+C / Cmd+C / Ctrl+Shift+C, Enter = копия и вниз, Esc снимает, подсказка в футере, строка «Copied N chars to host clipboard», выделение снимается после подтверждённой копии (pty text-100: рядом с oracle совпадает); сервис копирования clipboard-service (5026ad24c). Расхождения: конец диапазона у oracle до ячейки под курсором, у rtwk SelectionArea включает её («Copied 12 chars» вместо 11, rtwk gap); OSC 52 при копии по отпусканию пишется дважды (clipboard-provider в rtwk). Sweep: X11 primary, deselect сравнивает текст, не диапазон, oracle-снимки на 3-4 строках (kit::pending) | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Двойной/тройной клик: слово/строка/абзац | ❌ | по паку это жесты rtwk SelectionArea; тестов и проверки в бинаре у порта нет | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Shift-клик расширяет, правая кнопка копирует | 🟡 | правая кнопка копирует (Listener в SelectionHost, 542959815, тест в selection_copy.rs); Shift-клик и расширение с клавиатуры не построить: у SelectionAreaController есть `select(target, item, start, end)`, но нет чтения start/end (rtwk gap) | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Копия как Markdown/plain (payload) | ❌ | HTML-вариант Markdown-копии в сервисе готов (5026ad24c); выбора формата в UI нет; копия выделения мышью только plain text (CopyFormat::MarkdownSelection ждёт тексты элементов); transcript_view_3 в основном n/a | markdown_copy.rs, clipboard_html.rs | src/pages/session/transcript/copy.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Режим выделения с клавиатуры (select mode) | ❌ | нет модуля | transcript_view/select_mode.rs | src/pages/session/transcript/select_mode.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Режим копирования (copy mode, Ctrl+X-лидер) | ❌ | нет модуля | transcript_view/copy_mode.rs | src/pages/session/transcript/copy_mode.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | F3 только открывает sheet | transcript_view/search*.rs | src/pages/session/transcript/find.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Подсказки поиска и футер поиска | ❌ | — | transcript_view/search*.rs | src/pages/session/transcript/find.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Скорость колеса (mouse_scroll_speed) | ❌ | настройка есть, не применяется | tui/scrollback.rs, app/history_pagination.rs | src/pages/session/transcript/follow.rs, src/pages/session/transcript/history.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Старые страницы в overlay (history pages) | 🟡 | ee4c83bd9: статус-строка sheet транскрипта берёт спеллинги из `transcript/history.rs` (`sheet::status`); нет ctrl+home/alt+< и склейки групп (см. строку выше) | tui/scrollback.rs, app/history_pagination.rs | src/pages/session/transcript/follow.rs, src/pages/session/transcript/history.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Сайдбар (панель агентов/тредов) | ❌ | ADR-закрытая задача sidebar | app/sidebar*.rs, sidebar_tab_strip.rs | src/pages/session/sidebar/ (нет) | `sidebar` |
| Inline-режим (без alt-screen) | ❌ | задача created | tui.rs, insert_history.rs, app/resize_reflow.rs | src/app/screen_mode/ (нет) | `inline-mode` |
| Дерево UI (ui-tree) | ? | не относится к ленте; задача created | ui_tree.rs, ui_tree/ | src/widgets/ui_tree/ (нет) | `ui-tree-viz` |
| Клик по ссылке/файлу в overlay | 🟡 | обычный клик красный | transcript_view/selection.rs, rendered_selection.rs, text_selection.rs | src/pages/session/transcript/selection.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |

### Диалоги

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Предупреждения (/warnings, F2): страницы, wrap | 🟡 | warning_pages_wrap красный | bottom_pane/chat_composer.rs, textarea.rs, paste_burst.rs | src/pages/session/composer/, src/widgets/composer_draft/ | ядро: fw1-c2 (влито); `w2-composer` |
| Шорткаты: оверлей «?» со всеми действиями | 🟡 | ab78ae687 (батч 6): страница шорткатов — состояние sheet на границе ADR-013, F2 и копирование через keymap; gap: из чата оверлей достижим только из debug-меню | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Rename thread prompt (bare /rename) | ✅ | slice features/rename (a15c35bf8), вид prompt 29abc1aa5; с батча 9 prompt — UI-половина фичи (`features/rename/prompt.rs`); нет предложения имени (Done::Title(None), нужен SuggestTitle; gap из hand-in батча 9: предложение имени не в поле ввода — нужно покрытие pty/oracle) | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Confirm: archive/delete треда | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Daemon (подключение к daemon) | ❌ | попап «not yet» (см. /daemon) | app/daemon_menu.rs, bottom_pane/app_link_view.rs | src/sheets/select/ | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Ссылка на приложение (app link) диалог | ❌ | — | app/daemon_menu.rs, bottom_pane/app_link_view.rs | src/sheets/select/ | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Custom review prompt | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Верификация пользователя (user verification) | ❌ | — | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Settings: Theme, Memories (из чата) | 🟡 | Theme ✅ (лента фактов host подключена, батч 10); Memories открывается только при фактах memory tool (`facts.memories`); строки Theme и настройки принимают тап (`GestureDetector`) | theme_picker.rs, bottom_pane/memories_settings_view.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Settings: Config / Statusline / Title / Experimental / Skills / Hooks / Keymap | 🟡 | Statusline и Title ✅ (bare; см. строки команд); Config и Keymap — пустой слот без вида; Experimental, Skills, Hooks — sheet есть (features.rs, setup.rs, hooks.rs), открыватель `Which::*` → `return`, ленты host нет. `Fact::Offered` приходит только из seed/тестов/debug-меню, host его не шлёт | config_panel/, bottom_pane/status_line_setup.rs | src/features/settings/ | ядро: fw1-c3 (влито); `w2-settings` |
| Feedback (диалог) | 🟡 | ab78ae687: шаг feedback — состояние sheet; с батча 9 из чата достижим командой `/feedback` (страница Feedback); снимки consent/diagnostics ждут | bottom_pane/feedback_view.rs, app/feedback_upload.rs | src/sheets/dialogs/feedback.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Pets (диалог выбора) | 🟡 | ab78ae687: курсор и превью — состояние sheet; глаголы LoadPreview, ReleasePreview; ambient-часть и тесты не включены | pets/picker.rs | src/sheets/pets/ | `pets-ambient` |
| Voice (диалог, F8) | ❌ | — | chatwidget/realtime_settings.rs | src/sheets/voice/ | `voice` |
| Trust on open (запрос доверия к каталогу) | ? | в startup-шарде 29/32 зелёные; из чата не вызывается | onboarding/trust_directory.rs | src/pages/startup/ | `startup-rest` |
| Model provider switch (блокирует кэш выбора) | ❌ | provider_switch_* | app/provider/, chatwidget/model_popups.rs | core/src/screens/select/model.rs | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Debug menu F12 | ✅ | — | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |
| Список выбора (SelectionList): тап по строке | 🟡 | gap (2f68c2c75): тап по строке только двигает курсор, как у rtwk-виджета; выбор строки — клавишей | bottom_pane/warnings_view.rs, shortcut_overlay.rs, user_verification.rs, list_selection_view.rs | src/sheets/dialogs/ | ядро: fw1-c9 (влито); `w2-approvals` |
| Resume picker: клавиши oracle (ctrl-p/n/k/j/h/l/b/f), ctrl+e на пустом id | ✅ | landed 656a4d753 (батч 6, adr013-resume): список и курсор — через keymap (`LIST_*`), плотность и открытая строка — состояние страницы, дозагрузка — один verb `LoadMore`; ctrl+e на пустом id игнорируется как у oracle. На ee6bad4e8 resume_2 `page_and_jump_navigation` больше не красный (resume_1 + resume_2: 49 зелёные, 12 pending, 0 red) | bottom_pane/*_view.rs | src/sheets/dialogs/ | — |

### /usage

| Функция | Статус | Чего нет | Oracle | Порт | Пакет |
|---|---|---|---|---|---|
| Страница аналитики: бары, вкладки, часы плана, narrow-графики | 🟡 | рендер работает, 43 теста не включены; батч 7 (usage-agents, f1675bfa8, ecdfcbed1): usage live/Leave, группы не в Pos, нет whole-state watch, диалог ошибки по привязанным ключам, помощь закрывается, когда секция не меняется; поведение то же | analytics/ | src/pages/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Меню /usage: Analytics, Refresh, Reset | 🟡 | в чате 0 из 25: пункты меню и reset не подключены | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Reset лимита: пикер, подтверждение, idempotency-ключ, in-flight | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Billing и enterprise-планы | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Подсказка reset на старте | ❌ | — | status/card.rs, chatwidget/usage_notice.rs | src/pages/session/cells/status/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Статус после billing retries | ❌ | — | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
| Ранний redraw и повтор thread usage | ❌ | n/a в harness | chatwidget/usage.rs, reset_credits.rs, thread_usage.rs | src/pages/session/live/usage_menu.rs, core/src/screens/usage/ | ядро: fw1-c7, fw1-c3, fw1-c2 (влито); `w2-status` |
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
| Tab — в очередь (сообщения) | 🟡 | команды и shell в очереди ❌ | keymap.rs, chatwidget/input_queue.rs | src/pages/session/composer/drain.rs | ядро: fw1-c2 (влито); `w2-composer` |
| Alt+Up — вернуть из очереди | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Alt+, / Alt+. — reasoning | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Shift+Up/Down — reasoning | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Alt+R — raw | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| F2 — warnings | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| F3 — поиск | 🟡 | открывает sheet без поиска | keymap.rs, transcript_view/search.rs | src/pages/session/keys.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| F4 — focus activity | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | keymap.rs, transcript_view/search.rs | src/pages/session/keys.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| F8 — голос | 🟡 | открывает пустой sheet | keymap.rs, bottom_pane/voice_strip.rs | src/pages/session/key_actions/voice.rs (нет) | `voice` |
| F12 — debug menu | ✅ | — | keymap.rs | core/src/keymap/, src/pages/session/keys.rs | — |
| Ctrl+/ (Ctrl+_, Ctrl+7) — side | ❌ | `Notice::NotAvailable(Side)` (session_page.rs:683) | keymap.rs, app/side.rs | src/pages/session/key_actions/side.rs (нет) | ядро: fw1-c1, fw1-c2, fw1-c8, fw1-c4, fw1-c9 (влито); `w2-commands` |
| Ctrl+O — копировать ответ | ❌ | в keymap/catalogue.rs есть, обработчика нет | keymap.rs, chatwidget/copy_picker.rs | src/pages/session/key_actions/copy.rs (нет) | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Ctrl+G — внешний редактор | ❌ | в keymap/catalogue.rs есть, обработчика и запуска $EDITOR нет | keymap.rs, external_editor.rs | src/pages/session/key_actions/external_editor.rs (нет) | ядро: fw1-c2 (влито); `w2-composer` |
| Ctrl+V / Alt+V — вставка картинки | 🟡 | `Ask::ClipboardImage` обслуживается (см. строку composer); с настоящим буфером в бинаре не гонялось | clipboard_paste/ | src/services/clipboard/ | ядро: fw1-c2 (влито); `w2-composer` |
| Ctrl+L — очистить терминал | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | tui/job_control.rs, keymap.rs | src/app/terminal_effects.rs | `terminal-features` |
| Ctrl+Z — suspend | ? | в keymap зарезервирован; обработчик не найден | tui/job_control.rs, keymap.rs | src/app/terminal_effects.rs | `terminal-features` |
| Ctrl+X — leader (Ctrl+X b — смена бэкенда) | 🟡 | leader живёт только для баннера | keymap.rs (leader) | core/src/keymap/ | ядро: fw1-c3 (влито); `w2-settings` |
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
| Turn tips и tooltips | 🟡 | ячейка session_info показывает подсказки; нет `turn_tip` в ходе и каталога tooltips | turn_tip.rs, tooltips.rs, tooltips/, app/turn_tips.rs | src/pages/session/cells/session_info.rs | ядро: fw1-c1, fw1-c4 (влито); `w2-transcript` |
| Empty-state animation | 🟡 | порт: `blossom`; нет процедурного освещения и политики пустого состояния | empty_state_animation.rs, empty_state_animation/, app/empty_state_policy.rs, bottom_pane/empty_state_policy.rs | src/pages/session/blossom/ | `w2-live` |
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
| S10 | main chat | 🟡 | см. разделы «Транскрипт», «Живой ход», «Composer», «Футер» | `w2-live` |
| S11 | streaming and history cells, markdown, diff | 🟡 | см. «Транскрипт и ячейки» | `w2-cells` |
| S12 | composer popovers | ✅ | — | — |
| S13 | approvals and questions | 🟡 | решение в транскрипте, write_stdin, черновики вопросов | `w2-approvals` |
| S14 | generic selection dialog | 🟡 | model/permissions хвосты, small_dialogs | `w2-approvals` |
| S15 | settings dialogs | 🟡 | /config, /experimental, /skills, /hooks, keymap editor, провайдер | `w2-settings` |
| S16 | transcript pager, static pager, search | 🟡 | статический pager, поиск | `w2-transcript` |
| S17 | backtrack | ✅ | — | — |
| S18 | resume / fork picker | 🟡 | 12 pending, превью транскрипта | `resume-rest` |
| S19 | /status card | 🟡 | usage треда, строка сервера, лимиты | `w2-status` |
| S20 | /usage dashboard | 🟡 | reset-flow, billing, подсказка | `w2-status` |
| S21 | agents command center, subagent switcher | 🟡 | agent status feed; switcher (/subagents) | `agents-center` |
| S22 | sidebar and side conversation | ❌ | обе панели | `sidebar` |
| S23 | small dialogs | 🟡 | daemon menu, app link, user verification, feedback | `w2-approvals` |
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
| term.osc8-hyperlinks | OSC 8 hyperlinks | Гиперссылки в тексте (клик открывает) | 🟡 | `w2-transcript` |
| term.osc9-notifications | Desktop notifications (OSC 9 / BEL) | Уведомления терминала о завершении хода / approval | 🟡 | `terminal-features` |
| term.osc52-clipboard | Clipboard copy (native, tmux, WSL, OSC 52) | Clipboard copy (native, tmux, WSL, OSC 52) | ✅ | — |
| term.clipboard-paste-image | Clipboard image paste + right-click paste | Картинки из буфера Ctrl+V / Alt+V | 🟡 | `w2-composer` |
| term.mouse | Mouse support (wheel, click, drag, hover, right-click) | Мышь | ✅ | — |
| term.probe-colors | Terminal probing (OSC 10/11 colors, DA1, cursor pos) | Terminal probe (OSC 10/11, DA1) | ❌ | `terminal-features` |
| term.osc-title | Terminal title (OSC 0) with configurable items | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `w2-status` |
| term.images-kitty-sixel-iterm | Inline images (kitty graphics / sixel / iTerm2) | Inline images (kitty / sixel / iTerm2) | ❌ | `pets-ambient` |
| term.tmux-wsl-screen | tmux / WSL / screen adaptations | tmux / WSL / screen | 🟡 | `terminal-features` |
| term.windows-console | Windows console specifics | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.panic-restore | Panic hook and terminal restore | Focus, synchronized output, Windows console, panic hook | ? | `terminal-features` |
| term.screen-reader | Screen reader / accessibility mode | Screen reader | ❌ | `terminal-features` |
| term.motion-reduce | Reduced-motion preference | Reduced motion | 🟡 | `terminal-features` |
| term.session-lock | OS session lock detection (voice ends) | OS session lock | ❌ | `voice` |
| term.external-editor | External editor handoff ($EDITOR) | Внешний редактор Ctrl+G | ❌ | `w2-composer` |
| app.shell | App event loop, thread routing, event dispatch | App shell | ✅ | — |
| app.startup-draft | Startup draft (composer before app-server ready) | Startup draft (черновик из аргументов, file-search star | 🟡 | `w2-composer` |
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
| onboarding.windows-sandbox | Windows sandbox setup prompts | Windows sandbox: prompts, help-ссылки, fallback | ❌ | `w2-commands` |
| onboarding.security-setup | Security setup flow | Security setup / security prompts | ❌ | `w2-commands` |
| chat.widget | ChatWidget: transcript tail + bottom pane | Отправка сообщения Enter, индикатор working/«Thinking», | ✅ | — |
| chat.rendering-layout | Chat vertical layout | Автопрокрутка за хвостом, «вернуться вниз», строка ново | ✅ | — |
| chat.session-header | Session header card | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| chat.protocol-events | Protocol event handling | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `w2-cells` |
| chat.interrupt | Interrupt turn (Esc / Ctrl+C) | Прерывание хода Esc | ✅ | — |
| chat.queued-input | Queued user messages while turn runs | Очередь сообщений (Tab / Enter в ходе) над composer | ✅ | — |
| chat.status-indicator | Working indicator (spinner, shimmer, elapsed, hint) | Отправка сообщения Enter, индикатор working/«Thinking», | ✅ | — |
| chat.status-line | Configurable status line | Строка статуса: модель, effort, cwd, context %, токены | ✅ | — |
| chat.footer | Composer footer (hints, quit reminder, esc hint) | Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режим | ✅ | — |
| chat.shortcut-overlay | Shortcut help ('?') | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `w2-approvals` |
| chat.effort-ignition | Reasoning effort ignition animation + shortcuts | Индикатор эффекта effort ignition | ✅ | — |
| chat.turn-tips | Turn tips / tooltips | Turn tips и tooltips | 🟡 | `w2-transcript` |
| chat.empty-state-animation | Empty-state animation (procedural lighting) + Daybreak | Empty-state animation | 🟡 | `w2-live` |
| chat.goal | Thread goal display + menu | Goal: меню и edit prompt, resume-paused prompt | 🟡 | `w2-approvals` |
| chat.plan-mode | Plan mode + implement-plan prompt | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| chat.review-mode | Review flows | Review-ячейки (баннер Current changes, request hint, pr | ✅ | — |
| chat.compaction | Context compaction progress | Compaction: статус, таймер, ячейка завершения | ❌ | `w2-cells` |
| chat.recap | Thread recap | /daybreak | ❌ | `w2-commands` |
| chat.side-conversation | Side conversation docked pane | Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпи | ❌ | `w2-commands` |
| chat.sidebar | Sidebar (documents, files, links) with slide animation | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| chat.read-only-thread | Read-only thread notice | Read-only внешний writer: заметка вместо composer, пере | ✅ | — |
| chat.provider-error | Provider error + safety buffering/recourse/misalignment | Safety buffering: «повторить», подтверждение, короткое  | ❌ | `w2-cells` |
| chat.mcp-startup | MCP startup status | MCP startup: статус, lag, отмена, отказ как warning | ✅ | — |
| chat.reconnect | Reconnect/retry banners | Stream recovery: восстановление статуса после обрыва | ✅ | — |
| chat.rate-limits | Rate limit warnings / reset credits | Rate-limit подсказки/предупреждения во время хода | ❌ | `w2-status` |
| chat.copy-last | Copy last response (Ctrl+O, /copy picker) | Copy/export picker (кастомные клавиши, payload) | ❌ | `w2-transcript` |
| chat.transcript-export | Export transcript as markdown | /export | ✅ | — |
| chat.reasoning-replay | Reasoning summary and replay | Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | — |
| chat.user-verification | User verification flow | Верификация пользователя (user verification) | ❌ | `w2-approvals` |
| chat.tool-lifecycle | Tool call lifecycle to active cells | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `w2-cells` |
| composer.chat-composer | ChatComposer (multiline input + popups + history) | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.textarea | TextArea editor (emacs bindings, kill ring, undo) | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.vim-mode | Vim mode (normal/insert/visual, operators, text objects, sea | Vim: нормальный/вставка/операторы/текст-объекты/поиск ( | 🟡 | `w2-composer` |
| composer.paste-burst | Paste-burst detection | Вставка: paste burst, placeholder большой вставки [Past | ✅ | — |
| composer.large-paste-placeholder | Large paste placeholders | Вставка: paste burst, placeholder большой вставки [Past | ✅ | — |
| composer.image-attachments | Image attachments ([Image #N]) | Картинки: локальные [Image #N], нумерация, удаление | ✅ | — |
| composer.history | Composer history | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.history-search | Reverse history search | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.undo-redo | Edit history undo/redo | Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, | ✅ | — |
| composer.draft-restore | Draft state and restore | Восстановление очищенного Ctrl+C prompt из истории | ✅ | — |
| composer.sparkle | Composer sparkle effect | Sparkle/«искры» над composer | 🟡 | `w2-composer` |
| composer.warning-notice | Inline composer warning | Состояния composer: disabled, shutdown, parent-owned, a | ❌ | `w2-composer` |
| composer.mouse | Composer mouse handling | Мышь в composer (курсор, выделение, вставка правой кноп | ❌ | `w2-composer` |
| composer.agents-navigation | Subagent navigation from composer | Left на пустом composer — Agents | ✅ | — |
| composer.slash-popup | Slash command popup (fuzzy filter) | Slash-popup: фильтр, стрелки, Tab/Enter, описание, скры | ✅ | — |
| composer.file-mention-popup | @ file search popup | $ skills / плагины / apps в упоминаниях | 🟡 | `w2-composer` |
| composer.mentions-v2 | Mentions v2 popup (files, skills, plugins, apps, tasks; tabs | $ skills / плагины / apps в упоминаниях | 🟡 | `w2-composer` |
| composer.skill-popup | Skill popup ($skill) | $ skills / плагины / apps в упоминаниях | 🟡 | `w2-composer` |
| composer.voice-strip | Voice strip + realtime voice | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| composer.split-flap | Realtime split-flap display | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| composer.ide-context | IDE context integration | /daybreak | ❌ | `w2-commands` |
| composer.quick-dials | Quick dials / saved presets | Quick dials (saved dials: save/clear picker) | ❌ | `w2-commands` |
| composer.assistant-directives | Assistant directives parsing | Assistant directives и git action directives | ✅ | — |
| view.list-selection | ListSelectionView (generic picker: search, tabs, columns, si | Список выбора: поиск, стрелки, цифры, disabled-строки,  | ✅ | — |
| view.approval-overlay | ApprovalOverlay (exec / patch / permissions / MCP) | Permissions request approval, MCP elicitation форма | ✅ | — |
| view.mcp-elicitation | McpServerElicitationOverlay (MCP form) | Permissions request approval, MCP elicitation форма | ✅ | — |
| view.request-user-input | RequestUserInputOverlay (model questions) | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| view.async-questions | AsyncQuestions (non-blocking question stack) | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| view.app-link | AppLinkView (connect ChatGPT app) | Apps/connectors: список, статус, install-only | 🟡 | `w2-approvals` |
| view.custom-prompt | CustomPromptView (text prompt) | Custom review prompt | ✅ | — |
| view.feedback-note | FeedbackNoteView | Feedback: выбор, consent upload, диагностика, note | 🟡 | `w2-commands` |
| view.multi-select | MultiSelectPicker | Custom review prompt | ✅ | — |
| view.status-line-setup | StatusLineSetupView (/statusline) | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `w2-status` |
| view.title-setup | TerminalTitleSetupView (/title) | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `w2-status` |
| view.skills-toggle | SkillsToggleView | Hooks popup: список, диагностика | ❌ | `w2-settings` |
| view.experimental-features | ExperimentalFeaturesView | Hooks popup: список, диагностика | ❌ | `w2-settings` |
| view.hooks-browser | HooksBrowserView (/hooks) | Hooks popup: список, диагностика | ❌ | `w2-settings` |
| view.memories-settings | MemoriesSettingsView | Hooks popup: список, диагностика | ❌ | `w2-settings` |
| view.user-verification | UserVerificationView | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `w2-approvals` |
| view.warnings | WarningsView (/warnings) | Шорткаты: оверлей «?» со всеми действиями | 🟡 | `w2-approvals` |
| view.config-panel | ConfigPanel (/config, tabbed settings with sub-pickers) | Settings: Config / Statusline / Title / Experimental /  | 🟡 | `w2-settings` |
| view.keymap-capture | KeymapCaptureView | /keymap — редактор клавиш | ❌ | `w2-settings` |
| view.keymap-debug | KeymapDebugView | /keymap — редактор клавиш | ❌ | `w2-settings` |
| view.keymap-picker | Keymap setup picker | /keymap — редактор клавиш | ❌ | `w2-settings` |
| view.agents-overview | AgentsOverviewView (/agents command center) | Agents command center | 🟡 | `agents-center` |
| view.provider-editor | Provider editor / list | Provider editor и cloud environment | ❌ | `startup-rest` |
| view.daemon-menu | Daemon menu (/daemon) | Quick dials (saved dials: save/clear picker) | ❌ | `w2-commands` |
| view.view-stack | ViewStack / CenteredView / DialogOverlay | ViewStack, CenteredView, DialogOverlay | ✅ | — |
| view.actionable-banner | InlineBanner above composer | Баннер бэкенда (нумерованные строки, CTA, отклонение по | ✅ | — |
| view.unified-exec-footer | Unified exec footer (background terminals) | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `w2-cells` |
| popup.model-picker | Model & reasoning effort picker | Model picker: каталог, discovery, bedrock/custom displa | 🟡 | `w2-approvals` |
| popup.permissions | Permissions / approval mode popup + full access confirm | Permissions popup: пресеты, профили, retry discovery | 🟡 | `w2-approvals` |
| popup.theme-picker | Syntax theme picker with live preview | Settings: Theme, Memories (из чата) | 🟡 | `w2-settings` |
| popup.pets | Pet picker with animated preview | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| popup.skills-menu | Skills action menu | Hooks popup: список, диагностика | ❌ | `w2-settings` |
| popup.plugins | Plugins browser (marketplaces, detail, install/upgrade) | Plugins: список, вкладки, поиск, деталь, install, marke | 🟡 | `w2-approvals` |
| popup.apps | Apps / connectors list | Apps/connectors: список, статус, install-only | 🟡 | `w2-approvals` |
| popup.usage | Usage popup + reset flow | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `w2-status` |
| popup.worktrees | Worktree browser / managed worktree actions | Worktree/checkout picker | 🟡 | `w2-commands` |
| popup.copy-picker | Copy to clipboard picker | Copy/export picker (кастомные клавиши, payload) | ❌ | `w2-transcript` |
| popup.voice-settings | Voice / sound device / channel pickers | Voice (диалог, F8) | ❌ | `voice` |
| popup.tui-mode | TUI mode (Inline / Fullscreen) | Settings: Config / Statusline / Title / Experimental /  | 🟡 | `w2-settings` |
| popup.archive-delete-confirm | Archive / Delete session confirmations | Confirm: archive/delete треда | ✅ | — |
| popup.quit-confirm | Quit and stop running sessions | Quit-confirm и task-running | ❌ | `app-misc` |
| popup.task-running | Task is still running choice | Quit-confirm и task-running | ❌ | `app-misc` |
| popup.goal-replace | Goal replace / resume prompts | Goal: меню и edit prompt, resume-paused prompt | 🟡 | `w2-approvals` |
| popup.daybreak-turn-on | Turn on Daybreak for next request | /daybreak | ❌ | `w2-commands` |
| popup.cloud-environment | Cloud environment picker | Provider editor и cloud environment | ❌ | `startup-rest` |
| popup.subagents | Subagents switcher | /subagents (multi-agents picker) | ❌ | `w2-commands` |
| popup.statusline-provider | Status line provider picker | Настройка статус-строки (/statusline, порядок элементов | 🟡 | `w2-status` |
| popup.feedback | Feedback rating + consent | Feedback: выбор, consent upload, диагностика, note | 🟡 | `w2-commands` |
| popup.session-unavailable | Unable to resume session error | /resume | 🟡 | `resume-rest` |
| popup.windows-sandbox | Windows sandbox prompts | Windows sandbox: prompts, help-ссылки, fallback | ❌ | `w2-commands` |
| popup.misalignment-policy | Chat paused as a precaution | Misalignment/guardian stop треда | ❌ | `w2-cells` |
| popup.debug-config | /debug-config layers | /daybreak | ❌ | `w2-commands` |
| overlay.transcript-pager | Transcript pager overlay (Ctrl+T, inline mode) | Статический pager (/diff, вывод): строки, страницы, Hom | 🟡 | `w2-transcript` |
| overlay.static-pager | Static pager overlay (diff, long output) | Статический pager (/diff, вывод): строки, страницы, Hom | 🟡 | `w2-transcript` |
| overlay.analytics | Analytics dashboard (usage charts, tasks, plan, chats, model | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `w2-status` |
| overlay.backtrack | Backtrack (Esc Esc): pick earlier prompt, revert, edit | Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | — |
| overlay.diff-view | /diff git diff viewer | /diff | ❌ | `w2-transcript` |
| overlay.inline-visualization | Inline visualization viewer | Дерево UI (ui-tree) | ? | `ui-tree-viz` |
| overlay.ui-tree | UI tree renderer (model-emitted declarative UI) | Дерево UI (ui-tree) | ? | `ui-tree-viz` |
| overlay.sidebar-chooser | Sidebar chooser | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| transcript.viewport | Owned-screen transcript viewport | PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | `w2-transcript` |
| transcript.selection-copy | Mouse selection + copy-on-select | Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | `w2-transcript` |
| transcript.select-mode | Keyboard select mode | Режим выделения с клавиатуры (select mode) | ❌ | `w2-transcript` |
| transcript.search | Transcript find | Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | `w2-transcript` |
| transcript.chrome | Disclosure, turn details, sticky prompt header, follow butto | Закреплённая шапка prompt над транскриптом (pinned head | ❌ | `w2-transcript` |
| transcript.thread-transcript | Thread transcript projection | Replay истории при resume/fork: сообщения, tool, reason | ✅ | — |
| transcript.resume-preview | Transcript preview in resume picker | /resume | 🟡 | `resume-rest` |
| resume.picker | Resume / fork session picker | /resume | 🟡 | `resume-rest` |
| status.card | /status card | /status — карточка статуса | 🟡 | `w2-status` |
| status.rate-limits | Rate limit bars | Rate-limit подсказки/предупреждения во время хода | ❌ | `w2-status` |
| streaming.controller | Streaming controller (deltas to commit ticks) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| streaming.markdown-stream | Markdown streaming (fences, table holdback, prose preview) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| markdown.renderer | Markdown renderer | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| markdown.math | Math rendering | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| markdown.mermaid | Mermaid diagram as text | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| markdown.syntax-highlight | Syntax highlighting + themes + wrapping utils | Подсветка кода и diff в ответах | ✅ | — |
| diff.render | Diff rendering | /diff | ❌ | `w2-transcript` |
| keymap.ctx-app | Keymap context: App (panes, sidebar, transcript open/find/se | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-chat | Keymap context: Chat (presets, voice, interrupt, effort, per | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-composer | Keymap context: Composer (submit, queue, shortcuts, history  | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-editor | Keymap context: Editor textarea | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-vim | Keymap context: Vim normal/operator/text-object | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-pager | Keymap context: Pager | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-list | Keymap context: List navigation | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.ctx-agents | Keymap context: Agents command center | Agents command center | 🟡 | `agents-center` |
| keymap.ctx-approval | Keymap context: Approval | Tab — в очередь (сообщения) | 🟡 | `w2-composer` |
| keymap.runtime | Runtime keymap: config, chords, leader, conflicts, reserved  | /keymap — редактор клавиш | ❌ | `w2-settings` |
| cell.plain | Plain lines cell | Базовые ячейки | ✅ | — |
| cell.labeled-tool | Labeled tool cell | Базовые ячейки | ✅ | — |
| cell.web-hyperlink | Web hyperlink cell | Гиперссылки в тексте (клик открывает) | 🟡 | `w2-transcript` |
| cell.prefixed-wrapped | Prefixed wrapped text cell | Базовые ячейки | ✅ | — |
| cell.composite | Composite cell | Базовые ячейки | ✅ | — |
| cell.user | User message cell | Сообщение пользователя (блок с фоном, пути, картинки-пл | ✅ | — |
| cell.agent-message | Agent message cell | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| cell.agent-markdown | Agent markdown cell (source-backed reflow) | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| cell.streaming-agent-tail | Streaming agent tail | Ответ агента: стриминг markdown, списки, таблицы, код,  | 🟡 | `w2-live` |
| cell.reasoning-summary | Reasoning summary cell | Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | — |
| cell.exec | Exec cell (command, output preview, spinner, exploring group | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `w2-cells` |
| cell.unified-exec-interaction | Unified exec interaction cell | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `w2-cells` |
| cell.unified-exec-processes | Background terminals list | Unified exec: фоновые терминалы, wait-streak, interacti | ❌ | `w2-cells` |
| cell.patch | Patch cell (edited files with diffs) | Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | — |
| cell.patch-failure | Patch failure cell | Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | — |
| cell.view-image | View image cell | Ячейка просмотра/генерации изображений | 🟡 | `w2-cells` |
| cell.image-generation | Image generation cell | Ячейка просмотра/генерации изображений | 🟡 | `w2-cells` |
| cell.mcp-tool-call | MCP tool call cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.mcp-inventory-loading | MCP inventory loading cell | Инвентарь MCP (вывод /mcp) | ❌ | `w2-cells` |
| cell.dynamic-tool-call | Dynamic tool call cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.web-search | Web search cell | Ячейка web search (запрос, детали, усечение) | ✅ | — |
| cell.computer-activity | Computer-use activity cell | Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | `inline-mode` |
| cell.hook | Hook run cell | Ячейка hook (запущен/завершён) | ✅ | — |
| cell.proposed-plan | Proposed plan cell | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.proposed-plan-stream | Proposed plan stream cell | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.streaming-plan-tail | Streaming plan tail | Режим Plan: popup «Implement this plan?», scope рассужд | ✅ | — |
| cell.plan-update | Plan update checklist cell | Ячейка плана (update_plan) и предложенный план | ✅ | — |
| cell.final-message-separator | Final message separator | Final «Worked for …» разделитель с длительностью | 🟡 | `w2-cells` |
| cell.session-header | Session header card | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| cell.session-info | Session info cell | Session info: шапка сессии (модель, cwd, подсказки, too | ✅ | — |
| cell.session-notice | Session notice cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.forked-thread | Forked thread notice | Строка «Forked thread» / имя треда в истории | ❌ | `w2-commands` |
| cell.clear-screen-header | Clear-screen header | Базовые ячейки | ✅ | — |
| cell.tooltip | Tooltip cell | Базовые ячейки | ✅ | — |
| cell.update-available | Update available notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.safety-access-block | Safety access block notice | Safety buffering: «повторить», подтверждение, короткое  | ❌ | `w2-cells` |
| cell.deprecation-notice | Deprecation notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.interruption-notice | Interruption notice | Deprecation / update-available / version-notice | ✅ | — |
| cell.thread-recap-loading | Recap loading cell | /daybreak | ❌ | `w2-commands` |
| cell.thread-recap | Thread recap cell | /daybreak | ❌ | `w2-commands` |
| cell.startup-warnings | Startup warnings cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.warning | Warning cell | Deprecation / update-available / version-notice | ✅ | — |
| cell.delegated-message | Delegated (subagent) message cell | Ячейка delegated (делегированный вывод tool) | ✅ | — |
| cell.request-user-input-result | Request-user-input result cell | request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — |
| cell.agent-status | Agent status feed cell | Ячейка delegated (делегированный вывод tool) | ✅ | — |
| cell.file | Sidebar file/document cell | Сайдбар (панель агентов/тредов) | ❌ | `sidebar` |
| cell.split-flap-transcript | Split-flap transcript cell | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| cell.status | Status card cell | /status — карточка статуса | 🟡 | `w2-status` |
| cell.approval-decision | Approval decision cells (approved/denied/guardian) | Решение approval в транскрипте (Approved/Denied + коман | ❌ | `w2-approvals` |
| cell.activity-group | Activity groups (collapsed tool activity) | Ячейка exec: команда, вывод, статус, длительность | 🟡 | `w2-cells` |
| cell.spoken-artifacts | Spoken artifacts / local and web link annotation | Индикатор голоса (voice strip), F8 | ❌ | `voice` |
| slash.model | /model - choose what model and reasoning effort to use | /model — выбор модели и effort | 🟡 | `w2-approvals` |
| slash.presets | /presets - run or save reusable prompts and slash commands | /presets — выбор пресета (Daybreak presets) | ❌ | `w2-commands` |
| slash.daybreak | /daybreak - turn Daybreak on or off | /daybreak | ❌ | `w2-commands` |
| slash.ide | /ide - include current selection, open files, and other cont | /ide — контекст IDE | ❌ | `w2-commands` |
| slash.permissions | /permissions - choose what Codex is allowed to do | /permissions — approvals/профили | 🟡 | `w2-approvals` |
| slash.keymap | /keymap - remap TUI shortcuts | /keymap — редактор клавиш | ❌ | `w2-settings` |
| slash.vim | /vim - toggle Vim mode for the composer | /vim — переключить Vim composer | ✅ | — |
| slash.setup-default-sandbox | /setup-default-sandbox - set up elevated agent sandbox | /setup-default-sandbox (Windows) | ❌ | `w2-commands` |
| slash.experimental | /experimental - toggle experimental features | /experimental — feature-флаги | ❌ | `w2-settings` |
| slash.approve | /approve - approve one retry of a recent auto-review denial | /approve — Auto-review | ✅ | — |
| slash.memories | /memories - configure memory use and generation | /memories | 🟡 | `w2-settings` |
| slash.skills | /skills - use skills to improve how Codex performs specific  | /skills | ❌ | `w2-settings` |
| slash.import | /import - import setup, this project, and recent chats from  | /import — перенос из других агентов | ✅ | — |
| slash.hooks | /hooks - view and manage lifecycle hooks | /hooks | ❌ | `w2-settings` |
| slash.review | /review - review my current changes and find issues | /review — выбор ветки/коммита/custom | ✅ | — |
| slash.rename | /rename - rename the current thread | /rename — имя треда | ✅ | — |
| slash.new | /new - start a new chat during a conversation | /new | 🟡 | `w2-commands` |
| slash.archive | /archive - archive this session | /archive | ✅ | — |
| slash.delete | /delete - permanently delete this session | /delete | ✅ | — |
| slash.resume | /resume - resume a saved chat | /resume | 🟡 | `resume-rest` |
| slash.fork | /fork - fork the current chat | /fork | 🟡 | `w2-commands` |
| slash.worktree | /worktree - start or continue a conversation in a new worktr | /worktree | 🟡 | `w2-commands` |
| slash.app | /app - continue this session in the Desktop app | /app — открыть в desktop-приложении | ✅ | — |
| slash.init | /init - create an AGENTS.md file with instructions for Codex | /init | ✅ | — |
| slash.compact | /compact - summarize conversation to prevent hitting the con | /compact | 🟡 | `w2-cells` |
| slash.config | /config - configure Codex preferences | /config | ❌ | `w2-settings` |
| slash.recap | /recap - summarize the current conversation now | /recap | ✅ | — |
| slash.plan | /plan - switch to Plan mode | /plan [prompt] | ✅ | — |
| slash.voice | /voice - start or stop voice; use /voice settings to choose  | /voice | ❌ | `voice` |
| slash.goal | /goal - set or view the goal for a long-running task | /goal [цель/pause/resume/clear] | 🟡 | `w2-approvals` |
| slash.agents | /agents - open the agent command center | /agents — центр агентов | ✅ | — |
| slash.side | /side - start a side conversation | /side, /btw | ❌ | `w2-commands` |
| slash.btw | /btw - start a side conversation | /side, /btw | ❌ | `w2-commands` |
| slash.copy | /copy - copy the last response or part of it | /copy — копировать ответ | ❌ | `w2-transcript` |
| slash.export | /export - export the conversation as markdown | /export | ✅ | — |
| slash.raw | /raw - toggle raw scrollback mode for copy-friendly terminal | /raw | ✅ | — |
| slash.tui | /tui - choose the TUI mode for the next launch | /tui — быстрые настройки TUI | ❌ | `w2-settings` |
| slash.diff | /diff - show git diff (including untracked files) | /diff | ❌ | `w2-transcript` |
| slash.mention | /mention - mention a file | /mention | ✅ | — |
| slash.status | /status - show current session configuration and token usage | /status — карточка статуса | 🟡 | `w2-status` |
| slash.daemon | /daemon - Manage the local background server | /daemon | ❌ | `w2-commands` |
| slash.warnings | /warnings - view retained warnings and diagnostic details | /warnings (и F2) | ✅ | — |
| slash.cd | /cd - change the current working directory | /cd <путь> | ✅ | — |
| slash.pwd | /pwd - show the current working directory | /pwd, /cwd | ✅ | — |
| slash.usage | /usage - view account usage or use a usage limit reset | /usage | 🟡 | `w2-status` |
| slash.debug-config | /debug-config - show config layers and requirement sources f | /debug-config | ❌ | `w2-commands` |
| slash.title | /title - configure which items appear in the terminal title | /title — терминальный title | 🟡 | `w2-status` |
| slash.statusline | /statusline - configure which items appear in the status lin | /statusline | 🟡 | `w2-status` |
| slash.theme | /theme - choose a syntax highlighting theme | /theme | ✅ | — |
| slash.pets | /pets - choose or hide the terminal pet | /pets, /pet | 🟡 | `pets-ambient` |
| slash.mcp | /mcp - list MCP tools; /mcp verbose or /mcp login <name> [-- | /mcp | ❌ | `w2-cells` |
| slash.apps | /apps - manage apps | /apps | ✅ | — |
| slash.plugins | /plugins - browse plugins | /plugins | 🟡 | `w2-approvals` |
| slash.logout | /logout - log out of Codex | /logout | ❌ | `w2-commands` |
| slash.quit | /quit - exit Codex | /quit, /exit | ✅ | — |
| slash.exit | /exit - exit Codex | /quit, /exit | ✅ | — |
| slash.feedback | /feedback - send logs to maintainers or save a ZIP | /feedback | 🟡 | `w2-commands` |
| slash.rollout | /rollout - print the rollout file path | /rollout | ✅ | — |
| slash.ps | /ps - list background terminals | /ps — фоновые терминалы | ❌ | `w2-cells` |
| slash.stop | /stop - stop all background terminals | /stop, /clean | ✅ | — |
| slash.clear | /clear - clear the terminal and start a new chat | /goal [цель/pause/resume/clear] | 🟡 | `w2-approvals` |
| slash.test-approval | /test-approval - test approval request | /test-approval | ✅ | — |
| slash.subagents | /subagents - switch between this session's subagents | /subagents (multi-agents picker) | ❌ | `w2-commands` |
| slash.debug-m-drop | /debug-m-drop - DO NOT USE | /debug-m-drop, /debug-m-update | 🟡 | `w2-commands` |
| slash.debug-m-update | /debug-m-update - DO NOT USE | /debug-m-drop, /debug-m-update | 🟡 | `w2-commands` |
| pets.ambient | Terminal pet (animated companion via kitty/sixel) | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| pets.asset-pack | Pet asset packs / catalog | Pets (диалог выбора) | 🟡 | `pets-ambient` |
| analytics.panels | Analytics sections | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `w2-status` |
| analytics.plot | Plot engine | Страница аналитики: бары, вкладки, часы плана, narrow-г | 🟡 | `w2-status` |
| other.tooltips | Startup tooltips / announcements | Turn tips и tooltips | 🟡 | `w2-transcript` |
| other.public-widgets | Public widgets (composer input) | Публичные виджеты и Renderable | ✅ | — |
| other.session-log | Session log | Session log | ❌ | `app-misc` |
| other.renderable-trait | Renderable trait + UI consts | Публичные виджеты и Renderable | ✅ | — |

## Покрытие модулей oracle

Все 1230 файлов `codex-rs/tui/src` (с тестами) — по строке в `docs/rtwk-port/reports/port-map-modules.tsv` (путь, статус, пакет, место в порте). Сводка:

| Пакет | Файлов oracle | ✅ | 🟡 | ❌ |
|---|---|---|---|---|
| `w2-live` | 287 | 0 | 287 | 0 |
| `startup-rest` | 125 | 0 | 125 | 0 |
| `w2-composer` | 114 | 0 | 112 | 2 |
| `w2-transcript` | 106 | 0 | 79 | 27 |
| `w2-status` | 105 | 0 | 105 | 0 |
| `w2-approvals` | 92 | 0 | 92 | 0 |
| `w2-cells` | 80 | 0 | 80 | 0 |
| `w2-settings` | 60 | 0 | 60 | 0 |
| — (заменено rtwk / готово) | 54 | 54 | 0 | 0 |
| `w2-commands` | 46 | 0 | 20 | 26 |
| `terminal-features` | 33 | 0 | 0 | 33 |
| `voice` | 31 | 0 | 0 | 31 |
| `agents-center` | 29 | 0 | 29 | 0 |
| `inline-mode` | 25 | 0 | 0 | 25 |
| `resume-rest` | 14 | 0 | 14 | 0 |
| `pets-ambient` | 13 | 0 | 13 | 0 |
| `ui-tree-viz` | 12 | 0 | 0 | 12 |
| `app-misc` | 2 | 0 | 0 | 2 |
| `sidebar` | 2 | 0 | 0 | 2 |

## Тесты: 935 pending по пакетам

Правило отнесения (`tests_rules.py`): тест, чья причина названа семенем/входом/именами кадров kit, — в `kit-seeds` (это обвязка, пока она не влита, функция не зазеленит тест); остальные — по имени теста, затем по причине, затем по шарду. Неточность правил — не пробел: пакет получает свой список, перенос теста между пакетами — правка одной строки в `unpend/`.

| Пакет | Pending | Шарды (число) |
|---|---|---|
| `w2-transcript` | 194 | transcript_view_4 57, transcript_view_2 34, transcript_view_5 30, transcript_view_1 24, transcript_view_3 12, pager 7 |
| `w2-composer` | 81 | textarea 31, composer_4 27, chat_commands_1 4, chat_commands_2 4, composer_1 3, composer_3 3 |
| `w2-settings` | 9 | chat_popups_4 5, chat_commands_3 2, chat_status_2 1, transcript_view_1 1 |
| `w2-cells` | 95 | cells_4 16, cells_5 14, cells_1 13, chat_approvals_1 10, chat_live_1 8, cells_2 6 |
| `w2-approvals` | 121 | small_dialogs_1 23, chat_approvals_2 17, approvals_1 16, chat_approvals_1 14, composer_popups 10, approvals_2 7 |
| `w2-commands` | 30 | chat_popups_4 7, chat_live_5 3, chat_status_3 3, small_dialogs_1 3, transcript_view_1 3, transcript_view_5 3 |
| `w2-status` | 148 | pane_footer_2 40, chat_status_2 27, chat_status_1 26, usage 16, pane_footer_1 9, chat_status_3 6 |
| `w2-live` | 152 | markdown_streaming_2 34, markdown_streaming_1 30, markdown_streaming_4 26, markdown_streaming_5 18, chat_live_1 17, chat_commands_3 8 |
| `voice` | 1 | composer_popups 1 |
| `inline-mode` | 12 | markdown_streaming_2 4, textarea 3, markdown_streaming_5 2, cells_4 1, chat_commands_3 1, composer_1 1 |
| `startup-rest` | 20 | chat_commands_3 8, chat_live_3 4, startup_prompts 3, signin 2, cells_1 1, cells_2 1 |
| `resume-rest` | 5 | resume_2 3, resume_1 2 |
| `kit-seeds` | 67 | chat_commands_1 13, transcript_view_5 8, chat_commands_3 7, cells_4 5, composer_4 5, picker 5 |

По тегам владельца шарда: chat-live 545, dialogs-pane 151, ui-pane 126, chat-nav-pane 80, routes-lists 27, startup-prompts 3, onboarding 2, startup-flow 1.

## Связь с прежней картой

Пакеты P1-P21 из `chat-feature-map.md` («Пакеты для портеров») заменены пакетами этого документа: ядровые половины P1-P19 — fw1-c1..c9 (влиты), UI-половины и sweep — восемь пакетов `w2-*` (первый план из 20 пакетов `ui-*` и `*-sweep` слит в них 2026-11-24, см. `research/w2-packages/briefs.md`); P20/P21 — framework-пакеты. Строки `chat-feature-map.md` с задачей «устарела» теперь имеют пакет здесь.
