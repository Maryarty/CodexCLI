# Delta kinds

One entry per kind of difference between codex-tui and tui-rtwk, with the
person's decision. A delta of a decided kind needs no new decision; a NEW kind
goes to the person (process.md §A9), batched per wave. Started by
`port-skeleton`; the orchestrator keeps it from here.

| kind | what differs | decision |
|------|--------------|----------|
| K01 | A screen slot (or the chat placeholder) of the skeleton renders a stub, not the oracle's screen: the snapshot is an rtwk golden, not oracle-equal. Ends when the slice of that screen lands. | pending (proposed: accept, per slot, until its slice) |
| K02 | The composer sends a message typed while a turn runs to `apply`, which refuses it (R10); codex-tui queues it. Ends with S10's composer. | pending (proposed: accept until S10) |
