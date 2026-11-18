# Открытые пробелы rtwk framework, найденные портом tui-rtwk

Для мейнтейнера rtwk. Состояние на 2026-11-18. Проверено против rtwk main
`e3debfda` (читался исходник `/Users/admin/Documents/dev_projects/rtwk`, без
изменений) и против нашего пина `9f28b7fb`. Источники: `docs/rtwk-port/rtwk-gaps.md`
(строки G1..G36), отчёт о первопричинах `why-not-rtwk.md` с пробами,
`docs/rtwk-port/reviews/vs1-review.md` (S5), `docs/rtwk-port/parity/agents-live.md`,
`docs/rtwk-port/reports/2026-11-18-status.md` (п. 6).

Все пробы запущены на `e3debfda` в крейте `/tmp/rtwk-gaps-probe`
(`tests/gaps.rs`, `tests/probe.rs`, `tests/front.rs`, `tests/g21.rs`; `/tmp`
временный, поэтому ключевые фрагменты кода приведены ниже). Между пином
`9f28b7fb` и `e3debfda` в `rtwk-controls`, `rtwk-scroll`, `rtwk-unicode` и
`rtwk-widgets/src/text` нет изменений (только бюджеты тестов), так что все
открытые ниже строки подтверждены на main, а не только на пине.

## Итог

Строки G1..G36 в `rtwk-gaps.md` (G3 и G6 сняты в architecture.md, в таблице их
нет, это 34 строки) плюс 4 новых пробела, которых в таблице ещё нет (N1..N4).

| статус | число | что |
|---|---|---|
| открыт | 10 | G35, G36, N1, N2, N3, G1, G9, G24, G8, N4 |
| закрыт в main или в пине | 17 | G2, G7, G13, G14 (рецепт), G16, G17, G18 (рецепт), G19, G20, G25, G28, G29..G34 |
| частично | 3 | G5 (Windows Terminal не проверен), G11 (OSC 52 + `on_copy`; нативное копирование намеренно у приложения), G12 (рецепты есть, атомарных элементов нет) |
| не пробел или наша сторона | 8 | G4, G10, G15, G21, G22, G23, G26, G27 |

Открытые, по тому, сколько порта каждый разблокирует:

1. N1 `TextOverflow` теряет пробелы рядом с `…` (Ellipsis и EllipsisStart)
2. N2 `SelectionList`: построитель строки не знает, выбрана ли она и стоит ли на ней курсор
3. G35 `SelectionList` прячет `ListController` (прокрутка к курсору, страница, wrap, свои клавиши)
4. G36 `TabBar` не рисует закрашенную полосу вкладок оригинала
5. N3 `ThemeData` без слота расширений (как `ThemeExtension` во Flutter)
6. G12 (остаток) `TextArea`: атомарные элементы (вставленные блоки, упоминания)
7. G9 выделение и копирование по виртуализованному транскрипту
8. G1 переключение alt screen во время работы inline-приложения
9. G24 `Autocomplete`: выбор нельзя перехватить вместо вставки
10. G8 сброс накопленного ввода (typeahead) перед интерактивным экраном
11. N4 перенос слов `TextWrap::Word` не режет после `/` (необязательно)

(G12 в счёте стоит среди «частично», в списке выше показан его открытый остаток; поэтому в списке 11 пунктов, открытых строк 10.)

Закрытое подтверждено прогоном или чтением: `rtwk::style::Attrs` (06cf797e),
`run_app_view` (06cf797e), `Tester::pump_app_in` (673732d1, 3352d721),
`Dialog::bare` (1296f507), `EditingKeys` (e16f72e0, 0161be70), `Suggestion::enabled/marks`
и `Autocomplete::marker` (там же), `AppCx::clear_history` (e3debfda),
ссылки и картинки в committed rows (015fb54b), Zellij (gaps-inline).

## Открытые пробелы

### N1. `TextOverflow::Ellipsis` и `EllipsisStart` выбрасывают пробелы рядом с `…`

Новая строка (в `rtwk-gaps.md` её нет; G26 был про другое и оказался не пробелом,
см. конец). Подтверждено прогоном на main. Разблокирует больше всего: усечение
одной строки с `…` встречается в каждой строке списка и заголовка порта.

- Что нужно оригиналу. `truncate_line_with_ellipsis_if_overflow`
  (`codex-rs/tui/src/line_truncation.rs:79-105`): обрезать до `width - 1`, оставить
  пробелы как есть, затем `…`. `front_truncate`
  (`codex-rs/tui/src/text_formatting.rs:152-170`) делает то же с начала:
  `…` и хвост, который влез, с ведущим пробелом.
