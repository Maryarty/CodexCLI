//! The writes of the config file the host runs for `Ask::ConfigWrite` (oracle `config_update.rs`:
//! `write_config_batch`): `config/batchWrite` of the edits a feature asked for, and how the
//! server's answer reads.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit as WireEdit;
use codex_app_server_protocol::ConfigWriteResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::RequestId;

use crate::app::effects::ConfigEdit;

/// `config/batchWrite` of `edits`: the values replace, the user's config is reloaded.
pub fn config_write(edits: &[ConfigEdit], id: RequestId) -> ClientRequest {
    ClientRequest::ConfigBatchWrite {
        request_id: id,
        params: ConfigBatchWriteParams {
            edits: edits
                .iter()
                .map(|edit| WireEdit {
                    key_path: edit.key_path.clone(),
                    value: edit.value.clone(),
                    merge_strategy: MergeStrategy::Replace,
                })
                .collect(),
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// The server's answer to a write: saved, or the text of why the answer is none.
pub fn config_written(result: serde_json::Value) -> Result<(), String> {
    serde_json::from_value::<ConfigWriteResponse>(result)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_replaces_every_edit_and_reloads_the_user_config() {
        let edits = [ConfigEdit {
            key_path: "tui.theme".to_owned(),
            value: serde_json::json!("nord"),
        }];
        let ClientRequest::ConfigBatchWrite { params, .. } =
            config_write(&edits, RequestId::Integer(1))
        else {
            panic!("not a batch write");
        };
        assert_eq!(params.edits.len(), 1);
        assert_eq!(params.edits[0].key_path, "tui.theme");
        assert_eq!(params.edits[0].merge_strategy, MergeStrategy::Replace);
        assert!(params.reload_user_config);
    }

    #[test]
    fn an_answer_that_is_no_write_response_is_an_error() {
        assert!(config_written(serde_json::json!("nope")).is_err());
    }
}
