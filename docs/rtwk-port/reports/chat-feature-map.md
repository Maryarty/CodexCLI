# Карта функциональности экрана чата: oracle (codex-rs/tui) против порта (codex-rs/tui-rtwk)

Дата: 2026-11-23. Факт-финдинг, код не менялся. Строки обновлены после батчей 3 (0130f3747) и 4 (51abab645) по пакам parity, без нового прогона шардов.

**Итог.** 289 строк-функций: ✅ 123, 🟡 63, ❌ 99 (на 51abab645; на 87ad7733c было 121/62/102), «?» 4 (статус не установлен). Статус сменили четыре строки: история между сессиями ❌→✅, @-упоминания 🟡→✅, копирование выделения и «Shift-клик/правая кнопка» ❌→🟡 (двойной/тройной клик остался ❌: проверки нет); у остальных строк правки только в тексте. Тесты oracle (2607 на 87ad7733c, не пересчитаны): 865 зелёные, 1432 pending, 12 красных, 297 n/a; после 87ad7733c добавились зелёные из паков (chat_status_2 +5, composer +9, history pages +4, chat_commands_2 +5, chat_live_2 +2, markdown +5). Из pending 166 — «работает, тест не включён» (probe-stale: функция по коду и по проверке есть, тест ждёт включения).
**Пять самых больших дыр.**
1. Транскрипт: нет поиска (F3/F4), клавиатурного выделения, copy mode, закреплённой шапки prompt, раскрытия деталей хода. Копирование выделения мышью есть (542959815: отпускание, правая кнопка, Ctrl+C, Enter); нет Shift-клика/расширения с клавиатуры (rtwk-gap: у SelectionAreaController нет чтения start/end), Markdown-копии, X11 primary; клавиши ctrl+home / alt+< для старых страниц истории. transcript_view_4: 0 из 60 зелёные, transcript_view_5: 1 из 59.
2. Слэш-команды: ≈25 из ≈65 отвечают «NotAvailable» или открывают пустой слот: /side /btw /copy /worktree /mcp /ps /recap /vim /voice /tui /diff /hooks /keymap /config /experimental /skills /title /statusline /logout /ide /daybreak /debug-config /subagents.
3. Composer: Vim есть, но включить нечем (/vim, toggle без бинда; ветка fleet/block-composer fdadb68ff с /vim и картинками cc0bbabd5 в 51abab645 не влита); нет вставки картинки Ctrl+V, внешнего редактора Ctrl+G, состояний composer, очереди slash/shell-команд; $-приложения появляются только после /apps. История между сессиями и живой @-поиск файлов со skills/plugins готовы (батч 3).
4. Безопасность и лимиты: safety buffering, guardian, misalignment, лимит-предупреждения, весь reset-flow /usage (chat_live_6: 0 из 25), карточка /status: usage треда и строка сервера.
5. Ячейки инструментов: группы exploration, unified exec и /ps, инвентарь MCP, решение approval и результат вопроса в транскрипте, compaction, строка «Forked thread» (cells_3/4, chat_approvals_1/2: 3 из 76 зелёные).

## Как читать

- Статус: ✅ работает; 🟡 частично (в «Чего нет» сказано чего); ❌ нет; «?» не удалось установить (причина в строке). «работает, тест не включён» — функция по коду есть, а её тесты в oracle-шардах pending по причине обвязки (probe-stale).
- Колонка тестов: зелёные/всего (n/a не считаются) среди тестов oracle, чьё имя совпало с шаблоном строки; «не вкл. N» — сколько из них stale. Совпадение по имени приблизительное: для широких строк (markdown, /status, футер) это весь семейный счёт. Прочерк — подходящих тестов не нашлось (у клавиш счёта нет: их проверяют функции выше).
- Задача — alias в tm (статусы на 51abab645: feature-блоки ПРИОСТАНОВЛЕНЫ на миграцию ADR-013 — tm-контейнер adr013-migrate, решение 2026-11-23 «сначала архитектура»; новых задач из блоков не берут, пока он не закрыт. Сделаны в батчах 3 и 4: chunk-status-card, chunk-composer-live-mentions, chunk-composer-history, chunk-cells-rendering-prefs, chunk-live-banner-fallback, chunk-commands (/rename), chunk-transcript-2 (выделение и копирование) и chunk-transcript-2-history-pages; остальные задачи остались в состоянии до миграции — created, часть held/claimed раньше: chunk-popups-plugins, chunk-status-sparkle, settings). «—» — задачи нет или функция готова.
- Источники: slash_command.rs, research/codex-tui-features.csv, `core/src/commands/dispatch.rs`, `core/src/app/session.rs`, `src/pages/session/{actions,open,keys,session_page}.rs`, verdicts oracle-шардов (summary.py + probe), tests/pages и core/tests как свидетельства работы.
- Известные оговорки: 12 красных тестов — отдельные расхождения, не функции (cells_1: computer_activity, wrap_styled_spans; cells_4: 4 web search; chat_live_4: plan_slash_hidden_shell_paste; composer_1: deleting_prefix_keeps_bang_literal; markdown_streaming_4: recomputing_after_resize; resume_2: page_and_jump_navigation; small_dialogs_2: warning_pages_wrap; transcript_view_2: ordinary_click_opens_links).