- Что делает rtwk. `fit_text` пропускает пустоты рядом с меткой:
  `skip_blanks` в ветках `EllipsisStart` и `EllipsisMiddle`
  (`crates/rtwk-unicode/src/fit.rs:236-262`), `is_cut_blank` (`fit.rs:46`), то же
  для `Ellipsis`. Это сознательное поведение, но оно расходится с оригиналом.
- Проба (3 из 7 случаев расходятся; `/tmp/rtwk-gaps-probe/tests/probe.rs::ellipsis`):

  | строка | ширина | rtwk | оригинал |
  |---|---|---|---|
  | `" All 1 "` | 6 | `" All…"` | `" All …"` |
  | `"Agent command center  Group: Project  g"` | 30 | `"Agent command center  Group:…"` | `"Agent command center  Group: …"` |
  | `"a b c d e"` | 5 | `"a b…"` | `"a b …"` |
  | `"abcd efgh"`, EllipsisStart | 6 | `"…efgh"` | `"… efgh"` |

  ```rust
  Text::new(" All 1 ").wrap(TextWrap::None)
      .overflow(TextOverflow::Ellipsis).max_lines(1)   // w=6 -> " All…", want " All …"
  ```
- Предлагаемая форма. Не менять умолчание. Либо отдельные варианты
  (`TextOverflow::EllipsisExact`, `EllipsisStartExact`), либо флаг на `Text`:
  `Text::keep_blanks(true)` (в `fit_text` параметр `CutBlanks::{Drop, Keep}`,
  `skip_blanks` вызывается только при `Drop`). Флаг проще: не множит варианты
  enum.
- Обход у нас. `src/screens/agents/layout.rs` (`ellipsize`, `front_truncate`
  через `fit_text` и `TextSegmenter` напрямую, ~70 строк; ревью VS1 просит
  убрать их после закрытия пробела).
- Тесты, ждущие пробела. Маркера `pending(Gxx)` нет: снимки зелёные за счёт ручной
  функции (`older_server_narrow_overview`, `agents_overview_*`, `live_center_*` в
  `tests/agents.rs`). После закрытия они остаются эталоном, а ручные функции уходят.

### N2. `SelectionList`: построитель строки не знает, выбрана ли она и на ней ли курсор

Новая строка (ревью VS1, S5: `docs/rtwk-port/reviews/vs1-review.md:128-133, 424`).

- Что нужно оригиналу. Строки списка агентов, resume и usage рисуют маркер `›`
  и закраску выбранной строки; цвет маркера и пробелов под ним зависит от
  «выбрана» и «курсор» (`codex-rs/tui/src/app/agents_overview_view.rs`, снимки
  `agents_overview*`).
- Что делает rtwk. `SelectionList::new(item_count, key_of, item_builder)` берёт
  `item_builder: Fn(usize) -> AnyWidget`
  (`crates/rtwk-controls/src/selection_list.rs:202-211`; `with_builders` тоже
  `IndexedWidgetBuilder`, `:213`). Индекс есть, состояния нет. Закраска делается
  только целиком строки через `selected_style` / `cursor_style`
  (`:305`, `:312`), а свой маркер (`›` вместо `▎`) и перекраска содержимого
  недоступны.
- Минимальная проба: в `item_builder` нечем спросить, выбрана ли строка;
  единственный путь, `ListenableBuilder` на `SelectionController` в каждой строке
  (так сделано у нас).
- Предлагаемая форма: `SelectionList::item_state_builder(Rc<dyn Fn(usize, ItemState) -> AnyWidget>)`,
  где `ItemState { selected: bool, cursor: bool, focused: bool, enabled: bool }`;
  либо `SelectionList::marker(Str)` + `selection_fill(bool)` для частого случая
  (маркер и закраска без пересборки строки).
- Обход у нас. `src/screens/agents/view.rs:529-575` (`ListenableBuilder` на
  строку, `selected_style(Style::new())`), ~30 строк; список из-за свежих
  замыканий не равен себе и пересобирается каждый раз (S5).
- Ждущие тесты: маркера нет; снимки `agents_overview*`, `live_center_*`.

### G35. `SelectionList` прячет `ListController`

Статус в `rtwk-gaps.md`: open. Подтверждено прогоном на main.

