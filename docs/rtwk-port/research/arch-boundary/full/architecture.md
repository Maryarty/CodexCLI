# tui-rtwk: целевая архитектура (ADR-013 «граница по вводу-выводу» в полном виде)

Статус: **проект на утверждение человеком**, 2026-11-23, tm `adr013-review`. После утверждения документ заменяет целевую часть `docs/rtwk-port/architecture.md`:
- навигационную часть ADR-002 и поправки 2026-11-18 и 2026-11-19b;
- область ADR-003;
- форму ADR-012;
- раздел «Layers».

ADR-001, 004–011 не меняются, кроме мест, названных в §14. Обоснование и найденные ловушки — [review.md](review.md), схемы — [diagrams.md](diagrams.md) (ссылки вида «d8»). Пути кода от `codex-rs/tui-rtwk/`. Имена с пометкой *(новое)* в коде пока не существуют.

Документ самодостаточен: чтобы строить приложение, другие документы читать не нужно. Исследования, на которые он опирается, перечислены в §17.

---

## 0. Что строим и в каком порядке смотреть

- **Оракул** `codex-rs/tui` (ratatui) задаёт только вид и поведение. Его внутреннее устройство (нижняя панель со стеком видов, шина `AppEvent`, один `select!`) моделью не служит.
- **tui-rtwk** — нативное приложение на rtwk, построенное как приложение Flutter. Неизменяемые виджеты описывают UI, у элементов есть `State`, раскладка идёт по constraints, перестраивается только изменившееся поддерево.
- **Бэкенд** — `codex-app-server-client` и `codex-app-server-protocol` как есть. Фронтенд — чистая замена.
- **Цель документа** — сделать классы ошибок невозможными (типом или модульной приватностью) или пойманными (тестом-правилом). Слова без правила — долг с задачей.

---

## 1. Принципы

### 1.1 Как это называется во Flutter

| во Flutter | у нас | где |
|---|---|---|
| `main()`, platform channels | host: бинарь, in-process app-server, факты конфига | `host/` |
| repository / data layer, исполнитель эффектов | shell: единственный, кто делает IO и спавнит | `src/services/` |
| Bloc/Redux store, одно на приложение | `CodexApp` + `apply_app` | `core/src/app.rs` |
| Riverpod `.family` (стор на сущность) | `ThreadModel` на поток + `threads::apply` | `core/src/threads.rs` |
| `Cmd` / side effects как данные | `AppFx`, `ThreadFx`, `AppCx`-outbox | core |
| поток событий Bloc | ingress: один упорядоченный канал | `Ingress`, `AppCx::channel` |
| `context.select` / `watch` | провайдеры-проекции модели | `src/app/view.rs` |
| `BlocListener` | presenter-ы: запросы сервера, желания показать | `src/app/presenters/` *(новое место)* |
| `BlocListener` с `listenWhen: seq` | слушатели директив | `src/widgets/directives.rs` *(новое)* |
| Riverpod `autoDispose` + `ref.onDispose` | интерес поверхности со счётчиком | `src/widgets/interest.rs` *(новое)* |
| Riverpod `Mutation` / состояние операции | `Op` по доменному ключу | `core/src/features/<f>/` |
| feature-first папки | срез фичи: две половины, одна строка в списке | `core/src/features/<f>/`, `src/features/<f>/` |
| `Navigator` + `PageRoute` | страницы | `src/pages/` |
| Navigator 2.0 `pages:` в `State` | `SheetStack<Page>` в шторке | `src/widgets/sheet_stack.rs` *(новое)* |
| `Scaffold.bottomSheet` / `showModalBottomSheet` | шторка: докнутая (`SheetHost`) или модальная | §6, О2 |
| `showDialog` | диалог | `rtwk::overlay::show_dialog` |
| `Autocomplete`, `Overlay` | поповеры композера | `src/pages/session/popovers/` |
| `TextEditingController`, `ScrollController`, `TabController`, `ExpansionController`, `FocusNode` | те же контроллеры rtwk | `State` или `ThreadUi` |
| `PageStorage` / контроллеры над страницей | `ThreadUi` на поток | `src/widgets/thread_ui.rs` |
| `Shortcuts` / `Actions` / `Intent` | те же в rtwk; карта из keymap пользователя | §7 |
| `AppLocalizations.of(context)` | `strings::of(cx)` | `src/**/strings.rs`, `src/l10n/` |
| чистые Dart-библиотеки (форматтеры, парсеры) | `core::kit` | `core/src/kit/` *(новое место)* |
| `bloc_test` / widget test / golden / integration_test | core-тесты / `Tester` / oracle-шарды / pty-сценарии | §12 |

### 1.2 Пять родов состояния

| род | пример | владелец | кто меняет |
|---|---|---|---|
| **Д. Домен**: факты сервера и host и то, что из них выведено | транскрипт, каталог плагинов, лимиты, конфиг, keymap | core | `apply` по событию |
| **З. Запись запроса**: тикет, параметры как ключ ответа, статус операции | `Remote<Detail>` по `PluginKey`, `Listing` по `query`, `ops[Install(k)]` | core | `apply` по глаголу и по ответу |
| **Ж. Жизненный цикл и разовые сигналы домена**: шаги запуска, очередь сообщений, желания показать, директивы контроллерам | `start_held`, `queue`, `SurfaceWish`, `Directive<Restore>` | core | `apply` |
| **И. Интерес**: какая поверхность сейчас хочет данных | `Interest(Plugins{cwd})` со счётчиком | core (счётчик), UI (виджет `Interest`) | `Intent::Interest(Open/Close)` |
| **Н. Нарисованное**: курсор, прокрутка, вкладка, страница и стек шторки, свёрнутость, плотность, подсветка, текст поля до отправки, открыта ли поверхность | `ListController`, `TabController`, `SheetStack` | UI: `State` виджета или `ThreadUi` | жест в виджете, локальный `setState` |

### 1.3 IO-тест: где живёт значение

Вопросы задаются по порядку, и первое «да» решает:
1. Это ответ или факт сервера или host, или выведено из них? → **Д**, core.
2. Это параметр запроса или статус операции, без которого ответ не к чему приложить? → **З**, core. Это **кэш по параметру**, а не «текущее X»: core хранит `Map<Key, Remote<T>>`, а не «открытая деталь».
3. От этого зависит шаг жизненного цикла (старт потока, очередь, запуск) или это сигнал, который домен посылает поверхности, — открыть её или положить текст в контроллер? → **Ж**, core.
4. Домен должен знать, нужны ли данные прямо сейчас (загрузка при открытии, опрос, перезапрос после переподключения)? → **И**: счётчик в core, сам факт открытия в UI.
5. Иначе → **Н**, UI.