## Транскрипт и ячейки

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Сообщение пользователя (блок с фоном, пути, картинки-плейсхолдеры) | ✅ | Фон блока prompt по теме (?) | 10/20 (не вкл. 2) | — |
| Ответ агента: стриминг markdown, списки, таблицы, код, ссылки | 🟡 работает, тест не включён | landed c4c2af4e0: настройки `[tui.rendering]` (таблицы, mermaid, списки) читаются там, где строится сообщение; при выключении источник сохраняется (5 ключевых тестов markdown_streaming_2/5 зелёные). Нет: math off только флаг (G-math-off, G61), маркер списка `•` вместо `- ` (G-list-marker), `user.rs`/reasoning/resume рисуют markdown без настройки, lists_preserve_layout (G73) и MATH-тесты (G61/G62) — sweep; часть markdown-рендера ждёт (счёт не пересчитан) | 126/268 (не вкл. 11; +5 после c4c2af4e0) | chunk-cells-rendering-prefs (сделана) |
| Подсветка кода и diff в ответах | ✅ | — | 14/55 (не вкл. 3) | — |
| Гиперссылки в тексте (клик открывает) | 🟡 | обычный клик по ссылке красный (ordinary_click_opens_links), OSC-8 в активной ячейке ждёт | 3/17 (не вкл. 1) | chunk-transcript-2 |
| Ячейка reasoning (сводка, сырой reasoning, заголовок) | ✅ | landed a15c35bf8: сводка live и в истории, raw reasoning по show_raw_agent_reasoning, пустые блоки (7 тестов зелёные) | 20/55 (не вкл. 3) | chunk-cells-reasoning |
| Ячейка exec: команда, вывод, статус, длительность | 🟡 | компактная форма как у oracle (afd840c6e; non-zero exit — bold-буллет); нет: пульс буллета running (G-tool-bullet-pulse), форма `!cmd` с 50 строками (G-exec-user-shell), scrollback-снимки ждут inline-режима S27 | 5/50 (не вкл. 2) | chunk-cells-rendering-prefs, inline-mode |
| Группы exploration (Read/List/Search свёрнуты в одну) | ❌ | группировка, прерывание, нулевой exit, порядок с reasoning (≈20 тестов) | 0/9 | chunk-cells-tool-cells |
| Unified exec: фоновые терминалы, wait-streak, interaction-ячейка | ❌ | ячейка взаимодействия, «waited for», сохранение при interrupt | 2/19 | chunk-cells-background-terminals |
| Ячейка patch (apply_patch) со сводкой и раскрытием diff | ✅ | пустой/failed-patch формат, ручное approval-заголовок (тесты ждут) | 11/27 (не вкл. 1) | chunk-cells-tool-cells |
| Ячейка MCP-вызова (running/ok/failed, результат) | 🟡 | в чате совпадает с oracle (pty, afd840c6e); снимки cells_4/5 — scrollback-форма, ждут inline-режима S27; пульс буллета ❌ | 2/20 | inline-mode |
| Инвентарь MCP (вывод /mcp) | ❌ | ячейка и данные инвентаря | 0/4 | chunk-cells-mcp-inventory |
| Ячейка web search (запрос, детали, усечение) | ✅ | строка в чате и обёрнутая форма в Ctrl+T (afd840c6e) | 5/5 | — |
| Ячейка просмотра/генерации изображений | 🟡 | image_generation_call_adds_history_cell и просмотр: рендер есть (cells/viewed_image, generated_image), снимки ждут | 3/4 | chunk-cells-tool-cells |
| Ячейка плана (update_plan) и предложенный план | ✅ | — | 5/9 | — |
| Ячейка collab/sub-agent (spawn, wait, send, модель/effort) | 🟡 | spawn_end с моделью, wait-элементы, повторный рендер sub-agent | 0/4 (не вкл. 1) | chunk-cells-tool-cells |
| Ячейка delegated (делегированный вывод tool) | ✅ | — | 2/2 | — |
| Ячейка hook (запущен/завершён) | ✅ | — | — | — |
| Решение approval в транскрипте (Approved/Denied + команда) | ❌ | decision cell: усечение многострочных команд, network-allow, canceled host | 0/5 (не вкл. 2) | chunk-cells-question-result |
| Результат вопроса request_user_input в транскрипте | ✅ | landed a15c35bf8: ячейка ответа/прерывания (completed_and_interrupted_results); markdown из истории — sweep | 0/4 | chunk-cells-question-result |
| Ячейка ошибки (upstream, cyber policy, overloaded, safety access) | ✅ | chained config error wrap (pending) | 3/9 | — |
| Предупреждения (warning, sqlite, config, guardian, model verification) | ✅ | wrap уведомления в narrow, notice_details_fit | 6/6 | — |
| Deprecation / update-available / version-notice | ✅ | update-available и server-version от host (e263606f0); server-version дедуп в пределах запуска (oracle хранит last_shown) | 4/6 | — |
| Часы/время в уведомлениях (clock facts) | 🟡 | Clock и LinkDisplay от host (host/src/viewer.rs, e263606f0); RuntimeMetrics без источника (G-runtime-metrics → host-счётчики, core-live-debts) | 1/4 (не вкл. 1) | core-live-debts |
| Session info: шапка сессии (модель, cwd, подсказки, tooltips) | ✅ | landed 87ad7733c: карточка шапки, каталог подсказок и выбор tip-строки, model-changed notes, стартовые факты от host | 2/13 (не вкл. 1) | chunk-cells-session-info |
| Строка «Forked thread» / имя треда в истории | ❌ | fork-history line с именем/id, prompt-edit history line | 0/5 | chunk-cells-session-info |
| Review-ячейки (баннер Current changes, request hint, prompt скрыт) | ✅ | — | 4/4 | — |
| Compaction: статус, таймер, ячейка завершения | ❌ | timer, snapshot restore, clear on turn end | 1/5 | chunk-live-safety |
| Final «Worked for …» разделитель с длительностью | 🟡 | cumulative duration snapshot ждёт; разделитель есть (cells/separator) | 0/1 | chunk-cells-tool-cells |
| Переключатель плотности (compact/detailed) ячеек | ✅ работает, тест не включён | — | 3/5 (не вкл. 1) | — |
| Сворачивание деталей хода (collapse_turn_details), раскрытие групп | 🟡 | сворачивание, сводки групп, раскрытие кликом/Enter/Space (16ed6526f); нет: ширинно-зависимый счёт скрытых строк exec, хранение 50+50 строк вывода, TurnTip, закреплённая шапка prompt (G-new-usage-2) | 4/10 | chunk-transcript-2 |
| Фокус на activity-группах (F4) и подсказка раскрытия | ❌ | focus_activity целиком | 0/1 | chunk-transcript-2-turn-details |
| Автопрокрутка за хвостом, «вернуться вниз», строка новой активности | ✅ | — | 10/30 (не вкл. 5) | — |
| PageUp/PageDown, колесо мыши, Home/End в транскрипте | 🟡 | Home/End в ленте; скорость колеса (mouse_scroll_speed) не применяется | 4/18 | chunk-transcript-2 |
| Подгрузка старых страниц истории при прокрутке вверх | 🟡 | landed ee4c83bd9: статус-строка (loading / refused с retry / loadable, 4 ширины) из `transcript/history.rs`, join страницы не двигает читателя, 9 page-тестов, 4 oracle-теста зелёные; pty: тот же финальный экран. Нет: клавиши ctrl+home / alt+< (идти к началу, retry) и ctrl+end / alt+> (Need: session_page.rs); склейка групп oracle при join (page_join_* в transcript_view_3, sweep); join при читателе в 1-3 строках ниже первой сдвигает вид на строку; один тик колеса двигает много больше строк, чем у oracle | 6/27 (не вкл. 3; +4 после ee4c83bd9) | chunk-transcript-2-history-pages (сделана) |
| Закреплённая шапка prompt над транскриптом (pinned header) | ❌ | нет модуля prompt_header | 0/3 | chunk-transcript-2 |
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
| Очередь slash- и shell-команд во время хода | ❌ | команды в очереди, !shell в очереди, отклонение при MCP startup | 4/6 (не вкл. 1) | chunk-commands-queue |
| Правка очереди (Alt+Up, вернуть в composer) | ✅ | — | 0/2 (не вкл. 1) | — |
| Steer: Enter во время хода отправляет уточнение | 🟡 работает, тест не включён | pending steers: восстановление при interrupt ок; ordering/mention-bindings/failed steer не включены | 13/23 (не вкл. 7) | chunk-commands-submission |
| Предпросмотр pending steers над composer | ✅ | — | 9/15 (не вкл. 4) | — |
| Отправка во время review/plan stream (очередь вместо steer) | ✅ | — | 1/3 | — |
| Блокировка отправки при недоступной модели | ❌ | submit_user_message_blocks_when_thread_model_is_unavailable | 0/1 | chunk-commands-submission |
| Подтверждение Esc/Enter при диалогах поверх composer | ✅ | — | — | — |
| Баннер бэкенда (нумерованные строки, CTA, отклонение по occurrence) | ✅ | landed c4fff1a2b: авто-переключение fallback и `{time}`, F6 (обрыв соединения отменяет запросы в полёте), 2 ключевых теста chat_live_2 зелёные; в pty баннер не гонялся (в mock нет endpoint). Нет (sweep): три теста восстановления (нужен on_rate_limit_error из chunk-status) и снимок backend_banner_notification; has_pending_recovery не портирован, fallback только при чтении аккаунта, notice_model не сбрасывается при смене треда | 6/13 (+2 после c4fff1a2b) | chunk-live-banner-fallback (сделана) |
| Резерв Luna: селектор и статус | 🟡 | селектор работает; модель возврата резерва живёт только в памяти (нужен Ask::ConfigWrite); backend_auth заменяет has_chatgpt_account; патч переключения = модель + effort; статус по активной модели, переживание закрытия баннера — ждут | 1/4 (не вкл. 1) | chunk-live-banner-fallback (сделана) |
| Safety buffering: «повторить», подтверждение, короткое сообщение | ❌ | весь блок safety_buffering (9 тестов) | 0/7 | chunk-live-safety |
| Экран ошибки провайдера (сохраняет черновик, выход в Resume) | ✅ | сохранение между потоками, provider_id sync — ждут | 5/10 | — |
| Read-only внешний writer: заметка вместо composer, переход в Agents | ✅ | — | 4/6 | — |
| Уведомления терминала о завершении хода / approval | 🟡 | настройки notify_when; async-question уведомления n/a | 7/13 (не вкл. 1) | — |
| Stream recovery: восстановление статуса после обрыва | ✅ | — | 2/2 | — |
| MCP startup: статус, lag, отмена, отказ как warning | ✅ | mcp_startup_failure_renders_warning_history, booting snapshot — ждут | 15/20 (не вкл. 1) | — |
| Rate-limit подсказки/предупреждения во время хода | ❌ | limit warnings (90%/95%), prompt по лимиту | 0/1 | chunk-status-card-limit-warnings |
| Misalignment/guardian stop треда | ❌ | misalignment_* (3), guardian parallel reviews, aggregate status | 1/14 | chunk-live-safety |
| Side-разговор (/side, /btw, Ctrl+/): форк-вопрос, подпись родителя | ❌ | нет вообще: Ctrl+/ даёт NotAvailable(Side) | 1/10 | chunk-commands |
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
| Команды с inline-аргументами (/plan x, /goal x, /review x, /rename x) | 🟡 | /model x, /permissions-profile без args; submission-цепочка не вся | 2/8 (не вкл. 3) | chunk-commands-submission |
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
| Картинки из буфера Ctrl+V / Alt+V | ❌ | composer просит у app, app не читает (services/clipboard/read есть) | 0/1 | chunk-composer-images |
| Remote-картинки (URL): submit и нумерация | 🟡 работает, тест не включён | работает, тест не включён | 2/9 (не вкл. 3) | chunk-composer-images |
| Состояния composer: disabled, shutdown, parent-owned, agents-nav hint | ❌ | нет понятия состояний; read-only есть частично | 2/6 (не вкл. 1) | chunk-composer-states |
| Bang-shell режим: !команда, Enter в ходе → run_user_shell | ✅ | — | 7/14 | — |
| Vim: нормальный/вставка/операторы/текст-объекты/поиск (движок) | 🟡 работает, тест не включён | движок и клавиши есть; включить нельзя: /vim NotAvailable, toggle_vim_mode без бинда, vim_mode_default не читается (?) | 8/51 (не вкл. 5) | chunk-composer-vim |
| Vim: replace/yank/dot-repeat | 🟡 работает, тест не включён | работает, тест не включён (composer_4) | 1/7 (не вкл. 1) | chunk-composer-vim |
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
| /model — выбор модели и effort | 🟡 | bare ✅ (OpenModels), landed 87ad7733c (chunk-popups-sweep): 13 тестов chat_popups_4 на живом пикере, курсор по форме страницы, скрытые модели; /model <имя> NotAvailable (provider-write не портирован); advanced effort, refresh каталога не включены | 12/27 | chunk-popups-picker-rest |
| /presets — выбор пресета (Daybreak presets) | ❌ | core-модель select::presets есть, из чата не открывается | 1/1 | chunk-popups-picker-rest |
| /daybreak | ❌ | NotAvailable; отказ Daybreak | 0/2 | chunk-commands |
| /ide — контекст IDE | ❌ | NotAvailable | 0/4 | chunk-commands |
| /permissions — approvals/профили | 🟡 | bare ✅ (OpenPermissions), args через PermissionsArgs ✅; подтверждение full-access, Windows, history cell не все | 8/21 | chunk-popups-permission-args |
| /keymap — редактор клавиш | ❌ | открывается пустой слот; sheet keymap есть в settings/fixtures | 4/12 (не вкл. 1) | settings |
| /vim — переключить Vim composer | ❌ | NotAvailable | 0/1 | chunk-composer-vim |
| /setup-default-sandbox (Windows) | ❌ | ElevateSandbox NotAvailable; только Windows | 1/9 | chunk-popups-windows-sandbox |
| /experimental — feature-флаги | ❌ | пустой слот; sheet settings/features.rs есть | 3/7 (не вкл. 1) | settings |
| /approve — Auto-review | ❌ | core select::auto_review есть, не открывается; denials popup | 3/6 | chunk-popups-picker-rest |
| /memories | ✅ | reset-confirmation ✅ | 2/3 | — |
| /skills | ❌ | пустой слот; manage_skills_summary pending | 0/1 | settings |
| /import — перенос из других агентов | ✅ | route Import; shard import 16/16 | 16/16 | — |
| /hooks | ❌ | NotAvailable; sheet settings/hooks.rs не подключён; hooks_popup_shows_list_diagnostics | 0/1 | settings |
| /review — выбор ветки/коммита/custom | ✅ | branch/commit picker snapshots ждут (review_*_picker, 2 pending) | 32/74 (не вкл. 6) | — |
| /rename — имя треда | ✅ | landed a15c35bf8 (95826e191): с аргументом и bare prompt (slice features/rename); 29abc1aa5: вид prompt `src/sheets/dialogs/prompt.rs`, чат показывает любой открытый моделью prompt (так же файловый prompt /export и edit /goal), 5 тестов chat_commands_2 зелёные, pty rename-prompt проходит в обоих бинарях. Нет: предложение имени — хост отвечает Done::Title(None), oracle генерирует через ephemeral thread (thread_title.rs); нужен backend SuggestTitle в services/rpc | 1/7 (+5 chat_commands_2 после 29abc1aa5) | small-dialogs |
| /new | 🟡 | новый тред ✅; выбор checkout-директории (worktree choice) ❌ | 0/2 | chunk-popups-worktree |
| /archive | ✅ | confirm sheet + RPC | 2/2 | — |
| /delete | ✅ | confirm sheet + RPC | 1/1 | — |
| /resume | ✅ | route Resume; resume_1/2: 48 из 72 зелёные | 48/61 (не вкл. 4) | — |
| /fork | 🟡 | новый тред ✅; пикер checkout и строка «Forked thread» ❌ | 4/11 | chunk-popups-worktree |
| /worktree | ❌ | NotAvailable; core screens/select/worktree есть | 0/5 | chunk-popups-worktree |
| /app — открыть в desktop-приложении | ✅ | OpenUrl codex://threads/<id> | — | — |
| /init | ✅ | отправляет prompt AGENTS.md | 1/1 | — |
| /compact | 🟡 | команда ✅; статус/таймер compaction не включены | 9/24 (не вкл. 2) | chunk-live-safety |
| /config | ❌ | пустой слот; config panel fixture есть | — | settings |
| /recap | ❌ | NotAvailable; ячейка recap | 0/5 | chunk-commands |
| /plan [prompt] | ✅ | маска из каталога, уведомления disabled/unavailable | 1/8 (не вкл. 2) | chunk-popups-plan |
| /voice | ❌ | VoiceControl NotAvailable; F8 открывает sheet voice (пустая папка) | 0/5 | voice |
| /goal [цель/pause/resume/clear] | 🟡 работает, тест не включён | команда и модель (features/goal) ✅; меню goal и edit prompt (snapshots) ❌ | 1/20 (не вкл. 4) | chunk-popups-goal |
| /agents — центр агентов | ✅ | Target::Agents route | 0/1 | — |
| /side, /btw | ❌ | нет; тесты отказов в side-треде тоже ждут | 4/33 (не вкл. 2) | chunk-commands |
| /copy — копировать ответ | ❌ | NotAvailable(Copy); выбор copy picker и Ctrl+O не готовы | 0/11 (не вкл. 2) | chunk-transcript-2-copy-mode |
| /export | ✅ | landed a15c35bf8 (95826e191): путь, picker назначения, prompt имени (slice features/export) | 0/2 | chunk-commands-export |
| /raw | ✅ | режим + заметка, Alt+R | 19/35 (не вкл. 1) | — |
| /tui — быстрые настройки TUI | ❌ | NotAvailable | — | settings |
| /diff | ❌ | Target::Pager показывает «not yet» | — | pager |
| /mention | ✅ | вставляет «@» | 1/1 | — |
| /status — карточка статуса | 🟡 работает, тест не включён | карточка ✅ — ячейка транскрипта треда, как у oracle (status-card-adr003); landed e1795b779 (батч 3): живая карточка из фактов треда, config, аккаунта, каталога моделей и удалённого подключения; лимиты с обновлением по `RateLimitOrigin::Status(request)`, строки Model (display name, reasoning, summaries), Collaboration mode, Agents.md; pty: строки равны oracle. Нет: Thread name (oracle делает title-запрос, порт нет), версия в заголовке `(v0.0.0)` вместо `(v0.170.0)` (версия сборки); thread usage, server row, limit warnings ❌ (дети); лента фактов статус-строки (Cwd/Clock/SideLabel/AgentLabel, git-пробы) не построена | 42/168 (не вкл. 14, не пересчитано; +5 chat_status_2 после e1795b779) | chunk-status-card (сделана; sweep 6 тестов chat_status_2) |
| /daemon | ❌ | select::daemon есть в core, не открывается | 1/2 | chunk-popups-picker-rest |
| /warnings (и F2) | ✅ | — | 2/8 | — |
| /cd <путь> | ✅ | с отказами | 0/1 | — |
| /pwd, /cwd | ✅ | — | 16/16 | — |
| /usage | 🟡 работает, тест не включён | нужен backend auth, иначе заметка UsageNeedsLogin; страница есть (43 теста не включены); меню reset ❌ | 14/81 (не вкл. 44) | usage |
| /debug-config | ❌ | NotAvailable | — | chunk-commands |
| /title — терминальный title | ❌ | пустой слот; сама строка title работает | 2/9 | settings |
| /statusline | ❌ | пустой слот; status_line items работают от фактов | 2/33 (не вкл. 1) | settings |
| /theme | ✅ | settings Theme | 3/8 (не вкл. 1) | — |
| /pets, /pet | 🟡 работает, тест не включён | sheet pets открывается; ambient-часть и тесты не включены | 2/13 (не вкл. 4) | pets |
| /mcp | ❌ | NotAvailable; ячейка инвентаря | 0/4 | chunk-cells-mcp-inventory |
| /apps | ✅ | диалог ссылки приложения, вкл/выкл, сброс при смене аккаунта, обновление каталога (b2eeacb7c); partial-snapshot тесты n/a (нет is_final:false в протоколе) | 9/10 | — |
| /plugins | 🟡 | влит 8c492bcbc: список, установка, marketplaces; расхождения: Esc на карточке, заметка fresh, OpenUrl (core-popups) | 39/41 | core-popups |
| /logout | ❌ | NotAvailable | — | chunk-commands |
| /quit, /exit | ✅ | — | 7/16 (не вкл. 2) | — |
| /feedback | 🟡 | sheet dialogs/feedback.rs есть; upload-consent/diagnostics снимки ждут (27 тестов) | 3/30 (не вкл. 1) | chunk-popups-feedback |
| /rollout | ✅ | — | 2/2 | — |
| /ps — фоновые терминалы | ❌ | NotAvailable | 1/1 | chunk-cells-background-terminals |
| /stop, /clean | ✅ | чистит терминалы + заметка | 1/1 | — |
| /clear | ✅ | новый тред/экран | 1/2 | — |
| /test-approval | ✅ | Target::Approvals | — | — |
| /subagents (multi-agents picker) | ❌ | AgentPicker показывает «not yet» | 0/3 | chunk-commands |
| /debug-m-drop, /debug-m-update | 🟡 | заглушка-заметка (debug) | 1/1 | chunk-commands |