- Что нужно оригиналу. Агентский центр: `move_selection` с переходом с конца на
  начало (`codex-rs/tui/src/app/agents_overview_view.rs:375-394`), страницы
  (`ListAction::PageUp/PageDown`, `:748`, `:889`), Home/End, прокрутка к
  выбранному (`ensure_selected_visible`, `codex-rs/tui/src/resume_picker.rs:2224`
  для того же списка), заголовок группы над первой задачей, `j`/`k`/ctrl-n/ctrl-p/
  ctrl-f/ctrl-b, и страницы, пока фокус в строке поиска.
- Что делает rtwk. `ListCore.list: ListController` приватный
  (`crates/rtwk-controls/src/selection_list.rs:351`, создаётся `:386`); прокручивает
  только `move_to` (`:495-503`, `ensure_index_visible`); `by` зажимает в
  границах (`:506`); таблица клавиш фиксирована (`:951-958`: стрелки, страницы,
  Home/End, `SelectionMsg::PageDown`). У `SelectionController` нет вызова
  «шаг» или «страница».
- Проба (`tests/gaps.rs::selection_list_does_not_follow_an_external_cursor`):
  ```rust
  let c = SelectionController::<usize>::default();
  t.pump_widget(SelectionList::new(30, |i| i, |i| Text::new(format!("row {i:02}")).into()).controller(c.clone()));
  c.set_cursor(Some(20));
  t.pump_until_idle();   // экран 12x5 остаётся на row 00..04
  ```
- Предлагаемая форма (в стиле rtwk):
  `SelectionList::list_controller(ListController)` (приложение отдаёт свой, как
  `ScrollController` у `ListView`), либо `SelectionController::set_cursor`
  сам просит прокрутку; `SelectionList::wrap(bool)`;
  `SelectionController::step(isize)` и `page(isize)`; `SelectionList::key_table(..)`
  или `.extra_keys(&[("j", Msg::Down), ..])`, чтобы лишние клавиши шли тем же
  путём, что и свои.
- Обход у нас. `src/screens/agents/{mod,view}.rs` берёт стрелки/страницы виджета
  и шагает курсор по `Select` ядра для `j`/`k`/ctrl-n/ctrl-p (на одну строку);
  `neighbour`/`step_of`, ~25 строк.
- Ждущие тесты (`pending(Owner::Rtwk, ..)`): `live_center_pages_visible_rows_while_reconnecting`
  (`codex-rs/tui-rtwk/tests/agents.rs:501`) и
  `live_center_page_keys_and_wrap_follow_the_list_keymap` (`:513`). Это единственные
  маркеры, привязанные к пробелу rtwk во всём каталоге `tests`.

### G36. `TabBar` не рисует закрашенную полосу вкладок оригинала

Статус: open. Подтверждено прогоном на main (`tests/probe.rs::tabbar`,
`tests/gaps.rs::tabbar_new_and_strip`; `TabBar::strip` даёт ту же раскладку, он
меняет только стили).

- Что нужно оригиналу. `render_filled_tab_bar`
  (`codex-rs/tui/src/bottom_pane/selection_tabs.rs:42-…`): вкладки шириной
  `label + 2`, между ними одна пустая клетка; метки `‹`/`›` занимают по две
  клетки вне вкладок; окно из целых вкладок выбирается по
  `occupied(start, end) <= width` вокруг активной (`:56-61`); активная вкладка:
  `selection` + bold на обычной строке. Снимки `agents_overview_attention` и др.
  (5 вкладок, ширины 40..160).
- Что делает rtwk. Вкладки вплотную (`starts[i+1]` начало следующей,
  `crates/rtwk-controls/src/tabs.rs:915-935`); метки рисуются поверх первой и
  последней клетки строки (`:960`); прокрутка по колонке `offset`, а не по целым
  вкладкам.
- Проба (ширины 80, 44):
  ```text
  w=80  " All 1  Needs you 0  Working 0  Ready 1  Inactive 0"      // оригинал: по 3 пробела между вкладками
  w=44  " All 1  Needs you 0  Working 0  Ready 1  In›"             // метка затёрла клетку вкладки
  ```
  ```rust
  TabBar::new(&TabController::new(5), ["All 1","Needs you 0","Working 0","Ready 1","Inactive 0"])
  ```
- Предлагаемая форма: `TabBar::gap(u16)`, `TabBar::marks_outside(bool)`,
  `TabBar::window(TabWindow::WholeTabs)` (прокрутка по целым вкладкам);
  стиль активной вкладки на обычной строке задаётся уже `selected_style`, так что
  пункт (d) из `rtwk-gaps.md` закрывается стилями. Для полного совпадения
  достаточно (a)-(c).