Уточнения, которые снимают спорные случаи:
- Текст поля ввода всегда **Н** (контроллер). Core получает его только глаголом при отправке или поиске (`Submit`, `Search{query}`) и никогда не пишет обратно. Если домену нужно изменить текст, он шлёт `Directive` (§4.6).
- «Другой экран читает» — не довод. Довод — кто **пишет**. Значение, которое пишут только жесты одной поверхности, — это **Н**, даже если его читают двое: тогда его поднимают к общему предку в UI (`ThreadUi`, `UiState`), а не в core.
- Ошибка, видимая до следующего действия пользователя: факт — **Д** с `seq`, «погашено» — **Н** (UI помнит последний погашенный `seq`).

### 1.4 Инварианты (каждый закреплён правилом, §13)

1. **Один писатель.** Состояние приложения меняет только `apply`/`apply_app` на UI-потоке. Все входы идут через один ingress в порядке поступления (R10).
2. **Core без UI и IO.** В core нет rtwk, tokio, часов, файлов и прозы. Это держит компилятор (манифест) и сканы.
3. **IO только в shell.** Спавн, RPC, файлы, буфер обмена, часы — только в shell (R1, R11, R12).
4. **Аргументы — id, не данные** (R14). Маршрут, шторка или диалог получает id и читает данные через провайдеры. Копию берёт только редактор, и это его черновик.
5. **Ни одного тикета в UI.** UI сопоставляет ответы по доменному ключу операции (§4.5).
6. **Нарисованное не попадает в core.** Скан по типам, достижимым из корней домена (§13).
7. **Слова только в UI** (ADR-010). Core отдаёт ключи заметок и ошибок как типизированные значения.

---

## 2. Слои и модули

Рабочее пространство `codex-rs/tui-rtwk` (ADR-001): пакеты `codex-tui-core` (`core/`), `codex-tui-rtwk` (UI, `src/`), `codex-rtwk` (`host/`). Знание течёт в одну сторону: host → UI → core; core → protocol, serde, std.

### 2.1 Карта (d13)

| модуль | отвечает за | владеет | входы | выходы | может называть | не может |
|---|---|---|---|---|---|---|
| **host** `host/src` | `main`, `arg0`, tokio-рантайм, in-process app-server (`in_process.rs`), облако (`cloud.rs`), факты конфига и запуска (`facts.rs`, `launch.rs`), эффекты host (`effects.rs`) | клиент app-server, рантайм | CLI, окружение, конфиг | `Ingress` с эпохой; ответы `effects::Done` | всё | спавнить вне shell-API (`r1_the_host_neither_spawns_nor_allows`) |
| **UI · app** `src/app/` | корень композиции (`run.rs`), `UiApp { core, ui, link }`, `view` (провайдеры + `Navigator`), presenter-ы, debug-меню | `UiState`: `nav_key`, `thread_ui`, `lang`, presenter-ы, флаги старта | `Ingress` (через `ingest`), жесты через `Hs` | `AppEv` в ingress, `AppFx` в shell | core, rtwk, shell (`link`) | протокол в видах |
| **UI · services/shell** `src/services/` | единственный исполнитель эффектов: RPC (`shell.rs`), буфер обмена, open_url, поиск файлов, история, часы, fake-backend для тестов | соединения, подписки, эпоха каждой подписки | `AppFx` | события с тикетом и эпохой в ingress | core, протокол, tokio, `Cx`/`DialogResult` rtwk для спавна | виджеты (`shell_names_no_widget`) |
| **UI · pages** `src/pages/<p>/` | 5 страниц: Startup (шаги onboarding, import), Session (транскрипт, композер, футер), Agents, Resume, Usage | `State` страниц: Н | проекции, `ThreadUi` | глаголы через `Hs` | core (проекции, интенты), widgets, свои `strings.rs` | протокол, shell, IO, `apply`, `core::screens::*::wire` |
| **UI · sheets** `src/sheets/<s>/` | общие шторки и диалоги: approvals (presenter), dialogs, settings, pets, voice, общий select для небольших меню хаба | `State` шторки, `SheetStack` | проекции, аргументы-id | глаголы | как pages | как pages |
| **UI · features** `src/features/<f>/` | видимая половина фичи: `open(cx, args)`, её шторка и страницы, слова заметок и меню (`strings.rs`) | `State` своей шторки | `FeatureStates` (срез фичи) | `Intent::Feature(<f>::Intent)` | core::features::<f> (типы), widgets | чужие фичи, протокол |
| **UI · widgets** `src/widgets/` | общие кирпичи: `Picker` (+ его `List/Item/Span/Role`, `layout`, переехавшие из core), `session_list`, `options`, `tab_strip`, `thread_ui`, `sheet_stack`, `interest`, `directives`, `markdown` | ничего доменного | пропсы | события пропсов | rtwk, `core::kit`, `core::data`, `core::ids` | `core::screens`, `core::features`, pages (`widgets_never_import_pages`) |
| **UI · keymap, l10n, theme** | разрешённый keymap → `Shortcuts`; таблицы строк (`Lang`, `Tables<T>`), `TwkLocalizations`; токены стиля | — | `Keymap` (проекция), `UiState.lang` | — | rtwk | литералы слов и цветов вне таблиц |
| **core · app** `core/src/app*` | `CodexApp`, `apply_app`, `ingest` (эпоха R1), маршрутизация ответов, сессия, очередь запросов сервера, интересы, желания показать, факты | `session`, `ThreadSet` (приватно), `requests`, `screens`, `features`, `outgoing`, `interests` *(новое)*, `wishes` *(новое)*, `keymap`, `tui` | `Ingress` | `AppFx`; проекции через `&`-геттеры | protocol, serde, kit | rtwk, IO, прозу |
| **core · threads** `core/src/threads*` | `ThreadModel` (создаётся только в `ThreadSet::open`), `apply`, `project` (протокол → `ThreadEv`), транскрипт (R4), очередь и старт ходов, `ThreadAsk` (жизненный цикл RPC), директивы потока | на поток: `core`, `slots` (доменные части экранов), `features` (части фич), `seal` | `ThreadEv` | `ThreadFx` | как app | — |
| **core · features** `core/src/features/<f>/` | домен фичи: глаголы, каталоги, `Remote`, `Op`, заметки, факты, хуки жизненного цикла; одна строка в `feature_list!` | `State`, `ThreadPart` | `Intent`, ответы, факты, хуки | `AppCx`-outbox: `ask`, `effect`, `note`, `thread`, `settled`, `wish` *(новое)* | `AppView`/`ThreadView`, kit | `CodexApp`, `ThreadModel`, `AppFx`, UI-формы (`Menu/Pick/Prompt`) |
| **core · screens** `core/src/screens/<dir>/` | только домен экранов (Д/З/Ж): списки resume, лимиты usage, шаги onboarding, запросы approvals, каталоги select (модели, профили), popovers (совпадения файлов, история) | `ScreenStates` (срезы), `ThreadSlots` | `ScreenIntent` (глаголы), события | эффекты через `AppFx` | как features | поля отрисовки (Н) |
| **core · commands** `core/src/commands*` | slash-роутер: текст → `Target` + глагол (`SessionIntent`, `Intent::Feature`); фильтр команд (kit) | — | текст отправки | `Target`, глагол | — | открывать поверхности |
| **core · keymap** `core/src/keymap/` | разбор `tui.keymap.*`, контексты, аккорды, лидер, разрешение конфликтов | `Keymap` (в `CodexApp`) | факт конфига | `Rc<Keymap>` | — | rtwk-клавиши |
| **core · protocol, rpc, ids** | отображение протокола, `Ticket`, `Reply<T>`, `Pending::resolve`, `ConnEpoch`, `RequestKey`, newtypes id (R13) | — | — | — | protocol | — |
| **core · data** `core/src/data/` *(новое)* | простые доменные значения, общие для kit, screens и виджетов: что можно упомянуть и что список `@` показывает (`Mention`, `Candidate`, `Row`, `Selection`), задачи и выборы `$`, запрос и ответ поиска файлов, что сказал файл истории (`Store`, `Lookup`) | ничего: значения без логики экрана и без состояния отрисовки | — | — | std, protocol-типы данных, `core::ids` | любой другой модуль core (`adr013_data_names_only_ids`) |
| **core · kit** `core/src/kit/` *(новое место)* | чистая логика без состояния приложения: vim, paste_burst, навигация по истории (`History`), кодек упоминаний, фильтр команд (`CommandPopup`), машина backtrack, нечёткий поиск, форматирование | ничего; типы kit принадлежат тем, кто их держит (UI) | аргументы | значения | std, protocol-типы данных, `core::data`, `core::ids` | `core::screens`, `core::features`, корни домена (`CodexApp`, `ThreadModel`, `*States`) (`adr013_kit_names_no_screen`) |
| **core · fixtures** `core/src/fixtures/` | JSONL-скрипты (`seed`, `screen`, `notify`, `request`, `respond`, `reply`, `lagged`; UI-шаги `key`, `resize`, `ui` *(новое)* отдаются тестеру как есть), `Sandbox` | — | скрипт | состояние через живой `ingest` | — | строить состояние мимо `ingest` |
| **tests** | `core/tests` (домен), `tests/sheets`, `tests/pages` (виджеты), `tests/kit` + `tests/oracle` (чёрный ящик против оракула), `tests/rules` (правила), `tests/scenarios` (pty), `perf/` | — | — | — | — | — |

