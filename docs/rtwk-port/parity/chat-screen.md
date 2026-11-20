# Chat screen (S10)

## Screen composition

The chat uses a retained `Column` with one virtualized `ListView` inside one `SelectionArea`, following rtwk ADR-025. Every thread has its own list and editing controllers. Rows use the transcript's stable item key. The chat shell subscribes to lifecycle and footer data, and the list subscribes to transcript updates. The composer and status stay outside the viewport. A scroll notification near the top requests the next older page while the app limits a thread to one in-flight history request. Stable keys let the list anchor the visible row when history is inserted above it.

The pinned rtwk gallery exercises `ListView`, `KeyHintsBar`, `Spinner`, `CustomPaint`, terminal titles and desktop notifications. Its Markdown page has a streamed answer demo. S10 uses the inline `MarkdownView` in each answer. Its render object lays out blocks at the width it receives. Its parser retains settled blocks across append updates. If rtwk converges on its composed renderer, that renderer needs an embeddable block body so it can use the transcript's viewport. Very large live tables use the same embedded rendering in the port and can have a higher layout cost.

## Oracle cases

| Oracle | Port behavior |
|---|---|
| `chatwidget/rendering.rs` | Transcript, activity, pending input, backend notices, composer and footer. Native framework layout, focus and scrolling. |
| `bottom_pane/footer.rs` | Shortcut overlay, Esc reminder, Tab queue hint, warning counter. Ctrl+C interrupts a live turn and quits when idle. An empty idle composer also accepts Ctrl+D. |
| `status_indicator_widget.rs`, `shimmer.rs` | Native spinner, elapsed label, Thinking and Working, context compaction, active plan step, and animation settings. The header highlight and elapsed timer are scoped to the indicator. |
| `chatwidget/status_surfaces.rs`, `bottom_pane/title_setup.rs` | Configured order, live thread metadata and token usage, account windows, current task progress, and the terminal title. The title animates through rtwk's title binding. |
| `empty_state_animation.rs` | Existing blossom geometry and lighting, with Flutter-style `LayoutBuilder`, `GestureDetector`, `AnimationController` and `CustomPaint`. At rest the blossom needs no animation ticks. |
| `chatwidget/notifications.rs` | Typed terminal effects for completed turns, approvals and questions. The terminal condition and configured notification types gate delivery. Duplicate and historical turn ends do not emit a second notification. |
| `backend_banners.rs` | Inline and dismissible account usage notices and supported browser CTAs. A dismissal belongs to the backend and one occurrence. A newer occurrence cannot be dismissed by the old widget. |

## Visual comparison

The fullscreen oracle snapshots `owned_transcript_composer_gap`, `fresh_thread_header` and `renders_with_working_header` are the references for the chat frame. Short history aligns with the composer. Prompts and the draft sit on terminal-derived surfaces with one row of padding above and below. Idle has no separate status row. The footer status can wrap, and active work retains its own indicator.

Both binaries ran the same mocked Markdown prompt in an 80×24 PTY. The port snapshots cover an empty chat and a short completed exchange. The seeded `streamed_answer` entry also shows the configured model and working directory.

The multi-turn scenario also runs in both binaries at 148×34. Between ordinary transcript cells, the separator uses the terminal background. Adjacent user prompts share a surface, and display ignores trailing blank prompt lines. A headless comparison at widths 56 and 148 checks the prompt surface and its right edge, the separator colors, and multiline alignment after a Cyrillic Markdown table.

## Restored chat comparison

Both binaries opened the same saved mock conversation in the external-writer state at 148×30. The comparison now includes the restored user prompt, assistant messages, completion time and the lock notice. In the read-only view, short history starts at the top and the notice occupies the composer slot with its own key hints. The retry, fork and command center actions use the existing app intents. The read-only transcript opens the same provider-backed list on a separate route.

The session header is the first item in the history list. It uses the current session directory, shortens home paths and keeps both ends of a truncated path. Short live and restored histories with a header start at the top. The bottom sheet stays anchored to the composer slot while the header scrolls with history.

The mock's saved rollout showed no retained command previews in either binary. A separate protocol fixture supplies command and MCP items in `thread/turns/list` alongside message data. It verifies the preview rows, completion timing and retry through the actual keyboard path. The core carries the same activity types for live items and saved turns.

## Transcript comparison matrix

Use the oracle's render branches to choose fixtures. Each fixture needs an observable row or interaction. A mock response that reaches its final answer can still omit a tool row, so the fixture must verify that the row under comparison appeared.