## Попапы и селекторы

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Список выбора: поиск, стрелки, цифры, disabled-строки, hint по биндингам | ✅ | — | 44/58 | — |
| Model picker: каталог, discovery, bedrock/custom display, service tier | 🟡 | landed 87ad7733c: скрытые модели, seed провайдера, лимит потоков агента; осталось: refresh каталога, reasoning submenu (extra high, advanced), снимки | 13/24 | chunk-popups-picker-rest |
| Reasoning picker и Alt+,/. , Shift+Up/Down | ✅ | reach_max (красный), ultra, advanced-only ждут | 2/3 | chunk-popups-picker-rest |
| Permissions popup: пресеты, профили, retry discovery | 🟡 | full access confirm, Windows sandbox prompts, history cell решения | 9/40 | chunk-popups-permission-args |
| Windows sandbox: prompts, help-ссылки, fallback | ❌ | весь блок (≈10 тестов); только Windows | 1/9 | chunk-popups-windows-sandbox |
| Goal: меню и edit prompt, resume-paused prompt | 🟡 работает, тест не включён | команда есть, меню/prompt снимки нет; resume-paused работает, тест не включён | 0/8 (не вкл. 2) | chunk-popups-goal |
| Plan: popup реализации и scope reasoning | ✅ | — | 9/12 | chunk-popups-plan |
| Collaboration / mode catalog | 🟡 работает, тест не включён | каталог недоступен (тест не включён) | 0/1 (не вкл. 1) | chunk-popups-plan |
| Worktree/checkout picker | ❌ | модель в core, нет вида | 0/5 | chunk-popups-worktree |
| Quick dials (saved dials: save/clear picker) | ❌ | модель в core, не открывается | 0/1 | chunk-popups-quick-dials |
| Plugins: список, вкладки, поиск, деталь, install, marketplaces | ❌ | ≈40 тестов; задача в работе | 1/44 | chunk-popups-plugins |
| Apps/connectors: список, статус, install-only | 🟡 | простые состояния ✅; partial refresh ❌ | 3/9 | chunk-popups-apps-rest |
| Hooks popup: список, диагностика | ❌ | sheet есть, не открывается | 0/1 | settings |
| Experimental features popup (toggle сохраняется на выходе) | ❌ | — | 3/7 (не вкл. 1) | settings |
| Feedback: выбор, consent upload, диагностика, note | 🟡 | sheet есть; снимки ждут | 3/30 (не вкл. 1) | chunk-popups-feedback |
| Review picker (ветка/коммит/custom) | ✅ | — | 1/1 | — |
| Copy/export picker (кастомные клавиши, payload) | ❌ | — | 0/1 | chunk-commands-export |
| Reset-limit picker и подтверждение (usage menu) | ❌ | ≈30 тестов chat_live_6 | 0/11 | chunk-usage-reset |
| Security setup / security prompts | ❌ | security_setup_* (≥4) | 0/5 | chunk-popups-sweep |
| Memories settings и reset | ✅ | — | 2/3 | — |
| Manage skills summary | ❌ | — | 0/1 | settings |
| Current settings (model/permissions/status layout) | ❌ | current_settings_offer_* | 0/1 | settings |
| Sweep: прочие мелкие popup'ы | ? | состав «sweep» не установлен по коду; задача created | — | chunk-popups-sweep |

