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

The terminal capture placed the first prompt on row 6 in the oracle and row 2 in the port, with the oracle's startup header accounting for the extra rows. The completion summary appeared on rows 15 and 11 and had the same time and text. The lock title and shortcut footer appeared on rows 27 and 30 in both captures.

The mock's saved rollout showed no retained command previews in either binary. A separate protocol fixture supplies command and MCP items in `thread/turns/list` alongside message data. It verifies the preview rows, completion timing and retry through the actual keyboard path. Tool details use rtwk's native expansion widget. The core carries the same activity types for live items and saved turns.

## Remaining differences

- The footer and title use available thread settings, account limits and host data. Live PR and branch-change summaries, enterprise thread billing and workspace headline still need data sources.
- Rich transcript cell types, composer steer/rejection states and sidebar, voice, and UI tree integration belong to their tracked slices. The session queue still uses its existing submit behavior for Enter.
- Owner notification, reset-credit confirmation and the persisted Luna Reserve automatic switch and recovery are pending. Browser links are the supported banner actions in this slice.
- Terminal notification delivery uses the framework's negotiated method. Explicit notification-method overrides, title action-required flashing and plan-mode-specific notifications still differ from the oracle.
- Rich Markdown table and quote glyphs follow rtwk. Tool previews use native rtwk expansion glyphs and styles. The oracle also has richer compact tool grouping and a return-to-latest control while reading older history. The port follows the bottom when appropriate and preserves scroll position during streaming.
