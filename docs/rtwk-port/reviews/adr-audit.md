# ADR-аудит tui-rtwk — claude/tui-rtwk @ e4528721a (2026-11-19)

Источники: docs/rtwk-port/architecture.md (ADR-001…011, поправки 2026-11-18/19/19b, Layers),
process.md §5 и A4.4, AGENTS.md «Architecture is enforced, not described». Код только читался.
Rules-тесты на HEAD зелёные (`rules`, `layers`, `pin`, `carried`, `fixtures`: 47/47), так что каждое
нарушение ниже проходит мимо текущих тестов. Пути даны от `codex-rs/tui-rtwk/`.

Как закреплено: **T** — типом/компилятором; **Te** — rules-тестом или clippy-линтом (имя указано);
**W** — только словами. Соблюдено: **K** — да; **P** — частично или с дырой; **V** — нарушено.

## Сводка
65 правил. Закрепление: T 8, Te 32, W 25. Соблюдение: K 41, P 9, V 14, n/a 1 (устаревший текст).
Главное: правило ADR-003 о владении потоком (thread) записано в AGENTS.md, но не закреплено **ни типом,
ни тестом**. Все четыре его подпункта сегодня нарушены. Ветка open-rest (88ae44d9a) исправляет
*поведение* (один resume на поток), но не *структуру*: resume по-прежнему остаётся командой экрана agents,
а `ThreadModel::new` добавляется ещё в одном месте app.rs.

## Таблица

