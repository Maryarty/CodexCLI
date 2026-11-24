"""Adds `context`, `context_label`, `action` to the items of the keymap fixture (the search text
starts with the context label and the action name: `Global open_agents Open Agents ...`).
Rerunnable: an item that has them keeps them. python3 -I keymap_fixture.py <fixture.jsonl>..."""
import json
import sys

for path in sys.argv[1:]:
    lines = open(path).read().split("\n")
    out = []
    for line in lines:
        if line.startswith("{") and '"kind":"keymap"' in line:
            data = json.loads(line)
            for tab in data["fact"]["tabs"]:
                for item in tab.get("items", []):
                    label, action = item["search"].split(" ")[:2]
                    item.setdefault("context_label", label)
                    item.setdefault("context", label.lower())
                    item.setdefault("action", action)
            line = json.dumps(data, separators=(",", ":"), ensure_ascii=False)
        out.append(line)
    open(path, "w").write("\n".join(out))
