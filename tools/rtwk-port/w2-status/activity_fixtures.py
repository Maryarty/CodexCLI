#!/usr/bin/env python3
"""Writes the usage debug entries of the oracle's activity chart tests and of the consumer
chats test (src/pages/usage/fixtures): activity_daily, activity_weekly, activity_cumulative,
activity_empty (analytics/activity_chart_tests.rs: today 2026-05-29, the buckets of each test)
and chats_balance (analytics/consumer_refresh_tests.rs, the chats as the test opens them).

    cd codex-rs/tui-rtwk && python3 -I ../../tools/rtwk-port/w2-status/activity_fixtures.py
"""
import json
import pathlib

fx = pathlib.Path("src/pages/usage/fixtures")
over = (fx / "overview.jsonl").read_text().splitlines()
seed = over[3]
screen = json.loads(over[4])

TODAY = "2026-05-29"


def activity(name, note, buckets, group):
    ev = json.loads(json.dumps(screen))
    ev["ev"]["account"]["today"] = TODAY
    profile = ev["ev"]["profile"]["ready"]
    profile["metadata"]["stats_as_of"] = TODAY
    profile["stats"]["daily_usage_buckets"] = [
        {"start_date": d, "tokens": n} for d, n in buckets
    ]
    if group:
        ev["ev"]["groups"] = [{"section": "summary", "group": group}]
    lines = [f"# {note}", seed, json.dumps(ev, separators=(",", ":"))]
    (fx / f"{name}.jsonl").write_text("\n".join(lines) + "\n")


activity("activity_daily", "`daily_graph_snapshot_uses_distinct_empty_and_active_cells` (activity_chart_tests.rs): a Plus account's Overview, today 2026-05-29, one token on the 25th and four on the 29th.",
         [("2026-05-25", 1), ("2026-05-29", 4)], 0)
activity("activity_empty", "`daily_graph_snapshot_stays_left_aligned_in_wide_terminal` (activity_chart_tests.rs): the Overview of an account with no daily buckets, today 2026-05-29.",
         [], 0)
weekly = [("2026-05-11", 3), ("2026-05-18", 6), ("2026-05-25", 9)]
activity("activity_weekly", "`weekly_graph_snapshot_renders_bar_chart_and_caption` (activity_chart_tests.rs): the Overview's weekly view, 3, 6 and 9 tokens on three Mondays.",
         weekly, 1)
activity("activity_cumulative", "`cumulative_graph_snapshot_renders_running_total_bar_chart_and_caption` (activity_chart_tests.rs): the same buckets, the running-total view.",
         weekly, 2)


# chats_balance: tasks.jsonl cut to the chats as the consumer test opens them (the balance only).
tasks = (fx / "tasks.jsonl").read_text().splitlines()
ev = json.loads(tasks[2])
rows = ev["ev"]["tasks"]["ready"]["rows"][:3]
rows[2]["task"]["amounts"] = {"balance": "-0.000004"}
ev["ev"]["tasks"]["ready"]["rows"] = rows
note = [
    "# `consumer_chats_keep_missing_rows_and_only_offer_known_metrics` (consumer_refresh_tests.rs): a Plus account's",
    "# Chats report as the test opens it: three chats, only the balance known (the one metric), before the test adds a percent and a plan-covered chat.",
]
(fx / "chats_balance.jsonl").write_text(
    "\n".join(note + [tasks[1], json.dumps(ev, separators=(",", ":"))]) + "\n"
)
