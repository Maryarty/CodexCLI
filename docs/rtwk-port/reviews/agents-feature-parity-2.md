NOT YET — 76 пунктов (68 прежних + 8 новых): DONE 62 · PARTIAL 3 · MISSING 5 · PENDING 5 · N/A 1 (в прогоне 1: 42/3/15/5/3 из 68)

# Agent command center: паритет функций, прогон 2 (oracle rust-v0.170.0 против tui-rtwk @ c3372c5b9)

Как проверял. Взял ту же опись из 68 пунктов. Каждый пункт заново сверил с кодом (`core/src/screens/agents/*`,
`src/screens/agents/*`, `src/lib.rs`, `host/src/*`, `src/shell/*`) и с тестами. Ещё сверил с кодом оракула
`agents_overview_threads.rs` и `agents_overview_tests.rs` и сравнил имена всех 76 тестов оракула с нашими.
Отсюда 8 новых пунктов (69–76). Что запускал (CARGO_TARGET_DIR=/tmp/parity2-target):
- `nextest --workspace`: 745 run, **744 pass, 1 fail**. Упал `shell_agents::a_connection_that_does_not_start_ends_the_app`:
  тест нестабилен и падает в 4 запусках из 7 (полный прогон и 3 из 6 повторов). Подробнее в разделе «Риски».
- Тесты центра (core `agents*` и `backend_switch` — 94, `screens agents::*` — 27, `agents_mouse` — 10, `shell_agents` — 32,
  `shell_backends`/`shell_connection`/`shell_cycle` — 9, bin `codex-rtwk` — 17): всё зелёное, кроме того же нестабильного теста.
- Профиль `pending`: **5 из 5 падают, как задумано** (3 keymap, 2 reuse-carry).
- pty-сценарий `agents` (RTWK_BIN=/tmp/parity2-target/debug/codex-rtwk): `agents_oracle`, `agents_oracle_is_stable`,
  `agents_parity` — **3/3 PASS**; `SCENARIO_STRICT=1 agents_parity` — **PASS**. Теперь в checkpoint `overview` есть строка
  Prompt («Investigate the flaky upload test»), и она совпадает с оракулом. Сценарий по-прежнему проверяет только
  просмотр, help и фильтры, без действий.

Случай человека из прогона 1 («Untitled task», «No prompt available.») закрыт. Подробности теперь читаются через
`thread/read {include_turns:false}` + `thread/turns/list {limit:1}` (`reads.rs`, wire.rs:113-136). Читаются
10 самых свежих задач и задачи не в `NotLoaded`, задачи из `thread/loaded/list` и задачи, о которых пришло уведомление.
Уведомления, пришедшие во время чтения, накладываются поверх ответа (`overlay`).

## Опись

