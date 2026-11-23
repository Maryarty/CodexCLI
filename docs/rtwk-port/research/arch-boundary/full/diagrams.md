# tui-rtwk: диаграммы целевой архитектуры

К [architecture.md](architecture.md) и [review.md](review.md). d1–d7 — исправленные схемы из `research/arch-boundary/diagrams.md` (695e300e5); что исправлено, сказано под каждой. d8–d15 — новые. Числа — строки кода на `b2eeacb7c`.

## d1. Текущая карта слоёв и крейтов

Исправлено: добавлены зависимость фич от picker (UI-формы в `Feature`), `outgoing` в core и шаги фикстур, строящие формы списков в core.

```mermaid
flowchart TB
  subgraph HOST["host 6,2k"]
    H1["in_process: app-server"]
    H2["effects, facts: конфиг"]
  end
  subgraph UI["UI-крейт 77,6k"]
    U0["app 2,7k: UiApp, view, presenter"]
    U1["pages 45,2k: 5 страниц"]
    U2["sheets 13,6k: шторки"]
    U3["widgets 7,3k: picker"]
    U4["features 0,3k: FeatureView"]
    U5["services 7,6k: shell"]
  end
  subgraph CORE["core 69,2k без rtwk"]
    C1["app 5,2k: CodexApp, outgoing"]
    C2["threads 7,3k: ThreadModel"]
    C3["screens 47,1k: модели экранов"]
    C4["features 2,3k: Feature с Menu, Pick, Prompt"]
    C5["commands, keymap, protocol 6,4k"]
    C6["fixtures: шаг screen"]
  end
  H1 -->|"Ingress"| U5
  U5 -->|"ingest"| C1
  C1 --> C2
  C1 --> C3
  C1 --> C4
  U1 -->|"ScreenIntent, жесты"| C3
  U2 -->|"Choose id строки"| C3
  U3 -->|"list, layout"| C3
  C4 -->|"Menu формы picker"| C3
  U4 -->|"menu_list"| C4
  C6 -->|"seed форм списков"| C3
  C1 -->|"AppFx"| U5
  U5 -->|"RPC"| H1
```

## d2. Текущие владельцы состояния

Красное — отрисовка в core, жёлтое — смесь. Исправлено: `plugins.shown`, `resume.previews`, слой presenter, `outgoing` без очистки на attach, четыре seq-слушателя.

```mermaid
flowchart LR
  subgraph UIAPP["UiApp"]
    CA["CodexApp"]
    US["UiState: nav_key, presenter"]
    TU["ThreadUi: композер, follow, modes"]
  end
  CA --> SES["session"]
  CA --> TS["ThreadSet"]
  TS --> TM["ThreadModel: core, slots, features"]
  CA --> RQ["requests: снимаются на attach"]
  CA --> OG["outgoing: не снимаются на attach"]:::mixed
  CA --> FS["FeatureStates: один Rc"]
  CA --> SS["ScreenStates: один Rc"]
  SS --> SEL["select: stack, kept"]:::draw
  SS --> PLG["plugins: cache, waiting и menu, tab, shown"]:::mixed
  SS --> RES["resume: listing, query и density, expanded"]:::mixed
  SS --> SET["settings: facts и open, edits"]:::mixed
  SS --> DLG["dialogs: open, page"]:::draw
  TM --> RST["restore seq"]
  RST -->|"слушатель по ThreadId, seq"| TU
  W["State виджетов: курсор, прокрутка"]
  classDef draw fill:#fdd,stroke:#a33
  classDef mixed fill:#ffd,stroke:#aa3
```

## d3. Целевая карта слоёв

Исправлено: kit — отдельный модуль без корней домена; шторки в `SheetHost` или через `show_dialog`; presenter-ы; виджеты не называют половины экранов.

```mermaid
flowchart TB
  subgraph HOST["host"]
    H["app-server, конфиг, IO"]
  end
  subgraph UI["UI-крейт"]
    PR["app: view, presenter-ы"]
    P["pages: 5 страниц"]
    SHT["SheetHost или show_dialog"]
    S["sheets: approvals, dialogs, settings"]
    F["features/f: open, шторка, слова"]
    W["widgets: picker, SheetStack, Interest"]
    SH["services: shell"]
  end
  subgraph CORE["core без rtwk"]
    D["app, threads: домен"]
    DF["features/f: глаголы, каталог, Op"]
    DS["screens: только домен"]
    K["kit: чистые функции"]
  end
  P --> SHT
  SHT --> S
  SHT --> F
  P -->|"глагол"| D
  S -->|"ответ на запрос"| D
  F -->|"Intent::Feature"| DF
  F --> W
  P --> W
  W -.-> K
  D -->|"проекции, срезы"| P
  DS -->|"срез экрана"| P
  DF -->|"срез фичи"| F
  D -->|"запросы, желания"| PR
  PR -->|"открыть"| SHT
  D -->|"AppFx"| SH
  SH -->|"RPC"| H
  H -->|"Ingress"| SH
  SH -->|"ingest"| D
```

