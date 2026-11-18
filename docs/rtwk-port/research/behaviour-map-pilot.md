# Behaviour map pilot: can Haiku write the oracle's specification? (S14)

tm: `behaviour-map-pilot`. The person (2026-11-18): Haiku could build a map of
all actions and behaviours of codex-tui, to serve as the specification the
port is built from — process.md Part 1 §3 step 1 ("read the oracle as a
specification") done once for every screen instead of by each porter.
Piloted on S14, the generic selection dialog
(`codex-rs/tui/src/bottom_pane/list_selection_view.rs`, 1,936 production
lines, 1,622 test lines, plus `selection_picker_tests.rs`).

## Conclusion after two rounds (2026-11-18)

- **The map is evidence, not a specification.** Round 2 (fixed input) passed
  recall — 94.8 % of the reference, full or partial — but only **40 %** of its
  176 rows are product behaviour; the rest is how the oracle implements it
  (view stack, completion flags, callbacks: 56 rows), layout (22), one caller
  (11), fixtures (10), input plumbing (6). The Sonnet reference is 70 %
  product. Nobody should build from either directly.
- **The deliverable is a short specification in the port's terms**
  (`behaviour-map/s14/spec.md`, template `behaviour-map/spec-template.md`):
  the API callers use, use cases as Given → action → result with their
  acceptance test, the key table in precedence order, a few rules, the
  oracle snapshots as the look, and open points for the person.
- **A specification must be checked against the code.** The first S14 spec,
  written from the maps, had 9 statements the code contradicts and 15
  missing product rules (`s14/judge2-spec-gaps.md`) — e.g. the preview
  callback's firing rules, what Enter does when every row is disabled, when
  plain keys are typed into an open search bar. All are fixed in spec.md;
  five are open points (oracle quirks: keep for parity or fix).
- **Pipeline per screen:** script candidates → Haiku map (cheap; the
  completeness checklist) → Sonnet writes the spec from the map's product
  rows and the code → an independent Sonnet check: every spec statement
  against the code, every product row of the map covered or listed as not
  carried → the person decides the open points → the porter gets the spec.
  The map stays as the evidence index behind the spec's row references.

## Round 2 (fixed input)

v3 candidates (304: public struct fields, arm precedence, the host
`bottom_pane/mod.rs` filtered to selection wiring, the selection helpers),
the precedence prompt. Haiku: 176 rules, all 263 citations valid, 48.5k
tokens reported, 21 min. Judge (Sonnet, every row read): 164 correct, 11
imprecise, **1 wrong** (H094: the preview callback at construction). Recall
against the same reference: full 118, partial 29, none 8 = **94.8 %**; 49
rows the reference lacks, 2 of them real (the activation-key guard, and an
oracle defect: a refresh reopens a closed on-demand search bar). The recall
criterion (≥ 90 %) is met, the precision one (0 wrong) missed by one row;
the bigger finding is the noise share above.

## Round 1

- **Haiku is accurate where it looks**: of its 127 rules, a blind Sonnet
  judge reading every cited range found 119 correct, 8 imprecise, **0 wrong**;
  every citation (199 lines, 87 tests, 22 snapshots) resolves.
- **It misses about one rule in six**: against an independent Sonnet
  reference of 155 rules, Haiku has 116 in full and 15 in part (84.5 %), 24
  not at all. The misses fall into four kinds, three of which come from the
  input it was given, not from the model (below).
- **About 4× cheaper, 3× slower**: Haiku 126k tokens in 19 min; the Sonnet
  reference 161k in 6 min; at the rtwk weights (Haiku 1 : Sonnet 3) that is
  ~126 vs ~484 units.
- **Test links are unreliable from either model**: a test listed on a rule
  fully asserts that rule in 6 of 15 sampled Haiku rows and 7 of 15 Sonnet
  rows; the rest assert part of it or the neighbouring branch.
- Round-1 recommendation (superseded by the conclusion above): adopt the
  map as the specification index after one more round.

## Method

1. `behaviour-map/extract.py` (stdlib) lists candidates with `file:line`:
   key-handling `match` arms, `BottomPaneView` / `Renderable` hooks, `&mut
   self` state transitions, sends on the app-event sender, tests, snapshots —
   130 for S14 (`s14/candidates-v1.csv`).
2. Haiku (`codex-reviewer`, model haiku) described each candidate from its
   cited range only, in a fixed schema (trigger, precondition, user-visible
   behaviour, backend effect, oracle cite, tests, snapshots), and added rules
   it saw no candidate for (11 rows) → `s14/haiku-map.csv`.
3. `behaviour-map/validate.py` checks every cited line range, test name and
   snapshot name against the oracle tree.
4. Sonnet (`codex-reviewer`) built a reference map of the same screen blind —
   no candidates, no Haiku output → `s14/sonnet-reference.csv` (155 rows; it
   followed the behaviour into `bottom_pane/mod.rs` and the selection helper
   modules on its own).
5. A third agent (Sonnet) judged both maps under neutral labels on the code:
   coverage match per reference row, correctness of every Haiku row,
   correctness of a 30-row sample of the reference, 15 test links each →
   `s14/judge-*.csv`, `s14/judge-summary.md`.

## Results

| measure | Haiku map | Sonnet reference |
|---|---|---|
| rules | 127 | 155 |
| citations valid (script) | 199 / 199 | 253 / 253 |
| correct / imprecise / wrong (judge) | 119 / 8 / 0 (all rows) | 29 / 1 / 0 (30-row sample) |
| recall vs the other | 116 full + 15 partial of 155 = 84.5 % | lacks 2 real rules the Haiku map has |
| test link fully matches the rule | 6 / 15 | 7 / 15 |
| tokens, time | 126k, 19 min, 108 tool calls | 161k, 6 min, 39 tool calls |

What Haiku missed (24 none, 15 partial), by kind:
1. **Key precedence and edge branches** (8): a later `match` arm shadowed
   by an earlier one — Space bound to a list action beats the toggle;
   Backspace ignored when the search field is hidden; a plain-character
   accept/cancel key is query text during on-demand search.
2. **Wiring outside the view file** (6): `bottom_pane/mod.rs` replaces the
   footer hint, drops a secondary action whose key is a list binding, clears
   the stack when a paste completes the view. Haiku was given only the view
   file.
3. **Row and parameter attributes** (7 + 3): `selected_description`,
   `category_tag`, `name_prefix_spans`, `max_visible_rows` clamped to 1, an
   empty tab list disabling tabs — struct fields that change what the user
   sees, which no candidate pointed at.
4. **Render geometry** (in the 8 imprecise rows): arrow rows, panel gap,
   height sums.

Kinds 2 and 3 are input gaps; kind 1 is a prompt gap. Already fixed in
`extract.py` (v2, `s14/candidates-v2.csv`, 200 candidates): a `field` kind
for public struct fields (70 for S14) and `#arm<k>/<n>` on key arms so the
describer states what earlier arms shadow. Kind 2 needs the screen's file set
to include its host's routing (`bottom_pane/mod.rs` for bottom-pane views,
the chat widget for chat parts) and the helper modules it calls — the
registry's job (`port-refs`).

