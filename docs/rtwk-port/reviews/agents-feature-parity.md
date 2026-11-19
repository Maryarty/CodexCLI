NOT YET — 68 пунктов: DONE 42 · PARTIAL 3 · MISSING 15 (4 из них дефекты в уже перенесённом коде) · PENDING 5 · N/A 3

# Agent command center: паритет функций (oracle rust-v0.170.0 против tui-rtwk @ fd750fd99)

Как проверял. Опись собрана из кода оракула: `agents_overview*.rs`, `agent_center/*`,
`keymap.rs:2126-2140` (`AgentsKeymap`), `keymap_setup/actions.rs:293-305`, `cli/src/main.rs:349-363,1084-1150,2595`.
Каждую строку сверял с кодом tui-rtwk и с тестами. Что запускал:
- тесты центра (`screens agents::*`, `agents_mouse`, `shell_agents`, core `agents`, `agents_live`, `agents_followups`): **130/130 pass**;
- профиль `pending`: **5 из 5 падают как задумано** (3 keymap, 2 reuse-carry);
- pty-сценарий `agents`: `agents_oracle`, `agents_oracle_is_stable`, `agents_parity` — **3/3 PASS**, `SCENARIO_STRICT=1 agents_parity` — **PASS**
  (RTWK_BIN=/tmp/parity-target/debug/codex-rtwk). Сценарий покрывает только просмотр, help и фильтры, действий в нём нет.

## Случай человека: «Untitled task», «Prompt: No prompt available.», при этом «Last message» есть

Откуда это у оракула.
- Подробности задачи у оракула читаются отдельно от списка. Он читает их для 10 самых свежих задач
  (`RECENT_DETAIL_LIMIT`, `agents_overview_threads.rs:34,38-55`), для загруженных (`thread/loaded/list`,
  `:367-392`) и для каждой задачи, о которой пришло уведомление (`:211-219`, `event_dispatch.rs:443,3251`).
  На каждую делает `thread/read` и `thread/turns/list {limit: 1}` (`agents_overview_threads.rs:421-509`).
- Prompt — это первый `UserMessage` последнего хода (`:455-465`, `user_message_display_from_inputs`
  + `preview_markdown`). Оракул сохраняет его в `activity.latest_prompt` (`agents_overview.rs:269-273`).
  Last message берётся из последнего `AgentMessage` того же хода (`:467-473`) и попадает в `last_messages` (`:238-241,498,506`).
- `Thread` строки заменяется тем, что вернул `thread/read` (`agents_overview.rs:274-275`), а у него свежий `preview`
  («Usually the first user message», `thread_data.rs:226`).
- Заголовок без имени = первая строка `name`, иначе `preview`, а «Untitled task» — только если пусто и то и другое
  (`agents_overview_view.rs:122-125`). У Prompt тот же запасной вариант: `latest_prompt`, иначе `preview`, иначе
  «No prompt available.» (`agents_overview_view.rs:558-570`).

У нас.
- `thread/read` (без turns) и `thread/turns/list` не вызываются нигде: в `core/src/screens/agents/`, `src/shell*`, `host/src` их нет.
  Единственный `ThreadRead` — `include_turns: true` для Stop (`wire.rs:62-66`).
- `Activity.latest_prompt` (`core/src/screens/agents/mod.rs:179`) никто не записывает: его только читают (`details.rs:121-124`),
  а в `notifications.rs:124-125` он подменяет буфер, и тот сразу выбрасывается (`let _ = buffer`).
- `Model.last_messages` (`mod.rs:301`) тоже никто не заполняет, его только чистят (`notifications.rs:157`).
  Поэтому «Last message» у человека пришёл из живого `ItemCompleted` (`notifications.rs:113-119`): задача была
  загружена этим же процессом.
- Заголовок и запасной Prompt берутся только из `preview` ответа `thread/list` с `use_state_db_only: true`
  (`sources.rs:150-172`). Если в state DB у задачи пустой preview, получаем «Untitled task» и «No prompt available.»,
  и `thread/read` этого уже не исправит.
- Итог: у оракула для этой задачи были бы заголовок из preview и Prompt из последнего хода. Пункты 32–34, 42, 67 в таблице ниже.

## Опись

