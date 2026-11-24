//! The seed forms of the feature's facts, for the scripts' `fact` step (`WindowsSandbox::SEED`):
//! the sandbox as the host read it, the setup's end, the folder trusted.

use super::{Config, Fact, Host, Mode};
use crate::screens::SeedError;

/// `{"kind":"environment","mode":"unelevated","requirements":["elevated","unelevated"],
/// "host":"local"}`: `mode` is the configured sandbox (none: not set up), `requirements` the
/// modes the organization allows (absent: both, as when no policy limits them), `host` where the
/// executors are (`local`, `mixed`, `remote`; default `local`), `elevated_ready` the admin setup
/// is done. `{"kind":"completed","mode":"elevated","success":false,"error":"..."}` is
/// `windowsSandbox/setupCompleted`, `{"kind":"trusted"}` the folder trust the first prompt waits
/// for.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Environment {
        #[serde(default)]
        mode: Option<Mode>,
        #[serde(default)]
        requirements: Option<Vec<Mode>>,
        #[serde(default = "local")]
        host: HostSeed,
        #[serde(default)]
        elevated_ready: bool,
    },
    Completed {
        mode: Mode,
        success: bool,
        #[serde(default)]
        error: Option<String>,
    },
    Trusted,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum HostSeed {
    Local,
    Mixed,
    Remote,
}

fn local() -> HostSeed {
    HostSeed::Local
}

pub fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    let ev: SeedEv =
        serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))?;
    Ok(match ev {
        SeedEv::Environment {
            mode,
            requirements,
            host,
            elevated_ready,
        } => Fact::Environment {
            config: Config {
                mxc: false,
                mode,
                requirements: Some(requirements),
            },
            host: match host {
                HostSeed::Local => Host::Local,
                HostSeed::Mixed => Host::Mixed,
                HostSeed::Remote => Host::Remote,
            },
            local_server: !matches!(host, HostSeed::Remote),
            elevated_ready,
        },
        SeedEv::Completed {
            mode,
            success,
            error,
        } => Fact::Completed {
            mode,
            success,
            error,
        },
        SeedEv::Trusted => Fact::Trusted,
    })
}