## d4. Целевые владельцы состояния

Исправлено: вложенный `Navigator` заменён на `SheetStack`; добавлены `Interest`, `Directive`, `SurfaceWish`, `ops` по `OpKey`, `awaiting`.

```mermaid
flowchart LR
  subgraph CORE["core: Д, З, Ж, И"]
    CA["CodexApp"]
    TM["ThreadModel: транскрипт, Directive"]
    FS["FeatureStates: каталоги, Remote, ops"]
    SS["ScreenStates: query, listing"]
    IN["interests: счётчики"]
    WQ["wishes: SurfaceWish"]
  end
  subgraph UI["UI: Н"]
    UA["UiState: nav_key, presenter-ы"]
    TU["ThreadUi: контроллеры, seen seq"]
    WS["State шторки: SheetStack, awaiting"]
    CT["контроллеры rtwk"]
    NV["Navigator: страницы"]
  end
  CA --> TM
  CA --> FS
  CA --> SS
  CA --> IN
  CA --> WQ
  FS -->|"select среза"| WS
  SS -->|"select среза"| WS
  WS --> CT
  WS -->|"глагол"| CA
  WS -->|"Interest Open, Close"| IN
  WQ -->|"presenter"| UA
  UA -->|"open"| WS
  TM -->|"Directive по seq"| TU
  NV --> WS
```

## d5. Уведомление стрима хода: сейчас и в цели

Сейчас (без изменений):

```mermaid
sequenceDiagram
  participant AS as app-server
  participant H as host
  participant SH as shell
  participant UA as UiApp
  participant CA as CodexApp
  participant TM as ThreadModel
  participant V as view
  AS->>H: item/agentMessage/delta
  H->>SH: Ingress с эпохой
  SH->>UA: ingest
  UA->>CA: ingest, проверка эпохи R1
  CA->>CA: project и project_thread
  CA->>TM: threads::apply ThreadEv
  CA->>CA: ScreenThreadEv в slots экранов
  CA-->>UA: AppFx
  UA->>SH: run_effects
  UA->>V: view заново
  V->>V: watch ThreadModel, ListView
```

Цель. Исправлено: slots остаются, это доменные части экранов на поток (род Д), переносить их некуда. Меняются только срезы и `select`.

```mermaid
sequenceDiagram
  participant AS as app-server
  participant H as host
  participant SH as shell
  participant UA as UiApp
  participant CA as CodexApp
  participant TM as ThreadModel
  participant V as view
  AS->>H: item/agentMessage/delta
  H->>SH: Ingress с эпохой
  SH->>UA: ingest
  UA->>CA: ingest, проверка эпохи R1
  CA->>TM: threads::apply ThreadEv
  CA->>TM: ScreenThreadEv в slots
  CA-->>UA: AppFx
  UA->>UA: follow_core, presenter-ы
  UA->>SH: run_effects
  UA->>V: view заново
  V->>V: select ThreadModel активного, живой хвост
```

## d6. /plugins install: сейчас и в цели

Сейчас (без изменений):

```mermaid
sequenceDiagram
  actor U as Пользователь
  participant PK as Picker
  participant MD as MenuDialog
  participant CA as CodexApp
  participant PL as plugins в core
  participant SH as shell
  participant AS as app-server
  participant H as host
  U->>PK: Enter на строке
  PK->>MD: Event::Choose id
  MD->>CA: Intent::Choose через ingress
  CA->>PL: apply_choice, choose id
  PL->>PL: menu = InstallLoading, ask Install
  CA->>SH: Command::Plugins
  SH->>AS: plugin/install
  AS-->>SH: ответ, конфиг записан
  SH->>CA: Reply с тикетом
  CA->>PL: Ev::Plugins Answer
  PL->>PL: menu = шаг авторизации, заметка
  H->>CA: AppEv::PluginConfig
  CA-->>MD: watch ScreenStates
  MD->>PK: новая страница из core
```

Цель. Исправлено: ожидание `awaiting` в шторке, ключ операции — `PluginKey` с `gen`, двойное нажатие схлопывается, Auth открывается желанием даже при закрытой шторке (как у оракула, plugins.rs:446-500).

