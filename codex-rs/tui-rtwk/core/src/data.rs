//! `data` (ADR-013): plain domain values shared by the kit, the screens and the widgets: what a
//! mention is and what its popup lists, the tasks and picks it names, what a file search asks
//! and answers, and what the history file said. Pure values over std, protocol data and `ids`:
//! no logic of a screen, no draw state, no other module of core.

pub mod files;
pub mod history;
pub mod links;
pub mod mention;
pub mod task;
pub mod usage;
