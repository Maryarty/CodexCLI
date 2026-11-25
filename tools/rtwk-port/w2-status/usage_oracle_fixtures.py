#!/usr/bin/env python3
"""Writes the usage debug entries made of the oracle's `analytics/test_fixtures.rs`
(`fixture::view/history/tokens/chats`, the seeded reports of an account kind) with the edits the
oracle's tests make to them: dashboard_month, usage_month_other, denominators,
denominators_credits, business_tokens, business_updated_12h, tasks_updated_12h, enterprise_all,
large_values, chat_padding.  Run `python3 -I` from the repo root; the result is committed."""
import datetime
import json
import pathlib

OUT = pathlib.Path("codex-rs/tui-rtwk/src/pages/usage/fixtures")
END = datetime.date(2026, 9, 2)
FRESH = int(datetime.datetime(2026, 9, 2, 16, 0, tzinfo=datetime.timezone.utc).timestamp())
TOOL_DAILY = [[34, 23, 29, 26, 22, 23, 29], [8, 6, 9, 8, 7, 5, 11]]
GROUPS = [
    (["Desktop app", "CLI", "IDE extension", "Web"], [35, 36, 17, 12]),
    (["Tasks", "Code review", "Images", "Other"], [65, 15, 12, 8]),
    (["GPT-5.6-Sol", "GPT-5.6-Terra", "GPT-5.5", "Other"], [60, 25, 10, 5]),
    (["User messages", "Goals", "Automations", "Other"], [29, 10, 5, 5]),
    (["Standard", "Fast", "Priority", "Unknown"], [65, 30, 5, 0]),
    (["Low", "Medium", "High", "Other"], [10, 30, 50, 10]),
    (["Uncached input", "Cached input", "Output", "Other"], [20, 70, 10, 0]),
]
MODEL = [("usage", 2)]
SECTIONS = ["usage", "credits", "activity", "plugins", "skills"]


def daily(section, rng):
    earlier, recent = {
        "usage": (40, [20, 73, 64, 91, 81, 100, 86]),
        "credits": (35, [0, 41, 57, 85, 75, 109, 64]),
        "activity": (34, [62, 83, 107, 75, 59, 85, 103]),
    }[section]
    return [earlier] * (0 if rng == 0 else 23) + recent