```mermaid
sequenceDiagram
  actor U as Пользователь
  participant PS as PluginsSheet
  participant SP as SurfacePresenter
  participant CA as CodexApp
  participant PF as plugins домен
  participant SH as shell
  participant AS as app-server
  U->>PS: Enter на Install
  PS->>PS: awaiting = Install key, gen 3
  PS->>CA: Intent::Feature Install key
  CA->>PF: intent
  PF->>PF: ops key = gen 4, Running
  CA->>SH: Command::Feature
  U->>PS: Enter ещё раз
  PS->>CA: Install key
  CA->>PF: ops key в полёте, игнор
  SH->>AS: plugin/install
  AS-->>SH: ответ, конфиг записан
  SH->>CA: Reply с тикетом и эпохой
  CA->>PF: answered
  PF->>PF: ops key = Done, заметка
  PF->>CA: wish PluginAuth key
  CA-->>PS: select среза plugins
  PS->>PS: страница = f от стека, awaiting, ops
  CA-->>SP: wishes в ingest
  alt шторка закрыта
    SP->>PS: open с страницей Auth
  else шторка открыта
    PS->>PS: replace_top Auth
  end
  SP->>CA: WishShown seq
```

## d7. Фича в цели: обе половины

Исправлено: `SheetStack` вместо вложенного `Navigator`; `Wish`, `Interest`, `on_interest`; правила.

```mermaid
flowchart LR
  subgraph CORE["core/src/features/plugins"]
    I["Intent: Load, Install, Uninstall, SetEnabled"]
    ST["State: каталог по cwd, Remote, ops"]
    WR["wire, decode: RPC"]
    N["Note, Fact, Wish"]
    HK["хуки: on_fact, on_interest"]
  end
  subgraph UIH["src/features/plugins"]
    O["open cx, args"]
    SH["PluginsSheet: State, alive"]
    SK["SheetStack: List, Detail, Auth"]
    TC["TabController, ListController"]
    IT["Interest Plugins cwd"]
    STR["strings.rs: слова"]
  end
  subgraph T["тесты"]
    T1["core/tests/plugins.rs"]
    T2["tests/sheets/plugins.rs"]
    T3["tests/oracle: без изменений"]
  end
  O --> SH
  SH --> SK
  SH --> TC
  SH --> IT
  IT -->|"Open, Close"| HK
  SH -->|"глагол"| I
  ST -->|"select"| SH
  I --> ST
  ST --> WR
  HK --> ST
  N --> STR
  T1 --> CORE
  T2 --> UIH
  T3 --> UIH
```

## d8. Запуск

```mermaid
sequenceDiagram
  participant M as host main
  participant RT as tokio
  participant UA as UiApp
  participant SH as shell
  participant AS as app-server
  participant CA as CodexApp
  participant NV as Navigator
  M->>M: arg0, CLI, язык, факты запуска
  M->>RT: рантайм
  M->>UA: run_app_view, awaits_facts
  UA->>NV: MultiProvider, Navigator, StartupGate пуст
  UA->>SH: старт backend
  SH->>AS: in-process или облако
  SH->>CA: attach, эпоха 1
  SH->>CA: AppEv::Facts
  CA->>CA: onboarding: шаги, start_held
  CA-->>NV: StartupPage рисует шаг
  loop каждый шаг
    NV->>CA: глагол шага
  end
  CA->>SH: ThreadAsk Start
  SH->>AS: thread/start
  AS-->>CA: ответ
  CA->>CA: ThreadSet::open, session.active
  CA-->>UA: follow_core
  UA->>UA: ThreadUi нового потока
  UA->>NV: SessionPage в thread_scope id
```

## d9. Поток: переключение и переподключение

```mermaid
sequenceDiagram
  actor U as Пользователь
  participant AG as AgentsPage
  participant CA as CodexApp
  participant SH as shell
  participant UA as UiApp
  participant SC as thread_scope
  participant TU as ThreadUi
  U->>AG: Enter на потоке B
  AG->>CA: Intent::OpenThread B
  alt писатель B есть
    CA->>CA: session.active = B
  else
    CA->>SH: один resume B
    SH-->>CA: ответ, ThreadSet::open B
    CA->>CA: session.active = B
  end
  CA-->>UA: follow_core
  UA->>SC: ключ ThreadId B
  SC->>TU: контроллеры ThreadUi B
  SC->>SC: директивы B по seen B
  SC->>SC: шторки потока A: alive ложно, закрыты
  Note over SH,CA: обрыв связи
  SH->>CA: Disconnected
  SH->>CA: attach_backend, эпоха 2
  CA->>CA: requests backend сняты
  CA->>CA: outgoing backend: Failed Disconnected
  CA->>SH: переподписка, thread/read
  CA->>CA: interests больше 0: on_interest open
  CA-->>UA: шторки видят Failed или Loading
```

## d10. Шторка: жизнь и закрытие доменом

