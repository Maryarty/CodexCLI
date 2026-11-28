# Visual deltas: what is small and what is not

The person's decision (2026-11-28): a small visual difference does not block a feature. The
port is ready when every function of the oracle works, not when every snapshot matches. A test
whose screen differs from the oracle's snapshot only by a small visual delta is a `delta`, not
`pending` (the function is not missing) and not a `pass` (the screen is not the oracle's).

## The marker

```rust
kit::delta(Owner::ChatLive, "list indent: ours 2 columns, the oracle's 4; the rows say the same");
// ... the whole test, as for a pass
```

- Put it first. Write the whole test after it: every functional check runs and must pass.
- `what` is at least three words, says which colour, indent, glyph, spacing or wrap differs
  (`tests/rules/oracle_markers.rs` checks that, and that a test holds one verdict).
- `kit::delta` softens only the kit's comparisons of the screen with the oracle's rows or cell
  styles: `assert_snap*`, `assert_rows_are`, the help and layout comparisons (all go through
  `verdict::screen` / `verdict::screen_styles`). The cursor, `assert_contains` and every other text,
  link, style-presence, count or state check stay as strict as in a pass.
- A comparison that differs prints `DELTA[slice]: what` and the diff; `summary.py` counts the test in
  its own column `delta` (the last of the table; `TOTAL` keeps `pass n/a pending todo red` in
  positions 2..6, `stale total` after, `delta` last), `summary.py --list delta` lists them.
- A comparison that matches under a `delta` costs nothing: the test is a pass and
  `summary.py --list delta-stale` names it, so the marker is removed.
- Probing (`KIT_PROBE_PENDING`) is for `pending`; a delta needs no probe.

## What is small (a delta)

The rows say the same words and differ in how they are drawn:

- colour or style of cells (a foreground, a dim, a bold the oracle has or lacks);
- an indent or a column offset (a list nested by 2 where the oracle nests by 4), a gap row's width;
- a glyph (`•` against `◦`, a box-drawing character, an arrow, a bullet, a rule's character);
- spacing inside a row and where a row is cut (a wrap at another column, a URL broken at the
  width) while every letter and digit stays, in order.

The kit refuses what the words decide: under a `delta`, a row comparison whose letters and digits,
read in order, differ from the oracle's fails the test (`kit::delta refused: the rows say other
things`). That is the line, enforced, not trusted.

## What is not small (stays `pending`, with its G row or reason)

- a row, a cell or a control that is missing, or one the oracle does not draw;
- other words or numbers (math drawn as source against the oracle's symbols, another label, a
  clipped text with an ellipsis the oracle does not have);
- other behaviour: a key, a click, a scroll, a focus or a selection that does not work as the
  oracle's; a link destination that is lost (a wrapped URL whose OSC 8 target breaks);
- a state the screen cannot reach, a seed that does not exist (`pending`, a missing door);
- a cursor in another place, a different number of rows the app owns.

When in doubt it is not small: look at the diff, then decide. A converted test needs its diff read
by eye (the DELTA lines carry it).

## How a test is converted

`tools/rtwk-port/oracle-delta/` lists the candidates: tests whose probe failure is a rows or
styles comparison whose words agree (`python3 -I tools/rtwk-port/oracle-delta/candidates.py`).
The porter reads each diff, then replaces the test's `kit::pending(owner, reason)` with
`kit::delta(owner, what)` (the body stays whole) and runs the shard.