### 2.2 Что видит фича (ADR-012 в форме C)

`trait Feature` (core) — ассоциированные типы `State`, `ThreadPart`, `Intent`, `ThreadIntent`, `Command`, `Reply`, `Ctx`, `Note`, `Fact`, `Wish` *(новое)*, `Interest` *(новое)*. Шаги:
- `intent`, `answered`, `effect_done`;
- `on_fact`, `on_interest(state, interest, open, cx)` *(новое)*;
- `on_resumed`, `thread_intent`, `thread_answered`, `on_turn_end`, `on_interrupted`, `on_settled`;
- `note_tone`, `wire`, `decode`.

**Уходят**: `Menu`, `Pick`, `Prompt`, `choose`, `picked`, `prompt_submitted`, `AppCx::open`, `AppCx::prompt`, `FeatureView::menu_list`, `FeatureView::prompt_words`. Хаб теряет `Menu::Feature`, `Picked::Feature`, `PromptKind::Feature`. Бюджет `hub_budget.txt` уменьшается.

Видимая половина `src/features/<f>/`:
- `pub fn open(cx, args: <f>::OpenArgs)` — аргументы-id и начальная страница;
- шторка `<F>Sheet` (`StatefulWidget`, `eq`) на `SheetStack<Page>`;
- `note_words`, `strings.rs`.

Slash-роутер возвращает `Target::Feature(FeatureTarget)` и (необязательно) глагол. Страница сессии вызывает `features::open(cx, target)` (сгенерировано `feature_list!`).

---

## 3. Владельцы состояния (полная таблица, d9)

