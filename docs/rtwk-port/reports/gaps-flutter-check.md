# Открытые rtwk-гэпы, проверенные по-флаттеровски (2026-11-19, pin e3debfda)

Метод: для каждого открытого или частичного гэпа из `docs/rtwk-port/rtwk-gaps.md` и
`docs/rtwk-port/reports/rtwk-framework-gaps.md` сначала описано, как это сделал бы
Flutter-разработчик. Затем найден аналог в rtwk на пине `~/.cache/codexcli/rtwk-pin`, и
вердикт подтверждён пробой в `/tmp/gapcheck`.

**Итог:** не гэп 12, наш вопрос 4, реальный гэп 5 и одно мелкое замечание.
Не проверялись: G5 (нужен Windows Terminal) и G4 (условный, перепроверить на слайсе pets).

Как перезапустить пробы:
- `cd /tmp/gapcheck && ~/.cache/rtwk/acargo test -q --test gaps -- --nocapture --test-threads=1`
  — 17 проб, все зелёные;
- `~/.cache/rtwk/acargo test -q --test start -- --nocapture` — R-start-1, R-start-2, G11;
- `~/.cache/rtwk/acargo build --example absent` падает с E0599 нарочно: это доказывает,
  что API для G1, G8 и стилизации G12 нет.

## Таблица

