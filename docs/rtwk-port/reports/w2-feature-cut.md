# Остаток W2 по фичам (черновик для человека)

Задача: w2-recut-draft, 2026-11-25. Повод: человек сказал «пакеты собраны неправильно» (2026-11-25).
Здесь остаток W2 разрезан по фичам, которые видит пользователь, а не по папкам.

Данные:
- tip `claude/tui-rtwk` 19a4b3f4b. Pending-тесты взяты из
  `tools/rtwk-port/map-full/pending-tests.tsv` на 0d2d4a0aa.
- Сверху наложены три ветки в работе, они считаются сделанными: doors-r3 e6272dcc5, w2-live a3a605750
  и w2-approvals d58b59eca. Их diff по tsv вычтен: 956 → 930.
- Строки взяты из `port-map.md` (раздел «Строки»), их статусы сверены с 365 коммитами 01741db4b..0d2d4a0aa.
- Для каждого pending-теста использованы владелец-пакет (`unpend/<pkg>.txt`) и причина.
  По ним тест отнесён к фиче или к корзине «вне фич».

## Синтез

| | Число |
|---|---|
| Pending после веток в работе | **930** |
| Из них в W2-пакетах и kit-seeds | 893 |
| Из них в не-W2 пакетах (inline-mode 12, startup-rest 19, resume-rest 5, voice 1) | 37 |
| Фич (M) | **46** |
| Тестов в фичах | 645 |
| Из них фича закрывает (pass или n/a с причиной) | **612** |
| Из них остаются pending под гэпом внутри фичи | 33 |
| Задач doors, общих для нескольких фич | 1 (`d-transcript-kit`) |
| W2-тестов вне фич (только гэп / W3 / решение человека) | 248 |

Каждая фича рассчитана на один прогон портера: ≤27 тестов, порядка 150-250 ходов.
Двери (doors) фича строит сама. Отдельная задача doors нужна только одна: kit транскрипта,
им пользуются 12 фич.

## 1. Инвентарь

### 1.1 Pending по пакетам (после веток в работе)

| Пакет | На tip | После веток | В фичи | Вне фич |
|---|---|---|---|---|
| w2-transcript | 198 | 194 | 176 | 18 (textarea 5, markdown 4, person 4, W3 4, gap-text 1) |
| w2-status | 148 | 148 | 124 | 24 (person 9, pets 6, voice 4, gap-text 4, resume 1) |
| w2-live | 152 | 147 | 34 | 113 (markdown 110, pets 3) |
| w2-approvals | 129 | 121 | 115 | 6 (gap-text) |
| w2-cells | 95 | 95 | 45 | 50 (inline S27 30, gap-text 9, markdown 8, voice 3) |
| w2-composer | 81 | 81 | 51 | 30 (textarea G65 и OSC 8) |
| kit-seeds | 72 | 68 | 63 | 5 (markdown 2, W3 3) |
| w2-commands | 34 | 30 | 28 | 2 (person 1, resume 1) |
| w2-settings | 9 | 9 | 9 | 0 |
| Не-W2 (inline-mode, startup-rest, resume-rest, voice) | 38 | 37 | 0 | 37 |
| **Итого** | **956** | **930** | **645** | **285** |

Handoff называет baseline 936 pending. Разница с 930: шесть тестов, которые ветки в работе
переводят из pending.

### 1.2 Не-✅ строки W2

152 строки на 01741db4b. По пакетам: transcript 27, commands 26, approvals 24, composer 21,
status 20, settings 16, cells 14, live 3, kit-seeds 1.

Строки, которые после 01741db4b закрыты целиком или по сути. Их статус в port-map отстаёт,
и при регенерации `map-full/build.py` он обновится сам.

| Строка (port-map.md) | Чем закрыта |
|---|---|
| 509 /diff, 614 статический pager | 96f99f56f |
| 505 /copy, 692 Ctrl+O, 556 picker (кроме payload) | 9741256cb |
| 458 и 693 Ctrl+G | 20da78bf0. Остался G67: Vim Replace + editor (тест в f-vim) |
| 496 и 508 /config, /tui | d5f083b3a |
| 476 /keymap | 378751512. Остался debug-инспектор, он в f-settings-lists |
| 529 /ps, 522 /mcp | ce7d4084e, 7429ecde8. Остаток в f-mcp-inventory-rest и f-unified-exec |
| 386 compaction | 001c5978e. Остаток replay в f-replay-history |
| 366 exploration | 6296f3eca (compact form) |
| 425, 504, 691 side | ca327bb9e, 334a99472, 201d8045c. Остаток в f-side-rest |
| 512 /daemon, 533 /subagents, 641 daemon | e158801b4 |
| 478 и 544 Windows sandbox | 276b18e72, 974289099 |
| 558 security setup | e606c5c27 |
| 472 /presets, 549 quick dials | 531255c66, 138830573 |
| 517 /debug-config | f7dcc7fc1 |
| 661, 662, 557 меню /usage и reset | e3ef52787. Остаток в f-usage-rest |
| 601 usage треда | 5576c470a, af5ba7c01 |
| 423 rate-limit prompts | 28b783c6c, 4141b3677. Остаток в f-banners-luna |
| 518, 519, 591 превью statusline/title | 71f664b15. Остаток в f-status-line |
| 624, 625 Find | 959cb608b, 2269bc7c5, 7a090cac8. Остаток в f-find |
| 622 select mode | 19aeda6ab, d8b3c5806. Остаток в f-select-mode |
| 623 copy mode | 6cb6a9bff. Остаток в f-copy-mode |
| 621 markdown copy | 6c00739e0. Остаток в f-copy-payload |
| 390, 688 F4 | 45d51db3c, с дельтой: фокус уходит на последнюю группу, а не на первую в окне (нужен G-new-usage-2) |
| 408, 681 Tab ставит команду в очередь | fcf56dcf5. Остаток в f-queue-commands |
| 454 parent-owned и shutdown | 23b58a857. Остаток в f-composer-states |
| 479, 482, 484, 552, 553, 560 | `Which::Experimental/Hooks/Config/Keymap` теперь Shown (`src/features/settings/shown.rs:28-34`). Часть причин в tsv устарела. Остаток в f-settings-lists |

Строки без pending-тестов ни в одной фиче. Их закрывает сверка в соседней фиче, либо они уходят из W2:

| Строка | Что с ней |
|---|---|
| 363, 631 клик по ссылке | Решение человека: link wrap проверяется тестом виджета. Сверку делает f-tool-activity |
| 382 часы в уведомлениях | Сверку делает f-warning-cells |
| 420 уведомления терминала | Сверку делает f-working-status |
| 459 мышь в composer | Сверку делает f-composer-states. Тестов нет: right-click paste в composer не тестируется в oracle-шардах |
| 651 provider switch | Сверку делает f-picker-rows |
| 730, 731 off-chat | W3 |
| 396, 521, 648 pets; 594, 649, 689, 501 voice; 628 сайдбар; 629 inline | W3/W4 |
| 630 ui-tree, 650 trust, 562 sweep («?») | Нужно решение, относятся ли они к W2 |

## 2. Учтённые хвосты (рефлексии, doors NOTES, handoff)

| Источник | Хвост | Куда |
|---|---|---|
| handoff w2-live | banners usage-cap | f-banners-luna |
| handoff w2-live | mcp_startup | f-mcp-inventory-rest |
| handoff w2-live | остаток plan-mode | f-plan-mode |
| handoff w2-live | seed шапки сессии, Ran vs Exited | f-live-active-cell |
| handoff w2-live | G61-G73 | вне фич (f-markdown, гэпы) |
| handoff w2-approvals | AsyncQuestions inline editor (~29 тестов) | f-async-questions-view + f-async-questions-drafts |
| handoff w2-approvals | гэпы decision cells | f-approval-decisions |
| handoff w2-approvals | goal: fake `thread/goal/get` | f-goal (строит дверь сам) |
| handoff w2-approvals | covered fact, WSL fact | f-approval-decisions |
| handoff w2-approvals | NO_HISTORY | X-person (дельта дизайна, решение человека) |
| handoff w2-approvals | custom prompt (13) | f-custom-prompt-review |
| handoff w2-approvals | verify pager | f-elicitation-dialogs |
| рефлексия w2-transcript (01a2107f) | F4: последняя группа вместо первой в окне | f-turn-details (под G-new-usage-2) |
| рефлексия w2-transcript | Find в select mode закрывается после accept | f-select-mode |
| рефлексия w2-transcript | страница листается по строкам текста, а не по display rows | f-follow-control (под G-new-sel-display-rows) |
| рефлексия w2-transcript | markdown copy: таблицы, несколько сообщений, неформатированные слова | f-copy-payload |
| рефлексия w2-transcript | в запросе Find нет editor chords (ctrl+x h) | f-find |
| рефлексия w2-transcript | 191 тест `transcript_view_*` ждёт layout и kit seeds | d-transcript-kit, затем фичи транскрипта |
| рефлексия w2-transcript | 4 recap-теста (transcript_view_1/5) | f-rename-recap (форма скрипта `tests/pages/recap_session.rs`) |
| рефлексия w2-status | ellipsis по словам с ссылочным … в StatusLine (de58abeca) | f-status-line (гэп для человека) |
| рефлексия w2-status | 18 pending по usage: смена плана, ошибка lookup, release-события keymap, palette fixtures, model-only `panel()` | f-usage-rest |
| рефлексия w2-commands (01a2102a) | recap: автозаголовок по первому сообщению (`core/src/features/rename` + `src/features/rename`, `on_user_message`, `structured_turn` Title) | f-rename-recap |
| рефлексия w2-commands | кадры worktree, `feedback_selection_popup` (G68) | f-feedback-worktree |
| рефлексия w2-commands | cloud notice | f-banners-luna |
| рефлексия w2-commands | permissions NO_*, preset recall | f-permissions-rest |
| doors-r2 NOTES | факты pane_footer (MODE/QUEUE/FLASH/LINK/PROVIDER через `thread/settings/updated`, AGENT, FIND, REPLACE) | f-footer-hints, f-status-line |
| doors-r2 NOTES | WARN «/warnings to view» или «f2 to view» | решение человека |
| doors-r2 NOTES | EXTENSION | решение человека (уже n/a) |
| doors-r2 NOTES | FOCUS | гэп rtwk |
| doors-r2 NOTES | burst feedback note | f-feedback-worktree |
| doors-r2 NOTES | 32x6 даёт списку 1 ряд | d-transcript-kit |
| doors-r3 NOTES | обёртка ряда в `transcript_pane`; ячейки exec/MCP/exploration/guardian читают `expanded` | f-turn-details |
| doors-r3 NOTES | dedup `find::repeat` | f-find |
| doors-r3 NOTES | 32x6 `sm_entry_*`: лист Ctrl+T или вид отдельно | решение владельца перед d-transcript-kit |

## 3. Фичи

Колонки таблицы:
- «Тесты»: сколько всего, в скобках сколько остаются pending под гэпом. Затем шарды.
  Полный список имён в приложении A. Селектор для nextest: `-E 'test(=<shard>::<name>)'` по списку.
- «Oracle»: файлы в `codex-rs/tui/src/`.
- «Посадка»: файлы в `codex-rs/tui-rtwk/` (существующие пути; `pages/session` = `src/pages/session`).
- «Прогонов»: везде 1. Пограничные фичи помечены `*`, для них в §3.3 указана точка раздела.

### 3.1 Транскрипт (после d-transcript-kit)

| Фича | Что видит пользователь | Строки | Тесты (шарды) | Oracle | Посадка | Гэпы | Прогонов |
|---|---|---|---|---|---|---|---|
| f-find | Поиск по транскрипту F3, подсветка, счётчик, футер поиска, `/` и n/N в pager | 624, 625, 615, 687 | 27 (0): tv_4 19, tv_1 4, pane_footer_1, pager, tv_2, tv_3 | transcript_view/search.rs, search_footer.rs, search_presentation.rs, pager_overlay.rs | pages/session/transcript/find.rs, find_footer.rs, pager.rs; src/widgets/finder.rs | editor chords в запросе (G-new-focus?) | 1* |
| f-select-mode | Выделение с клавиатуры (select mode) | 622 | 18 (0): tv_4 16, pager 2 | transcript_view/select_mode.rs, select_mode/ | transcript/select.rs, selection.rs | — | 1 |
| f-selection-keys | Расширение выделения клавишами, Ctrl+C по выделению | 618, 620 | 15 (0): tv_4 12, tv_1, tv_2, tv_5 | transcript_view/selection.rs, input.rs | transcript/selection.rs, copy_feedback.rs | — | 1 |
| f-mouse-units | Двойной и тройной клик (слово, строка, абзац), колесо, скорость колеса | 619, 626, 392 (колесо) | 15 (0): tv_2 6, tv_5 4, tv_3 2, tv_4 2, pager | transcript_view/selection.rs, input.rs | transcript/selection.rs, transcript_pane.rs | — | 1 |
| f-copy-mode | Режим копирования (лидер Ctrl+X) | 623 | 11 (0): tv_2 11 | transcript_view/copy_mode.rs | transcript/copy.rs | — | 1 |
| f-copy-payload | Копия как Markdown или plain: таблицы, несколько сообщений, feedback копии | 621, 556 (payload) | 15 (1): chat_commands_2 4, chat_commands_3 4, tv_1 2, tv_2 2, … | markdown_copy.rs, clipboard_copy.rs, chatwidget/copy_picker.rs | transcript/markdown_copy.rs, copy_feedback.rs; src/features/copy | G63 | 1 |
| f-history-pages | Подгрузка старых страниц истории, страницы в overlay, live tail в pager | 393, 627, 616 | 22 (0): tv_4 9, tv_3 4, pager 3, tv_5 3, tv_2 2, tv_1 | transcript_view/mutations.rs, pager_overlay.rs, chatwidget/replay.rs | transcript/history.rs, pager.rs; core/src/threads/replay.rs | — | 1 |
| f-follow-control | PageUp/PageDown, Home/End, «к последнему», Esc и Enter возвращают вниз | 392 | 21 (5): tv_2 12, tv_1 7, tv_5 2 | transcript_view/follow_control.rs, footer.rs | transcript/follow.rs, transcript_pane.rs | G63 ×5; display rows (G-new-sel-display-rows) | 1 |
| f-entries-tabs-styles | Записи транскрипта: табы, стилизованные спаны, скрытые записи | 614 (строки) | 15 (0): tv_5 13, tv_3 2 | transcript_view/text_tabs.rs, text.rs, text_logical.rs | transcript/item_text.rs, markdown_text.rs | — | 1 |
| f-entries-resize-live | Транскрипт при ресайзе и при растущем хвосте хода | 614 | 18 (0): tv_5 13, tv_1 3, tv_2, status | transcript_view/layout.rs, mutations.rs | transcript_pane.rs, transcript/sheet.rs | — | 1 |
| f-browse-prompts | Листание прошлых prompt'ов из транскрипта | 392 (часть) | 8 (0): tv_1 7, composer_1 | transcript_view/bookmark.rs, prompt_header.rs | pages/session/live/browse.rs | — | 1 |
| f-turn-details | Сворачивание деталей хода, раскрытие ячеек, закреплённая шапка prompt | 389, 394, 390 (дельта) | 7 (4): tv_3 4, tv_2 2, tv_5 | transcript_view/turn_details.rs, disclosure.rs, prompt_header.rs | transcript/turn_details.rs, cells/disclosure.rs, exec.rs, tool_call.rs, exploration.rs; sheets/approvals/guardian.rs | G-new-usage-2 ×4 (шапка) | 1 |