| состояние | род | владелец | читают | меняют | переживает |
|---|---|---|---|---|---|
| сессия: эпохи, активный поток, статус, backend-ы | Д/Ж | `CodexApp.session` (`Rc<SessionSlice>`) | `select::<SessionSlice>` | `apply_app` (активный поток — только там, `the_shown_thread_is_set_only_in_apply_app`) | всё, кроме выхода |
| поток: транскрипт, ход, очередь, steers, директивы | Д/Ж | `ThreadModel` (через `ThreadSet::open`) | `select::<ThreadModel>` активного; `AppView::thread(id)` | `ThreadEv` → `threads::apply` | смену потока, переподключение (снимок заменяет ходы, R4) |
| доменные части экранов на поток | Д | `ThreadModel.slots` (`ThreadSlots`) | проекция потока | `ScreenThreadEv` | как поток |
| запросы сервера (approvals, elicitation, user input) | Д | `CodexApp.requests` (ключ `(BackendId, ConnEpoch, ThreadId, ServerRequestId)`) | presenter, шторка запроса | `apply_app`; на `attach` удаляются запросы этого backend | до ответа, разрешения или переподключения |
| наши RPC в полёте | З | `CodexApp.outgoing: Pending<AppCtx>` | никто вне core | `apply_app`; на `attach` → `Failed(Disconnected)` *(новое, F6)* | до ответа или переподключения |
| каталоги и факты фич, `Remote<T>` по ключу, `Op` по `OpKey` | Д/З | `FeatureStates` (срез `Rc` на фичу *(новое)*) | `select` по срезу фичи | шаги `Feature` | смену потока; переподключение (перезапрос интересов) |
| домен экранов (resume `Listing`, usage, onboarding, select-каталоги, popovers) | Д/З/Ж | `ScreenStates` (срез `Rc` на экран *(новое)*) | `select` по срезу | `ScreenIntent` → `apply_app` | как фичи |
| интересы | И | `CodexApp.interests: Map<InterestKey, u32>` *(новое)* | core | `Intent::Interest` из виджета `Interest` | переподключение (перезапрос) |
| желания показать | Ж | `CodexApp.wishes: VecDeque<SurfaceWish>` *(новое)* | `SurfacePresenter` | `AppCx::wish`, `apply_app`; снимается presenter-ом (`Intent::WishShown(seq)`) или устаревает по `scope` | до показа или смены `scope` |
| директивы контроллерам (восстановить черновик, префилл, засеять историю, очистить отправленное) | Ж | `ThreadModel` (`Directive<T>` с `seq`) | слушатели `widgets/directives.rs` | `threads::apply` | смену потока (видимый `seq` в `ThreadUi`) |
| keymap, `[tui]` | Д | `CodexApp.keymap`, `tui` | провайдеры | `AppEv::Keymap/Tui` | всё |
| стек страниц приложения | Н | `Navigator` (ключ `UiState.nav_key`, G15) | `Navigator::of` | push/pop из виджетов | всё |
| стек и страница шторки, вкладка, курсоры, прокрутка, подсветка, плотность, свёрнутость | Н | `State` шторки или страницы; контроллеры rtwk; `SheetStack` | — | жесты | только жизнь элемента |
| ожидание своей операции | Н | `State` шторки: `awaiting: Option<(OpKey, Gen)>` | — | emit глагола; ответ в домене | жизнь шторки |
| черновик композера, история-навигация, kill ring, vim, режимы транскрипта (Ctrl+T, backtrack), прокрутка транскрипта, виденные `seq` директив | Н | `ThreadUi` по `ThreadId` | поддерево потока | контроллеры | смену потока; удаляется с потоком |
| presenter-ы (показанный запрос, последнее желание, время последней клавиши) | Н | `UiState.presenters` | — | presenter-ы | всё |
| язык | Н (факт UI) | `UiState.lang` | `strings::of(cx)` | раз при старте | всё |

---

## 4. Пути

### 4.1 Событие сервера (d5)
`app-server` → host (`in_process.rs`) → `Ingress { backend, epoch, ev }` → shell → `UiApp::ingest` → `CodexApp::ingest`. Там сверяется эпоха (R1): чужая — отбросить. Дальше `apply_event` → `threads::project` / `project_unrouted` → `threads::apply(ThreadEv)` и `screens::project_thread` → доменные части. Результат — `AppFx`.

После `ingest` UI делает:
1. `follow_core` — создаёт `ThreadUi` новых потоков и удаляет их у закрытых *(новое)*;
2. presenter-ы: запросы, желания;
3. `shell::run_effects(fx)`;
4. view перезапускается, проекции получают новые `Rc`, перестраиваются только `select`-нувшие.

### 4.2 Жест → глагол (d6)
Жест обрабатывает сам виджет: `Shortcuts/Actions` или мышь. Если жест меняет только **Н**, это локальный `setState` или вызов контроллера, и до core он не доходит. Если меняет **Д/З/Ж**, виджет делает `cx.emit(&hs.<handler>, verb)`. Обработчик кладёт `AppEv::Intent(...)` в ingress под текущей эпохой (R10): глагол не обгонит уже полученное событие. Глаголы называют намерение в терминах домена (`Install{key}`, `Search{query}`, `NeedMore`, `OpenThread(id)`), а не жест (`Choose(row)`, `Cursor(i)`, `Toggle`) — правило `adr013_core_intents_name_no_gesture`.

### 4.3 Эффект и ответ
`apply` кладёт эффект как данные:
- `AppFx`/`ThreadFx` — RPC с тикетом из `TicketMint`;
- `effects::Ask` — эффекты host.

Shell выполняет их через rtwk-tokio. Ответ приходит в ingress с тикетом и эпохой отправки → `Pending::resolve` (R1) → шаг владельца (`answered`, `thread_answered`, `Reply::screen`). Старый тикет отбрасывается.

### 4.4 Желание показать (d10)
Домен решает, что поверхность должна появиться (запрос сервера, установка требует авторизации приложений, стартовый промпт). Запросы сервера идут через `RequestQueue`, всё остальное — через `SurfaceWish { seq, scope, kind }`; фича пишет `cx.wish(<f>::Wish)`. `SurfacePresenter` в UI — обобщение `src/sheets/presenter.rs`; решает он по модели в `ingest`, а не по проекции в том же flush (G14). Дальше:
- если поверхность закрыта — открыть её с начальной страницей;
- если открыта — она сама увидит желание через свой срез и перейдёт на страницу;
- presenter шлёт `WishShown(seq)`, core снимает желание.

Порядок как у оракула: запрос сервера встаёт поверх открытой шторки, после ответа шторка снова сверху (маршруты в стеке).

### 4.5 Операция фичи и её страница
1. Шторка шлёт `Install{key}` и запоминает `awaiting = (OpKey::Install(key), gen_before)`.
2. Core: если `ops[key]` в полёте, игнорирует (двойное нажатие). Иначе `ops[key] = Op { gen+1, Running }` и `ask`.
3. Ответ: `ops[key] = Done(..)` или `Failed(..)`, заметка в транскрипт, при необходимости `wish`.
4. Шторка выводит страницу как f(свой `SheetStack`, `awaiting`, `ops[key]`). Пока `ops[key].gen <= gen_before`, страница «загрузка» берётся из `awaiting`.
5. Если шторку закрыли, ответ всё равно ложится в домен. Заметка появляется, а если оракул открывает поверхность и без пользователя — срабатывает `wish`.

### 4.6 Директива контроллеру (d11)
Домену иногда нужно изменить контроллер UI:
- вернуть сообщения в композер (`give_back`, threads.rs:2995);
- префилл после backtrack;
- засеять историю из реплея;
- очистить отправленное после подтверждения `last_submit`.

Это `Directive<T> { seq, value }` в `ThreadModel`. Слушатель по одному на род в `widgets/directives.rs`. Он сравнивает `seq` с `ThreadUi.seen.<род>` **своего** потока и применяет к контроллерам этого потока. Поток, который не показан, применяет директиву при показе: не теряется и не повторяется.

### 4.7 Интерес (d12)
`Interest { key, child }` берёт `Hs` в `did_change_dependencies`. В `init_state` он шлёт `Interest(key, Open)`, в `dispose_with` — `Close` (rtwk разрешает `emit` там). Core:
- счётчик `0 → 1`: вызывает `Feature::on_interest(open)` (загрузить, начать опрос);
- `1 → 0`: `on_interest(close)` (остановить опрос, перестать ждать);
- на `attach`: для каждого ключа со счётчиком `> 0` снова `on_interest(open)`.

