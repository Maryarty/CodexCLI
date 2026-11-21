//! The shell's tests (backend, connection, cycle, agents services), one binary: each file of
//! `tests/services/` is a module here (docs/rtwk-port/build-speed.md). A new service test file
//! is a line in this list (`every_test_file_is_registered` fails without it).

#[path = "services/shell_agents.rs"]
mod shell_agents;
#[path = "services/shell_backends.rs"]
mod shell_backends;
#[path = "services/shell_connection.rs"]
mod shell_connection;
#[path = "services/shell_cycle.rs"]
mod shell_cycle;