### 3.2 Остальные фичи

| Фича | Что видит пользователь | Строки | Тесты (шарды) | Oracle | Посадка | Гэпы | Прогонов |
|---|---|---|---|---|---|---|---|
| f-approval-decisions | Решение approval в транскрипте, заголовок apply_patch, баннер ожидающих approvals, keymap seeds | 377, 569, 582 | 18 (0): chat_approvals_1 6, pane_footer_2 4, cells_4 3, approvals_1 2, approvals_2 2, chat_status_1 | history_cell/approvals.rs, bottom_pane/approval_overlay.rs, pending_thread_approvals.rs | sheets/approvals/decision_cell.rs, banner.rs, keys.rs; cells/decision.rs; core/src/threads/decision.rs | — | 1 |
| f-async-questions-view | Асинхронные вопросы: форма, пустые ответы, skip, мышь | 578, 579, 580 | 20 (0): approvals_1 12, chat_approvals_2 8 | bottom_pane/async_questions/, request_user_input/, questions.rs | sheets/approvals/user_input.rs, form.rs; cells/request_input.rs | — | 1 |
| f-async-questions-drafts | Черновики ответов, навигация, replay, общая очередь с queued messages | 576, 577, 581 | 14 (0): chat_approvals_2 10, approvals_1 3, approvals_2 | async_questions/, history_cell/async_replies.rs, chatwidget/questions.rs | sheets/approvals/user_input.rs, form.rs; core/src/threads/followups.rs | — | 1 |
| f-custom-prompt-review | Свой промпт, review-попапы | 562 (часть) | 18 (0): small_dialogs_1 13, chat_live_5 4, chat_live_2 | bottom_pane/custom_prompt_view.rs, chatwidget/review_popups.rs, review.rs | src/features/prompt.rs; sheets/dialogs | — | 1 |
| f-elicitation-dialogs | Диалоги app link, user verification, MCP elicitation, tool suggestion; verify pager | 642, 644 | 13 (1): small_dialogs_1 9, approvals_2 3, chat_live_1 | bottom_pane/app_link_view.rs, user_verification.rs, mcp_server_elicitation.rs | sheets/dialogs, sheets/approvals/dialog.rs | G72 | 1 |
| f-queue-commands | Очередь slash- и shell-команд во время хода, steer, Tab | 408, 410, 681 | 15 (1): chat_commands_2 5, chat_commands_1 3, chat_commands_3 2, … | chatwidget/input_queue.rs, input_submission.rs, bottom_pane/pending_input_preview.rs | pages/session/composer/queue.rs; core/src/threads/queue.rs | G73 | 1 |
| f-composer-states | Состояния composer: disabled, reconnect, blockquote-вставка, блок при недоступной модели | 454, 413, 459 | 17 (0): 9 шардов (composer_1-4, chat_commands_1/3, chat_live_1/4, tv_1) | chatwidget/reconnect.rs, input_restore.rs, bottom_pane/chat_composer.rs | pages/session/composer/mod.rs, editing.rs; src/services/backend/reconnect.rs | — | 1 |
| f-image-attach | Картинки из буфера Ctrl+V/Alt+V, путь файла превращается в картинку | 452, 694, 442 | 14 (0): chat_commands_1 6, composer_4 4, tv_1 2, … | chatwidget/image_submission.rs, bottom_pane/chat_composer.rs | pages/session/composer/editing.rs, burst.rs; cells/user.rs | — | 1 |
| f-apps-mentions | Упоминания $ skills, плагины, apps, inline-аргументы | 450, 440 | 19 (7): composer_popups 11, composer_1 2, composer_2 2, chat_popups_3 2, … | bottom_pane/mentions_v2/, skill_popup.rs, prompt_args.rs, chatwidget/connector_mentions.rs | pages/session/composer/mentions.rs, suggest.rs | G68 ×4, G-new-1 ×3 | 1 |
| f-history-search | Поиск по истории composer (Ctrl+R) | — (хвост w2-composer) | 6 (3): composer_popups 4, pane_footer_1 2 | bottom_pane/chat_composer_history.rs, chat_composer_history/ | pages/session/composer/search.rs | G-new-1, G78 ×2 | 1 |
| f-startup-draft | Черновик из аргументов запуска, ранняя очередь | 460 | 10 (0): chat_commands_1 5, chat_commands_3 4, chat_live_4 | startup_draft*.rs, chatwidget/startup_submission.rs | pages/session/composer/mod.rs; core/src/threads/composer.rs | — | 1 |
| f-vim | Vim: операторы, replace, yank, dot-repeat | 456, 457 | 25 (1): composer_4 23, composer_3, textarea | bottom_pane/textarea/ (vim), chat_composer.rs | pages/session/composer/vim.rs; src/widgets/composer_draft/ | G67 | 1* |
| f-plan-mode | Plan mode: скрытая shell-вставка, уведомления о смене модели или effort | 443, 547 | 11 (2): chat_live_4 6, tv_1 2, … | chatwidget/plan_implementation.rs | pages/session/live/plan_mode.rs | G-mask ×2 (без имени) | 1 |
| f-status-line | Статус-строка и превью /statusline, /title | 591, 518, 519 | 17 (3): pane_footer_2 11, pane_footer_1 3, chat_status_2 2, small_dialogs_2 | bottom_pane/status_line.rs, status_line_setup.rs, status_surface_preview.rs | pages/session/footer/status_rows.rs; src/widgets/status_line.rs; pages/session/status_surfaces.rs | G48 ×3; ellipsis по словам | 1 |
| f-footer-hints | Подсказки футера, flash, override подсказки | 598, 638 | 11 (0): pane_footer_2 6, pane_footer_1 4, composer_4 | bottom_pane/footer.rs, footer/, shortcut_hud.rs | pages/session/footer/hints.rs, flash.rs, indicators.rs | WARN-слова (решение) | 1 |
| f-working-status | Индикатор хода «Working…», детали, прерывание | 420 | 15 (0): chat_status_2 10, chat_status_1 5 | bottom_pane/effort_status_line.rs, effort_ignition.rs, chatwidget/status_state.rs | pages/session/footer/effort.rs, effort_frames.rs, lead.rs | — | 1 |
| f-banners-luna | Баннеры usage-cap и rate-limit, резерв Luna, cloud notice | 416, 423, 605, 664 | 14 (0): pane_footer_2 9, chat_live_2 5 | bottom_pane/actionable_banner.rs, chatwidget/backend_banners.rs, luna_reserve_*.rs, rate_limits.rs | pages/session/banners.rs, live/limits.rs | — | 1 |
| f-usage-rest | /usage: смена плана, ошибка lookup, палитры, панель модели, billing | 516, 660, 663, 665, 666 | 23 (1): usage 18, chat_live_6 5 | chatwidget/usage.rs, usage_history.rs, reset_credits.rs, thread_usage.rs | src/features/usage_menu, thread_usage; core/src/features/usage_menu | G-new-usage-1 | 1* |
| f-goal | Goal: меню, редактирование, статус, resume-paused | 545 | 11 (0): chat_popups_1 6, chat_commands_2 2, … | chatwidget/goal_menu.rs, goal_status.rs | src/features/goal, core/src/features/goal; core/src/threads/goal.rs | — | 1 |
| f-settings-lists | Experimental, skills, popup хуков, keymap-инспектор, memories, export | 479, 482, 552, 553, 559, 560, 561, 476 (остаток) | 13 (0): chat_popups_4 7, chat_commands_3 4, … | bottom_pane/experimental_features_view.rs, skills_toggle_view.rs, hooks_browser_view.rs, memories_settings_view.rs, chatwidget/keymap_picker.rs | src/features/settings, export | — | 1 |
| f-feedback-worktree | Feedback: выбор, consent, note burst; кадры worktree | 527, 554, 647, 492, 548 | 11 (0): chat_popups_4 8, small_dialogs_1 3 | bottom_pane/feedback_view.rs, feedback_note_view.rs, chatwidget/worktree_picker.rs | src/features/feedback; sheets/select | — | 1 |
| f-picker-rows | Ряды пикеров: вкладки, чекбоксы, колонки, узкая ширина; провайдер модели | 653, 541, 651 | 16 (4): picker 9, small_dialogs_2 5, approvals_2, pane_footer_2 | bottom_pane/picker_rows.rs, selection_*.rs, multi_select_picker.rs | src/widgets/picker | G54, G39b ×2, G56 | 1 |
| f-unified-exec | Unified exec: фоновые терминалы, wait-streak, write_stdin | 367, 571, 529 (остаток) | 14 (0): chat_approvals_1 10, cells_3 2, cells_4 2 | history_cell/exec.rs, bottom_pane/unified_exec_footer.rs, chatwidget/exec_state.rs | cells/exec.rs, tool_rows.rs; core/src/threads/processes.rs | — | 1 |
| f-tool-activity | Computer activity, dynamic calls, collab и subagents, ячейки картинок | 372, 374, 363 (сверка) | 20 (0): chat_status_1 8, chat_live_1 4, chat_approvals_1 3, chat_status_3 3, … | history_cell/computer_activity.rs, dynamic.rs, chatwidget/dynamic_activity.rs, activity_groups.rs | cells/computer_activity.rs, collab.rs, generated_image.rs, viewed_image.rs; core/src/threads/collab.rs, activity.rs | — | 1 |
| f-live-active-cell | Активная ячейка хода: шапка сессии, Ran или Exited, «Worked for», малые ширины, отказ от blossom | 365, 387, 397 | 22 (0): chat_live_1 11, cells_5 4, chat_approvals_1 2, … | chatwidget/session_header.rs, streaming.rs, history_cell/session.rs, separators.rs | cells/session_info.rs, separator.rs, exec.rs; pages/session/blossom | — | 1 |
| f-hooks-live | Ячейки хуков во время хода | — (хвост w2-cells) | 14 (0): chat_status_2 14 | chatwidget/hooks.rs, hook_lifecycle.rs, history_cell/hook_cell.rs | cells/hook.rs | — | 1 |
| f-warning-cells | Ячейки предупреждений, /warnings | 637, 382 (сверка) | 10 (0): chat_status_1 3, cells_5 2, chat_status_2 2, … | history_cell/warnings.rs, bottom_pane/warnings_view.rs | cells/warning.rs | — | 1 |
| f-safety-misalignment | Safety buffering: повтор, подтверждение; остановка misalignment | 417, 424 | 9 (0): chat_live_1 6, chat_live_3 3 | chatwidget/safety_buffering.rs, safety_recourse.rs, misalignment_policy.rs | cells/safety.rs; core/src/threads/safety | — | 1 |
| f-guardian-daybreak | Guardian review, /daybreak | 573, 473 | 7 (0): chat_approvals_2 4, … | history_cell/approvals.rs (guardian), chatwidget/protocol_requests.rs | sheets/approvals/guardian.rs; core/src/threads/daybreak.rs | — | 1 (в паре) |
| f-rename-recap | Recap: автозаголовок по первому сообщению, строка «Forked thread» | 384 | 9 (0): tv_5 3, chat_live_1 3, … | chatwidget/recap.rs, history_cell/session.rs | src/features/rename, recap; core/src/features/rename, recap | — | 1 |
| f-replay-history | Replay истории: compaction, решения при resume | 386 (остаток) | 6 (0): chat_live_2 3, chat_approvals_1 2, cells_2 | chatwidget/replay.rs, compaction.rs | core/src/threads/replay.rs, compaction.rs | — | 1 (в паре) |
| f-mcp-inventory-rest | Инвентарь MCP, mcp_startup | 369, 370 | 4 (0): cells_3 2, cells_4 2 | history_cell/mcp.rs, chatwidget/mcp_startup.rs | src/features/mcp_inventory; cells/tool_call.rs | — | 1 (в паре) |
| f-side-rest | Side: остаток (подпись контекста и т.п.) | 604 | 4 (0): chat_live_5 3, chat_live_1 | chatwidget/side.rs | src/features/side, core/src/features/side | — | 1 (в паре) |
| f-permissions-rest | Permissions: NO_*, повтор пресета | 543 | 3 (0): chat_popups_1 2, chat_popups_2 | chatwidget/permission_popups.rs, permissions_menu.rs | src/features/settings (permissions) | — | 1 (в паре) |

Каждая фича закрывает и kit-seeds своих тестов: 63 из 68 тестов kit-seeds распределены по фичам.
Своих fake-обработчиков фича добавляет в `src/services/backend/fake.rs` отдельным блоком.

### 3.3 Пограничные фичи (`*`) и их точки раздела

| Фича | Тестов | Раздел, если первый прогон не уложится |
|---|---|---|
| f-find | 27 | транскрипт (tv_4 19) / pager `/`, n/N и футер поиска (8) |
| f-vim | 25 | операторы и текстовые объекты / replace, yank, dot-repeat |
| f-usage-rest | 23 | панель /usage (usage 18) / chat_live_6 billing (5) |
| f-history-pages, f-live-active-cell | 22 | По шарду. Обе фичи разнородны, но тесты у них независимы |

Раздел означает две фичи, каждая доводится до DONE. Сдача «наполовину» не допускается.

### 3.4 Общая задача doors

**d-transcript-kit**. Нужна до фич транскрипта: f-find, f-select-mode, f-selection-keys, f-mouse-units,
f-copy-mode, f-copy-payload, f-history-pages, f-follow-control, f-entries-tabs-styles,
f-entries-resize-live, f-browse-prompts, f-turn-details.

Что делает:
- Монтирует вид транскрипта отдельно или лист Ctrl+T на размерах oracle.
  До запуска задачи владелец решает, какой вариант: doors-r3, п. 8, `sm_entry_*` 32x6.
  Сейчас 32x6 даёт списку 1 ряд.
