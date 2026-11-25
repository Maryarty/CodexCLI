//! The RPCs of the provider list (oracle `app/provider/rpc.rs`): the catalog, a write of one
//! provider's settings and the default for new chats.

use crate::rpc::ServerError;
use codex_app_server_protocol::{
    ConfigValueWriteParams, ConfigWriteResponse, MergeStrategy, ModelProviderConfigWriteParams,
    ModelProviderConfigWriteResponse, ModelProviderListParams, ModelProviderListResponse,
    ModelProviderSummary, WriteStatus,
};

/// What an ask answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    List(ModelProviderListResponse),
    Saved(ModelProviderSummary),
    /// A write failed in a way that can follow a committed save (the write of a new gateway: an
    /// internal error of the server or a lost reply; the choice of a default: a lost reply):
    /// the text of the failure. The protocol's reading of the refusal (`unconfirmed`), never a
    /// decode.
    Unconfirmed(String),
    /// The server does not know `modelProvider/list` (`unsupported`).
    Unsupported,
    Chosen(Chosen),
}

/// How `config/value/write` of `model_provider` ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    Written,
    /// A higher-priority setting overrides the choice; the server's reason, if it gave one.
    Overridden(Option<String>),
}

crate::rpc_list! {
    /// What the feature asks the server.
    pub enum Command => Reply;
    /// `modelProvider/list`.
    List, Self::List
        => ModelProviderList(ModelProviderListParams {})
        -> ModelProviderListResponse, |answer| Reply::List(answer);
    /// `modelProvider/config/write`.
    Write(ModelProviderConfigWriteParams), Self::Write(params)
        => ModelProviderConfigWrite(params.clone())
        -> ModelProviderConfigWriteResponse, |answer| Reply::Saved(answer.provider);
    /// `config/value/write` of `model_provider`: the default for new chats.
    SetDefault(String), Self::SetDefault(id)
        => ConfigValueWrite(ConfigValueWriteParams {
            key_path: "model_provider".into(),
            value: id.as_str().into(),
            merge_strategy: MergeStrategy::Replace,
            file_path: None,
            expected_version: None,
        })
        -> ConfigWriteResponse, |answer| Reply::Chosen(chosen(answer));
}

fn chosen(answer: ConfigWriteResponse) -> Chosen {
    match answer.status {
        WriteStatus::Ok => Chosen::Written,
        WriteStatus::OkOverridden => {
            Chosen::Overridden(answer.overridden_metadata.map(|metadata| metadata.message))
        }
    }
}

/// What a refusal of the server means for `command`, when it is an answer and not a failure
/// (oracle `rpc::request_error`, `write_provider`, `write::Write::run`): the list the server
/// does not know, a write whose reply may have been lost after the server committed it.
pub fn refusal(command: &Command, error: &ServerError) -> Option<Reply> {
    match command {
        Command::List if unsupported(error) => Some(Reply::Unsupported),
        // A lost reply or a failed read-back can follow a committed write.
        Command::Write(params)
            if super::form::creates(params) && matches!(error.code, None | Some(-32603)) =>
        {
            Some(Reply::Unconfirmed(error.message.clone()))
        }
        // The server may have committed the write before its reply was lost.
        Command::SetDefault(_) if error.code.is_none() => {
            Some(Reply::Unconfirmed(error.message.clone()))
        }
        _ => None,
    }
}

/// The server does not have the list (`-32601`, or the request refused as an unknown variant).
fn unsupported(error: &ServerError) -> bool {
    error.code == Some(-32601)
        || (error.code == Some(-32600)
            && error.message.starts_with(
                "modelProvider/list failed: Invalid request: unknown variant `modelProvider/list`",
            ))
}
