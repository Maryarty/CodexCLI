//! The debug-menu entries of the agents page.

use super::*;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "three tasks of one project; the first one needs you",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "attention",
            description: "a task whose child waits for an approval and a response",
            script: include_str!("fixtures/attention.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "navigation",
            description: "a ready, an errored and an unloaded task under the chat",
            script: include_str!("fixtures/navigation.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "search",
            description: "one task, for the search line",
            script: include_str!("fixtures/search.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "clip",
            description: "one task, for long text in the search and rename lines",
            script: include_str!("fixtures/clip.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging5",
            description: "five tasks of one project",
            script: include_str!("fixtures/paging5.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging30",
            description: "thirty tasks of two projects",
            script: include_str!("fixtures/paging30.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "confirm",
            description: "one task, for the archive and delete confirmations",
            script: include_str!("fixtures/confirm.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "progress",
            description: "three inactive tasks, for the archive and delete in progress",
            script: include_str!("fixtures/progress.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "columns",
            description: "six tasks of every status and age, pins, an unread one",
            script: include_str!("fixtures/columns.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "colors",
            description: "two unread tasks, ids 42 and 43, thread colours on",
            script: include_str!("fixtures/colors.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "colors_off",
            description: "two unread tasks, ids 42 and 43, thread colours off",
            script: include_str!("fixtures/colors_off.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "empty_narrow",
            description: "no tasks at 48 cells, with nowhere to go back to",
            script: include_str!("fixtures/empty_narrow.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "busy",
            description: "no tasks, a worktree is being created",
            script: include_str!("fixtures/busy.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "older_server",
            description: "no tasks, and the server is older than the client",
            script: include_str!("fixtures/older_server.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: false,
                    open: false,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "paging30_offline",
            description: "thirty tasks while the connection is lost",
            script: include_str!("fixtures/paging30_offline.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    can_return: true,
                    open: false,
                }
                .into()
            }),
        },
    ]
}
