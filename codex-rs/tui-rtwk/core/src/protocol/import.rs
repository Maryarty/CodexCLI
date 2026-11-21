//! The import flow's protocol mapping: the requests the shell sends
//! (`externalAgentConfig/detect` per source, `externalAgentConfig/import`)
//! and the pure reading of what came back. Oracle:
//! `external_agent_config_migration/flow.rs` (the detect params),
//! `app_server_session/external_agent_config.rs` (the import params and the
//! import id the completion is matched with).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ExternalAgentConfigDetectParams;
use codex_app_server_protocol::ExternalAgentConfigDetectResponse;
use codex_app_server_protocol::ExternalAgentConfigImportParams;
use codex_app_server_protocol::ExternalAgentConfigImportResponse;
use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;
use codex_app_server_protocol::RequestId;
use serde_json::Value;

use crate::rpc::RpcError;
use crate::screens::import::Source;

/// `externalAgentConfig/detect` of `source`: the home and the project
/// directory `cwd`.
pub fn detect(source: Source, cwd: Option<&str>, request_id: RequestId) -> ClientRequest {
    ClientRequest::ExternalAgentConfigDetect {
        request_id,
        params: ExternalAgentConfigDetectParams {
            include_home: true,
            cwds: cwd.map(|cwd| vec![cwd.into()]),
            max_session_age_days: None,
            max_sessions: None,
            source: None,
            migration_source: Some(source.id().to_owned()),
        },
    }
}

/// The items `externalAgentConfig/detect` found.
pub fn found(result: Value) -> Result<Vec<Item>, RpcError> {
    decode::<ExternalAgentConfigDetectResponse>(result).map(|response| response.items)
}

/// `externalAgentConfig/import` of `items` found by `source` (its id).
pub fn import(source: &str, items: Vec<Item>, request_id: RequestId) -> ClientRequest {
    ClientRequest::ExternalAgentConfigImport {
        request_id,
        params: ExternalAgentConfigImportParams {
            migration_items: items,
            source: Some("cli".to_owned()),
            provider_id: Some(source.to_owned()),
            migration_source: Some(source.to_owned()),
        },
    }
}

/// The id of the import `externalAgentConfig/import` started: its completion
/// notice carries it.
pub fn imported(result: Value) -> Result<String, RpcError> {
    decode::<ExternalAgentConfigImportResponse>(result).map(|response| response.import_id)
}

fn decode<T: serde::de::DeserializeOwned>(result: Value) -> Result<T, RpcError> {
    serde_json::from_value(result).map_err(|error| RpcError::Malformed(error.to_string()))
}
