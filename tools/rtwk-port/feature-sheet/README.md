# FeatureView + FeatureSheet split (fw-ui-reg-2)

`python3 -I tools/rtwk-port/feature-sheet/split.py [--check]` moves `type Args` / `fn args` /
`fn open` out of `impl FeatureView for X` of every `codex-rs/tui-rtwk/src/features/<f>/mod.rs`:
into `impl FeatureSheet for X` when core declares `Feature::OPENS` for the slice (the list is read
from `core/src/features/<f>/mod.rs`, not written in the script), else deleted (the
`Args = Never<Self>` / `.absurd()` stubs) with the imports they were the last user of. Rerunnable:
an impl without the three items is skipped. `--check` lists what it would change.
