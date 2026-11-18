//! Discovery over the oracle's two sources: the interactive threads and the
//! Exec / AppServer ones, merged by recency (oracle:
//! `agents_overview_discovery.rs`, `AgentsOverviewDiscovery::next_batch`).
//! The oracle retains each source's cursor and unshown rows in the app; here
//! the retained state is the page cursor the model already keeps, made of
//! both sources' cursors (`Cursors`), so core stays a pure function of the
//! replies: the shell sends `Fanout::requests` and hands the results back to
//! `Fanout::reply`. No IO.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use super::Command;
use super::Reply;
use crate::rpc::RpcError;

/// The requests one `List` or `Search` command stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fanout {}

/// The fan-out of `command`, when it is one: a page of the list or of a
/// history search. `cloud`: the backend's task list cannot be filtered to
/// Exec and AppServer threads, so only the interactive source is asked.
pub fn fanout(command: &Command, cloud: bool) -> Option<Fanout> {
    let _ = (command, cloud);
    todo!("sources")
}

impl Fanout {
    /// One `thread/list` per source that is not exhausted, each under an id
    /// from `ids`.
    pub fn requests(&self, ids: impl FnMut() -> RequestId) -> Vec<ClientRequest> {
        let _ = ids;
        todo!("sources")
    }

    /// The merged page out of the results of `requests`, in the same order
    /// (a JSON result or the client's failure text). A source that failed
    /// fails the page: the retry asks both again from the same cursor.
    pub fn reply(
        &self,
        results: Vec<Result<serde_json::Value, String>>,
    ) -> Result<Reply, RpcError> {
        let _ = results;
        todo!("sources")
    }
}