- Добавляет формы entry-скриптов: tabs, styled spans, plan, reasoning, recap, exec entry, hidden entries.
- Добавляет шаг commit живого хвоста и шаги page-join / older-page для истории.
- Делает dedup `find::repeat`.

Файлы:
- `tests/kit/{seed,input,states,asserts}.rs`
- `tests/oracle_snap/*`
- `core/src/fixtures/script.rs`, `core/src/fixtures/builders.rs`
- `src/services/backend/fake.rs` (только thread/history)

Остальные общие файлы — это реестры с одной строкой на фичу: `core/src/features/mod.rs` `feature_list!`,
`src/features/mod.rs` `sheet_list!`, `key_actions/mod.rs` `action_list!`, `cells/mod.rs` `cell_list!`,
`transcript/mod.rs` `mode_list!`. Отдельная задача doors для них не нужна.
Фича добавляет одну строку, конфликт при landing решается механически.

## 4. Порядок: по три параллельно, с непересекающимися файлами

Правила раскладки:
- В одной волне не больше одной фичи на каждый из кластеров `transcript/selection.rs`,
  `composer/mod.rs`, `footer/*`, `sheets/approvals/*`, `sheets/dialogs`.
- f-turn-details идёт после f-unified-exec и f-mcp-inventory-rest: она переписывает чтение `expanded` в их ячейках.

| Волна | Слот A | Слот B | Слот C |
|---|---|---|---|
| 0 | **d-transcript-kit** | f-vim (composer/vim.rs, widgets/composer_draft) | f-settings-lists (features/settings) |
| 1 | f-find (find.rs, find_footer.rs, pager.rs, widgets/finder.rs) | f-approval-decisions (sheets/approvals/decision_cell, banner; cells/decision.rs) | f-queue-commands (composer/queue.rs, core/threads/queue.rs) |
| 2 | f-select-mode (select.rs, selection.rs) | f-async-questions-view (sheets/approvals/user_input, form) | f-status-line (footer/status_rows, widgets/status_line) |
| 3 | f-history-pages (history.rs, core/threads/replay.rs) | f-unified-exec (cells/exec.rs, core/threads/processes.rs) | f-apps-mentions (composer/mentions, suggest) |
| 4 | f-mouse-units (selection.rs, transcript_pane.rs) | f-async-questions-drafts (sheets/approvals/user_input, core/threads/followups) | f-usage-rest (features/usage_menu, thread_usage) |
| 5 | f-selection-keys (selection.rs, copy_feedback.rs) | f-custom-prompt-review (features/prompt.rs, sheets/dialogs) | f-image-attach (composer/editing, burst; cells/user) |
| 6 | f-copy-mode (copy.rs) | f-elicitation-dialogs (sheets/dialogs, approvals/dialog) | f-composer-states (composer/mod.rs, backend/reconnect.rs) |
| 7 | f-copy-payload (markdown_copy.rs, features/copy) | f-tool-activity (cells/computer_activity, collab, *_image) | f-startup-draft (composer/mod.rs, core/threads/composer.rs) |
| 8 | f-follow-control (follow.rs, transcript_pane.rs) | f-live-active-cell (cells/session_info, separator; blossom) | f-footer-hints (footer/hints, flash, indicators) |
| 9 | f-entries-tabs-styles (item_text.rs, markdown_text.rs) | f-warning-cells (cells/warning.rs) | f-banners-luna (banners.rs, live/limits.rs) |
| 10 | f-entries-resize-live (transcript_pane.rs, sheet.rs) | f-goal (features/goal, core/threads/goal.rs) | f-guardian-daybreak (sheets/approvals/guardian, core/threads/daybreak.rs) |
| 11 | f-browse-prompts (live/browse.rs) | f-hooks-live (cells/hook.rs) | f-plan-mode (live/plan_mode.rs) |
| 12 | f-history-search (composer/search.rs) | f-safety-misalignment (cells/safety.rs, core/threads/safety) | f-working-status (footer/effort*, lead.rs) |
| 13 | f-feedback-worktree (features/feedback, sheets/select) | f-picker-rows (widgets/picker) | f-mcp-inventory-rest + f-replay-history (features/mcp_inventory, cells/tool_call; core/threads/replay, compaction) |
| 14 | f-turn-details (turn_details.rs, cells/disclosure + exec/tool_call/exploration, approvals/guardian) | f-side-rest + f-permissions-rest | f-rename-recap (features/rename, recap) |

Итого 15 волн: 1 задача doors и 46 фич, из них 4 мелкие идут парами в одном прогоне.

Во время волны 0 f-vim и f-settings-lists не трогают `tests/kit/*` и `core/src/fixtures/script.rs`.
Эти файлы в волне 0 принадлежат d-transcript-kit.

Первые три фичи, которые можно запустить сейчас без задачи doors (файлы не пересекаются):
**f-vim**, **f-approval-decisions**, **f-queue-commands**.
Их можно поставить вместо волны 0, если решение владельца по d-transcript-kit задерживается.

## 5. Итоги

| | Тестов | Под гэпом |
|---|---|---|
| Пул W2 и kit-seeds после веток в работе | 893 | 221 |
| В 46 фичах | 645 | 33 |
| **Фичи закрывают (pass или n/a с причиной)** | **612** | — |
| Остаются pending под гэпом внутри фич | 33 | 33 |
| W2 вне фич | 248 | 188 |
| Не-W2 пакеты (inline-mode, startup-rest, resume-rest, voice) | 37 | 19 |

После всех 46 фич pending станет 930 − 612 = **318**:

| Куда | Тестов |
|---|---|
| Под гэпом rtwk или S27 (33 в фичах, 188 W2 вне фич, 19 не-W2) | 240 |
| Ждут решения человека | 14 |
| W3 без гэпа: pets 11, voice 9, resume 4, inline 8 | 32 |
| Markdown, textarea, gap-text без явного гэпа: кандидаты на n/a или на новый гэп | 14 |
| Не-W2 пакеты без гэпа | 18 |

### 5.1 Вне фич (W2) и почему

| Корзина | Тестов | Под гэпом | Почему вне фич |
|---|---|---|---|
| Markdown-рендер (бывший f-markdown) | 124 | 112 | Гэпы rtwk (у теста может быть несколько): G73 83, G61 27, G64 5, G62 4, G63 и др. Остальные 12: list spacing ×2 (реальное поведение, нет консолидации списка при завершении хода), adaptive_wrap URL ×4 (как G72), controller state ×2 и growing fences D5 (кандидаты на n/a), initial indent, file citations OSC 8, windows paths (kit). Фича станет возможной, когда закроют G73 и G61 |
| Textarea composer | 35 | 34 | G65 26 и OSC 8-ссылки в composer («rtwk-editing has no hyperlink annotation», около 8). Ещё 1: нет seed подсветки |
| Перенос текста | 20 | 19 | G49, G72, G39b, G44, G48, G51, G74, G75, G76, G-syntax, G-new (dim indent, HTML block, bare URL) |
| Решения человека | 14 | 0 | См. 5.3 |
| Inline-режим S27 (W3/W4) | 31 | 23 | Форма scrollback, а не alt-screen |
| Pets (W3) | 11 | 0 | Пакет pets |
| Voice (W3) | 9 | 0 | Пакет voice |
| Resume (W3) | 4 | 0 | resume-rest |

### 5.2 Гэпы без имени: завести строки в rtwk-gaps.md

| Метка в причинах | Где | Тестов |
|---|---|---|
| G-mask (нет каталога collaboration-mask) | f-plan-mode 2, inline-mode 1 | 3 |
| G-syntax (подсветка кода) | перенос текста | 1 |
| G-new: dim indent | перенос текста | 1 |
| G-new: HTML block | перенос текста | 1 |
| G-new: bare URL | перенос текста | 1 |
| OSC 8-ссылки в composer (`rtwk-editing has no hyperlink annotation`) | textarea | около 8 |

Ещё не заведены гэпы, которые человек должен назвать: display-rows query;
ListController `ensure_row_visible` / `item_extent` / `visible_range` (G-new-usage-2 уже назван);
Text word ellipsis с ссылочным `…`.

### 5.3 Решения человека (14 тестов и 3 вопроса)

| Решение | Тесты |
|---|---|
| Выделение reverse-video против accent bold | small_dialogs_1 `wide_titles`, small_dialogs_2 `selected_event_rows` |
| В тестовом чате oracle нет thread id | chat_status_3 `hardcoded_only` ×3 |
| NO_HISTORY: в чате oracle нет треда | chat_popups_2 `permissions_selection_emits_history`, `history_snapshot` ×2 |
| Ряд подсказок при включённой навигации агентов | pane_footer_2 `status_only`, `status_and_queued`, `status_with_details`, `status_indicator_visible`, `queued_messages_visible`; chat_status_2 `status_line_estimated_thread_cost` |

Вопросы:
1. 32x6 `sm_entry_*`: лист Ctrl+T или вид отдельно. Блокирует d-transcript-kit.
2. WARN в футере: «/warnings to view» или «f2 to view».
3. 12 оставшихся markdown-тестов: n/a или ждать G73/G61.

## Приложение A. Тесты по фичам

Сгенерировано из pending-tests.tsv на 0d2d4a0aa с наложенными ветками в работе.
`(gap)` — тест остаётся pending под гэпом, который назван в его причине.

### f-approval-decisions (18, под гэпом 0)

- `approvals_1::exec_history_cell_wraps_with_two_space_indent`
- `approvals_1::network_exec_prompt_title_includes_host`
- `approvals_2::render_multiple_threads_snapshot`
- `approvals_2::render_single_thread_snapshot`
- `cells_4::prefixed_wrapped_history_cell_does_not_split_url_like_token`
- `cells_4::prefixed_wrapped_history_cell_height_matches_wrapped_rendering`
- `cells_4::prefixed_wrapped_history_cell_indents_wrapped_lines`
- `chat_approvals_1::app_server_write_stdin_approval_renders_terminal_input`
- `chat_approvals_1::apply_patch_events_emit_history_cells`
- `chat_approvals_1::apply_patch_manual_approval_adjusts_header`
- `chat_approvals_1::apply_patch_manual_flow_snapshot`
- `chat_approvals_1::exec_approval_emits_proposed_command_and_decision_history`
- `chat_approvals_1::network_exec_approval_history_describes_canceled_host_request`
- `chat_status_1::status_widget_and_approval_modal_snapshot`
- `pane_footer_2::approval_request_is_delayed_after_recent_typing`
- `pane_footer_2::composer_shown_after_denied_while_task_running`
- `pane_footer_2::delayed_approval_shortcut_works_after_idle_deadline`
- `pane_footer_2::overlay_not_shown_above_approval_modal`

### f-apps-mentions (19, под гэпом 7)

- `chat_commands_1::task_mention_submission_and_transcript_preserve_the_visible_title`
- `chat_popups_3::apps_notification_update_excludes_inaccessible_apps_from_mentions`
- `chat_popups_3::apps_popup_keeps_existing_full_snapshot_while_partial_refresh_loads`
- `chat_popups_4::apps_refresh_failure_without_full_snapshot_falls_back_to_installed_apps`
- `composer_1::plugin_at_mentions_render_with_plugin_accent_snapshot` (gap)
- `composer_1::recalled_plugin_at_mentions_keep_plugin_accent_style` (gap)
- `composer_2::slash_popup_model_clipping_snapshot` (gap)
- `composer_2::task_mention_popup_and_selected_composer_snapshots` (gap)
- `composer_popups::app_command_popup_snapshot` (gap)
- `composer_popups::changing_filter_resets_selection_after_scrolling` (gap)
- `composer_popups::command_popup_wrap_boundary_preserves_following_choice` (gap)
- `composer_popups::fallback_completions_keep_the_draft_cursor_and_footer_anchored`
- `composer_popups::live_picker_frame_snapshots`
- `composer_popups::long_mentions_leave_room_for_distinguishing_parent_paths`
- `composer_popups::lowercase_expansion_preserves_match_ranking_and_highlighting`
- `composer_popups::mention_menu_reserves_height_above_composer_and_owns_cursor_over_interactive_footer`
- `composer_popups::mention_scopes_overlay_history_without_moving_composer_or_footer`
- `composer_popups::skill_completion_attaches_typed_punctuation_snapshot`
- `composer_popups::unavailable_side_conversation_command_is_shown_only_when_searched`

### f-async-questions-drafts (14, под гэпом 0)

- `approvals_1::arrival_preserves_vim_undo_and_navigation_flushes_buffered_input`
- `approvals_1::question_inline_history_search`
- `approvals_1::question_navigation_resets_history_recall`
- `approvals_2::questions_flush_buffered_typing_and_record_activity`
- `chat_approvals_2::question_drafts_survive_navigation_and_snapshot_replay`
- `chat_approvals_2::question_history_search_uses_updated_bindings`
- `chat_approvals_2::question_inputs_support_vim_editing_and_search`
- `chat_approvals_2::question_queue_pop_becomes_an_ordinary_composer_draft_and_clears_questions_on_send`
- `chat_approvals_2::question_turn_end_appends_open_drafts_in_order_once`
- `chat_approvals_2::question_turn_end_escapes_command_drafts_before_submission`
- `chat_approvals_2::question_turn_end_keeps_multiline_question_and_answer_as_text`
- `chat_approvals_2::question_turn_end_recovers_after_interruption_restores_queued_input`
- `chat_approvals_2::question_turn_end_recovers_collapsed_drafts_on_completion_and_failure`
- `chat_approvals_2::questions_and_queued_messages_share_the_resolved_shortcut`

### f-async-questions-view (20, под гэпом 0)

- `approvals_1::existing_cross_context_keymaps_load_without_misleading_submit_hints`
- `approvals_1::long_prompt_keeps_the_active_input_visible`
- `approvals_1::markdown_question_preserves_formatting_and_link_destinations`
- `approvals_1::named_choices_share_selection_while_other_keeps_its_inline_cursor`
- `approvals_1::question_choice_remaps_paging_and_inline_feedback`
- `approvals_1::question_options_reserve_wrapped_height_at_the_width_boundary`
- `approvals_1::question_other_excludes_clipped_options_at_the_width_boundary`
- `approvals_1::question_other_remains_visible_at_nine_columns_and_distinguishes_named_other`
- `approvals_1::question_wrapped_options_and_other_share_the_text_indent`
- `approvals_1::question_wrapped_other_shares_the_text_indent`
- `approvals_1::selected_other_renders_as_a_dim_placeholder`
- `approvals_1::wrapped_question_url_keeps_its_complete_destination`
- `chat_approvals_2::accepted_question_answer_uses_existing_delivery_and_keeps_main_draft`
- `chat_approvals_2::async_question_markdown_matches_loaded_history`
- `chat_approvals_2::async_questions_reject_blank_answers_but_allow_explicit_skip`
- `chat_approvals_2::desktop_async_answer_dismisses_only_its_question_and_preserves_the_other_draft`
- `chat_approvals_2::expanded_question_yields_mouse_to_terminal_for_links`
- `chat_approvals_2::question_editor_keeps_working_status_and_queued_messages_visible`
- `chat_approvals_2::selected_answers_preserve_long_labels_and_reject_oversized_submissions`
- `chat_approvals_2::single_question_spacing_with_working_status`

