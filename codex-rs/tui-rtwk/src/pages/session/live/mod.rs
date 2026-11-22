//! The live turn's own pieces of the chat page: what the model gives back to
//! the composer, and (as the chunk grows) the banners and rows the oracle's
//! `chatwidget` draws only while a turn runs.

pub(super) mod browse;
mod restore;

pub(super) use restore::put_back;
