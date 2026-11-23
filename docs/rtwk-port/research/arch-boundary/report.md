# arch-boundary: где проходит граница core/UI, и черновик ADR-013

tm `arch-boundary` (2026-11-23), codex-reviewer, только чтение кода. Замеры на `claude/tui-rtwk` @ `b2eeacb7c`. Пути от `codex-rs/tui-rtwk/`. Диаграммы d1–d7: [diagrams.md](diagrams.md). (Текст записан оркестратором из хэнд-бэка исследователя: хук не дал субагенту записать .md.)

## 0. Синтез

Гипотеза оркестратора **подтверждается, с поправкой**. Граница проходит между «моделью экрана» и «отрисовкой» (MVU/headless UI), а не между «доменом» и «UI», как во Flutter. Её так поставили три решения подряд: state-v2.md:21-25 (TEA с `composer` и `pickers` в core); ADR-001 architecture.md:117-118 (в core «each screen's model»); поправка 2026-11-18 architecture.md:357-358 («core view stack… changed by `apply`»). Поэтому всякая новая форма UI (вкладка, горячая клавиша, переключатель, страница шторки) становится протоколом core↔view — «дверью».

Поправка: большая часть `core/src/screens` — честный домен (данные сервера, запросы с тикетами, потоки входа). В UI уходит **15–18 %** её строк. У состояния экрана три рода: домен; «что спросили» (параметры запроса как ключ ответа); «что нарисовано» (курсор, вкладка, страница шторки, стек меню, свёрнутость). Первые два остаются в core, третий уходит в State виджета.

**Рекомендация: вариант C — граница по вводу-выводу (IO-тест).** ADR-003, ADR-012 и headless-тесты сохраняются. Из контракта фичи уходят формы UI (`Menu`, `Pick`, `Prompt`, `Choice`, `MenuEvent`, `close_menu`); фича говорит с UI доменными глаголами (`Intent::Feature(f::Intent)`) и проекцией `FeatureStates`, свою шторку строит в `src/features/<f>/` на контроллерах rtwk. Цена: ≈6,5–7,5 тыс. строк из 42,2 тыс. прод-строк `core/src/screens`, ≈140 тестов core, ≈143 шага фикстур.

## 1. Текущая карта (замер)

| крейт / модуль | строк | ответственность |
|---|---|---|
| core src | 69 207 (тесты в файлах 5 863) | модели, `apply`, протокол, парсинг команд; без rtwk |
| — screens/ (19 каталогов) | 47 127 | модели, события, интенты, RPC экранов |
| — threads* | 7 346 | `ThreadModel`, `apply`, `project`, очередь |
| — app* | 5 189 | `CodexApp`, `apply_app`, маршрутизация ответов |
| — features/ | 2 279 | ADR-012: `feature_list!`, `Feature`, `AppCx` |
| — commands, keymap, protocol | 6 358 | slash-роутер, keymap, протокол |
| core/tests | 37 154 (1 529 тестов) | headless через `CodexApp` |
| UI src | 77 576 | pages 45 176 (session 24 932, startup 8 551, usage 5 225, agents 3 964, resume 2 495), sheets 13 572, widgets 7 327, services 7 573, app 2 663, features 305 |
| host | 6 248 | бинарь, in-process app-server, факты конфига |
| tests | 82 740 | oracle 2 622, pages+sheets 869, kit, правила |

Прод-строки screens: select 8 842, composer 6 264, usage 5 083, onboarding 3 788, agents 3 510, approvals 2 312, chat 2 281, resume 1 806, settings 1 608, status 1 075, startup 1 071, import 878, backtrack 712, dialogs 703, transcript 615, popovers 517, cells 402, pets 279, voice 99 = 42 162 (66 % прод-кода core 63 524).

Владельцы: `CodexApp` (app.rs:540-570: session, threads, requests, screens :546, features :549, outgoing, keymap, tui, effects); `ThreadModel {core, slots, features, seal}` (threads.rs:1027-1036, только в `ThreadSet::open`); `UiApp {core, ui, link}` (src/app/mod.rs:58-62), `UiState` (:31-55); `ThreadUi` (widgets/thread_ui.rs:22-32). Точку возврата курсора хранит core: `select::Model.kept` (select.rs:315, `cursor_row()` :326).

Отрисовка в core: select.rs:290 `stack`, :315 `kept`; plugins.rs:206-224 `menu`/`tab`/`shown` рядом с `cache`/`waiting`; plugins.rs:511-525 `choose(id)` разбирает id строки; resume.rs:320-335 `ToggleDensity/Expand/Cursor/Room/End`, :73 `density/expanded`; settings.rs:679 `open/edits`, `Activate/Preview/Move/Close`; dialogs.rs:276 `open`, `Page/ToggleKeyboard/Close`; pets `Highlight`; usage/mod.rs:71-81 `section/ranges` (это «что спросили»); select/list.rs, layout.rs — форма строк и высоты пикера. `src/widgets/picker/*` импортируют `core::screens::select::{list,layout}` (pane.rs:18-19, rows.rs:26); 15 файлов UI.

Правило `core_screen_models_hold_no_ui_state_fields` (tests/rules/layers.rs:315) проверяет 8 имён только в `pub`-полях структур `Model`: мимо проходят `kept`, `stack`, `tab`, `density`, `expanded`, `section`. Правило дырявое.

Потоки (d5, d6): уведомление — host in_process.rs:446 → Ingress → shell.rs:103-119 → `UiApp::ingest` → `CodexApp::ingest` (app.rs:939, эпоха R1) → `apply_app_from` :957 / `apply_event` :1063 → `threads::project` (threads.rs:1906) + `screens::project_thread` → `threads::apply` (:2251) → эффекты → view (`Provider::value_shared`, view.rs:124-139). Клавиша — picker `Event::Choose(id)` → `MenuDialog` `Intent::Choose` (dialog.rs:122) → ingress → `select::apply_intent` (select.rs:1114) → `Plugins::choose` (plugins.rs:511) → `plugins_asked` (app/select.rs:81) → shell `plugin/install` → `apply_reply` (app.rs:1653) → `answered` → `Ev::Plugins` → `watch::<ScreenStates>` (dialog.rs:95). Каждый шаг «клавиша → страница» идёт через core.

ADR: 001 соблюдено; **002 отступление** (architecture.md:230 «navigation state is rtwk's» против поправки :357 и `select::Model.stack`); 003 соблюдено для потоков, R14 нарушен по смыслу (`kept`, `Intent::Cursor`); 004, 006, 007, 011 соблюдены; 010 дорого (`Note` ломает match в pages/session/strings.rs); **012 с изъяном** (`type Menu/Pick/Prompt` features/mod.rs:145-149, `choose/picked/prompt_submitted` :180-191, `FeatureView::menu_list`).

## 2. Почему так

Дало: 1 529 headless-тестов; 949 фикстур `.jsonl` через живой `ingest` (448 с шагом `screen`); один писатель и тикеты (R1, R3); core годен для upstream; компилятор запрещает rtwk в core.

Стоило: 355 вариантов/методов в hub_budget (`select::Note` 38, `Intent` 24, `Picked` 14, `Menu` 10, `PromptKind` 3, `Feature` 16); из 446 UI-коммитов 213 (48 %) правили core, 114 (26 %) — хаб-файл core; WIP /plugins (`6cafa8de1`): `Registered`, `menu_open`, `Out::CloseMenu/OpenUrl/RefreshApps`, `Choice::Act`, `MenuEvent`, `menu_event`, `on_scope_changed` — домену нужны только `OpenUrl` и `on_scope_changed`; форма UI (`List/Item/Span/Role`, `Heights`) в core; ≈143 шага фикстур засевают отрисовку.

Корень: семейство 1 + граница эффектов (ADR-003) верны для состояния приложения, но state-v2 и ADR-001 записали в него модели экранов. Flutter: ephemeral state в `State`/контроллерах, app state в bloc/provider (rtwk docs/guide/state.md:5-20, :209-226).

## 3. Альтернативы

**A. Как есть + двери.** Решает: ничего не ломается. Приносит: каждая форма UI — вариант хаба или метод `Feature`; ~28 core-задач с меню впереди; 48 % UI-коммитов трогают core.

**B. Core только домен, всё состояние экранов в UI.** Решает: дверей нет, core ~20 тыс. Приносит: второй владелец app-state с тикетами в UI (R1/R3/R10 заново, почва ошибки VS1, architecture.md:546); headless-тесты уходят в крейт с rtwk; 448 фикстур; ≈25–30 тыс. строк, фичи встают.

**C. Граница по вводу-выводу (рекомендуется).** Значение в core, если: (1) ответ/факт сервера или host или выведено из них; (2) запись о запросе — тикет и параметры как ключ ответа; (3) его читает другой экран, поток или шаг жизненного цикла; (4) оно пишется наружу. Остальное — виджету. Решает: двери исчезают, picker не зависит от core, ответы ложатся в домен (`Remote<T>`, R6), страница шторки = f(страница UI, домен); ADR-003/012 остаются. Приносит: перенос 15–18 % screens, ≈140 тестов core, ≈143 шага фикстур на `DebugEntry.open`; спорные случаи IO-теста решает ревью; query/range живут в `CodexApp` с явными `Open`/`Close`. Тесты: core — домен; шторки — `Tester`; kit/oracle без изменений; чистая логика (vim, paste_burst, история, упоминания) — `core::kit`.

| | A | B | C |
|---|---|---|---|
| двери на форму UI | растут | 0 | 0 |
| один писатель app-state | да | нет | да |
| headless-тесты домена | да | в UI | да |
| перенос | 0 | ≈25–30 тыс. | ≈6,5–7,5 тыс. |
| UI-коммиты, трогающие core | 48 % | ≈0 | ≤20 % (оценка) |
| как Flutter | нет | да | да |
| фичи во время переезда | — | встают | идут |

## 4. Черновик ADR-013 «Граница core/UI по вводу-выводу»

> Core хранит состояние приложения: данные и факты сервера и host, запись каждого запроса (тикет, параметры как ключ ответа), то, что читает больше одной поверхности или шаг жизненного цикла, и то, что пишется наружу. Всё, что меняет только нарисованное — курсор, прокрутка, вкладка, страница и стек шторки, свёрнутость, плотность, подсветка, открыта ли поверхность, черновик до отправки — живёт в `State` виджета или его контроллере rtwk (`ListController`, `SelectionController`, `TabController`, `TextEditingController`, вложенный `Navigator`), а то, что переживает смену потока, — в `ThreadUi`. Core не знает ни строк, ни страниц, ни жестов.

Слои: `core::domain` (app, threads, features/<f> — домен, screens/<dir> — домен, commands, protocol, keymap); `core::kit` (чистая логика без состояния приложения); UI (pages, sheets, features/<f> — шторка и слова, widgets с picker и его list/layout, services/shell); host (композиция и IO).

| состояние | владелец | читают | меняют |
|---|---|---|---|
| поток | `ThreadModel` | `select::<ThreadModel>` | `ThreadEv` → `threads::apply` |
| сессия, запросы, факты, каталоги, операции фич | `CodexApp`, `FeatureStates` | провайдеры | `Intent`/`AppEv` → `apply_app` |
| параметры запроса страницы | core (ключ ответа) | `select` | глагол `Search{query}`, `Range{..}` |
| страница, вкладка, курсор, стек шторки | `State` виджета, контроллеры | — | жесты в виджете |
| черновик композера, прокрутка потока | `ThreadUi` | — | контроллеры |
| навигация | `Navigator` rtwk (в т.ч. вложенный) | `Navigator::of` | push/pop |

Контракты: UI → core `Intent::Feature(<f>::Intent)` (глаголы `Load{cwd}`, `Install{target}`, `Uninstall{id}`, `SetEnabled{id,on}`, `AddSource{text}`, `RemoveMarketplace{name}`) и `ScreenIntent` того же рода, через `Handler` в ingress (R10). Core → UI: `FeatureStates`/`ScreenStates` как проекции, операции `Remote<T>`/`Op{ticket,status}`, заметки `NoteKey::Feature` со словами из `src/features/<f>/strings.rs`. Slash-роутер (dispatch.rs:17, :64) возвращает `Target` (в т.ч. `Target::Feature`) и глагол; шторку открывает UI (pages/session/open.rs:49), фича даёт `open(cx, args)`. `Feature` теряет `Menu/Pick/Prompt/choose/picked/prompt_submitted`; `AppCx` теряет `open/prompt`. Ответ приходит в домен; страница = f(своя страница, домен); шаг после ответа ключуется тикетом операции.

Правила (tests/rules/layers.rs, с синтетическим укусом): 1 `adr013_core_states_hold_no_draw_state` (все поля структур screens/features против `cursor|selected|scroll|focus|highlight|hover|tab|page|stack|kept|expanded|density|menu|shown`, ratchet) — заменяет layers.rs:315; 2 `adr013_feature_trait_has_no_view_types`; 3 `adr013_core_intents_name_no_gesture` (`Choose|Secondary|Toggle|Reorder|Close|Back|Tab|Hot|Highlight|Cursor|Expand|ToggleDensity|Activate|Preview|Move`); 4 `adr013_widgets_name_no_screen_half`; 5 `adr013_kit_is_not_app_state`; 6 hub_budget только уменьшается. Смысловая часть IO-теста — чек-лист ревью и долговая задача (process.md A4.1). Departure: поправка ADR-002 2026-11-18 («core view stack») отменяется для меню и настроек.

## 5. Миграция

| каталог | прод-строк | в UI | что |
|---|---|---|---|
| select (+ app/select.rs 716) | 8 842 | 4–5 тыс. | stack, kept, Menu/Picked/Page/Confirm, apply_choice, picks, страницы меню; list.rs+layout.rs → widgets/picker |
| settings | 1 608 | ~0,8 тыс. | open, edits, Activate/Preview/Move/Close |
| dialogs | 703 | ~0,5 тыс. | open, Page, ToggleKeyboard |
| resume | 1 806 | ~0,3 тыс. | density, expanded, action |
| agents, onboarding | 7 298 | ~0,6 тыс. | overlay/displayed, переходы шагов |
| usage | 5 083 | ~0,2 тыс. | show_zero_credit_groups |
| pets | 279 | ~0,1 тыс. | Highlight |
| composer | 6 264 | 0 | → `core::kit` |
| итого | 42 162 | ≈6,5–7,5 тыс. | |

Тесты: 23 файла core/tests, 244 обращения к меню; ≈140 тестов переписываются (models 49, settings 37, select_live 22, dialogs 12, select 9, worktree_popups 6, review_popup 5). Oracle 2 622 без изменений. ≈143 шага `screen` → `DebugEntry.open`.

Порядок: 0) ADR-013 + правила ratchet (½ смены); 1) пилот /plugins (1 смена: ядро WIP без дверей, шторка `PluginsSheet` на вложенном Navigator + TabController); 2) общий select (1 смена); 3) по касанию: settings, dialogs, resume, pets, agents; /goal, /export, /rename переносят меню и промпты в UI-половину. `95826e191` сливается как есть; `6cafa8de1` — нет.

## 6. Решает человек

1. C, A или B; IO-тест и отмена поправки 2026-11-18 для меню. 2. `core::kit` или UI-крейт для чистой UI-логики (рекомендация: kit). 3. Пилот /plugins вместо дверей WIP; сливать ли 95826e191 до пилота. 4. Кто пишет правила шага 0 и долговую задачу.