### f-banners-luna (14, под гэпом 0)

- `chat_live_2::backend_banner_changed_remedy_keeps_fallback_until_applicable_replacement`
- `chat_live_2::backend_banner_invalid_content_and_absence_restore_fallback`
- `chat_live_2::luna_reserve_prompt_preserves_the_composer_palette_on_exit`
- `chat_live_2::luna_reserve_status_tracks_the_active_model`
- `chat_live_2::luna_reserve_usage_survives_banner_dismissal_and_typing_during_a_turn`
- `pane_footer_2::backend_banner_snapshots_and_numbered_actions`
- `pane_footer_2::banner_copy_preserves_wrapped_url_destinations`
- `pane_footer_2::banner_hidden_during_task_waits_for_redraw_before_accepting_dismissal`
- `pane_footer_2::escape_closes_shortcut_help_before_inline_banner`
- `pane_footer_2::hidden_banner_preserves_input_before_render_and_after_resize`
- `pane_footer_2::information_banner_preserves_input_and_honors_dismissal`
- `pane_footer_2::inline_banner_snapshot`
- `pane_footer_2::partially_clipped_banner_preserves_hidden_action_shortcuts`
- `pane_footer_2::promoted_banner_uses_picker_hints_and_preserves_the_draft`

### f-browse-prompts (8, под гэпом 0)

- `composer_1::footer_mode_snapshots`
- `transcript_view_1::browsing_details_use_the_remapped_chord_without_cancelling_preview`
- `transcript_view_1::browsing_footer_adapts_to_width`
- `transcript_view_1::cancel_restores_both_positions_after_resize_and_pagination`
- `transcript_view_1::double_escape_browses_prompts_without_reverting_and_explains_editing`
- `transcript_view_1::escape_restores_reading_origin_after_details_navigation_and_resize`
- `transcript_view_1::following_origin_returns_to_new_output_in_original_presentation`
- `transcript_view_1::owned_details_keep_the_composer_cursor_and_screen`

### f-composer-states (17, под гэпом 0)

- `chat_commands_1::reconnect_queues_enter_and_resumes_ordinary_next_turn_submission`
- `chat_commands_1::reconnect_resumes_unsent_input_and_reconciles_confirmed_submissions`
- `chat_commands_3::slash_cd_rejects_pending_input_and_unsupported_session_ownership`
- `chat_commands_3::slash_clear_after_ctrl_c_keeps_stashed_draft_recallable`
- `chat_commands_3::slash_copy_picker_remains_available_from_parent_owned_threads`
- `chat_live_1::owned_bottom_pane_preserves_draft_cursor_and_read_only_notice`
- `chat_live_1::provider_error_preserves_draft_and_survives_thread_navigation`
- `chat_live_4::submit_user_message_blocks_when_thread_model_is_unavailable`
- `composer_1::light_terminal_palette_renders_light_composer_snapshot`
- `composer_2::ascii_burst_treats_enter_as_newline_even_when_parent_owned`
- `composer_3::agents_navigation_hint_snapshots`
- `composer_3::input_disabled_ignores_keypresses_and_hides_cursor`
- `composer_4::blockquote_paste_large_expands_quoted_text_on_submit`
- `composer_4::offline_delete_reveals_the_edited_row_without_moving_the_caret`
- `composer_4::unavailable_thread_expands_pasted_commands_without_dispatching`
- `transcript_view_1::external_writer_escape_returns_to_command_center_without_editing`
- `transcript_view_1::offline_find_closes_before_the_next_ctrl_c_quits`

### f-copy-mode (11, под гэпом 0)

- `transcript_view_2::copy_highlight_fills_a_block_across_blank_and_wrapped_rows`
- `transcript_view_2::copy_navigation_keeps_bottom_context_when_prompt_header_appears`
- `transcript_view_2::copy_navigation_leaves_context_and_preserves_the_center_band`
- `transcript_view_2::copy_resize_keeps_whole_user_padding_inside_context`
- `transcript_view_2::copy_reveals_offscreen_block_with_context`
- `transcript_view_2::marker_only_prompt_is_a_copy_target`
- `transcript_view_2::navigate_copy_resize_and_exit`
- `transcript_view_2::rendered_fenced_table_has_a_block_highlight`
- `transcript_view_2::rendered_mermaid_copies_source`
- `transcript_view_2::user_messages_support_vim_navigation_without_an_assistant_response`
- `transcript_view_2::whole_user_turn_highlight_includes_bottom_padding`

### f-copy-payload (15, под гэпом 1)

- `chat_commands_2::slash_copy_picker_copies_status_fields_and_preserves_source_after_copying`
- `chat_commands_2::slash_copy_picker_uses_completed_commentary_during_active_turn`
- `chat_commands_2::slash_copy_status_omits_missing_fields_and_copies_full_directory`
- `chat_commands_2::slash_copy_status_yields_to_later_turns_and_commands_even_after_refresh`
- `chat_commands_3::copy_shortcut_can_be_remapped`
- `chat_commands_3::copy_shortcut_submits_markdown_and_reports_completion_once`
- `chat_commands_3::slash_copy_reports_when_no_agent_response_exists`
- `chat_commands_3::transcript_copy_feedback_stays_in_the_footer_without_history_or_interrupts`
- `chat_popups_1::copy_export_picker_custom_keys_preserve_payloads_and_composer_draft`
- `pane_footer_1::copying_from_find_preserves_the_query_above_selection_feedback`
- `transcript_view_1::feedback_releases_navigation_targets_and_expiry_restores_them` (gap)
- `transcript_view_1::pending_copy_feedback_survives_expiry_and_selection_replacement`
- `transcript_view_2::copy_completion_keeps_payload_count_and_follow_intent`
- `transcript_view_2::copying_a_disabled_fenced_table_uses_the_original_body`
- `transcript_view_4::copy_shortcuts_clear_selection_only_after_confirmed_delivery`

### f-custom-prompt-review (18, под гэпом 0)

- `chat_live_2::replayed_nested_review_prompts_do_not_render_or_seed_composer_history`
- `chat_live_5::review_branch_picker_escape_navigates_back_then_dismisses`
- `chat_live_5::review_commit_picker_shows_subjects_without_timestamps`
- `chat_live_5::review_custom_prompt_escape_navigates_back_then_dismisses`
- `chat_live_5::review_queues_user_messages_snapshot`
- `small_dialogs_1::background_prefill_preserves_pasted_text`
- `small_dialogs_1::background_prefill_preserves_replace_backspace_recovery`
- `small_dialogs_1::background_prefill_preserves_typing_even_after_the_text_is_deleted`
- `small_dialogs_1::background_prefill_requires_a_matching_unedited_prompt`
- `small_dialogs_1::background_prefill_survives_cursor_movement`
- `small_dialogs_1::delayed_enter_after_typing_submits`
- `small_dialogs_1::escape_without_vim_cancels_prompt`
- `small_dialogs_1::paste_burst_newline_after_tab_does_not_submit`
- `small_dialogs_1::paste_burst_newline_does_not_submit_short_first_line`
- `small_dialogs_1::picker_prompt_keeps_multiline_cursor_and_escape_hint_visible_after_resize`
- `small_dialogs_1::vim_insert_cursor_tracks_mode_and_normal_mode_commands`
- `small_dialogs_1::vim_insert_escape_clears_paste_burst_before_enter`
- `small_dialogs_1::vim_prompt_hint_tracks_escape_behavior`

### f-elicitation-dialogs (13, под гэпом 1)

- `approvals_2::full_screen_pager_reaches_the_end_of_a_long_request`
- `approvals_2::long_url_tokens_remain_visible_in_a_narrow_verification_prompt` (gap)
- `approvals_2::waiting_snapshot_and_duplicate_approval_suppression`
- `chat_live_1::tool_suggestion_install_url_is_validated_before_opening`
- `small_dialogs_1::auth_suggestion_with_reason_snapshot`
- `small_dialogs_1::declined_tool_suggestion_resolves_elicitation_decline`
- `small_dialogs_1::enable_suggestion_with_reason_snapshot`
- `small_dialogs_1::enable_tool_suggestion_resolves_elicitation_after_enable`
- `small_dialogs_1::generic_url_elicitation_confirmation_snapshot`
- `small_dialogs_1::generic_url_elicitation_resolves_without_connector_refresh`
- `small_dialogs_1::generic_url_elicitation_snapshot`
- `small_dialogs_1::install_suggestion_with_reason_snapshot`
- `small_dialogs_1::install_tool_suggestion_resolves_elicitation_after_confirmation`

### f-entries-resize-live (18, под гэпом 0)

- `status::transcript_overlay_remeasures_status_after_rate_limit_refresh`
- `transcript_view_1::fullscreen_composer_scrolls_without_moving_caret_or_transcript`
- `transcript_view_1::raw_multiline_paste_defers_rendering_and_submits_once`
- `transcript_view_1::slash_picker_overlays_history_without_moving_the_transcript_or_composer`
- `transcript_view_2::annotated_diff_preserves_the_existing_hard_wrapped_visual_output`
- `transcript_view_5::command_and_output_copy_preserve_hard_lines_across_resize`
- `transcript_view_5::empty_selected_rows_keep_copy_spacing_without_newline_highlights`
- `transcript_view_5::fullscreen_markdown_keeps_a_fitting_url_on_one_row`
- `transcript_view_5::hidden_initial_entries_do_not_add_a_separator_but_visible_entries_do`
- `transcript_view_5::offsets_can_reach_rows_after_u16_limit`
- `transcript_view_5::plan_copy_retains_checkboxes_but_omits_wrapping_gutters`
- `transcript_view_5::reasoning_copy_joins_soft_wraps_and_retains_the_existing_rendering`
- `transcript_view_5::repeated_resize_preserves_plain_and_markdown_passages_before_the_next_draw`
- `transcript_view_5::resize_keeps_the_logical_reading_anchor`
- `transcript_view_5::resizing_repeated_lines_keeps_the_same_occurrence`
- `transcript_view_5::scrolling_out_of_held_live_content_releases_its_revision`
- `transcript_view_5::selected_newlines_respect_wrapping_and_synthetic_rows`
- `transcript_view_5::single_row_scrolling_crosses_an_entry_separator_in_both_directions`

### f-entries-tabs-styles (15, под гэпом 0)

- `transcript_view_3::prompt_margin_and_blank_background_survive_resize_without_changing_copy`
- `transcript_view_3::visible_rows_fill_inherited_styles_without_painting_outside_the_viewport`
- `transcript_view_5::aligned_text_excludes_synthetic_leading_columns`
- `transcript_view_5::consecutive_user_prompts_share_one_background_block`
- `transcript_view_5::hard_wrapped_tabs_keep_source_characters_and_actual_row_stops`
- `transcript_view_5::leading_and_repeated_tabs_remain_selectable_below_one_tab_stop`
- `transcript_view_5::mutable_history_formats_once_per_frame_and_refreshes_the_next_frame`
- `transcript_view_5::narrow_wrapping_keeps_joined_emoji_whole`
- `transcript_view_5::ordinary_spaces_before_tabs_affect_the_stop_without_entering_the_gutter`
- `transcript_view_5::resize_restores_the_exact_styles_of_whitespace_omitted_by_wrapping`
- `transcript_view_5::selected_revision_rewraps_both_directions_with_styles_and_indents`
- `transcript_view_5::styled_tabs_use_row_stops_including_the_gutter`
- `transcript_view_5::tab_stops_restart_after_wrapping_and_resize_preserves_source_and_links`
- `transcript_view_5::user_prompt_has_a_background_without_a_terminal_palette`
- `transcript_view_5::visible_rows_preserve_styles_and_wrapped_hyperlinks`

### f-feedback-worktree (11, под гэпом 0)

- `chat_popups_4::feedback_good_result_consent_popup_includes_connectivity_diagnostics_filename`
- `chat_popups_4::feedback_selection_popup_snapshot`
- `chat_popups_4::feedback_stops_before_collecting_a_note_when_no_destination_is_available`
- `chat_popups_4::feedback_upload_consent_popup_snapshot`
- `chat_popups_4::saved_quick_dials_save_and_clear_picker_snapshots`
- `chat_popups_4::slash_new_and_fork_offer_checkout_choices_inside_local_git_repository`
- `chat_popups_4::worktree_browser_actions_and_stale_results`
- `chat_popups_4::worktree_picker_custom_keys_preserve_draft_and_protect_current_checkout`
- `small_dialogs_1::feedback_disclosure_matches_audience_at_normal_and_narrow_widths`
- `small_dialogs_1::feedback_disclosure_scrolls_without_hiding_input_in_short_panes`
- `small_dialogs_1::feedback_paste_burst_preserves_multiline_note_until_submit`

### f-find (27, под гэпом 0)

- `pager::opening_find_retains_the_selected_mutable_revision`
- `pane_footer_1::retained_command_suggestions_yield_paint_and_cursor_to_find`
- `transcript_view_1::accepted_find_defers_paste_rendering_but_keeps_enter`
- `transcript_view_1::find_owns_editor_chords_without_changing_the_composer_draft`
- `transcript_view_1::find_refreshes_live_details_before_searching_the_first_query`
- `transcript_view_1::inline_transcript_search_draws_and_escape_precedes_backtrack`
- `transcript_view_2::narrow_selection_and_search_keep_complete_exit_hints`
- `transcript_view_3::folding_a_reasoning_only_page_retains_its_find_and_copy_revision`
- `transcript_view_4::bounded_query_paste_keeps_the_suffix_and_whole_graphemes`
- `transcript_view_4::empty_find_preserves_reading_inside_a_joined_group`
- `transcript_view_4::find_expands_only_the_match_and_preserves_manual_disclosures`
- `transcript_view_4::find_keeps_context_and_allows_reading_without_losing_the_query`
- `transcript_view_4::find_reduces_context_to_keep_a_tall_match_visible`
- `transcript_view_4::find_reveals_hidden_command_output_and_restores_compact_presentation`
- `transcript_view_4::find_searches_hidden_live_content_and_collapses_it_when_leaving_the_match`
- `transcript_view_4::legacy_search_shortcuts_navigate_in_both_directions_without_wrapping`
- `transcript_view_4::live_match_stays_displayed_after_commit_and_a_fresh_query_searches_current_content`
- `transcript_view_4::next_previous_highlight_and_cancel_share_the_transcript_position`
- `transcript_view_4::paging_away_from_a_live_match_does_not_restart_find`
- `transcript_view_4::paging_below_a_live_find_result_resumes_current_output`
- `transcript_view_4::reading_without_a_match_searches_from_the_visible_entry_in_each_direction`
- `transcript_view_4::refining_a_query_restores_the_reading_position_while_history_loads`
- `transcript_view_4::resizing_a_read_find_result_preserves_relative_navigation`
- `transcript_view_4::search_match_separates_user_prompts_only_while_visible`
- `transcript_view_4::selected_hidden_live_match_keeps_its_revision_without_persisting_expansion`
- `transcript_view_4::selection_keys_precede_find_and_typing_returns_to_query`
- `transcript_view_4::selection_pauses_find_and_normalized_navigation_resumes_the_same_query`

