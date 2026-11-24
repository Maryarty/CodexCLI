//! `rpc_list!` (ADR-012, "the wire is generated"): the RPCs of one feature as a list. Each line is
//! one RPC: the ask's variant, how the protocol request is made of it, the response the server
//! answers with and what the feature reads of it. The macro writes the ask enum, `request` (the
//! `ClientRequest`) and `reply` (the decode of the JSON result, a parse failure is
//! `RpcError::Malformed`), so a feature has no `wire.rs`: the protocol's method, params and
//! response types are named once, on one line, and nothing else matches an ask to a request.
//!
//! ```ignore
//! rpc_list! {
//!     /// The one ask.
//!     pub enum Command => Reply, request thread: &ThreadId;
//!     /// `thread/name/set`.
//!     SetName(String), Self::SetName(name)
//!         => ThreadSetName(ThreadSetNameParams { thread_id: thread.to_string(), name: name.clone() })
//!         -> ThreadSetNameResponse, |_| Reply::Ack;
//! }
//! ```
//!
//! The head names the enum, the reply type the list maps into (written by hand: it is what the
//! feature reads, and several asks may share a variant) and, when a line needs one, a context
//! binder for `request` (`request thread: &ThreadId` for RPCs about a thread) and one for `reply`
//! (`reply wanted: &Wanted`): the ask carries what the RPC is about, and a context is only what
//! the caller knows besides it. A line
//! is `Variant`, `Variant(fields)` or `Variant { fields }` as the enum has it, the pattern that binds the
//! fields (`Self::Variant(..)`), `=> Method(params)` (`ClientRequest::Method`'s `params`
//! expression, which sees the pattern's bindings and the binder), `-> Response` and the closure
//! from the parsed response to the reply (the same bindings are in scope).
//!
//! The protocol crates are not touched: no trait per method exists there, the list is ours.

/// The RPCs of one feature (see the module's text).
#[macro_export]
macro_rules! rpc_list {
    (
        $(#[$meta:meta])*
        $vis:vis enum $command:ident => $reply:ty
            $(, request $rctx:ident: $rty:ty)?
            $(, reply $dctx:ident: $dty:ty)?;
        $(
            $(#[$variant_meta:meta])*
            $variant:ident $(($($tuple:tt)*))? $({$($record:tt)*})?, $pat:pat
                => $method:ident($params:expr)
                -> $response:ty, |$answer:pat_param| $map:expr;
        )+
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        $vis enum $command {
            $($(#[$variant_meta])* $variant $(($($tuple)*))? $({$($record)*})?,)+
        }

        #[allow(unused_variables)]
        impl $command {
            /// The protocol request of the ask, under `request_id`.
            $vis fn request(
                &self,
                $($rctx: $rty,)?
                request_id: ::codex_app_server_protocol::RequestId,
            ) -> ::codex_app_server_protocol::ClientRequest {
                match self {
                    $($pat => ::codex_app_server_protocol::ClientRequest::$method {
                        request_id,
                        params: $params,
                    },)+
                }
            }

            /// What the JSON result of the ask says.
            $vis fn reply(
                &self,
                $($dctx: $dty,)?
                result: ::serde_json::Value,
            ) -> ::std::result::Result<$reply, $crate::rpc::RpcError> {
                match self {
                    $($pat => ::serde_json::from_value::<$response>(result)
                        .map(|$answer| $map)
                        .map_err(|error| $crate::rpc::RpcError::Malformed(error.to_string())),)+
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use codex_app_server_protocol::{
        ClientRequest, RequestId, ThreadGoalClearParams, ThreadGoalClearResponse,
    };

    use crate::rpc::RpcError;

    #[derive(Debug, PartialEq, Eq)]
    enum Reply {
        Cleared(bool),
    }

    rpc_list! {
        enum Probe => Reply, request thread: &str, reply scale: u8;
        Clear(u8), Self::Clear(origin)
            => ThreadGoalClear(ThreadGoalClearParams { thread_id: thread.to_owned(), origin: None })
            -> ThreadGoalClearResponse, |answer| Reply::Cleared(answer.cleared && *origin == scale);
    }

    #[test]
    fn a_line_makes_the_request_and_reads_the_reply() {
        let ClientRequest::ThreadGoalClear { request_id, params } =
            Probe::Clear(1).request("t1", RequestId::Integer(7))
        else {
            panic!("not a goal clear");
        };
        assert_eq!(
            (request_id, params.thread_id),
            (RequestId::Integer(7), "t1".to_owned())
        );
        assert_eq!(
            Probe::Clear(1).reply(1, serde_json::json!({ "cleared": true })),
            Ok(Reply::Cleared(true))
        );
        assert_eq!(
            Probe::Clear(0).reply(1, serde_json::json!({ "cleared": true })),
            Ok(Reply::Cleared(false))
        );
    }

    #[test]
    fn a_result_that_is_no_response_is_malformed() {
        assert!(matches!(
            Probe::Clear(1).reply(1, serde_json::json!("nope")),
            Err(RpcError::Malformed(_))
        ));
    }
}
