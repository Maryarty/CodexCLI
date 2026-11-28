# Карта функциональности экрана чата: oracle (codex-rs/tui) против порта (codex-rs/tui-rtwk)

> Карта всего порта (весь codex-rs/tui, пакеты по владению файлами, pending-тесты по пакетам): [port-map.md](port-map.md). Пакеты P1-P21 ниже заменены ею.

Дата: 2026-11-24. Факт-финдинг, код не менялся. Строки перепроверены по коду `claude/tui-rtwk` на 1edc043bb (батчи 10, 11, 12 влиты: батч 10 — 88813c141, батч 11 — 6cf767e85, батч 12 — 1edc043bb; ADR-013 миграция закрыта: контейнер adr013-migrate) и по прогону oracle на 3342f9664 (то же дерево). До этого: батчи 3 (0130f3747), 4 (51abab645), 5 (de6ff75d3), 6 (60d7f4fa6), 7 (9e2d8ae75), 8 (3955761c0), 9 (ee6bad4e8). Батчи 10-12 архитектурные; поведение сменили: settings-slice (/theme, /statusline, /title, /memories открываются и пишут через `Ask::ConfigWrite`; host/settings.rs подключён), семьи select (/model, /permissions, /apps, /review, /worktree и подтверждения — отдельные sheet на вложенном Navigator), /approve, fork-ошибки по исходному треду, vim `/` и `!` через keymap. Числа «зелёные/всего» в строках остались с прежних пересчётов (не перебирались); новые числа — в разделе «Прогон oracle на 3342f9664» в конце. Новый раздел «Пакеты для портеров» — планирование батчей.

**Итог.** 291 строк-функций: ✅ 124, 🟡 76, ❌ 87, «?» 4 — на 1edc043bb (на ee6bad4e8: 291 строка, 123/69/95/4; изменения строк: /approve и «Auto-review denials popup» ❌→✅; /worktree, «Worktree/checkout picker», /title, /statusline, «Настройка статус-строки», «Settings: Config/Statusline/…» ❌→🟡 и /resume ✅→🟡; пересчитано скриптом по ячейкам статуса, «работает, тест не включён» считается по первому знаку). Тесты oracle на 3342f9664: 1085 зелёные, 1228 pending, 0 красных, 301 n/a из 2614 (на ee6bad4e8: 1075 / 1228 / 6 / 301 из 2610; +4 — шард `guards`, +6 красных исправлены). Пять дыр ниже: пункт 2 пересчитан, остальные числа — на ee6bad4e8. Миграция ADR-013 закрыта в батче 12: feature-блоки можно планировать (см. «Пакеты для портеров»). Заглушки порта (где команда отвечает «not yet», NotAvailable или пустым слотом) — в разделе «Заглушки порта».
**Пять самых больших дыр.**
1. Транскрипт: нет поиска (F3/F4), клавиатурного выделения, copy mode, закреплённой шапки prompt, раскрытия деталей хода. Копирование выделения мышью есть (542959815: отпускание, правая кнопка, Ctrl+C, Enter); нет Shift-клика/расширения с клавиатуры (rtwk-gap: у SelectionAreaController нет чтения start/end), Markdown-копии, X11 primary; клавиши ctrl+home / alt+< для старых страниц истории. transcript_view_4: 0 из 60 зелёные, transcript_view_5: 1 из 59.
2. Слэш-команды: ≈26 из ≈65 отвечают «NotAvailable», «not yet» или пустым слотом: /side /btw /copy /mcp /ps /recap /voice (bare) /diff /hooks /keymap /config /tui /experimental /skills /logout /ide /daybreak /debug-config /subagents /daemon /presets /setup-default-sandbox, а также аргументные формы `/model <имя>`, `/resume <имя>`, `/title <args>`, `/statusline <args>`; частично: /worktree, /new, /fork, /title, /statusline, /memories.
3. Composer: Vim включается (/vim, F9, `tui.vim_mode_default`; батч 5), remote-картинки идут через submit, историю и восстановление (батч 5); нет внешнего редактора Ctrl+G, состояний composer, очереди slash/shell-команд, мыши в composer; Vim в поле ответа на вопрос и режим Replace/поиск `/` в части тестов composer_4 не показаны; $-приложения появляются только после /apps. История между сессиями и живой @-поиск файлов со skills/plugins готовы (батч 3).
4. Безопасность и лимиты: safety buffering, guardian, misalignment, лимит-предупреждения, весь reset-flow /usage (chat_live_6: 0 из 25), карточка /status: usage треда и строка сервера.
5. Ячейки инструментов: группы exploration, unified exec и /ps, инвентарь MCP, решение approval и результат вопроса в транскрипте, compaction, строка «Forked thread» (cells_3/4, chat_approvals_1/2: 3 из 76 зелёные).

## Как читать

- Статус: ✅ работает; 🟡 частично (в «Чего нет» сказано чего); ❌ нет; «?» не удалось установить (причина в строке). «работает, тест не включён» — функция по коду есть, а её тесты в oracle-шардах pending по причине обвязки (probe-stale).
- Колонка тестов: зелёные/всего (n/a не считаются) среди тестов oracle, чьё имя совпало с шаблоном строки; «не вкл. N» — сколько из них stale. Совпадение по имени приблизительное: для широких строк (markdown, /status, футер) это весь семейный счёт. Прочерк — подходящих тестов не нашлось (у клавиш счёта нет: их проверяют функции выше).
- Задача — alias в tm на 1edc043bb. Миграция ADR-013 закрыта (батч 12), feature-блоки больше не приостановлены. Завершены и не владеют пробелом: chunk-popups-picker-rest, chunk-commands (сделан только /rename; команды NotAvailable остались без задачи), chunk-popups-apps-rest, chunk-cells-tool-cells, chunk-cells-question-result, chunk-cells-session-info, chunk-transcript-2 (мышь), chunk-transcript-2-turn-details (F4, шапка prompt остались как Needs rtwk). В ячейке — «задачи нет (X завершена)». «(устарела: …)» — описание или area называет то, чего нет после ADR-013 (Menu::*, `core/src/screens/settings*`, `src/sheets/settings`, `src/screens/*`); задачу надо пересоздать. Валидны по сути, но с устаревшими номерами строк: chunk-cells-background-terminals, chunk-live-safety, chunk-usage-reset, chunk-composer-states, chunk-commands-queue (ядро готово, осталась view-часть), chunk-transcript-2-find / -select-mode / -copy-mode, chunk-cells-mcp-inventory, chunk-status-card-limit-warnings. Статус остальных alias (chunk-status-card-thread-usage, chunk-status-card-server-row, chunk-status-voice, chunk-status-sparkle, chunk-popups-sweep, chunk-popups-windows-sandbox, chunk-commands-submission, chunk-composer-mention-seeds, usage, pets, sidebar, inline-mode, ui-tree, trust-on-open, backtrack) при этом обновлении в tm не перечитывался и остался как в прошлой карте. «—» — задачи нет или функция готова.
- Источники: slash_command.rs, research/codex-tui-features.csv, `core/src/commands/dispatch.rs`, `core/src/app/session.rs`, `src/pages/session/{actions,open,keys,session_page}.rs`, verdicts oracle-шардов (summary.py + probe), tests/pages и core/tests как свидетельства работы.
- Известные оговорки: красных тестов oracle на 3342f9664 нет (6 красных ee6bad4e8 исправлены debt-b9-session и поздними батчами); чего нет в тестах — pending по причине обвязки. Pty-сценарии при обновлении карты не гонялись: статусы settings (/theme, /memories) выведены по коду. Колонка тестов «зелёные/всего» не пересчитана.

## Транскрипт и ячейки

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Сообщение пользователя (блок с фоном, пути, картинки-плейсхолдеры) | ✅ | Фон блока prompt по теме (?) | 10/20 (не вкл. 2) | — |
| Ответ агента: стриминг markdown, списки, таблицы, код, ссылки | 🟡 работает, тест не включён | landed c4c2af4e0: настройки `[tui.rendering]` (таблицы, mermaid, списки) читаются там, где строится сообщение; при выключении источник сохраняется (5 ключевых тестов markdown_streaming_2/5 зелёные). Нет: math off только флаг (G-math-off, G61), маркер списка `•` вместо `- ` (G-list-marker), `user.rs`/reasoning/resume рисуют markdown без настройки, lists_preserve_layout (G73) и MATH-тесты (G61/G62) — sweep; часть markdown-рендера ждёт (счёт не пересчитан) | 126/268 (не вкл. 11; +5 после c4c2af4e0) | chunk-cells-rendering-prefs (сделана) |
| Подсветка кода и diff в ответах | ✅ | — | 14/55 (не вкл. 3) | — |
| Гиперссылки в тексте (клик открывает) | 🟡 | обычный клик по ссылке красный (ordinary_click_opens_links), OSC-8 в активной ячейке ждёт | 3/17 (не вкл. 1) | задачи нет (chunk-transcript-2 завершена) |
| Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | landed a15c35bf8: сводка live и в истории, raw reasoning по show_raw_agent_reasoning, пустые блоки (7 тестов зелёные) | 20/55 (не вкл. 3) | chunk-cells-reasoning |
| Ячейка exec: команда, вывод, статус, длительность | 🟡 | компактная форма как у oracle (afd840c6e; non-zero exit — bold-буллет); нет: пульс буллета running (G-tool-bullet-pulse), форма `!cmd` с 50 строками (G-exec-user-shell), scrollback-снимки ждут inline-режима S27 | 5/50 (не вкл. 2) | chunk-cells-rendering-prefs, inline-mode |
| Группы exploration (Read/List/Search свёрнуты в одну) | ❌ | группировка, прерывание, нулевой exit, порядок с reasoning (≈20 тестов) | 0/9 | задачи нет (chunk-cells-tool-cells завершена) |
| Unified exec: фоновые терминалы, wait-streak, interaction-ячейка | ❌ | ячейка взаимодействия, «waited for», сохранение при interrupt | 2/19 | chunk-cells-background-terminals |
| Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | пустой/failed-patch формат, ручное approval-заголовок (тесты ждут) | 11/27 (не вкл. 1) | chunk-cells-tool-cells |
| Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | в чате совпадает с oracle (pty, afd840c6e); снимки cells_4/5 — scrollback-форма, ждут inline-режима S27; пульс буллета ❌ | 2/20 | inline-mode |
| Инвентарь MCP (вывод /mcp) | ❌ | ячейка и данные инвентаря | 0/4 | chunk-cells-mcp-inventory |
| Ячейка web search (запрос, детали, усечение) | ✅ | строка в чате и обёрнутая форма в Ctrl+T (afd840c6e) | 5/5 | — |
| Ячейка просмотра/генерации изображений | 🟡 | image_generation_call_adds_history_cell и просмотр: рендер есть (cells/viewed_image, generated_image), снимки ждут | 3/4 | задачи нет (chunk-cells-tool-cells завершена) |
| Ячейка плана (update_plan) и предложенный план | ✅ | — | 5/9 | — |
| Ячейка collab/sub-agent (spawn, wait, send, модель/effort) | 🟡 | spawn_end с моделью, wait-элементы, повторный рендер sub-agent | 0/4 (не вкл. 1) | задачи нет (chunk-cells-tool-cells завершена) |
| Ячейка delegated (делегированный вывод tool) | ✅ | — | 2/2 | — |
| Ячейка hook (запущен/завершён) | ✅ | — | — | — |
| Решение approval в транскрипте (Approved/Denied + команда) | ❌ | decision cell: усечение многострочных команд, network-allow, canceled host | 0/5 (не вкл. 2) | задачи нет (chunk-cells-question-result завершена) |
| Результат вопроса request_user_input в транскрипте | ✅ | landed a15c35bf8: ячейка ответа/прерывания (completed_and_interrupted_results); markdown из истории — sweep | 0/4 | chunk-cells-question-result |
| Ячейка ошибки (upstream, cyber policy, overloaded, safety access) | ✅ | chained config error wrap (pending) | 3/9 | — |
| Предупреждения (warning, sqlite, config, guardian, model verification) | ✅ | wrap уведомления в narrow, notice_details_fit | 6/6 | — |
| Deprecation / update-available / version-notice | ✅ | update-available и server-version от host (e263606f0); server-version дедуп в пределах запуска (oracle хранит last_shown) | 4/6 | — |
| Часы/время в уведомлениях (clock facts) | 🟡 | Clock и LinkDisplay от host (host/src/viewer.rs, e263606f0); RuntimeMetrics без источника (G-runtime-metrics → host-счётчики, core-live-debts) | 1/4 (не вкл. 1) | core-live-debts |
| Session info: шапка сессии (модель, cwd, подсказки, tooltips) | ✅ | landed 87ad7733c: карточка шапки, каталог подсказок и выбор tip-строки, model-changed notes, стартовые факты от host | 2/13 (не вкл. 1) | chunk-cells-session-info |
| Строка «Forked thread» / имя треда в истории | ❌ | fork-history line с именем/id, prompt-edit history line | 0/5 | задачи нет (chunk-cells-session-info завершена) |
| Review-ячейки (баннер Current changes, request hint, prompt скрыт) | ✅ | — | 4/4 | — |
| Compaction: статус, таймер, ячейка завершения | ❌ | timer, snapshot restore, clear on turn end | 1/5 | chunk-live-safety |
| Final «Worked for …» разделитель с длительностью | 🟡 | cumulative duration snapshot ждёт; разделитель есть (cells/separator) | 0/1 | задачи нет (chunk-cells-tool-cells завершена) |
| Переключатель плотности (compact/detailed) ячеек | ✅ работает, тест не включён | — | 3/5 (не вкл. 1) | — |
| Сворачивание деталей хода (collapse_turn_details), раскрытие групп | 🟡 | сворачивание, сводки групп, раскрытие кликом/Enter/Space (16ed6526f); нет: ширинно-зависимый счёт скрытых строк exec, хранение 50+50 строк вывода, TurnTip (закреплённая шапка prompt: отдельная строка ниже) | 4/10 | задачи нет (chunk-transcript-2 завершена) |
| Фокус на activity-группах (F4) и подсказка раскрытия | ❌ | focus_activity целиком | 0/1 | задачи нет (chunk-transcript-2-turn-details завершена; Need G-new-usage-2 `ListController::visible_range()`) |
| Автопрокрутка за хвостом, «вернуться вниз», строка новой активности | ✅ | — | 10/30 (не вкл. 5) | — |
| PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | Home/End в ленте; скорость колеса (mouse_scroll_speed) не применяется | 4/18 | задачи нет (chunk-transcript-2 завершена) |
| Подгрузка старых страниц истории при прокрутке вверх | 🟡 | landed ee4c83bd9: статус-строка (loading / refused с retry / loadable, 4 ширины) из `transcript/history.rs`, join страницы не двигает читателя, 9 page-тестов, 4 oracle-теста зелёные; pty: тот же финальный экран. Нет: клавиши ctrl+home / alt+< (идти к началу, retry) и ctrl+end / alt+> (Need: session_page.rs); склейка групп oracle при join (page_join_* в transcript_view_3, sweep); join при читателе в 1-3 строках ниже первой сдвигает вид на строку; один тик колеса двигает много больше строк, чем у oracle | 6/27 (не вкл. 3; +4 после ee4c83bd9) | chunk-transcript-2-history-pages (сделана) |
| Закреплённая шапка prompt над транскриптом (pinned header) | 🟡 | owned transcript: `transcript/prompt_header.rs` (b924bdd53, 50841b968, fix round 2), `ViewportCx` + `ListParentData` + `RenderList::item_place` вместо `visible_range()`; смена видимого хода, скрытие при видимом prompt и высоте < 4, резерв строки, клик по шапке (только простое нажатие: заканчивает выделение, Find, copy mode, фокус F4, затем прыжок), 10 page-тестов `pages::transcript_prompt_header`, oracle `prompt_header_tracks_reading_and_yields_to_visible_prompts_and_short_screens`. Нет: обычный (не owned) transcript; 5 oracle-тестов `transcript_view_3` и 2 `transcript_view_2` pending по реальным причинам (голый вид oracle без composer; шапка не рисуется, пока первый видимый элемент — сам prompt; сдвиг цели copy mode под резервом) | 1/3 | s10-reader-prompt-header (в review) |
| Replay истории при resume/fork: сообщения, tool, reasoning, review | ✅ | resumed_initial_messages_render_history, nested review prompts — ждут | 12/24 | — |
| Ambient pet в транскрипте/футере | 🟡 работает, тест не включён | pets sheet есть; ambient-тесты chat_status_1 не включены | 2/13 (не вкл. 4) | pets |
| Фоновая «starfield»/sparkle эффекты композера | 🟡 | задача занята; 12 тестов не зелёные | 0/12 (не вкл. 3) | chunk-status-sparkle |

