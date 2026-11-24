//! The rules tests, one binary (`binary(rules)`): the static checks of the repository, each a
//! module of its own file. They read sources and manifests and never run the app, so six
//! binaries to build and link bought nothing (docs/rtwk-port/build-speed.md). A new rules
//! file is a new line here (`every_test_file_is_registered` fails without it).

mod adr013;
mod carried;
mod connection;
mod feature_sheet;
mod fixtures;
mod layers;
mod pin;
mod registration;
mod rules;
mod test_roots;
mod theme;
mod ui_registry;
mod wire;