| # | Функция (оракул) | Статус | Изменилось с прогона 1 | Доказательство / чего нет |
|---|---|---|---|---|
| 1 | `codex agents` открывает центр сразу | DONE | — | сценарий `agents_parity` strict |
| 2 | при подключении задача не создаётся | DONE | — | `start_thread_on_connect` |
| 3 | `agents --help/-h`, `help agents` | DONE | — | tests/cli.rs |
| 4 | по умолчанию общий daemon, `--no-daemon` встраивает сервер | DONE | MISSING→DONE | host/src/target.rs `Mode::resolve`, `the_command_center_uses_the_daemon_unless_told_otherwise` |
| 5 | `--remote` (ws/wss/unix), `--remote-auth-token-env` | DONE | MISSING→DONE | target.rs:146 `resolve_remote_addr` + 4 перенесённых теста |
| 6 | `--cd/-C` | DONE | MISSING→DONE | src/lib.rs:118, host/src/main.rs:58 |
| 7 | `--no-alt-screen` | DONE | MISSING→DONE | main.rs:85 → `TerminalOptions.alternate_screen`; inline-режим проверен только разбором флага, pty-теста нет |
| 8 | отказы CLI | PARTIAL | MISSING→PARTIAL | есть: `--no-daemon`+`--remote`, token-env без `--remote`, токен по ws на не-loopback. Нет: prompt/images (у нас «unknown argument»), «conflicting remote server endpoints», remote + локальные overrides, «stdin/stdout is not a terminal» (cli/main.rs:1088-1148, 2595) |
| 9 | Left на пустом composer | DONE | — | |
| 10 | `/agents` | DONE | замечание | `/subagents` теперь ведёт в `Target::AgentPicker`, но вместо picker'а показывается заглушка «Not implemented yet» |
| 11 | `global.open_agents` | PENDING keymap | — | src/keymap пуст |
| 12 | Esc возвращает в чат, подсказка back | DONE | — | |
| 13 | колонки, обрезка, ширины | DONE | — | |
| 14 | только корневые сессии | DONE | — | |
| 15 | точка непрочитанного / idle | DONE | PARTIAL→DONE | `unread` из ответа `thread/read` (reads.rs:104-108) |
| 16–19 | voice, цвета, фильтры, группировка `g` | DONE | — | |
| 20 | группировка сохраняется в `tui.agents_overview_grouping` | DONE | MISSING→DONE | `SaveGrouping`, `Ev::Grouping`; `the_configured_grouping_is_the_one_the_center_opens_with` |
| 21–25 | порядок, Show more, навигация, мышь, `/` и F3 | DONE | — | |
| 26 | повторный F3 закрывает поиск | DONE | MISSING→DONE | `f3_toggles_the_search_line` |
| 27–29 | поиск на сервере, задержка 180 мс, вставка | DONE | — | |
| 30 | vim в строке переименования | PENDING | — | vim composer'а нет |
| 31 | детали: заголовок, статус, Model, Reasoning, Project, Branch | DONE | — | |
| 32 | **Prompt** последнего хода | DONE | MISSING→DONE | reads.rs:132, сценарий `overview`. Замечание: склеиваются только Text-входы, `user_message_display_from_inputs`/`decode_task_links` не перенесены |
| 33 | Last message из прочитанного хода | DONE | MISSING→DONE | reads.rs:133-135 |
| 34 | заголовок из `preview` после `thread/read` | DONE | PARTIAL→DONE | reads.rs:91-104 |
| 35 | внимание, Waiting…, `Agent:` | DONE | — | |
| 36 | Latest activity (`**…**` по нескольким delta) | DONE | PARTIAL→DONE | `latest_activity_is_the_first_bold_run_across_deltas`. Сброс при отсоединении не смоделирован: у центра нет «attachment» |
| 37 | Markdown в Prompt/Last message | PENDING reuse-carry | — | pending-тест |
| 38 | OSC 8 ссылки | PENDING reuse-carry | — | pending-тест |
| 39–40 | usage, бюджет высоты | DONE | — | |
| 41 | `ThreadReverted` сбрасывает Prompt, Last message, токены | DONE | MISSING→DONE | notifications.rs:194-204, старый ответ Turns отбрасывается (`stale`) |
| 42 | перечитывание задачи после уведомления | DONE | MISSING→DONE | notifications.rs:111-130 |
| 43 | Enter/Right — resume задачи | DONE | — | |
| 44 | `o` открывает resume picker | PARTIAL | MISSING→PARTIAL | клавиша работает (`o_opens_the_resume_picker`). **Но в живом приложении picker пуст и ничего не открывает**: shell не загружает в него сессии (rows/Load приходят только из fixtures), а выбор пишет `resume::Model.resuming` (core/src/screens/resume.rs:108), который никто не читает. Скрытая задача (`h`) через picker не возвращается |
| 45–51 | n, w, f, r, x, a, Backspace | DONE | — | |
| 52 | `h` скрыть до явного resume | DONE | — | риск остаётся: вернуть скрытую задачу можно только Enter в центре (см. 44) |
| 53–55 | pin, блокировка ввода, выделение после удаления | DONE | — | |
| 56 | конфликт attach → read-only и повтор | MISSING | — | в коде нет; слайс чата/сессии |
| 57–59 | ошибка resume, ошибка обновления, пустой центр | DONE | — | |
| 60 | уведомление о старом сервере | DONE | N/A→DONE | src/shell/version.rs, tests/shell_connection.rs |
| 61 | Reconnecting / Reconnect failed | DONE | N/A→DONE | src/shell/reconnect.rs, `agents_reconnect` (3), shell_connection |
| 62–63 | help `?`, подсказки внизу | DONE | — | |
| 64 | настраиваемые `agents.*`/`list.*` | PENDING keymap | — | 3 pending-теста |
| 65 | общий обзор нескольких backend'ов: владелец строки, ранги pin, маршрут действия к владельцу | DONE | MISSING→DONE | `agents_backends` (7), `agents_shared` (2), `shell_backends`. `overview_keeps_rows_and_routes_actions_to_the_owner` перенесён по частям; `hidden_system/title_thread_does_not_refresh_shared_overview` не перенесены (по коду поведение то же: эфемерный ThreadStarted не читается) |
| 66 | ctrl+c вне строки — выход | DONE | — | |
| 67 | `thread/loaded/list` при первом обнаружении | DONE | MISSING→DONE | refresh.rs:285, reads.rs:68-73 |
| 68 | ctrl+l | N/A | — | глобальный clear_terminal оракула |
| 69 | Esc в picker'е, открытом из центра, возвращает в центр с его состоянием (группировка, поиск, выделение) | MISSING | новый | оракул `cancelling_resume_picker_preserves_command_center_state`. У нас `push_replacement` (src/screens/agents/mod.rs:946) убирает центр, и `Navigator::maybe_pop` (resume/mod.rs:484) уводит в чат |
| 70 | `ThreadSettingsUpdated` обновляет cwd/model/provider строки и перечитывает её | MISSING | новый | оракул agents_overview_threads.rs:180-208; у нас его нет в `observes` (notifications.rs:86-106) |
| 71 | `ThreadUnarchived`/`ThreadStarted` снимают пометку «удалена» | MISSING | новый | оракул :129, :147; у нас `removed` очищается только при `Intent::Open` (update.rs:36), разархивированная задача не появится до повторного открытия центра |
| 72 | Ctrl+X b переключает backend пустой беседы, контрол с меткой над composer | DONE | новый | `backend_switch` (7), `shell_cycle::empty_thread_backend_chord_preserves_draft_until_first_submission` |
| 73 | `-c` overrides: исключение daemon и уведомление | PARTIAL | новый | исключение есть (target.rs:41, 2 перенесённых теста). Уведомление и ошибка Cloud попадают в `/warnings`, а оракул пишет их строкой в историю чата (`StartupWarningsCell`). `--enable/--disable/--search` не перенесены |
| 74 | ошибка запуска daemon завершает процесс текстом оракула | DONE | новый | `a_start_failure_ends_the_app`; второй тест нестабилен (см. «Риски») |
| 75 | Cloud-backend под условием оракула, его строки после переключения | DONE | новый | host/src/cloud.rs `gate`, `a_cloud_owned_row_archives_only_the_task_from_a_local_session` |
| 76 | переключение корневой сессии через центр сохраняет состояние чата каждой задачи (черновик, vim yank, локальные настройки, разрешения невыгруженных, cwd при холодном resume) | MISSING | новый | тесты оракула `root_switch_*`, `resume_picker_round_trip_preserves_each_threads_input`, `overview_selection_applies_user_permissions_only_to_unloaded_threads`, `overview_cold_resume_honors_working_directory_selection`; в core нет черновика по задачам; слайс чата/сессии |