```mermaid
stateDiagram-v2
  [*] --> Closed
  Closed --> Mounted: open жестом или SurfacePresenter
  Mounted --> Live: init_state, Interest Open
  Live --> Live: жест меняет SheetStack
  Live --> Waiting: глагол, awaiting
  Waiting --> Live: ops gen больше awaiting
  Live --> Closing: Esc на первой странице
  Live --> Closing: alive ложно
  Waiting --> Closing: alive ложно
  Live --> Closing: желание снято
  Live --> Closing: поддерево снято
  Closing --> Disposed: close_dialog или слот пуст
  Disposed --> [*]: Interest Close, фокус назад
```

## d11. Директива контроллеру

```mermaid
sequenceDiagram
  participant CA as CodexApp
  participant TM as ThreadModel A
  participant DL as слушатель restore
  participant TU as ThreadUi A
  participant CO as композер A
  CA->>TM: give_back, Directive seq 7
  alt A показан
    TM-->>DL: select среза A
    DL->>TU: seen.restore меньше 7
    DL->>CO: текст в контроллер
    DL->>TU: seen.restore = 7
  else A не показан
    Note over DL,TU: директива ждёт в модели
    DL->>TU: при показе A сравнить seen
    DL->>CO: применить один раз
  end
```

## d12. Интерес поверхности

```mermaid
sequenceDiagram
  participant SH1 as PluginsSheet
  participant IW as Interest
  participant CA as CodexApp
  participant PF as plugins домен
  participant SV as shell
  SH1->>IW: монтирование
  IW->>CA: Interest Plugins cwd, Open
  CA->>CA: счётчик 0 в 1
  CA->>PF: on_interest open
  PF->>SV: plugin/list
  Note over CA: переподключение
  CA->>PF: on_interest open снова
  SH1->>IW: dispose_with
  IW->>CA: Interest Plugins cwd, Close
  CA->>CA: счётчик 1 в 0
  CA->>PF: on_interest close
```

## d13. Карта модулей и ответственности

```mermaid
flowchart TB
  subgraph HOST["host: main, рантайм, app-server"]
    HF["facts, launch, effects"]
  end
  subgraph UIC["UI-крейт: Н, слова, жесты"]
    AP["app: UiApp, UiState, view, presenter-ы"]
    SV["services: shell, единственный IO"]
    PG["pages: Startup, Session, Agents, Resume, Usage"]
    ST["sheets: approvals, dialogs, settings"]
    FE["features/f: open, шторка, strings"]
    WG["widgets: Picker, SheetStack, Interest, directives"]
    KM["keymap, l10n, theme"]
  end
  subgraph CORE["core: Д, З, Ж, И без rtwk и IO"]
    CAP["app: CodexApp, apply_app, ingest"]
    CTH["threads: ThreadModel, apply"]
    CFE["features/f: глаголы, Op, Remote"]
    CSC["screens: домен экранов"]
    CCM["commands: slash в Target"]
    CKM["keymap: разбор, контексты"]
    CPR["protocol, rpc, ids"]
    CKT["kit: чистые функции"]
    CFX["fixtures: скрипты"]
  end
  HOST -->|"Ingress"| SV
  SV -->|"ingest"| CAP
  CAP --> CTH
  CAP --> CFE
  CAP --> CSC
  CAP --> CCM
  CAP --> CKM
  CTH --> CPR
  CFE --> CPR
  CAP -->|"AppFx"| SV
  AP --> PG
  PG --> ST
  PG --> FE
  PG --> WG
  ST --> WG
  FE --> WG
  WG --> CKT
  KM --> CKM
  CFX -->|"через ingest"| CAP
```

## d14. Пирамида тестов

```mermaid
flowchart TB
  L7["7 perf: pty A/B против оракула"]
  L6["6 сценарии: оба бинаря в pty"]
  L5["5 паритет: tests/oracle через kit"]
  L4["4 виджеты: Tester над UiApp"]
  L3["3 kit: unit"]
  L2["2 домен: core/tests через ingest"]
  L1["1 правила: tests/rules"]
  L7 --> L6
  L6 --> L5
  L5 --> L4
  L4 --> L3
  L3 --> L2
  L2 --> L1
```

## d15. Фокус и клавиши

```mermaid
flowchart TB
  KEY["клавиша из rtwk-input"]
  NB["back_key навигатора: Esc"]
  RT["верхний маршрут или диалог"]
  SHC["Shortcuts шторки из Keymap"]
  ACT["Actions в State шторки"]
  FLD["поле: TextField, Picker"]
  LOC["контроллер: Н"]
  VERB["глагол в ingress: Д"]
  POP["pop маршрута"]
  KMAP["Keymap в core"]
  KEY --> FLD
  FLD -->|"не взято"| SHC
  KMAP -->|"привязки"| SHC
  SHC --> ACT
  ACT --> LOC
  ACT --> VERB
  SHC -->|"не взято"| RT
  RT -->|"не взято"| NB
  NB --> POP
```