### 4.8 Заметки и ошибки
Заметка в транскрипт — `NoteKey` (хаб) или `FeatureNote` с аргументами-данными, слова из `strings.rs` UI. Ошибка RPC — `RpcError` в `Remote::Failed`/`Op::Failed`, слова в UI. Ошибка поверхности до следующего действия — факт с `seq`, гасится в UI. Уведомления рабочего стола — `DesktopNotice` из `AppFx` (политика из `[tui]`), выполняет UI-обёртка `TerminalEffects`.

---

## 5. Жизненные циклы

### 5.1 Запуск (d8)
1. `host::main` → `arg0_dispatch_or_else` → многопоточный tokio-рантайм → разбор CLI, язык (`Cli::lang`) → факты запуска (конфиг, auth, trust, обновление).
2. `run_app_view` на потоке `block_on` с входом в рантайм (rtwk-tokio). `UiApp::new(backend)`, `UiState.awaits_facts = true`.
3. Shell стартует backend: in-process app-server, облачный или fake. `CodexApp::attach()` даёт эпоху. Подписка штампует ею события.
4. Корневой view: `MultiProvider` (фиксированный список, G15) → `Navigator` (`GlobalKey`) → home `StartupGate`. Пока фактов нет, home пуст (оракул не мигает чатом до промптов).
5. Факты (`AppEv::Facts`) → `onboarding::Model`: шаги sign-in, provider, trust, migration, import, стартовые промпты. Шаги — род **Ж** (они держат старт первого потока: `FirstThread::AfterLaunch`, `start_held`).
6. `StartupPage` выводит текущий шаг как f(домен). Выбор внутри шага — **Н**, решение шага — глагол.
7. Шаги кончились → `apply_app` стартует первый поток (`ThreadAsk::Start`). Композер работает как черновик и до `Ready`: отправка ждёт в `queued_submits` (R10).
8. Ответ → `ThreadSet::open(id, Origin)` → `session.active = id` → `follow_core` создаёт `ThreadUi` → `SessionPage` в `thread_scope(id)`.

`codex-rtwk resume` и `codex-rtwk agents` кладут `start_resume` / `start_agents` в `UiState`, и `StartPage` пушит соответствующую страницу над чатом.

### 5.2 Поток: открыть, переключить, закрыть (d9)
- **Открыть**: любая страница (resume, agents, переключатель субагентов) шлёт `Intent::OpenThread(id)`. `apply_app` решает так:
  - писатель уже держится → показать;
  - resume в полёте → ждать;
  - иначе выпустить один resume.

  Esc отменяет навигацию, а поздний успешный ответ всё равно устанавливает единственного писателя. События до ответа attach копятся в `attaching_events`.
- **Переключить**: `session.active` меняется только в `apply_app`. UI: `thread_scope(new_id)` перемонтирует поддерево потока, контроллеры берутся из `ThreadUi[new_id]` (черновик A→B→A цел, R2). Директивы B применяются по его `seen`. Шторки, привязанные к потоку (аргумент `ThreadId`), закрываются по предикату жизни. Докнутая шторка (О2) уходит вместе с поддеревом, как у оракула (`replace_chat_widget`). Шторки уровня приложения (`/apps`, `/plugins`, `/status`-диалоги) остаются.
- **Фон**: каждый поток получает свои события и без показа (`ThreadModel` всегда актуален). Переключение не требует реплея в UI.
- **Закрыть** (архив, удаление, отписка): `ThreadSet` убирает модель. `follow_core` удаляет `ThreadUi[id]` и через `dispose` освобождает контроллеры. Запросы сервера этого потока снимаются.

### 5.3 Переподключение (d9)
`Disconnected` → `ConnectionState`, баннер. Новый `attach_backend(b)` → новая эпоха, после чего:
- (а) запросы сервера этого backend удаляются, и шторки запросов закрываются по предикату;
- (б) **все `outgoing` этого backend разрешаются `Err(RpcError::Disconnected)`** *(новое, F6)*, `Op`/`Remote` → `Failed`;
- (в) потоки переподписываются, снимок `thread/read` заменяет затронутые ходы (R4);
- (г) интересы со счётчиком `> 0` получают `on_interest(open)` снова.

Шторки остаются открытыми и видят `Loading`/`Failed` в своих срезах.

### 5.4 Выход
`Hs.quit` (Ctrl+C или Ctrl+D по правилам оракула, `/quit`, выход из шага onboarding) → `AppCx::quit`. `fx.ends_app()`: соединение не поднялось, host сообщает причину. Shell роняет клиента, задачи отменяются. Ctrl+Z приостанавливает приложение, внешний редактор тоже (ADR-006).

### 5.5 Шторка (d10)
1. **Открытие.** Жест пользователя — страница зовёт `features::open(cx, target)` или `sheets::<s>::open(cx, args)`. По воле домена — открывает `SurfacePresenter`. Аргументы: `scope` (поток или cwd), начальная страница, id.
2. **Монтирование.** `init_state` создаёт контроллеры. `Interest` шлёт `Open`. Срез домена читается через `select`.
3. **Жизнь.** Жесты меняют `SheetStack` и контроллеры, глаголы уходят в core, страница = f(стек, `awaiting`, срез).
4. **Закрытие** — одной из четырёх причин:
   - (а) пользователь (Esc на первой странице, выбор, который завершает);
   - (б) предикат жизни стал ложным: запроса больше нет, `scope` устарел, поток сменился;
   - (в) домен снял желание;
   - (г) снятие поддерева.

   В (а)–(в) шторка сама вызывает `close_dialog` или снимает свой слот. Решение принимается в `did_change_dependencies` по срезу, после flush, а не в том же flush, что и emit.
5. **dispose.** `Interest` шлёт `Close`, контроллеры освобождаются, фокус возвращается открывшему (rtwk делает это для маршрутов; докнутая шторка возвращает фокус композеру явно).

---

## 6. Навигация и поверхности

### 6.1 Классы (поправка 2026-11-19b, без изменений по сути)
- **Страницы** (`PageRoute` корневого `Navigator`): Startup (шаги), Session (home), Agents, Resume, Usage.
- **Режимы** страницы — состояние её контроллера, а не новая страница: развёрнутый транскрипт (Ctrl+T, `/diff`, backtrack), активный поток (субагенты). Режим одного виджета ведёт его контроллер, второй копии виджета нет.
- **Шторки** — виды нижней панели оракула: approvals, пикеры, настройки, pets, голос, `/plugins`, `/apps`, подтверждения.
- **Диалоги** — центрированные виды оракула и настоящие диалоги (`show_dialog`).
- **Поповеры** — `/`, `@`, `$`, поиск по истории (`Autocomplete` над композером).
- **Виджеты** — ячейки транскрипта (`/status` — ячейка), строка статуса, футер, питомец.