### f-follow-control (21, под гэпом 5)

- `transcript_view_1::empty_enter_returns_to_latest_with_contextual_hints` (gap)
- `transcript_view_1::escape_returns_to_latest_without_changing_the_composer_draft` (gap)
- `transcript_view_1::follow_control_click_preserves_draft_caret_and_composer_geometry`
- `transcript_view_1::owned_transcript_reserves_a_row_above_the_composer` (gap)
- `transcript_view_1::submitting_a_draft_from_history_queues_once_and_follows_immediately`
- `transcript_view_1::tip_links_follow_alignment_and_release_stale_targets`
- `transcript_view_1::visible_updates_and_empty_tail_cells_do_not_report_activity` (gap)
- `transcript_view_2::control_tracks_pause_activity_hover_resize_and_click`
- `transcript_view_2::copying_selection_at_bottom_does_not_show_back_to_bottom`
- `transcript_view_2::empty_viewport_does_not_show_back_to_bottom`
- `transcript_view_2::enlarging_paused_view_hides_back_to_bottom_when_tail_fits` (gap)
- `transcript_view_2::hidden_control_does_not_consume_clicks_and_drag_out_cancels`
- `transcript_view_2::layouts_refresh_for_width_animation_and_mutable_frames`
- `transcript_view_2::loading_completion_restores_unseen_activity_and_failure_stops_motion`
- `transcript_view_2::mouse_scroll_speed_scales_rows_and_accumulates_fractional_movement`
- `transcript_view_2::passive_footer_keeps_jump_actions_in_one_row`
- `transcript_view_2::selection_at_bottom_restores_footer_without_resuming_following`
- `transcript_view_2::selection_hints_follow_selected_text_instead_of_mouse_down`
- `transcript_view_2::transcript_footer_keys_remain_distinct_from_prose_and_query_text`
- `transcript_view_5::page_keys_follow_pager_scroll_bindings`
- `transcript_view_5::selected_latest_content_keeps_new_activity_visible`

### f-footer-hints (11, под гэпом 0)

- `composer_4::shortcut_footer_displays_configured_chords`
- `pane_footer_1::footer_layout_in_fullscreen_and_scrollback`
- `pane_footer_1::interactive_query_keeps_focus_over_underlying_hints_but_empty_override_wins`
- `pane_footer_1::passive_activity_keeps_shortcuts_only_when_the_complete_hint_fits`
- `pane_footer_1::slash_popup_preserves_footer_and_selection_style`
- `pane_footer_2::centered_dialog_retains_backdrop_and_routes_cursor`
- `pane_footer_2::esc_dismisses_slash_command_popup_without_interrupting_task`
- `pane_footer_2::esc_release_after_dismissing_agent_picker_does_not_interrupt_task`
- `pane_footer_2::footer_snapshots`
- `pane_footer_2::paste_completion_clears_stacked_views_and_restores_composer_input`
- `pane_footer_2::remapped_interrupt_turn_uses_configured_key_including_agent_drafts`

### f-goal (11, под гэпом 0)

- `chat_commands_2::goal_control_slash_command_without_thread_shows_full_usage`
- `chat_commands_2::goal_control_slash_commands_emit_goal_events`
- `chat_live_2::goal_slash_command_emits_only_inserted_paste_text_element`
- `chat_popups_1::goal_edit_prompt_snapshot`
- `chat_popups_1::goal_edit_prompt_submits_preserved_status_and_budget`
- `chat_popups_1::goal_menu_active_snapshot`
- `chat_popups_1::goal_menu_budget_limited_snapshot`
- `chat_popups_1::goal_menu_paused_snapshot`
- `chat_popups_1::resume_paused_goal_prompt_snapshot`
- `chat_status_1::esc_interrupt_pauses_active_goal_turn`
- `chat_status_3::goal_clock_refresh_redraws_only_when_elapsed_label_changes`

### f-guardian-daybreak (7, под гэпом 0)

- `chat_approvals_2::app_server_guardian_write_stdin_denial_preserves_child_action`
- `chat_approvals_2::auto_review_denials_popup_lists_stored_auto_review_denials`
- `chat_approvals_2::guardian_denied_exec_renders_warning_and_denied_request`
- `chat_approvals_2::guardian_timed_out_exec_renders_warning_and_timed_out_request`
- `chat_live_2::resumed_initial_messages_render_history`
- `chat_status_1::daybreak_refusal_offers_enable_for_the_next_turn`
- `chat_status_3::daybreak_status_surfaces_follow_the_thread_preference`

### f-history-pages (22, под гэпом 0)

- `pager::jump_top_holds_the_visible_page_until_all_history_arrives`
- `pager::navigation_supersedes_home_before_and_after_the_first_draw`
- `pager::transcript_overlay_snapshots_paginated_history_states`
- `transcript_view_1::cancel_restores_a_group_replaced_during_browsing`
- `transcript_view_2::loading_and_failure_preserve_search_query_and_caret`
- `transcript_view_2::reading_a_find_result_shows_history_loading_and_failure`
- `transcript_view_3::absorbing_a_read_canonical_tail_holds_it_until_navigation_leaves`
- `transcript_view_3::page_join_keeps_groups_scrolled_into_view_after_selecting_elsewhere`
- `transcript_view_3::page_join_keeps_selection_and_clearing_it_retains_the_reading_position`
- `transcript_view_3::page_join_merges_unseen_groups_while_an_unrelated_selection_is_held`
- `transcript_view_4::beginning_motion_finishes_at_the_first_grapheme_after_paging`
- `transcript_view_4::empty_find_keeps_following_new_output_after_a_page_join`
- `transcript_view_4::enter_preserves_the_initial_scan_and_each_older_page_until_the_first_match`
- `transcript_view_4::leaving_a_regrouped_search_match_restarts_from_current_history`
- `transcript_view_4::loading_older_history_extends_the_snapshot_without_adopting_new_commits`
- `transcript_view_4::previous_match_continues_through_regrouped_pages_without_revisiting_newer_hits`
- `transcript_view_4::previous_match_finds_the_prepended_part_of_a_regrouped_entry`
- `transcript_view_4::reading_a_find_result_loads_older_context`
- `transcript_view_4::search_finds_the_first_older_entry_after_replaying_the_session_header`
- `transcript_view_5::reading_survives_prepend_and_new_output_then_returns_to_latest`
- `transcript_view_5::returning_to_latest_cancels_beginning_intent_while_retaining_the_loaded_page`
- `transcript_view_5::scrolling_to_latest_cancels_beginning_and_keeps_selection_when_the_final_page_arrives`

### f-history-search (6, под гэпом 3)

- `composer_popups::history_search_highlights_matches_until_accepted` (gap)
- `composer_popups::history_search_large_paste_clamps_cursor` (gap)
- `composer_popups::history_search_paste_shows_separators_and_matches_original_query` (gap)
- `composer_popups::history_search_paste_uses_full_sanitized_text`
- `pane_footer_1::fresh_draft_replacement_ends_search_and_dismisses_sparkles`
- `pane_footer_1::input_history_search_uses_the_final_row`

### f-hooks-live (14, под гэпом 0)

- `chat_status_2::blocked_and_failed_hooks_render_feedback_and_errors`
- `chat_status_2::completed_hook_output_precedes_following_assistant_message`
- `chat_status_2::completed_hook_with_output_flushes_immediately`
- `chat_status_2::completed_same_id_hook_output_survives_restart`
- `chat_status_2::completed_turn_clears_visible_running_hook`
- `chat_status_2::hook_events_render_consistently`
- `chat_status_2::interrupted_turn_clears_visible_running_hook`
- `chat_status_2::overlapping_hook_live_cell_tracks_parallel_quiet_hooks`
- `chat_status_2::quiet_hook_cleanup_starts_when_the_hook_is_revealed`
- `chat_status_2::running_hook_does_not_displace_active_exec_cell`
- `chat_status_2::running_hooks_fit_around_background_activity_and_finish_without_history_snapshot`
- `chat_status_2::session_end_hook_has_standalone_activity_row`
- `chat_status_2::stopped_hook_hides_model_context_and_preserves_stop_reason_snapshot`
- `chat_status_2::user_prompt_submit_app_server_hook_notifications_render_snapshot`

### f-image-attach (14, под гэпом 0)

- `chat_commands_1::blocked_image_restore_preserves_mention_bindings`
- `chat_commands_1::handoff_image_survives_model_rejection_and_submission_failure`
- `chat_commands_1::image_preparation_failure_restores_full_input_without_submitting`
- `chat_commands_1::image_preparation_keeps_input_responsive_and_preserves_pending_input`
- `chat_commands_1::image_submission_is_portable_for_new_turns_and_steers`
- `chat_commands_1::local_clipboard_image_does_not_turn_a_held_bang_into_a_shell_command`
- `composer_3::adjacent_image_completion_inserts_leading_separator`
- `composer_4::deleting_a_selected_remote_image_is_one_undoable_vim_edit`
- `composer_4::rich_paste_renders_unique_image_and_paste_labels`
- `composer_4::undo_and_redo_restore_deleted_image_attachment`
- `composer_4::vim_editing_modes_use_the_same_large_and_image_paste_handling`
- `pane_footer_2::remote_images_render_above_composer_text`
- `transcript_view_1::composer_thumbnails_follow_draft_deletion_and_resize`
- `transcript_view_1::fullscreen_composer_remote_images_leave_an_editable_prompt_row`

### f-live-active-cell (22, под гэпом 0)

- `cells_1::active_command_without_animations_is_stable`
- `cells_3::streaming_agent_tail_blank_line_uses_one_viewport_row`
- `cells_5::agent_markdown_cell_narrow_width_shows_prefix_only`
- `cells_5::render_clears_area_when_cell_content_shrinks`
- `cells_5::streamed_agent_list_paragraph_preserves_item_indent_when_wrapped`
- `cells_5::wrapped_and_prefixed_cells_handle_tiny_widths`
- `chat_approvals_1::external_writer_snapshot_freezes_active_command_and_mcp_rows`
- `chat_approvals_1::final_worked_for_uses_cumulative_turn_duration_snapshot`
- `chat_commands_3::configured_welcome_opt_out_disables_blossom_and_clicks`
- `chat_live_1::active_cell_layout_preserves_custom_height_and_bottom_scroll`
- `chat_live_1::active_cell_layout_reuses_heights_without_freezing_animation`
- `chat_live_1::active_transcript_preserves_clipped_markdown_hyperlinks`
- `chat_live_1::initial_session_header_starts_at_the_top_of_the_viewport`
- `chat_live_1::live_app_server_command_execution_strips_shell_wrapper`
- `chat_live_1::live_app_server_command_output_delta_transcript_snapshot`
- `chat_live_1::live_app_server_user_message_omits_unsupported_media`
- `chat_live_1::owned_live_history_keeps_all_sources_in_each_presentation`
- `chat_live_1::session_and_settings_sync_server_provider_id`
- `chat_live_1::session_header_uses_catalog_display_name_without_changing_model`
- `chat_live_1::thread_settings_updated_updates_visible_state_without_transcript`
- `chat_live_2::completion_follows_plain_and_streamed_tool_answers`
- `chat_live_5::interrupt_exec_marks_failed_snapshot`

### f-mcp-inventory-rest (4, под гэпом 0)

- `cells_3::mcp_tools_output_lists_tools_for_hyphenated_server_names`
- `cells_3::mcp_tools_output_masks_sensitive_values`
- `cells_4::mcp_inventory_loading_snapshot`
- `cells_4::mcp_inventory_loading_without_animations_is_stable`

### f-mouse-units (15, под гэпом 0)

- `pager::focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns`
- `transcript_view_2::copy_on_select_copies_word_and_line_on_release_without_dragging`
- `transcript_view_2::copy_on_select_preserves_link_activation_and_ignores_empty_drags`
- `transcript_view_2::copy_on_select_waits_for_release_and_confirmed_delivery`
- `transcript_view_2::hover_links_match_wrapped_click_targets_and_refresh_after_scrolling`
- `transcript_view_2::selection_mouse_does_not_open_links`
- `transcript_view_2::x11_release_preserves_primary_when_auto_copy_is_disabled`
- `transcript_view_3::reserved_margin_uses_the_same_alignment_for_painting_and_selection`
- `transcript_view_3::right_click_retains_selection_until_confirmed_and_preserves_reading_position`
- `transcript_view_4::existing_mouse_selection_keeps_its_word_or_line_unit`
- `transcript_view_4::word_and_line_drags_keep_the_initial_unit_when_direction_changes`
- `transcript_view_5::scrolled_plaintext_selection_matches_painted_tab_and_word_boundaries`
- `transcript_view_5::selecting_within_an_edge_row_does_not_start_selection_autoscroll`
- `transcript_view_5::selection_highlights_wide_graphemes_without_trailing_padding`
- `transcript_view_5::wheel_scrolling_pauses_selection_autoscroll_until_the_next_drag`

### f-permissions-rest (3, под гэпом 0)

- `chat_popups_1::clipped_full_access_confirmation_discloses_hidden_warning_text`
- `chat_popups_1::permission_shortcuts_respect_managed_mode_requirements`
- `chat_popups_2::permissions_command_waits_for_covering_modal_before_selecting_profile`

### f-picker-rows (16, под гэпом 4)

