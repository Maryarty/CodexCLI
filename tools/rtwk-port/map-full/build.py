"""Builds docs/rtwk-port/reports/port-map.md (+ port-map-modules.tsv, unpend/<package>.txt) from the data
files of this directory.  python3 -I tools/rtwk-port/map-full/build.py [--check]  (check.py calls verify())."""
import collections
import csv
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cover  # noqa: E402
import modules  # noqa: E402
import packages  # noqa: E402
import rows_chat  # noqa: E402
import rows_extra  # noqa: E402
import tests_rules  # noqa: E402

ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
OUT = os.path.join(ROOT, "docs/rtwk-port/reports/port-map.md")
OUT_MOD = os.path.join(ROOT, "docs/rtwk-port/reports/port-map-modules.tsv")
CHAT = os.path.join(ROOT, "docs/rtwk-port/reports/chat-feature-map.md")
CSV = os.path.join(ROOT, "docs/rtwk-port/research/codex-tui-features.csv")
ORACLE = os.path.join(ROOT, "codex-rs/tui/src")
P = packages.P


def chat_rows():
    sec, rows, on = "", [], False
    for i, l in enumerate(open(CHAT, encoding="utf-8").read().split("\n"), 1):
        if l.startswith("## "):
            sec = l[3:]
            on = sec.startswith("Транскрипт и ячейки") or (on and not sec.startswith("Заглушки порта"))
        if not on or not l.startswith("| ") or l.startswith("| Функция") or l.startswith("|---"):
            continue
        c = [x.strip() for x in l.strip().strip("|").split(" | ")]
        rows.append(dict(sec=sec, title=c[0], status=c[1][:1], note=c[2], line=i))
    for r in rows:
        key = next((k for k in rows_chat.SECTIONS if r["sec"].startswith(k)), None)
        hit = next((x for x in rows_chat.RULES if re.search(x[0], r["title"])), None)
        dflt = rows_chat.SECTIONS[key]
        r["oracle"] = hit[2] if hit else dflt[0]
        r["port"] = hit[3] if hit else dflt[1]
        r["owner"] = (hit[1] if hit else dflt[2]) if r["status"] != "✅" else "—"
    return rows


def all_rows():
    rows = chat_rows()
    for t, st, gap, orc, port, own in rows_extra.EXTRA:
        rows.append(dict(sec="Вне экрана чата", title=t, status=st, note=gap or "—", oracle=orc, port=port, owner=own))
    return rows


def pending():
    out = []
    for l in open(os.path.join(HERE, "pending-tests.tsv"), encoding="utf-8"):
        n, s, r = l.rstrip("\n").split("\t")
        out.append((n, s, r, tests_rules.classify(n, r)))
    return out


def csv_ids():
    return list(csv.DictReader(open(CSV, encoding="utf-8")))


def resolve(cid, rows):
    for rx, target in cover.COVER:
        if re.search(rx, cid):
            if target == "slash":
                n = re.escape(cid[6:])
                for r in rows:
                    if re.search(r"^/(?:[\w|-]+, )*(?:/)?%s\b" % n, r["title"]) or re.search(r"(?:^|[ ,/])/?%s(?:\b| )" % n, r["title"]):
                        if r["title"].startswith("/"):
                            return r
                return None
            kind, _, key = target.partition(":")
            for r in rows:
                if kind == "x" and r["sec"] == "Вне экрана чата" and r["title"] == key:
                    return r
                if kind == "chat" and r["sec"] != "Вне экрана чата" and re.search(key, r["title"]):
                    return r
            return None
    return None


def module_rows():
    out = []
    for d, _, names in os.walk(ORACLE):
        for f in names:
            if f.endswith(".rs"):
                out.append(os.path.relpath(os.path.join(d, f), ORACLE))
    res = []
    for f in sorted(out):
        h = next((m for m in modules.MODULES if re.search(m[0], f)), None)
        res.append((f, h))
    return res