## Approvals и вопросы

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Approval exec: модал, варианты, execpolicy-префикс, причина | ✅ | multiline-prefix скрытие execpolicy ждёт | 3/11 (не вкл. 2) | — |
| Approval apply_patch: модал, call_id, заголовок | 🟡 | untrusted модал, manual flow, patch snapshot ждут | 0/4 | chunk-cells-question-result |
| Approval network host (разово/сессия) | 🟡 работает, тест не включён | history decision cell ждёт; запрос ✅ | 0/4 (не вкл. 2) | chunk-cells-question-result |
| Approval write_stdin | ❌ | render terminal input | 0/3 | chunk-cells-question-result |
| Permissions request approval, MCP elicitation форма | ✅ | — | 0/8 | — |
| Guardian review (approved/denied/timed out, aggregate status) | ❌ | 0 из 11 | 1/11 | chunk-live-safety |
| Auto-review denials popup | ❌ | — | 0/1 | chunk-popups-picker-rest |
| request_user_input: вопросы с вариантами, цифры/буквы | ✅ | — | 9/16 | — |
| Вопросы: черновики, навигация, replay черновиков | ❌ | question_drafts_survive_*, turn_end drafts (≈10) | 0/8 | chunk-cells-question-result |
| Вопросы: поиск по истории, Vim в поле ответа | ❌ | history search, vim editing | 0/2 | chunk-composer-vim |
| Асинхронные вопросы (desktop answer, уведомления) | 🟡 | уведомления n/a; многое n/a — другой клиент | 0/2 | — |
| Отклонение пустых ответов, явный skip | ❌ | — | 0/1 | chunk-cells-question-result |
| Разворот вопроса уступает мышь терминалу для ссылок | ❌ | — | 0/1 | chunk-cells-question-result |
| Очередь вопросов и queued messages делят один shortcut | ❌ | — | 0/2 | chunk-cells-question-result |
| Keymap-seeds для approval | ❌ | — | 3/5 | chunk-composer-mention-seeds |

