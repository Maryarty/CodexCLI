# Batch review fix round 1: startup-deltas (FULL, Sonnet), on 451d7698d

Verdict: FIX. Only one item needs fixing, the documentation row for G51. The code of all 12 items follows the oracle.

## Items
- F1: done. The trust cursor is the shared Options controller, and Up/Down do not wrap. There is no separate red commit.
- F2: done. The height math is gone. Three snaps were green before F2 and are now pending(G51): long_checkout_40x13, long_repository_root_40x17 and only_repository_root_fits_40x16. That is a regression, and it is acceptable only if G51 is real. There is no red commit.
- N-O2, N-O1 and deltas 1-8 follow the oracle:
  - delta 6 matches cli/src/main.rs:805-889;
  - delta 4 matches lib.rs:2046;
  - delta 2 is R1-clean;
  - delta 7 is core/src/app.rs:709;
  - delta 8 covers the main home only.

## ADR / layers
ADR-003 is kept (startup state goes through apply; awaits_facts is UI-only). Core is pure; I/O happens only in services/startup.rs; R15 is clean.

## G51 (FIX)
G51 is plausibly real: RenderFlex (rtwk-render flex.rs:1029-1052) gives the remainder to the first flex children. But the row names only route 1, with no file:line.
- Route 2 is not tried: a panel-level LayoutBuilder could choose between prebuilt column variants by the measured height.
- There is no grep of src/widgets or src/pages/agents.
- Route 3 is not stated.
- The row should say that the oracle itself does reserved-row arithmetic (trust_directory.rs:190-200).
- trust.rs path() measures with measure::grapheme_rows and center_truncate_path inside a LayoutBuilder. These are carried helpers, and no gap row names the missing "max-lines + middle-ellipsis by segments" Text.

## Risks
1. host/src/startup.rs:278 calls tokio::task::spawn_blocking in the host. The R1 rule test (tests/rules/rules.rs:144-160) greps for "spawn(" and so misses it.
2. import_is_a_step_of_the_startup_page is flaky under load (task flaky-apply-wait).
3. The update offer never appears: the version is CARGO_PKG_VERSION = 0.0.0 (host/src/startup.rs:132, :361). The oracle's source build behaves the same way. The pack's wording "as the oracle's debug build" is wrong.
4. Delta 3 only refreshes the status line: Startup.config is not replaced, and the provider-change restart (oracle lib.rs:2079-2110) is missing.
5. Delta 5: `running` is not reset on Disconnected.

## Notes
- The pack's Checks section is stale.
- core/src/screens/onboarding.rs carries an unrelated rustfmt reflow.
- The four not-covered items (agents/resume flash, login-time reload, provider-change restart, dev builds) are recorded in prose in startup-flow.md:82-86, with no task alias.