def breakdown(total, group):
    values = [total * w // 100 for w in GROUPS[group][1]]
    if group != 3:
        values[3] = total - sum(values[:3])
    return values


def start(rng):
    return END - datetime.timedelta(days=6 if rng == 0 else 29)


def history(report, rng, group):
    section = SECTIONS[report]
    if report < 3:
        series = daily(section, rng)
    else:
        series = [20 if report == 3 else 5] * (0 if rng == 0 else 23) + TOOL_DAILY[report - 3]
    unit = "relative_usage" if report == 0 else "credits" if report == 1 else "count"
    days = []
    for index, total in enumerate(series):
        if report < 3:
            labels = GROUPS[group][0]
        elif report == 3:
            labels = ["Figma", "Slack", "Notion", "GitHub"]
        else:
            labels = ["Code review", "Planning", "Debugging", "Testing"]
        values = breakdown(total, group if report < 3 else 0)
        if report == 0 and group == 0:
            pairs = [("Codex", total * 65 // 100), ("Work", total - total * 65 // 100)]
        else:
            pairs = list(zip(labels, values))
        days.append(
            {
                "date": str(start(rng) + datetime.timedelta(days=index)),
                "total": float(total),
                "values": [{"key": k, "label": k, "value": float(v)} for k, v in pairs],
            }
        )
    return {"unit": unit, "updated_at": None, "data": days}


def tokens(rng, group):
    days = []
    for index, total in enumerate(daily("usage", rng)):
        counts = {"Uncached input": total * 1000, "Cached input": total * 7000, "Output": total * 500}
        if group == 2:
            counts = {"GPT-5.5": sum(counts.values())}
        values = [{"key": k, "label": k, "value": float(counts[k])} for k in sorted(counts)]
        days.append(
            {
                "date": str(start(rng) + datetime.timedelta(days=index)),
                "total": float(sum(counts.values())),
                "values": values,
            }
        )
    return {"unit": "tokens", "updated_at": FRESH, "data": days}


def chats():
    rows = []
    for title, credits in [
        ("Q3 planning analysis", 120),
        ("Refactor billing usage", 113),
        ("Other usage", 109),
        ("Launch readiness review", 85),
        ("Plan the week", 0),
    ]:
        micros = credits * 1_000_000
        group = {"model": "GPT-5.6-Sol", "effort": "high", "speed": "standard", "credits_micros": micros}
        rows.append({"title": title, "estimate": {"credits_micros": micros, "groups": [group]}})
    return {"ready": {"rows": rows}}


def ready(value):
    return {"ready": value}


def seeded(plan, section, ranges=None, groups=None, hour24=True, reports=None, **more):
    """`view(kind)` with `seed_reports`: the reports of the sections the account shows."""
    ev = {"account": {"plan": plan, "today": str(END), "hour24": hour24}, "section": section}
    if groups:
        ev["groups"] = [{"section": s, "group": g} for s, g in groups]
    if ranges:
        ev["ranges"] = ranges
    ev["histories"] = [{"section": s, "load": ready(h)} for s, h in (reports or [])]
    ev.update(more)
    return ev


def write(name, head, ev):
    lines = [f"# {h}" for h in head]
    lines.append(json.dumps({"step": "seed", "seed": "session", "thread": "thread-1"}, separators=(",", ":")))
    lines.append(json.dumps({"step": "screen", "dir": "usage", "ev": ev}, separators=(",", ":")))
    (OUT / f"{name}.jsonl").write_text("\n".join(lines) + "\n")


def business_reports():
    """A business account's Tokens report starts grouped by model (group 2)."""
    return [
        ("credits", history(1, 0, 0)),
        ("usage", tokens(0, 2)),
        ("plugins", history(3, 0, 0)),
        ("skills", history(4, 0, 0)),
    ]


write(
    "dashboard_month",
    [
        "`analytics_narrow_charts_and_small_terminals_keep_valid_cursors` (analytics_tests.rs): a Plus",
        "account's Usage, Messages, Plugins and Skills reports over 30 days (the three ranges),",
        "`fixture::history(report, 1, 0)`.",
    ],
    seeded(
        "plus",
        "usage",
        ranges=[1, 1, 1],
        reports=[(s, history(r, 1, 0)) for s, r in [("usage", 0), ("activity", 2), ("plugins", 3), ("skills", 4)]],
    ),
)


def month_other():
    data = history(0, 1, 2)
    for day in data["data"]:
        day["values"][3]["value"] += 500.0
        day["total"] += 500.0
    return data


write(
    "usage_month_other",
    [
        "`analytics_series_rank_by_range_and_keep_calendar_ticks_stable` (analytics_tests.rs): a Plus",
        "account's Usage report over 30 days by model, 500 more units on every day's `Other`.",
    ],
    seeded("plus", "usage", ranges=[1, 0, 0], groups=[("usage", 2)], reports=[("usage", month_other())]),
)


def one_day(total, label, value, unit, key=None):
    return {
        "unit": unit,
        "updated_at": None,
        "data": [
            {
                "date": str(END),
                "total": total,
                "values": [{"key": key or label, "label": label, "value": value}],
            }
        ],
    }


write(
    "denominators",
    [
        "`analytics_preserves_full_denominators_and_signed_small_credits` (analytics_tests.rs): a Plus",
        "account's Usage report by task start, one day of 100 units with 30 of them goals.",
    ],
    seeded("plus", "usage", groups=[("usage", 3)], reports=[("usage", one_day(100.0, "Goals", 30.0, "relative_usage", "goal"))]),
)
write(
    "denominators_credits",
    [
        "`analytics_preserves_full_denominators_and_signed_small_credits` (analytics_tests.rs): an",
        "Enterprise account's Credits report, one day of -0.000004 credits (CLI).",
    ],
    seeded(
        "enterprise_cbp_usage_based",
        "credits",
        reports=[("credits", one_day(-0.000004, "CLI", -0.000004, "credits"))],
    ),
)
write(
    "business_tokens",
    [
        "`tall_reports_use_available_height_and_wide_reports_put_details_beside_plot`,",
        "`reports_keep_tabs_fixed_and_expose_data_on_small_screens` (recovery_tests.rs,",
        "redesign_tests.rs; `fixture::view(Business)`): a Team account's reports of the week, the",
        "Tokens report (731,000 on the last day) in front, and its chats.",
    ],
    seeded("team", "usage", groups=MODEL, reports=business_reports(), chats=chats()),
)
failed = business_reports()
failed[1] = ("usage", {"error": "Couldn't load tokens."})
failed_ev = seeded("team", "usage", groups=MODEL, reports=[], chats=chats())
failed_ev["histories"] = [
    {"section": s, "load": (h if "error" in h else ready(h))} for s, h in failed
]
write(
    "business_tokens_failed",
    [
        "`reports_keep_tabs_fixed_and_expose_data_on_small_screens` (redesign_tests.rs): the Team account",
        "of `business_tokens` after its Tokens report failed (`Couldn't load tokens.`).",
    ],
    failed_ev,
)
write(
    "business_updated_12h",
    [
        "`twelve_hour_metadata_keeps_utc_across_renderers` (recovery_tests.rs): the Team account of",
        "`business_tokens` on a 12-hour clock, the Tokens report made at 16:00 UTC.",
    ],
    seeded("team", "usage", hour24=False, groups=MODEL, reports=business_reports(), chats=chats()),
)
write(
    "tasks_updated_12h",
    [
        "`twelve_hour_metadata_keeps_utc_across_renderers` (recovery_tests.rs): a Plus account's Chats",
        "report, one chat without usage, listed at 16:00 UTC, on a 12-hour clock.",
    ],
    seeded(
        "plus",
        "chats",
        hour24=False,
        tasks=ready({"rows": [{"title": "Clock fixture"}], "updated_at": FRESH}),
    ),
)
write(
    "enterprise_all",
    [
        "`analytics_sections_adapt_to_terminal_colors` (styles_tests.rs; `fixture::view(Enterprise)`):",
        "an Enterprise account with the report of every section and the chats.",
    ],
    seeded("enterprise_cbp_usage_based", "credits", groups=MODEL, reports=business_reports(), chats=chats()),
)


def big(total, unit):
    return one_day(total, "Example", total, unit, "test")


write(
    "large_values",
    [
        "`report_headlines_and_axes_compact_large_values` (compact_tests.rs): an Enterprise account's",
        "Tokens, Plugins and Skills reports of one day with 12,280,365,226 tokens, 9,749 calls and",
        "2,901 uses.",
    ],
    seeded(
        "enterprise_cbp_usage_based",
        "usage",
        reports=[
            ("usage", big(12_280_365_226.0, "tokens")),
            ("plugins", big(9_749.0, "count")),
            ("skills", big(2_901.0, "count")),
        ],
    ),
)
write(
    "chat_padding",
    [
        "`chat_selection_keeps_a_highlighted_trailing_blank_after_wrapping` (styles_tests.rs): a Plus",
        "account's Chats report of one chat, `Building Analytics`, 42% weekly, 15% 5-hour, 35.83.",
    ],
    seeded(
        "plus",
        "chats",
        tasks=ready(
            {
                "rows": [
                    {
                        "title": "Building Analytics",
                        "task": {
                            "status": "available",
                            "amounts": {"weekly_percent": 42.0, "five_hour_percent": 15.0, "balance": "35.83"},
                            "groups": [],
                        },
                    }
                ]
            }
        ),
    ),
)


def credits_exact_with(edit):
    rows = [json.loads(line) for line in (OUT / "credits_exact.jsonl").read_text().splitlines() if not line.startswith("#")]
    ev = rows[1]["ev"]
    data = ev["histories"][0]["load"]["ready"]["data"]
    edit(ev["histories"][0]["load"]["ready"], data)
    return ev


def zeroed(ready_load, data):
    for day in data:
        day["total"] = 0.0
        day["values"] = []


write(
    "credits_zero",
    [
        "`analytics_dates_distinguish_zero_missing_and_exact_details` (analytics_tests.rs): the Credits",
        "report of `credits_exact` with every day emptied to zero (the test's fourth panel).",
    ],
    credits_exact_with(zeroed),
)
write(
    "credits_empty",
    [
        "`analytics_dates_distinguish_zero_missing_and_exact_details` (analytics_tests.rs): the Credits",
        "report of `credits_exact` without a day (the test's fifth panel).",
    ],
    credits_exact_with(lambda load, data: data.clear()),
)