### 6.2 Стек страниц шторки
`SheetStack<P>` *(новое, `widgets/sheet_stack.rs`)* — это `State`-владеемый `Vec<Entry<P>>`, где `Entry { page: P, list: ListController, tabs: Option<TabController> }`. Рисуется верхняя запись, нижние держат свои контроллеры: Esc возвращает родителя с курсором на исходной строке, и поле `kept` в core не нужно. Операции стека: `push`, `pop`, `replace_top`, `reset(initial)`. Анимации и затемнения нет (паритет). Высота равна высоте верхней страницы. Вложенный `Navigator` для страниц шторки **не используется**: под ограниченными constraints его `RenderTheater` занимает всю высоту (rtwk navigator.rs:2650-2661). Другой путь — решение О1.

### 6.3 Где живёт шторка (О2)
Рекомендация: виды нижней панели оракула стоят в `SheetHost` *(новое)* — слоте в `Column` страницы сессии под транскриптом, внутри `thread_scope`. Это `Scaffold.bottomSheet`: хвост транскрипта поднимается над шторкой, как у оракула, а смена потока сбрасывает слот. `SheetHost` держит стек открытых шторок уровня потока (верхняя видна; запрос сервера всегда сверху). Центрированные виды и диалоги идут через `show_dialog` корневого навигатора. Шторки уровня приложения в debug-входах открываются на ближайшем навигаторе (seeded-scope, F9). До решения О2 шторки остаются, как сейчас, модальными маршрутами с `sheet_options()` (src/sheets/mod.rs:19-27), а правила §6.4 действуют одинаково.

### 6.4 Открытие и закрытие
- Открывает только тот, кто просит: страница, фича (`open`) или presenter. Аргументы — id, `scope`, начальная страница (R14). Шторка не копирует доменные данные в `State` (`r14_widget_state_fields_hold_no_model_types`).
- Каждая шторка объявляет **предикат жизни** `fn alive(args, &срез) -> bool`: запрос ещё в очереди, `scope` совпадает с доменом, поток активен. Ложный предикат → шторка закрывает себя. Правило: у каждой шторки есть `alive` (скан `impl Sheet`).
- Шторка не решает по проекции в том же flush, в котором отправила глагол (G14): своё ожидание она хранит в `awaiting`.

### 6.5 Аргументы и результаты
`Navigator::push::<T>` / `show_dialog::<T>` возвращают результат. Если результат меняет домен, он уходит глаголом с доменным ключом, а не через сохранённый тикет. Страницы получают `ThreadId`/ключи. Редактор (настройки) копирует значение как черновик и возвращает его глаголом.

---

## 7. Фокус и клавиши

- **Владение.** Разбор и разрешение `tui.keymap.*` (9 контекстов, аккорды, лидер) — в core (`core/src/keymap`), результат `Rc<Keymap>` в `CodexApp` (факт конфига). Каждая поверхность объявляет свои `Intent` rtwk (`MoveDown`, `Accept`, `Back`, `NextTab`…) и строит `Shortcuts` из контекста `Keymap` (`src/keymap`). `Actions` живут в `State` поверхности и вызывают контроллеры (**Н**) или шлют глаголы (**Д**).
- **Порядок.** Фокус-дерево rtwk: маршрут → шторка/диалог → поле. Ближайший к фокусу `Shortcuts` берёт клавишу первым. `back_key` навигатора лежит слоем выше маршрутов (rtwk navigator.rs:1693-1699). Если маршрут клавишу не взял, Esc снимает маршрут. Шторка с операцией в полёте держит Esc через `PopScope`, если так делает оракул.
- **Фокус при открытии и закрытии.** Маршрут или диалог берёт фокус и возвращает его открывшему (rtwk). Докнутая шторка (О2) запрашивает фокус в `init_state` и отдаёт его композеру в `deactivate`.
- **Ввод никогда не замораживается** RPC (ADR-006). Карантин набора перед approvals — gap G8. Задержка показа approval после набора (`TYPING_IDLE_DELAY`) — в presenter-е (UI).
- **Правило**: `KeyCode::`/`KeyPress::code` только в `src/keymap/` и примитивах `widgets/` (ratchet от 280 вхождений в 31 файле). Подсказки футера строятся из тех же привязок, что и `Shortcuts`.

## 8. Мышь (ADR-011 без изменений)
Мышь — род **Н**: клик выделяет, клик по выделенному или двойной клик принимает, колесо прокручивает, вкладки и кнопки кликаются. Её обрабатывает виджет: `Picker`, `session_list`, `tab_strip`, `SheetStack`. Каждый вид с выделением тестируется событиями мыши `rtwk_test`. Выделение текста терминала и копирование работают (G9, G11).

## 9. Ошибки и заметки
- Ошибка — значение: `RpcError` (тикет, несовпадение, отказ, `Disconnected`), `Failure` потока, `Problem` экрана. Паник в путях `apply` нет.
- Видимые слова ошибок и заметок — только в `strings.rs` UI. Core хранит ключ и данные (`core_sources_hold_no_user_visible_prose`).
- Ошибки задачи вида (rtwk `report_task_error`) уходят в debug-журнал. Пользователю их не показывают, если оракул молчит.

## 10. Локализация (ADR-010 без изменений по сути)
Строки — типизированные `const`-таблицы, `strings.rs` на каталог UI (pages, sheets, features, widgets) и `src/l10n/` для общих. Английский — текст оракула байт в байт. Язык согласуется один раз при старте и попадает в дерево как `TwkLocalizations`. Шторки и страницы, переехавшие из core, забирают с собой свои слова. Ключи заметок остаются в core как типы.

