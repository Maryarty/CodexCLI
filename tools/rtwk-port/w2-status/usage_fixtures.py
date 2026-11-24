#!/usr/bin/env python3
"""Writes the usage debug entries' fixtures made of the oracle's `analytics/test_fixtures.rs`
(`fixture::history(1, 0, 0)`: the Credits report over seven days, grouped by surface) with the
edits the oracle's tests make to it.  Run `python3 -I` from the repo root; the result is committed."""
import json
import pathlib

OUT = pathlib.Path("codex-rs/tui-rtwk/src/pages/usage/fixtures")
RECENT = [0, 41, 57, 85, 75, 109, 64]
SURFACES = ["Desktop app", "CLI", "IDE extension", "Web"]
WEIGHTS = [35, 36, 17, 12]


def day(date, total):
    values = [total * w // 100 for w in WEIGHTS]
    values[3] = total - sum(values[:3])
    return {
        "date": date,
        "total": float(total),
        "values": [
            {"key": k, "label": k, "value": float(v)} for k, v in zip(SURFACES, values)
        ],
    }


def credits_week():
    dates = ["2026-08-27", "2026-08-28", "2026-08-29", "2026-08-30", "2026-08-31", "2026-09-01", "2026-09-02"]
    return [day(d, t) for d, t in zip(dates, RECENT)]


def entry(name, head, edit):
    data = credits_week()
    edit(data)
    ev = {
        "account": {"plan": "enterprise_cbp_usage_based", "today": "2026-09-02"},
        "section": "credits",
        "histories": [
            {"section": "credits", "load": {"ready": {"unit": "credits", "updated_at": None, "data": data}}}
        ],
    }
    lines = [f"# {h}" for h in head]
    lines.append(json.dumps({"step": "seed", "seed": "session", "thread": "thread-1"}, separators=(",", ":")))
    lines.append(json.dumps({"step": "screen", "dir": "usage", "ev": ev}, separators=(",", ":")))
    (OUT / f"{name}.jsonl").write_text("\n".join(lines) + "\n")


def one_product(data, total):
    data[1]["values"] = data[1]["values"][:1]
    data[1]["total"] = total
    data[1]["values"][0]["value"] = total
    del data[2]


entry(
    "credits_days",
    [
        "`analytics_day_navigation_keeps_geometry_and_legend` (analytics_tests.rs): an Enterprise",
        "account's Credits report over seven days with one product on Aug 28 and no row for Aug 29.",
    ],
    lambda data: one_product(data, data[1]["values"][0]["value"]),
)
entry(
    "credits_exact",
    [
        "`analytics_dates_distinguish_zero_missing_and_exact_details` (analytics_tests.rs): the",
        "same week with 12746.441206 credits on Aug 28 (one product) and no row for Aug 29.",
    ],
    lambda data: one_product(data, 12746.441206),
)
