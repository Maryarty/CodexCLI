# Core refactor plan — merged from core-review a/b/c (orch16, 2026-11-24)

Sources: `core-review-a.md` (hubs: F*), `core-review-b.md` (features, keymap, kit: K/H/S/P/D/N),
`core-review-c.md` (screens, data: D/R/N, prefixed c-). Ranked by payoff / cost; "clash" = a
running or next W2 package that holds the files; "rule" = how the fix is held by a rules test.

## Tier A — defects and rule holes; no clash, do now beside W2
| # | Items | What | Lines | Clash | Rule |
|---|---|---|---|---|---|
| A1 | F1 | one connection lost/up path; a reconnect answers every in-flight ask (/plugins loads forever today) | ~150 | none (task conn-lost-path) | no `_` arm over AppCtx; behaviour test per variant |
| A2 | c-D1, c-D2, c-R7, b-D1, b-N1, F12, b-D5 | dead outboxes = user-visible defects: Ctrl+O in /warnings copies nothing; a /pets pick is neither saved nor shown; approvals::Model a second dead request store; settings `edits` grows forever; 8 dead pub fns; feedback sends a real RPC as a failure sentinel (wire -> Result) | ~300 | dialogs `note` touches w2-commands' feedback: leave that field | core state field read only by tests is refused |
| A3 | b-H1, b-H2, b-H3, b-H4, b-K3, c-R3, c-R4, F9 | widen the rules that pass by name today: prose (37 multi-line sites), slice reading another slice's ThreadPart, unlisted thread RPCs, hand-written RPC count, ActionId::new outside tables, ADR-013 draw/gesture names, string ids (20+ `thread: String`), fixture-only Seed events | ~250 tests | none (tests/rules only) | each IS a rule; ratchets at today's counts, so nothing turns red |
| A4 | c-R1, c-R2 | /usage tab+metric and the pager open state sit in core (ADR-013 holes A3 exposes): debt tasks, fixed by the owner of the next touch | ~150 | w2-status (usage), w2-transcript (pager) | A3's widened rule ratchets them to 0 |

## Tier B — performance (perf must beat the oracle); right after W2 lands
| B1 | F2, F3, c-D3 | copy-on-write: the shown ThreadModel and every screen model are deep-copied each frame / clock tick; global passes after each streamed delta. `Shared<>` per slot (the ScreenStates pattern), one clock, + the ADR-003 allocation budget test | ~300 | threads.rs, screens/mod.rs, app/features.rs (hubs) | allocation-budget test (ADR-003 promised it) |
| B2 | b-K1, b-K2 | per keystroke: linear scan of 110 actions + SipHash + Vec (~70 allocs on an unbound Vim key; oracle none); /keymap picker O(n^2) | ~200 | keymap/fixed.rs (w2-commands adds actions), vim (composer) | bench/alloc test per key |
| B3 | b-P1, b-P2, b-P3, c-N4 | Ctrl+R re-decodes history per key; vim elements per candidate; plugin tabs rebuilt per frame; resume filters 6x per change | ~200 | composer (P1, P2) | none |

## Tier C — seams (why parallel packages collide); between W2 and W3, before W3 briefs
| C1 | b-S1, b-S2, F6 | OPENS with gate + args + NEEDS: per-feature cases leave commands/dispatch.rs; the command table derived, not hand-edited | ~250 | after w2-status + w2-commands | ratchet on dispatch.rs arms |
| C2 | c-R11, c-R12, F4, F5, c-N5 | one `AppFx::ask` door with generated request ids (5 ways to send work, 12 hand counters); thread RPCs in one place; per-screen host channels out of the hub Command (each costs arms in protocol, host and fake.rs) | ~600 | hubs: app.rs, protocol, fake.rs | ratchets on Command variants / counters |
| C3 | F8, c-N2, c-R10, F7, c-R5 | split app.rs `apply_event` (620 lines) and threads.rs into connection/rates/ingress/transcript/asks; generated screen observe; select to half-slices (closes c-R9: notes land in the wrong thread after a switch); routing out of the agents screen | ~1500 moved | every package touching app.rs/select | `arms` budgets per hub |
| C4 | b-H5, c-N1, b-S3, b-N3, N2-doc | `feature!` everywhere, empty-half macros for screens, host_feed_list generated, fixed keymap table per area, ADR-012 recipe fixed | ~400 | low | registry tests |

## Tier D — words and types out of core (ADR-010); swept as families are touched
b-K4, b-D2, b-D3, b-D6, F10, c-N6, c-N7, b-D4 (RpcError::server_text), b-D7, F11, c-R8, c-R13, c-R14, N3:
held by A3's prose and id ratchets, which only go down.

## Recommendation
A1-A4 now (3 small packages beside W2, no shared files), B1 right after W2 lands, then C1-C4 as one
"seams wave" before W3 briefs are written (W3 then pre-assigns doors instead of hub edits), B2/B3
and D folded into whichever package next owns the files.