## 11. Производительность
- **Горячие пути.** Дельта стрима → `threads::apply` → `select::<ThreadModel>` активного потока → живой хвост транскрипта (один элемент, не чаще раза в кадр, ADR-004). Клавиша в шторке → локальный `setState`, перестраивается одно поддерево, core не участвует (C убирает курсоры из `apply`).
- **Срезы.** `ScreenStates` и `FeatureStates` разбиты на `Rc` по экрану и по фиче *(новое)*, провайдер на срез, `select` по срезу. Клонируется только изменённый срез, и проснутся только его читатели. Запрещено `watch` целого `ScreenStates`/`FeatureStates` (ratchet от 27).
- **memo.** Один уровень на регион. Ключи — дешёвые значения по строке или `(rev, Rc<T>)` (`Versioned<T>`). `unchanged()` в коде фич запрещён.
- **Виджеты.** Экраны и шторки — `stateful eq` (ADR-002, условие 1). Списки — `ListView` с builder-ом (виртуализация). Монтируются только видимые экраны.
- **Измерение.** ADR-008: pty A/B против оракула, 11 сценариев, отчёт с интервалами. Бюджет на сценарий масштаба (1 000 потоков, 30 панелей) — тест на релиз. Запас по семейству 4 (entities + notify) включается только по профилю выше ~1 мс на кадр.

## 12. Тестирование (d14)

| уровень | что проверяет | где | чем | когда |
|---|---|---|---|---|
| 1. правила | слои, R-правила, ADR-010/012/013, бюджеты хаба | `tests/rules` | сканы исходников с синтетическим укусом | каждый коммит |
| 2. домен | `apply`: события → модель + эффекты; гонки (поздний ответ, смена потока посреди стрима, approvals во время диалога, переподключение посреди операции), перестановки R9/R10, `assert_consistent` R7 | `core/tests/*.rs`, `core/tests/adr003.rs`, фичи через `CodexApp` | события и скрипты JSONL через `ingest`, без UI | коммит фичи |
| 3. kit | чистая логика: vim, paste_burst, история, фильтр команд, backtrack | `core/src/kit/**` (тесты в файлах) | обычные unit | коммит |
| 4. виджеты | шторки, страницы, кирпичи: жесты, мышь, `SheetStack`, предикаты жизни, директивы, `Interest` (Open/Close/dispose), presenter-ы | `tests/sheets`, `tests/pages`, `tests/app` | `rtwk_test::Tester` над `UiApp` с fake backend или `Sandbox` | коммит |
| 5. паритет (чёрный ящик) | строки и стили против снимков оракула при его размере | `tests/oracle/<shard>.rs` через `tests/kit` (`Screen::build(owner).entry(dir, entry)` / `.script` / `.live`) | debug-входы (`DebugEntry` = скрипт + `ui`-шаг) | ключевые тесты в задаче, полный прогон — land-check |
| 6. сценарии | оба бинаря в pty против одного мок-модели | `tests/scenarios` | rtwk-term `TerminalSession`, wiremock SSE | функция готова |
| 7. производительность | A/B против оракула | `perf/` | `drive.py`, `report.py` | вехи |

Правила тестов: core-тест не знает виджетов; тест виджета не строит состояние core мимо `ingest` (скрипт или `Sandbox`); тест kit не трогает `Tester`; oracle-тест не видит ни core, ни виджетов (`tests/kit/mod.rs:3-8`). Начальная страница шторки в debug-входе задаётся шагом `ui` (`{"step":"ui","open":"<entry>","props":{…}}`). Core отдаёт его как есть, как `key` и `resize` (core/src/fixtures/script.rs:25-26). Имена входов стабильны, поэтому oracle-тесты при переезде не меняются.

---

## 13. Правила, которые закрепляют решения

Существующие (остаются): `core_manifest_has_no_ui_or_runtime_dependency`, `core_sources_name_no_ui_runtime_or_io`, `core_sources_hold_no_user_visible_prose`, `views_never_name_protocol_shell_tokio_or_io`, `views_never_apply_the_model`, `screen_dirs_hold_view_files_only`, `ui_crate_has_only_the_known_folders`, `shell_names_no_widget`, `the_ui_model_is_the_core_model_plus_ui_state`, `thread_model_is_made_only_in_threads_rs`, `codex_app_holds_its_threads_privately`, `thread_rpcs_only_in_threads`, `the_shown_thread_is_set_only_in_apply_app`, `widgets_never_import_pages`, `features_are_named_only_through_the_list`, `features_hold_no_app`, `feature_slices_ask_only_through_the_outbox`, `hub_enums_only_shrink`, `the_feature_trait_has_the_hooks_the_budget_says`, `threads_apply_matches_no_screen_or_feature_intent`, R1 (`r1_*`), R4, R11/R12, R13, R14 (`r14_*`), R15, ADR-010, `adr002_the_navigator_is_keyed`.

Новые (каждое с синтетическим укусом; ratchet, где перенос идёт по шагам):

| правило | что ловит | решение |
|---|---|---|
| `adr013_domain_types_hold_no_draw_state` | поле типа, достижимого из `CodexApp`, `ThreadModel`, `ScreenStates`, `FeatureStates`, с именем или типом отрисовки (`cursor`, `selected`, `scroll`, `focus`, `highlight`, `hover`, `tab`, `page`, `stack`, `kept`, `expanded`, `density`, `menu`, `shown`, `open`, контроллеры, `ScrollState`); ratchet | заменяет `core_screen_models_hold_no_ui_state_fields` |
| `adr013_kit_is_not_app_state` | тип `core::kit` как поле типа домена; kit называет корни домена | граница kit |
| `adr013_feature_trait_has_no_view_types` | `Menu`, `Pick`, `Prompt`, `choose`, `picked`, `prompt_submitted` в `Feature`, `AppCx::open/prompt` | ADR-012 в C |
| `adr013_core_intents_name_no_gesture` | варианты интентов core `Choose`, `Secondary`, `Toggle`, `Reorder`, `Close`, `Back`, `Tab`, `Hot`, `Highlight`, `Cursor`, `Expand`, `ToggleDensity`, `Activate`, `Preview`, `Move`, `Room`, `End`; ratchet | глаголы, не жесты |
| `adr013_widgets_name_no_screen_half` | `core::screens::*`, `core::features::*` в `src/widgets` | picker независим |
| `adr013_no_ticket_in_ui` | `Ticket` в `src/pages`, `src/sheets`, `src/features`, `src/widgets` (кроме баннеров, где тикет — id сущности, по списку) | §1.4.5 |
| `adr013_interest_made_in_one_place` | `Intent::Interest` вне `src/widgets/interest.rs` | §4.7 |
| `adr013_directives_read_in_one_place` | чтение `Directive`/`restore()` вне `src/widgets/directives.rs` | §4.6 |
| `adr013_wishes_shown_by_the_presenter` | `WishShown` вне `src/app/presenters` | §4.4 |
| `adr013_sheets_declare_alive` | шторка (`impl Sheet`) без `alive` | §6.4 |
| `adr013_no_whole_state_watch` | `watch::<ScreenStates>` / `watch::<FeatureStates>`; ratchet от 27 | §11 |
| `r2_thread_subtree_is_keyed` | ключ по `ThreadId` вне `thread_scope` | R2 |
| `adr002_screen_and_sheet_widgets_are_eq` | `impl_widget!(stateful …)` без `eq` в pages, sheets, features | ADR-002 |
| `adr006_keycode_only_in_keymap` | `KeyCode::` вне `src/keymap/` и примитивов `widgets/`; ratchet | §7 |
| поведенческий `adr003_reconnect_fails_inflight_ops` (core/tests/adr003.rs) | операция в полёте после `attach` → `Failed`, интерес перезапрошен | §5.3 |