def verify():
    rows, fail = all_rows(), 0
    bad = [r["title"] for r in rows if r["status"] != "✅" and r["owner"] not in P]
    print("rows: %d; non-✅ rows without a package: %d" % (len(rows), len(bad)), bad[:5])
    fail += bool(bad)
    bad = [r["title"] for r in rows if r["owner"] != "—" and r["owner"] not in P]
    fail += bool(bad)
    unc = [c["id"] for c in csv_ids() if resolve(c["id"], rows) is None]
    print("CSV ids: %d; without a row: %d" % (len(csv_ids()), len(unc)), unc[:8])
    fail += bool(unc)
    mods = module_rows()
    un = [f for f, h in mods if h is None or (h[2] != "—" and h[2] not in P)]
    print("oracle modules: %d; without a package/row: %d" % (len(mods), len(un)), un[:8])
    fail += bool(un)
    pend = pending()
    un = [n for n, s, r, p in pend if p not in P]
    print("pending tests: %d; unassigned: %d" % (len(pend), len(un)), un[:5])
    fail += bool(un)
    nonok = {r["owner"] for r in rows if r["status"] != "✅"}
    idle = [p for p in P if not p.startswith("fw1-") and p not in nonok and not any(t[3] == p for t in pend)
            and not p.startswith("fw-")]
    print("packages closing no row and no test: %d" % len(idle), idle)
    fail += bool(idle)
    s = {sc[0] for sc in rows_extra.SCREENS}
    print("screens S01-S27: %d" % len(s))
    return fail


def span(runs):
    m = re.match(r"(\d+)(?:-(\d+))?(\+)?", runs)
    lo = int(m[1])
    hi = int(m[2]) if m[2] else lo + (1 if m[3] else 0)
    return lo, hi


def esc(s):
    return str(s).replace("|", "\\|").replace("\n", " ")