- Обход у нас. `src/screens/agents/layout.rs` (`strip`, окно оригинала чистой
  функцией, юнит-тесты на 44 и 76 клетках) и `view.rs` (`header`, спаны в
  одной `Text::rich_line`, `GestureDetector` на вкладку), ~100 строк.
- Ждущие тесты: маркера `pending` нет (снимки зелёные за счёт ручной полосы):
  `agents_overview*` и `live_center_*` в `tests/agents.rs`.

### N3. `ThemeData` без слота расширений

Новая строка (`parity/agents-live.md`, `reports/2026-11-18-status.md` п. 6).

- Что нужно оригиналу. Цвет потока агента зависит от темы подсветки синтаксиса:
  `thread_color` (`codex-rs/tui/src/thread_color.rs:22`) хэширует id FNV-1a и
  берёт акцент активной темы (`render/highlight.rs`,
  `current_syntax_theme`). Такая палитра живёт рядом с темой, но не входит в её
  фиксированные слоты.
- Что делает rtwk. `ThemeData` — struct с фиксированными полями
  (`crates/rtwk-widgets/src/style/theme.rs:131-175`: `background`, `surface`,
  `text`, `muted`, `primary`, `accent`, `border`, `selection`, `error`, `warning`,
  `success`, `info`, плюс `chrome`/`well`/`faint`/`dim`/…); прикрепить к теме
  данные приложения нельзя, `copy_with` меняет только имеющиеся слоты.
- Проба: `ThemeData::DARK.copy_with(|t| ...)` не принимает ничего вне слотов;
  нужен отдельный `InheritedWidget`, не переключающийся вместе с `Theme`
  (тема меняется, палитра остаётся от старой).
- Предлагаемая форма, как `ThemeExtension` во Flutter:
  ```rust
  trait ThemeExtension: Clone + 'static { fn lerp(&self, other: &Self, t: f32) -> Self { other.clone() } }
  ThemeData::DARK.with_extension(CodexPalette::dark())
  Theme::extension::<CodexPalette>(cx)   // Option<&CodexPalette>
  ```
  Хранение: `Rc<AnyMap>` в `ThemeData` (клон остаётся дешёвым), `copy_with`
  сохраняет расширения; смена `Theme` пересобирает зависимые.
- Обход у нас: палитра сессий живёт рядом с темой (`src/theme.rs`, `Tokens`),
  не carried.
- Ждущие тесты: `overview_thread_colors_match_footer_and_respect_color_suppression`
  (`tests/agents.rs:205`, `pending(Owner::ReuseCarry, ..)`; его причина шире
  (нет палитры syntect/two_face), расширение темы лишь место, куда палитра
  ляжет); сам маркер на rtwk не ссылается.

### G12 (остаток). `TextArea`: атомарные элементы

Статус: partly. Закрыто в main через `EditingKeys`
(`crates/rtwk-editing/src/key_hook.rs:34-146`, e16f72e0, 0161be70): рецепты
kill ring и yank, вызова истории, vim как состояние приложения; `on_paste`
(`text_area.rs:131`) даёт вставку первым. Эти части считаем закрытыми.

- Остаётся. Атомарные элементы: диапазон текста, который курсор перепрыгивает,
  Backspace удаляет целиком, ввод внутри разбивает; рисуется своим стилем
  (вставленные блоки, упоминания, картинки). Оригинал:
  `codex-rs/tui/src/bottom_pane/textarea.rs:124` (`TextElement`), `:276`
  (`set_text_with_elements`). Определение paste burst по таймингу без bracketed
  paste (`bottom_pane/paste_burst.rs:12`) приложение сделает само на
  `EditingKeys`, в framework не просим.
- rtwk: `rtwk-editing/src` не содержит ни атомарных элементов, ни диапазонов с
  ограничениями курсора.
- Предлагаемая форма: `TextEditingController::add_element(range, ElementId)` /
  `elements()` (диапазоны следуют за правками буфера), стиль через
  `TextArea::element_style(|id| Style)`; правила курсора и удаления внутри
  `TextArea`.
- Ждущие тесты: оракул `oracle/textarea.rs` (61 тест, ~8 про элементы),
  `oracle/composer_*.rs` (24 теста с `burst` в имени); висят на слайсе
  composer (`kit::pending`), не на rtwk, пока слайс не начат.