## Футер и статус-строка

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Idle-футер: подсказки (? shortcuts, Esc, Ctrl+C), режимы | ✅ | — | 26/118 (не вкл. 13) | — |
| Подсказка очереди «Alt+Up edit» | ✅ | — | — | — |
| Строка статуса: модель, effort, cwd, context %, токены | ✅ | git-ветка, PR со ссылкой, изменения ветки, headline, raw output (ea69879b9); нет: Cwd/Clock-факты и пробы git/headline от host, заголовок треда в строке | 2/33 (не вкл. 1) | chunk-status-card |
| Настройка статус-строки (/statusline, порядок элементов) | ❌ | — | — | settings |
| Терминальный заголовок (title) с элементами | 🟡 | рендер ✅; настройка ❌ | 2/9 | settings |
| Индикаторы режимов: Plan, goal, side, agent, Vim | ✅ | рисуются (ea69879b9); метки side/agent ждут фактов от host | 3/9 (не вкл. 1) | — |
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
| Подпись контекста side-разговора | ❌ | — | 0/4 | chunk-commands |
| Подсказка reset-лимита на старте | ❌ | — | 0/2 | chunk-usage-reset |

## Транскрипт-оверлей / pager / backtrack / выделение-копирование-поиск

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Ctrl+T: подробный транскрипт (sheet) с reasoning и warnings | ✅ | — | 14/53 (не вкл. 1) | — |
| Esc-Esc: backtrack — выбор прошлого prompt, Enter откат | ✅ | — | 2/4 | backtrack |
| Backtrack: стрелки влево/вправо по prompt'ам, подсветка | ✅ | ?: часть тестов backtrack всего 4 | 2/4 | backtrack |
| Статический pager (/diff, вывод): строки, страницы, Home/End, q, проценты | 🟡 | есть как режим страницы; /diff «not yet», live-tail sync и поиск ❌ | 5/18 | pager |
| Pager: поиск /, n/N | ❌ | — | 0/1 | pager |
| Pager: live tail sync при росте транскрипта | ❌ | — | — | pager |
| Мышью выделить текст в транскрипте (подсветка) | ✅ | SelectionArea рисует выделение | 4/64 (не вкл. 1) | chunk-transcript-2 |
| Копирование выделения (Ctrl+C / по отпусканию) | 🟡 | landed 542959815: копия по отпусканию, Ctrl+C / Cmd+C / Ctrl+Shift+C, Enter = копия и вниз, Esc снимает, подсказка в футере, строка «Copied N chars to host clipboard», выделение снимается после подтверждённой копии (pty text-100: рядом с oracle совпадает); сервис копирования clipboard-service (5026ad24c). Расхождения: конец диапазона у oracle до ячейки под курсором, у rtwk SelectionArea включает её («Copied 12 chars» вместо 11, rtwk gap); OSC 52 при копии по отпусканию пишется дважды (clipboard-provider в rtwk). Sweep: X11 primary, deselect сравнивает текст, не диапазон, oracle-снимки на 3-4 строках (kit::pending) | 0/11 (не вкл. 1; transcript_view_1 14/36/1 на 542959815) | chunk-transcript-2 |
| Двойной/тройной клик: слово/строка/абзац | ❌ | по паку это жесты rtwk SelectionArea; тестов и проверки в бинаре у порта нет | 0/1 | chunk-transcript-2 |
| Shift-клик расширяет, правая кнопка копирует | 🟡 | правая кнопка копирует (Listener в SelectionHost, 542959815, тест в selection_copy.rs); Shift-клик и расширение с клавиатуры не построить: у SelectionAreaController есть `select(target, item, start, end)`, но нет чтения start/end (rtwk gap) | 1/5 (не вкл. 1) | chunk-transcript-2 |
| Копия как Markdown/plain (payload) | ❌ | HTML-вариант Markdown-копии в сервисе готов (5026ad24c); выбора формата в UI нет; копия выделения мышью только plain text (CopyFormat::MarkdownSelection ждёт тексты элементов); transcript_view_3 в основном n/a | 1/1 | chunk-transcript-2-copy-mode |
| Режим выделения с клавиатуры (select mode) | ❌ | нет модуля | 0/6 | chunk-transcript-2-select-mode |
| Режим копирования (copy mode, Ctrl+X-лидер) | ❌ | нет модуля | — | chunk-transcript-2-copy-mode |
| Поиск по транскрипту F3/F4 (подсветка, счётчик, n/N) | ❌ | F3 только открывает sheet | 5/39 | chunk-transcript-2-find |
| Подсказки поиска и футер поиска | ❌ | — | 1/1 | chunk-transcript-2-find |
| Скорость колеса (mouse_scroll_speed) | ❌ | настройка есть, не применяется | 1/2 | chunk-transcript-2 |
| Старые страницы в overlay (history pages) | 🟡 | ee4c83bd9: статус-строка sheet транскрипта берёт спеллинги из `transcript/history.rs` (`sheet::status`); нет ctrl+home/alt+< и склейки групп (см. строку выше) | — | chunk-transcript-2-history-pages (сделана) |
| Сайдбар (панель агентов/тредов) | ❌ | ADR-закрытая задача sidebar | 0/1 | sidebar |
| Inline-режим (без alt-screen) | ❌ | задача created | — | inline-mode |
| Дерево UI (ui-tree) | ? | не относится к ленте; задача created | — | ui-tree |
| Клик по ссылке/файлу в overlay | 🟡 | обычный клик красный | 0/1 | chunk-transcript-2 |

