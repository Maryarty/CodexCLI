# Delta kinds

One entry per kind of difference between codex-tui and tui-rtwk, with the
person's decision. A delta of a decided kind needs no new decision; a NEW kind
goes to the person (process.md §A9), batched per wave. Started by
`port-skeleton`; the orchestrator keeps it from here.

| kind | what differs | decision |
|------|--------------|----------|
| K01 | A screen slot (or the chat placeholder) of the skeleton renders a stub, not the oracle's screen: the snapshot is an rtwk golden, not oracle-equal. Ends when the slice of that screen lands. | pending (proposed: accept, per slot, until its slice) |
| K02 | (resolved in port-skeleton's fix round) `apply` queues a message typed while a turn runs and sends it at the turn's end, as codex-tui does; what remains (the queue's look, edit, steer) is K01 for S10. | resolved: no delta of this kind |
| R01 | (rejected, the person 2026-11-18) Any look or behaviour taken from the nearest ready rtwk widget instead of the oracle's: dialogs in a framed `Dialog` instead of the frameless bottom pane with `›` (N01) and its `max_width` wrap tuning (N02); pager without the `~` filler (D1); selected row without its fill (D2); composer popups in `Autocomplete`'s look, Tab running the command (N1, review G3); flat `@` picker (N2); unstyled history-search footer (N5); a command's notice as a line instead of the red `■` error cell (N7); no Esc hint (N8). | rejected: each is fixed to the oracle (research `parity-routes`, fix slices); no delta of this kind is proposed again |