## Что осталось, по порядку

1. **Resume picker из центра не работает (п.44, 69, 52).** `o` открывает пустой список. Выбор строки ничего не возобновляет
   (`resuming` никто не читает). Esc уводит в чат, а не в центр. Нужны:
   - shell загружает сессии в `resume::Model` (Loading → Listed);
   - выбор идёт через тот же путь resume, что и Enter в центре (снимает скрытие, закрывает центр);
   - центр открывает picker через `push`, а не `push_replacement`, чтобы Esc возвращал в центр с его состоянием.
   Тесты оракула: `cancelling_resume_picker_preserves_command_center_state`, `resume_picker_round_trip_*`.
2. **`ThreadSettingsUpdated` (п.70)**: обновить cwd/model/model_provider строки и перечитать её. От этого зависят группировка Project/Directory и Model в деталях.
3. **`ThreadUnarchived` / `ThreadStarted` снимают `removed` (п.71).**
4. **Отказы CLI (п.8)**: prompt/images, conflicting endpoints, remote + локальные overrides, stdin/stdout не tty — тексты оракула.
5. **Стартовые уведомления в истории чата (п.73)**: исключение daemon по `-c` и ошибка Cloud сейчас видны только в `/warnings`.
   Это видимое расхождение. Его либо закрывает строка `StartupWarningsCell` в чате, либо оно уходит в gap-строку
   (решение человека: расхождения не принимаются).
6. **Состояние чата при переключении корня (п.76) и конфликт attach → read-only (п.56)** относятся к слайсу чата/сессии.
7. PENDING по владельцам: Markdown и OSC 8 (reuse-carry, п.37, 38), keymap (п.11, 64), vim (п.30).
8. Мелкие замечания к DONE: в Prompt идут только Text-входы (п.32); сброс Latest activity при отсоединении не смоделирован (п.36);
   inline-режим `--no-alt-screen` без pty-теста (п.7); 2 теста `hidden_*_does_not_refresh_shared_overview` не перенесены (п.65);
   `/subagents` показывает заглушку (п.10).

## Риски

- **Нестабильный тест** `tests/shell_agents.rs:795 a_connection_that_does_not_start_ends_the_app` (коммит 9a204877b).
  В полном прогоне и в 3 из 6 повторов приложение после `inject(epoch, StartFailed)` и `settle()` продолжает работать.
  Причина не установлена. Возможны две: ingress с этим epoch отброшен как устаревший (после agents-backend-switch
  epoch ведётся по каждому backend отдельно), или `settle` на фальшивых часах не дожидается канала. Нужно `wait_for` или проверка epoch.
  Пока он красный примерно в половине прогонов, «nextest 745 green» в отчётах porter'а — удача, а не гарантия.
- Пакет `parity/agents-live.md` пишет, что `o` открывает picker, и не упоминает, что picker в живом приложении пуст (п.44).

## Что решить человеку и оркестратору

- Нужна задача fix для пунктов 1–3 (дефекты уже перенесённого центра) и для нестабильного теста.
- Пункт 5: строка в истории чата или gap-строка.
- Пункты 4 и 76 — отдельные задачи (CLI и слайс чата/сессии).