| # | Функция (оракул) | Статус | Доказательство / чего нет |
|---|---|---|---|
| 1 | `codex agents` открывает центр сразу | DONE | `with_start_agents` (host/src/main.rs:49), сценарий `agents_parity` strict |
| 2 | при подключении задача не создаётся (нет строки `Untitled task`) | DONE | `start_thread_on_connect`; shell_agents `the_live_center_lists_both_sources` |
| 3 | `agents --help/-h`, `help agents` | DONE | tests/cli.rs (6 тестов) |
| 4 | `--no-daemon`: по умолчанию общий daemon app-server | MISSING | всегда встроенный сервер; флаг отвергается («unknown argument», src/lib.rs:103) |
| 5 | `--remote`, `--remote-auth-token-env` | MISSING | нет удалённого сервера |
| 6 | `--cd/-C DIR` (каталог новых задач) | MISSING | src/lib.rs:78-108 |
| 7 | `--no-alt-screen` | MISSING | src/lib.rs:78-108 |
| 8 | отказы CLI: prompt/images, конфликт remote, remote+локальные overrides, stdin/stdout не tty | MISSING | main.rs:1088-1148,2595 |
| 9 | Left на пустом composer открывает центр | DONE | navigation `left_on_an_empty_composer_opens_the_agents_page` |
| 10 | `/agents` | DONE | core/src/commands/dispatch.rs:339 (заметка: `/subagents` там же ведёт в центр, а у оракула открывает `AgentPicker`, slash_dispatch.rs:412) |
| 11 | глобальная клавиша `global.open_agents` (по умолчанию не назначена) | PENDING keymap | src/keymap пуст |
| 12 | Esc возвращает в чат, если чат под центром; подсказка «back» | DONE | `can_return`; `live_center_navigation_and_complete_hints` |
| 13 | колонки Tasks/Status/Updated, обрезка метаданных, разные ширины | DONE | `live_center_columns`, `_metadata_clips_at_grapheme_boundaries`, `the_center_draws_at_widths_one_to_ten` |
| 14 | только корневые сессии, субагенты внутри | DONE | `shared_overview_shows_only_root_sessions` (core и экран) |
| 15 | точка непрочитанного / кружок idle | PARTIAL | из `read_states` списка и `ThreadReadStateChanged`; нет перепроверки старых строк через `thread/read` (threads.rs:401-420) |
| 16 | значок `voice` | DONE | `live_center_columns` |
| 17 | цвета задач и их отключение | DONE | `overview_thread_colors_match_footer_and_respect_color_suppression` |
| 18 | фильтры Tab/Shift+Tab, счётчики, пустой фильтр | DONE | `filter_without_tasks_resets_...`, `a_click_on_a_filter_tab_filters`, сценарий `filter-*` |
| 19 | группировка `g`: Project→Directory→Status→Model | DONE | `overview_model_grouping_...`, `worktrees_overview_groups_by_project_or_directory` |
| 20 | группировка сохраняется в `tui.agents_overview_grouping` и читается при старте | MISSING | `Grouping::persisted/from_config` вызываются только в тестах; `CycleGrouping` (update.rs:40) команды записи не выдаёт, host значение не читает |
| 21 | порядок: статус, время обновления, ранги закреплённых | DONE | core `live_center_*_order*`, `pin_action_updates_shared_section_state` |
| 22 | «Show more», дозагрузка после архивации, повтор с тем же курсором | DONE | `paging_flow`, `overview_show_more_refills_archived_rows` |
| 23 | навигация: стрелки, j/k, ctrl+n/p/j/k, PgUp/PgDn, ctrl+b/f, Home/End, переход с конца на начало | DONE | `live_center_page_keys_and_wrap_follow_the_list_keymap`, `_pages_visible_rows_*` |
| 24 | мышь (сверх оракула) | DONE | agents_mouse.rs (8 тестов) |
| 25 | `/` и F3 открывают поиск; Esc/ctrl+c закрывают и возвращают выделение | DONE | `live_center_search_row_appears_only_while_editing`, `ctrl_c_closes_a_line_first_then_quits` |
| 26 | повторный F3 закрывает поиск | MISSING (дефект) | оракул: view.rs:734-742 (`searching` → `on_ctrl_c`); у нас `Act::Search` → `set_mode(Search)`, а он при той же моде ничего не делает (mod.rs:899-903, 571-573) |
| 27 | поиск по истории на сервере, «Searching history…», «Show more matches», ошибка | DONE | core `live_center_show_more_navigation_search_and_loading` |
| 28 | задержка 180 мс, новый поиск отменяет старый | DONE | `the_search_waits_...`, `a_newer_search_drops_...`, `typing_a_word_asks_the_server_once` |
| 29 | вставка в строку поиска / переименования | DONE | rtwk `TextField`; тест через `t.paste` (agents.rs:738) |
| 30 | vim-режим в строке переименования | PENDING | vim composer'а в tui-rtwk нет (view.rs:296 оракула) |
| 31 | детали: заголовок, статус, Model, Reasoning, Project, Branch | DONE | снимок `agents_overview` |
| 32 | **Prompt** (первое сообщение пользователя в последнем ходе) | MISSING (дефект) | см. раздел выше: `latest_prompt` никто не пишет |
| 33 | Last message из прочитанных данных (не только из живого события) | MISSING | `last_messages` никто не пишет |
| 34 | заголовок безымянной задачи из `preview` после `thread/read` | PARTIAL | только preview из state DB (`sources.rs:150-172`) |
| 35 | внимание: Needs attention, превью запроса, Waiting…, `Agent:` дочернего | DONE | `agents_overview_details_show_available_attention_without_expanding_rows` |
| 36 | Latest activity (жирный заголовок reasoning) | PARTIAL (дефект) | `notifications.rs:121-127` ищет `**…**` в одном delta, буфера нет; оракул копит до 512 символов по (turn,item,index) (details.rs:72-99). Его тест делит заголовок на два delta (`agents_overview_tests.rs:1300`), у нас заголовок не появится; тест не перенесён |
| 37 | Markdown в Prompt и Last message | PENDING reuse-carry | `agents_overview_details_render_markdown` (pending) |
| 38 | OSC 8 ссылки при переносе и обрезке | PENDING reuse-carry | `agents_overview_preview_links_...` (pending) |
| 39 | usage: токены, credits, USD, кэш 60 с, тайм-аут 65 с, неполный биллинг | DONE | `selected_usage_is_cached`, `the_usage_fetch_gives_up_as_the_oracle_does`, `usage::tests::incomplete_billing_...` |
| 40 | бюджет высоты: активность важнее Prompt, «…» | DONE | view.rs:841-857, снимок attention |
| 41 | `ThreadReverted` сбрасывает Prompt, Last message, токены | MISSING | `observes` (notifications.rs:34-49) его не читает |
| 42 | перечитывание задачи после её уведомления (`refresh_thread_ids`) | MISSING | нет `thread/read` (см. выше) |
| 43 | Enter/Right открывают задачу в чате (resume), центр закрывается | DONE | `overview_right_opens_current_or_highlighted_task`, shell `resumes` |
| 44 | `o` открывает resume picker | MISSING (дефект) | оракул: view.rs:766-769 `OpenResumePicker`; у нас `Act::Open` → `Accept(sel)`, то есть открывается выделенная задача, а в пустом центре ничего (mod.rs:933-937). Пакет (agents-live.md, строка `empty_command_center...`) пишет иначе, тест проверяет только снимок |
| 45 | `n` новая задача в каталоге выделенной | DONE | `command_center_new_actions_use_selection_...` |
| 46 | `w` новый worktree (если включены), занятость, ошибки | DONE | `w_creates_a_worktree_and_opens_a_task_in_it`, `overview_worktree_creation_busy_state`, `a_worktree_that_cannot_be_made_...` |
| 47 | `f` fork, копия становится текущей | DONE | `fork_opens_the_copy_of_the_selected_task` (боковая беседа и очередь ввода — состояние чата) |
| 48 | `r` переименование (Enter, пустое имя не принимается) | DONE | `live_center_rename_retains_target_...`, `hiding_rename_target_...` |
| 49 | `x` стоп (только активная; ActiveTurn → Interrupt) | DONE | `lifecycle_shortcuts_target_filtered_task_in_any_state` |
| 50 | `a` архив: подтверждение 1/2, текст cloud, прогресс, диалог ошибки | DONE | `archive_confirmation_number_keys_act_immediately`, `lifecycle_failure_dialogs`, `modal_consumes_input_...`, `cloud_archive_...` |
| 51 | Backspace — удаление, подтверждение только Enter | DONE | `delete_task_confirmation_and_failure_snapshots` |
| 52 | `h` скрыть до явного resume | DONE | `hidden_task_stays_hidden_...` (риск: без п.44 вернуть скрытую задачу через центр нельзя) |
| 53 | `p` закрепить/открепить, раздел Pinned, только если сервер умеет | DONE | `the_pins_of_the_server_reach_the_center_and_its_help`, `pins_follow_the_cursor_...` |
| 54 | во время удаления, worktree или offline работают только перемещения | DONE | `overview_right_preserves_editors_and_offline_state`, `modal_consumes_input_...` |
| 55 | выделение после удаления/скрытия — соседняя строка | DONE | `external_removals_preserve_adjacent_selection` (матрицы оракула с F-клавишами ждут keymap) |
| 56 | конфликт attach → открыть read-only и повторить | MISSING | `command_center_attach_conflict_opens_read_only_and_retries`, слайс чата/сессии |
| 57 | ошибка resume оставляет центр с текстом ошибки | DONE | refresh.rs:279-283; `command_center_action_failures_remain_visible` |
| 58 | ошибка обновления видна внизу и пропадает при успехе | DONE | `command_center_refresh_failure_is_inline_and_clears_on_success` |
| 59 | пустой центр, загрузка, до согласия на папку | DONE | `empty_command_center_can_open_resume_picker` (снимок), `agents_overview_before_folder_consent_...` |
| 60 | уведомление о старом сервере (и в узком окне) | N/A сейчас | отрисовка проверена тестами, но встроенный сервер всегда той же версии; станет нужным с п.4/5 |
| 61 | Reconnecting / reconnect failed | N/A сейчас | только из seed; встроенному серверу переподключаться не к чему; станет нужным с п.4/5 |
| 62 | help `?`: группы, условные пункты (worktree, pin), 1/2/3 колонки, ярлык `delete` | DONE | `live_center_navigation_and_complete_hints`, strict-сценарий `help` |
| 63 | подсказки внизу (help, back, move, open/rename, new) | DONE | тот же тест, снимки `*_hints` |
| 64 | настраиваемые клавиши `agents.*`/`list.*` для действий и подсказок | PENDING keymap | 3 pending-теста |
| 65 | общий обзор нескольких backend'ов (daemon + встроенный), ранги закреплённых между ними, действие уходит владельцу | MISSING | 6 тестов `shared_overview_*`/`refreshing_one_backends_pins_*`/`overview_keeps_rows_and_routes_actions_to_the_owner` не перенесены; связано с п.4 |
| 66 | ctrl+c вне строки — выход | DONE | `ctrl_c_closes_a_line_first_then_quits` |
| 67 | `thread/loaded/list` в первом обнаружении | MISSING | нигде не вызывается |
| 68 | ctrl+l (list move_right) | N/A | у оракула ctrl+l раньше перехватывает глобальный `clear_terminal` (input.rs:823) |

