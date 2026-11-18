# S19 spec.md: simplicity review

spec.md is 699 lines (S14: 294 lines, 13 UCs). Line numbers below are spec.md.

## Cuts, by section

| Section | Lines | Verdict |
|---|---|---|
| Intro / terms | 1-25 | keep, 10 lines enough; L19 "a fifth of rows plumbing" is wrong (14/112) and not needed |
| 1 API code block | 28-114 (87) | over-detailed: the 3-4 Intent/Fx/Event types and one snapshot struct suffice (~35 lines); per-field comments repeat the rules (e.g. thread usage micros, ticket types) |
| 1 "How it fits" | 116-137 | duplicates Rules 5, 6, 9; cut, ~20 lines saved |
| 2 UC-01..UC-21 | 139-367 (229) | 21 UCs for 4 behaviours (card, refresh, thread usage, remote). Merge UC-03/04/05 (one refresh lifecycle), UC-12/15/16 (one settlement flow), UC-13/Rule 2, UC-19/3.4, UC-20/3.2; ~110 lines |
| 3.1 / 3.2 | 374-393, 395-438 | 3.2 permission tables (418-438): carried pure function (`status_permissions_*`); replace by pointer + 4 examples (and fix the ":" ids) |
| 3.3 / 3.4 | 440-463 | one-line pointers (compact tokens, directory shortening are carried pure code) |
| 3.5 / 3.6 | 465-542 (78) | formatting algorithms (bars, credit amounts, breakdown rounding): carried pure code; keep 6 example lines each; ~60 lines saved |
| Rules | 544-607 (64) | 15 rules; rules 1, 3, 7, 10, 14 restate UCs. Keep 2, 4, 5, 6, 8, 9 (fixed), 11-13 as one-liners |
| 4 acceptance | 609-629 | keep, correct A-01..A-03 |
| 5 not carried | 631-647 | keep, but remove rows cited as rules (C-09) |
| 6 open points | 649-699 (51) | 12 points; OP1, 3, 6, 9, 12 are one-liners or decided, OP2 is wrong (C-01); keep 3-4 real ones |

Oracle mechanics a porter does not need: `refreshing_status_outputs` map and
request ids (L116-137, Rule 5 tail), `status_history_handles` list internals
(UC-15), `Arc<RwLock>` handle (L50-55), 65 s timeout, wrapping and value-column
arithmetic (Rule 11, 3.1 L391-393), hyperlink fragment detection (Rule 12).

## Ambiguous statements (13)

1. "account is ChatGPT": three different flags (`has_chatgpt_account` for the
   prefetch, `StatusAccountDisplay::ChatGpt` for the Token usage row,
   `has_codex_backend_auth`); say which one each rule uses.
2. "first-party-auth provider": undefined; code predicate is
   `provider_uses_first_party_auth_path` (requires_openai_auth, no env key, no
   bearer token, no custom auth, no aws).
3. `ThreadFx::ReadThreadUsage { ticket, card }`: per card, oracle has one
   thread-wide pending read with a list of waiting cards.
4. "cache order": BTreeMap ascending limit id, replace per id.
5. "frozen" card vs billing plan: which rows freeze, which update.
6. UC-20 host paths and `../` vs `..` + separator.
7. Raw "same lines" (see C-07).
8. Billed-token sums include negative-credit groups: say so or exclude.
9. Rule 8 "other thread": two statements that read against each other (C-06).
10. Rule 4 "status override" (C-04).
11. "selected" in OP4 vs UC-16 (C-03).
12. "refreshing" for a card with fresh cached rows (A-01).
13. "stale" wording: strict `>` 15 minutes on the oldest of rolling and monthly
    snapshots; spec says "older than 15 minutes" without the boundary.

## Length estimate

A spec of the S14 kind (API ~35 lines, ~10 merged UCs at ~10 lines, a behaviour
section with example-only tables ~80, acceptance ~25, not carried ~15, open ~15):
**330-400 lines**, about half of the present 699. Biggest cut: the formatting
algorithms and label tables of 3.2-3.6 (L395-542, ~150 lines, replaced by pointers
to the carried functions with examples) together with the API block trimming
and merged UCs (a further ~150-200).
