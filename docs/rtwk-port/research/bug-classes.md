# Which Codex TUI bug classes disappear on rtwk

Research, read-only on both repos. Date 2026-11-17. Sources: git history of
codex-rs/tui (+ tui2), workarounds in codex-rs/tui/src, public issues of
openai/codex (gh worked, no rate limit hit), rtwk docs, rtwk-for-codex.md §2,
architecture.md (ADR-003 open).

## 1. Summary

History base: 3,612 non-merge commits touched codex-rs/tui or tui2 since
2025-04-24; 508 have a bug-flavoured subject (fix, revert, stale, duplicate,
race, panic, crash, hang, flicker, garbled...), the "fix set" below. Subject-
keyword classification, primary class per commit, then a verdict per class;
the 169 "other" commits are mostly backend/config/CI/product text and were
split by a 50% sample read (about 80% non-UI). The shares carry roughly +-5
points of error, they are an estimate, not a count.

| verdict | share of 508 | what is in it |
|---|---|---|
| GONE by construction | ~10% (~53) | cursor placement, suspend/restore, manual Rect/height/clip arithmetic and zero-area panics, partial/doubled-row/stale-cell artifacts, OSC52/tmux clipboard + notification plumbing, key-sequence repair |
| SHRINKS | ~27% (~139) | terminal quirks (tmux, zellij, kitty, Windows, Terminal.app), Unicode width/graphemes, text wrap, inline scrollback/resize, paste, images, parts of rendering |
| STAYS (our logic, backend, terminal reality) | ~62% (~316) | state desync (stale popups/rows after events), event-order races, key routing and bindings, composer logic, markdown/diff semantics, product/backend. Of this, ~22 points (state_ui+race, ~111) are what ADR-003 (state/logic separated from UI) can shrink; rtwk alone does not |
| NEW RISK (extra, not a share) | n/a | Windows evidence partial, inline popups unverified, committed-history reflow (terminal owns it), alt-screen pager missing, paste-burst/input-history/placeholder atoms are app code, 4.7 ms/token large live table, API instability |

File-touch cross-check (same 508 commits, one commit can touch several areas):
controller/app state 244, bottom pane/composer/popups 142, transcript/history
cells 74, terminal layer (custom_terminal, insert_history, tui.rs, terminal_*,
clipboard_copy, notifications) 49, layout/text/wrap/render 43, input/keys/paste
23. The terminal and layout layers rtwk replaces are ~18% of fix commits; the
state/controller/bottom-pane layer ~75%.

