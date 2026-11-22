# host-services: the host and service Needs, collected, and what answers them

Task `host-services` (absorbs `clipboard-service`). Sources: every `Needs` / `Left` section of
`docs/rtwk-port/parity/*.md` and §5 of `docs/rtwk-port/research/briefs/*.md` (read 2026-11-22 on
`claude/tui-rtwk` 1759cf2bc). Core (`codex-tui-core`) is not touched here: `core-wiring` owns
`core/src/app.rs`, `threads.rs`, `protocol/` and defines each `Command`/`Ev`; this task gives every
service a typed entry point and names the `Command`/`Ev` it expects.

Where the code lives, and why (R1, R11, R12, clippy.toml): every call that touches the machine
(fs, process, thread, clock, env) is allowed only in `src/services/` of the library crate; `host/`
may not spawn, block or `#[allow]`. So the machine parts of each service are in `src/services/*`,
tested there with a tempdir or a scripted fake; backend crates the library does not link yet are
added to the library `Cargo.toml` (one commit, named in the handover). `host/` keeps what needs
the app-server client or the process's flags (the dispatcher arm, the connection fact's source).

## 1. The table

| # | Need | Source (pack / brief) | Mine? | Answer |
|---|---|---|---|---|
| 1 | Clipboard copy with a result: `Command::Copy` / `Ev::Copied(CopyStatus)`; routes native (arboard, HTML flavour), WSL PowerShell, tmux `load-buffer -w`, OSC 52, X11 PRIMARY | transcript.md "fourth handoff" (4), tm `clipboard-service`, briefs transcript §3, chat-commands §3 (/export Clipboard), status-card (copy), composer_4 | yes | DONE `src/services/clipboard.rs` + `clipboard/{machine,tmux,wsl}.rs`: `Routes` trait, `route()` (oracle `copy_to_clipboard_with`, same error texts), `Clipboard` worker (5 s setup budget, `Busy`), `Machine` (the real routes) |
| 2 | Clipboard text / image read (Ctrl+V image, right-click paste, PRIMARY read), path paste | composer.md "Also there: ClipboardImage", brief composer §3 (G11), §6 step 9 | yes (last) | DONE `src/services/clipboard/read.rs` + `read/{text,picture,decode,macos}.rs`: `TextReader::read_within` (native `arboard` text with the oracle's deadline/size checks, WSL framed PowerShell reader thread, SSH refusal), `paste_image` / `paste_image_to_temp_png` (file list first, macOS pasteboard PNG/JPEG in item order, `arboard` image, WSL PowerShell fallback), `PasteImageError` (the oracle's texts), `pasted_image_format`. The path-paste half is the composer's `services/images.rs` |
| 3 | Message history: `history_metadata` on thread open, `append_entry` on submit, `lookup`, `lookup_batch` -> `popovers::Ev::{HistoryMetadata, HistoryEntry, HistoryBatch, HistoryBatchFailed}` | composer.md "Needs", brief composer §3 | yes | DONE `src/services/history.rs` (`HistoryFile` seam, `metadata`/`append`/`lookup` run as the oracle does) + `history/file.rs` (`MessageHistory` over `codex-message-history`, keeps the batch cursor's byte anchor) |
| 4 | Mention usage file (`mention_usage.json`): load window -> `Ev::MentionUsage`, write each `Ask::RecordMention` | composer.md step 8 Needs, core `mentions/usage.rs` | yes | DONE `src/services/history/usage.rs` (`MentionFile::{load, record}`; file lock 100 ms, 1 MiB cap, atomic write, window 50) |
| 5 | File search: app-server `fuzzyFileSearch/session*` default (`AppServerSearch`, core: requests only) and the in-process `codex-file-search` second implementation; the person's choice is open | composer.md "File-search backend choice", brief composer D1 | yes | DONE the in-process one: `src/services/file_search.rs` (`InProcessSearch::{query, stop, ask, set_search_dir}`, oracle `FileSearchManager`, sink = the app's ingress). The default needs only core-wiring's routing (below) |
| 6 | `/diff`: `WorkspaceExec` over app-server `command/exec` (`OneOffCommandExec`), `Command::GitDiff` / `Ev::GitDiff` | transcript.md Needs (3), brief transcript §3 | yes (half) | DONE `src/services/exec.rs` (`AppServerExec<F>: WorkspaceExec` over the same `send` closure as `backend::rpc::answer`); `git_diff` is in `shell.rs` (chunk-transcript-2) |
| 7 | `/export` file write ("Saved conversation to {path}") | brief chat-commands §3, step 8 | yes | DONE `src/services/transcript_file.rs` (`write_transcript`, `export`; oracle error texts). chunk-commands' area names `host/src/export.rs`: call this, do not write a second writer |
| 8 | Connection fact `workspace.remote` (Embedded / local daemon / Remote / Cloud) | brief chat-commands §3 (/cd refusal), startup `Target::remote_workspace` | yes (pure fn) | DONE `src/services/remote.rs` (`Connection::uses_remote_workspace`: Remote or Cloud). The field in `Facts` and its source is `host/src/facts.rs` (chunk-status-card's): one line in the handover |
| 9 | `Command::OpenUrl{url}` (error "Failed to open browser for {url}: {err}"), `/app` desktop open (`codex://threads/{id}`) | chat-live.md C5/C6 Needs, brief chat-commands §3 (/app) | yes | DONE `src/services/open_url.rs` (`open_url` over the `Browser` seam, `open_desktop_thread`: macOS `open`, Windows PowerShell, else the oracle's refusal) |
| 10 | Fake-backend arm per `Command` | task text, rules-wave2 | no | `src/services/backend/fake.rs` is chunk-commands'; the arms are named in §2 |
| 11 | Analytics (`host/src/analytics.rs`, `codex-backend-client`) | usage.md, brief usage §5 | no | chunk-usage |
| 12 | `facts::settings` (theme, experimental features, hooks/list, skills config, memories) | settings.md "Needs outside the area" | no | startup-rest / settings owner; persistence `Command`s are core-wiring's |
| 13 | git summary probe (`branch_summary.rs`: `git`, `gh pr view`), workspace headline, thread usage polling | brief chat-status §3, §5 | no | chunk-status / keymap porter (`facts.rs`); would use #6's executor |
| 14 | `userVerification/verify`, `thread/approveGuardianDeniedAction` | approvals.md step 6/7 | no | approvals owner + core-wiring (requests) |
| 15 | `Ev::LinkDisplay` from terminal detection, `tui.rendering` preferences, syntax theme, version check, remote server version, clock and 12/24-hour | cells.md Needs, brief cells §3 | no | chunk-cells / host facts owner |
| 16 | External editor (Ctrl+G, `external_editor.rs`) | brief composer D3 (G67 stays pending) | no | the person's decision; not started |
| 17 | `model/list`, `permissionProfile/list`, `collaborationMode/list`, plugin and app RPCs, `thread/search` task mentions, `skills/list`, `app/list`, `plugin/list` | popups.md, brief composer §3 | no | protocol requests: core-wiring |
| 18 | `thread/backgroundTerminals/clean`, `thread/goal/*`, `review/start`, `thread/archive`, `thread/delete` | brief chat-commands §3 | no | `ThreadAsk` in `threads.rs`: core-wiring |
| 19 | `SuggestThreadTitle` (ephemeral thread + turn with schema) | brief chat-commands §3 | no | service named by chunk-commands |

## 2. The expected `Command` / `Ev` (core-wiring reads this)

| Service | Command (expected) | Ev (expected) | Typed entry point |
|---|---|---|---|
| clipboard | `Command::Copy { ticket, text: String, format: CopyFormat, dest: CopyDestination }` | `Ev::Copied { ticket, result: Result<CopyStatus, String> }`, `CopyStatus::{Confirmed, Unconfirmed, Busy}` (`Pending` is the model's own state: a request out, no answer yet) | `Clipboard::copy(text, Format, Destination) -> Done { result, terminal }`; `Done::terminal` is the OSC 52 text the terminal's owner writes (rtwk has no `AppCx` clipboard write, gap G-clip below) |
| clipboard read | `Command::ReadClipboardText { ticket }` (Ctrl+Shift+V / right-click paste) and `Command::PasteClipboardImage { ticket }` (Ctrl+V) | `Ev::ClipboardText { ticket, Result<String, String> }`; `Ev::ClipboardImage { ticket, Result<(PathBuf, PastedImageInfo), PasteImageError> }` (the composer attaches `[Image #N]` from the path, label `EncodedImageFormat::label`) | `TextReader::new(&environment()).read_within(timeout)`; `paste_image()` |
| history | `Command::History(popovers::Ask)` (`Append`, `Lookup`); metadata when a thread opens | `popovers::Ev::{HistoryMetadata, HistoryEntry, HistoryBatch, HistoryBatchFailed}` | `history::{metadata, append, lookup}` over `Arc<dyn HistoryFile>` (`MessageHistory::new(codex_home, &History)`) |
| mention usage | `Command::History(Ask::RecordMention)` and a load at start | `popovers::Ev::MentionUsage` | `MentionFile::{load, record}` |
| file search | `Ask::FileSearch(FileSearchAsk)` drained from `popovers::Model::take_asks`: routed either to `AppServerSearch::requests` (default, answered by the `sessionUpdated` notification through `matches_of`) or to `InProcessSearch::ask` | `popovers::Ev::FileMatches` (both) | `InProcessSearch::{ask, query, stop}` |
| diff | `Command::GitDiff { ticket, cwd }` | `Ev::GitDiff { ticket, Result<diff::Report, diff::Failure> }` | `AppServerExec::new(send, ids)` + `shell::git_diff` |
| export | `Command::ExportTranscript { ticket, cwd, path, markdown }` (markdown built by core from the loaded turns; `cwd` = launch dir when `uses_remote_workspace`, else the session's) | `Ev::Exported { ticket, Result<PathBuf, String> }` | `transcript_file::export` |
| url | `Command::OpenUrl { url }` (exists, answered by none) | `Ev::Failed { text }` or a note through the thread, with `OpenFailure{url, error}` | `open_url::open_url(browser, url)` |
| desktop | `Command::OpenDesktop { thread }` | note or error, words by the UI | `open_url::open_desktop_thread` |
| connection | a `Facts` field `remote: bool` | `AppEv::Facts` | `remote::Connection::uses_remote_workspace` |

Fake-backend arms (chunk-commands' `fake.rs`): `Copy` answers `Copied{Confirmed}` (and
`Copied{Err}` for a scripted failure); `GitDiff` answers a scripted `Report`; `ExportTranscript`
answers the path under the fake's directory; `History(Ask)` answers from a scripted in-memory
`HistoryFile` (the test double of `history/tests.rs` is the model); `OpenUrl` records the url.

## 3. Decisions of this task

- Machine code in `src/services/*`, not `host/` (R1/R11/R12). The seams are traits (`Routes`,
  `HistoryFile`, `WorkspaceExec`, `Browser`) so tests run without a disk or a network.
- The terminal route of the clipboard (OSC 52) is rtwk's sequence (`rtwk_core::terminal::
  clipboard_sequence`, wrapped for tmux and GNU screen), the oracle's 100 000-byte raw cap and its
  error texts are kept. The sequence cannot be written from a service: it returns in `Done::terminal`
  to the terminal's owner. Gap: rtwk has `editing::copy_to_clipboard(cx: &Cx, text)` for a widget's
  own selection but no `AppCx` counterpart (Flutter: `Clipboard.setData` from anywhere).
- The oracle's copy worker (`clipboard_copy/worker.rs`, FrameRequester polling, selection
  publications) is replaced by one worker thread per `Clipboard` with the oracle's 5 s setup
  budget and the "delivery claim cannot be abandoned" rule; the UI parts of its tests (overlay
  exit, frame scheduling, PRIMARY publication ordering) are n/a here.
- Markdown to HTML (`clipboard_html.rs`, `pulldown-cmark`) is not carried: `Format::Html(html)`
  takes the HTML from the owner of the copy source (`markdown_copy`, transcript).
- `arboard` 3.7 (library) vs the oracle's pinned 3.6.1: same API, no behaviour used here differs.
- The history batch cursor: core asks by `end_offset` only; `MessageHistory` keeps the cursor the
  last batch returned and uses it when the next ask is its offset, so the backend's byte-anchored
  continuation is kept (and falls back to the offset scan when the file changed).
- File search: core's `FileSearchBackend` trait only builds app-server requests, so the in-process
  implementation is not "the same trait" but the same `Ask`/`Ev` pair; the one that routes the ask
  (core-wiring) picks. Default app-server; `InProcessSearch` needs a local backend and a sink.

## 4. Done / left

Done (library `src/services/`, each with its own unit tests; `acargo nextest -E 'test(services::)'`
100 tests): clipboard (34), clipboard read (10: the oracle's text, decode and macOS pasteboard tests carried, plus
PasteImageError texts, `pasted_image_format`, WSL path map on Linux), history seam (5) + `MessageHistory` over the backend crate (7, tempdir),
mention usage (7), in-process file search (5, real `codex-file-search` sessions over a tempdir),
`AppServerExec` (5, scripted `send`), `/export` file writer (6), `open_url` (3 + 1 off-macOS), connection
fact (1). Gates at the end (a51fc68d5, base 1759cf2bc = the tip of claude/tui-rtwk): fmt, `clippy --workspace
--all-targets -D warnings` rc 0, `shards.py --check` ok, `nextest --workspace --no-fail-fast` 2173 of 2175 passed.
The two reds are pty scenarios of `codex-rtwk::session_sse` (`exec-approval`: "mock served 0 of 2 model
request(s) in 60s", `interrupted`: "the model request did not start before Escape"), stuck at `starting…` on a
machine at load average ~180; the same stall hits `check.py --only markdown` with another worktree's binary
(an oracle run of it passes), and nothing of this branch is reachable from the host binary. Noted, not chased.

Left, and why:

- The dispatcher arms in `host/src/in_process.rs::work` (chunk-status-card's file): once core-wiring
  adds the Commands. The `OpenUrl` arm exists in core and has no arm in the host (`answer` returns
  none): `Command::OpenUrl { url } => Box::pin(async move { open_url(browser, url).await.err().map(failed) })`
  with the `Browser` of `Startup` (`webbrowser::open`).
- Host construction: `MessageHistory::new(codex_home, &config.history)` and `MentionFile::new(codex_home,
  persist)` once per app; `InProcessSearch::new(cwd, sink)` when the person picks it; `AppServerExec::new(
  send, ids)` where `git_diff` runs; `Clipboard::new(Machine::default())` once per app, with `Done::terminal`
  written by the terminal's owner.
- rtwk gap (for the person): no `AppCx::copy_to_clipboard` (Flutter `Clipboard.setData(ClipboardData)` from
  anywhere); `rtwk::editing::copy_to_clipboard(cx: &Cx<S>, text)` needs a widget `Cx`. The OSC 52 text of a
  `Command::Copy` therefore travels back as `Done::terminal`. Not entered in rtwk-gaps.md (not my file).
- rtwk gap, the read side (same family): rtwk has no clipboard READ (Flutter `Clipboard.getData`): a
  bracketed paste arrives as an event, but Ctrl+V of an image, the right-click paste and the text paste of a
  terminal that sends no paste event need the system clipboard. `read::TextReader` and `read::paste_image`
  are the app's own, over `arboard`, as the oracle's.
- Not compiled here: the Linux-only code of the read side (the WSL PowerShell text reader thread, the
  PowerShell image fallback, `convert_windows_path_to_wsl`) and the Windows paths: this machine builds macOS
  only (the Linux target's std is not installed), so they are the oracle's code carried over and read, not
  built. The macOS pasteboard code and every test above ran.
- Path paste: the composer's `services/images.rs` `normalize_pasted_path` has no WSL mapping of `C:\...`
  to `/mnt/c/...` (oracle `normalize_windows_path`, `is_probably_wsl`, test `normalize_windows_path_in_wsl`);
  `clipboard::is_wsl_session()` is the probe it needs. Not mine (the composer's file): for its holder.
- Run it: services only, no page. `acargo build -q -p codex-rtwk --bin codex-rtwk` is green; no scenario of
  `tools/rtwk-port/mock-model` exercises a service yet (no `Command` reaches it).
