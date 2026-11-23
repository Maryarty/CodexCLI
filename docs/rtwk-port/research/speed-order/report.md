# speed-order: the order of the remaining work and when each test layer runs

(tm speed-order 01a2088f, 2026-11-23; written by the orchestrator from the researcher's result deliverable. Accepted by the person 2026-11-23: one land-check per landing by the lander, slow-tests task + nextest slow-timeout, porter cap 6.)

# speed-order: порядок оставшейся работы и момент каждого слоя тестов
Измерено: 128 транскриптов codex-porter с 11-22 (105 ч активных, паузы >15 мин исключены), git log --first-parent claude/tui-rtwk (46 посадок с 11-22 17:00), land-check-times.log (61 прогон 11-23), свои прогоны на e263606f0 (весь default-набор с временем каждого теста; весь oracle-набор). Скрипты в 152337fa7 воспроизводят каждое число.

## 0. Итог
1. Cargo = 55 % времени портера: тесты 29 %, сборка 11 %, land-check 5 %, фоновые cargo-ожидания 9 %. В прямых вызовах тестов 90 % — компиляция/линковка (test:scoped 731 вызов: 7,14 ч стены, 0,70 ч прогона, медиана 4 теста).
2. Чтение 23 % (19,4 ч думания модели перед 14 812 чтениями + 5,1 ч самих чтений). Модель всего 34 % (23 077 ходов по 5,6 с).
3. land-check: медиана 431 с (lint-build 68, nextest 351, макс 957); 61 прогон на 33 посадки 11-23 (1,85/посадку), 19 красных (31 %: 5 lint-build, 14 nextest/сценарии). Портеры ждали её 5,6 ч. 11-21 было 189 с.
4. Хвост default-набора: 3028 тестов = 1138 тест-с (384 с стены, 3 потока); 89 тестов >=3 с = 647 с (57 %). agents::the_center_draws_at_width_1..10 = 230 с (22-27 с каждый; 11-21 было 2,4-2,6 с: регрессия x9). screens::approvals 128, usage 106, select 93, resume 58.
5. Посадки с 11-22 17:00: 46 = функции 20, batch 4, фреймворк/core 6, переделка 7, тесты/флейки 8, docs 1. 11-23: 14 функций из 33; карта 110 -> 113 ✅.
6. Критический путь: block-commands (~29 открытых строк) и block-settings (15) не начаты; commands упирается в core-slices-migrate (HELD на вопросе о новых дверях для /plugins), которая держит и 5 детей popups.
7. Хабы (посадок с 11-22 / строк): core/src/app.rs 23/2771, session_page.rs 21/1173, core/src/threads.rs 21/3286, strings.rs 15/1149, actions.rs 11/420.

## 1. Время портера (porter_time.py)
модель 35,8 ч 34 % (54 % перед чтением, 20 % перед тестом/сборкой, 15 % перед правкой); тесты 30,8 ч 29 % (scoped 14,1; oracle-shard 5,4+0,7; wait wide 4,3; rules 2,5+0,4; pty 0,9); сборка 11,8 ч 11 % (build 7,9, clippy 2,2); фоновые без команды 9,1 ч 9 %; land-check 5,6 ч 5 %; чтение 5,1 ч 5 %; idle 3,3; правка/git/tm 3,1.
Формы тестовых вызовов (вызовов / медиана с / ч): без корня 499/23,2/5,78; summary.py --run 382/18,9/3,39; --test screens 246/18,8; -p codex-tui-core 221/26,9; --test oracle 312/8,9; --test rules 132/23,2; --test pages 123/15,3; --workspace 22/64,4.
Стоки: полный workspace-nextest в портерах 290 мин (100 мин в одном транскрипте chunk-status 11-22, до правила no-gate-per-step); land-check в 47 транскриптах по 3-17 мин; >=238 cargo-вызовов ждали слот memcap (медиана 40 с против 16); переориентация: 128 транскриптов на 46 посадок, до первой правки медиана 5,6 мин и 30 чтений, всего 12,6 ч (12 %).

## 2. Посадки против функций
функции: e263606f0 7204119df afd840c6e 5026ad24c 2b550ffdf 6e990d47c 8c492bcbc 2e28c3c29 197a80719 7fca83869 65b483168 b3de26c50 b46d66fa0 2fb8b0ea3 e0daf84db 46b0ad051 8f8c7acc5 2082fcf5d b9d5d1116 5cfedc782; batch f64d263e9 e8cba07ee 2b3a26103 b87e88f85; фреймворк 66b7c81a7 7399a9bad 3030194f1 0c34cc2fd ba78422cc 78bb8e3d4; переделка 760ecd94c d7baab717 ba5a017b3 1422b9cb0 dca49b8dc 2149237a6 6a180ea96; тесты/флейки e45ec99d8 e093acaf1 a50a6624c 6d118154c ed16724d9 33552855f f966d4846 828d229d6; docs 1253df3d0.
Карта на e263606f0: 289 строк, ✅113 🟡65 ❌106. Функция часто переводит ❌ -> 🟡; считать прогресс по строкам -> ✅ и закрывать строку целиком в блоке.

## 3. Слои тестов: цена и момент
- check --tests: 2,5-6,8 с — после каждой правки.
- один корень + фильтр (--test oracle -E 8,9 / pages 15,3 / screens 18,8 / core 26,9 с) — цикл правки; корень всегда назван (без него 23,2 с и 7 бинарей).
- --test rules 23,2 с — перед коммитом.
- KEY oracle по имени --test oracle -E 'test(=a)|test(=b)' 8,9 с — перед сдачей (вместо summary.py --run 18,9 с).
- pty-сценарий функции рядом с oracle ~30 с; fmt+clippy крейта ~25 с — перед сдачей.
- land-check 431 с — один раз на посадку, у посадчика, портер не ждёт.
- весь oracle-набор 141 с со сборкой (55 с прогон; 2621 тест: 971 pass, 297 n/a, 1342 pending, 7 todo) — после посадки, неблокирующе, diff множества pass с прошлым типом.
- probe stale / un-pend батчами — ночью / codex-filler.
Хвост: 2773 теста <1 с = 157 с; 166 по 1-3 с = 335; 73 по 3-10 с = 332; 16 >=10 с = 315. Срез хвоста до ~500 тест-с = nextest ~170 с, land-check ~250 с.
Флейк под нагрузкой 38: kit_proof::opening_a_script_screen_gives_the_same_rows_every_time, нет в known-red.

## 4. Порядок
Блоки (открытые строки): commands ~29 (chunk-commands 12, small-dialogs 5, submission 4, export 2, queue 2, mcp-inventory 2, background-terminals 2; не начат, area пересекается с migrate: core/src/app/session.rs, dialogs); popups ~27 (5 детей ждут migrate); cells ~26 (question-result 9, live-safety 5); transcript ~25 (transcript-2 8; find/select/copy-mode ждут transcript-2); status ~20 (usage-reset 6); composer ~19 (states 6, vim 5, images 4); settings 15 (не начат, от migrate не зависит); rest ~9 (inline-mode после чата).
Хаб-дети: session_page.rs — chunk-transcript-2, chunk-composer-states, chunk-commands-queue; actions.rs — submission, export, queue, settings, migrate; threads.rs — submission, queue. chunk-cells-question-result правит core/src/app.rs:1895-1930, но app.rs нет в её area.
Расписание, 5 слотов, до migrate: S1 migrate (без новых дверей); S2 cells: question-result -> live-safety -> session-info -> rendering-prefs -> banner-fallback -> reasoning; S3 transcript: transcript-2 первым -> find -> select-mode -> copy-mode -> pager -> backtrack; S4 status: indicators -> карточка -> usage -> usage-reset; S5 settings (перерезать S15). Мелкие дети popups (sweep, goal, feedback, windows-sandbox) — шестым портером после правок 1-2.
После migrate: S1 -> block-commands по слайсу на ребёнка (rename-view, export-view, submission, mcp, ps, small-dialogs, группы NotAvailable), queue последним; popups — worktree, quick-dials, permission-args, plan, core-popups; composer — vim/history/images/live-mentions сразу, states после transcript-2; rest последним. Филлеры — un-pend sweep-тестов функции батчем по шарду после её посадки.

## 5. Changes (по выигрышу)
1. Одна land-check на посадку, никто её не ждёт: портер сдаёт после гейта сдачи и берёт следующего ребёнка; land-check гоняет посадчик. Альтернатива на решение: зелёная проверка портера принимается, если тип менял только непересекающиеся файлы. Выигрыш ~28 прогонов x 431 с ≈ 3,4 слот-ч/день + 5 % времени портеров.
2. Срезать хвост default-набора: сначала регрессия x9 agents width 1..10 (230 с); страж slow-timeout {period=3s, terminate-after=2} в [profile.default] .config/nextest.toml или rules-тест бюджета. nextest 351 -> ~170 с, land-check ~430 -> ~250 с.
3. block-settings стартовать сейчас; block-commands веером по feature-слайсам сразу после migrate (44 строки без портера).
4. migrate без новых дверей: /plugins остаётся в select, остальное садится сейчас (держит 5 детей popups и commands). Решение человека.
5. session_page.rs цепочкой blocks в tm: transcript-2 -> composer-states -> commands-queue; расширить area question-result на core/src/app.rs.
6. Правила тестовых команд в fleet-brief: корень всегда; KEY oracle через --test oracle -E вместо summary.py --run. ~2 %.
7. Неблокирующий oracle-сторож после посадки (141 с): pass -> red = баг посадки.
8. Меньше рестартов: checkpoint несёт список read: с file:line (чтение 23 %, ориентация 12,6 ч).
9. Лимит портеров 5 -> 6-7 после п.1-2 (3 слота memcap сейчас узкое место).
Решают человек/оркестратор: п.1 (кто гоняет land-check), п.4 (/plugins), п.9 (лимит портеров).
