# oracle-delta

`delta.py` (python3 -I, stdlib): moves oracle tests whose pending reason is only a small visual
difference from `kit::pending` to `kit::delta` (docs/rtwk-port/visual-deltas.md). Steps in the
docstring of the script: probe run, `candidates`, `apply`, run, `settle` (a converted test that is not
green goes back to pending, the kit refuses rows that say other things), `show` (read each delta),
`revert` (by name). Rerunnable: the ledger holds the original call of each converted test.
Used by kit-delta (10 tests of the free shards) and by rtwk-gap-probe.