| ADR | правило | закрепление | соблюдено? (доказательство) | как закрепить |
|---|---|---|---|---|
| 001 | своё рабочее пространство, нет зависимости от codex-tui | W | K: Cargo.toml:64-81 | в pin.rs: assert, что ни один манифест не называет `codex-tui =`/`ratatui` |
| 001 | rtwk == RTWK_REF | Te `the_checkout_is_at_the_commit_rtwk_ref_names` | K | — |
| 001 | `[patch]` дословно как в codex-rs | Te `the_patch_tables_are_codex_rs_s_verbatim` | K | — |
| 001 | manifest policy / core-upstream-check | W (ручной tools/rtwk-port/core-upstream-check) | K (по последнему прогону) | запуск в чеклисте посадки (A6) как gate-шаг |
| 001 | codex-rs, его lock и codex-tui не тронуты | W (sanity при посадке) | K (только санкционированный oracle-0170) | gate-скрипт: `git diff --stat <ORACLE_REF> -- codex-rs ':!codex-rs/tui-rtwk'` пуст |
| 001 | свой clippy.toml с R-списками | Te `the_lint_lists_name_the_rules_banned_paths` | K | — |
| 002 | корень: MultiProvider проекций → Navigator(home) | Te (частично) | K: src/app/view.rs:76-82 | — |
| 002 | усл.1: route/dialog-виджеты `stateful eq` | W | K 31/32, исключение — debug `SeededScope` (src/app/debug/seeded.rs:70) | rules-тест: в src/pages, src/sheets каждый `impl_widget!(stateful|stateless` содержит ` eq ` |
| 002 | усл.2: presenter и show_dialog под Navigator | Te tests/sheets/presenter.rs (8 тестов) | K: presenter через handle home | rules-тест: `show_dialog` не встречается в src/app/view.rs и run.rs |
| 002 | усл.3: фиксированный список провайдеров + GlobalKey | Te `adr002_the_navigator_is_keyed` (только ключ) | K: view.rs:77-82 | расширить тест: между `MultiProvider::new()` и `.child(` ровно 5 `.with(` и нет `if`/`match` |
| 002 | усл.4: handlers `update_should_notify(false)`; нет решений по `read` того же flush; presenter не переоткрывает id | W / Te (no reopen) | K, риск: диспетчер slash-команд решает в view по `read::<ThreadModel>`/`SessionSlice` (src/pages/session/actions.rs:55-66) | `Intent::RunLine(line)`, а `DispatchCx` строит apply из модели; тест: `read::<ThreadModel`/`read::<SessionSlice` запрещены в src/pages, src/sheets |
| 002/R14 | в route/dialog передаются id, а не данные | Te (сканирует только поля `State`) | V: `ConfirmDialog{ask: Ask{name: String,…}}` (src/pages/agents/dialogs.rs:40, view/dialogs.rs:92) | передавать id задачи и читать имя через `select`; rules-тест: pub-поля виджетов в sheets/ и `*Dialog` — только id-типы, `Handler<_>`, Copy-enum |
| 002 | результаты идут в модель событиями через Handler | Te `views_never_apply_the_model` | K: все handlers делают `app.send` (view.rs:41-53) | rules-тест: `cx.on(`/`on_with(` только в src/app/view.rs |
| 002 | состояние навигации принадлежит rtwk, в модели не дублируется | W | K; заметка: навигацией управляют счётчики модели `session.switches`, `agents.resumes` (src/app/mod.rs:127-129, pages/agents/state.rs:230) | оставить; описать этот паттерн (BlocListener) в ADR-002 |
| 002 | запросы backend показывает presenter из очереди модели | Te presenter.rs, skeleton.rs:196,269 | K | — |
| 002/R2 | контроллеры потока в `ThreadUi` внутри `UiApp.ui` | Te `the_ui_model_is_the_core_model_plus_ui_state` | K: src/widgets/thread_ui.rs, app/mod.rs:31 | — |
| 002/R2 | per-thread поддеревья ключуются одним helper `thread_scope` | W | V: helper нет (grep 0); ChatState держит `Rc<RefCell<TextEditingController>>` и подменяет его в build (session_page.rs:76,459) | `thread_scope(id, child)` = `KeyedSubtree(ValueKey(ThreadId))`; rules-тест: `TextEditingController` в src/pages только через `ThreadUi` |
| Am.19 | режим одного виджета переключается контроллером, без второй копии | W | V (отступление): загрузка — отдельная страница `Opening` (src/pages/agents/opening.rs:1-20), а не режим SessionPage; pager пока заглушка | SessionPage(thread) с режимом loading (FutureBuilder-аналог); тест: transcript `ListView` строится только в pages/session/session_page.rs |
| Am.19b | страницы: Startup/Session/Agents/Resume/Usage | Te `ui_crate_has_only_the_known_folders` | K: src/pages/{startup,session,agents,resume,usage} | — |
| Am.19b | классификация в core совпадает с 19b | W | V: `KIND` устарел — status=Sheet (это cell), import/onboarding=Route (это шаги StartupPage), startup=Dialog (core/src/screens/*.rs:18) | убрать `Kind` из core (это UI-понятие в core) или тест «Kind ↔ папка» |
| Am.19b | sheets (approvals, pickers, settings, pets) и widgets (/status как cell) | Te (только по папкам) | K: src/sheets/*, pages/session/cells/status | — |
| 003 | один владелец и один писатель; модель меняется только через apply | T частично (`AppFx.commands` pub(crate), `Ticket::new` pub(crate)) | K сегодня, но не закреплено: все поля `CodexApp` (core/src/app.rs:243-256) и `ThreadModel` (threads.rs:201-218) `pub`, любой `&mut UiApp` может их писать (пример app/mod.rs:87) | поля pub(crate) + геттеры `&Rc<…>`; конфиг через `CodexApp::new(Config)` |
| 003 | `ThreadModel` создаётся только в threads.rs | W | **V**: core/src/app.rs:278, 690, 708, 748 (open-rest добавляет ещё одно) | `ThreadModel::new` → private; `threads::ThreadSet` владеет map и единственным `open(id, Origin) -> &mut ThreadModel`; `CodexApp.threads: ThreadSet` |
| 003/R3 | thread-RPC (resume, turns, read, interrupt, start, unsubscribe) только через `ThreadFx` | W (в ADR указано «types», но типа нет) | **V**: `agents::Command::{Resume,Fork,Start,Read,TurnsList,Interrupt,…}` (core/src/protocol/agents.rs:55-168, screens/agents/update.rs:51-58); shell сам шлёт thread/start и thread/unsubscribe (src/services/agents/workspace.rs:130-150) | удалить эти варианты из `agents::Command`; `ThreadFx::{resume,read,turns_page,interrupt}`; rules-тест `thread_rpcs_only_in_threads`: `ClientRequest::(Thread(Resume|Fork|Read|TurnsList|Unsubscribe|Start)|Turn(Start|Interrupt))` только в core/src/threads*.rs |
| 003 | экран просит `OpenThread(id)`, сам не делает resume и не держит копию | W | **V**: `OpenThread` нет (grep 0); resume picker идёт через agents (core/src/app.rs:603-605); `resume_into_chat` и `attach_known` (open-rest) переключают `session.active` в трёх местах | `AppIntent::OpenThread{id, from}` → одна функция в threads.rs; экраны эмитят только его |
| 003/R2 | per-thread данные только в `ThreadModel`; тест switch round-trip | W | **V**: agents `Model` держит `current` (копия `session.active`), `turning`, `requests`, `activity`, `last_messages` по String-id (core/src/screens/agents/mod.rs:315-371); теста round-trip нет | выводить из `CodexApp` (active, `ThreadModel.turn`, очередь запросов); proptest A→B→A в core/tests |
| 003/R3 | ключи `outgoing (ThreadId,Ticket)`, `server_requests (ConnEpoch,ThreadId,Id)` | T `RequestKey` (core/src/ids.rs:90), `Pending` в ThreadModel | K | — |
| 003/R3 | intent несёт свой ThreadId | T (`Submit{thread: Option<ThreadId>}`) | K, риск: id берётся из `session.active` в момент handler'а, а не у composer'а (view.rs:42-43) | `Handler<(ThreadId,String)>`; id передаёт composer из своего `thread_scope` |
| 003/R1 | `Reply` читается только через `pending.resolve` | T (core/src/rpc.rs:68 private, :144) | K | — |
| 003/R1 | spawn только в shell | Te clippy deny + `r1_only_the_shell_allows_disallowed_methods`, `r1_spawn_paths_are_the_shells`, `r1_the_host_neither_spawns_nor_allows` | K: единственный allow в src/services/mod.rs:10 | — |
| 003/R1 | штамп epoch, ingress отбрасывает старые события | Te `an_event_of_an_older_connection_is_dropped` | K | — |
| 003/R4 | transcript: upsert без push; snapshot заменяет целиком | Te `r4_the_transcript_has_no_push`, `replace_turn_takes_the_snapshot_wholesale` | K (теста resume/reconnect mid-turn нет) | добавить его в core/tests/model.rs |
| 003/R5 | `TurnState{Idle,Running(..)}` | T threads.rs:115 | K (end-path matrix нет) | тест-матрица концов хода |
| 003/R6 | `Remote<T>{Loading,Loaded,Failed}` | W | V: типа нет, у каждого экрана свой `Load` (pages/resume/mod.rs:266) | `core::remote::Remote<T>`, перевести экраны |
| 003/R7 | производное не хранится; `assert_consistent` после каждого события | W | **V**: `assert_consistent` нет (grep 0); хранятся `session.cycle` (`refresh_cycle`, core/src/app.rs:383,723), `agents.pinned` (agents/mod.rs:590), `agents.current` | методы вместо полей; общий harness `core/tests/invariants.rs` вызывает `assert_consistent` в `run()` |
| 003/R8 | жизненные циклы как enum; тест lifecycle × intent | W | P: enum есть, теста нет | табличный тест |
| 003/R9 | upsert из ответа или уведомления, что придёт первым; property-тест | Te (только пример `turn_reply_and_notification_in_either_order…`, model.rs:296) | P | proptest перестановок |
| 003/R10 | intents идут в тот же ingress | T (`send(&self)`, app/mod.rs:143) + Te | K | тест перестановок |
| 003/R11-12 | часы как вход модели; нет IO в apply и views | Te clippy + `r11_r12_no_clock_and_no_file_system_outside_the_shell`, core/clippy.toml | K | `disallowed_types = "deny"` в [workspace.lints] (сейчас warn, держится только на -D warnings) |
| 003/R13 | newtype на каждый вид id | T частично (`ThreadId(pub String)`, ids.rs:10) + Te `r13_…` (4 шаблона) | **V**: thread-id как `String`/`BTreeSet<String>` в agents (mod.rs:316-371, `Command::Resume{thread: String}`) | поле newtype private; тест: `BTreeSet<String>`, `thread: String`, `Option<String>` для id в core/src/screens |
| 003/R14 | данные приложения не попадают в локальный state | Te `r14_*` (6) + clippy `Obs`/`use_state` | P: бан clippy `TextEditingController::new` обходится через `::default()` (pages/agents/mod.rs:188, sheets/pets/mod.rs:132, widgets/options.rs:64); allowlist разросся до 7 файлов (tests/rules/rules.rs:433-443), а clippy reason говорит «только thread_ui.rs»; `SeededScopeState.sandbox` (app/debug/seeded.rs:41) не в списке типов | решить: контроллер в `State` — норма Flutter, поэтому снять бан конструкторов и сканировать типы данных в полях контроллеров; `Sandbox` внести в model_types с явным debug-исключением |
| 003 | `unchanged()` запрещён | Te `a6_unchanged_is_banned_in_feature_code` | K | — |
| 003 | memo на `Versioned<T>`, бюджетный тест | W | P: `Versioned` нет | ввести при первом memo по ревизии |
| 004 | контент через rtwk-rich, без строковых рендереров (R15) | Te `r15_*` (3) | P: R15_NOT_YET — session/actions.rs, sheets/dialogs/feedback.rs | закрыть оба файла |
| 004 | перенесённый код с комментарием о файле оракула | Te `carried_fragments_equal_the_oracle` | K | — |
| 005 | сначала full screen, inline позже | W | K: run.rs:50 | — |
| 006 | ключи через Focus/Shortcuts/KeyBindings из keymap; ввод не замерзает | W | P: ручной разбор `KeyCode::` в 6 файлах view (session_page.rs:155, search_key.rs, agents/dialogs.rs, …) | rules-тест: `KeyCode::` только в src/keymap и в allowlist с причиной |
| 007 | screen-тесты, pty-сценарии, snap, реестр | Te tests/oracle, scenarios | P: `docs/rtwk-port/refs/codex-registry.csv` отсутствует | создать реестр или убрать п.4 из ADR |
| 008 | perf-харнесс A/B | W | K (perf/) | — |
| 010 | строки только в таблицах | Te `adr010_*` (2), tests/app/l10n.rs | K | — |
| 011 | мышь в каждом view с rtwk_test-тестом | W | P: тесты мыши только у agents, resume, pets | тест-таблица view → mouse-тест |
| Layers | в core нет rtwk | T (core/Cargo.toml) + Te `core_manifest_has_no_ui_or_runtime_dependency` | K | — |
| Layers | в core нет runtime, IO, часов, прозы | Te `core_sources_name_no_ui_runtime_or_io`, `core_sources_hold_no_user_visible_prose`, core/clippy.toml | K | — |
| Layers | views не называют protocol, shell, tokio, IO | Te `views_never_name_protocol_shell_tokio_or_io`, `views_read_projections_not_core_held_protocol_values` (3 строки) | P: core отдаёт protocol-типы в pub-полях (`agents::Model.threads: Vec<Thread>`, mod.rs:315), так что view может читать их без имени протокола | pub(crate) на полях с protocol-типами, наружу — только проекции (`RowData`) |
| Layers | views не вызывают apply/ingest | Te `views_never_apply_the_model` | K | — |
| Layers | папки экранов содержат только view-файлы; известные папки | Te `screen_dirs_hold_view_files_only`, `ui_crate_has_only_the_known_folders` | K | — |
| Layers | shell не называет виджеты | Te `shell_names_no_widget` | K | — |
| Layers | `UiApp = core + ui` | Te `the_ui_model_is_the_core_model_plus_ui_state` | K | — |
| Layers/§5 | в моделях core нет UI-state (cursor, tab, scroll) | Te `core_screen_models_hold_no_ui_state_fields` (смотрит только `pub struct Model {`) | **V**: settings `ConfigPanel`/`KeymapPicker` хранят `tab`, `cursor`, которые двигают `Intent::Move/Tab` (core/src/screens/settings.rs:132-135, 286-290, 409-426); перенесённый `ScrollState{selected_idx, scroll_top}` (core/src/commands.rs:567-570, 711) | курсор и вкладка — в `SelectionList`/`TabController` в sheet; `ScrollState` удалить; тест расширить на все pub struct в core/src/screens и commands.rs |
| Layers | mock-state из JSONL через apply | Te tests/rules/fixtures.rs (8) | K | — |
| Layers | тест `every_screen_dir_has_a_core_half_and_a_view_half` | W (architecture.md:746 называет его, но теста нет) | n/a: устарело, папки больше не зеркальны (layers.rs:224) | поправить architecture.md:746; AGENTS.md ссылается на `tests/app/layers.rs`, а файл лежит в `tests/rules/layers.rs` |
| §5 | нет ratatui, нет Buffer-моста | W (зависимости нет, Cargo.toml:108 закомментирована) | K | см. первую строку (pin.rs) |
| §5 | build ничего не пишет; нет глобального состояния | W | **V**: build пишет `self.control.set`, `*self.composer.borrow_mut() =` (src/pages/session/session_page.rs:458-459); `thread_local! LAST` как test hook во view (pages/session/open.rs:19-23) | rules-тест: `thread_local!|static mut|OnceLock|OnceCell` запрещены в src вне services; в телах `fn build` нет `.set(`, `borrow_mut()`, `.replace(` |
| §5 | не копировать механику оракула (offsets, индексы, высоты) | W + R15 | **V**: settings cursor и `ScrollState` (выше); `Options.heights` — кэш высот, записанный layout'ом, чтобы ключ держал курсор в виду (src/widgets/options.rs:49,137) | `ListController::ensure_visible`/`SelectionList` rtwk; при нехватке — строка gap + pending(Gxx) |
| §5 | не обходить gap rtwk; pending(Gxx) | Te `the_pending_profile_lists_exactly_the_pending_markers` | K | — |

## Что делать дальше (сначала самое рискованное)

1. **ADR-003, владение потоком, типом** (риск: «thread already has an active writer», двойной resume).
   В `core/src/threads.rs` завести `ThreadSet` (private map, `get`/`iter`, единственный `open(id, Origin)`),
   `ThreadModel::new`/`blank` сделать private, `CodexApp.threads: ThreadSet`. app.rs:278/690/708/748 и
   `attach_known` из open-rest перейдут на `open`. После этого нарушение не скомпилируется.
2. **Thread-RPC только в `ThreadFx`.** Убрать `Resume/Fork/Start/Read/TurnsList/Interrupt` из
   `agents::Command`, построители `ClientRequest::Thread*` перенести в private `threads::wire`, а shell'овый
   thread/start+unsubscribe (services/agents/workspace.rs:130-150) — в `Command`-значения. Добавить
   `AppIntent::OpenThread{id, from}`. Rules-тест `thread_rpcs_only_in_threads` (grep выше).
   Сказать orchestrator'у: open-rest @88ae44d9a этого не делает и поэтому не закрывает ADR-003.
3. **Закрыть поля модели.** `CodexApp`, `ThreadModel` и `agents::Model`: поля pub(crate) плюс геттеры.
   UI-крейт сможет только читать, и protocol-типы перестанут утекать (Layers P).
4. **Убрать копию состояния потоков из agents.** `current`, `turning`, `requests`, `activity` выводить
   из `CodexApp`; String-id заменить на `ThreadId` (R13, R2, R7).
5. **UI-shadow в core.** `tab`/`cursor` из settings перенести в `SelectionList`/`TabController`, `ScrollState`
   удалить; `core_screen_models_hold_no_ui_state_fields` проверять на всех pub struct.
6. **Обязательные тесты ADR-003**, которых нет: `core/tests/invariants.rs` с `assert_consistent` после
   каждого события, switch round-trip (R2), end-path matrix (R5), lifecycle × intent (R8), перестановки
   (R9/R10), resume mid-turn (R4).
7. **Запись в build и глобальное состояние**: session_page.rs:458-459, open.rs:19-23, options.rs:137,
   плюс rules-тест.
8. **R14, решение для человека**: бан конструкторов контроллеров противоречит rtwk-first (контроллер в
   `State` — это Flutter) и обходится через `::default()`. Нужно либо снять бан и проверять типы, либо
   запретить и `default` с честным allowlist.
9. **ADR-002 мелочи**: `ConfirmDialog` должен получать id; тесты на `eq`, на фиксированный список
   провайдеров, на «handlers только в view.rs», на запрет `read::<ThreadModel` во view; `run_line`
   отдать в apply; `Submit` должен нести ThreadId composer'а.
10. **Документы**: `Kind` в core против 19b; architecture.md:746 (несуществующий тест); путь
    `tests/app/layers.rs` в AGENTS.md; реестр ADR-007; `disallowed_types = "deny"`.
