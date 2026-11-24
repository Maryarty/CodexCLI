# Опоры архитектуры: ADR-012/013 против полной модели Elm / Redux / Flutter

Задача tm `arch-pillars` (codex-reviewer, Opus), 2026-11-24. Код прочитан на land/feature-1 (волна 1: c1, c5-c9) и в fleet/fw1-c2, c3, c4. Пути внутри codex-rs/tui-rtwk/. Отчёт записал оркестратор: harness не дал субагенту записать файл.

## Вывод

Сторона записи совпадает с образцами:
- `feature_list!` собирает слайсы, как `combineReducers`;
- outbox `AppCx` (`Out`) — это `Cmd` из Elm;
- `Remote<T>` — это AsyncValue;
- `SurfaceWish` + `WishShown` — это push-for-result;
- счётчик Interest — это autoDispose.

В rtwk уже есть Focus/Shortcuts/Actions, `show_dialog::<T>`, Navigator, providers и Timer с поддельными часами.

Не хватает стороны входа: пять видов входящих данных не доходят ни до одной функции, а четыре входа — хабы. Каждый обход в волне 1 соответствует одной из опор ниже.

## Таблица опор

**Есть:**
- 1 — update/reducer на модуль (Feature, features/mod.rs:118; feature_list! :270);
- 2 — Cmd/thunk (Out, features/cx.rs:158; исполняет app/features.rs:58);
- 3 — цепочки шагов через Ctx;
- 16 — отмена по соглашению: устаревшее отбрасывается по Ctx или OpKey+gen;
- 17 — нормализованное состояние (ThreadSet, Remote по ключу);
- 18 — AsyncValue (features/remote.rs:9);
- 19 — autoDispose (on_interest, mod.rs:249);
- 20 — широковещательные события, намеренно только жизненного цикла;
- 22 — сохранение (Ask::ConfigWrite);
- 26 — освобождение таймеров UI (rtwk release_timers).

**Частично:**
- 4 — init: State по умолчанию, данные пушит host (c8, c1).
- 8 — селекторы: каждое чтение — новый метод в общем cx.rs (c4 +4, c5 terminals :146). ThreadCx не видит фактов приложения: c7 нужны plan/auth/now, c9 — turn_id/rollout.
- 9 — чтение чужого слайса (c4 через with_parts).
- 10 — навигация: слайс не может открыть тред (c3 гонит Wish::OpenThread через UI).
- 13 — возможности host: Ask/Done написаны руками, исполнители в 3 файлах, маршрутизация в app.rs:866. Задеты c1 BackgroundServer, c2 Editor (переименован в обход правила), c4 (правка app.rs).
- 14 — факты host: in_process.rs называет 5 функций в 13 строках (FACT_SENDERS, layers.rs:702).
- 15 — жизненный цикл: нет шагов connected / lost / shutting-down (c2, c8).
- 21 — глаголы между модулями: Wish::Status/Model у c3 идут через UI.
- 24 — ячейки функций: note_words отдаёт только String (ячейка-ссылка c9, ячейка /ps c5).
- 27 — нет поддельных часов в core.

**Нет:**
- 5 — подписки на время: shell.rs:163 опрашивает часы раз в 20 с и передаёт в Agents/Status по имени; c7 заблокирован.
- 6 — внедряемые часы: нет now в AppView.
- 7 — поток сервера для функций: threads::project (threads.rs:2002) раскладывает уведомления по вариантам ThreadEv в хабе. c2 вызывает side::observe руками, c5 свернул варианты ThreadEv.
- 11 — эффекты треда из слайса: у ThreadCx есть только request, нет start_turn/interrupt (Retry/Rewrite у c6).
- 12 — команды с аргументами и команды-глаголы: OPENS понимает только голую команду, открывающую лист. 3 ветки в session.rs на main и ещё 12 от c2/c3/c4 плюс помощники slash()/small().
- 23 — реестр действий: все клавиши через одно перечисление keys.rs:20.
- 25 — вклады в слоты: функция не может добавить строку футера, уведомление usage или шапку compaction.

