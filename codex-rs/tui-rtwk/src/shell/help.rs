//! What `--help` and `--version` print: the shipped oracle's own text
//! (`codex` 0.170.0, captured from its binary at the width clap uses without
//! a terminal), for the top level and the `agents` subcommand. It is data
//! for the terminal, not UI; the host prints it before anything starts.

pub const VERSION: &str = include_str!("help/version.txt");
pub const TOP_LONG: &str = include_str!("help/top_long.txt");
pub const TOP_SHORT: &str = include_str!("help/top_short.txt");
pub const AGENTS_LONG: &str = include_str!("help/agents_long.txt");
pub const AGENTS_SHORT: &str = include_str!("help/agents_short.txt");
