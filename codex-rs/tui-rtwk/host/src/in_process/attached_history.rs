//! Local attached-history IO. Each job runs on one exact request handle.

use std::collections::{BTreeSet, HashSet};
use std::future::Future;
use std::path::Path;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol as wire;
use codex_tui_core::app::{
    AttachedHistory, LocalSideStorage, PreparationCheckpoint, SavedAttachment,
};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::threads::ThreadAsk;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{Ids, saved_checkpoint, server_error};

pub(super) async fn run(
    parent: &ThreadId,
    storage: LocalSideStorage,
    work: &AttachedHistory,
    handle: &AppServerRequestHandle,
    ids: &Ids,
) -> Result<Value, ServerError> {
    let send = |ask: ThreadAsk| async move {
        handle
            .request_typed::<Value>(ask.wire(ids.next()))
            .await
            .map_err(|error| server_error(true, &error))
    };
    execute(parent, storage, work, &send).await
}

async fn execute<F: Future<Output = Result<Value, ServerError>>>(
    parent: &ThreadId,
    storage: LocalSideStorage,
    work: &AttachedHistory,
    send: &impl Fn(ThreadAsk) -> F,
) -> Result<Value, ServerError> {
    if storage == LocalSideStorage::Daemon {
        let supported = match send(ThreadAsk::ProbeAttachedSide {
            thread: parent.clone(),
        })
        .await
        {
            Err(error) if error.code.is_some() => {
                error.code == Some(-32600)
                    && error.message
                        == "attachedToThreadId cannot be combined with parentThreadId or ancestorThreadId"
            }
            Err(error) => return Err(error),
            Ok(_) => false,
        };
        if !supported {
            return Ok(Value::Null);
        }
    }
    match work {
        AttachedHistory::Discover => discover(parent, send).await,
        AttachedHistory::Verify { child } | AttachedHistory::Archive { child } => {
            let thread = read(child, send).await?;
            if thread.id != child.as_str() || attestation(parent, &thread).await.is_none() {
                return Ok(Value::Bool(false));
            }
            if matches!(work, AttachedHistory::Archive { .. }) {
                send(ThreadAsk::Archive {
                    thread: child.clone(),
                })
                .await?;
            }
            Ok(Value::Bool(true))
        }
    }
}

async fn read<F: Future<Output = Result<Value, ServerError>>>(
    thread: &ThreadId,
    send: &impl Fn(ThreadAsk) -> F,
) -> Result<wire::Thread, ServerError> {
    let result: wire::ThreadReadResponse = decode(
        send(ThreadAsk::Read {
            thread: thread.to_string(),
        })
        .await?,
    )?;
    Ok(result.thread)
}

async fn discover<F: Future<Output = Result<Value, ServerError>>>(
    parent: &ThreadId,
    send: &impl Fn(ThreadAsk) -> F,
) -> Result<Value, ServerError> {
    let metadata = read(parent, send).await?;
    if metadata.id != parent.as_str()
        || metadata.ephemeral
        || metadata.history_mode != wire::ThreadHistoryMode::Paginated
    {
        return Ok(Value::Null);
    }
    let Some(threads) = candidates(parent, send).await? else {
        return Ok(Value::Null);
    };
    let named_ids = threads
        .iter()
        .filter(|thread| thread.name.is_some())
        .filter_map(|thread| codex_protocol::ThreadId::from_string(&thread.id).ok())
        .collect::<HashSet<_>>();
    let codex_home = metadata
        .path
        .as_deref()
        .and_then(|path| {
            path.ancestors()
                .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "sessions"))
        })
        .and_then(Path::parent);
    let names = match codex_home {
        Some(home) => codex_rollout::find_thread_name_entries_by_ids(home, &named_ids)
            .await
            .unwrap_or_default(),
        None => Default::default(),
    };
    let mut saved = Vec::new();
    for thread in threads {
        let Some(checkpoint) = attestation(parent, &thread).await else {
            continue;
        };
        let Ok(protocol_id) = codex_protocol::ThreadId::from_string(&thread.id) else {
            continue;
        };
        let id = ThreadId::new(thread.id);
        let name = thread.name.filter(|name| {
            let Some(entry) = names.get(&protocol_id) else {
                return true;
            };
            if entry.thread_name == *name
                && let Ok(indexed) = chrono::DateTime::parse_from_rfc3339(&entry.updated_at)
                && let Ok(prepared) = chrono::DateTime::parse_from_rfc3339(&checkpoint)
            {
                return indexed > prepared;
            }
            true
        });
        let preview = if name.is_none() {
            first_prompt(&id, send).await.unwrap_or_default()
        } else {
            String::new()
        };
        saved.push(SavedAttachment { id, name, preview });
    }
    serde_json::to_value(saved).map_err(|error| error.to_string().into())
}

