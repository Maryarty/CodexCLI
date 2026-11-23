# arch-boundary: диаграммы

К [report.md](report.md). Числа — строки кода на `b2eeacb7c`.

## d1. Текущая карта слоёв и крейтов

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
    C1["app 5,2k: CodexApp"]
    C2["threads 7,3k: ThreadModel"]
    C3["screens 47,1k: модели 19 экранов"]
    C4["features 2,3k: Feature"]
    C5["commands, keymap, protocol 6,4k"]
  end
  H1 -->|"Ingress"| U5
  U5 -->|"ingest"| C1
  C1 --> C2
  C1 --> C3
  C1 --> C4
  U1 -->|"ScreenIntent"| C3
  U2 -->|"Choose id строки"| C3
  U3 -->|"list, layout"| C3
  U4 -->|"menu_list"| C4
  C1 -->|"AppFx"| U5
  U5 -->|"RPC"| H1
```

## d2. Текущие владельцы состояния

Красное — отрисовка в core, жёлтое — смесь домена и отрисовки.

```mermaid
flowchart LR
  subgraph UIAPP["UiApp"]
    CA["CodexApp"]
    US["UiState: nav_key, presenter"]
    TU["ThreadUi: композер, follow"]
  end
  CA --> SES["session"]
  CA --> TS["ThreadSet"]
  TS --> TM["ThreadModel: core, slots, features"]
  CA --> RQ["requests"]
  CA --> FS["FeatureStates"]
  CA --> SS["ScreenStates"]
  SS --> SEL["select: stack, kept"]:::draw
  SS --> PLG["plugins: cache, waiting и menu, tab"]:::mixed
  SS --> RES["resume: listing, query и density, expanded"]:::mixed
  SS --> SET["settings: facts и open, edits"]:::mixed
  SS --> DLG["dialogs: open, page"]:::draw
  W["State виджетов: курсор, прокрутка"]
  classDef draw fill:#fdd,stroke:#a33
  classDef mixed fill:#ffd,stroke:#aa3
```

## d3. Целевая карта слоёв

```mermaid
flowchart TB
  subgraph HOST["host"]
    H["app-server, конфиг, IO"]
  end
  subgraph UI["UI-крейт"]
    P["pages: 5 страниц"]
    S["sheets: approvals, dialogs"]
    F["features/f: шторка, страницы, слова"]
    W["widgets: picker со своими list, layout"]
    SH["services: shell"]
  end
  subgraph CORE["core без rtwk"]
    D["домен: app, threads"]
    DF["features/f: глаголы, каталог, операции"]
    DS["screens: только домен"]
    K["kit: vim, paste_burst, история"]
  end
  P -->|"глагол"| D
  S -->|"ответ на запрос"| D
  F -->|"Intent::Feature"| DF
  F --> W
  P --> W
  P -.->|"чистые функции"| K
  D -->|"проекции"| P
  DS -->|"ScreenStates"| P
  DF -->|"FeatureStates"| F
  D -->|"AppFx"| SH
  SH -->|"RPC"| H
  H -->|"Ingress"| SH
  SH -->|"ingest"| D
```

## d4. Целевые владельцы состояния

```mermaid
flowchart LR
  subgraph CORE["core: что спросили и что ответили"]
    CA["CodexApp"]
    TM["ThreadModel"]
    FS["FeatureStates: каталоги, Op, Remote"]
    SS["ScreenStates: query, range, listing"]
  end
  subgraph UI["UI: что нарисовано"]
    UA["UiState: nav_key, presenter"]
    TU["ThreadUi: композер, прокрутка"]
    WS["State шторки: страница, вкладка"]
    CT["контроллеры rtwk: курсор, табы"]
    NV["Navigator: стек страниц"]
  end
  CA --> TM
  CA --> FS
  CA --> SS
  FS -->|"select"| WS
  SS -->|"select"| WS
  WS --> CT
  WS --> NV
  WS -->|"глагол через ingress"| CA
  TM -->|"select"| TU
```

## d5. Уведомление стрима хода: сейчас и в цели

Сейчас:

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

Цель (поток — домен, путь почти не меняется):

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
  Note over CA,TM: доменные части chat и status в ThreadModel, без slots экранов
  CA-->>UA: AppFx
  UA->>SH: run_effects
  UA->>V: view заново
  V->>V: select ThreadModel, ListView
```

## d6. /plugins install: от клавиши до записи конфига и обновления

Сейчас:

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

Цель:

```mermaid
sequenceDiagram
  actor U as Пользователь
  participant PS as PluginsSheet
  participant CA as CodexApp
  participant PF as plugins домен
  participant SH as shell
  participant AS as app-server
  participant H as host
  U->>PS: Enter на Install
  PS->>PS: своя страница Detail
  PS->>CA: Intent::Feature Install через ingress
  CA->>PF: intent
  PF->>PF: ops id = Installing
  CA->>SH: Command::Feature
  SH->>AS: plugin/install
  AS-->>SH: ответ, конфиг записан
  SH->>CA: Reply с тикетом
  CA->>PF: answered
  PF->>PF: ops id = Installed с apps, заметка
  H->>CA: AppEv::Feature Fact конфиг
  CA-->>PS: select FeatureStates
  PS->>PS: страница = f от Detail и ops id
```

## d7. Фича в цели: обе половины

```mermaid
flowchart LR
  subgraph CORE["core/src/features/plugins"]
    I["Intent: Load, Install, Uninstall, SetEnabled"]
    ST["State: каталог, ops, Remote"]
    W["wire, decode: RPC"]
    N["Note, Fact"]
    HK["хуки: on_fact, on_scope_changed"]
  end
  subgraph UIH["src/features/plugins"]
    O["open cx, cwd"]
    SH["PluginsSheet: State"]
    NV["вложенный Navigator: List, Detail, Auth"]
    TC["TabController, ListController"]
    STR["strings.rs: слова"]
  end
  subgraph T["тесты"]
    T1["core/tests/plugins.rs"]
    T2["tests/sheets/plugins.rs"]
    T3["tests/oracle: без изменений"]
  end
  O --> SH
  SH --> NV
  SH --> TC
  SH -->|"глагол"| I
  ST -->|"select"| SH
  I --> ST
  ST --> W
  HK --> ST
  N --> STR
  T1 --> CORE
  T2 --> UIH
  T3 --> UIH
```