## Диалоги

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Предупреждения (/warnings, F2): страницы, wrap | 🟡 | warning_pages_wrap красный | 0/2 | small-dialogs |
| Шорткаты: оверлей «?» со всеми действиями | ✅ | — | 2/4 | — |
| Rename thread prompt (bare /rename) | ✅ | slice features/rename (a15c35bf8), вид prompt 29abc1aa5; нет предложения имени (Done::Title(None), нужен SuggestTitle) | — | small-dialogs |
| Confirm: archive/delete треда | ✅ | — | 12/19 | — |
| Daemon (подключение к daemon) | ❌ | — | 1/2 | small-dialogs |
| Ссылка на приложение (app link) диалог | ❌ | — | — | small-dialogs |
| Custom review prompt | ✅ | — | 0/1 | — |
| Верификация пользователя (user verification) | ❌ | — | 1/3 | small-dialogs |
| Settings: Theme, Memories (из чата) | ✅ | — | 5/11 (не вкл. 1) | settings |
| Settings: Config / Statusline / Title / Experimental / Skills / Hooks / Keymap | ❌ | из чата открывается пустой слот | 7/19 (не вкл. 1) | settings |
| Feedback (диалог) | 🟡 | — | 3/30 (не вкл. 1) | chunk-popups-feedback |
| Pets (диалог выбора) | 🟡 | — | 2/13 (не вкл. 4) | pets |
| Voice (диалог, F8) | ❌ | — | 0/5 | voice |
| Trust on open (запрос доверия к каталогу) | ? | в startup-шарде 29/32 зелёные; из чата не вызывается | 10/11 | trust-on-open |
| Model provider switch (блокирует кэш выбора) | ❌ | provider_switch_* | 0/1 | chunk-popups-picker-rest |
| Debug menu F12 | ✅ | — | 0/2 | — |

