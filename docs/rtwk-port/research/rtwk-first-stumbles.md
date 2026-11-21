# Where porters stumble on rtwk-first, and what helps (2026-11-21)

Research by codex-reviewer (rtwk-gaps.md, reviews/*.md, tests/rules, the skill, tm, git log); saved by the orchestrator.

## Synthesis
Porters rarely get the principle wrong; they fail on its boundary. Of ~48 gap rows ~20 closed "not a gap / by design" (G21-23, G26-28, G35, G36, G39-G46, N1-N3, G38); real rtwk limits are rare (G48, N4, G1, G8, G12 partly). Main cost: wrongly claimed gaps; second: hand-rolled mechanics. Root causes: the porter tests ONE widget and stops (the Flutter route is a composition or a test setting, written down nowhere); four skill port-table rows point at the wrong widget.

## Ranked patterns
1. Gap claimed after testing only the named widget: G36/G45 (TabBar; `pages/agents/tabs.rs` existed; select went back to TabBar e3e98f3c6), G39 (Table -> Grid in SingleChildScrollView, 8c02a00d9), G35/G42 (SelectionList -> ListView::builder + ListController + SelectionController, f01ba600b, widgets/session_list.rs), G40 (CustomPaint + CellPainter, status/card.rs:170), G44/G46 (pattern 5). Wrong skill rows: tab strip -> TabBar; Table + widths -> Table; center_truncate_path -> EllipsisMiddle (G37); truncate_text -> Ellipsis (N1). Landed answers (dots.rs, tabs.rs, options.rs, session_list.rs) indexed nowhere.
2. Hand windowing/cursor/scroll in the view: sheets/select/dialog.rs (top, window_top, rows[top..end]; S1), trust.rs Highlight (F1), signin rem_euclid (O2), import/nav.rs 258 lines (I3), resume scroll_top Rc<Cell>, pets top: Cell; four list mechanisms at once (N-S3). Cause: oracle ScrollState shape, no single port list named.
3. Width arithmetic R15 does not see: trust.rs:225-285 (F2); string_width(.., TextMode::Graphemes, ..) in sheets/dialogs/shortcuts.rs:83, pages/resume/chrome.rs:16, widgets/status_line.rs:164, widgets/page_keys.rs:87; vs1 D4 usize underflow. R15_PATTERNS (tests/rules/rules.rs:637) lacks segmenter_of(, string_width(, measure::, width -.
4. Composition vs workaround undefined: panel.rs words() = workaround (G48: pending, no workaround); dots_end, tabs.rs = compositions. process.md says both "compose until rows equal" and "no workaround".
5. Config differences taken for gaps: G44 = TextMode::Legacy (rtwk-unicode segment.rs:153; rtwk-test tester.rs:316); G46 = presenter drops invisible DIM (rtwk-core style/color.rs:150-152) -> assert frame_loop().styles().attrs_of(surface.style(x,y)).
6. Dialogs by hand instead of show_dialog + Dialog::bare (rtwk-overlay dialog.rs:822).
7. Hand key/focus routing: .passing() (G27), FocusNode::set_on_key_event (G43).
8. Over-pending (203 of 255 stale were vacuous) and state shadows (tab: Option beside TabController, N-S1).

## Help
1. Fix the four skill rows + add the cookbook (generator source, not SKILL.md). Checklist before a gap row: three routes with file:line (widget alone; composition: LayoutBuilder/Row/Grid+GridTrack/ListView+controllers/CustomPaint; test setting: TextMode, attrs_of) + grep src/widgets, src/pages/agents. Reviewer: no route 2 = FIX. Rule: pending(Gxx) only on an open row.
2. One list: widgets/session_list.rs (cursor rows), widgets/options.rs (numbered options); controllers own cursor and scroll. Rule M2 (window state scan).
3. R15_PATTERNS += segmenter_of(, string_width(, measure::, TextMode::, width - ; allowlist measuring owners only. A width enters only as a branch under a LayoutBuilder.
4. Boundary into process §5, fleet brief, porter agent: composition = rtwk widgets lay out and paint, the app only picks WHICH widget/spelling from a measured width; cutting/rewrapping/re-measuring strings = workaround -> gap row + pending.
5. Kit Screen::legacy() and attrs_at(x,y) (gaps-close-w2).
6. Non-page panes: show_dialog + Dialog::bare first.
7. "Keys and focus" recipe.
8. Pending sweep (KIT_PROBE_PENDING=1) once per wave.

## Draft cookbook
Try in order: the widget alone; a composition; a test setting. Only then a gap row.

| oracle pattern | rtwk route |
|---|---|
| filled tab strip, whole tabs, ‹ › | NOT TabBar: pages/agents/tabs.rs (pure strip() window + Row under LayoutBuilder, widths by segmenter_of(cx)) |
| `…` keeping blanks | Ellipsis drops them (N1): widgets/dots.rs dots_end (LayoutBuilder + Row[Expanded(clipped Text), Text("…")]) |
| list with cursor, whole-row scroll, ↑/↓ more | widgets/session_list.rs: ListView::builder + ListController (rtwk-scroll list_view.rs:134-157) + SelectionController + ListenableBuilder + NotificationListener |
| numbered options, label column + wrapped description | widgets/options.rs: Column(min)[Flexible(SingleChildScrollView(Grid[content().within(0,cap), fr(1)]))]; NOT Table |
| label/value card | Grid with content tracks (pages/session/cells/status/card.rs) |
| bar of whole cells | CustomPaint + CellPainter (status/card.rs:170) |
| footer hints, first spelling that fits | widgets/page_keys.rs footer_fit |
| bottom-pane view (frameless, ›) | show_dialog + Dialog::bare, BOTTOM_CENTER (sheets/select/dialog.rs:416) |
| rows a Text takes at width w | break_lines (rtwk-render text.rs:100) via widgets/measure.rs; first try Flexible / max_lines |
| display width | segmenter_of(cx) (rtwk-widgets text/segmenter.rs:34) |
| `ｶﾞ` as 2 cells | TextMode::Legacy in the test terminal |
| DIM asserted on blanks | assert the app surface style (attrs_of) |
| Wrap{trim:false} blanks-only | real gap G48: pending, no Wrap-of-words |
| path cut keeping first/last segment | pages/agents/path.rs (G37) |
| Esc / ? / ← reaching the page | cx.keys(..).passing() (G27); FocusNode::set_on_key_event (G43) |
| selected-row fill under blanks | Container::color / ColoredBox (G22) |
| prompt "› " trailing blank | Text::new("› ").wrap(TextWrap::None) (G26) |

Not re-run: G22, G26, G27, G43 routes. widgets/options.rs still jumps scroll by measure::rows (no public ensureVisible from a handler): candidate ergonomics row for the person.