### G9. Выделение и копирование по виртуализованному транскрипту

Статус: open, решение у вас (в rtwk это сознательное «по виджету»).

- Что нужно оригиналу. Выделение мышью и копирование текста по всему
  транскрипту, включая строки вне экрана (`codex-rs/tui/src/text_selection.rs`,
  `rendered_selection.rs`); автопрокрутка у края
  (`oracle/transcript_view_5.rs:580, 601`); пейджер (S16) просматривает тот же
  текст.
- Что делает rtwk. `docs/design/transcript.md:80-86`: выделение по элементам,
  ушедшим из списка, остаётся «по виджету» (`LogView`, `CodeView`, Markdown), пока
  третий случай не оправдает механизм уровня `SelectionArea` через ADR. Порт это
  третий случай: транскрипт и пейджер.
- Предлагаемая форма: `SelectionArea` над `ListView` с адресом выделения
  «индекс элемента + смещение» и колбэком `item_text(index) -> String` для
  элементов вне окна; копирование собирает текст из колбэка, а не из клеток.
- Ждущие тесты: слайсы S10/S16 (`kit::pending` в `oracle/transcript_view_*.rs`);
  маркеров Gxx нет.

### G1. Переключение alt screen во время работы inline-приложения

Статус: open (нет `set_mode`/`enter_alt` в `rtwk-core/src`; `ScreenMode`
задаётся при старте, `crates/rtwk-core/src/presenter/modes.rs:63`; `App::suspend`
отдаёт терминал другой программе и режим не меняет,
`crates/rtwk-core/src/app.rs:3076`).

- Что нужно оригиналу. Пейджер и diff-оверлей открываются на alt screen поверх
  inline-чата и возвращают историю (`codex-rs/tui/src/tui.rs:1136`
  `enter_alt_screen`, `:1189` `leave_alt_screen`; вызовы `app_backtrack.rs:207`,
  `model_migration.rs:387`, `tui.rs:869, 973`).
- Предлагаемая форма: `AppCx::enter_fullscreen()` / `leave_fullscreen()` (или
  `ScreenMode`, меняемый обработчиком), с гарантией, что committed rows остаются в
  обычном буфере и регион после выхода перерисован корректно.
- Ждущие тесты: слайс S27 (inline), `oracle/pager.rs`; маркеров Gxx нет.

### G24. `Autocomplete`: выбор нельзя перехватить вместо вставки

Статус: open (узко; rtwk ответил «действуйте по `Suggestion.text`»).

- Что нужно оригиналу. Команда, выбранная в попапе слэш-команд, выполняется
  вместо вставки её текста (`codex-rs/tui/src/bottom_pane/chat_composer.rs`,
  слайс chat-nav).
- Что делает rtwk. `Autocomplete::pick`
  (`crates/rtwk-editing/src/autocomplete.rs:791-801`) сперва `c.replace(..)`, затем
  `selected.push` (`on_selected`, `:334`); `EditingKeys` спрашивается после
  показывающих клавиш `Autocomplete`, так что Enter при открытом попапе до него
  не доходит.
- Обход у нас. `src/screens/chat/home.rs:222, 375`: `on_selected` как сообщение
  после замены.
- Предлагаемая форма: `Autocomplete::on_pick(Rc<dyn Fn(&Suggestion) -> PickAction>)`
  с `PickAction::{Insert, Take}`; `Take` закрывает попап и не трогает текст.
- Ждущие тесты: Gxx-маркеров нет.

### G8. Сброс накопленного ввода

Статус: open, низкий приоритет.

- Что нужно оригиналу. Перед интерактивным экраном выбросить ввод, набранный
  заранее: `discard_pending_input_before_interactive_screen`
  (`codex-rs/tui/src/tui.rs:987-993`; вызовы `daemon_recovery.rs:64`,
  `update_prompt.rs:68`, `model_migration.rs:153`; тесты
  `tui_startup_tests.rs:47, 314`).
- Что делает rtwk. Ни в `rtwk-core`, ни в `rtwk-input` нет сброса; есть только
  `rtwk-term/src/pty/raw_tty.rs:131-143` (`restore(discard_input)`).
- Замечание. Если экраны до запуска остаются отдельными приложениями, это делается
  нашей стороной до `run_app_view`; пробел настоящий, только когда они маршруты
  одного приложения.
- Предлагаемая форма: `AppCx::discard_pending_input()`, сбрасывающий события,
  пришедшие до вызова.
- Ждущие тесты: Gxx-маркеров нет.

