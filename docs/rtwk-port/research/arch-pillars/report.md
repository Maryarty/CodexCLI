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

## Шаблонный код и дубли на стыках

Подсчёт на land/feature-1 (48d868eaa, после слияния c3 и c4) плюс fleet/fw1-c2. В feature_list! 15 слайсов.

### (а) Шаблонный код, написанный руками в каждом слайсе

| Что | Где (примеры) | Строк в волне 1 | Чем генерировать | Экономия |
|---|---|---|---|---|
| wire/decode на каждый RPC: Command, Reply, request(), reply() | core/src/features/rename/wire.rs (46 строк на один ThreadSetName); mcp_inventory/wire.rs (126); все features/*/wire.rs | ~924 | `rpc_list!` в core/src/protocol: корни Call/Response из списка методов (протокол связывает вариант, params и ответ в app-server-protocol/src/protocol/common.rs:215, но трейта на метод нет; workspace не трогаем, список живёт в tui-rtwk). Слайс пишет `cx.call(thread, Call::SetName(p), ctx)`, FeatureCtx даёт идентичность фичи | ~500-600, и ~40 строк на каждый будущий RPC |
| Зеркальные перечисления протокола | mcp_inventory/wire.rs server() и др. | ~55 | реэкспорт типов данных протокола через core::data (ADR-013 это разрешает) | ~55 |
| Never-типы и заглушки absurd | core/src/features/processes/mod.rs (80 строк, 8 Never-типов, absurd wire/decode); так же в каждом слайсе без какой-то части | ~64 в core | декларативный макрос `feature!`, заполняющий отсутствующие ассоциированные типы Never, а методы — absurd (умолчания ассоциированных типов в Rust нестабильны) | ~64 |
| Заглушки FeatureView (Args = Never, absurd args/open) | src/features/security_setup/mod.rs (33) и подобные | ~56 + ~100 на части листа у слайсов без листа | FeatureView + FeatureSheet: второй трейт реализуют только слайсы с OPENS | ~150 |
| Проводка Fact/Intent/Note через хабы | SessionIntent: ~17 вариантов слайсов из 41 (~45 строк); ветки session.rs (~47 строк, 12 ссылок `crate::features::`: session_intent :154, slash() :381, small() :716); ветка OPENS dispatch.rs:460 (~25); варианты слайсов в ThreadEv (Goal, Mcp, McpLagged, Command); in_process.rs: 13 ссылок `features::` и ~41 строка функций фактов (:632, :639, :651, :678, :787) | ~160 + host ~41 | feature_list! генерирует маршрут Command -> фича из OPENS/VERBS (P1); host_capability_list! — двери host (P6); host-половина слайса (P7) | ~115 удаляется, ~41 переезжает |
| Строки регистрации | core features/mod.rs (mod + строка feature_list!), src/features mod + feature_dirs!, tests/all.rs mod, hub_budget.txt | ~5-6 строк в 4-5 файлах на слайс, ~80 | build.rs сканирует каталоги features/ | ~80, плюс все конфликты слияния из-за порядка mod (сборка волны 1) |
| Строки текста (strings.rs, note_words) | src/features/*/strings.rs | отдельно не считались | не генерируются: у оракула эти тексты тоже есть | 0 |

Собственный шаблонный код каркаса (Never, заглушки, регистрация, FeatureView) мал: ~400-500 строк на 15 слайсов. Основной перерасход — wire-код на каждый RPC (~1 тыс.) и двустороннее деление core/UI с зеркальными типами.

### (б) Дубли на стыках: где не хватало двери и слайс обходил через хаб или копировал код