Top classes that go away: (1) manual Rect/height/clip arithmetic and zero-area
panics, (2) partial frames, ghost/doubled rows and stale cells, (3) cursor
placement, (4) suspend/resume and terminal restore, (5) measure-vs-paint width
mismatches for Unicode/emoji cells (contained, not removed: width errors never
leave the glyph's own cells). Also OSC52/tmux clipboard and key-sequence repair.

Top stay/new: (1) state desync and stale UI after async events (ADR-003, not
rtwk), (2) event-order races and thread-switch/resume duplicates, (3) NEW RISK:
Windows/ConPTY, inline-region popups and committed-history reflow.

## 2. Class table

Counts: C = fix-set commits (primary class, of 508), I = open+closed openai/codex
issues with label TUI and the keyword in the title (label TUI total 2,593;
body-text counts are noisy and listed in section 3), W = workaround lines in
codex-rs/tui/src (grep, lines/files). Verdict in caps.

| # | class | C | I | W | examples | mechanism in Codex | verdict, rtwk mechanism, what keeps it gone |
|---|---|---|---|---|---|---|---|
| 1 | Rendering artifacts: flicker, ghost/doubled rows, stale cells, partial frames | 24 | flicker 6, garbled 8, duplicate 18 | custom_terminal.rs 1,635 loc; "flicker in some terminals" app/session_lifecycle.rs:591 | 841f057f2 doubled blank rows while streaming (#26636); 488a40211 doubled lines/hanging list markers (#2789); 5a4b2702f clear first inline viewport render (#21450); issues #25004, #48129, #52003 | ratatui diff against a custom terminal copy; Codex owns buffer-vs-terminal truth, DECSTBM-style scroll regions and its own insert_history | GONE for artifacts caused by the paint path. rtwk: damage spans + presenter model of the terminal, 0-byte no-change frame, DEC 2026 envelope, autowrap off (design/presenter.md §1.4.2, surface.md §1.3); kept by presenter property tests (32 caps sets x scenes, replay through a terminal model, inline.md §1.12.5) and allocation budgets. Semantic render bugs (wrong markdown/diff output) stay |
| 2 | Manual Rect/height arithmetic, clipping, zero-size | 35 (about half are text/test/clippy noise) | wrap 18 | Rect::new / Rect { 982 lines in 257 files; saturating_ 1,156 lines in 268 files | 1e82bf9d9 panic when exec cell area is zero height (#3133); 0de154194 MIN_ANIMATION_HEIGHT (#3656); 03cb32941 status truncation for wide Unicode (#61692) | each widget computes Rects and heights by hand; measure and paint are separate code | GONE for arithmetic that moves into render-tree constraints (rtwk-render box/flex/stack/grid, ARCHITECTURE.md layers) once components are ported; ratatui-core text widgets kept via ADR-004 keep the old arithmetic until replaced. Kept by rtwk's tested layout and the oracle goldens; Codex insta snapshots (1,549 .snap) become the parity suite |
| 3 | Wrapping and width mismatch between measure and paint | incl. in 2 (27 by keyword) | wrap 18, unicode 7, emoji 3 | wrapping.rs 2,096 loc; word_wrap/wrap_width 355 lines / 80 files; unicode_width 65 / 29 | f27cf9db0 wrapped prose beside URLs (#21760); 4b0f5eb6a wrapping bugfix (#4674); 7ecd3153a wrap history items (#1685); 8a17396d1 live stream reflow at allocated width (#58927) | Codex wraps to lines, caches, then paints; width from unicode-width crate, terminals disagree | SHRINKS. One text engine for measure and paint (rtwk-render text object; cut_agreement is a pending port test in unicode.md §1.5.9, say so). URL-preserving wrap and history-cell wrap stay Codex logic. Kept by fit_text reference tests and snapshot parity |
| 4 | Unicode: graphemes, emoji, CJK, bidi | 10 | unicode 7, emoji 3, CJK/IME 26+36 (body search) | unicode_width 65 lines / 29 files | e649c6d74 grapheme highlight in history search (#61725); 82b061afb CJK word navigation in composer (#16829); bd65c4db8 crash backspacing placeholders next to multibyte (#2674); issue #60588, #37578 | custom editing and cursor math over bytes/chars; width crate vs terminal | SHRINKS. Painting: GONE-ish, UAX#29 own segmenter, uncertain-width containment (ECH, rewrite of overflowed cells; unicode.md §1.5.5-1.5.8), bidi in core-bidi. Editing logic (word motion, @-token, placeholders) stays unless TextArea replaces the composer; TextArea has grapheme-aware editing but lacks atoms. Kept by GraphemeBreakTest (853 lines), 12,000 differential strings vs twk, adversarial set. Caveat: the "disagreeing terminal" Rust test is still to be written (unicode.md §1.5.6) |
| 5 | Resize and reflow | 4 | resize 15 | app/resize_reflow.rs 775 loc; tests/suite/resize_reflow.rs 726 loc; "Some terminals settle final width after repaint" resize_reflow.rs:502 | 5591912f0 reflow scrollback on resize (#18575); 5fa7d46dd resize on wezterm (#2600); c66c99c5b crash on resize (#1683); issue #58516, #37987 | Codex re-renders its transcript and re-inserts history on resize | SHRINKS. Live region: Screen::resize repaints from scratch, cursor-relative, no absolute addressing (inline.md §1.12.2) = GONE. Committed history belongs to the terminal and cannot be re-wrapped (rtwk-for-codex §2 "partial", gap 2): Codex's transcript re-render on resize needs a design. NEW RISK until decided. Kept by modes_oracle (40 inline scenes, 1,748 frames), resize property tests |
| 6 | Inline scrollback insertion/corruption, history | 18 | scrollback 37, history 50 (mixed) | insert_history.rs 1,318 loc; history_ui.rs:392 purge+clear pairing for Terminal.app and Warp; custom_terminal.rs:628 | 5a4b2702f clear first inline viewport render (#21450); 9a638dbf4 propagate errors in insert_history_lines (#4266); 8360c6a3e insert_history modifier handling (#1774); 7daaabc79 alternate_screen config for Zellij scrollback (#8555); issues #62007, #57302 | history lines written above a movable viewport with scroll regions; terminal-specific purge/clear | SHRINKS to GONE for insertion corruption: Screen::commit puts rows in with IL above the region in a synchronized update, never rewritten, no row addressed absolutely (inline.md §1.12.1, §1.12.3). Kept by presenter properties incl. committed rows and commit tests. Transcript pager (Ctrl+T) is a different thing: no alt-screen switch in rtwk (gap 1) = NEW RISK |
| 7 | Cursor placement | 3 | cursor 35 (mixed with composer cursor) | cursor handling in custom_terminal.rs, composer | 679a944db restore cancelled prompt cursor at end (#26457) (logic, STAYS); terminal cursor shown/hidden around frames | Codex positions the hardware cursor after drawing | GONE for terminal cursor (presenter hides during frame, rests_cursor, relative moves only: presenter.md §1.4.2). Composer caret logic stays (editing state) |
| 8 | Terminal-compat quirks: tmux, zellij, kitty keyboard, Terminal.app, iTerm, Ghostty, VS Code | ~30 of the 38 | tmux 41, zellij 14, kitty 18, ghostty 17, terminal.app 3, iterm 5, vscode 11 | text mentions: tmux 290 lines / 31 files, kitty 207/35, iterm 102/23, ghostty 57/21, zellij 42/11; "some terminals" 13/11 | cac535445 Shift+Enter in tmux csi-u panes (#21943); 8a4a537e4 avoid modifyOtherKeys for unknown tmux formats (#24371); 833c19ed5 raw output above composer in zellij (#24593); 0bd31dc38 zellij redraw (#16578); 69283aa1c /copy in tmux (#20207); 1101dec9a disable enhanced keys for VS Code WSL (#18741) | Codex probes env vars and sets keyboard modes per terminal (tui/keyboard_modes.rs 61 tmux lines), with ad hoc fallbacks | SHRINKS. rtwk has a probe (DECRQM 2026/2027/2048/69/2031, CPR, bg colour) and capability-driven output (terminal.md, rtwk-for-codex §2 rows "keyboard enhancement", "terminal probing"). Per-terminal bugs in a terminal's own reporting stay (a Konsole ESC example is already in input.md §1.6.2). Kept by ConPTY/pty e2e and strict VT model in rtwk-test; no rtwk test exists for zellij/tmux behaviours that Codex has no fixture for: gap, port those fixtures into the parity suite |
| 9 | Windows console, ConPTY, WSL | ~12 | windows 182, wsl 40 | Windows-specific cfg/branches across 9 files (conpty/windows terminal 19 lines), wsl 201 lines / 23 files | acd851e89 restore Windows VT before TUI renders (#24082); 7b28b350e flush input buffer on init (#10729); 4c8cf3964 SGR mouse for Windows terminal (#48799); 0f8bb4579 Windows paste non-ASCII multiline (#8774) | crossterm + hand-patched timings (paste burst 30 ms on Windows) | NEW RISK / SHRINKS. rtwk has platform/windows.rs + console_input; CI run 41938132149 passed 11 ConPTY scenarios, TESTING/terminal.md call it partial: no real-user evidence at Codex scale. Needs a Windows CI row in the parity suite from day 1 |
| 10 | Paste, IME, input parsing | 22 | paste 119 (title; many are image/attachment features), IME 36 (body) | bottom_pane/paste_burst.rs 610 loc; chat_composer.rs:276 Windows deliver pastes as rapid Char events | ce49e9284 harden paste-burst state (#9124); ed32da04d IME submissions dropping leading digits (#4359); 36912ce3d shared paste burst interval on Windows (#18914); a3a09dfc9 external editor expansion for same-size large pastes (#21190) | no bracketed paste on some terminals so Codex heuristically detects bursts | SHRINKS. Bracketed paste and key-parse (input.md §1.6.1-1.6.3, lone-ESC and paste timeouts table) are rtwk's, fuzzed (`cargo fuzz`). Paste-burst detection, placeholders ([Image #1]), numbering after Ctrl+C are app code over TextArea (rtwk-for-codex §2 row "multiline editor": missing) = STAYS |
| 11 | Focus and key routing conflicts | 38 | shift 48, ctrl 122, esc 35 (shared with 12) | key_hint.rs 493 loc; activation_key_guard.rs; multi_agents.rs Option+b/f (:140, :166); app/input.rs:561 | 7accd2795 Escape in memory reset confirmation (#58971); 2c087a0cc Escape in read-only agent sessions (#47320); 8a299fb70 /review crash on Esc in steer (#22879); ff66b3c7e alt-enter alias (#20535) | one global keymap plus per-popup stacks, mode flags | STAYS mostly. rtwk gives focus tree, KeyBindings, dialog barrier and focus return (overlay/dialog.rs; rtwk-for-codex §2 "approvals modal"), which removes the "which view gets the key" bug by construction if popups become routed overlays; mapping of Codex shortcuts is our logic. Kept by key-routing tests in rtwk-test + Codex vt100/Tester scenarios |
| 12 | State desync logic vs UI: stale popups/rows, wrong view after events, composer state | 87 + ~34 of "other" | stale 23, popup 12, composer 57, queue 52 | controller: 244/508 fix commits touch app/, chatwidget, app_server_*; bottom_pane 142 | b83105710 clear stale transcript history when switching threads (#43994); 44403dd3d clear stale hook row after turn (#27619); 5c18cc0ac clear stale Guardian reviews (#34371); 764f3c7d0 defer backtrack trim until rollback confirms (#9401); 1096d6453 plan prompt reappearing after /agent switch (#10447) | state and view fused in chatwidget (large components own both data and drawing); every event handler must remember to clear every view | STAYS with rtwk alone (our logic). SHRINKS only through ADR-003: a store of immutable view models with revisions + rtwk_state::Source/Select (1 wake, 1 rebuild per 100 writes) makes "forgot to clear the row" a render of one source of truth. Kept by headless reducer tests (no terminal) plus rtwk-test Tester scenarios; the state design decides the size of this win, rtwk does not |
| 13 | Event-ordering races, duplicates, interleaving | 24 | duplicate 18 | app event channel 335 AppEvent variants; 320 async fn / 1,111 .await in app/ | 6107b9886 duplicate assistant messages resuming Cloud threads (#60335); c313eb75f duplicate messages around interleaved commands (#54365); 4d7a5c7c7 liveness races in side conversations (#34199); e43a2e297 stale background terminal poll events (#23231) | multiple async sources (app-server, threads, timers) mutate UI-held state | STAYS (backend and controller). ADR-002 puts the controller on rtwk's loop: input events are not frame-capped (responsiveness.md), but ordering of backend events is Codex semantics. Kept by Codex's replayable event tests; rtwk fake-clock tests make them deterministic |
| 14 | Frame scheduling, latency, streaming lag, performance | 11 | lag 7, slow 11, cpu 9, freeze 26, hang 24 (mixed; many are backend) | frame request/draw coalescing in tui.rs 1,756 loc | 841f057f2 doubled blank rows while streaming; issues #42421, #24668 freeze, #60294 hang | draw on demand, full re-wrap of transcript per frame, markdown re-parse per delta | SHRINKS. Input is not held to the frame cap, 0 idle frames, 24 us per token for streaming markdown at 1000 turns, 0.52 elements rebuilt per token (rtwk-for-codex §2; PERFORMANCE.md); kept by allocation budgets and responsiveness release gate. NEW RISK: no Codex-shaped benchmark; one huge live table costs 4.7 ms per token at 2,000 rows; 10^3-10^4 cells in an owned retained tree (architecture.md Risks 3) |
| 15 | Suspend/resume, terminal restore, panic restore | 7 | suspend 10, crash 23, panic 9 | tui.rs; panic hook | 76135cbe7 restore TUI after suspend (#28342); 320f150c6 Ctrl-Z suspends TUI (#2113); 861ba8640 show error message after panic (#1752); 3f8c06e45 /review interrupt and exit wedges (#18921) | Codex's own SIGTSTP/raw-mode bookkeeping | GONE for terminal restore and job control: App::suspend_to_shell, terminal_jobcontrol tests with real bash and zsh in a pty, terminal_restore (jobcontrol.md "Defects found and fixed"). POSIX only: Windows has no Ctrl-Z. Panic-in-widget crashes (not terminal state) stay our logic |
| 16 | Hyperlinks, clipboard, notifications (OSC 8/52/9) | 14 (keyword 'link' loose) | hyperlink 8, clipboard 34, link 12 | clipboard_copy/tmux.rs 25 tmux lines; terminal_hyperlinks.rs; notifications/ | b876f8898 clipboard routing for tmux and SSH (#45457); 69283aa1c /copy in tmux without passthrough (#20207); 32e4a3a4d WSL clipboard image paths (#3990) | Codex implements OSC wrapping/passthrough per terminal | SHRINKS. Terminal::copy_to_clipboard is tmux-wrapped, Terminal::notify, Link spans (rtwk-for-codex §2: ready). Clipboard image read (arboard/WSL paths) is OS-level = STAYS |
| 17 | Images | 4 | image 63 (mostly attachments) | pets/image_protocol.rs 33 tmux lines | 00ef9d378 image paste on native Windows (#7514); a803467f5 image paste in WSL (#9473) | Codex reads the clipboard itself and renders pet images with a hand protocol | SHRINKS for display (rtwk-core images with 13 goldens, 10 terminals x 120 scenes properties, 0 alloc); attachment handling stays |
| 18 | Markdown/diff/syntax semantics and theming | ~31 by keyword | markdown 41, diff 40 | render/highlight.rs 1,700 loc; diff_render.rs | 7b7d93440 math rendering (#48551); 3b5996f98 Windows ansi16 to truecolor (#13016); 52e12f2b6 revert streaming markdown (#1981) | own markdown renderer, syntect | STAYS (our logic) with SHRINK from rtwk-rich streaming markdown (MarkdownStream, take_settled) and the oracle; syntax coverage is 10 languages vs syntect (gap 4); diff as a non-scrolling history item unverified |
| 19 | Product/backend/config/CI text in tui paths | ~135 of "other" | n/a | n/a | 3620b2caf SQLite corruption; 27c4c67b1 TUI wrong CWD (#23538); 5314f5509 standalone updates | not UI | STAYS (not a TUI bug class; stays whatever the framework) |
| 20 | Retained-tree/framework risks (introduced by rtwk) | 0 (no history) | n/a | n/a | architecture.md Risks 1-6; rtwk-for-codex §5 | rebuild costs in an arena tree, immature API (early status, facade cut), ADR-002 UI-thread controller unproven | NEW RISK. Kept by: Codex-shaped bench, pinned rtwk commit and generated API index per bump, the differential parity suite, allocation budgets |

## 3. Evidence notes

### Local history
- 3,612 non-merge commits on codex-rs/tui + tui2 paths (current branch history,
  starting 2025-04-24; tui2 had 164 commits in all refs, folded into the same
  count by path). Latest commit in range 2026-11-17.
- Strict fix set 508: subject matches fix|bug|regress|revert|panic|crash|hang|
  flicker|glitch|garbl|corrupt|broken|stale|duplicate|race|leak|deadlock|freez,
  excluding feat/docs prefixes (saved as strict.txt, cls.txt in the research dir).
  A looser filter gives 595; bugs fixed under non-fix subjects are missed.
- Primary class (ordered regex on the subject): other 169, state_ui 87, keys 38,
  compat 38, wrap_rect 35, render 24, race 24, paste 22, scrollback 18, link 14,
  sched_perf 11, unicode 10, suspend 7, resize 4, image 4, cursor 3. Several
  "compat" hits are Windows CI/test-fixture fixes (6368f506b, cb9fb562a), not TUI
  bugs; several "wrap_rect" are clippy/test fixes (7c3cc1db8, 0184dfa6a); I
  kept them in their class and counted that as the "noise" caveat in the table.
- Verdict mapping used for the shares: GONE = cursor 3 + suspend 7 + half of
  wrap_rect 17 + 30% of render 7 + 8 of link + 10 of keys (sequence repair)
  = ~53; SHRINKS = compat 38, unicode 10, resize 3, scrollback 18, paste 22,
  text-wrap 18, render 10, keys 10, image 4, link 6 = ~139; STAYS = state_ui
  87 + race 24 + sched 11 + other 169 + keys 18 + render 7 = ~316.
- Fixes cluster at the logic layer: of 508, 49 touch the terminal layer files,
  43 layout/text, 23 input/keys, 142 bottom pane, 244 controller. A fix often
  touches several; the ordering is robust.

### Workarounds in codex-rs/tui/src (read as a map of terminal reality)
- FIXME 5 lines / 1 file, XXX 1; "workaround"/"hack"/"terminal bug" 0 literal
  hits: Codex states quirks as comments, not as workaround markers.
- "Some terminals ..." comments (13 lines, 11 files): app/session_lifecycle.rs:591
  (flicker), app/resize_reflow.rs:502 (width settles after repaint),
  app/input.rs:561 and multi_agents.rs:140,166 (macOS Option+b/f), app/history_ui.rs:392
  and custom_terminal.rs:628,642 (Terminal.app and Warp need purge+clear paired),
  transcript_view/search.rs:199 (held Enter indistinguishable from press),
  tui.rs:269 (legacy Windows consoles), diff_render.rs:139 (Windows Terminal color),
  chat_composer.rs:276 (Windows pastes as rapid keys).
- Terminal names in code: tmux 290 lines / 31 files, kitty 207/35, wsl 201/23,
  iterm 102/23, ghostty 57/21, zellij 42/11, windows terminal/conpty 19/9,
  terminal.app 15/7; "repair" 53/22, "regression" 50/39 files. Regression-named
  test functions 38; 1,549 insta .snap files; vt100 suite in tests/suite:
  vt100_history.rs 164 loc (7 tests), vt100_live_commit.rs 43, resize_reflow.rs
  726 loc (4 tests).
- The terminal layer Codex owns: custom_terminal.rs 1,635, insert_history.rs
  1,318, tui.rs 1,756, wrapping.rs 2,096, paste_burst.rs 610, resize_reflow.rs
  775, key_hint.rs 493, terminal_palette.rs 558 loc (~9,200 loc) that rtwk's
  core/presenter/input/terminal stack replaces or shrinks.

### Public issues (gh, label TUI, 2,593 total; in:title count, example links)
- flicker 6 (#25004, #19535), scrollback 37 (#62007, #57302), resize 15 (#58516,
  #37987), wrap 18 (#57309, #36653), paste 119 (#59932, #60879), tmux 41 (#62005,
  #43137), zellij 14 (#57768, #45747), windows 182 (#44830, #59620), wsl 40 (#35413,
  #23485), unicode 7 (#37578, #56833), emoji 3 (#60588, #8586), cursor 35 (#21666,
  #31044), garbled 8 (#52003, #49589), duplicate 18 (#48129, #47538), freeze 26
  (#24668, #35896), hang 24 (#60294, #59559), lag 7 (#42421, #59672), cpu 9
  (#11077, #19958), crash 23 (#39128, #37506), panic 9 (#54979, #31714), stale 23
  (#30714, #61054), kitty 18 (#34092, #37454), shift 48 (#12882, #30989), ctrl 122
  (#55688, #35599), esc 35 (#14509, #48567), ghostty 17 (#59608, #55134),
  terminal.app 3 (#15416, #39265), hyperlink 8 (#43137, #36653), clipboard 34
  (#30305, #34160), image 63, suspend 10 (#29730, #47243), popup 12 (#21398,
  #45036), queue 52 (#14213, #35357), composer 57 (#51640, #42587), markdown 41
  (#43137, #36653), diff 40 (#62011, #62025), streaming 11 (#36577).
- Body-text counts (label TUI, all of issue text; noisy, upper bounds): scrollback
  214, resize 269, paste 401, tmux 332, windows 773, wsl 217, cursor 231, kitty 117,
  shift+enter 98, CJK 26, IME 36, hyperlink 37, clipboard 205, selection 361.
  Without the label filter: flicker 198, scrollback 247, wrapping 282, tmux 739,
  zellij 171, unicode 352, emoji 96, CJK 102, IME 175.
- Reading: by title, the issue mass is input/paste/keys (~330 with shift, ctrl,
  esc, paste), Windows/WSL (~220), composer/queue/history state (~160), then
  scrollback/resize/wrap/flicker/garbled/duplicate (~100) and terminal-compat
  (~90). Windows is the biggest single compat class in issues and rtwk's weakest
  evidence. Rendering artifacts are a minority in issues (~15 per cent) but
  scrollback/resize issues are mostly about history the terminal owns.
- Issue classes were not individually read; counts are title keyword counts and
  overlap. Public data only; no account details in queries.

### rtwk side (what removes a class)
- Presenter (design/presenter.md): packed cell buffer, damaged spans only,
  synchronized-update envelope (DEC 2026) unless the change is a single short
  row, autowrap off so no width error can wrap into the next row, cursor hidden
  while drawing, empty frame = 0 bytes, no allocation per frame.
- Inline mode (design/inline.md §1.12): a region that never writes rows above it,
  no absolute cursor addressing, DL/IL scrolling inside the block, commit for
  history, resize repaints from scratch.
- Unicode (design/unicode.md): own UAX#29 segmenter, Graphemes vs Legacy text mode
  from DECRQM 2027, ambiguous width measured by probe, uncertain-width
  containment (ECH, neighbour rewrite); bidi in core-bidi.
- Input (design/input.md): state machine with lone-ESC, sequence, paste timeouts;
  kitty protocol aware; parser fuzzed; mouse move coalescing.
- Job control (design/jobcontrol.md): bash and zsh in a pty, terminal modes at rest
  as before, defects found and fixed in platform/unix.rs.
- Responsiveness (design/responsiveness.md): input frames not held to frame cap,
  scroll flicker fixed with synchronized output.
- Tests that keep classes gone (TESTING.md): differential/oracle tests against twk
  (Dart reference commit 8135656...), property tests replaying presenter output
  through a terminal model, counting-allocator budgets, parser fuzzing, image
  goldens, job-control tests, framework golden of 300 scenes, mutation checks;
  Windows CI with 11 ConPTY scenarios. Missing: a Codex-shaped bench, a
  Rust test of uncertain widths against a disagreeing terminal, cut_agreement
  (both noted in the docs as pending).

### What this means for the plan
- rtwk removes the terminal-reality half of the TUI (about a fifth of the fixes)
  and part of the compat quirks. The majority of historical fixes are Codex's own
  state/controller logic and will stay; ADR-003 is where a second large win
  exists (stale/duplicate/wrong-view classes, 87+24 fixes, issues stale 23,
  composer 57, queue 52). rtwk imposes no state manager, so the separation of
  state from UI has to be done by the port, with headless reducer tests as the
  guard that keeps the class gone.
- Port the regression suite, not just the code: 38 regression-named tests, the
  vt100 suites and the insta snapshots are the parity oracle; add fixtures for
  tmux/zellij/Windows quirks that today are only comments.

## 4. Commands run
- git log --no-merges --format='%h|%ad|%s' --date=short -- codex-rs/tui codex-rs/tui2 > all.txt
- grep -iE '\b(fix|bug|regress|revert|panic|crash|hang|flicker|glitch|garbl|corrupt|broken|stale|duplicate|race|leak|deadlock|freez)' all.txt (strict.txt, 508)
- bash classify.sh < strict.txt (ordered regex classes, cls.txt); sample reads of "other"
- git log --no-merges --name-only --format='@%h' -- codex-rs/tui codex-rs/tui2 + awk file-group tally (fg.awk)
- grep -rniE --include='*.rs' for workaround|hack|FIXME|XXX|some terminals|repair|regression|tmux|zellij|kitty|wsl|iterm|ghostty|Rect::new|saturating_|word_wrap|unicode_width in codex-rs/tui/src; regression-named fns; find *.snap | wc -l
- gh api -X GET search/issues -f q='repo:openai/codex is:issue label:TUI in:title <kw>' (about 45 keywords, per_page=3), same without label and in body for comparison
- sed -n ranges over rtwk README.md, ARCHITECTURE.md, design/{inline,unicode,input,presenter,jobcontrol}.md, TESTING.md, terminal.md "What is left"; docs/rtwk-port/research/rtwk-for-codex.md §2, §5; architecture.md ADR-003, Risks
- Not done: no cargo builds; the 508 commits were classified by subject, diffs read only through file-touch tallies; issues were counted, not read; PERFORMANCE.md only referenced through rtwk-for-codex numbers.
