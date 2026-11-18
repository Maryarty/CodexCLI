# Delta kinds

One entry per kind of difference between codex-tui and tui-rtwk, with the
person's decision. A delta of a decided kind needs no new decision; a NEW kind
goes to the person (process.md §A9), batched per wave. Started by
`port-skeleton`; the orchestrator keeps it from here.

| kind | what differs | decision |
|------|--------------|----------|
| K01 | A screen slot (or the chat placeholder) of the skeleton renders a stub, not the oracle's screen: the snapshot is an rtwk golden, not oracle-equal. Ends when the slice of that screen lands. | pending (proposed: accept, per slot, until its slice) |
| K02 | (resolved in port-skeleton's fix round) `apply` queues a message typed while a turn runs and sends it at the turn's end, as codex-tui does; what remains (the queue's look, edit, steer) is K01 for S10. | resolved: no delta of this kind |