## Живой ход (стриминг, прерывание, очередь, steer)

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Отправка сообщения Enter, индикатор working/«Thinking», таймер хода | ✅ | — | 2/6 | — |
| Прерывание хода Esc | ✅ работает, тест не включён | Esc с ожидающими steer: отправка всех сразу и сохранение (тест ждёт) | 11/39 (не вкл. 5) | — |
| Ctrl+C: прерывает ход, второй раз выходит; «armed quit» подсказка | ✅ | Ctrl+C при caps lock (n/a) | 7/11 | — |
| Ctrl+D на пустом composer выходит (когда idle) | ✅ | — | 0/1 | — |
| Очередь сообщений (Tab / Enter в ходе) над composer | ✅ | — | 2/12 (не вкл. 1) | — |
| Очередь slash- и shell-команд во время хода | ❌ | ядро готово (`core/src/threads/queue.rs`: `Queued` Message/Literal/Command/Shell, `DueCommand`, `QueueRan`/`QueueResume`); нет вида: Tab ставит строку команды, запуск при drain, !shell в очереди, отклонение при MCP startup | 4/6 (не вкл. 1) | chunk-commands-queue (в сути валидна: ядро сделано, осталась view-часть) |
| Правка очереди (Alt+Up, вернуть в composer) | ✅ | — | 0/2 (не вкл. 1) | — |
| Steer: Enter во время хода отправляет уточнение | 🟡 работает, тест не включён | pending steers: восстановление при interrupt ок; ordering/mention-bindings/failed steer не включены | 13/23 (не вкл. 7) | chunk-commands-submission |
| Предпросмотр pending steers над composer | ✅ | — | 9/15 (не вкл. 4) | — |
| Отправка во время review/plan stream (очередь вместо steer) | ✅ | — | 1/3 | — |
| Блокировка отправки при недоступной модели | ❌ | submit_user_message_blocks_when_thread_model_is_unavailable | 0/1 | chunk-commands-submission |
| Подтверждение Esc/Enter при диалогах поверх composer | ✅ | — | — | — |
| Баннер бэкенда (нумерованные строки, CTA, отклонение по occurrence) | ✅ | landed c4fff1a2b: авто-переключение fallback и `{time}`, F6 (обрыв соединения отменяет запросы в полёте), 2 ключевых теста chat_live_2 зелёные; в pty баннер не гонялся (в mock нет endpoint). Нет (sweep): три теста восстановления (нужен on_rate_limit_error из chunk-status) и снимок backend_banner_notification; has_pending_recovery не портирован, fallback только при чтении аккаунта, notice_model не сбрасывается при смене треда | 6/13 (+2 после c4fff1a2b) | chunk-live-banner-fallback (сделана) |
| Резерв Luna: селектор и статус | 🟡 | селектор работает; модель возврата резерва живёт только в памяти (нужен Ask::ConfigWrite); backend_auth заменяет has_chatgpt_account; патч переключения = модель + effort; статус по активной модели, переживание закрытия баннера — ждут | 1/4 (не вкл. 1) | chunk-live-banner-fallback (сделана) |
| Safety buffering: «повторить», подтверждение, короткое сообщение | ❌ | весь блок safety_buffering (9 тестов) | 0/7 | chunk-live-safety (валидна; файлов `core/src/threads/safety.rs` ещё нет) |
| Экран ошибки провайдера (сохраняет черновик, выход в Resume) | ✅ | сохранение между потоками, provider_id sync — ждут | 5/10 | — |
| Read-only внешний writer: заметка вместо composer, переход в Agents | ✅ | — | 4/6 | — |
| Уведомления терминала о завершении хода / approval | 🟡 | настройки notify_when; async-question уведомления n/a | 7/13 (не вкл. 1) | — |
| Stream recovery: восстановление статуса после обрыва | ✅ | — | 2/2 | — |
| MCP startup: статус, lag, отмена, отказ как warning | ✅ | mcp_startup_failure_renders_warning_history, booting snapshot — ждут | 15/20 (не вкл. 1) | — |
| Rate-limit подсказки/предупреждения во время хода | ❌ | limit warnings (90%/95%), prompt по лимиту | 0/1 | chunk-status-card-limit-warnings |
| Misalignment/guardian stop треда | ❌ | misalignment_* (3), guardian parallel reviews, aggregate status | 1/14 | chunk-live-safety |
| Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись родителя | ❌ | нет вообще: Ctrl+/ даёт `Notice::NotAvailable(Side)` (session_page.rs:683), /side и /btw — NotAvailable (core/src/app/session.rs); тесты отказов в side-треде тоже ждут | 1/10 | задачи нет (chunk-commands завершена) |
| Режим Plan: popup «Implement this plan?», scope рассуждений | ✅ | plan_mode reasoning-scope popup (тесты n/a), reasoning override metka | 9/12 | chunk-popups-plan |
| Shift+Tab переключение режима (collab mode) только в idle | ✅ | — | — | — |
| Бюджет цели: сообщения budget_limited / interrupted | ✅ | — | 2/3 | — |
| Replay незавершённого хода помечает task running | ✅ | — | 2/2 | — |