## /usage

| Функция | Статус | Чего нет | Тесты зелёные/всего | Задача |
|---|---|---|---|---|
| Страница аналитики: бары, вкладки, часы плана, narrow-графики | 🟡 работает, тест не включён | рендер работает, 43 теста не включены | 3/46 (не вкл. 43) | usage |
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
| F4 — focus activity | ❌ | — | — | chunk-transcript-2-turn-details |
| F8 — голос | 🟡 | открывает пустой sheet | — | chunk-status-voice |
| F12 — debug menu | ✅ | — | — | — |
| Ctrl+/ (Ctrl+_, Ctrl+7) — side | ❌ | NotAvailable | — | chunk-commands |
| Ctrl+O — копировать ответ | ❌ | — | — | chunk-transcript-2-copy-mode |
| Ctrl+G — внешний редактор | ❌ | — | — | chunk-composer-states |
| Ctrl+V / Alt+V — вставка картинки | ❌ | — | — | chunk-composer-images |
| Ctrl+L — очистить терминал | ❌ | — | — | chunk-commands |
| Ctrl+Z — suspend | ? | в keymap зарезервирован; обработчик не найден | — | — |
| Ctrl+X — leader (Ctrl+X b — смена бэкенда) | 🟡 | leader живёт только для баннера | — | chunk-live-banner-fallback |
| ? — оверлей шорткатов | ✅ | — | — | — |
| Left на пустом composer — Agents | ✅ | — | — | — |
| Enter / Shift+Enter — отправка / перенос | ✅ | — | — | — |
| PgUp/PgDn, колесо — прокрутка ленты | ✅ | — | — | — |
| Vim-клавиши (69 действий) — normal/operator/text-object/search | 🟡 | работают при включении, включить нечем | 8/51 (не вкл. 5) | chunk-composer-vim |

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
- Composer: Remote-картинки (URL): submit и нумерация (chunk-composer-images)
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