async fn candidates<F: Future<Output = Result<Value, ServerError>>>(
    parent: &ThreadId,
    send: &impl Fn(ThreadAsk) -> F,
) -> Result<Option<Vec<wire::Thread>>, ServerError> {
    let mut cursor = None;
    let mut cursors = BTreeSet::new();
    let mut known_ids = BTreeSet::new();
    let mut threads = Vec::new();
    loop {
        let result = send(ThreadAsk::ListAttachedSide {
            parent: parent.clone(),
            cursor,
        })
        .await;
        let page: wire::ThreadListResponse = match result {
            Err(error)
                if error.code == Some(-32601)
                    && error.message == "attached threads is not supported yet" =>
            {
                return Ok(None);
            }
            result => decode(result?)?,
        };
        threads.extend(
            page.data
                .into_iter()
                .filter(|thread| known_ids.insert(thread.id.clone())),
        );
        let Some(next) = page.next_cursor else {
            return Ok(Some(threads));
        };
        if !cursors.insert(next.clone()) {
            return Err("Repeated side chat list cursor".into());
        }
        cursor = Some(next);
    }
}

async fn attestation(parent: &ThreadId, thread: &wire::Thread) -> Option<String> {
    if thread.ephemeral
        || thread.id == parent.as_str()
        || thread.history_mode != wire::ThreadHistoryMode::Paginated
        || thread.attached_to_thread_id.as_deref() != Some(parent.as_str())
    {
        return None;
    }
    let id = ThreadId::new(&thread.id);
    let checkpoint = PreparationCheckpoint {
        parent: parent.clone(),
        rollout: thread
            .path
            .as_ref()
            .map(|path| path.as_path().to_path_buf()),
    };
    match saved_checkpoint(&id, &checkpoint).await {
        Ok(time) => time,
        Err(error) => {
            tracing::warn!(thread = %id, %error, "failed to read side chat context");
            None
        }
    }
}

async fn first_prompt<F: Future<Output = Result<Value, ServerError>>>(
    thread: &ThreadId,
    send: &impl Fn(ThreadAsk) -> F,
) -> Result<String, ServerError> {
    let mut cursor = None;
    let mut cursors = BTreeSet::new();
    loop {
        let page: wire::ThreadItemsListResponse = decode(
            send(ThreadAsk::FirstAttachedPrompt {
                thread: thread.clone(),
                cursor,
            })
            .await?,
        )?;
        if let Some(prompt) = page
            .data
            .iter()
            .find_map(|entry| match &entry.item.payload {
                wire::ThreadItemPayload::UserMessage { content, .. } => prompt_preview(content),
                _ => None,
            })
        {
            return Ok(prompt.to_owned());
        }
        let Some(next) = page.next_cursor else {
            return Ok(String::new());
        };
        if !cursors.insert(next.clone()) {
            return Err("Repeated side chat item cursor".into());
        }
        cursor = Some(next);
    }
}

