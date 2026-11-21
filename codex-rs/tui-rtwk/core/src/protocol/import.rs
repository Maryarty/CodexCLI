//! The import flow's protocol mapping: the requests the shell sends
//! (`externalAgentConfig/detect` per source, `externalAgentConfig/import`)
//! and the pure reading of what came back. Oracle:
//! `external_agent_config_migration/flow.rs` (the detect params),
//! `app_server_session/external_agent_config.rs` (the import params and the
//! import id the completion is matched with).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;
use codex_app_server_protocol::RequestId;
use serde_json::Value;

use crate::rpc::RpcError;
use crate::screens::import::Source;

/// `externalAgentConfig/detect` of `source`: the home and the project
/// directory `cwd`.
pub fn detect(source: Source, cwd: Option<&str>, request_id: RequestId) -> ClientRequest {
    let _ = (source, cwd, request_id);
    todo!("detect")
}

/// The items `externalAgentConfig/detect` found.
pub fn found(result: Value) -> Result<Vec<Item>, RpcError> {
    let _ = result;
    todo!("found")
}

/// `externalAgentConfig/import` of `items` found by `source` (its id).
pub fn import(source: &str, items: Vec<Item>, request_id: RequestId) -> ClientRequest {
    let _ = (source, items, request_id);
    todo!("import")
}

/// The id of the import `externalAgentConfig/import` started: its completion
/// notice carries it.
pub fn imported(result: Value) -> Result<String, RpcError> {
    let _ = result;
    todo!("imported")
}