- `approvals_2::halfwidth_sound_marks_are_truncated_and_rendered_at_a_grapheme_boundary`
- `pane_footer_2::selection_view_esc_respects_remapped_list_cancel`
- `picker::choice_labels_and_descriptions_keep_complete_url_destinations`
- `picker::exact_fit_description_reserves_only_the_rendered_row`
- `picker::filled_tab_unknown_palette_uses_readable_underlined_defaults` (gap)
- `picker::fuzzy_matched_emoji_graphemes_keep_description_alignment`
- `picker::picker_appearance_renders_checkboxes_preview_and_overflow` (gap)
- `picker::snapshot_auto_visible_col_width_mode_scroll_behavior` (gap)
- `picker::snapshot_narrow_width_counts_halfwidth_sound_marks` (gap)
- `picker::wrapped_two_column_rows_count_halfwidth_sound_marks`
- `picker::wrapped_two_column_rows_preserve_hard_line_breaks`
- `small_dialogs_2::renders_event_browser`
- `small_dialogs_2::renders_event_browser_with_review_column_when_needed`
- `small_dialogs_2::scroll_hint_tracks_wheel_and_keys_across_wrapped_rows`
- `small_dialogs_2::shortcut_overlay_clipping_preserves_customization_in_small_areas`
- `small_dialogs_2::shortcut_overlay_uses_runtime_bindings_and_wsl_paste`

### f-plan-mode (11, под гэпом 2)

- `chat_commands_1::interrupted_turn_restore_keeps_active_mode_for_resubmission`
- `chat_live_4::mode_switch_surfaces_model_change_notification_when_effective_model_changes` (gap)
- `chat_live_4::mode_switch_surfaces_reasoning_change_notification_when_model_stays_same` (gap)
- `chat_live_4::plan_implementation_popup_skips_when_rate_limit_prompt_pending`
- `chat_live_4::plan_mode_reasoning_override_is_marked_current_in_reasoning_popup`
- `chat_live_4::plan_reasoning_scope_popup_mentions_built_in_plan_default_when_no_override`
- `chat_live_4::plan_reasoning_scope_popup_mentions_selected_reasoning`
- `chat_popups_1::collaboration_catalog_unavailable_preserves_task_and_plan_input`
- `chat_status_1::completed_plan_table_tail_skips_provisional_history_insert`
- `transcript_view_1::plan_menu_allows_transcript_selection_and_copy_but_respects_popup_bounds`
- `transcript_view_1::plan_menu_allows_transcript_wheel_scrolling_and_keeps_keyboard_ownership`

### f-queue-commands (15, под гэпом 1)

- `chat_approvals_1::disabled_slash_command_while_task_running_snapshot`
- `chat_commands_1::default_shortcuts_edit_most_recent_queued_message`
- `chat_commands_1::hidden_shell_paste_queued_during_turn_submits_literal_prompt`
- `chat_commands_1::unbound_queued_message_edit_does_not_fall_back_to_alt_up`
- `chat_commands_2::queued_empty_bang_shell_reports_help_when_dequeued_and_drains_next_input`
- `chat_commands_2::queued_settings_selection_applies_before_next_input`
- `chat_commands_2::queued_slash_review_with_args_restores_for_edit`
- `chat_commands_2::queued_unknown_slash_reports_error_when_dequeued`
- `chat_commands_2::slash_compact_eagerly_queues_follow_up_before_turn_start`
- `chat_commands_3::compact_queues_user_messages_snapshot`
- `chat_commands_3::queued_menu_slash_keeps_agent_turn_complete_notification`
- `chat_live_5::failed_pending_steer_submit_does_not_add_pending_preview`
- `composer_2::tab_queues_slash_led_prompts_while_task_running_without_validation`
- `composer_4::paste_burst_modified_queue_binding_still_dispatches`
- `markdown_streaming_5::controller_keeps_pre_table_lines_queued_when_table_is_confirmed` (gap)

### f-rename-recap (9, под гэпом 0)

- `chat_commands_2::queued_bare_rename_drains_next_input_after_name_update`
- `chat_live_1::live_app_server_automatic_thread_name_update_is_silent`
- `chat_live_1::live_app_server_manual_thread_name_updates_status_surfaces`
- `chat_live_1::thread_name_suggestion_requires_matching_thread_and_request`
- `pane_footer_2::apply_text_suggestion_updates_matching_prompt_beneath_an_overlay`
- `transcript_view_1::recap_spacing_belongs_to_the_transcript_tail`
- `transcript_view_5::recap_copy_preserves_hard_lines_and_reflows_only_soft_wraps`
- `transcript_view_5::recap_tabs_preserve_url_wrapping_and_source_across_resize`
- `transcript_view_5::selected_recap_remains_visible_when_resize_cannot_fit_its_gutter`

### f-replay-history (6, под гэпом 0)

- `cells_2::compact_patches_retain_full_changes_and_failure_details`
- `chat_approvals_1::replayed_command_completion_preserves_tracking_without_duplicate_starts`
- `chat_approvals_1::replayed_commands_preserve_individual_output_and_failure_status`
- `chat_live_2::compaction_history_does_not_start_a_timer_or_finish_live_compaction`
- `chat_live_2::compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion`
- `chat_live_2::snapshot_formatter_empty_success_and_failed_patch_keep_their_outcomes`

### f-safety-misalignment (9, под гэпом 0)

- `chat_live_1::safety_buffering_ignores_hidden_stale_and_historical_updates`
- `chat_live_1::safety_buffering_offers_one_retry_with_app_wording`
- `chat_live_1::safety_buffering_retry_confirmation_can_keep_waiting`
- `chat_live_1::safety_buffering_retry_confirmation_closes_when_response_starts`
- `chat_live_1::safety_buffering_retry_confirmation_closes_when_turn_completes`
- `chat_live_1::safety_buffering_without_retry_shows_short_app_message`
- `chat_live_3::misalignment_policy_failure_stops_the_thread_and_renders_once`
- `chat_live_3::misalignment_review_requires_current_findings`
- `chat_live_3::misalignment_turn_end_discards_history_search_and_question_drafts`

### f-select-mode (18, под гэпом 0)

- `pager::empty_pending_select_mode_closes_instead_of_retaining_overlay_focus`
- `pager::select_enter_closes_and_remapped_action_marks_in_the_inline_overlay`
- `transcript_view_4::accepted_find_pins_an_expanded_match_before_select_mode_exits`
- `transcript_view_4::accepted_find_repeats_into_the_select_cursor`
- `transcript_view_4::character_motion_crosses_line_and_entry_boundaries_without_invisible_stops`
- `transcript_view_4::configured_pager_page_motion_moves_the_select_cursor`
- `transcript_view_4::confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme`
- `transcript_view_4::enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows`
- `transcript_view_4::entry_starts_on_the_last_selectable_visible_line`
- `transcript_view_4::find_accepts_at_the_match_and_cancel_restores_the_saved_selection`
- `transcript_view_4::logical_line_end_handles_empty_lines_and_shifted_dollar`
- `transcript_view_4::passive_mouse_events_do_not_reanchor_a_reverse_line_mark`
- `transcript_view_4::reverse_line_selection_copies_complete_lines_in_document_order`
- `transcript_view_4::select_mode_renders_cursor_and_scannable_footer_hints`
- `transcript_view_4::swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint`
- `transcript_view_4::transcript_end_character_selection_includes_the_last_grapheme`
- `transcript_view_4::vertical_motion_keeps_the_preferred_display_column`
- `transcript_view_4::word_motion_crosses_entries_and_word_end_selects_the_destination_word`

### f-selection-keys (15, под гэпом 0)

- `transcript_view_1::configured_ctrl_space_submit_wins_over_transcript_selection`
- `transcript_view_2::shift_click_extends_a_double_clicked_word_in_both_directions`
- `transcript_view_4::clicking_and_clearing_selection_preserves_the_separator_row`
- `transcript_view_4::command_c_ignores_release_and_copies_only_an_active_selection`
- `transcript_view_4::enter_on_an_empty_selection_does_not_copy_or_reach_the_composer`
- `transcript_view_4::keyboard_selection_moves_by_grapheme_and_keeps_its_column_across_short_lines`
- `transcript_view_4::live_selection_survives_commits_finalization_new_live_content_and_resize`
- `transcript_view_4::modified_arrows_end_selection_and_return_input_to_the_owner`
- `transcript_view_4::pending_copy_only_finishes_the_original_selection`
- `transcript_view_4::plain_and_shift_arrows_move_selection_on_press_and_repeat`
- `transcript_view_4::released_arrows_preserve_selection_and_reading_position`
- `transcript_view_4::selection_highlights_hard_breaks_within_and_between_entries`
- `transcript_view_4::shift_click_extends_from_the_keyboard_marks_fixed_anchor`
- `transcript_view_4::tool_selection_copies_its_displayed_revision_after_the_tool_commits`
- `transcript_view_5::live_reader_survives_commit_new_tail_resize_and_selection_then_rejoins_latest`

### f-settings-lists (13, под гэпом 0)

- `chat_commands_3::slash_export_opens_destination_picker`
- `chat_commands_3::slash_keymap_debug_can_inspect_app_shortcuts`
- `chat_commands_3::slash_keymap_debug_opens_keypress_inspector`
- `chat_commands_3::slash_memories_opens_memory_menu`
- `chat_popups_3::hooks_popup_shows_list_diagnostics`
- `chat_popups_4::experimental_features_popup_snapshot`
- `chat_popups_4::experimental_features_toggle_saves_on_exit`
- `chat_popups_4::experimental_popup_available_snapshot`
- `chat_popups_4::experimental_popup_loading_snapshot`
- `chat_popups_4::feature_enable_prompts_snapshot`
- `chat_popups_4::manage_skills_summary_snapshot`
- `chat_popups_4::skills_menu_default_mentions_shortcut_snapshot`
- `small_dialogs_2::footer_hint_uses_list_keymap_accept_and_cancel`

### f-side-rest (4, под гэпом 0)

- `chat_live_1::safety_buffering_does_not_offer_retry_in_side_conversation`
- `chat_live_5::side_context_label_preserves_status_line_snapshot`
- `chat_live_5::side_context_label_shows_parent_status_snapshot`
- `chat_live_5::slash_btw_is_rejected_before_the_session_starts`

### f-startup-draft (10, под гэпом 0)

- `chat_commands_1::hidden_shell_paste_queued_before_session_submits_literal_prompt`
- `chat_commands_1::hidden_shell_paste_recalled_from_history_submits_literal_prompt`
- `chat_commands_1::rejected_hidden_shell_paste_preserves_colliding_draft_paste`
- `chat_commands_1::startup_draft_file_search_waits_for_protected_view_and_enabled_input`
- `chat_commands_1::startup_draft_handoff_rebases_cursor_around_colliding_large_pastes`
- `chat_commands_3::initial_prompt_image_preparation_preserves_recall_metadata`
- `chat_commands_3::initial_prompt_queued_before_session_is_recalled_once_after_dispatch`
- `chat_commands_3::startup_draft_applies_super_editor_keymap_without_inserting_pending_newline`
- `chat_commands_3::startup_submission_waits_for_protected_view_and_preserves_paste_provenance`
- `chat_live_4::plan_slash_command_with_hidden_shell_paste_queued_before_session_submits_literal_prompt`

### f-status-line (17, под гэпом 3)

- `chat_status_2::status_line_invalid_items_warn_once`
- `chat_status_2::status_line_processing_tier_updates_with_selection`
- `pane_footer_1::effort_transition_never_replaces_a_footer_flash`
- `pane_footer_1::focus_startup_work_and_configuration_do_not_extend_a_started_deadline`
- `pane_footer_1::native_status_wraps_links_within_the_host_budget`
- `pane_footer_2::arabic_prefix_keeps_link_on_the_painted_cells_at_exact_fit_and_overflow`
- `pane_footer_2::clipped_links_keep_their_cells_after_a_halfwidth_katakana_grapheme`
- `pane_footer_2::effort_ignition_animation_gallery_snapshot`
- `pane_footer_2::footer_status_line_truncates_to_keep_mode_indicator`
- `pane_footer_2::light_status_line_corrects_pale_custom_theme_colors`
- `pane_footer_2::long_url_like_message_does_not_expand_into_wrapped_ellipsis_rows` (gap)
- `pane_footer_2::narrow_preview_clips_the_overflow_and_edit_hint` (gap)
- `pane_footer_2::oversized_preview_urls_keep_complete_destinations` (gap)
- `pane_footer_2::prepared_tree_uses_its_natural_width_and_paints_only_the_footer_viewport`
- `pane_footer_2::status_line_theme_colors_at_limited_color_depths`
- `pane_footer_2::transition_frames_snapshot`
- `small_dialogs_2::picker_previews_the_pr_link_and_preserves_the_color_preference`

### f-tool-activity (20, под гэпом 0)

- `chat_approvals_1::image_generation_call_adds_history_cell`
- `chat_approvals_1::view_image_tool_call_adds_history_cell`
- `chat_approvals_1::view_image_tool_call_preserves_foreign_path`
- `chat_live_1::collab_spawn_end_shows_requested_model_and_effort`
- `chat_live_1::live_app_server_collab_spawn_completed_renders_requested_model_and_effort`
- `chat_live_1::live_app_server_collab_wait_items_render_history`
- `chat_live_1::live_app_server_sub_agent_activity_renders_once`
- `chat_live_2::failed_repl_mcp_tool_call_preserves_status_and_result`
- `chat_status_1::completed_reasoning_stays_in_expanded_transcript_for_live_and_replay`
- `chat_status_1::computer_activity_keeps_reasoning_in_order_live_and_replayed`
- `chat_status_1::computer_activity_live_and_replay_group_identically`
- `chat_status_1::computer_activity_out_of_order_completion_and_interruption`
- `chat_status_1::computer_activity_preserves_boundaries_and_expanded_output`
- `chat_status_1::owned_dynamic_activity_updates_the_retained_row_after_config_refresh`
- `chat_status_1::terminal_dynamic_activity_retains_calls_across_both_fallbacks`
- `chat_status_1::turn_finalization_drains_lifecycle_queue_before_interrupting_remaining_calls`
- `chat_status_3::image_result_live_and_replay_render_in_the_owning_call`
- `chat_status_3::interrupted_answer_flushes_queued_subagent_activity`
- `chat_status_3::subagent_completion_preserves_stream_and_authoritative_answer`
- `transcript_view_2::fullscreen_image_previews_follow_layout_without_entering_text_surfaces`

### f-turn-details (7, под гэпом 4)

