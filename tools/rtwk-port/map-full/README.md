# map-full: the map of the whole port

Source of `docs/rtwk-port/reports/port-map.md` (+ `port-map-modules.tsv`) and of the per-package un-pend
lists `unpend/<package>.txt`. Data: `packages.py` (packages, owned files, sizes), `rows_chat.py` (owner and files
of the rows of chat-feature-map.md by title), `rows_extra.py` (rows outside the chat screen, screen map),
`cover.py` (CSV id -> row), `modules.py` (every oracle file -> package), `tests_rules.py` + `pending-tests.tsv`
(1228 pending oracle tests -> package).

```
python3 -I tools/rtwk-port/map-full/build.py      # rewrite the map, the tsv and unpend/
python3 -I tools/rtwk-port/map-full/check.py      # no file in two packages, every non-✅ row / CSV id / module / test has a package
```

Rerunnable. A change of owner is an edit of one data line and a rerun; `pending-tests.tsv` is re-cut from
`summary.py --list pending` when the shards change.