### N4. `TextWrap::Word` не режет после `/` (необязательно)

Новая строка (`why-not-rtwk.md`, раздел про wrap). Влияет на строки с путями и URL.

- Проба: `textwrap` 0.16 (`WordSeparator::new()`, UAX #14, `HyphenSplitter`,
  `FirstFit`, `break_words`) против `TextWrap::Word`: 10 из 63 случаев (в общей
  пробе 10 из 45 для сравнения с ручным wrap) расходятся, все с путями и URL;
  проза и три тела диалогов идентичны на ширинах 20..72
  (`/tmp/rtwk-gaps-probe/tests/probe.rs::wrap`).
  ```text
  w=34 "Fix the flaky test in codex-rs/tui-rtwk/tests/agents.rs and re-run ..."
  оригинал: "Fix the flaky test in codex-rs/"      rtwk: "Fix the flaky test in codex-"
  ```
- Оригинал: `word_wrap_lines` (`codex-rs/tui/src/wrapping.rs:1332`).
- Предлагаемая форма: `TextWrap::WordUax14` (перенос возможен после `/` как в
  UAX #14), либо опция у `Word`.
- Ждущие тесты: Gxx-маркеров нет; ручной wrap у нас расходится в 17 из 45
  случаев, rtwk в 10 из 45.

## Частично

- G5, Zellij и Windows Terminal. Zellij закрыт в main (`Multiplexer::Zellij`, поиск
  по `ZELLIJ` или XTVERSION, `hardware_scroll`; `docs/design/inline.md` §1.12.6).
  Windows Terminal «unverified» (там же, строка 282); `WT_SESSION` только в
  определении простого терминала (`terminal/env.rs:76`). Остаётся проверка, не код.
- G11, буфер обмена. Закрыто частично: `ClipboardSelection::{Clipboard, Primary,
  Both}` и `AppView::on_copy` (06cf797e, fa4e1bb7). Нативное копирование (pbcopy,
  wl-copy, xclip, clip.exe, WSL), которым пользуется оригинал
  (`codex-rs/tui/src/clipboard_copy.rs:97-318`, `arboard`, tmux), rtwk намеренно не
  запускает; это работа `on_copy` у нас.

## Наша сторона (не framework)

- G4: картинки в регионе. Условно, перепроверить на слайсе питомцев
  (`rtwk-rich` `Image`, `widget.rs:144`); проб пока нет.
- G10: зонды tmux (`display-message`, `show-options`:
  `codex-rs/tui/src/tui/tmux_tests.rs:109-150`). Это запуск процесса `tmux`; по
  решению rtwk процессы запускает приложение (shell-слой порта).
- G15: смена списка провайдеров обнуляет `Navigator` без ключа. Семантика Flutter;
  у нас `GlobalKey` (`src/app.rs:21, 48`), в rustdoc `MultiProvider` это теперь
  сказано.
- G21: `Scrollable` не шлёт начальный `ScrollNotification`. Проба на main:
  `NotificationListener` получает 0, но `ScrollController` уведомляет слушателя на
  первой раскладке (1 уведомление, `max_scroll_extent = 25`,
  `/tmp/rtwk-gaps-probe/tests/g21.rs`), как в twk. Не пробел.
- G22: закраска выбранной строки. `ColoredBox`/`Container::color` в растягивающей
  колонке; наша работа (часть закроет N2).
- G23: нет «~» под коротким содержимым пейджера. rtwk ответил: `LayoutBuilder` и
  строки `~`; косметика (`tests/pager.rs:27-33` вычёркивает `~`). Если захотите в
  framework: `SingleChildScrollView::fill_glyph(Str)`.
- G26: `Text::new("› ")` в `Row` теряет хвостовой пробел. Проба на main:
  `Text::new("› ").wrap(TextWrap::None)` в `Row` даёт `"› X"`. Не пробел,
  применено у нас.
- G27: `Focus::bind(Esc)` берёт Esc раньше Navigator. Есть
  `Focus::on_key(cx.keys(..).passing())` (`rtwk-framework/src/focus/keys.rs:376`);
  применяем у нас.

## Что решить

1. Какие из N1..N4 завести в rtwk как задачи (N1 и N2 дают больше всего; N1 и G35
   самые дешёвые).
2. G9: открыть ли ADR на `SelectionArea` уровня списка, раз транскрипт и пейджер
   стали третьим случаем.
3. G36: закрашенная полоса как режим `TabBar` или отдельный `TabStrip`.