- `transcript_view_2::activity_focus_keeps_controls_without_passive_hints`
- `transcript_view_2::patch_disclosure_preserves_manual_scrolling`
- `transcript_view_3::header_sanitizes_controls_and_truncates_by_display_width` (gap)
- `transcript_view_3::mouse_selection_preserves_header_reservation_at_turn_boundaries` (gap)
- `transcript_view_3::pinned_prompt_matches_the_original_prompt_style` (gap)
- `transcript_view_3::prompt_header_tracks_reading_and_yields_to_visible_prompts_and_short_screens` (gap)
- `transcript_view_5::terminal_output_disclosure_counts_only_revealable_lines`

### f-unified-exec (14, под гэпом 0)

- `cells_3::unified_exec_interaction_cell_retains_detailed_input`
- `cells_3::unified_exec_interaction_cell_retains_detailed_wait`
- `cells_4::unified_exec_interaction_details_do_not_split_url_like_stdin_token`
- `cells_4::unified_exec_interaction_details_wrap_long_input`
- `chat_approvals_1::interrupt_preserves_unified_exec_wait_streak_snapshot`
- `chat_approvals_1::replayed_background_terminals_belong_to_the_displayed_thread`
- `chat_approvals_1::unified_exec_begin_restores_working_status_snapshot`
- `chat_approvals_1::unified_exec_empty_then_non_empty_snapshot`
- `chat_approvals_1::unified_exec_non_empty_then_empty_snapshots`
- `chat_approvals_1::unified_exec_unknown_end_with_active_exploring_cell_snapshot`
- `chat_approvals_1::unified_exec_wait_after_final_agent_message_snapshot`
- `chat_approvals_1::unified_exec_wait_before_streamed_agent_message_snapshot`
- `chat_approvals_1::unified_exec_wait_status_renders_command_in_single_details_row_snapshot`
- `chat_approvals_1::unified_exec_waiting_multiple_empty_snapshots`

### f-usage-rest (23, под гэпом 1)

- `chat_live_6::account_change_dismisses_reset_popup_beneath_overlay`
- `chat_live_6::rate_limit_reset_load_result_updates_popup_beneath_overlay`
- `chat_live_6::rate_limit_reset_popup_states_snapshot`
- `chat_live_6::rate_limit_reset_success_updates_popup_beneath_overlay`
- `chat_live_6::startup_reset_hint_waits_for_active_output_snapshot`
- `usage::account_lookup_failure_recovers_on_refresh_with_the_server_plan`
- `usage::analytics_dates_distinguish_zero_missing_and_exact_details`
- `usage::analytics_narrow_charts_and_small_terminals_keep_valid_cursors`
- `usage::analytics_preserves_full_denominators_and_signed_small_credits`
- `usage::analytics_sections_adapt_to_terminal_colors`
- `usage::analytics_series_rank_by_range_and_keep_calendar_ticks_stable`
- `usage::chat_selection_keeps_a_highlighted_trailing_blank_after_wrapping`
- `usage::configured_question_mark_keeps_its_action_and_hides_help_hint`
- `usage::current_palette_uses_effective_terminal_color_level`
- `usage::horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat`
- `usage::keyboard_tabs_and_help_preserve_report_reading_position`
- `usage::live_account_identity_is_visible_and_closing_aborts_pending_loads`
- `usage::navigation_keeps_its_row_when_sections_change`
- `usage::report_headlines_and_axes_compact_large_values`
- `usage::reports_keep_tabs_fixed_and_expose_data_on_small_screens`
- `usage::resize_keeps_visible_weekly_selection_and_respects_manual_scroll` (gap)
- `usage::tall_reports_use_available_height_and_wide_reports_put_details_beside_plot`
- `usage::twelve_hour_metadata_keeps_utc_across_renderers`

### f-vim (25, под гэпом 1)

- `composer_3::left_respects_editor_and_vim_history_remaps`
- `composer_4::backspace_cancels_empty_vim_search`
- `composer_4::buffered_typing_survives_vim_changes_and_cancellation`
- `composer_4::canceled_history_preview_preserves_active_vim_commands`
- `composer_4::composer_handoff_preserves_only_the_live_rich_register`
- `composer_4::direct_edits_join_the_active_vim_insert_transaction`
- `composer_4::editor_actions_insert_shell_prefixes`
- `composer_4::empty_insert_and_replace_yanks_do_not_replay_a_later_register`
- `composer_4::empty_vim_normal_slash_opens_commands`
- `composer_4::equal_length_plain_replace_yank_records_dot`
- `composer_4::external_editor_import_preserves_replace_backspace_recovery` (gap)
- `composer_4::fresh_vim_draft_records_typing_after_submission`
- `composer_4::insert_yank_repeat_keeps_original_text_after_word_deletion`
- `composer_4::linewise_rich_paste_preserves_the_cursor_on_the_leading_element`
- `composer_4::plain_enter_submits_expired_vim_insert_input`
- `composer_4::plain_replace_yank_preserves_previous_backspace_recovery`
- `composer_4::recovered_answers_undo_as_one_vim_edit`
- `composer_4::recovery_keeps_unselected_text_after_a_held_replace_key`
- `composer_4::vim_redo_keeps_its_binding_when_history_is_empty`
- `composer_4::vim_replace_completion_preserves_suffix`
- `composer_4::vim_replace_delete_backward_restores_characters_hidden_by_prompt_prefixes`
- `composer_4::vim_search_query_edits_and_paste_preserve_the_draft`
- `composer_4::vim_search_query_paste_does_not_invalidate_draft_redo`
- `composer_4::yank_replaces_selection_without_recording_pointer_edits`
- `textarea::vim_e_advances_from_each_word_end`

### f-warning-cells (10, под гэпом 0)

- `cells_5::usage_warnings_render_in_both_transcripts_including_composites`
- `cells_5::warning_count_deduplicates_messages_mcp_summaries_and_composites`
- `chat_live_2::chained_config_error_wraps_in_history_snapshot`
- `chat_popups_4::ultra_reasoning_selection_warns_for_high_multi_agent_concurrency`
- `chat_status_1::app_server_cyber_policy_error_renders_dedicated_notice`
- `chat_status_1::app_server_model_verification_renders_warning`
- `chat_status_1::warning_event_adds_warning_history_cell`
- `chat_status_2::repeated_model_metadata_warning_is_hidden_for_same_slug`
- `chat_status_2::unsupported_code_mode_warning_renders_as_warning_history_cell`
- `small_dialogs_2::warning_links_preserve_destinations_when_wrapped_and_scrolled`

### f-working-status (15, под гэпом 0)

- `chat_status_1::answer_phase_controls_working_status_snapshot`
- `chat_status_1::reasoning_status_accepts_bold_text_with_a_plain_suffix`
- `chat_status_1::reasoning_status_tracks_items_and_restores_after_tool_activity`
- `chat_status_1::resumed_session_hides_unknown_token_usage_until_an_update_arrives`
- `chat_status_1::status_widget_active_snapshot`
- `chat_status_2::chatwidget_exec_and_status_layout_vt100_snapshot`
- `chat_status_2::chatwidget_markdown_code_blocks_vt100_snapshot`
- `chat_status_2::chatwidget_tall`
- `chat_status_2::deltas_then_same_final_message_are_rendered_snapshot`
- `chat_status_2::final_reasoning_then_message_without_deltas_are_rendered`
- `chat_status_2::multiple_agent_messages_in_single_turn_emit_multiple_headers`
- `chat_status_2::reasoning_delta_does_not_double_schedule_visible_status_redraw`
- `chat_status_2::reasoning_delta_restores_recreated_status_indicator_header`
- `chat_status_2::runtime_metrics_websocket_timing_logs_and_final_separator_sums_totals`
- `chat_status_2::unterminated_prose_is_visible_before_completion`

### вне фич: markdown (124, под гэпом 112)

- `chat_live_2::list_spacing_streams_compact_then_reflows_on_completion_or_interruption`
- `markdown_streaming_1::append_markdown_agent_unwraps_markdown_fences_for_no_outer_table_rendering` (gap)
- `markdown_streaming_1::append_markdown_agent_unwraps_markdown_fences_for_single_column_table` (gap)
- `markdown_streaming_1::append_markdown_agent_unwraps_markdown_fences_for_table_rendering` (gap)
- `markdown_streaming_1::append_markdown_agent_unwraps_markdown_fences_for_two_column_no_outer_table` (gap)
- `markdown_streaming_1::blank_table_continuations_do_not_repeat_the_first_cell` (gap)
- `markdown_streaming_1::does_not_split_long_url_like_token_without_scheme` (gap)
- `markdown_streaming_1::does_not_wrap_code_blocks` (gap)
- `markdown_streaming_1::file_citations_inside_code_existing_links_and_html_remain_literal`
- `markdown_streaming_1::file_citations_preserve_adjacent_entities_and_escaped_punctuation` (gap)
- `markdown_streaming_1::integral_limits_and_parenthesized_matrices` (gap)
- `markdown_streaming_1::issue_48403_renders_zero_and_multiline_display` (gap)
- `markdown_streaming_1::key_value_table_keeps_web_annotations` (gap)
- `markdown_streaming_1::list_spacing_is_uniform_per_list_and_preserves_item_paragraphs` (gap)
- `markdown_streaming_1::pipe_table_fallback_keeps_web_annotations` (gap)
- `markdown_streaming_1::unicode_math_aligned_fraction_and_narrow_fallback` (gap)
- `markdown_streaming_1::unicode_math_aligned_preserves_fraction_geometry` (gap)
- `markdown_streaming_1::unicode_math_display_limits` (gap)
- `markdown_streaming_1::unicode_math_grouped_scripts_and_annotations` (gap)
- `markdown_streaming_1::unicode_math_narrow_layout_stays_meaningful` (gap)
- `markdown_streaming_1::unicode_math_nested_fractions_and_oversized_pending_stay_source` (gap)
- `markdown_streaming_1::unicode_math_optimization_model` (gap)
- `markdown_streaming_1::unicode_math_schrodinger_equation_snapshot` (gap)
- `markdown_streaming_1::unicode_math_snapshot` (gap)
- `markdown_streaming_1::unicode_math_structured_bounds_and_invalid_input` (gap)
- `markdown_streaming_1::wrap_cell_preserves_hard_break_lines` (gap)
- `markdown_streaming_1::wraps_blockquotes` (gap)
- `markdown_streaming_1::wraps_blockquotes_inside_lists` (gap)
- `markdown_streaming_1::wraps_list_items_containing_blockquotes` (gap)
- `markdown_streaming_1::wraps_nested_lists` (gap)
- `markdown_streaming_2::bare_urls_keep_link_style_when_wrapped_in_tables` (gap)
- `markdown_streaming_2::blockquote_heading_inherits_heading_style` (gap)
- `markdown_streaming_2::blockquote_in_ordered_list_on_next_line` (gap)
- `markdown_streaming_2::blockquote_in_unordered_list_on_next_line` (gap)
- `markdown_streaming_2::blockquote_inside_nested_list` (gap)
- `markdown_streaming_2::blockquote_list_then_nested_blockquote` (gap)
- `markdown_streaming_2::blockquote_multiple_with_break` (gap)
- `markdown_streaming_2::blockquote_nested_two_levels` (gap)
- `markdown_streaming_2::blockquote_single` (gap)
- `markdown_streaming_2::blockquote_soft_break` (gap)
- `markdown_streaming_2::blockquote_surrounded_by_blank_lines` (gap)
- `markdown_streaming_2::blockquote_three_paragraphs_short_lines` (gap)
- `markdown_streaming_2::blockquote_two_paragraphs_inside_ordered_list_has_blank_line` (gap)
- `markdown_streaming_2::blockquote_with_code_block` (gap)
- `markdown_streaming_2::blockquote_with_heading_and_paragraph` (gap)
- `markdown_streaming_2::blockquote_with_list_items` (gap)
- `markdown_streaming_2::blockquote_with_multiline_code_block` (gap)
- `markdown_streaming_2::blockquote_with_ordered_list` (gap)
- `markdown_streaming_2::empty_list_items_keep_their_markers` (gap)
- `markdown_streaming_2::label_only_web_links_keep_targets_after_prose_and_table_wrapping` (gap)
- `markdown_streaming_2::list_item_blockquote_then_text` (gap)
- `markdown_streaming_2::list_item_text_blockquote_text` (gap)
- `markdown_streaming_2::list_item_text_then_blockquote` (gap)
- `markdown_streaming_2::list_nested` (gap)
- `markdown_streaming_2::lists_preserve_layout` (gap)
- `markdown_streaming_2::mermaid_fences_use_native_renderer_for_every_family` (gap)
- `markdown_streaming_2::nested_ordered_in_unordered` (gap)
- `markdown_streaming_2::nested_unordered_in_ordered` (gap)
- `markdown_streaming_2::table_url_with_tilde_keeps_complete_hyperlink` (gap)
- `markdown_streaming_2::unicode_math_accents_and_symbols_snapshot` (gap)
- `markdown_streaming_2::unicode_math_accents_reject_ambiguous_arguments` (gap)
- `markdown_streaming_2::unicode_math_bounds_and_unsupported_input` (gap)
- `markdown_streaming_2::unicode_math_named_delimiters_snapshot` (gap)
- `markdown_streaming_3::deeply_nested_mixed_three_levels` (gap)
- `markdown_streaming_3::file_links_preserve_foreign_windows_paths_and_anchors_snapshot`
- `markdown_streaming_3::markdown_render_file_link_snapshot` (gap)
- `markdown_streaming_3::web_link_labels_keep_link_style_in_wrapped_prose_and_tables` (gap)
- `markdown_streaming_4::code_block_indented` (gap)
- `markdown_streaming_4::code_block_inside_unordered_list_item_is_indented` (gap)
- `markdown_streaming_4::code_block_multiple_lines_inside_unordered_list` (gap)
- `markdown_streaming_4::code_block_multiple_lines_root` (gap)
- `markdown_streaming_4::code_block_no_lang_plain` (gap)
- `markdown_streaming_4::code_block_preserves_trailing_blank_lines` (gap)
- `markdown_streaming_4::code_block_unknown_lang_plain` (gap)
- `markdown_streaming_4::code_block_with_inner_triple_backticks_outer_four` (gap)
- `markdown_streaming_4::growing_code_fences_preserve_styles_unicode_links_and_blank_lines`
- `markdown_streaming_4::horizontal_rule_renders_em_dashes` (gap)
- `markdown_streaming_4::html_block_is_verbatim_multiline` (gap)
- `markdown_streaming_4::list_item_after_code_block_keeps_blank_separator` (gap)
- `markdown_streaming_4::markdown_render_complex_snapshot` (gap)
- `markdown_streaming_4::nested_five_levels_mixed_lists` (gap)
- `markdown_streaming_4::nested_item_continuation_paragraph_is_indented` (gap)
- `markdown_streaming_4::ordered_item_with_code_block_and_nested_bullet` (gap)
- `markdown_streaming_4::outer_list_item_after_nested_code_block_keeps_blank_separator` (gap)
- `markdown_streaming_4::table_alignment_respects_markers` (gap)
- `markdown_streaming_4::table_falls_back_to_key_value_records_if_grid_cannot_fit` (gap)
- `markdown_streaming_4::table_inside_blockquote_has_quote_prefix` (gap)
- `markdown_streaming_4::table_keeps_grid_when_only_one_compact_record_fragments_snapshot` (gap)
- `markdown_streaming_4::table_key_value_fallback_preserves_rich_values_and_themed_labels` (gap)
- `markdown_streaming_4::table_renders_app_style_rows_with_themed_bold_header` (gap)
- `markdown_streaming_4::table_renders_halfwidth_sound_marks_at_constrained_width_snapshot` (gap)
- `markdown_streaming_4::table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot` (gap)
- `markdown_streaming_4::table_renders_records_when_multiple_prose_columns_are_starved_snapshot` (gap)
- `markdown_streaming_4::table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot` (gap)
- `markdown_streaming_4::table_separates_logical_rows_after_wrapped_content` (gap)
- `markdown_streaming_4::table_wraps_file_paths_before_collapsing_narrative_columns_snapshot` (gap)
- `markdown_streaming_5::controller_keeps_markdown_fenced_no_outer_tables_mutable_until_finalize`
- `markdown_streaming_5::controller_keeps_markdown_fenced_tables_mutable_until_finalize`
- `markdown_streaming_5::controller_live_tail_rerenders_table_tail_after_resize` (gap)
- `markdown_streaming_5::controller_renders_separators_for_multi_table_response_shape` (gap)
- `markdown_streaming_5::controller_renders_separators_for_no_outer_pipes_table_shape` (gap)
- `markdown_streaming_5::controller_stabilizes_first_no_outer_pipes_table_in_response` (gap)
- `markdown_streaming_5::controller_stabilizes_two_column_no_outer_table_in_response` (gap)
- `markdown_streaming_5::finalized_plan_stream_preserves_semantic_url_fragments` (gap)
- `markdown_streaming_5::incremental_render_keeps_final_block_mutable_and_matches_full_render` (gap)
- `markdown_streaming_5::mermaid_controller_keeps_diagram_mutable_and_returns_original_source` (gap)
- `markdown_streaming_5::mermaid_holdback_tracks_nested_blocks_after_normalized_markdown` (gap)
- `markdown_streaming_5::mermaid_stream_closes_resizes_and_preserves_raw_source` (gap)
- `markdown_streaming_5::plan_controller_streamed_markdown_fenced_table_matches_final_render` (gap)
- `markdown_streaming_5::plan_controller_streamed_table_matches_final_render` (gap)
- `markdown_streaming_5::rejected_math_openers_allow_bounded_incremental_rendering` (gap)
- `markdown_streaming_5::shell_pid_preserves_following_equations` (gap)
- `markdown_streaming_5::unicode_math_aligned_stream_matches_complete_render` (gap)
- `markdown_streaming_5::unicode_math_raw_preview_preserves_line_on_newline` (gap)
- `markdown_streaming_5::unicode_math_rejected_display_keeps_its_closer_and_following_text` (gap)
- `markdown_streaming_5::unicode_math_stream_holds_display_until_closed_and_preserves_source` (gap)
- `renderer_internals::adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word`
- `renderer_internals::adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width`
- `renderer_internals::adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks`
- `renderer_internals::adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme`
- `renderer_internals::collector_source_chunks_round_trip_into_agent_fence_unwrapping` (gap)
- `renderer_internals::loose_vs_tight_list_items_streaming_matches_full` (gap)
- `renderer_internals::wrap_lines_applies_initial_indent_only_once`
- `transcript_view_1::list_spacing_completion_preserves_the_scrolled_reader`