## Composer (ввод, история, вставка, картинки, @/$ упоминания, Vim)

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Многострочный ввод, Shift+Enter, курсор, wrap, прокрутка по высоте | ✅ | — | 18/58 (не вкл. 1) | — |
| Редактирование: kill ring (Ctrl+K/U/W/Y), Alt+D, слова, undo | ✅ | часть textarea-тестов ждёт (39) | 18/58 (не вкл. 1) | — |
| Placeholder-подсказка пустого composer | ✅ | — | 13/20 (не вкл. 2) | — |
| Slash-popup: фильтр, стрелки, Tab/Enter, описание, скрытие недоступных | ✅ | — | 22/47 (не вкл. 1) | — |
| Недоступная команда во время хода: «disabled» текст, черновик цел | ✅ | disabled_slash_command_while_task_running_snapshot ждёт | 0/1 | — |
| Команды с inline-аргументами (/plan x, /goal x, /review x, /rename x) | 🟡 | /model x, /permissions-profile без args; submission-цепочка не вся; после батчей 10-11: `/permissions x` ✅ (`PermissionsArgs`); `/model x`, `/title x`, `/statusline x`, `/resume x`, `/review` без args — NotAvailable или не все (см. раздел «Заглушки») | 2/8 (не вкл. 3) | chunk-commands-submission |
| Вставка: paste burst, placeholder большой вставки [Pasted Content N] | ✅ | — | 16/28 (не вкл. 1) | — |
| Вставка пути к файлу превращается в элемент/картинку | 🟡 работает, тест не включён | путь→картинка есть, hidden shell paste очередь/восстановление не включены | 5/15 (не вкл. 5) | chunk-composer-images |
| Скрытая shell-вставка с /plan (hidden shell paste) | 🟡 работает, тест не включён | работает, тест не включён (4 из 5 stale), 1 красный: submits_literal | 4/14 (не вкл. 5) | chunk-commands-submission |
| История Up/Down в сессии | ✅ | — | 5/14 (не вкл. 1) | — |
| История между сессиями (persistent history) | ✅ | landed 2054483f3: Up после перезапуска вспоминает последнее сообщение (pty: rtwk = oracle), slash-команды записываются как у oracle (`/di` из popup → `/diff`), история засевается из replay resume (3 oracle-теста зелёные, 2 page-теста). Нет (sweep, ждут картинок и Vim): восстановление картинок и vim-навигация по истории (4 теста composer_3); история текстовая; `/diff` до pager закрывается Esc | 1/1 (+3 после 2054483f3) | chunk-composer-history (сделана) |
| Поиск по истории Ctrl+R / Ctrl+S | ✅ | — | 16/29 (не вкл. 3) | — |
| Восстановление очищенного Ctrl+C prompt из истории | ✅ | — | 1/1 | — |
| @-упоминания: popup, вставка, Tab, множественные | ✅ | landed 2054483f3: живой file search (через in-process поиск хоста, `Ev::FileMatches`; app-server fuzzyFileSearch не портирован, не нужен) и server skills/plugins в списке, строки как у oracle (pty: `@sr` совпадает построчно); n/a при MentionsV2 off (решение 2026-11-23): 4 теста composer_1. Нет (sweep): adjacent_image_completion_inserts_leading_separator (ждёт картинок и Vim) | 7/34 (не вкл. 5; +6 ключевых после 2054483f3) | chunk-composer-live-mentions (сделана) |
| Семена mention (при Esc/восстановление steer возвращают биндинги) | ✅ | биндинги доходят до отправки и восстанавливаются (mention-seeds, влит) | 0/3 (не вкл. 1) | — |
| $ skills / плагины / apps в упоминаниях | 🟡 | landed 2054483f3: каталог skills (`skills/list`) и plugins (`plugin/installed`) грузится раз на тред, когда известен cwd; установленные apps берутся из кэша /apps (pty: `$sk` совпадает построчно). Нет: кэш apps загружается только при открытии /apps, поэтому `$`-приложения появляются после /apps, oracle грузит коннекторы при старте (Need владельцу select/apps); предупреждения о ошибках загрузки skill (invalid SKILL.md) не портированы; legacy MentionsV2-off — n/a (решение 2026-11-23) | 10/24 (не вкл. 1) | chunk-composer-live-mentions (сделана; Need select/apps) |
| Картинки: локальные [Image #N], нумерация, удаление | ✅ | — | 7/9 (не вкл. 1) | — |
| Картинки из буфера Ctrl+V / Alt+V | 🟡 | поток есть: Ctrl+V/Alt+V → `Intent::PasteImage` → `Ask::ClipboardImage`, ответ в `src/services/effects.rs` (`paste_image`) → `Ev::ImagePasted`; в тестах ответ хоста подаёт kit (`clipboard_image`, батч 5: снята причина pending у теста с картинкой из буфера после удержанного `!`, chat_commands_1). Нет: прогона с настоящим буфером в бинаре; `adjacent_image_completion_inserts_leading_separator` (composer_3) ждёт (sweep) | 0/1 | chunk-composer-images (сделана) |
| Remote-картинки (URL): submit и нумерация | ✅ | landed cc0bbabd5 (батч 5): `UserMessage.remote_images` идут в turn_input перед локальными, через merge_messages, `Draft::message` и put_back; Enter и Tab на одних remote-картинках отправляют; вызванная из истории запись снова рисует remote-строки; выделение, режущее элемент, расширяется до него; kit-семена `Open::remote_images` и `clipboard_image`. Тесты с этими семенами включены в chat_commands_1, composer_2, composer_3, composer_4. Нет (sweep): тела composer_4 без семян Vim/remote-картинок (удаление выбранной remote-строки как одна правка Vim, часть снимков) | 2/9 (не пересчитано; не вкл. 3 до батча 5) | chunk-composer-images (сделана) |
| Состояния composer: disabled, shutdown, parent-owned, agents-nav hint | ❌ | нет понятия состояний; read-only есть частично | 2/6 (не вкл. 1) | chunk-composer-states |
| Bang-shell режим: !команда, Enter в ходе → run_user_shell | ✅ | — | 7/14 | — |
| Vim: нормальный/вставка/операторы/текст-объекты/поиск (движок) | 🟡 работает, тест не включён | landed fdadb68ff (батч 5): `/vim` включает и выключает с заметкой (`NoteKey::Toggled`), F9 (`global.toggle_vim_mode`) тоже, `tui.vim_mode_default` начинает черновик нового треда в Insert (TuiSettings, config host), подпись футера следует режиму (страница перестраивается, когда клавиша меняет режим), `!` в Normal входит в Insert; vim-тесты composer_1, composer_3, chat_live_4, chat_commands_1 включены. Нет (sweep): composer_3 `left_respects_editor_and_vim_history_remaps` (нет keymap-семени), тела composer_4 без Vim-семян (поиск `/`, Ctrl+R на пустой истории, redo, Replace); Vim в поле ответа на вопрос (см. «Вопросы») | 8/51 (не пересчитано; не вкл. 5 до батча 5) | chunk-composer-vim (сделана) |
| Vim: replace/yank/dot-repeat | 🟡 работает, тест не включён | dot-повтор теперь записывает удержанный или сброшенный paste-burst символ (fdadb68ff); тела composer_4 ждут (Vim Replace, kit не включает Vim в этих телах) | 1/7 (не пересчитано) | chunk-composer-vim (сделана) |
| Внешний редактор Ctrl+G | ❌ | нет ни обработчика, ни запуска $EDITOR | 0/1 | chunk-composer-states |
| Мышь в composer (курсор, выделение, вставка правой кнопкой) | ❌ | right_click_paste и мышь не выполняются | 1/4 (не вкл. 2) | chunk-composer-states |
| Startup draft (черновик из аргументов, file-search startup) | 🟡 работает, тест не включён | работает, тест не включён | 0/13 (не вкл. 5) | chunk-composer-states |
| Подсказки «? shortcuts» / Esc-Esc / «Ctrl+C to quit» в футере composer | ✅ | — | 3/6 | — |
| Элементы composer: [Image], [Pasted], mention — атомарное удаление | ✅ | — | 5/10 | — |
| Расширение (collapse) длинной вставки при submit | ✅ | — | 4/4 | — |
| Отправка с пустым / только пробелы / только плейсхолдер | ✅ | — | 13/28 (не вкл. 1) | — |
| Тема-цвет курсора и подсказки; narrow-терминал | ✅ | — | — | — |

## Слэш-команды (каждая — своя строка; порядок как в slash_command.rs)

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| /model — выбор модели и effort | 🟡 | bare ✅ (OpenModels), landed 87ad7733c (chunk-popups-sweep): 13 тестов chat_popups_4 на живом пикере, курсор по форме страницы, скрытые модели; /model <имя> NotAvailable (provider-write не портирован); advanced effort, refresh каталога не включены; батчи 10-11: лист models на вложенном Navigator (advanced-страница, скрытые модели); `/model <имя>` — `Intent::Model{..}` → NotAvailable (app/session.rs) | 12/27 | задачи нет (chunk-popups-picker-rest завершена) |
| /presets — выбор пресета (Daybreak presets) | ❌ | лист quick-dials (`Opening::Quick`, `select/quick_dials.rs`) и `select::presets` есть, debug-вход есть; страница `Intent::Quick` не шлёт: `SlashCommand::Presets` показывает попап «not built» (actions.rs, b11-fixes) | 1/1 | chunk-popups-quick-dials (устарела: Menu::Quick нет, остался открыватель) |
| /daybreak | ❌ | NotAvailable; отказ Daybreak | 0/2 | задачи нет (chunk-commands завершена) |
| /ide — контекст IDE | ❌ | NotAvailable | 0/4 | задачи нет (chunk-commands завершена) |
| /permissions — approvals/профили | 🟡 | bare ✅ (OpenPermissions), args через PermissionsArgs ✅; подтверждение full-access, Windows, history cell не все | 8/21 | chunk-popups-permission-args |
| /keymap — редактор клавиш | ❌ | `Which::Keymap` → `return` (core/src/features/settings/mod.rs), пустой слот «settings (S15)» без вида; ленты keymap-настроек нет | 4/12 (не вкл. 1) | settings (устарела: area до ADR-013) |
| /vim — переключить Vim composer | ✅ | fdadb68ff (батч 5): включает и выключает черновик, пишет заметку; сброс режима в Insert после submit (composer_1 `vim_mode_resets_to_insert_after_submission` включён); pty-прогона в этом пересмотре нет | 0/1 (не пересчитано) | chunk-composer-vim (сделана) |
| /setup-default-sandbox (Windows) | ❌ | ElevateSandbox NotAvailable; только Windows | 1/9 | chunk-popups-windows-sandbox |
| /experimental — feature-флаги | ❌ | `Which::Experimental` → `return`; sheet `src/features/settings/features.rs` есть, нет открывателя и ленты experimentalFeature/list | 3/7 (не вкл. 1) | settings (устарела: area до ADR-013) |
| /approve — Auto-review | ✅ | открывает sheet denials (`sheets::approvals::open_auto_review`), без denials — заметка; снимки ждут | 3/6 | — |
| /memories | 🟡 | `host/src/settings.rs` подключён (`settings_fact` в in_process.rs, батч 10): /memories открывается, когда host выдал факты memory tool (`facts.memories`), иначе молча ничего; reset-confirmation ✅ как UI-состояние. Pty при обновлении карты не перегонялся | 2/3 | settings (устарела: area до ADR-013) |
| /skills | ❌ | `Which::Skills` → `return`; sheet `src/features/settings/setup.rs` есть, нет открывателя и ленты skills/list; manage_skills_summary pending | 0/1 | settings (устарела: area до ADR-013) |
| /import — перенос из других агентов | ✅ | route Import; shard import 16/16 | 16/16 | — |
| /hooks | ❌ | NotAvailable (`S::Hooks` в core/src/app/session.rs); sheet `src/features/settings/hooks.rs` есть, нет ленты hooks/list; hooks_popup_shows_list_diagnostics | 0/1 | settings (устарела: area до ADR-013) |
| /review — выбор ветки/коммита/custom | ✅ | branch/commit picker snapshots ждут (review_*_picker, 2 pending) | 32/74 (не вкл. 6) | — |
| /rename — имя треда | ✅ | landed a15c35bf8 (95826e191): с аргументом и bare prompt (slice features/rename); батч 9 (features-trait): prompt — UI-половина `src/features/rename/prompt.rs` на общем виде `features/prompt.rs` (Menu/Pick/Prompt у Feature убраны), pty rename-prompt в land-check батча; 29abc1aa5: вид prompt `src/sheets/dialogs/prompt.rs`, чат показывает любой открытый моделью prompt (так же файловый prompt /export и edit /goal), 5 тестов chat_commands_2 зелёные, pty rename-prompt проходит в обоих бинарях. Нет: предложение имени — хост отвечает Done::Title(None), oracle генерирует через ephemeral thread (thread_title.rs); нужен backend SuggestTitle в services/rpc | 1/7 (+5 chat_commands_2 после 29abc1aa5) | small-dialogs |
| /new | 🟡 | новый тред ✅; выбор checkout (`Intent::OpenCheckout`) — модель в core есть, приложение его не шлёт (select.rs:706) | 0/2 | chunk-popups-worktree (устарела: Menu::Worktree нет) |
| /archive | ✅ | confirm sheet + RPC | 2/2 | — |
| /delete | ✅ | confirm sheet + RPC | 1/1 | — |
| /resume | 🟡 | route Resume ✅ (resume_1/2: 48 из 72 зелёные); `/resume <имя>` (`ResumeNamed`) — NotAvailable | 48/61 (не вкл. 4) | задачи нет |
| /fork | 🟡 | новый тред ✅; ошибка fork привязана к исходному треду (b11-fixes); пикер checkout (`OpenCheckout` не шлётся) и строка «Forked thread» ❌ | 4/11 | chunk-popups-worktree (устарела: Menu::Worktree нет) |
| /worktree | 🟡 | sheet открывается (`S::Worktree` → `OpenWorktrees`); список не грузится: у `Ask::Worktrees` и `Ask::WorktreeRemove` нет исполнителя (services/effects.rs → None, host/src/effects.rs не обслуживает); Continue/Start-цепочка ждёт `OpenCheckout` и старт | 0/5 | chunk-popups-worktree (устарела: Menu::Worktree нет) |
| /app — открыть в desktop-приложении | ✅ | OpenUrl codex://threads/<id> | — | — |
| /init | ✅ | отправляет prompt AGENTS.md | 1/1 | — |
| /compact | 🟡 | команда ✅; статус/таймер compaction не включены | 9/24 (не вкл. 2) | chunk-live-safety |
| /config | ❌ | `Which::Config` → пустой слот «settings (S15)» (`Shown::Other`), вида нет; то же для `/tui` | — | settings (устарела: area до ADR-013) |
| /recap | ❌ | NotAvailable; ячейка recap | 0/5 | задачи нет (chunk-commands завершена) |
| /plan [prompt] | ✅ | маска из каталога, уведомления disabled/unavailable | 1/8 (не вкл. 2) | chunk-popups-plan |
| /voice | ❌ | bare /voice, mute, stop → NotAvailable (`VoiceControl`); F8 и fallback-ветка open.rs → пустой sheet voice | 0/5 | voice (устарела: src/screens/*) |
| /goal [цель/pause/resume/clear] | 🟡 работает, тест не включён | команда и модель (features/goal) ✅; батч 9 (features-trait): у цели своя UI-половина `src/features/goal/` (sheet.rs: меню цели и edit prompt, общий вид prompt `features/prompt.rs`), `Target::Feature`; снимки меню и edit prompt не включены | 1/20 (не вкл. 4) | chunk-popups-goal (описание устарело: остались снимки и sweep) |
| /agents — центр агентов | ✅ | Target::Agents route | 0/1 | — |
| /side, /btw | ❌ | нет; тесты отказов в side-треде тоже ждут | 4/33 (не вкл. 2) | задачи нет (chunk-commands завершена) |
| /copy — копировать ответ | ❌ | NotAvailable(Copy); copy picker и Ctrl+O не готовы; в чате есть только копия выделения мышью | 0/11 (не вкл. 2) | chunk-transcript-2-copy-mode |
| /export | ✅ | landed a15c35bf8 (95826e191): путь, picker назначения, prompt имени (slice features/export); батч 9: UI-половина `src/features/export/` (sheet.rs), `Target::Feature` | 0/2 | chunk-commands-export |
| /raw | ✅ | режим + заметка, Alt+R | 19/35 (не вкл. 1) | — |
| /tui — быстрые настройки TUI | ❌ | `Which::Config` → пустой слот (dispatch_bare шлёт Tui в `Target::Settings`; вида нет) | — | settings (устарела: area до ADR-013) |
| /diff | ❌ | `Target::Pager` показывает «not yet» (open.rs). Готово: core `DiffStep` (screens/transcript/diff.rs), `git_diff` + `WorkspaceExec` (src/services/shell.rs), `Ask::GitDiff` → `Done::GitDiff` (services/effects.rs, host-тесты). Нет: ответ → документ pager, проверка stale cwd, открыватель | — | pager (устарела: area, строить как slice) |
| /mention | ✅ | вставляет «@» | 1/1 | — |
| /status — карточка статуса | 🟡 работает, тест не включён | карточка ✅ — ячейка транскрипта треда, как у oracle (status-card-adr003); landed e1795b779 (батч 3): живая карточка из фактов треда, config, аккаунта, каталога моделей и удалённого подключения; лимиты с обновлением по `RateLimitOrigin::Status(request)`, строки Model (display name, reasoning, summaries), Collaboration mode, Agents.md; pty: строки равны oracle. Нет: Thread name (oracle делает title-запрос, порт нет), версия в заголовке `(v0.0.0)` вместо `(v0.170.0)` (версия сборки); thread usage, server row, limit warnings ❌ (дети); лента фактов статус-строки (Cwd/Clock/SideLabel/AgentLabel, git-пробы) не построена | 42/168 (не вкл. 14, не пересчитано; +5 chat_status_2 после e1795b779) | chunk-status-card (сделана; sweep 6 тестов chat_status_2) |
| /daemon | ❌ | `Target::Select` (остаток /daemon) показывает «not yet» (`daemon_action`); модель `select::daemon` в core есть | 1/2 | задачи нет (chunk-popups-picker-rest завершена) |
| /warnings (и F2) | ✅ | — | 2/8 | — |
| /cd <путь> | ✅ | с отказами | 0/1 | — |
| /pwd, /cwd | ✅ | — | 16/16 | — |
| /usage | 🟡 работает, тест не включён | нужен backend auth, иначе заметка UsageNeedsLogin; страница есть (43 теста не включены); меню reset ❌ | 14/81 (не вкл. 44) | usage |
| /debug-config | ❌ | NotAvailable | — | задачи нет (chunk-commands завершена) |
| /title — терминальный title | 🟡 | bare /title открывается и сохраняет через `Ask::ConfigWrite` (`items::title`, батч 10); `/title <args>` — NotAvailable | 2/9 | settings (устарела: area до ADR-013) |
| /statusline | 🟡 | bare /statusline открывается и сохраняет (`items::status_line`); `/statusline <args>` — NotAvailable | 2/33 (не вкл. 1) | settings (устарела: area до ADR-013) |
| /theme | ✅ | settings Theme; 790cc44a6 (батч 6): без темы в config пикер открывается на теме терминала по умолчанию (строка «(current)» и курсор), превью по центру под строкой отступа панели — строки пикера равны live oracle (pty settings-theme). Нет: подсказка футера (у нас `Press enter to confirm or esc to go back`, у live oracle `enter select · esc back`; снимки oracle-шардов совпадают со стандартной); лента фактов host подключена (`settings_fact`, батч 10), темы из config и `{codex_home}/themes` должны быть в бинаре (pty не перегонялся); клик по барьеру не возвращает тему превью (как было) | 3/8 (не пересчитано) | settings |
| /pets, /pet | 🟡 работает, тест не включён | sheet pets открывается; ambient-часть и тесты не включены | 2/13 (не вкл. 4) | pets |
| /mcp | ❌ | NotAvailable; ячейка инвентаря | 0/4 | chunk-cells-mcp-inventory |
| /apps | ✅ | диалог ссылки приложения, вкл/выкл, сброс при смене аккаунта, обновление каталога (b2eeacb7c); partial-snapshot тесты n/a (нет is_final:false в протоколе) | 9/10 | — |
| /plugins | 🟡 | влит 8c492bcbc: список, установка, marketplaces. Пилот ADR-013 (батч 8, 454b13f11): Esc как у oracle (закрывает на карточке, loading и failure; возвращает только на подтверждении удаления и prompt add-marketplace), `Back to plugins` — строка карточки (курсор, вкладка и запрос сохраняются), setup-ссылка через `Ask::OpenUrl`, страницы — маршруты вложенного Navigator; pty рядом с oracle: браузер, вкладки, поиск, карточка, add-marketplace равны. Различия: усечение `…` после пробела (общий TextOverflow::Ellipsis пикера), переход prompt → failure add-marketplace не гонялся; красный на tip chat_popups_2 `plugins_popup_preserves…` (debt-b9-session) | 39/41 (chat_popups_3: 48 pass, 0 red на пилоте) | chunk-popups-plugins (сделана) |
| /logout | ❌ | NotAvailable | — | задачи нет (chunk-commands завершена) |
| /quit, /exit | ✅ | — | 7/16 (не вкл. 2) | — |
| /feedback | 🟡 | батч 9 (features-trait, d776d1a43): `/feedback` открывает страницу Feedback листа dialogs (категория, consent) — раньше открывал пустой лист; заметка и загрузка логов не написаны; upload-consent/diagnostics снимки ждут (27 тестов) | 3/30 (не вкл. 1) | chunk-popups-feedback (устарела: R15_NOT_YET, AppCtx::Dialogs) |
| /rollout | ✅ | — | 2/2 | — |
| /ps — фоновые терминалы | ❌ | NotAvailable | 1/1 | chunk-cells-background-terminals |
| /stop, /clean | ✅ | чистит терминалы + заметка | 1/1 | — |
| /clear | ✅ | новый тред/экран | 1/2 | — |
| /test-approval | ✅ | Target::Approvals | — | — |
| /subagents (multi-agents picker) | ❌ | `Target::AgentPicker` показывает «not yet» (`action_subagents` в open.rs); `app/multi_agents.rs` не портирован | 0/3 | задачи нет (chunk-commands завершена) |
| /debug-m-drop, /debug-m-update | 🟡 | заглушка-заметка `NoteKey::Stub(MemoryMaintenance)` (debug) | 1/1 | задачи нет (chunk-commands завершена) |

## Попапы и селекторы

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Список выбора: поиск, стрелки, цифры, disabled-строки, hint по биндингам | ✅ | — | 44/58 | — |
| Model picker: каталог, discovery, bedrock/custom display, service tier | 🟡 | landed 87ad7733c: скрытые модели, seed провайдера, лимит потоков агента; осталось: refresh каталога, reasoning submenu (extra high, advanced), снимки | 13/24 | задачи нет (chunk-popups-picker-rest завершена) |
| Reasoning picker и Alt+,/. , Shift+Up/Down | ✅ | reach_max (красный), ultra, advanced-only ждут | 2/3 | chunk-popups-picker-rest |
| Permissions popup: пресеты, профили, retry discovery | 🟡 | full access confirm, Windows sandbox prompts, history cell решения | 9/40 | chunk-popups-permission-args |
| Windows sandbox: prompts, help-ссылки, fallback | ❌ | весь блок (≈10 тестов); только Windows | 1/9 | chunk-popups-windows-sandbox |
| Goal: меню и edit prompt, resume-paused prompt | 🟡 работает, тест не включён | меню и edit prompt — лист фичи `features/goal/sheet.rs` (батч 9); снимки не включены; resume-paused работает, тест не включён | 0/8 (не вкл. 2) | chunk-popups-goal (описание устарело: остались снимки и sweep) |
| Plan: popup реализации и scope reasoning | ✅ | — | 9/12 | chunk-popups-plan |
| Collaboration / mode catalog | 🟡 работает, тест не включён | каталог недоступен (тест не включён) | 0/1 (не вкл. 1) | chunk-popups-plan |
| Worktree/checkout picker | 🟡 | лист worktree (`select/worktree.rs`, `Opening::Worktree`) открывается из /worktree; нет исполнителя `Ask::Worktrees`/`WorktreeRemove` и шага старта; `OpenCheckout` не шлётся (/new, /fork) | 0/5 | chunk-popups-worktree (устарела: Menu::Worktree нет) |
| Quick dials (saved dials: save/clear picker) | ❌ | лист `Opening::Quick` и debug-вход есть, входа из чата нет (`Intent::Quick` не шлётся, /presets) | 0/1 | chunk-popups-quick-dials (устарела: Opening::Quick, остался открыватель) |
| Plugins: список, вкладки, поиск, деталь, install, marketplaces | 🟡 | влит 8c492bcbc и пилот ADR-013 (см. строку /plugins); осталось: усечение после пробела, add-marketplace failure не гонялся, красный plugins_popup_preserves… | 39/44 | chunk-popups-plugins (сделана) |
| Apps/connectors: список, статус, install-only | 🟡 | простые состояния ✅; partial refresh ❌ | 3/9 | задачи нет (chunk-popups-apps-rest завершена) |
| Hooks popup: список, диагностика | ❌ | sheet `features/settings/hooks.rs` есть; `Which::Hooks` → `return`, ленты hooks/list нет | 0/1 | settings (устарела: area до ADR-013) |
| Experimental features popup (toggle сохраняется на выходе) | ❌ | sheet `features/settings/features.rs` есть; `Which::Experimental` → `return`, ленты experimentalFeature/list нет (toggle сохраняется на выходе — не проверено) | 3/7 (не вкл. 1) | settings (устарела: area до ADR-013) |
| Feedback: выбор, consent upload, диагностика, note | 🟡 | страница Feedback открывается из `/feedback` (батч 9); note и логи нет; снимки ждут | 3/30 (не вкл. 1) | chunk-popups-feedback (устарела: R15_NOT_YET, AppCtx::Dialogs) |
| Review picker (ветка/коммит/custom) | ✅ | — | 1/1 | — |
| Copy/export picker (кастомные клавиши, payload) | ❌ | export-назначение есть (`features/export`, см. /export); `/copy` picker и payload нет | 0/1 | chunk-commands-export |
| Reset-limit picker и подтверждение (usage menu) | ❌ | ≈30 тестов chat_live_6 | 0/11 | chunk-usage-reset |
| Security setup / security prompts | ❌ | security_setup_* (≥4) | 0/5 | chunk-popups-sweep |
| Memories settings и reset | 🟡 | экран и reset готовы; открывается, когда host выдал факты memory tool (см. /memories) | 2/3 | settings (устарела: area до ADR-013) |
| Manage skills summary | ❌ | sheet `features/settings/setup.rs` есть; `Which::Skills` → `return`, ленты skills/list нет | 0/1 | settings (устарела: area до ADR-013) |
| Current settings (model/permissions/status layout) | ❌ | current_settings_offer_* | 0/1 | settings (устарела: area до ADR-013) |
| Sweep: прочие мелкие popup'ы | ? | состав «sweep» не установлен по коду; задача created | — | chunk-popups-sweep |

## Approvals и вопросы

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Approval exec: модал, варианты, execpolicy-префикс, причина | ✅ | multiline-prefix скрытие execpolicy ждёт | 3/11 (не вкл. 2) | — |
| Approval apply_patch: модал, call_id, заголовок | 🟡 | untrusted модал, manual flow, patch snapshot ждут | 0/4 | задачи нет (chunk-cells-question-result завершена) |
| Approval network host (разово/сессия) | 🟡 работает, тест не включён | history decision cell ждёт; запрос ✅ | 0/4 (не вкл. 2) | задачи нет (chunk-cells-question-result завершена) |
| Approval write_stdin | ❌ | render terminal input | 0/3 | задачи нет (chunk-cells-question-result завершена) |
| Permissions request approval, MCP elicitation форма | ✅ | — | 0/8 | — |
| Guardian review (approved/denied/timed out, aggregate status) | ❌ | 0 из 11 | 1/11 | chunk-live-safety |
| Auto-review denials popup | ✅ | sheet denials (`open_auto_review`); снимок ждёт | 0/1 | — |
| request_user_input: вопросы с вариантами, цифры/буквы | ✅ | батч 7 (b3ae82940): клавиши диалогов вопросов через keymap (`keys.rs`), `Draft.selected` → `choice`; поведение то же | 9/16 | — |
| Вопросы: черновики, навигация, replay черновиков | ❌ | question_drafts_survive_*, turn_end drafts (≈10) | 0/8 | задачи нет (chunk-cells-question-result завершена) |
| Вопросы: поиск по истории, Vim в поле ответа | ❌ | history search; Vim в поле ответа не подключён (approvals_1, chat_approvals_2 ждут, батч 5 их не трогал) | 0/2 | chunk-composer-vim |
| Асинхронные вопросы (desktop answer, уведомления) | 🟡 | уведомления n/a; многое n/a — другой клиент | 0/2 | — |
| Отклонение пустых ответов, явный skip | ❌ | — | 0/1 | задачи нет (chunk-cells-question-result завершена) |
| Разворот вопроса уступает мышь терминалу для ссылок | ❌ | — | 0/1 | задачи нет (chunk-cells-question-result завершена) |
| Очередь вопросов и queued messages делят один shortcut | ❌ | — | 0/2 | задачи нет (chunk-cells-question-result завершена) |
| Keymap-seeds для approval | ❌ | — | 3/5 | chunk-composer-mention-seeds |

## Футер и статус-строка

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режимы | ✅ | — | 26/118 (не вкл. 13) | — |
| Подсказка очереди «Alt+Up edit» | ✅ | — | — | — |
| Строка статуса: модель, effort, cwd, context %, токены | ✅ | git-ветка, PR со ссылкой, изменения ветки, headline, raw output (ea69879b9); нет: Cwd/Clock-факты и пробы git/headline от host, заголовок треда в строке | 2/33 (не вкл. 1) | chunk-status-card |
| Настройка статус-строки (/statusline, порядок элементов) | 🟡 | bare /statusline открывает и сохраняет (`Ask::ConfigWrite`); аргументы NotAvailable | — | settings (устарела: area до ADR-013) |
| Терминальный заголовок (title) с элементами | 🟡 | рендер ✅; настройка bare /title ✅ (`/title <args>` NotAvailable) | 2/9 | settings (устарела: area до ADR-013) |
| Индикаторы режимов: Plan, goal, side, agent, Vim | ✅ | рисуются (ea69879b9); подпись Vim следует режиму (fdadb68ff); метки side/agent ждут фактов от host | 3/9 (не вкл. 1) | — |
| Индикатор голоса (voice strip), F8 | ❌ | — | 0/5 | chunk-status-voice |
| Индикатор эффекта effort ignition | ✅ | — | 1/2 | — |
| Sparkle/«искры» над composer | 🟡 | задача занята | 0/12 (не вкл. 3) | chunk-status-sparkle |
| Счётчик предупреждений (F2) в футере | ✅ | — | 2/9 | — |
| Flash в футере и временный override подсказки | 🟡 | таймер flash и override есть (ea69879b9); на странице нет источника вспышки | 1/4 | chunk-composer-states |
| Строка follow («вернуться вниз») над composer | ✅ | — | 23/56 (не вкл. 5) | — |
| Карточка /status: основное | ✅ | ячейка транскрипта (ADR-003), e1795b779: Model с reasoning и summaries, Collaboration mode, Permissions, Agents.md, Session, Token usage, Context window, Limits; нет Thread name (title не генерируется) | 24/27 | chunk-status-card (сделана) |
| Карточка /status: usage треда | ❌ | — | 1/6 (не вкл. 1) | chunk-status-card-thread-usage |
| Карточка /status: строка сервера | ❌ | — | — | chunk-status-card-server-row |
| Предупреждения о лимитах в карточке и ленте | ❌ | — | — | chunk-status-card-limit-warnings |
| Подпись контекста side-разговора | ❌ | — | 0/4 | задачи нет (chunk-commands завершена) |
| Подсказка reset-лимита на старте | ❌ | — | 0/2 | chunk-usage-reset |

## Транскрипт-оверлей / pager / backtrack / выделение-копирование-поиск

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Ctrl+T: подробный транскрипт (sheet) с reasoning и warnings | ✅ | — | 14/53 (не вкл. 1) | — |
| Esc-Esc: backtrack — выбор прошлого prompt, Enter откат | ✅ | — | 2/4 | backtrack |
| Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | ?: часть тестов backtrack всего 4 | 2/4 | backtrack |
| Статический pager (/diff, вывод): строки, страницы, Home/End, q, проценты | 🟡 | есть как режим страницы (`StaticPager`, core/src/screens/transcript.rs); /diff «not yet» (см. /diff), live-tail sync и поиск ❌ | 5/18 | pager (устарела: area, строить как slice) |
| Pager: поиск /, n/N | ❌ | — | 0/1 | pager (устарела: area, строить как slice) |
| Pager: live tail sync при росте транскрипта | ❌ | — | — | pager (устарела: area, строить как slice) |
| Мышью выделить текст в транскрипте (подсветка) | ✅ | SelectionArea рисует выделение | 4/64 (не вкл. 1) | chunk-transcript-2 |
| Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | landed 542959815: копия по отпусканию, Ctrl+C / Cmd+C / Ctrl+Shift+C, Enter = копия и вниз, Esc снимает, подсказка в футере, строка «Copied N chars to host clipboard», выделение снимается после подтверждённой копии (pty text-100: рядом с oracle совпадает); сервис копирования clipboard-service (5026ad24c). Расхождения: конец диапазона у oracle до ячейки под курсором, у rtwk SelectionArea включает её («Copied 12 chars» вместо 11, rtwk gap); OSC 52 при копии по отпусканию пишется дважды (clipboard-provider в rtwk). Sweep: X11 primary, deselect сравнивает текст, не диапазон, oracle-снимки на 3-4 строках (kit::pending) | 0/11 (не вкл. 1; transcript_view_1 14/36/1 на 542959815) | задачи нет (chunk-transcript-2 завершена) |
| Двойной/тройной клик: слово/строка/абзац | ❌ | по паку это жесты rtwk SelectionArea; тестов и проверки в бинаре у порта нет | 0/1 | задачи нет (chunk-transcript-2 завершена) |
| Shift-клик расширяет, правая кнопка копирует | 🟡 | правая кнопка копирует (Listener в SelectionHost, 542959815, тест в selection_copy.rs); Shift-клик и расширение с клавиатуры не построить: у SelectionAreaController есть `select(target, item, start, end)`, но нет чтения start/end (rtwk gap) | 1/5 (не вкл. 1) | задачи нет (chunk-transcript-2 завершена) |
| Копия как Markdown/plain (payload) | ❌ | HTML-вариант Markdown-копии в сервисе готов (5026ad24c); выбора формата в UI нет; копия выделения мышью только plain text (CopyFormat::MarkdownSelection ждёт тексты элементов); transcript_view_3 в основном n/a | 1/1 | chunk-transcript-2-copy-mode |
| Режим выделения с клавиатуры (select mode) | ❌ | нет модуля | 0/6 | chunk-transcript-2-select-mode |
| Режим копирования (copy mode, Ctrl+X-лидер) | ❌ | нет модуля | — | chunk-transcript-2-copy-mode |
| Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | F3 только открывает sheet | 5/39 | chunk-transcript-2-find |
| Подсказки поиска и футер поиска | ❌ | — | 1/1 | chunk-transcript-2-find |
| Скорость колеса (mouse_scroll_speed) | ❌ | настройка есть, не применяется | 1/2 | задачи нет (chunk-transcript-2 завершена) |
| Старые страницы в overlay (history pages) | 🟡 | ee4c83bd9: статус-строка sheet транскрипта берёт спеллинги из `transcript/history.rs` (`sheet::status`); нет ctrl+home/alt+< и склейки групп (см. строку выше) | — | chunk-transcript-2-history-pages (сделана) |
| Сайдбар (панель агентов/тредов) | ❌ | ADR-закрытая задача sidebar | 0/1 | sidebar |
| Inline-режим (без alt-screen) | ❌ | задача created | — | inline-mode |
| Дерево UI (ui-tree) | ? | не относится к ленте; задача created | — | ui-tree |
| Клик по ссылке/файлу в overlay | 🟡 | обычный клик красный | 0/1 | задачи нет (chunk-transcript-2 завершена) |

## Диалоги

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Предупреждения (/warnings, F2): страницы, wrap | 🟡 | warning_pages_wrap красный | 0/2 | small-dialogs (устарела: src/screens/*) |
| Шорткаты: оверлей «?» со всеми действиями | 🟡 | ab78ae687 (батч 6): страница шорткатов — состояние sheet на границе ADR-013, F2 и копирование через keymap; gap: из чата оверлей достижим только из debug-меню | 2/4 | small-dialogs (устарела: src/screens/*) |
| Rename thread prompt (bare /rename) | ✅ | slice features/rename (a15c35bf8), вид prompt 29abc1aa5; с батча 9 prompt — UI-половина фичи (`features/rename/prompt.rs`); нет предложения имени (Done::Title(None), нужен SuggestTitle; gap из hand-in батча 9: предложение имени не в поле ввода — нужно покрытие pty/oracle) | — | small-dialogs |
| Confirm: archive/delete треда | ✅ | — | 12/19 | — |
| Daemon (подключение к daemon) | ❌ | попап «not yet» (см. /daemon) | 1/2 | small-dialogs (устарела: src/screens/*) |
| Ссылка на приложение (app link) диалог | ❌ | — | — | small-dialogs (устарела: src/screens/*) |
| Custom review prompt | ✅ | — | 0/1 | — |
| Верификация пользователя (user verification) | ❌ | — | 1/3 | small-dialogs (устарела: src/screens/*) |
| Settings: Theme, Memories (из чата) | 🟡 | Theme ✅ (лента фактов host подключена, батч 10); Memories открывается только при фактах memory tool (`facts.memories`); строки Theme и настройки принимают тап (`GestureDetector`) | 5/11 (не вкл. 1) | settings (устарела: area до ADR-013) |
| Settings: Config / Statusline / Title / Experimental / Skills / Hooks / Keymap | 🟡 | Statusline и Title ✅ (bare; см. строки команд); Config и Keymap — пустой слот без вида; Experimental, Skills, Hooks — sheet есть (features.rs, setup.rs, hooks.rs), открыватель `Which::*` → `return`, ленты host нет. `Fact::Offered` приходит только из seed/тестов/debug-меню, host его не шлёт | 7/19 (не вкл. 1) | settings (устарела: area до ADR-013) |
| Feedback (диалог) | 🟡 | ab78ae687: шаг feedback — состояние sheet; с батча 9 из чата достижим командой `/feedback` (страница Feedback); снимки consent/diagnostics ждут | 3/30 (не вкл. 1) | chunk-popups-feedback (устарела: R15_NOT_YET, AppCtx::Dialogs) |
| Pets (диалог выбора) | 🟡 | ab78ae687: курсор и превью — состояние sheet; глаголы LoadPreview, ReleasePreview; ambient-часть и тесты не включены | 2/13 (не вкл. 4) | pets |
| Voice (диалог, F8) | ❌ | — | 0/5 | voice (устарела: src/screens/*) |
| Trust on open (запрос доверия к каталогу) | ? | в startup-шарде 29/32 зелёные; из чата: /cd да (f-startup-trust); остальные вызовы (resume, новая задача из центра, смена backend, fresh) — f-startup-trust-callers | 10/11 | trust-on-open |
| Model provider switch (блокирует кэш выбора) | ❌ | provider_switch_* | 0/1 | задачи нет (chunk-popups-picker-rest завершена) |
| Debug menu F12 | ✅ | — | 0/2 | — |
| Список выбора (SelectionList): тап по строке | 🟡 | gap (2f68c2c75): тап по строке только двигает курсор, как у rtwk-виджета; выбор строки — клавишей | — | small-dialogs (устарела: src/screens/*) |
| Resume picker: клавиши oracle (ctrl-p/n/k/j/h/l/b/f), ctrl+e на пустом id | ✅ | landed 656a4d753 (батч 6, adr013-resume): список и курсор — через keymap (`LIST_*`), плотность и открытая строка — состояние страницы, дозагрузка — один verb `LoadMore`; ctrl+e на пустом id игнорируется как у oracle. На ee6bad4e8 resume_2 `page_and_jump_navigation` больше не красный (resume_1 + resume_2: 49 зелёные, 12 pending, 0 red) | — | — |

## /usage

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Страница аналитики: бары, вкладки, часы плана, narrow-графики | 🟡 работает, тест не включён | рендер работает, 43 теста не включены; батч 7 (usage-agents, f1675bfa8, ecdfcbed1): usage live/Leave, группы не в Pos, нет whole-state watch, диалог ошибки по привязанным ключам, помощь закрывается, когда секция не меняется; поведение то же | 3/46 (не вкл. 43) | usage |
| Меню /usage: Analytics, Refresh, Reset | 🟡 | в чате 0 из 25: пункты меню и reset не подключены | 0/2 | chunk-usage-reset |
| Reset лимита: пикер, подтверждение, idempotency-ключ, in-flight | ❌ | — | 0/10 | chunk-usage-reset |
| Billing и enterprise-планы | ❌ | — | 0/4 | chunk-usage-reset |
| Подсказка reset на старте | ❌ | — | 0/1 | chunk-usage-reset |
| Статус после billing retries | ❌ | — | 0/2 | chunk-status-card-limit-warnings |
| Ранний redraw и повтор thread usage | ❌ | n/a в harness | — | chunk-status-card-thread-usage |
| Нужен вход (UsageNeedsLogin) | ✅ | — | — | — |

## Горячие клавиши (дефолты oracle)

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Ctrl+T — подробный транскрипт | ✅ | — | — | — |
| Esc — прервать ход | ✅ | — | — | — |
| Esc Esc (пустой composer) — backtrack | ✅ | — | — | backtrack |
| Ctrl+C — прервать/выход | ✅ | — | — | — |
| Ctrl+D — выход на пустом | ✅ | — | — | — |
| Up/Down — история | ✅ | — | — | — |
| Ctrl+R / Ctrl+S — поиск по истории | ✅ | — | — | — |
| Shift+Tab — режим | ✅ | — | — | — |
| Tab — в очередь (сообщения) | 🟡 | команды и shell в очереди ❌ | — | chunk-commands-queue |
| Alt+Up — вернуть из очереди | ✅ | — | — | — |
| Alt+, / Alt+. — reasoning | ✅ | — | — | — |
| Shift+Up/Down — reasoning | ✅ | — | — | — |
| Alt+R — raw | ✅ | — | — | — |
| F2 — warnings | ✅ | — | — | — |
| F3 — поиск | 🟡 | открывает sheet без поиска | — | chunk-transcript-2-find |
| F4 — focus activity | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | — | задачи нет (chunk-transcript-2-turn-details завершена) |
| F8 — голос | 🟡 | открывает пустой sheet | — | chunk-status-voice |
| F12 — debug menu | ✅ | — | — | — |
| Ctrl+/ (Ctrl+_, Ctrl+7) — side | ❌ | `Notice::NotAvailable(Side)` (session_page.rs:683) | — | задачи нет (chunk-commands завершена) |
| Ctrl+O — копировать ответ | ❌ | в keymap/catalogue.rs есть, обработчика нет | — | chunk-transcript-2-copy-mode |
| Ctrl+G — внешний редактор | ❌ | в keymap/catalogue.rs есть, обработчика и запуска $EDITOR нет | — | chunk-composer-states |
| Ctrl+V / Alt+V — вставка картинки | 🟡 | `Ask::ClipboardImage` обслуживается (см. строку composer); с настоящим буфером в бинаре не гонялось | — | chunk-composer-images (сделана) |
| Ctrl+L — очистить терминал | ❌ | в keymap/catalogue.rs есть, обработчика в session_page нет | — | задачи нет (chunk-commands завершена) |
| Ctrl+Z — suspend | ? | в keymap зарезервирован; обработчик не найден | — | — |
| Ctrl+X — leader (Ctrl+X b — смена бэкенда) | 🟡 | leader живёт только для баннера | — | chunk-live-banner-fallback |
| ? — оверлей шорткатов | ✅ | — | — | — |
| Left на пустом composer — Agents | ✅ | — | — | — |
| Enter / Shift+Enter — отправка / перенос | ✅ | — | — | — |
| PgUp/PgDn, колесо — прокрутка ленты | ✅ | — | — | — |
| Vim-клавиши (69 действий) — normal/operator/text-object/search | ✅ | включаются /vim и F9 (батч 5); sweep: см. строки Vim в Composer; keys-5 (батч 12): `/` поиск и `!` идут через keymap (SHELL_PREFIX, SLASH_COMPLETE); KeyPress-строк не осталось | 8/51 (не пересчитано) | chunk-composer-vim (сделана) |

## Заглушки порта: где что отвечает «not yet», NotAvailable или пустым слотом

Индекс найденных по коду мест; в счёт строк не входит. У каждой заглушки есть строка в таблицах выше (в скобках).

| Место | Что видит пользователь | Строка карты |
|---|---|---|
| `src/pages/session/open.rs` `Target::Select` (остаток /daemon, `daemon_action`) | попап «not yet» | /daemon, «Daemon (подключение)» |
| `open.rs` `Target::AgentPicker` (`action_subagents`) | попап «not yet» | /subagents |
| `open.rs` `Target::Pager` (`action_transcript`) | попап «not yet» | /diff |
| `open.rs` `Target::Settings`, `Target::Feature` (пустые `{}`: открывает фича) и `Which::Config|Skills|Keymap|Experimental|Hooks` → `return` в `core/src/features/settings/mod.rs` | строка «settings (S15): empty slot» или ничего | /config, /tui, /keymap, /experimental, /skills, «Settings: Config/…» |
| `open.rs` fallback `_ => voice::open` | пустой sheet voice | /voice, F8 |
| `src/pages/session/actions.rs` `SlashCommand::Presets` (`presets_action`) | попап «not built» | /presets, «Quick dials» |
| `src/pages/session/session_page.rs:683` `Action::ToggleSide` | `Notice::NotAvailable(Side)` | Ctrl+/ |
| `core/src/app/session.rs` арм NotAvailable (~365-385): Recap, Side, Copy, Tui (bare идёт в Target::Settings), Mcp, Apps (недостижим), Logout, Ide, Daybreak, ElevateSandbox, Hooks, DebugConfig, Ps; `Review{args: None}` и `Permissions{args: None}` (недостижимы) | заметка NotAvailable | /recap, /side, /copy, /mcp, /logout, /ide, /daybreak, /setup-default-sandbox, /hooks, /debug-config, /ps |
| тот же арм: `Model{..}`, `Title{..}`, `Statusline{..}`, `VoiceControl(_)`, `ResumeNamed{..}` | NotAvailable для аргументных форм | /model, /title, /statusline, /voice, /resume |
| `S::MemoryMaintenance` → `NoteKey::Stub(Stub::MemoryMaintenance)` | заглушка-заметка | /debug-m-drop, /debug-m-update |
| `Intent::OpenCheckout` никем не шлётся (`core/src/screens/select.rs:706`) | /new и /fork без выбора checkout | /new, /fork, «Worktree/checkout picker» |
| `Intent::Quick` никем не шлётся | лист Quick недостижим из чата | /presets, «Quick dials» |
| `Ask::Worktrees`, `Ask::WorktreeRemove` → `return None` в `src/services/effects.rs`, host не обслуживает | список worktree не загружается | /worktree |
| `Fact::Offered(Offer)` приходит только из seed, тестов, debug-меню (в `host/src` нет `Offer::`) | offer-диалоги settings из host не появляются | «Current settings» |
| `catalogue.rs`: Ctrl+O, Ctrl+G, Ctrl+L, F4 без обработчика в `session_page.rs`; F3 только `modes.open()` | клавиша ничего не делает | Ctrl+O, Ctrl+G, Ctrl+L, F4, F3 |

## Строки «работает, тест не включён»

Функция есть по коду и по probe, а тесты oracle pending по причине обвязки; включить их — задача harness, а не порта.

- Транскрипт и ячейки: Ответ агента: стриминг markdown, списки, таблицы, код, ссылки (chunk-cells-rendering-prefs)
- Транскрипт и ячейки: Ячейка reasoning (сводка, сырой reasoning, заголовок) (chunk-cells-reasoning)
- Транскрипт и ячейки: Переключатель плотности (compact/detailed) ячеек (—)
- Транскрипт и ячейки: Ambient pet в транскрипте/футере (pets)
- Живой ход: Прерывание хода Esc (—)
- Живой ход: Steer: Enter во время хода отправляет уточнение (chunk-commands-submission)
- Composer: Вставка пути к файлу превращается в элемент/картинку (chunk-composer-images)
- Composer: Скрытая shell-вставка с /plan (hidden shell paste) (chunk-commands-submission)
- Composer: Vim: нормальный/вставка/операторы/текст-объекты/поиск (движок) (chunk-composer-vim)
- Composer: Vim: replace/yank/dot-repeat (chunk-composer-vim)
- Composer: Startup draft (черновик из аргументов, file-search startup) (chunk-composer-states)
- Слэш-команды: /goal [цель/pause/resume/clear] (chunk-popups-goal)
- Слэш-команды: /status — карточка статуса (chunk-status-card)
- Слэш-команды: /usage (usage)
- Слэш-команды: /pets, /pet (pets)
- Попапы и селекторы: Goal: меню и edit prompt, resume-paused prompt (chunk-popups-goal)
- Попапы и селекторы: Collaboration / mode catalog (chunk-popups-plan)
- Approvals и вопросы: Approval network host (разово/сессия) (chunk-cells-question-result)
- /usage: Страница аналитики: бары, вкладки, часы плана, narrow-графики (usage)

## Архитектура ADR-013: что изменили батчи 7-12

Строки-функции выше этой секции не считаются: она описывает архитектурные шаги, у которых нет собственной функции. Граница ADR-013: в ядре (core) живёт только то, что нужно IO — факты сервера и host, записи запросов и статус операций по ключу домена, шаги жизненного цикла и одноразовые сигналы; всё нарисованное (курсор, выделение, прокрутка, вкладка, стек страниц листа, раскрытие, плотность, текст поля до отправки) — состояние UI. Ядро принимает глаголы, не жесты. Миграция закрыта в батче 12; правила проверяет `tests/rules` (`adr013.rs`; `adr013_ratchets.txt` сведён к двум строкам `exempt`, файлов по семьям нет).

| Семья | Батч (коммит) | Что изменилось | Поведение |
|---|---|---|---|
| usage-agents | 7 (f1675bfa8, ecdfcbed1) | usage live/Leave, нулевые группы в `Pos` страницы, нет whole-state watch, диалог ошибки по привязанным ключам; Pick/Step перестраивают страницу, чтобы помощь закрывалась при той же секции; ratchet-файл удалён | то же |
| approvals | 7 (b3ae82940) | диалоги вопросов читают клавиши через keymap (`keys.rs`), `Draft.selected` → `choice`; ratchet-файл удалён | то же |
| composer | 8 (9932d915e) | `core::data`, kit не называет экраны, 32 именованных fixed keymap action; новые правила на нуле | то же |
| пилот /plugins | 8 (454b13f11, bded969d6) | typed keys, `RpcError`, без registry, `Ask::OpenUrl`; страницы — маршруты вложенного Navigator (O1); шаблон фичи `core/src/features/<f>` + `src/features/<f>` | Esc как у oracle по страницам; setup-ссылка через `Ask::OpenUrl` |
| session | 9 (94cbf1660) | без ticket и whole-state watch, restore читается в одном месте, один слушатель directives, клавиши через keymap; session.txt удалён | то же |
| features-trait | 9 (998f16cd4) | `Feature` без Menu/Pick/Prompt; /goal, /export, /rename получают UI-половину (`src/features/<f>/`, общий `features/prompt.rs`), `Target::Feature` + `OpenArgs`; features.txt удалён | `/feedback` открывает страницу Feedback (раньше пустой лист); rename prompt и goal/export-листы — через UI-половину фичи |
| unit-tests-2 | 9 (c53a6b031) | 28 модулей unit-тестов перенесены в корни скриптом, 35 оставлены (приватность) | нет |
| select-shared, пять семей select (worktree, quick, apps, models, permissions), select-finish | 10-11 (88813c141, 6cf767e85) | /model, /permissions, /apps, /review, /worktree и подтверждения — отдельные sheet (вложенный Navigator, `Opening::*`); `Menu` и стек удалены | /worktree открывает sheet; /approve ✅; `OpenCheckout` и `Quick` не шлются |
| settings-slice | 10 (b23c744cd, c27133684, 46bca2df9) | `core/src/features/settings/`, `src/features/settings/`; `core/src/screens/settings.rs` и `src/sheets/settings` удалены; host/settings.rs подключён (`settings_fact`, `config_write_job`) | /theme, /statusline, /title, /memories открываются и пишут через `Ask::ConfigWrite`; Config/Skills/Keymap/Experimental/Hooks — `return` |
| debt-b9-session, debt-b9-features | 10-11 | 6 красных oracle-тестов исправлены; plan-mode free check, место debug-меню, continue при ошибке провайдера | красных нет |
| b11-fixes | 11 | формы провайдера принимают только plain-клавиши; ошибка fork привязана к исходному треду (`Ctx::Fork{thread}`, `fork_failed`); попап /presets «not built»; Esc в многостраничных sheet — по страницам как у oracle | fork-ошибка |
| adr013-keys, keys-4, keys-5 | 11-12 | последние `KeyPress`-строки убраны (`keypress.txt` удалён); vim `/` и `!` идут через keymap (SHELL_PREFIX, SLASH_COMPLETE) | vim `/` поиск и `!` |
| oracle-guard-red, oracle-guard-debt, b12-fixes | 12 (1edc043bb) | шард `guards` (4 теста), шарды black-box через kit, `summary.py` считает падения guard красными; vim-слэш требует `/` И клавишу поиска; правило ADR-006 расширено | нет |

## Прогон oracle на 3342f9664 (батч 12; то же дерево, что 1edc043bb)

Полный прогон 24 ноября (shards.py, junit профиля oracle, `summary.py`): **1085 pass / 301 n/a / 1228 pending / 0 todo / 0 red / 0 stale из 2614** (на ee6bad4e8: 1075 / 301 / 1228 / 6 red из 2610; +4 — шард `guards`, остальной прирост +6 — шесть бывших красных стали зелёными). Pending по слайсам владельца: chat-live 627, chat-nav-pane 103, dialogs-pane 248, onboarding 2, routes-lists 55, startup-flow 1, startup-prompts 3, ui-pane 189. Разбор причин pending (не построено / не хватает кита / пробел rtwk / ждёт keymap) в этом обновлении не пересчитывался: число pending не изменилось (1228), на ee6bad4e8 было 622 / 358 / 195 / 53.

| Группа шардов | pass | n/a | pending | red | всего |
|---|---|---|---|---|---|
| Подтверждения (approvals_1/2, chat_approvals_1/2) | 53 | 54 | 120 | 0 | 227 |
| Пикеры и попапы (picker, chat_popups_1-4) | 148 | 25 | 76 | 0 | 249 |
| Настройки и мелкие диалоги (small_dialogs_1/2) | 34 | 0 | 58 | 0 | 92 |
| Слэш-команды (chat_commands_1-3) | 38 | 38 | 85 | 0 | 161 |
| Композер (composer_1-4, composer_popups, textarea) | 200 | 15 | 116 | 0 | 331 |
| Resume (resume_1/2) | 49 | 14 | 12 | 0 | 75 |
| Pager и транскрипт (pager, transcript_view_1-5) | 32 | 60 | 203 | 0 | 295 |
| Ячейки и Markdown (cells_1-5, markdown_streaming_1-5, renderer_internals) | 259 | 21 | 210 | 0 | 490 |
| Статус и футер (chat_status_1-3, pane_footer_1/2, status) | 103 | 26 | 183 | 0 | 312 |
| Живой ход (chat_live_1-6) | 91 | 37 | 116 | 0 | 244 |
| StartupPage (signin, onboarding, startup_prompts, import) | 71 | 11 | 6 | 0 | 88 |
| Usage (usage) | 3 | 0 | 43 | 0 | 46 |
| guards (guards) | 4 | 0 | 0 | 0 | 4 |
| Всего | 1085 | 301 | 1228 | 0 | 2614 |

## Пакеты для портеров

Каждый пакет — две половины: core (`core/src/features/<f>/` или `core/src/…`, core-тесты, host-исполнители, строки в общих core-файлах) и UI (`src/features/<f>/`, `src/pages/…`, виды). Core-половины идут первой широкой волной, UI-половины — второй. Размер половины — один прогон порта (≈150-250 ходов); оценки «строки» — грубо, кода без тестов; «строк карты» — сколько строк-функций половина закрывает. ADR-013: всё нарисованное — UI-состояние, поэтому у чисто визуальных пакетов core-половины нет. Задача tm в скобках — владелец сегодня; «новая» — задачи нет, нужна новая.

Общие файлы без реестра (на них пакеты сталкиваются) названы в каждом пакете, после пакетов собраны в таблицу столкновений. Регистрация фичи (`feature_list!` в `core/src/features/mod.rs`) конфликта не создаёт.

Принятые сокращения путей: `app/session.rs` = `core/src/app/session.rs` (арм NotAvailable, ~365-385); `dispatch.rs` = `core/src/commands/dispatch.rs`; `open.rs`, `actions.rs`, `session_page.rs`, `keys.rs` = `src/pages/session/…`; `catalogue.rs` = `core/src/keymap/catalogue.rs`; `host effects` = `host/src/effects.rs` + `src/services/effects.rs`.

### Волны

Core-половины первой волны: P1, P2, P3, P4, P5, P6, P9, P10, P11, P12, P13, P14, P15, P16, P17, P18, P19 (P21 заблокирован). Только UI (без core-половины): P7, P8, P20. Пары с общим файлом идут подряд или одним прогоном (таблица в конце).

### P1. pager-diff: /diff и документ pager
- Строки (3): /diff ❌, «Статический pager» 🟡, «Pager: live tail sync» ❌ (поиск pager — в P5).
- Oracle: `pager_overlay.rs`, `pager_overlay/{scrolling,transcript}.rs`, `get_git_diff.rs`, `diff_model.rs`, `diff_render.rs`.
- Тесты: diff 1 по имени; pager-шард и transcript_view_1 (≈10 pending, не разбирались).
- Core (≈350 строк, 2-3 строки карты, 1 прогон): `core/src/screens/transcript/diff.rs` (reply `Done::GitDiff` → документ, проверка stale cwd — машина `DiffStep` уже есть), `core/src/screens/transcript.rs`, verb `Intent::Diff` в `dispatch.rs` и `app/session.rs` вместо `Target::Pager`; core-тесты рядом.
- UI (≈600 строк, 1 прогон): `open.rs` (`Target::Pager` вместо `not_yet`), `src/pages/session/transcript/pager.rs` (`StaticPager`: загрузка документа, live-tail sync), порт `diff_render.rs` в новый `src/pages/session/transcript/diff_view.rs`.
- Общие: `open.rs`, `app/session.rs`, `dispatch.rs`, `transcript/mod.rs`.

### P2. composer-queue-states: Ctrl+G, состояния composer, мышь, очередь команд
- Строки (6): «Внешний редактор Ctrl+G» ❌, «Состояния composer» ❌, «Мышь в composer» ❌, «Очередь slash- и shell-команд» ❌, «Tab — в очередь» 🟡, Ctrl+G ❌ (клавиша). «Startup draft» — только тесты.
- Oracle: `external_editor.rs`, `chatwidget/input_queue.rs`, `input_flow.rs`, `session_queue_commands.rs`, `chatwidget/command_lifecycle.rs` (часть), состояния в `bottom_pane/chat_composer*.rs`.
- Тесты: queue 37 (chat_commands_2 8, chat_commands_1 7, chat_approvals_2 4), ext editor 7, composer-состояния 8 (composer_3 3), startup-draft 27.
- Core (≈450 строк, 4 строки, 1 прогон): `Ask::ExternalEditor`/`Done::ExternalEditor` в `core/src/app/effects.rs` + исполнитель в host effects (запуск $EDITOR, чтение файла); verb `Intent::OpenExternalEditor`; шаги Tab-очереди (ядро `core/src/threads/queue.rs` готово: `Queued`, `DueCommand`, `QueueRan/QueueResume`; нужен verb «поставить команду в очередь» и отказ при MCP startup в `core/src/threads/chat.rs`); факты состояний composer (read-only, parent-owned, shutdown) в `core/src/threads.rs`.
- UI (≈800 строк, 1 прогон; если больше 200 ходов — вынести мышь в отдельный прогон): новый `src/pages/session/composer/states.rs`, Ctrl+G/Tab в `keys.rs` и `session_page.rs`, мышь в composer (`Listener`, правый клик — вставка), запуск очереди при drain.
- Общие: `keys.rs`, `session_page.rs`, `catalogue.rs` (Ctrl+G уже есть), `core/src/threads.rs`, `host effects`.

### P3. openers: /presets, /daemon, /subagents, app link
- Строки (7): /presets ❌, «Quick dials» ❌, /daemon ❌, «Daemon (подключение)» ❌, /subagents ❌, «Ссылка на приложение» ❌, «Model provider switch» ❌.
- Oracle: `chatwidget/quick_dials.rs`, `app/quick_dials.rs`, `app/daemon_menu.rs`, `app/multi_agents.rs`, small_dialogs app link.
- Тесты: presets 1, daemon 2 (picker, small_dialogs), provider_switch 1; шарды picker, small_dialogs_1/2 (≈8).
- Core (≈400 строк, 7 строк, 1 прогон): verb `Intent::Quick` из `SlashCommand::Presets` в `app/session.rs`/`dispatch.rs` (модель `select::presets`/`quick_dials.rs` готова), открывалка `select::daemon`, новая модель выбора sub-агента (`core/src/screens/select/agent_picker.rs`, порт `app/multi_agents.rs`), факты provider switch.
- UI (≈450 строк, 1 прогон): `open.rs` (`Target::Select`, `Target::AgentPicker` вместо `not_yet`), `actions.rs` (убрать `presets_action`, `daemon_action`, `action_subagents`), листы daemon и agent picker (`src/sheets/`), лист Quick уже есть.
- Общие: `open.rs`, `actions.rs`, `app/session.rs`, `dispatch.rs`.

### P4. settings-feeds: /experimental, /skills, /hooks
- Строки (7): /experimental ❌, /skills ❌, /hooks ❌, «Hooks popup», «Experimental features popup», «Manage skills summary» ❌, «Current settings» ❌.
- Oracle: `experimental_features.rs`, `bottom_pane/experimental_features_view.rs`, `bottom_pane/skills_toggle_view.rs`, `bottom_pane/hooks_browser_view.rs`, `hooks_rpc.rs`, `chatwidget/hooks.rs`.
- Тесты: hooks 17 (chat_status_2 14), experimental 4, skills 2, current_settings 1 — ≈24.
- Core (≈500 строк, 7 строк, 1 прогон): `core/src/features/settings/{mod,catalog,hooks,items}.rs` (`open` для `Which::Experimental|Skills|Hooks` вместо `return`), три ленты как `Ask`/`Done` (`experimentalFeature/list`+write, `skills/list`, `hooks/list`) в `core/src/app/effects.rs` и исполнители host (`host/src/settings.rs`, `host/src/in_process.rs`), `S::Hooks` как intent вместо NotAvailable (`app/session.rs`), `Fact::Offered` из host.
- UI (≈400 строк, 1 прогон): подключить готовые sheet `src/features/settings/{features,setup,hooks}.rs` к фактам, `src/features/settings/mod.rs`; снимки chat_status_2/chat_popups_4.
- Общие: `core/src/features/settings/mod.rs`, `src/features/settings/mod.rs`, `host/src/in_process.rs`, `app/session.rs`, `core/src/app/effects.rs`.

### P5. settings-config-keymap: /config, /tui, /keymap
- Строки (5): /config ❌, /tui ❌, /keymap ❌ (команды), «Settings: Config/Keymap» (часть строки 🟡), «Current settings» (offers, пересекается с P4).
- Oracle: `chatwidget/config_panel.rs`, `app/config_panel.rs`, `config_panel/`, `bottom_pane/config_panel.rs`, `keymap_setup/`, `chatwidget/keymap_picker.rs`, `chatwidget/tui_mode_picker.rs`, `app/tui_mode_picker.rs`.
- Тесты: keymap 7 (chat_commands_3 4), config_panel/tui 0 по имени; chat_commands_3, small_dialogs.
- Core (≈600 строк, 4 строки, 1 прогон): `Which::Config|Keymap` в `core/src/features/settings/mod.rs` (сейчас `return`), запись keymap/tui-mode через `Ask::ConfigWrite` (новые `items` и `persist`), верб `Intent::OpenConfig`.
- UI (≈1500 строк, 2 прогона): (a) config-панель и tui-mode picker, (b) редактор клавиш (`keymap_setup/`: захват клавиши, конфликты, сохранение) — новые `src/features/settings/{config_panel,tui_mode,keymap_editor}.rs`.
- Общие: `core/src/features/settings/mod.rs`, `src/features/settings/mod.rs` (с P4), `dispatch.rs`, `app/session.rs`, keymap-таблицы.

### P6. copy: /copy, Ctrl+O, copy picker, Markdown-копия
- Строки (6): /copy ❌, «Copy/export picker» ❌, «Копия как Markdown/plain» ❌, Ctrl+O ❌, «Копирование выделения» 🟡 (формат), «Shift-клик» 🟡 (rtwk-gap).
- Oracle: `chatwidget/copy_picker.rs`, `markdown_copy.rs`, `clipboard_copy.rs`, `transcript_view/copy_mode.rs` (общие части).
- Тесты: copy 56 по имени (transcript_view_2 14, chat_commands_2 13, chat_commands_3 7, transcript_view_5 5).
- Core (≈350 строк, 4 строки, 1 прогон): verb `Intent::Copy` вместо `NotAvailable(Copy)` (`app/session.rs`, `dispatch.rs`), модель copy picker (`core/src/screens/select/copy.rs`), `Ask::Clipboard` с payload plain/HTML (HTML-сервис готов, 5026ad24c).
- UI (≈450 строк, 1 прогон): лист copy picker, Ctrl+O в `keys.rs`/`session_page.rs`, `src/pages/session/transcript/copy.rs` (формат payload).
- Общие: `app/session.rs`, `dispatch.rs`, `keys.rs`, `session_page.rs`, `transcript/copy*.rs` (с P8).

### P7. transcript-find: F3/F4, поиск по транскрипту и pager
- Строки (5): «Поиск по транскрипту F3/F4» ❌, «Подсказки поиска и футер поиска» ❌, «Pager: поиск /, n/N» ❌, F3 🟡, «Скорость колеса» ❌ (мелочь рядом).
- Oracle: `transcript_view/{search,search_footer,search_presentation}.rs`, `pager_overlay.rs`.
- Тесты: find/search 56 (transcript_view_4 23, pane_footer_1 6, transcript_view_1 6, composer_popups 5, composer_4 4).
- Core: нет (поиск — UI-состояние по ADR-013; нужен только core-тест правил).
- UI (≈900 строк, 1 прогон): новые `src/pages/session/transcript/{find,find_footer}.rs`, F3 в `session_page.rs` (сейчас `FindTranscript => modes.open()`), поиск pager после P1.
- Общие: `transcript/mod.rs`, `session_page.rs`, `keys.rs`; pager-часть зависит от P1-UI.

### P8. select-copy-modes (UI-b): режим выделения и copy mode
- Строки (3): «Режим выделения с клавиатуры» ❌, «Режим копирования (Ctrl+X)» ❌, «Двойной/тройной клик» ❌ (rtwk-gap).
- Oracle: `transcript_view/{select_mode,copy_mode,selection}.rs`.
- Тесты: select mode 6 (transcript_view_4 4), плюс часть copy (transcript_view_5).
- Core: нет.
- UI (≈900 строк, 1 прогон): новые `src/pages/session/transcript/{select_mode,copy_mode}.rs`; перекрывается с P6/P7 файлами `transcript/mod.rs`, `selection.rs`.
- Общие: `transcript/mod.rs`, `selection.rs`, `session_page.rs`.

### P9. exec-mcp: exploration, фоновые терминалы, /ps, инвентарь MCP
- Строки (5): «Группы exploration» ❌, «Unified exec» ❌, /ps ❌, /mcp ❌, «Инвентарь MCP» ❌.
- Oracle: `chatwidget/command_lifecycle.rs`, `bottom_pane/unified_exec_footer.rs`, `history_cell/exec.rs`, `history_cell/mcp.rs`, `app/background_requests.rs`.
- Тесты: ps/unified exec 17 (chat_approvals_1 13, cells_3 2, cells_4 2), mcp inventory 23 (cells_4 11, chat_live_3 4), exploration ≈20 (cells_3).
- Core (≈900 строк, 5 строк, 1 прогон, предел): новые `core/src/threads/{processes,mcp_inventory}.rs`, группировка exploration в `core/src/threads/cells.rs`, `Ask::McpInventory` (host `mcpServerStatus/list`), verb `/ps`, `/mcp` вместо NotAvailable (`app/session.rs`).
- UI (≈1000 строк, 1-2 прогона, MCP-часть отдельным): новые `src/pages/session/cells/{processes,mcp_inventory}.rs`, группа exploration в `cells/`, футер unified exec, interaction-ячейка (`cells/interaction` есть).
- Общие: `core/src/threads/cells.rs`, `core/src/threads.rs`, `src/pages/session/cells/mod.rs`, `cells/notice.rs`, `app/session.rs`, `dispatch.rs`.

### P10. safety-compaction: safety buffering, guardian, misalignment, compaction
- Строки (5): «Safety buffering» ❌, «Misalignment/guardian stop» ❌, «Guardian review» ❌, «Compaction» ❌, /compact 🟡.
- Oracle: `chatwidget/safety_buffering.rs`, `safety_recourse`, `misalignment_policy.rs`, `app/misalignment_policy.rs`, `app/safety_buffering.rs`, `chatwidget/compaction.rs`.
- Тесты: safety 8 (chat_live_1 7), misalignment 3 (chat_live_3), guardian 10 (chat_approvals_2), compaction 13 (chat_live_2 5, markdown_streaming_4 2).
- Core (≈800 строк, 5 строк, 1 прогон): новый `core/src/threads/safety.rs` (буферизация, recourse, политика), compaction timer/факты и aggregate guardian в `core/src/threads.rs` и `cells.rs`; verbs повтора/подтверждения.
- UI (≈600 строк, 1 прогон): `src/pages/session/cells/{safety,misalignment}.rs` (`safety` есть), ячейка compaction, статус-заголовок хода.
- Общие: `core/src/threads.rs`, `core/src/threads/cells.rs`, `cells/mod.rs`, `cells/notice.rs` (с P9).

### P11. usage-reset: reset лимита, billing, подсказка на старте
- Строки (7): «Reset-limit picker» ❌, «Меню /usage» 🟡, «Reset лимита» ❌, «Billing и enterprise» ❌, «Подсказка reset на старте» (две строки) ❌, «Статус после billing retries» ❌.
- Oracle: `chatwidget/usage.rs`, `reset_credits.rs`, `usage_notice.rs`.
- Тесты: usage reset 34 (chat_live_6 20, chat_status_1 4, chat_status_2 3, usage 3).
- Core (≈500 строк, 7 строк, 1 прогон): новый `core/src/screens/usage/reset.rs` (запрос, idempotency-ключ, in-flight по ключу), `Ask::ResetCredits` + host, пункты меню как verbs.
- UI (≈550 строк, 1 прогон): новый `src/pages/session/live/usage_menu.rs`, picker и confirm sheet, подсказка на старте.
- Общие: `core/src/screens/usage*`, `core/src/app/effects.rs`, `host/src/in_process.rs`, `src/pages/session/live/*` (с P12).

### P12. status-limits: карточка /status и предупреждения лимитов
- Строки (5): «/status: usage треда» ❌, «/status: строка сервера» ❌, «Предупреждения о лимитах» ❌, «Rate-limit подсказки во время хода» ❌, «Ранний redraw thread usage» ❌.
- Oracle: `chatwidget/thread_usage.rs`, `rate_limits.rs`, `status` модули.
- Тесты: limit 38 (chat_status_1 14, chat_live_6 10, chat_status_3 3).
- Core (≈400 строк, 5 строк, 1 прогон; может слиться с P11 core): факты usage треда и rate limits (90/95%) в `core/src/threads.rs`, строка сервера как факт host.
- UI (≈300 строк, ≤1 прогон; слить с UI P11): строки карточки и ленточные предупреждения.
- Общие: `core/src/threads.rs`, `core/src/screens/usage*`, `cells/notice.rs`, `live/*` (с P11).

### P13. side: /side, /btw, Ctrl+/, подпись контекста
- Строки (4): «Side-разговор» ❌, /side,/btw ❌, Ctrl+/ ❌, «Подпись контекста side-разговора» ❌. Задачи нет.
- Oracle: `app/side.rs` и side-файлы.
- Тесты: side 9 (chat_live_5 7); по шардам «4/33 (не вкл. 2)».
- Core (≈700 строк, 4 строки, 1 прогон): новый вид треда side, форк-вопрос, отказы в side-треде, `Notice::NotAvailable(Side)` убрать из `app/session.rs`.
- UI (≈400 строк, 1 прогон): Ctrl+/ в `session_page.rs:683`, подпись родителя в статус-строке.
- Общие: `app/session.rs`, `dispatch.rs`, `session_page.rs`, `core/src/threads.rs`.

### P14. cmds-small: /logout, /ide, /daybreak, /recap, /debug-config, /debug-m-*
- Строки (6): /logout ❌, /ide ❌, /daybreak ❌, /recap ❌, /debug-config ❌, /debug-m-drop|update 🟡. Задачи нет (chunk-commands завершена).
- Oracle: `daybreak.rs`, `ide_context.rs`, `debug_config.rs`, `app/recap.rs`, `chatwidget/recap.rs`.
- Тесты: recap 5 (transcript_view_5 3), ide 4, daybreak 2.
- Core (≈600 строк, 6 строк, 1 прогон): verbs и `Ask` (logout, ide context, recap) в `dispatch.rs`/`app/session.rs`, host-исполнители; убрать `Stub(MemoryMaintenance)`.
- UI (≈500 строк, 1 прогон): ячейки recap и debug-config, заметки.
- Общие: `app/session.rs` (арм NotAvailable — правят все), `dispatch.rs`, `cells/notice.rs`.

### P15. cmd-args: аргументные формы
- Строки (4): `/model <имя>`, `/resume <имя>`, `/title <args>`, `/statusline <args>` — в таблице строки команд 🟡; также «Команды с inline-аргументами» 🟡.
- Oracle: `app/thread_title.rs`, `status_surface_commands.rs`, `chatwidget/status_surfaces.rs`, провайдер-запись /model.
- Тесты: statusline/title 30 по имени (chat_status_3 10, chat_status_2 7), resume_2 12.
- Core (≈400 строк, 4 строки, 1 прогон): `Intent::Model{..}` (запись провайдера, `Ask::ConfigWrite`), `ResumeNamed` (поиск по имени), `Which::Title|StatusLine` с аргументами (`features/settings/items.rs`) — убрать NotAvailable-армы в `app/session.rs`.
- UI: ≈100 строк, отдельного прогона не нужно (в core-прогоне).
- Общие: `app/session.rs`, `core/src/features/settings/mod.rs` (с P4/P5), `dispatch.rs`.

### P16. worktree-checkout: /new, /fork, /worktree
- Строки (5): /new 🟡, /fork 🟡, /worktree 🟡, «Worktree/checkout picker» 🟡, «Forked thread» ❌.
- Oracle: `chatwidget/worktree_picker.rs`, `app/managed_worktree_creation.rs`.
- Тесты: worktree 6 (chat_popups_4 5), fork 4/11.
- Core (≈600 строк, 5 строк, 1 прогон): исполнители `Ask::Worktrees`/`Ask::WorktreeRemove` (в `src/services/effects.rs` сейчас `return None`; `host/src/workspace.rs` с `WorktreeManager` есть), шлёт ли приложение `Intent::OpenCheckout` в /new и /fork (`select.rs:706`), создание worktree, fork-history ячейка.
- UI (≈300 строк, 1 прогон): вид checkout-выбора и цепочка старта, строка «Forked thread» в `cells/`.
- Общие: `app/session.rs`, `actions.rs`, `core/src/threads/cells.rs`, `host/src/in_process.rs`.

### P17. feedback: диалог отчёта
- Строки (3): /feedback 🟡, «Feedback: выбор, consent…» 🟡, «Feedback (диалог)» 🟡. Задача chunk-popups-feedback устарела.
- Oracle: `bottom_pane/feedback_view.rs`, `feedback_note_view.rs`, `app/feedback_upload.rs`.
- Тесты: feedback 27 (small_dialogs_1 14).
- Core (≈400 строк, 3 строки, 1 прогон): шаги note/upload, `Ask::FeedbackUpload` + host.
- UI (≈400 строк, 1 прогон): note view, consent-страницы в `src/features/…` листа dialogs.
- Общие: `host/src/in_process.rs`, `core/src/app/effects.rs`.

### P18. sandbox-prompts: Windows sandbox, security setup, подтверждение full access
- Строки (4): «Windows sandbox» ❌, /setup-default-sandbox ❌, «Security setup» ❌, «Permissions popup» 🟡 (full-access confirm).
- Oracle: `chatwidget/windows_sandbox_prompts.rs`, `security_setup.rs`, `chatwidget/security_setup.rs`.
- Тесты: windows 13 (chat_popups_2 10), security 5 (chat_popups_4).
- Core (≈400 строк, 4 строки, 1 прогон): шаги `ElevateSandbox` вместо NotAvailable, `security_setup`.
- UI (≈500 строк, 1 прогон): листы подтверждения.
- Общие: `app/session.rs`, `core/src/screens/select/permission_*.rs`.

### P19. approvals-rest: решение approval, вопросы
- Строки (10): «Решение approval» ❌, «Approval apply_patch» 🟡, «Approval network» 🟡, «Approval write_stdin» ❌, «Вопросы: черновики» ❌, «поиск по истории/Vim в поле ответа» ❌, «пустые ответы», «мышь/ссылки», «очередь вопросов» ❌, «Keymap-seeds» ❌. Задачи нет (chunk-cells-question-result завершена).
- Oracle: `chatwidget` approval cells, `bottom_pane/request_user_input*`.
- Тесты: chat_approvals_1/2 ≈120 pending (из них guardian 10 — в P10, ps 13 — в P9).
- Core (≈300 строк, 3 строки, 1 прогон): запись решения в `ThreadModel` (decision cell данные), write_stdin событие.
- UI (≈1000 строк, 2 прогона): ячейка решения, черновики, Vim в ответе, мышь.
- Общие: `core/src/threads/cells.rs`, `cells/mod.rs`, `cells/notice.rs`.

### P20. cells-dialogs-sweep (UI-только): мелочь и снимки
- Строки (≈15): F4 «Фокус на activity-группах» ❌ (только UI-часть; нужен rtwk `visible_range()`), «Ячейка collab/sub-agent», «Просмотр/генерация изображений», «Worked for», «Гиперссылки», «Сворачивание деталей», «Ambient pet», «Sparkle», «Flash», «Предупреждения (/warnings)», «Шорткаты «?»», «SelectionList: тап», /goal и «Goal: меню» (снимки), «Sweep: прочие popup'ы» (?), «Trust on open» (?), /pets, «Верификация пользователя».
- Oracle: `history_cell`, `status` модули, `chatwidget/goal_menu.rs`.
- Тесты: pets 11, goal 15 (chat_popups_1 8), user verification 2, theme 5; большая часть «работает, тест не включён» (разбор — задача harness).
- Core: нет. UI (≈700 строк, 1-2 прогона): по строкам; пересекается с P9/P10 в `cells/mod.rs`.

### P21. voice: /voice, F8, voice strip (заблокирован)
- Строки (4): /voice ❌, «Voice (диалог)» ❌, «Индикатор голоса» ❌, F8 🟡 (пустой sheet).
- Oracle: `bottom_pane/voice_strip.rs`, `chatwidget/realtime*.rs`.
- Тесты: voice 5.
- Core (≈500 строк): состояние realtime, `VoiceControl` вместо NotAvailable; в host нет аудио — Need: решение о host-аудио. UI (≈400 строк). Не брать до решения.

### Не пакеты: n/a, заблокировано, rtwk
- Сайдбар (`app/sidebar*.rs`), inline-режим (`S27`: scrollback-снимки cells_4/5, exec/MCP), дерево UI (`ui_tree.rs`) — вне ленты чата, задачи sidebar/inline-mode/ui-tree остаются как есть.
- rtwk-gaps (Needs, задач нет): `ListController::visible_range()` (закреплённая шапка prompt, TurnTip/head-tail 50+50), чтение start/end у `SelectionAreaController` (Shift-клик, расширение с клавиатуры), жесты двойного/тройного клика, пульс буллета (G-tool-bullet-pulse), форма `!cmd` (G-exec-user-shell).
- Ctrl+Z suspend (?) и Ctrl+X leader — ждут решения, пакета нет.

### Столкновения: пакеты, правящие один и тот же файл без реестра
| Файл | Пакеты |
|---|---|
| `core/src/app/session.rs` (арм NotAvailable) | P1, P3, P4, P5, P6, P9, P13, P14, P15, P16, P18 |
| `core/src/commands/dispatch.rs` | P1, P3, P5, P6, P9, P13, P14 |
| `src/pages/session/open.rs` | P1, P3 |
| `src/pages/session/actions.rs` | P3, P16 |
| `src/pages/session/session_page.rs`, `keys.rs` | P2, P6, P7, P8, P13 |
| `src/pages/session/transcript/mod.rs`, `selection.rs` | P1, P6, P7, P8 |
| `core/src/threads.rs`, `core/src/threads/cells.rs` | P2, P9, P10, P12, P13, P16, P19 |
| `src/pages/session/cells/mod.rs`, `cells/notice.rs` | P9, P10, P12, P14, P16, P19, P20 |
| `core/src/features/settings/mod.rs`, `src/features/settings/mod.rs` | P4, P5, P15 |
| `host/src/in_process.rs`, `core/src/app/effects.rs`, `host effects` | P2, P4, P9, P11, P14, P16, P17 |
| `core/src/screens/usage*`, `live/*` | P11, P12 |

Практические слияния/порядок: P11+P12 одной цепочкой (общие usage-файлы); P4 → P5 → P15 подряд (settings-слайс); P6 → P8 → P7 подряд (transcript); P9 → P10 подряд в core (`threads/cells.rs`); P1 до P7 (pager-поиск). Арм NotAvailable в `app/session.rs` правят почти все core-половины — по строке на команду, конфликты тривиальны, но слияние должен делать оркестратор в одном порядке.