| Гэп | Как во Flutter | Путь в rtwk / чего нет | Результат пробы | Вердикт |
|---|---|---|---|---|
| N1 «…» с пробелом | `LayoutBuilder` + `TextPainter.width`; если не влезает, `Row[Expanded(Text clip), Text('…')]` | `LayoutBuilder` + `rtwk::text::segmenter_of(cx).width()`; `Row[Expanded(Text::line(s).overflow(Clip)), Text::line("…")]`. Для начала строки — `OverflowBox(...).alignment(CENTER_RIGHT)` | 4 случая равны оракулу: `" All …"`, `"…Group: …"`, `"a b …"`, `"… efgh"`. У `TextOverflow` выходит `" All…"` (fit.rs:236-262 съедает пробел) | НЕ ГЭП |
| G35 курсор SelectionList | `ListView.builder` + `ScrollController`; курсор во внешнем `ValueNotifier`; клавиши через `Focus(onKeyEvent)`; `Scrollable.ensureVisible` | `ListView::builder` + `ListController::ensure_index_visible` (list_view.rs:134-160) + `SelectionController<usize>` + `ListenableBuilder` на строку + `Focus::new(..).autofocus()` для j/k/ctrl+n/ctrl+p/ctrl+f | k с переходом на 29; ctrl+n ctrl+n j → 02; ctrl+f → 07 (страница = `viewport_dimension`); внешний `set_cursor(20)` виден | НЕ ГЭП |
| G42 окно resume | своё правило top + `jumpTo`, строки «more» как отдельные виджеты | `ListController::jump_to_index(oracle_top)`; ↑ — `ListenableBuilder` по scroll (pixels>0), ↓ — если sel+1<n | `["↑ more","› item-2","    meta 2","","  item-3","↓ more"]` = оракулу | НЕ ГЭП |
| G36 заполненные табы | `LayoutBuilder` + измеренный `Row` табов с окном и ‹ › (TabBar здесь не годится) | то же: `LayoutBuilder` + `segmenter_of` + `Row`; так и сделано в tabs.rs | 36 кол.: `" All 3   Needs you 1   Working 0   ›"` = оракулу; 96 кол. = оракулу. `TabBar` даёт другой вид | НЕ ГЭП (наш tabs.rs — и есть путь Flutter) |
| G38 высота текста | `TextPainter.layout().computeLineMetrics()` | `rtwk_render::objects::text::break_lines` (text.rs:100, в фасаде не экспортирован) | 38 кол.: 4 строки, как рисует `Text` | НЕ ГЭП (экспорт в фасад — пожелание) |
| G40 полоса прогресса | `CustomPaint` + `CustomPainter` | `CustomPaint` + `CellPainter` + `Canvas::fill_char` (custom_paint.rs:28-52, пример в доке — прогресс-бар) | `[██████░░░░░░░░░░░░░░] 28% left` = оракулу | НЕ ГЭП |
| G39 диалог выбора | `Column(min)[header, Flexible(SingleChildScrollView(Table)), hint]`; курсор через `ensureVisible` | `Flexible` + `SingleChildScrollView` + `Grid(GridTrack::content, fr(1), column_gap 2)`; курсор — `ScrollController::jump_to(сумма измеренных строк)` после layout | короткий: по контенту, hint на строке 3; длинный на 8 строках: колонка общая, описания переносятся, hint на последней; `"› 4. None"` в окне | НЕ ГЭП (см. мелкое замечание про ensureVisible) |
| G24 «/mo»+Enter | `Autocomplete.onSelected` → выполнить и `controller.clear()` | `Autocomplete::on_selected` (autocomplete.rs:334) + `ctrl.clear()` | ran=["/model"], поле пустое | НЕ ГЭП |
| «?» / G43 | `FocusNode.onKeyEvent` → `handled` при пустом поле | `FocusNode::set_on_key_event` | «?» на пустом → popups=1, текст ""; «a?» → текст "a?" | НЕ ГЭП (заметка в pages-nav.md устарела) |
| N3 палитра темы | `ThemeExtension` или `Provider` рядом с `Theme` | `Theme::new(..., Provider::value(Palette).child(..))`, обе из одного состояния | тема и палитра переключаются вместе | НЕ ГЭП (ThemeExtension — только удобство) |
| R-start-2 проба терминала | — | `RunOptions.terminal = TerminalOptions{probe_timeout, ..}` (info.rs:358/389, по умолчанию 1 s) | компилируется; оракул платит тот же round trip | НЕ ГЭП |
| KeyHintsBar | — | закрыто через G41 (`LayoutBuilder` + варианты) | — | НЕ ГЭП (заметка устарела) |
| G37 путь «…/» | `TextOverflow.ellipsis` режет посимвольно, сегменты — своей функцией | `EllipsisMiddle` даёт `"~/hello/the/…s/very/fast"`, оракул — `"…/very/fast"`; чистая `center_truncate_path` перенесена в path.rs | path.rs = оракулу | НАШ (перенос чистой логики) |
| R-start-1 atexit | — | `libc::atexit` (unix.rs:1185) дёшев: 7-20 µs на 1.2 MB; 3.5 ms — это таблица символов 333 MB бинаря | stripped ×2 быстрее | НАШ (`strip="symbols"` в профиле) |
| G11 копирование | `Clipboard.setData` | `AppView::on_copy` | компилируется | НАШ |
| G12 поведение | `TextEditingController` + перехват Backspace | `EditingKeys` (key_hook.rs) → `c.replace(s,e,"")` | "see [Pasted 12 chars]" → "see " | НАШ (сделать через EditingKeys) |
| G9 выделение в ленте | `SelectionArea` над `ListView`; `SelectionContainerDelegate` держит выделение при выгрузке элементов | `SelectionArea` есть, но якорь живёт в смонтированном элементе (docs/design/transcript.md: «Selection across unmounted items» — до ADR) | внутри окна копируется; перетаскивание за край прокручивает, а копирует **пусто** | РЕАЛЬНЫЙ ГЭП |
| G1 переход в fullscreen | — (во Flutter нет), аналог — `SystemChrome.setEnabledSystemUIMode` | `ScreenMode` задаётся только в `RunOptions` (app_start.rs:25-44); у presenter нет смены режима | E0599 `enter_fullscreen` / `set_screen_mode` | РЕАЛЬНЫЙ ГЭП |
| G8 сброс ввода | — (аналог — `tcflush` у оракула, tui.rs:987) | ни в rtwk-core, ни в rtwk-input сброса нет; у событий нет времени; байты, набранные во время пробы, ложатся в `early` (terminal/mod.rs:2081-2112) | E0599 `discard_pending_input` | РЕАЛЬНЫЙ ГЭП |
| G12 стиль элемента | `TextEditingController.buildTextSpan` | у `TextArea` нет хука для span-стиля (text_area.rs:175-330); оракул красит элементы в cyan (textarea.rs:2311) | E0599 `span_builder` | РЕАЛЬНЫЙ ГЭП |
| N4 перенос после «/» | ICU (UAX #14, класс SY) переносит после «/» | `TextWrap::Word` переносит только после пробела, после дефиса и между CJK (text.rs:54-60) | Word: `"…in codex-"`; с U+200B после «/» = оракулу `"…in codex-rs/"`, но обход меняет копируемый текст | РЕАЛЬНЫЙ ГЭП |
| G5 | — | нужен Windows Terminal | не проверить здесь | — |
| G4 | — | условный | перепроверить на pets | — |

## Реальные гэпы и API в форме Flutter

1. **G9 — выделение через невидимые элементы.** Нужен `SelectionArea` над `ListView`, который хранит якорь как (индекс элемента, смещение), а не ссылку на элемент. Текст невидимых элементов берётся у делегата `item_text(index)`, как `SelectionContainerDelegate` во Flutter. Пример: `ListView::builder(..).selectable_text(|i| String)`. Без этого копирование после автопрокрутки возвращает пусто.
2. **G1 — смена режима экрана на ходу.** `AppCx::enter_fullscreen()` / `leave_fullscreen()` или `AppCx::set_screen_mode(ScreenMode)`. Presenter должен переходить inline↔alt screen без перезапуска.
3. **G8 — сброс ожидающего ввода.** `AppCx::discard_pending_input()`: очищает очередь событий и `early`, делает `tcflush(TCIFLUSH)`. Нужно перед интерактивным экраном, как у оракула.
4. **G12 (стиль) — стилизация диапазонов в TextArea.** Аналог `TextEditingController.buildTextSpan`: `TextArea::span_builder(|text| Vec<(Range, Style)>)` или диапазоны элементов со стилем в контроллере. Поведенческая половина уже решается через `EditingKeys`.
5. **N4 — перенос UAX #14.** `TextWrap::Unicode` (полный UAX #14) или класс SY («/») в `TextWrap::Word`. Обход через ZWSP работает, но портит копирование.
6. *Мелкое:* нет публичного `Scrollable.ensureVisible(context)`. `AppCx::request_visible_rect` (app.rs:1018) и `Cx::request_viewport` (cx.rs:346) помечены `#[doc(hidden)]`. Предложение: `AppCx::ensure_visible(&GlobalKey)`. Пока работает `ScrollController::jump_to` с измеренным смещением (проба G39).

## Fix list (по файлам; пробы — `/tmp/gapcheck/tests/gaps.rs`, функции названы по гэпу)

**docs/rtwk-port**
- `parity/pages-nav.md:21-31` — заметка про «?» («EditingKeys пробовали, взяли on_changed») устарела. Заменить на «`FocusNode::set_on_key_event`, G43 — не гэп; так сделано в chat/home.rs:247-257».
- `research/reuse-map.md:124` («KeyHintsBar not used in parity») и `:185` («TabBar has another look») — заметить: KeyHintsBar закрыт через G41; заполненные табы — это `LayoutBuilder` + `Row` (tabs.rs), не гэп.
- `rtwk-gaps.md`:
  - строки N1, G35, G36, G38, G39, G40, G42, G24, N3 → статус «not a gap», с маршрутом из таблицы;
  - G37, G11, R-start-1 → «ours»;
  - G9, G1, G8, G12-стиль, N4 → остаются открытыми, с API из списка выше.
- `reports/rtwk-framework-gaps.md` — то же сокращение.

**codex-rs/tui-rtwk/src**
- `screens/agents/path.rs` (вызов view.rs:423 `path::fit_heading`) — оставить; это перенос чистой `center_truncate_path` оракула (комментарий о файле оракула обязателен). В R15 указать причину «ours: carried pure logic», не G37.
- `screens/agents/soft.rs` (вызов view.rs:752 `soft::rows`, DETAILS_WIDTH view.rs:172) — оставить: `break_lines` — это аналог `TextPainter`. Убрать метку гэпа G38.
- `screens/agents/tabs.rs` (вызов view.rs:45) — оставить: это путь Flutter (`LayoutBuilder` + `segmenter_of` + `Row`). Убрать метку гэпа G36.
- `screens/agents/view.rs:229` (`neighbour`, шаг G35), `:543`, `:677` (строки `SelectionList`) → `ListView::builder(..).item_extent(..)` + `ListController` + `SelectionController<usize>` + `ListenableBuilder` на строку + `Focus` с клавишами оракула; после каждого шага `ensure_index_visible`. Образец — проба `g35_*`.
- `screens/agents/view.rs:1156` (choices `SelectionList`) — то же, если нужны клавиши или окно оракула; иначе оставить.
- `screens/agents/view.rs`, места с `Text::line(..).overflow(Ellipsis…)` для N1-строк (уведомление о старом сервере и заголовки) → общий виджет `ellipsis_end(s)` / `ellipsis_start(s)` (`LayoutBuilder` + `Row[Expanded(clip), "…"]`). Образец — проба `n1_*`.
- `screens/approvals/dialog.rs:136` (`SizedBox::new(None, Some(count), …)`) и `:164-195` (`Table`) → `Column(min)[header, Flexible(SingleChildScrollView(Grid[content, fr(1)]).column_gap(2)), hint]`. Курсор — `ScrollController::jump_to(сумма измеренных строк)` после первого layout. Образец — проба `g39_*`.
- `screens/chat/home.rs:75` (`Picked`), `:290-300`, `:472` (`on_selected`) → в `on_selected` сразу выполнить `Suggestion.text` и вызвать `controller.clear()`. Промежуточный ход через текст композера не нужен. Образец — проба `g24_*`.
- `screens/pets/mod.rs:245` (`SelectionList`) → как у agents (G35).
- `screens/resume/mod.rs:255-289` (`list_of` :258, `SelectionList`) → `ListView::builder` + `ListController::jump_to_index(top)`. `top` считать по правилу `ensure_selected_visible` / `available_content_rows` оракула (resume_picker.rs:2224, :2465-2512). Строки ↑/↓ more — отдельные виджеты. Образец — проба `g42_*`.
- `screens/select/dialog.rs:103`, `:119` (`SizedBox`) → как approvals (G39).
- `screens/status/format.rs:41-47` (`progress_bar`, `"█".repeat`/`"░".repeat`) → виджет `CustomPaint` с `CellPainter`, где `fill_char('█')` на `round(w*pct/100)` и `'░'` на остаток. Образец — проба `g40_*`.
- `theme.rs:23` (`Tokens(Rc<ThemeData>)`) → палитру кода вынести в `Provider::value(Palette)` рядом с `Theme`; строить обе из одного состояния, читать через `watch::<Palette>`. Образец — проба `n3_*`.

**codex-rs/tui-rtwk/tests**
- `agents.rs:129-137` (`agents_overview_tab_strip_draws_oracle_filled_tabs_on_tabbar`, pending G36) → удалить: TabBar здесь не путь, tabs.rs уже даёт строку оракула.
- `agents.rs:205` (pending ReuseCarry, цвета потоков, N3) → снять pending после переноса палитры в `Provider` (theme.rs).
- `agents.rs:345-356` (`older_server_notice_truncates_in_narrow_overview`, pending N1) → снять pending после `ellipsis_end`.
- `agents.rs:669`, `:680` (live_center page keys, pending G35) → снять pending после перевода на `ListView` + `Focus`.
- `oracle/approvals_2.rs:631`, `:668`, `:707`, `:722` и `oracle/chat_popups_2.rs:181` (pending G39) → снять после `Flexible` + `SingleChildScrollView` + `Grid`.
- `oracle/resume_2.rs:139` (pending G42) и комментарий `resume.rs:59` → снять после `jump_to_index`.
- `pets.rs:81` (`#[ignore = "pending(G35)…"]` `the_cursor_row_stays_in_the_window_when_the_arrows_pass_its_end`) → снять ignore.
- `rules.rs:488-501` `R15_GAPS`:
  - path.rs/G37 (:490-491) → причина «ours: carried oracle pure fn»;
  - tabs.rs/G36 (:494-495) и soft.rs/G38 (:498-499) → «measure, not a gap» (разрешённое измерение через `segmenter_of` / `break_lines`).
- `rules.rs:502-507` `R15_NOT_YET`: убрать `status/format.rs` после `CustomPaint`; `select/dialog.rs` — после Grid.
  - chat/home.rs и dialogs/feedback.rs — проверить отдельно.
- `status.rs:33-38` (`#[ignore pending(G40)]`) → снять ignore.

**Профиль сборки (R-start-1)** — `strip = "symbols"` в релизном профиле бинаря. Это вне `codex-rs/Cargo.toml` области порта, так что решает человек.