| Oracle source | Variations to exercise | Coverage |
|---|---|---|
| `history_cell/messages.rs` | Plain text and explicit newlines, typed Markdown, image-only, remote/local attachments, inline placeholders, adjacent prompts, narrow wrapping. Spoken and async-reply presentations have their own branches. | Four image snapshots compare the actual prompt region to the oracle. Headless tests check prompt surfaces, padding and wrapping. Spoken and async-reply prompts remain in S11. |
| `exec_cell/model.rs`, `exec_cell/compact.rs`, `exec_cell/render.rs` | Running, success, nonzero exit, user shell, exploration of read/list/search, absent and multiline output, width-clipped lines, disclosure, background and stdin interaction. | Oracle snapshots cover mixed actions and deduplicated reads within and across calls. The exploration scenario runs one search and two reads in both binaries. Protocol tests cover live and restored output, omission by line count, and expand/collapse. Background execution and stdin interaction remain in S11. |
| `history_cell/mcp_preview.rs`, `history_cell/mcp.rs`, `history_cell/dynamic.rs` | Running, success, error, text, structured result, image/audio, multiple output blocks, long results, tool-specific labels and details. | Real MCP echo and image calls in the paired PTY capture. History mock covers text, name and disclosure. Tool-specific labels and exact ordering of mixed media remain in S11. |
| `history_cell/messages.rs`, `markdown_render/`, `streaming/` | Live tail and finalized answer, headings, code, links, quotes, tables, CJK and emoji, long rows, resize. | The embedded rtwk Markdown theme draws a heavy header rule and light body rules. It preserves table alignment and renders records when the columns cannot fit. Markdown PTY frames pair live and restored tables. |
| `history_cell/separators.rs`, `app/owned_transcript.rs`, `chatwidget/rendering.rs` | Fresh and restored history, editable or locked, short or longer than a viewport, gaps and selection, completion timing, retry and expansion. | Paired PTY frames for live and restored chat, including a Ctrl+T checkpoint. The locked-writer fixture verifies compact and detailed rendering in the same viewport, timing and retry. Raw transcript and selection parity have their own shards. |
| `history_cell/{patches,plans,notices,session,search,hook_cell,computer_activity}.rs` | Diff and patch failure, plan stages, warning and error cells, session header, search and hooks, image generation and computer activity. | Follow the registered oracle `cells_*`, `chat_live_*` and `transcript_view_*` shards for S11. |

Run from the repository root to record each app at startup, during work when captured, after each scripted turn and after reopening the same oracle-produced session. Use a fresh capture directory for each size and palette.

```sh
python3 -I tools/rtwk-port/mock-model/check.py \
  --only exploration,markdown \
  --size 148x34 --palette dark --capture-resume --capture-details --capture-dir /tmp/chat-parity-148
cargo run --quiet --manifest-path codex-rs/tui-rtwk/Cargo.toml -p codex-tui-rtwk \
  --example compare-transcript -- 148x34 \
  /tmp/chat-parity-148/markdown/oracle/resumed.ansi \
  /tmp/chat-parity-148/markdown/rtwk/resumed.ansi \
  --frames /tmp/chat-parity-148/markdown/resumed
cmp /tmp/chat-parity-148/markdown/resumed/{oracle,rtwk}.frame
```

Repeat at 56×34 for wrapping and at 80×30 with the light palette for surface contrast. Pair the same named checkpoint from each binary. Compare `ready`, every captured `turn-*-working` and `turn-*`, `detailed`, `resumed`, and `resumed-detailed` when that state exists in both recordings. Record a checkpoint available in only one binary separately so the missing state is visible.

The comparator replays both ANSI streams through the same rtwk terminal emulator. The frame files contain the glyph and wide-glyph continuation, foreground, background, underline color, attribute bits and hyperlink of every cell, plus the visible cursor's position and shape. `--check` exits nonzero when the frame bytes differ. The report gives a differing cell count and the first changed column in each affected row. Every cell is compared, including styled spaces. Terminal session lifecycle notifications have no grid effect and are skipped. Unsupported terminal operations and pixel image overlays are reported as errors.

Use the resumed checkpoints when checking transcript geometry, since they reopen one saved session with the same paths, timestamps and turns in both applications. Test live behavior in the live checkpoints, whose timings may vary with process startup. The shell comparison included a successful four-line command and an exit-3 command in both binaries, with collapsed previews before and after reopening.

### Lists in long answers

Assistant lists keep adjacent items together while text arrives. In a finished or restored answer, siblings get a blank row between them if any item wraps or has more than one line. The same Markdown widget recomputes this from its current width. Short lists stay compact. The transcript uses the original's built-in heading, bold text, unordered marker and link roles for dark and light terminals.

The `markdown-list-layout` mock adds a Cyrillic section with long and short lists. Use `COLORTERM=truecolor` and unset `NO_COLOR` when capturing it. Compare the restored sections at 56×45 and 148×45. The heading and list regions match in position, glyphs and visible styles at both widths and in the light palette at 56×45. Three separator spaces inside the bold list introductions retain a different foreground or attribute. The original also displays the destination of the prose link before the lists. The rtwk tests check that collapsing provisional gaps preserves link hits and text selection.

## Remaining differences

- The footer and title use available thread settings, account limits and host data. Live PR and branch-change summaries, enterprise thread billing and workspace headline still need data sources.
- Rich transcript cell types, composer steer/rejection states and sidebar, voice, and UI tree integration belong to their tracked slices. The session queue still uses its existing submit behavior for Enter.
- Owner notification, reset-credit confirmation and the persisted Luna Reserve automatic switch and recovery are pending. Browser links are the supported banner actions in this slice.
- Terminal notification delivery uses the framework's negotiated method. Explicit notification-method overrides, title action-required flashing and plan-mode-specific notifications still differ from the oracle.
- The oracle pins a one-row preview of the request preceding the first visible answer when that request has scrolled out of view. Use the list viewport’s laid-out item positions for this behavior, including reverse lists and variable-height children. The header belongs to the viewport and clicking it returns to the original request.
- Embedded Markdown uses source markers for headings and quotes and compact spacing for adjacent tight lists. The oracle's narrow-table record heuristics differ from the layout's minimum-width fallback. The return-to-latest control and the inline transcript overlay remain tracked by their own slices.