## Что чинить, по порядку (MISSING и PARTIAL)

1. **Чтение подробностей (случай человека; п.32, 33, 34, 42, 67, 15).**
   - Core: `Command::ReadDetails{threads}`. Shell: на каждую задачу `thread/read {include_turns:false}` + `thread/turns/list {limit:1}`, до 16 параллельно.
   - Ответ: `Ev::Details{thread, latest_prompt, last_message, unread}`. Он записывает `activity.latest_prompt` и `last_messages` и заменяет `Thread` (вместе с preview).
   - Кого читать: 10 самых свежих задач и не-`NotLoaded` (`detail_thread_ids`), загруженные (`thread/loaded/list` при первом обнаружении) и задачи из наблюдаемых уведомлений, кроме Reverted/TokenUsage/ReadState.
   - Тесты брать из оракула: `shared_overview_keeps_rows_and_replays_changes_over_stale_reads`, `shared_overview_seeds_once_and_retains_locally_resumed_history`.
2. **`o` → resume picker (п.44).** `Act::Open` должен открывать `resume::route()` так же, как `chat/open.rs:44`, а не `Accept`. Нужен тест клавиши в пустом и в непустом центре. Выбранная в picker'е сессия должна снимать скрытие и закрывать центр (оракул: `resume_picker_round_trip_*`).
3. **Буфер reasoning (п.36).** В `Activity` хранить `(turn, item, index, buffer ≤512)` и искать первый `**…**` в буфере. Перенести `agents_overview_reasoning_uses_existing_events_and_expires_with_attachment`.
4. **Сохранение группировки (п.20).** Читать `tui.agents_overview_grouping` при старте (`Ev`), на `CycleGrouping` выдавать `Command::PersistGrouping(persisted())`, запись в config делает host (ConfigEdits). Довести тест `overview_grouping_persists_across_config_reloads` до записи и повторного чтения.
5. **F3 закрывает поиск (п.26).** В `Mode::Search` клавиша `Act::Search` вызывает `cancel_editing`.
6. **`ThreadReverted` (п.41):** сбросить `latest_prompt`, `last_messages`, `usage.tokens`.
7. **Конфликт attach → read-only (п.56):** решение в слайсе чата/сессии.
8. **CLI `--cd/-C`, `--no-alt-screen` (п.6, 7)** и отказы с текстами оракула (п.8).
9. **Daemon / `--remote` / общий обзор (п.4, 5, 65; с ними оживают п.60, 61).** Это большой архитектурный пункт: решать человеку.
10. PENDING по владельцам: Markdown и OSC 8 (reuse-carry, п.37, 38), keymap (п.11, 64), vim (п.30).

Что решить человеку и оркестратору:
- нужен ли daemon/remote-режим (п.4, 5, 65) для паритета VS1;
- пункты 1–6 — дефекты уже перенесённого слайса, им нужна задача fix;
- в пакете `parity/agents-live.md` неверно сказано про `o` (п.44) и устарело «reasoning not modelled» (п.36).
