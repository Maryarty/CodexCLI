# Round 2: status of the 30 earlier findings (judged on the code, codex-rs/tui/src/)

Tally: fixed 26, partly/not fixed 2 (C-08, C-05 headings), wrongly fixed 1 (G-02), withdrawn as wrong findings 2 (G-03, G-10), plus C-05 breakdown fixed.

| id | verdict | note |
|----|---------|------|
| C-01 cache cleared on account change | FIXED | Rule 6 / UC-02(c) say "clears the cache", card finished from an empty slice (settings.rs:267-278). Open point gone. |
| C-02 colon profile ids | FIXED | Residual: 3.2 examples omit `Read Only with network access` / `Workspace with network access` and the fall-through when the sandbox summary does not match (card.rs:677,684-698). |
| C-03 settlement conditional, waiting cards only | FIXED | UC-09(c) states "selected" and waiting cards. New problem: immediate post-turn read and relative delays unstated (see new_contradictions N-07). |
| C-04 reasoning effort source | FIXED | Rule 4 matches status_controls.rs:231-236. |
| C-05 model names | PARTLY | Breakdown: FIXED (3.4 names the free fn). Limit-bucket headings: NOT FIXED, 3.3 does not say headings use `model_display_name(limit_name)` (rate_limits.rs:262). |
| C-06 Disabled not for good | FIXED | Rule 8, UC-09(e), Open point 11. |
| C-07 raw drops style | FIXED | Rule 10. Echo-in-raw unstated (N-13). |
| C-08 "not accepted" case | NOT FIXED | UC-02(a) "or the shared refresh state discards the reply" vs section 5 "a reply 'not accepted' is no case". Code: app/rate_limit_refresh.rs `finish` returns Ignore (generation, older id, Failed); event_dispatch.rs:2240-2262. |
| C-09 section 5 rows cited elsewhere | FIXED | Citations resolved; noise list (H009,H054,H077,H078,H103,H110) not cited elsewhere (map.csv not read by me). |
| C-10 ShowStatus thread Option, description | FIXED | `Option<ThreadId>`; description string correct (slash_command.rs:122). |
| C-11 hidden labels reason | FIXED | Open point 2 names `Permissions`. Residual: claim that Models/Reasoning/Speed widen the column is moot (reserved `  Billed tokens`, 15, is wider). |
| C-12 outer indent | FIXED | 3.1. |
| C-13 monthly captured_at, types | FIXED | i32/i64. |
| C-14 foreign path host-dependent | FIXED | UC-12. |
| A-01 refreshing notice snapshot | FIXED | |
| A-02 excerpt of second card | FIXED | |
| A-03 loop snapshots | FIXED | |
| G-01 read also feeds notices | FIXED | Rule 5. But see ambiguity on dropped-ticket effect. |
| G-02 Model row derivation | WRONGLY FIXED | Spec: "the default model's name if empty". Code: empty model -> `DEFAULT_MODEL_DISPLAY_NAME = "loading"` (chatwidget.rs:172, settings.rs:656-662). Also free fn maps `gpt-reserve` -> `Luna Reserve` even outside the catalog. |
| G-03 (disputed: API key gives no Account row) | AUTHOR RIGHT, withdrawn | card.rs:770 gives `API key configured (run codex login to use ChatGPT)`; Token usage row hidden only for ChatGpt (card.rs:890). Plan table/workspace covered by UC-04. |
| G-04 collaboration row | FIXED | `enabled` is vacuously true (settings.rs:462); my earlier "not default" claim was wrong. Labels `Plan`/`Default` not given in spec. |
| G-05 availability, description | FIXED | |
| G-06 cache outlives widget | FIXED | |
| G-07 plan source, Disabled effect | FIXED | Non-billable plan does NOT remove rows of waiting cards (thread_usage.rs:59-76), only Disabled does; spec right. |
| G-08 raw | FIXED | |
| G-09 reset text at ingestion | FIXED with ambiguity | Spec "port may derive in the view (same result)" vs carried compose taking preformatted strings; oracle tests inject `"soon"`. |
| G-10 (disputed: combined headings only when same windows) | AUTHOR RIGHT, withdrawn | Heading unless exactly one window (rate_limits.rs:278-285); repeated Credits row in UC-07. |
| G-11 window labels | FIXED | |
| G-12 no timeout | FIXED | Rule 7, Open point 4. |
| G-13 ordering with running turn | FIXED | delegated to transcript slice (Rule 11). |

Late-reply check (C-01/UC-02(c)): the account-change handler bumps `rate_limit_hard_stop_generation` and calls `rate_limit_refresh_state.invalidate_recovery()` (app/app_server_events.rs:421-423), so a late status reply is not accepted and observe_quota_plan/banner do not run. "A late reply changes nothing" holds in the oracle, but only through the "not accepted" mechanism the spec says it removed (C-08).
