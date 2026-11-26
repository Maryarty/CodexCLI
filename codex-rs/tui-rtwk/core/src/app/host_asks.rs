//! Host continuations bind to their request handle or to the connector that owns local work.
//! A gateway install can itself replace the transport before acknowledging its setup ask.

use super::{AppEv, AppFx, BackendId, CodexApp, Command, ScreenEv, ScreenStates};
use crate::app::effects::Ask as HostAsk;
use crate::ids::Ticket;
use crate::rpc::Address;
use crate::screens::{import, onboarding, startup};
use onboarding::Ask;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReplyKind {
    Facts,
    TrustWrite,
    Consent,
    Update,
    Auth,
    Launch,
    Cwd,
    Hooks,
    Gateway,
    GatewayLogins,
    GatewayOAuth,
    GatewayDevice,
    Detect,
    Import,
}

impl ReplyKind {
    fn on_transport(self) -> bool {
        !matches!(self, Self::Update | Self::Cwd | Self::Gateway)
    }

    pub(super) fn transport_address(self, app: &CodexApp) -> Option<Address> {
        self.on_transport()
            .then(|| app.address(BackendId::default()))
    }

    fn address(self, app: &CodexApp) -> Address {
        self.transport_address(app)
            .unwrap_or_else(|| connector(app))
    }

    fn is_current(self, screens: &ScreenStates, ticket: Ticket) -> bool {
        let request = ticket.seq();
        match self {
            // The host may have started a browser/device login while Esc was being applied.
            // Its terminal outcome must still be seen once so auth can cancel the server login.
            Self::Auth => true,
            Self::Cwd | Self::Hooks => screens.startup.waits_for(request),
            Self::Detect | Self::Import => screens.import.waits_for(request),
            Self::Facts
            | Self::TrustWrite
            | Self::Consent
            | Self::Update
            | Self::Launch
            | Self::Gateway
            | Self::GatewayLogins
            | Self::GatewayOAuth
            | Self::GatewayDevice => screens.onboarding.waits_for(request),
        }
    }

    pub(super) fn of_command(command: &Command) -> Option<(Ticket, Self)> {
        let (request, kind) = match command.host_ask()? {
            HostAsk::Startup { ask } => match ask {
                Ask::Facts { request } => (request, Self::Facts),
                Ask::WriteTrust { request, .. } => (request, Self::TrustWrite),
                Ask::CheckConsent { request, .. } => (request, Self::Consent),
                Ask::RunUpdate { request } => (request, Self::Update),
                Ask::Login { request, .. } => (request, Self::Auth),
                Ask::Launch { request } => (request, Self::Launch),
                Ask::SaveResumeCwd { request, .. } => (request, Self::Cwd),
                Ask::TrustHooks { request, .. } => (request, Self::Hooks),
                Ask::SelectGateway { request, .. } => (request, Self::Gateway),
                Ask::GatewayLogins { request, .. } => (request, Self::GatewayLogins),
                Ask::GatewayOAuth { request, .. } => (request, Self::GatewayOAuth),
                Ask::GatewayDevice { request, .. } => (request, Self::GatewayDevice),
                Ask::DismissUpdate { .. }
                | Ask::CancelLogin { .. }
                | Ask::OpenBrowser { .. }
                | Ask::ReloadConfig
                | Ask::SaveMigration { .. }
                | Ask::AcknowledgeGovCloud
                | Ask::OssChoice { .. }
                | Ask::DaemonChoice { .. }
                | Ask::CancelGatewayOAuth
                | Ask::FailLaunch { .. }
                | Ask::RefreshUpdateCache => return None,
            },
            HostAsk::Import { ask } => match ask {
                import::Request::Detect { request, .. } => (request, Self::Detect),
                import::Request::Import { request, .. } => (request, Self::Import),
            },
            _ => return None,
        };
        Some((Ticket::new(*request), kind))
    }

    fn of_event(event: &AppEv) -> Option<(Ticket, Self)> {
        use onboarding::Ev;

        let (request, kind) = match event {
            AppEv::Screen(ScreenEv::Onboarding(ev)) => match ev {
                Ev::Facts { request, .. } => (*request, Self::Facts),
                Ev::TrustSaved { request } | Ev::TrustFailed { request, .. } => {
                    (*request, Self::TrustWrite)
                }
                Ev::Consent { request, .. }
                | Ev::ConsentAt { request, .. }
                | Ev::ConsentFailed { request, .. } => (*request, Self::Consent),
                Ev::UpdateQueued { request } => (*request, Self::Update),
                Ev::Auth(result) => (result.request, Self::Auth),
                Ev::Launched { request, .. } => (*request, Self::Launch),
                Ev::GatewayStarted { request } => (*request, Self::Gateway),
                Ev::GatewayLogins { request, .. } => (*request, Self::GatewayLogins),
                Ev::GatewayOAuth { request, .. } => (*request, Self::GatewayOAuth),
                Ev::GatewayDevice { request, .. } => (*request, Self::GatewayDevice),
                _ => return None,
            },
            AppEv::Screen(ScreenEv::Startup(startup::Ev::CwdSaved { request, .. })) => {
                (*request, Self::Cwd)
            }
            AppEv::Screen(ScreenEv::Startup(startup::Ev::HooksTrusted { request, .. })) => {
                (*request, Self::Hooks)
            }
            AppEv::Screen(ScreenEv::Import(import::Ev::Detected { request, .. })) => {
                (*request, Self::Detect)
            }
            AppEv::Screen(ScreenEv::Import(import::Ev::Imported { request, .. })) => {
                (*request, Self::Import)
            }
            _ => return None,
        };
        Some((Ticket::new(request), kind))
    }
}