Семантическую часть IO-теста (вопрос 2 против 5: «это ключ запроса или курсор?») скан решить не может. Она идёт в чек-лист ревью (process.md §A5) и в долговую задачу «IO-тест: семантический аудит после переезда select» (process.md A4.1).

---

## 14. Что меняется в действующих ADR

| ADR | было | стало |
|---|---|---|
| 001 | core держит «each screen's model» | core держит домен экранов и фич (роды Д/З/Ж/И); kit — модуль core |
| 002 | поправка 2026-11-18: «core view stack», слот рисует верх стека core | **снята**. Стек шторки — `SheetStack` в `State`; шторки в `SheetHost` (О2) или модальные маршруты; навигация — состояние rtwk, как в исходном тексте ADR-002 |
| 003 | область: «application state» без определения границы | область задаёт IO-тест (§1.3); пять родов состояния; R14 дополнен: аргументы-id и для шторок фич; новые механизмы `SurfaceWish`, `Directive`, `Interest`, `OpKey` |
| 006 | keymap читается в `Shortcuts` | плюс правило `KeyCode` и `Intent`/`Actions` в каждой поверхности |
| 010 | — | слова меню и промптов фич переезжают в `src/features/<f>/strings.rs` |
| 012 | `Feature` с `Menu/Pick/Prompt`, `FeatureView::menu_list`, «живое меню» через picker по глубине | `Feature` без UI-форм, с `Wish`, `Interest`, `on_interest`; видимая половина фичи — `open` и своя шторка |

Отступление на время переезда: семейства `select::Menu` (models, permissions, review, worktree, apps, quick dials, collaboration, export), `settings`, `dialogs`, `resume` (плотность, развёрнутость), `pets` (подсветка) держат **Н** в core до своего переезда. Долг — задачи миграции §15, ratchet правил §13.

---

## 15. Миграция (фичи не встают)

0. **ADR-013 + правила в режиме ratchet** (`hub_budget`, счётчики нарушений). С этого дня новый код не добавляет `Menu/Pick/Prompt` и интентов-жестов. Сюда же исправление F6 (`outgoing` на `attach`): оно не зависит от C.
1. **Пилот `/plugins`.** Домен из WIP `6cafa8de1` без дверей. Шторка `PluginsSheet` на `SheetStack` + `TabController`, `Interest(Plugins{cwd})`, `ops` по `PluginKey`, `wish(PluginAuth)`. Прототип О1/О2 на нём же. Тесты: core (глаголы, двойное нажатие, поздний ответ, смена cwd, переподключение), `Tester` (страницы, мышь, Esc), oracle-шард `/plugins` без изменений.
2. **Общий select.** `list.rs`, `layout.rs` → `widgets/picker`; `stack`, `kept` → `SheetStack`; `Picked` → глаголы; шаги фикстур `screen` с формами списков → шаг `ui`. Семейства по одному, каждое целиком (F15).
3. **По касанию**: settings, dialogs, resume (`Cursor/Room/End` → `NeedMore`, `density`/`expanded` → `State`), pets, agents; `/goal`, `/export`, `/rename` переносят меню и промпты в UI-половину.
4. **Срезы** `ScreenStates`/`FeatureStates` и `thread_scope` — до шага 2: от них зависит стоимость шторок.

`95826e191` сливается как есть.

---

## 16. Открытые решения (решает человек; рекомендация первой)

| # | вопрос | варианты | рекомендация |
|---|---|---|---|
| О0 | принять C | C / A (двери) / B (всё в UI) | **C** в редакции этого документа (с §4.4–§4.7) |
| О1 | страницы шторки | `SheetStack` в `State` / вложенный `Navigator` (+ gap rtwk «навигатор по размеру верхнего маршрута») | **`SheetStack`**: без анимации, без затемнения, по высоте содержимого, засевается аргументом |
| О2 | где стоит шторка | докнутый `SheetHost` в странице сессии / модальный маршрут `show_dialog` у нижнего края (как сейчас) | **`SheetHost`** для видов нижней панели оракула (паритет: хвост транскрипта над панелью, сброс при смене потока); сначала сверить снимок оракула |
| О3 | где чистая UI-логика | `core::kit` / отдельный крейт | **`core::kit`** (один крейт для upstream) |
| О4 | ключ операций | доменный ключ + `gen` / корреляционный id от UI | **доменный ключ**: двойное нажатие схлопывается естественно |
| О5 | кто пишет правила шага 0 и долговую задачу | — | одна задача `adr013-rules` (codex-porter) до пилота |
| О6 | WIP `6cafa8de1` | сливать двери / пилот с нуля | **не сливать двери**, домен из WIP взять в пилот |
| О7 | `Rc` или `Arc` в core для upstream | — | без изменений: решает `session-core` (уже открыто) |

---

## 17. Источники

Код: `core/src/app.rs`, `core/src/threads.rs`, `core/src/features/mod.rs`, `core/src/screens/select.rs`, `core/src/screens/select/plugins.rs`, `src/app/view.rs`, `src/app/mod.rs`, `src/sheets/presenter.rs`, `src/sheets/select/dialog.rs`, `src/pages/session/session_page.rs`, `src/widgets/thread_ui.rs`, `tests/rules/layers.rs`, `tests/kit/mod.rs`. rtwk: `crates/rtwk/src/navigation/navigator.rs`, `crates/rtwk-framework/src/widget.rs`, `crates/rtwk-overlay/src/dialog.rs`, `crates/rtwk-controls/src/tabs.rs`, `docs/guide/state.md`. Оракул: `chatwidget/plugins.rs`, `bottom_pane/mod.rs`, `app/session_lifecycle.rs`. Исследования: `docs/rtwk-port/research/arch-boundary/report.md`, `state-approaches.md`, `state-bug-classes.md`, `nav-proto.md`, `pages-vs-widgets.md`, `core-slices/report.md`.
