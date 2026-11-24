//! `host_capability_list!`: the one list of what the host can do for the app (ADR-012 "the host's
//! doors", arch-pillars P6). One entry is a capability: the `Ask` variant (with the fields it
//! carries), the `Done` variant that answers it, the executor method the host implements, and,
//! for an ask about a thread, the field the owner is read from. The macro writes
//!
//! - `Ask` and `Done`, the two enums the app and the host speak;
//! - `Ask::thread`, the routing: the thread an ask is about, whose owner's connection runs it;
//! - `HostDoors`, the trait of the host's executors: a method per capability, whose default is
//!   "this host does not do that" (`None`), so a capability that is listed before anybody built
//!   it needs no executor;
//! - `run_ask`, the dispatch of an `Ask` to the method of its capability, and `Chain`, which
//!   asks one set of executors and then the next (the host's executors live in several files).
//!
//! An entry is
//!
//! ```text
//! /// What the capability is.
//! method_name => Variant { field: Type, .. } by owner_field -> new Answer(Payload);
//! method_name => Variant(arg: Type) -> shared Answer;
//! ```
//!
//! `by owner_field` and the payload of the variant are optional; `new Answer(..)` declares the
//! `Done` variant, `shared Answer` names one another entry declared.

/// See the module's words.
macro_rules! host_capability_list {
    (
        $(
            $(#[$meta:meta])*
            $m:ident => $v:ident
                $( ( $($tf:ident : $tt:ty),* $(,)? ) )?
                $( { $($f:ident : $t:ty),* $(,)? } )?
                $( by $owner:ident )?
                -> $(new $dv:ident $db:tt)? $(shared $sv:ident)? ;
        )*
    ) => {
        /// What the host is asked.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Ask {
            $(
                $(#[$meta])*
                $v $( ( $($tt),* ) )? $( { $($f : $t),* } )?,
            )*
        }

        /// What the host answers: the variant each `Ask` declares (`OpenDesktop` and `OpenUrl`
        /// share one).
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum Done {
            $( $(
                #[doc = concat!("The answer to `Ask::", stringify!($v), "`.")]
                $dv $db,
            )? )*
        }

        impl Ask {
            /// The thread the ask is about, if it is about one: the connection of its owner runs
            /// it (the routing of `CodexApp::backend_of`).
            pub fn thread(&self) -> Option<&ThreadId> {
                match self {
                    $( $( Ask::$v { $owner, .. } => Some($owner), )? )*
                    _ => None,
                }
            }
        }

        /// The host's executors, one method per capability: what the host does for the app.
        /// Every method answers `None` unless the host implements it ("not this host's": the
        /// pump hands the command on), so adding a capability to the list builds everywhere.
        #[allow(unused_variables)]
        pub trait HostDoors: Sync {
            /// What an executor answers with: the `Done`, and whatever the host's pump needs
            /// beside it.
            type Out: Send;

            $(
                $(#[$meta])*
                fn $m(&self $( $(, $tf: $tt)* )? $( $(, $f: $t)* )?)
                    -> impl std::future::Future<Output = Option<Self::Out>> + Send
                {
                    async { None }
                }
            )*
        }

        /// Runs `ask` on the executor of its capability.
        pub async fn run_ask<H: HostDoors>(host: &H, ask: Ask) -> Option<H::Out> {
            match ask {
                $(
                    Ask::$v $( ( $($tf),* ) )? $( { $($f),* } )? => {
                        host.$m( $( $($tf),* )? $( $($f),* )? ).await
                    }
                )*
            }
        }

        /// Two sets of executors as one: the first that answers wins.
        pub struct Chain<A, B>(pub A, pub B);

        impl<A, B> HostDoors for Chain<A, B>
        where
            A: HostDoors,
            B: HostDoors<Out = A::Out>,
        {
            type Out = A::Out;

            $(
                fn $m(&self $( $(, $tf: $tt)* )? $( $(, $f: $t)* )?)
                    -> impl std::future::Future<Output = Option<Self::Out>> + Send
                {
                    async move {
                        if let Some(out) = self
                            .0
                            .$m( $( $($tf.clone()),* )? $( $($f.clone()),* )? )
                            .await
                        {
                            return Some(out);
                        }
                        self.1.$m( $( $($tf),* )? $( $($f),* )? ).await
                    }
                }
            )*
        }
    };
}