def main():
    rows, pend, mods = all_rows(), pending(), module_rows()
    cnt = collections.Counter(r["status"] for r in rows)
    byown = collections.defaultdict(list)
    for r in rows:
        if r["owner"] != "—":
            byown[r["owner"]].append(r)
    tests = collections.defaultdict(list)
    for n, s, rsn, p in pend:
        tests[p].append((n, s))
    # unpend lists
    ud = os.path.join(HERE, "unpend")
    os.makedirs(ud, exist_ok=True)
    for f in os.listdir(ud):
        os.remove(os.path.join(ud, f))
    for p, lst in tests.items():
        open(os.path.join(ud, p + ".txt"), "w").write("\n".join(sorted(n for n, _ in lst)) + "\n")
    # modules tsv
    with open(OUT_MOD, "w", encoding="utf-8") as f:
        f.write("oracle_file\tstatus\tpackage\tport\n")
        for path, h in mods:
            f.write("%s\t%s\t%s\t%s\n" % (path, h[1], h[2], h[3]))
    L = []
    w = L.append
    lo = sum(span(p["runs"])[0] for p in P.values() if not p["id"].startswith("fw1-"))
    hi = sum(span(p["runs"])[1] for p in P.values() if not p["id"].startswith("fw1-"))
    wave = collections.Counter(p["wave"] for p in P.values())
    w("# Карта всего порта: oracle (codex-rs/tui) против порта (codex-rs/tui-rtwk)\n")
    w("Дата: 2026-11-24, база 01741db4b (батчи 10-12). Документ собран скриптом `tools/rtwk-port/map-full/build.py` из "
      "данных рядом с ним: правится дата, перезапуск. Проверка непротиворечивости — `tools/rtwk-port/map-full/check.py`. "
      "Карта экрана чата `chat-feature-map.md` остаётся (подробные ячейки, числа тестов, архитектура ADR-013) и ссылается сюда; "
      "здесь — вся функциональность oracle без исключений, пакеты работ по владению файлами, и %d pending-тестов по пакетам.\n" % len(pend))
    w("**Итог.** Строк-функций: %d (экран чата — %d из `chat-feature-map.md` + %d вне экрана чата): ✅ %d, 🟡 %d, ❌ %d, «?» %d. "
      "Все %d строк `codex-tui-features.csv` покрыты строкой (раздел «Покрытие CSV»), все %d файлов `codex-rs/tui/src` — пакетом или «✅ rtwk» "
      "(`port-map-modules.tsv`), все %d экранов карты S01-S27 (в карте нет S07-S09) — строкой. Пакетов: %d (волна 1 — %d, влита; волна 2 — %d; волна 3 — %d); "
      "кроме волны 1 — %d-%d прогонов порт-агента (прогон ≈150-250 ходов, ≈1500 написанных строк). Pending-тестов oracle: %d, "
      "у каждого пакет (раздел «Тесты»).\n" % (
          len(rows), sum(1 for r in rows if r["sec"] != "Вне экрана чата"), len(rows_extra.EXTRA), cnt["✅"], cnt["🟡"], cnt["❌"], cnt["?"],
          len(csv_ids()), len(mods), len(rows_extra.SCREENS), len(P), wave["W1 (влита)"], wave["W2"], wave["W3"], lo, hi, len(pend)))
    w("## Как читать\n")
    w("- Статус: ✅ работает или заменено rtwk; 🟡 частично; ❌ нет; «?» не удалось установить (первый шаг пакета — проверить). "
      "Чего нет — слово в слово из `chat-feature-map.md` для строк чата (их числа и оговорки не перебирались), для остальных — по коду на 01741db4b.")
    w("- Oracle — файлы относительно `codex-rs/tui/src`; порт — относительно `codex-rs/tui-rtwk` (`(нет)` — файла ещё нет, его создаёт пакет). "
      "Пакет — владелец, чьими файлами строка закрывается; «ядро: fw1-cN» — ядровая половина влита в волне 1 (`dc3417d92`), "
      "её файлы здесь не переплановываются, пакет делает UI-половину и вешает проверку.")
    w("- Нет списка «не пакеты»: строка, упирающаяся в rtwk, получает пакет, первый шаг которого называет, что просить у rtwk (поле «первый шаг»).\n")
    w("## Правила владения\n")
    w("1. **Один файл — один пакет.** Каждый production-файл порта (`src`, `core/src`, `host/src`, `tests/rules`) в точности у одного пакета: "
      "явным путём, `dir/**` или `rest:dir` (все ещё не названные файлы каталога). `check.py` падает при пересечении файла или пути; вывод проверки (0 пересечений, 0 файлов без владельца) — в `task_done`.")
    w("2. **Хаб не отдаётся одному пакету на все фичи** (ADR-012). Фича входит в приложение только дверьми слайса: `feature_list!`/`target_of` "
      "(слэш-команды), `AppCx::ask` и `ThreadCx::request` (RPC сервера), `AppCx::effect` с вариантом `effects::Ask` (только возможность хоста, "
      "строка в `hub_budget.txt`), `ThreadModel.features` (состояние треда). Правка хаба — только СЖАТИЕ, и это работа двух framework-пакетов: "
      "**fw-core-hubs** (общий вариант `SessionIntent::Feature`, конец арма NotAvailable, одна дверь возможностей хоста) и **fw-ui-registry** "
      "(реестры действий клавиш, целей open, ячеек, режимов и тестовых корней; `session_page.rs` — один владелец с причиной). "
      "Остальные пакеты называют только файлы своего слайса, своих страниц и свои тесты.")
    w("3. **Двери.** `core/src/features/mod.rs` и `src/features/mod.rs` (`feature_list!`, `sheet_list!`), `key_actions/mod.rs` (`action_list!`), `cells/mod.rs` (`cell_list!`), `transcript/mod.rs` (`mode_list!`) — по одной строке на слайс; их не владеет никто, "
      "слайс дописывает свою строку (слияние без конфликта по смыслу). Это единственное исключение из пункта 1.")
    w("4. **Тесты.** Файлы тестов (шарды `tests/oracle/*.rs`, `tests/pages/*`) не входят в проверку пересечений: шард собран из независимых функций, "
      "пакет правит только функции из своего списка `tools/rtwk-port/map-full/unpend/<пакет>.txt` (сгенерирован, по одному `шард::тест` в строке); "
      "несколько пакетов в одном шарде сливаются построчно. Новые тестовые файлы пакета — `tests/pages/<пакет>_*.rs`, строки `mod` в корнях добавляет реестр fw-ui-registry.")
    w("5. **Порядок.** Волна 1 (fw1-c1..c9) влита (`dc3417d92`), её файлы — у пакетов W2. Волна 2: сначала framework-пакеты, пока они не влиты, UI-пакеты стоят; затем восемь UI-пакетов `w2-*` параллельно "
      "(пересечений нет). Волна 3: целые фичи (voice, sidebar, inline, ui_tree, pets, startup, …); часть упирается в rtwk (см. «первый шаг»).\n")
    w("## Пакеты\n")
    w("| Пакет | Волна | Прогонов | Строк | Строк закрыто | Pending-тестов | Зависит от |")
    w("|---|---|---|---|---|---|---|")
    for p in P.values():
        if p["id"].startswith("fw1-"):
            closed = "ядро для %d" % sum(len(v) for o, v in byown.items() if p["id"] in P[o]["deps"])
        else:
            closed = str(len(byown.get(p["id"], []))) if not p["id"].startswith("fw-") else "— (сжатие хабов)"
        w("| `%s` — %s | %s | %s | %s | %s | %d | %s |" % (p["id"], esc(p["title"]), p["wave"], p["runs"], p["lines"],
                                                       closed, len(tests.get(p["id"], [])), esc(p["deps"] or "—")))
    w("")
    for p in P.values():
        w("### `%s`\n" % p["id"])
        w("- **Что:** %s. **Волна:** %s; **размер:** %s прогонов, %s строк." % (p["title"], p["wave"], p["runs"], p["lines"]))
        if p["brief"]:
            names = [r["title"] for r in byown.get(p["id"], [])]
            w("- **Бриф и задача:** `docs/rtwk-port/research/w2-packages/briefs.md` %s; задача tm `%s`, её `area` — список владения "
              "(%d путей, `tools/rtwk-port/map-full/packages.py`)." % (p["brief"], p["tm"], len(p["own"])))
            if p["deps"]:
                w("- **Зависит:** %s." % p["deps"])
            w("- **Строки (%d):** %s." % (len(names), "; ".join(n[:60] for n in names) if names else "—"))
            w("- **Тесты:** %d pending (список — `tools/rtwk-port/map-full/unpend/%s.txt`).\n" % (len(tests.get(p["id"], [])), p["id"]) if tests.get(p["id"]) else "- **Тесты:** —.\n")
            continue
        if p["tm"]:
            w("- **Задача tm:** `%s` (влита; ядровая половина, файлы — у пакетов W2)." % p["tm"])
        w("- **Файлы (владеет):** %s." % ", ".join("`%s`" % x for x in p["own"]))
        if p["oracle"]:
            w("- **Oracle:** %s." % p["oracle"])
        if p["core"]:
            w("- **Ядро / шаги:** %s." % p["core"])
        if p["ui"]:
            w("- **UI:** %s." % p["ui"])
        if p["first"]:
            w("- **Первый шаг:** %s." % p["first"])
        if p["deps"]:
            w("- **Зависит:** %s." % p["deps"])
        names = [r["title"] for r in byown.get(p["id"], [])]
        w("- **Строки (%d):** %s." % (len(names), "; ".join(n[:60] for n in names) if names else "—"))
        w("- **Тесты:** %d pending (список — `tools/rtwk-port/map-full/unpend/%s.txt`).\n" % (len(tests.get(p["id"], [])), p["id"]) if tests.get(p["id"]) else "- **Тесты:** —.\n")
    # rows by section
    w("## Строки\n")
    secs = []
    for r in rows:
        if r["sec"] not in secs:
            secs.append(r["sec"])
    def dep_w1(owner):
        ws = re.findall(r"fw1-c\d", P[owner]["deps"]) if owner in P else []
        return ("ядро: " + ", ".join(ws) + " (влито); ") if ws else ""
    for s in secs:
        w("### %s\n" % s)
        w("| Функция | Статус | Чего нет | Oracle | Порт | Пакет |")
        w("|---|---|---|---|---|---|")
        for r in rows:
            if r["sec"] != s:
                continue
            w("| %s | %s | %s | %s | %s | %s |" % (esc(r["title"]), r["status"], esc(r["note"]), esc(r["oracle"]), esc(r["port"]),
                                                    "—" if r["owner"] == "—" else dep_w1(r["owner"]) + "`%s`" % r["owner"]))
        w("")
    w("### Карта экранов S01-S27\n")
    w("| Экран | Что | Статус | Чего нет | Пакет |")
    w("|---|---|---|---|---|")
    for sid, what, st, gap, own in rows_extra.SCREENS:
        w("| %s | %s | %s | %s | %s |" % (sid, esc(what), st, esc(gap), "—" if own == "—" else "`%s`" % own))
    w("")
    # coverage
    w("## Покрытие CSV (`research/codex-tui-features.csv`)\n")
    w("Каждая из %d строк — в строке этого документа; ✅ — готово, иначе пакет закрывает строку.\n" % len(csv_ids()))
    w("| id | Название | Строка | Статус | Пакет |")
    w("|---|---|---|---|---|")
    for c in csv_ids():
        r = resolve(c["id"], rows)
        w("| %s | %s | %s | %s | %s |" % (c["id"], esc(c["title"][:60]), esc(r["title"][:55]), r["status"], "—" if r["owner"] == "—" else "`%s`" % r["owner"]))
    w("")
    w("## Покрытие модулей oracle\n")
    w("Все %d файлов `codex-rs/tui/src` (с тестами) — по строке в `docs/rtwk-port/reports/port-map-modules.tsv` (путь, статус, пакет, место в порте). Сводка:\n" % len(mods))
    agg = collections.defaultdict(lambda: collections.Counter())
    for path, h in mods:
        agg[h[2]][h[1]] += 1
    w("| Пакет | Файлов oracle | ✅ | 🟡 | ❌ |")
    w("|---|---|---|---|---|")
    for k, v in sorted(agg.items(), key=lambda kv: -sum(kv[1].values())):
        w("| %s | %d | %d | %d | %d |" % ("— (заменено rtwk / готово)" if k == "—" else "`%s`" % k, sum(v.values()), v["✅"], v["🟡"], v["❌"]))
    w("")
    w("## Тесты: %d pending по пакетам\n" % len(pend))
    w("Правило отнесения (`tests_rules.py`): тест, чья причина названа семенем/входом/именами кадров kit, — в `kit-seeds` (это обвязка, пока она не влита, "
      "функция не зазеленит тест); остальные — по имени теста, затем по причине, затем по шарду. Неточность правил — не пробел: пакет получает свой список, "
      "перенос теста между пакетами — правка одной строки в `unpend/`.\n")
    w("| Пакет | Pending | Шарды (число) |")
    w("|---|---|---|")
    for p in P:
        if p in tests:
            sh = collections.Counter(n.split("::")[0] for n, _ in tests[p])
            w("| `%s` | %d | %s |" % (p, len(tests[p]), ", ".join("%s %d" % kv for kv in sh.most_common(6))))
    w("")
    w("По тегам владельца шарда: %s.\n" % ", ".join("%s %d" % kv for kv in collections.Counter(s for _, s, _, _ in pend).most_common()))
    w("## Связь с прежней картой\n")
    w("Пакеты P1-P21 из `chat-feature-map.md` («Пакеты для портеров») заменены пакетами этого документа: ядровые половины P1-P19 — fw1-c1..c9 (влиты), "
      "UI-половины и sweep — восемь пакетов `w2-*` (первый план из 20 пакетов `ui-*` и `*-sweep` слит в них 2026-11-24, см. `research/w2-packages/briefs.md`); P20/P21 — framework-пакеты. Строки `chat-feature-map.md` с задачей «устарела» теперь имеют пакет здесь.\n")
    open(OUT, "w", encoding="utf-8").write("\n".join(L))
    print("wrote", OUT, len(L), "lines")


if __name__ == "__main__":
    if "--check" in sys.argv:
        sys.exit(1 if verify() else 0)
    main()