| Стык | Где | Строк | Опора | Итог |
|---|---|---|---|---|
| Ветки session.rs для слайсов (Rename, Export, Copy, Goal, Logout/Ide/Daybreak/DebugConfig через small(), Hooks/Title/Statusline/Model/ResumeNamed через slash()) + варианты SessionIntent + ветка dispatch | core/src/app/session.rs:154/381/716; commands/dispatch.rs:460 | ~47 + ~45 + ~25 | P1 | ~115 удаляется |
| /side живёт в core/src/app/side.rs (fork, linked, start_failed, discard, sync), называет features::side и краснит features_are_named_only_through_the_list | fw1-c2 core/src/app/side.rs (251), app.rs +23 (AppCtx::NewConversation{side}, `app.features.side.is_side()`), session.rs +11 | ~285 | P5 (AppCx::spawn Fork{ephemeral} + thread_opened + discard) + P4 | ~285 переезжает в слайс, ~80 склейки удаляется |
| Ручные вызовы observe | side::observe в app.rs apply_event; side::sync в show_held_thread; side.rs observe :175 / observe_request :186 | часть из 285 | P4 (объявленная подписка) | внутри ~80 |
| Ретрансляция через Wish в UI | core/src/features/settings/mod.rs:99 Wish::{Status, Model, OpenThread, KeymapConflict}, хаб передаёт дальше | ~60 | P9 PublicVerb + P5 open_thread | ~60 и класс ошибок «ретрансляция разошлась» |
| Селекторы слайсов в каркасе | core/src/features/cx.rs: +74 строки в волне 1; cx.rs:55-62 daybreak; cx.rs:193-194 `super::recap::Message` (каркас называет слайс; cx.rs исключён в layers.rs:660) | ~74 | P3 (селекторы у слайса) + правило framework_names_no_slice | ~74 переезжает в слайсы |
| Типизированные факты host | host/src/settings.rs:20-23 называет features::settings::{Fact, Facts, Row, ...} (красное c3); in_process.rs 13 ссылок | ~41 + c3 | P7 (host/src/features/<f>.rs называет только features::<f>; FACT_SENDERS становится этим образцом) | переезжает, правило зеленеет |
| Переименованные варианты | правка слияния 82507d701: Ev/Line переименованы в 14 файлах (+77/-71), потому что правило adr013 ищет по голому имени; Ask::Editor | ~150 правок на слияние | разрешение по полному пути в правиле adr013 + P6 | ~150 на волну |
| Тики часов через хабы | src/services/shell.rs:163 ambient() каждые 20 с (CLOCK_EVERY :158) -> Agents::Clock, Status::Clock; ветки в screens/agents/mod.rs:390, screens/chat/facts.rs:136, screens/cells.rs:158, host/src/viewer.rs:63 | 4 ветки + 2 варианта | P2 (селектор now + wake_at) | варианты и ветки удаляются, в тестах поддельные часы |
| Не построенные возможности host | src/services/effects.rs:188-199: SuggestTitle -> Done::Title(None), StructuredTurn -> Err("not built"). /recap всегда падает, имя треда не предлагается | 12 | P5 временный структурированный ход (эфемерный thread/start -> turn/start с outputSchema -> последнее сообщение -> unsubscribe), RPC жизненного цикла в core | видимое пользователю расхождение; человек 2026-11-24: исправить (tm fix-recap-title) |
| Потеря/восстановление потока треда | fw1-c2 host/src/in_process.rs:467, host/src/cloud.rs:132 отбрасывают ThreadStreamDisconnected/Reconnected; оракул обрабатывает их в tui/src/app/app_server_events.rs:130-170 (history_may_be_incomplete, строка ошибки, maybe_send_next_queued_input при восстановлении, переподключение простаивающего облачного треда) | 2 ветки-отбрасывателя | P8: ThreadEv lost/restored, ThreadCore.stream_lost + блокировка очереди, ThreadView::stream_lost() | видимое расхождение на облачных бэкендах; человек 2026-11-24: исправить (tm fix-stream-lost) |

Итого по волне 1: ~250-300 строк удаляется на стыках, ~400 переезжает в слайсы, ~150 правок на каждое слияние. Вместе с (а) — на ~1,0-1,2 тыс. строк меньше на сегодняшних 15 слайсах.

### Сравнение с оракулом (строк на фичу, оракул против порта)
- daemon: ~207 против 459; goal: ~250 против 1042; feedback: ~940 против 724 (UI порта не закончен); plugins: ~3,1 тыс. против 5469.
- Комментарии — 11% кода фич порта против ~0,4% у оракула; пустые строки — 7%.
- Прогноз: 165 тыс. / 0,55 ≈ 300 тыс., около 1,2x оракула (~248 тыс.). Источники перерасхода по убыванию: (1) двустороннее деление с зеркальными типами плюс wire на каждый RPC — закрывают rpc_list! и core::data; (2) стиль doc-комментариев — человек 2026-11-24 решил оставить как есть; (3) обходы на стыках — закрывают P1/P3/P4/P5/P7/P9; (4) шаблонный код каркаса, небольшой — закрывают макрос feature!, FeatureSheet и build.rs. С закрытыми (1), (3) и (4) прогноз ~1,0-1,05x оракула (без учёта комментариев).

Для fw-core-hubs: rpc_list! и реэкспорт core::data идут в прогон C (вместе с P6); макрос feature!, FeatureSheet и регистрация через build.rs — в прогон 1 fw-ui-registry. Правила: wire_is_generated (никаких новых features/*/wire.rs после появления rpc_list!), framework_names_no_slice, host_half_names_one_feature.

## Решения человека (2026-11-24)

1. Пробуждение — собственный интент слайса (Elm): `wake_at -> (Now, Intent)`, приходит в обычный update.
2. Уведомления — типизированный Event: `project(&ServerNotification) -> Option<Event>` + `on_event`.
3. PublicVerb в core: короткий белый список, `AppCx::verb`, runtime доставляет владельцу; Wish только для «покажи».
4. Ленты host — второй список (`host_feed_list!`, host/src/feeds/<f>.rs).
5. Все три прогона fw-core-hubs сейчас, A -> B -> C по очереди, fw-ui-registry параллельно с A.
6. Стиль комментариев — оставить как есть.
7. Красное thread_rpcs_only_in_threads (settings/wire.rs:145) принято в волне 1, чинит B (P5 open_thread).
8. Два видимых расхождения (/recap и имя треда; потеря/восстановление потока) — исправить.
