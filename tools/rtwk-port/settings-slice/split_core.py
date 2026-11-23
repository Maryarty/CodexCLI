#!/usr/bin/env python3 -I
"""Splits the old `core/src/screens/settings.rs` (read from git, so the script reruns) into the
slice's `catalog.rs` (the types of the dialogs) and `seed.rs` (the script seeds); `mod.rs` (the
feature's marker, state and steps) is written by hand. Run from the repo root."""
import subprocess

OLD = "codex-rs/tui-rtwk/core/src/screens/settings.rs"
OUT = "codex-rs/tui-rtwk/core/src/features/settings/"
BASE = subprocess.run(
    ["git", "merge-base", "HEAD", "land/batch-9"], capture_output=True, text=True, check=True
).stdout.strip()
text = subprocess.run(
    ["git", "show", f"{BASE}:{OLD}"], capture_output=True, text=True, check=True
).stdout
lines = text.split("\n")


def span(first, last):
    """Lines first..last of the old file, 1-based inclusive."""
    return "\n".join(lines[first - 1 : last]) + "\n"


catalog = (
    "//! The catalogues the settings dialogs are offered and the edits they ask for (oracle\n"
    "//! `config_panel`, `theme_picker`, `status_line_setup`, `skills_toggle`, `keymap_picker`,\n"
    "//! `experimental_features`, `memories_settings`): data the host and the server gave, never\n"
    "//! what is drawn of them (cursor, checks, order: the sheets' drafts).\n\n"
    "use std::collections::{BTreeMap, BTreeSet};\n\n"
    "use super::hooks::Hooks;\n\n"
    + span(28, 636)
    + "\n"
    + span(651, 720)
    + "\n"
    + span(1000, 1007)
    + "\n"
    + span(1097, 1148)
)
catalog = catalog.replace("    fn theme_picker(&self)", "    pub fn theme_picker(&self)")
catalog = catalog.replace("struct CatalogueItem", "pub(super) struct CatalogueItem")
catalog = catalog.replace("    id: String,\n    description: String,\n    placeholder: String,",
                          "    pub(super) id: String,\n    pub(super) description: String,\n    pub(super) placeholder: String,")
catalog = catalog.replace("\nfn setup(", "\npub(super) fn setup(")
catalog = catalog.replace("/// The settings dialog open now.\n", "")
catalog = catalog.replace("src/sheets/settings/draft.rs", "src/features/settings/draft.rs")
catalog = catalog.replace("/// (`src/sheets/settings/draft.rs`)", "/// (`src/features/settings/draft.rs`)")
with open(OUT + "catalog.rs", "w") as out:
    out.write(catalog)

seed = (
    "//! The seed forms of the feature's facts, for the scripts' `fact` step (`Settings::SEED`).\n\n"
    "use std::collections::BTreeMap;\n\n"
    "use super::catalog::{\n"
    "    CatalogueItem, ConfigPanel, Experimental, Facts, Feature, FeaturesStatus, KeymapPicker,\n"
    "    KeymapTab, Memories, MemoryFacts, Row, Setup, SetupItem, SetupKind, ThemePicker, setup,\n"
    "};\n"
    "use super::hooks::{Hook, HookIssue, Hooks};\n"
    "use super::{Fact, Offer};\n"
    "use crate::screens::SeedError;\n\n"
    + span(1009, 1095)
    + "\n"
    + span(1150, 1257)
)
seed = seed.replace("Result<Ev, crate::screens::SeedError>", "Result<Fact, SeedError>")
seed = seed.replace("crate::screens::SeedError::Bad", "SeedError::Bad")
seed = seed.replace("Ok(Ev::Offered(match seed", "Ok(Fact::Offered(match seed")
seed = seed.replace("return Ok(Ev::FeaturesDiscovered {", "return Ok(Fact::FeaturesDiscovered {")
seed = seed.replace("return Ok(Ev::Facts(Facts {", "return Ok(Fact::Facts(Facts {")
seed = seed.replace("/// A script's `screen` step for this directory: the dialog `value` opens.",
                    "/// A script's `fact` step for the feature: the fact `value` makes.")
seed = seed.replace("/// The seed forms of this screen's events (`kind`)", "/// The seed forms of the feature's facts (`kind`)")
seed = seed.replace("(`Ev::Facts`)", "(`Fact::Facts`)")
with open(OUT + "seed.rs", "w") as out:
    out.write(seed)
print("catalog.rs", catalog.count("\n"), "seed.rs", seed.count("\n"))