### вне фич: textarea (35, под гэпом 34)

- `textarea::breakable_unicode_space_hangs_before_following_text` (gap)
- `textarea::composer_wrapped_url_fragments_keep_the_complete_destination` (gap)
- `textarea::cursor_wraps_after_filling_the_final_line` (gap)
- `textarea::delete_backward_removes_thai_marks_one_at_a_time` (gap)
- `textarea::distinct_urls_respect_punctuation_wide_prefixes_and_tabs` (gap)
- `textarea::full_final_line_scrolls_to_its_visible_cursor_row` (gap)
- `textarea::full_non_final_line_cursor_snapshot` (gap)
- `textarea::full_non_final_lines_reserve_visible_cursor_rows` (gap)
- `textarea::halfwidth_sound_marks_wrap_and_align_with_the_cursor` (gap)
- `textarea::hanging_tabs_keep_cursor_scrolling_and_navigation_aligned` (gap)
- `textarea::hyperlink_cache_is_invalidated_when_text_changes` (gap)
- `textarea::joined_emoji_preserve_complete_url_cell_ranges` (gap)
- `textarea::leading_space_reflows_across_following_wrapped_fragments` (gap)
- `textarea::mandatory_breaks_keep_distinct_cursor_positions` (gap)
- `textarea::many_urls_render_with_the_complete_destination` (gap)
- `textarea::masked_graphemes_align_with_the_cursor` (gap)
- `textarea::masked_url_input_never_exposes_hyperlink_destinations` (gap)
- `textarea::maximum_length_urls_render_without_osc8_annotations` (gap)
- `textarea::mouse_click_maps_wrapping_unicode_tabs_and_empty_lines` (gap)
- `textarea::mouse_drag_highlights_and_edits_wrapped_selection` (gap)
- `textarea::mouse_multiclick_preserves_the_initial_word_or_wrapped_line_when_reversing` (gap)
- `textarea::overwide_halfwidth_sound_marks_keep_the_final_cursor_visible` (gap)
- `textarea::render_highlights_apply_style_without_mutating_text`
- `textarea::scrolled_url_fragments_keep_the_offscreen_destination` (gap)
- `textarea::separator_after_full_line_hangs_before_following_text` (gap)
- `textarea::styled_text_rendering_does_not_overwrite_cells_outside_the_textarea` (gap)
- `textarea::tabs_render_as_spaces_and_align_with_cursor_snapshot` (gap)
- `textarea::text_rendering_does_not_overwrite_cells_outside_the_textarea` (gap)
- `textarea::trailing_spaces_wrap_without_letting_the_cursor_escape` (gap)
- `textarea::unicode_whitespace_separates_url_destinations` (gap)
- `textarea::url_hyperlinks_preserve_existing_highlight_styles` (gap)
- `textarea::vertical_navigation_clamps_saved_column_after_resize` (gap)
- `textarea::wheel_at_edges_preserves_caret_following_or_browsing_after_resize` (gap)
- `textarea::wrapped_navigation_with_wide_graphemes` (gap)
- `textarea::wrapped_url_fragments_keep_the_complete_destination` (gap)

### X-gap-text (20, под гэпом 19)

- `approvals_1::approval_headers_preserve_wrapped_url_destinations` (gap)
- `approvals_1::form_prompt_preserves_the_complete_wrapped_url_destination` (gap)
- `approvals_2::request_user_input_hidden_options_footer_snapshot` (gap)
- `approvals_2::request_user_input_long_option_text_snapshot` (gap)
- `approvals_2::wrapped_question_preserves_the_complete_url_destination` (gap)
- `cells_1::cpp_module_extensions_use_cpp_highlighting` (gap)
- `cells_2::completion_renders_with_dim_default_colors` (gap)
- `cells_2::typed_prompt_markdown_keeps_the_prompt_gutter_and_links` (gap)
- `cells_3::cyber_policy_error_event_narrow_snapshot` (gap)
- `cells_3::proposed_plan_cell_unwraps_markdown_fenced_table` (gap)
- `cells_4::session_header_truncates_halfwidth_directory` (gap)
- `cells_5::plan_update_does_not_split_url_like_tokens_in_note_or_step`
- `cells_5::plan_update_with_note_and_wrapping_snapshot` (gap)
- `cells_5::render_uses_wrapping_for_long_url_like_line` (gap)
- `chat_status_3::hook_status_reflows_without_displacing_controls_or_details` (gap)
- `onboarding::long_paths_keep_disclosure_and_choices_visible` (gap)
- `small_dialogs_1::wrapped_hook_command_preserves_the_complete_url_destination` (gap)
- `small_dialogs_2::renders_command_details_with_three_line_cap` (gap)
- `status::status_snapshot_includes_enterprise_monthly_credit_limit` (gap)
- `status::status_snapshot_wraps_halfwidth_kana_in_narrow_terminal` (gap)

### X-person (14, под гэпом 0)

- `chat_popups_2::permissions_selection_emits_history_cell_when_current_is_selected`
- `chat_popups_2::permissions_selection_history_snapshot_after_mode_switch`
- `chat_popups_2::permissions_selection_history_snapshot_full_access_to_default`
- `chat_status_2::status_line_estimated_thread_cost_footer_snapshot`
- `chat_status_3::status_line_setup_popup_hardcoded_only_snapshot`
- `chat_status_3::status_surface_preview_lines_hardcoded_only_snapshot`
- `chat_status_3::terminal_title_setup_popup_hardcoded_only_snapshot`
- `pane_footer_2::queued_messages_visible_when_status_hidden_snapshot`
- `pane_footer_2::status_and_queued_messages_snapshot`
- `pane_footer_2::status_indicator_visible_during_command_execution`
- `pane_footer_2::status_only_snapshot`
- `pane_footer_2::status_with_details_and_queued_messages_snapshot`
- `small_dialogs_1::wide_titles_preserve_reverse_video_selection_and_dim_index`
- `small_dialogs_2::selected_event_rows_use_the_shared_selection_style`

### X-w3-inline (31, под гэпом 23)

- `cells_1::command_truncation_ellipsis_does_not_include_transcript_hint`
- `cells_1::computer_activity_prioritizes_errors_and_images_without_reordering` (gap)
- `cells_1::output_lines_ellipsis_includes_transcript_hint`
- `cells_1::output_lines_handles_newline_dense_output_without_materializing_every_line`
- `cells_1::projected_content_preserves_full_rendering` (gap)
- `cells_1::streamed_output_renders_head_tail_previews`
- `cells_1::titled_image_call_keeps_error_and_full_title_when_narrow` (gap)
- `cells_1::truncate_lines_middle_keeps_omitted_count_in_line_units`
- `cells_1::truncated_ansi_sequence_does_not_hide_the_retained_tail`
- `cells_1::truncated_live_output_preview_and_transcript_snapshot`
- `cells_1::user_shell_output_is_limited_by_screen_lines` (gap)
- `cells_4::active_mcp_tool_call_snapshot` (gap)
- `cells_4::code_mode_tool_call_preserves_failure_details` (gap)
- `cells_4::code_mode_tool_call_uses_title_and_preserves_full_transcript` (gap)
- `cells_4::completed_mcp_tool_call_accepts_data_url_image_blocks` (gap)
- `cells_4::completed_mcp_tool_call_error_snapshot` (gap)
- `cells_4::completed_mcp_tool_call_image_after_text_snapshot` (gap)
- `cells_4::completed_mcp_tool_call_multiple_image_blocks_snapshot` (gap)
- `cells_4::completed_mcp_tool_call_multiple_outputs_snapshot` (gap)
- `cells_4::completed_mcp_tool_call_success_snapshot` (gap)
- `cells_4::completed_mcp_tool_call_wrapped_outputs_snapshot` (gap)
- `cells_4::multiline_command_wraps_with_extra_indent_on_subsequent_lines` (gap)
- `cells_4::single_line_command_over_highlight_limit_uses_plain_text_fallback` (gap)
- `cells_4::single_line_command_wraps_with_four_space_continuation` (gap)
- `cells_5::multiline_command_both_lines_wrap_with_correct_prefixes` (gap)
- `cells_5::multiline_command_without_wrap_uses_branch_then_eight_spaces` (gap)
- `cells_5::plan_update_without_note_snapshot`
- `cells_5::ran_cell_multiline_with_stderr_snapshot` (gap)
- `cells_5::stderr_tail_more_than_five_lines_snapshot` (gap)
- `chat_approvals_1::adjacent_exploration_groups_across_reasoning_live_and_replayed` (gap)
- `chat_approvals_1::exploration_reasoning_during_followup_command_stays_ordered` (gap)

### X-w3-pets (11, под гэпом 0)

- `chat_commands_3::slash_pets_in_tmux_shows_notice_and_preserves_draft`
- `chat_commands_3::slash_pets_on_old_iterm2_shows_upgrade_warning`
- `chat_commands_3::slash_pets_on_unsupported_terminal_shows_terminal_warning`
- `chat_commands_3::slash_pets_opens_picker`
- `chat_commands_3::slash_pets_with_arg_on_unsupported_terminal_shows_notice_without_selection`
- `chat_status_1::added_history_uses_pet_adjusted_terminal_width`
- `chat_status_1::ambient_pet_draw_uses_terminal_screen_area_not_short_inline_viewport`
- `chat_status_1::ambient_pet_hides_notification_text_overlay`
- `chat_status_1::ambient_pet_reduces_stream_width_and_composer_text_width`
- `chat_status_1::ambient_pet_screen_bottom_anchor_uses_terminal_bottom`
- `chat_status_1::ambient_pet_stays_hidden_until_a_pet_is_selected`

### X-w3-voice (9, под гэпом 0)

- `cells_2::spoken_artifacts_link_only_real_workspace_files_and_preserve_existing_urls`
- `cells_2::spoken_user_messages_have_a_red_chevron_without_changing_raw_text`
- `cells_2::spoken_windows_relative_paths_keep_original_text_and_link_the_workspace_file`
- `chat_status_1::finalized_voice_transcript_renders_beside_the_streamed_cell`
- `composer_4::draft_and_voice_composer_snapshots`
- `pane_footer_2::active_dashboard_preserves_live_colors_controls_and_width`
- `pane_footer_2::voice_controls_follow_configured_mute_binding_and_unbinding`
- `pane_footer_2::voice_footer_rendering_preserves_text_and_styles`
- `transcript_view_5::spoken_prompt_selection_keeps_its_fallback_contrast`

### X-w3-resume (4, под гэпом 0)

- `resume_1::ctrl_o_keeps_toggled_density_when_persistence_fails`
- `resume_1::footer_prioritizes_active_sort_timestamp`
- `resume_1::linked_worktree_sessions_follow_directory_scope`
- `resume_2::thread_colors_and_selection_contrast_in_both_picker_layouts`