Useful by-products: both models flagged rules no oracle test pins (the
filter keeps the visible row number, not the item; `on_selection_changed`
fires at construction when nothing is preselected; the shortcut path searches
all items, not the active tab's). A porter writes a test for those from the
code.

## Recommendation for the full run (round 1; see the conclusion at the top)

1. **One more round on S14 before scaling** (~130k Haiku + a judge): v2
   candidates, the file set with `bottom_pane/mod.rs` and the selection
   helpers, the prompt asking for precedence per arm. Pass: full + partial
   ≥ 90 % against the same reference, 0 wrong. Fail: Sonnet maps everywhere,
   Haiku not used for this.
2. **If it passes**, a W1 task `behaviour-map` beside `port-refs`, before
   the slice briefs: per screen, script → Haiku → validator → a **Sonnet gap
   pass** (reads the files and the map, adds missing rules; measure whether it
   is cheaper than a full Sonnet map on the first screens). No full precision
   check — the judge found 0 wrong rows — but a 10 % sample per screen.
3. **Sonnet maps for the risky screens**: composer, transcript cells and
   streaming, owned transcript, approvals, pager, inline mode — behaviour
   there spans modules, where the pilot shows Haiku is weakest.
4. **Where the map goes in the process**: `refs/behaviours/<screen>.csv`.
   Step 1 of process.md §3 becomes "take your screen's rows, check them
   against the oracle, add what is missing"; the parity pack maps rules to
   tests (ported / new for an unpinned rule / n/a); `parity-sweep` in W4
   checks every rule is covered. The oracle's code and tests stay the
   source: where the map disagrees, the row is corrected.
5. Running ~25 screens is a multi-agent run; as a Workflow it needs the
   person's explicit opt-in.
