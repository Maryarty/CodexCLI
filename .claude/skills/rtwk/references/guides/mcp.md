<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Letting an agent drive your app: `rtwk mcp`

<!-- not rtwk: Cargo -->

`rtwk mcp` is an [MCP](https://modelcontextprotocol.io) server on stdio. A
coding agent that registers it can start your app in a pseudo-terminal, look
at its screen, press keys, click a widget by what it is rather than where it
is, move the app's time forward and read its log, which is the loop a person
runs by hand with `cargo run`, with answers an agent can read. It is the
agent's side of [Testing](testing.md): the same driven app as `rtwk tape` and
`rtwk-driver`, offered as tools.

```sh
rtwk mcp --help
```

## Registering it

The server takes no argument and no configuration: the client starts it, talks
to it over its stdin and stdout, and ends it by closing them. Nothing but
protocol is written to stdout, and the app it runs ends with it.

```sh
# Claude Code
claude mcp add rtwk -- rtwk mcp
```

```toml
# Codex: ~/.codex/config.toml
[mcp_servers.rtwk]
command = "rtwk"
args = ["mcp"]
```

Any other client takes the same two words, as JSON where it wants that:
`{"mcpServers": {"rtwk": {"command": "rtwk", "args": ["mcp"]}}}`. It speaks MCP
2025-06-18, 2025-03-26 and 2024-11-05 over one JSON message a line, answering
a version it does not know with its newest.

## What the app must be

- **A debug build, started by its binary.** `cargo build` first, then
  `start_app` with `target/debug/<name>`. A `cargo run` would put Cargo between
  the server and the app, and a release build ignores the driving
  (`RTWK_DRIVEN`) altogether.
- **Unix.** The server needs a pseudo-terminal; on Windows `rtwk mcp` says so
  and exits with 1.

The app is started in the nearest directory above its binary that has a
`Cargo.toml`, in a terminal of 80x24 unless told otherwise (`cwd`, `env`,
`width` and `height` of `start_app` change the directory, add variables and
set the size).

## Time is the agent's

The app runs on a clock only the server moves. A timer, a debounce or an
animation does not fire while the agent thinks; it fires when `wait_for` or
`elapse` moves the clock to it, and the app's frames and the answer come out
in the same step. So an agent never sleeps and a test of a timed behaviour has
no flake. `press`, `type` and `tap` move the clock to the frame the input
asked for.

Every action answers with the screen after it, a line `--`, and a line that
says where the app stands:

```text
frame 7, input read 3 events, quiet (no timer, no frame asked for; it will not change by itself)
frame 4, input read 0 events, not quiet (next timer in 0.500 s)
```

The second line is what tells an agent whether waiting can help: a quiet app
changes only when the agent does something.

## The tools

| Tool | Does |
| --- | --- |
| `start_app` | Starts the app (`program`, and `arguments`, `cwd`, `env`, `width`, `height`) and answers its first screen. One app at a time. |
| `screen` | The screen as of the last frame, and where the app stands. |
| `tree` | The widget tree as indented text, each node's type, id, key, size and properties (`depth` cuts it; the nodes cut off are counted). |
| `find` | The nodes a finder matches: type, id, key, text and the cells each occupies. |
| `press` | Presses keys named as bindings are (`"ctrl+s"`, `"g g"`, `"down down enter"`); pressing the key that quits ends the app and the answer says with which code. |
| `type` | Types text, one key event for each character. |
| `tap` | Clicks the one node a finder matches, after checking that a click there reaches it; it fails saying what covers the node rather than clicking nothing. |
| `wait_for` | Moves the app's time, timer to timer, until the screen has `text` (or no longer has `absent`), within `budget_ms` of the app's time (10 000). |
| `elapse` | Moves the app's time on by `ms` and answers the screen then. |
| `resize` | Sets the terminal's size; the app is told, redraws, and the answer is the new screen. |
| `logs` | The records the app logged since the last call, one a line (the last 50 are kept: call often enough). |
| `stop_app` | Ends the app and reports what it left behind: the terminal not back in cooked mode, the alternate screen not left, the cursor hidden. |

A tool's failure is a result with `isError`, not a protocol error, and says
why. The ones an agent should read rather than retry:

- `wait_for` on a quiet app fails at once with `never`: nothing will change
  the screen, so more waiting cannot help. It names `io_ms`, which waits in
  real time for what the clock cannot see (a socket, a file, a process).
- `wait_for` that moved the clock by its budget says how far it moved and
  shows the screen then (an animation that never ends, say).
- `tap` names the widget that covers the one it was given.
- A finder the app does not know is answered with the finders it does.

### Finders

A finder is JSON: `{"text": "Save"}`, `{"textContaining": "ave"}`,
`{"type": "Button"}`, `{"key": {"value": "ok"}}`, `{"node": 42}` (an id from
`tree` or `find`), `{"descendant": F, "of": F}`, `{"ancestor": F, "of": F}`,
and `{"first": F}`, `{"last": F}` and `{"at": 2, "of": F}` to pick among
several. A finder that matches none or several fails where one is meant. There
is no `textMatches` (the app's inspector has no regular expressions), and an
object given as a JSON string is taken too, as agents often send one.

## A session

```text
start_app   {"program": "target/debug/counter", "width": 40, "height": 6}
press       {"keys": "+ +"}
tap         {"finder": {"text": "Reset"}}
wait_for    {"text": "Saved", "budget_ms": 3000}
logs        {}
stop_app    {}
```

## Where it differs from `twk mcp`

- The tools are twk's, less `hot_reload` (rtwk has none yet) and plus
  `resize`, and `start_app` takes `cwd` and `env`.
- The status line counts input events, not bytes: rtwk's terminal reads
  events, not a byte stream.
- `tree` is the inspector's: it names only `Text`, `Column`, `Row`, `Padding`
  and the framework's root, and every other widget `Widget`. A `type` finder
  knows the real types (`{"type": "Button"}`), and a node's id from either is
  what `{"node": id}` takes.
- A tool that panics fails its call and the server serves the next one.