fn prompt_preview(items: &[wire::UserInput]) -> Option<&str> {
    if let [
        wire::UserInput::Text {
            text,
            text_elements,
        },
    ] = items
        && text_elements.is_empty()
        && text.trim().starts_with("<realtime_delegation>")
        && text.trim().ends_with("</realtime_delegation>")
    {
        return None;
    }
    let text = items.iter().find_map(|item| match item {
        wire::UserInput::Text { text, .. } => {
            let text = codex_protocol::protocol::strip_user_message_prefix(text);
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    });
    text.or_else(|| {
        items.iter().find_map(|item| match item {
            wire::UserInput::Image { .. } | wire::UserInput::LocalImage { .. } => Some("[Image]"),
            wire::UserInput::Audio { .. } | wire::UserInput::LocalAudio { .. } => Some("[Audio]"),
            _ => None,
        })
    })
    .and_then(|text| text.lines().next())
    .map(str::trim)
}

fn decode<R: DeserializeOwned>(value: Value) -> Result<R, ServerError> {
    serde_json::from_value(value).map_err(|error| error.to_string().into())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::future::ready;
    use std::io::Write as _;
    use std::path::Path;

    use codex_protocol::protocol::SessionMeta;
    use serde_json::json;

    use super::*;

    fn row(
        id: codex_protocol::ThreadId,
        parent: Option<codex_protocol::ThreadId>,
        path: Option<&Path>,
        name: Option<&str>,
    ) -> Value {
        json!({
            "id":id, "sessionId":id, "attachedToThreadId":parent, "forkedFromId":parent,
            "ephemeral":false, "historyMode":"paginated", "status":{"type":"idle"},
            "cwd":"/tmp", "modelProvider":"openai", "cliVersion":"0.0.0", "source":"cli",
            "createdAt":1,"updatedAt":1,"recencyAt":1,"preview":"inherited parent preview",
            "name":name,"path":path,"turns":[],"environments":null,"extra":null,
        })
    }

    fn write_checkpoint(
        path: &Path,
        id: codex_protocol::ThreadId,
        parent: codex_protocol::ThreadId,
    ) -> anyhow::Result<()> {
        let meta = SessionMeta {
            id,
            session_id: id.into(),
            forked_from_id: Some(parent),
            attached_thread: true,
            ..SessionMeta::default()
        };
        let record = |kind, ordinal, payload| {
            json!({
                "timestamp":"2026-11-27T00:00:00.000Z","ordinal":ordinal,"type":kind,"payload":payload,
            })
        };
        let message = |role, ordinal, text| {
            record(
                "response_item",
                ordinal,
                json!({
                    "type":"message","role":role,"content":[{"type":"input_text","text":text}],
                }),
            )
        };
        let lines = [
            record("session_meta", 0, serde_json::to_value(meta)?),
            message(
                "developer",
                1,
                "<side_conversation>policy</side_conversation>",
            ),
            message(
                "user",
                2,
                "Side conversation boundary.\n\nHistory is context.",
            ),
        ]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
        std::fs::File::create(path)?.write_all(lines.join("\n").as_bytes())?;
        Ok(())
    }

    #[tokio::test]
    async fn attached_discovery_attests_deduplicated_pages_and_archive_rechecks_the_child()
    -> anyhow::Result<()> {
        let parent_id = codex_protocol::ThreadId::from_u128(1);
        let child_id = codex_protocol::ThreadId::from_u128(2);
        let unrelated = codex_protocol::ThreadId::from_u128(3);
        let parent = ThreadId::new(parent_id.to_string());
        let child = ThreadId::new(child_id.to_string());
        let dir = tempfile::tempdir()?;
        let actual = dir.path().join("child.jsonl");
        let other_rollout = dir.path().join("wrong.jsonl");
        write_checkpoint(&actual, child_id, parent_id)?;
        write_checkpoint(&other_rollout, unrelated, unrelated)?;
        let valid = row(child_id, Some(parent_id), Some(&actual), Some("Own Name"));
        let wrong = row(
            unrelated,
            Some(parent_id),
            Some(&other_rollout),
            Some("Wrong File"),
        );
        let reads = RefCell::new(VecDeque::from([
            (
                ThreadAsk::Read {
                    thread: parent.to_string(),
                },
                json!({"thread":row(parent_id,None,None,None)}),
            ),
            (
                ThreadAsk::ListAttachedSide {
                    parent: parent.clone(),
                    cursor: None,
                },
                json!({"data":[valid.clone()],"nextCursor":"second"}),
            ),
            (
                ThreadAsk::ListAttachedSide {
                    parent: parent.clone(),
                    cursor: Some("second".to_owned()),
                },
                json!({"data":[valid.clone(),wrong.clone()],"nextCursor":null}),
            ),
            (
                ThreadAsk::Read {
                    thread: child.to_string(),
                },
                json!({"thread":valid}),
            ),
            (
                ThreadAsk::Archive {
                    thread: child.clone(),
                },
                json!({}),
            ),
            (
                ThreadAsk::Read {
                    thread: unrelated.to_string(),
                },
                json!({"thread":wrong}),
            ),
        ]));
        let send = |ask| {
            let step = reads.borrow_mut().pop_front();
            ready(match step {
                Some((expected, result)) => {
                    assert_eq!(ask, expected);
                    Ok(result)
                }
                None => Err("unexpected request".into()),
            })
        };
        let rows = execute(
            &parent,
            LocalSideStorage::Embedded,
            &AttachedHistory::Discover,
            &send,
        )
        .await?;
        assert_eq!(
            decode::<Vec<SavedAttachment>>(rows)?,
            vec![SavedAttachment {
                id: child.clone(),
                name: Some("Own Name".to_owned()),
                preview: String::new(),
            }]
        );
        assert_eq!(
            execute(
                &parent,
                LocalSideStorage::Embedded,
                &AttachedHistory::Archive { child },
                &send
            )
            .await?,
            json!(true)
        );
        assert_eq!(
            execute(
                &parent,
                LocalSideStorage::Embedded,
                &AttachedHistory::Archive {
                    child: ThreadId::new(unrelated.to_string()),
                },
                &send
            )
            .await?,
            json!(false)
        );
        assert!(reads.borrow().is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn attached_discovery_refuses_an_ignored_daemon_filter_or_repeated_cursor()
    -> anyhow::Result<()> {
        let parent = ThreadId::new("parent");
        let unsupported = |ask| {
            assert!(matches!(ask, ThreadAsk::ProbeAttachedSide { .. }));
            ready(Ok(json!({"data":[],"nextCursor":null})))
        };
        assert_eq!(
            execute(
                &parent,
                LocalSideStorage::Daemon,
                &AttachedHistory::Discover,
                &unsupported
            )
            .await?,
            Value::Null
        );
        let repeats = |ask| {
            assert!(matches!(ask, ThreadAsk::ListAttachedSide { .. }));
            ready(Ok(json!({"data":[],"nextCursor":"repeated"})))
        };
        assert!(candidates(&parent, &repeats).await.is_err());
        Ok(())
    }
}