fn connector(app: &CodexApp) -> Address {
    let backend = BackendId::default();
    Address {
        backend,
        epoch: app.epoch_of(backend),
    }
}

pub(super) fn uses_transport_without_reply(command: &Command) -> bool {
    matches!(
        command.host_ask(),
        Some(HostAsk::Startup {
            ask: Ask::CancelLogin { .. } | Ask::CancelGatewayOAuth
        })
    )
}

/// Lend the app's mint to a host-owning screen step. Register its requests before any effects
/// are sent. Its command order, including fire-and-forget work, is the order it emitted.
pub(super) fn step<R>(
    app: &mut CodexApp,
    fx: &mut AppFx,
    step: impl FnOnce(&mut ScreenStates, &mut AppFx) -> R,
) -> R {
    let first = fx.commands.len();
    std::mem::swap(&mut app.mint, &mut fx.host_mint);
    let result = step(Rc::make_mut(&mut app.screens), fx);
    std::mem::swap(&mut app.mint, &mut fx.host_mint);
    let screens = &*app.screens;
    app.host_outgoing
        .retain_tickets(|ticket, kind| kind.is_current(screens, ticket));
    for (ticket, kind) in fx.commands[first..]
        .iter()
        .filter_map(ReplyKind::of_command)
    {
        if kind.is_current(screens, ticket) {
            app.host_outgoing.insert(ticket, kind.address(app), kind);
        }
    }
    result
}

pub(super) fn may_dispatch(app: &CodexApp, ticket: Ticket, kind: ReplyKind) -> bool {
    app.host_outgoing.address(ticket) == Some(kind.address(app))
        && app.host_outgoing.context(ticket) == Some(&kind)
        && kind.is_current(&app.screens, ticket)
}

/// Consume a host reply exactly once. Unasked notifications (including auth progress) follow
/// their own domain correlation. Zero is reserved for screen fixtures seeded without a request.
pub(super) fn accept(app: &mut CodexApp, origin: Address, event: &AppEv, allow_seed: bool) -> bool {
    let Some((ticket, kind)) = ReplyKind::of_event(event) else {
        return true;
    };
    if allow_seed
        && origin.backend == BackendId::default()
        && ticket.seq() == 0
        && kind == ReplyKind::Detect
    {
        return true;
    }
    take(app, origin, ticket, kind)
}

pub(super) fn accept_failure(app: &mut CodexApp, origin: Address, ticket: Ticket) -> bool {
    let Some(kind) = app.host_outgoing.context(ticket).copied() else {
        return false;
    };
    take(app, origin, ticket, kind)
}

fn take(app: &mut CodexApp, origin: Address, ticket: Ticket, kind: ReplyKind) -> bool {
    if origin.backend != BackendId::default()
        || (kind.on_transport() && origin != kind.address(app))
        || !may_dispatch(app, ticket, kind)
    {
        return false;
    }
    app.host_outgoing.drop_ticket(ticket);
    true
}

pub(super) fn lost(app: &mut CodexApp, backend: BackendId) {
    if backend == BackendId::default() {
        app.host_outgoing = crate::rpc::Pending::default();
        app.host_transport_lost = false;
        let screens = Rc::make_mut(&mut app.screens);
        onboarding::disconnected(&mut screens.onboarding);
        startup::connection_lost(&mut screens.startup, true);
        import::connection_lost(&mut screens.import, None);
    }
}

pub(super) fn transport_lost(app: &mut CodexApp, backend: BackendId) {
    if backend == BackendId::default() {
        app.host_transport_lost = true;
        app.host_outgoing.retain(|kind| !kind.on_transport());
        onboarding::session::lost(&mut Rc::make_mut(&mut app.screens).onboarding);
    }
}

pub(super) fn transport_replaced(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    if backend != BackendId::default() {
        return;
    }
    app.host_transport_lost = false;
    step(app, fx, |screens, fx| {
        onboarding::transport_replaced(&mut screens.onboarding, fx);
        startup::connection_lost(&mut screens.startup, false);
        import::connection_lost(&mut screens.import, Some(fx));
    });
}

pub(super) fn reconnected(app: &mut CodexApp, fx: &mut AppFx) {
    if app.host_transport_lost {
        transport_replaced(app, BackendId::default(), fx);
    }
    // Native reconnect installs the handle first, which may already have asked for new facts.
    if !app.host_outgoing.any(|kind| *kind == ReplyKind::Facts) {
        step(app, fx, |screens, fx| {
            onboarding::connected(&mut screens.onboarding, fx)
        });
    }
}