## Новое правило

`features_are_named_only_through_the_list` (layers.rs:753) ловит путь `features::<f>`, но не поле `app.features.<f>`: `app.features.side.is_side()` у c2 его проходит. Новое правило: `app_names_no_feature_state`.

## Изменения

- **P1, команды.** `const RUNS` + `fn slash(state, Slash{command,args,source}, cx)`, генерируемые runs_of/apply_slash, один вариант `SessionIntent::Feature`. Правила: SLASH_ROUTERS пуст, one_owner_per_command. SessionIntent 41 → около 26; уходят все ветки слайсов и помощники.
- **P2, время (Elm).** Now в приложении, ставится через AppEv::Clock; `AppView::now()`; `wake_at -> Option<(Now, Intent)>` и thread_wake_at; AppFx::wake несёт ближайший момент; в shell один таймер; шаг скрипта `clock` для тестов. Правило: no_clock_loops (ratchet: варианты Clock в экранах → 0). Разблокирует c7.
- **P3, селекторы.** ThreadCx несёт AppFacts (now, account, tui, выбор статуса); типизированные read-only ручки; селекторы пишет сам слайс; `features()` отдаёт состояние до шага. Правило: view_methods_budget. Чтения c4/c5/c7/c9 переезжают в слайсы.
- **P4, поток.** type Event, `project(&ServerNotification)`, on_event; для запросов сервера только наблюдение; один ThreadEv::FeatureEvent. Mcp/McpLagged уходят из хаба, позже Goal; observe-вызовы c2 уходят.
- **P5, жизненный цикл.** AppCx::open_thread и spawn(ThreadSpawn, ctx), ответ в thread_opened; ThreadCx::start_turn, interrupt. RPC жизненного цикла остаются в threads.rs. Правило: app_ctx_names_no_feature. app/side.rs c2 (9 функций) уходит в слайс; c6 может сделать Retry/Rewrite.
- **P6.** host_capability_list! генерирует Ask/Done, маршрутизацию и диспетчер исполнителей. 6 возможностей из port-map — 6 строк списка.
- **P7.** host/src/feeds/<f>.rs на функцию (Startup, ConfigWritten, AccountUpdated); in_process.rs не называет слайсов. Закрывает красное правило c3.
- **P8.** on_connection(Up / Lost / ShuttingDown).
- **P9.** AppCx::verb(PublicVerb), короткое перечисление под бюджетом, вместо передачи через UI.
- **UI.** FeatureView::cell, slot, actions.
- **Итого:** методов Feature 14 → около 21, Out 6 → 9. На каждый — свой абзац в ADR-012.

## Порядок

1. **fw-core-hubs A** (~220 ходов): P1 с выносом веток, P3, P2. Нужен src/services/shell.rs из области fw-ui-registry.
2. **fw-core-hubs B** (~200): P4, P5, P8, два новых правила. Разблокирует side и Retry/Rewrite у c6.
3. **fw-core-hubs C** (~180): P6 с 6 возможностями, P7, P9. Должен влиться до voice, terminal-features, inline-mode.
4. **fw-ui-registry** — параллельно с A, 2 прогона: методы cell / slot / actions.

A, B и C делят app.rs и hub_budget.txt, поэтому идут по очереди.

## Вопросы человеку

1. Пробуждение отдаёт собственный интент слайса, как в Elm (рекомендация), или отдельный хук on_wake.
2. Уведомления через типизированный Event с project/on_event (рекомендация) или сырой on_notification.
3. PublicVerb в core (рекомендация) или оставить передачу через UI.
4. Feeds host отдельным вторым списком или const HAS_FEED.
5. Сразу три прогона, или сейчас только A, а B и C после первых UI-пакетов волны 2.
