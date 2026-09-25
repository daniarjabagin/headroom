#[cfg(target_os = "linux")]
mod bus;
pub mod diagnostics;
pub mod log_level;
pub mod system_info;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::task::JoinSet;

use crate::config::{DaemonConfig, Shutdown};
use crate::core::{Core, CoreParts};
use crate::error::{DaemonError, StorageError};
use crate::events::{self, EventSink, EventSinks};
use crate::home::HomeDisplay;
use crate::ipc::{self, Hub, SocketFile};
use crate::notify::{Notifier, release_task};
use crate::random::ThreadRandom;
use crate::rescan::{RescanRequests, Rescans};
use crate::service::Service;
use crate::status::StatusFetch;
use crate::storage::Storage;
use crate::update::{UpdateCheckRequests, UpdateChecks, UpdateConfig};
use crate::{credentials, registry, rescan, status, update};
use diagnostics::{DiagnosticsContext, TransportKind};
use log_level::LogControl;

pub async fn run(config: DaemonConfig) -> Result<(), DaemonError> {
    let socket = config.socket.as_deref().map(ipc::bind).transpose()?;
    let hub = Arc::new(Hub::default());
    let logging = config.logging.clone();
    let storage = open_storage(config.db_path.clone()).await?;
    #[cfg(target_os = "linux")]
    let bus = bus::BusTransport::connect(&config.bus).await?;
    #[cfg(target_os = "linux")]
    let notifier = bus.notifier();
    #[cfg(not(target_os = "linux"))]
    let notifier: Arc<dyn Notifier> = hub.clone();
    let status_pages = config.status_pages.clone();
    let (parts, updates, shutdown) = core_parts(config, storage, notifier);
    let core = Arc::new(Core::load(parts).await?);
    let (rescans, rescan_requests) = rescan::channel();
    let (update_checks, updates) = update_channel(updates);
    let diagnostics = diagnostics_context(&core, socket.is_some(), logging.clone());
    let service = build_service(&core, rescans, update_checks, diagnostics);
    let mut sinks = EventSinks::default();
    #[cfg(target_os = "linux")]
    bus.serve(service.clone(), &mut sinks).await?;
    let mut tasks = JoinSet::new();
    let socket_file = socket.map(|socket| {
        sinks.push(hub.clone());
        serve_socket(socket, &service, &hub, &mut tasks)
    });
    let sink: Arc<dyn EventSink> = Arc::new(sinks);
    let background = Background {
        rescan_requests,
        logging,
        updates,
        status_pages,
    };
    spawn_background(&mut tasks, &core, &service, background);
    #[cfg(target_os = "linux")]
    tasks.spawn(bus.forward_actions(sink.clone()));
    tasks.spawn(events::publish_changes(core, sink));
    tracing::info!("headroom daemon running");
    stop(shutdown, tasks, socket_file).await;
    Ok(())
}

fn update_channel(
    updates: Option<UpdateConfig>,
) -> (
    Option<UpdateChecks>,
    Option<(UpdateConfig, UpdateCheckRequests)>,
) {
    let (checks, requests) = update::channel();
    match updates {
        Some(updates) => (Some(checks), Some((updates, requests))),
        None => (None, None),
    }
}

fn build_service(
    core: &Arc<Core>,
    rescans: Rescans,
    update_checks: Option<UpdateChecks>,
    diagnostics: DiagnosticsContext,
) -> Service {
    let service = Service::new(core.clone(), rescans).with_diagnostics(diagnostics);
    match update_checks {
        Some(checks) => service.with_update_checks(checks),
        None => service,
    }
}

struct Background {
    rescan_requests: RescanRequests,
    logging: Option<Arc<dyn LogControl>>,
    updates: Option<(UpdateConfig, UpdateCheckRequests)>,
    status_pages: Option<Arc<dyn StatusFetch>>,
}

fn spawn_background(
    tasks: &mut JoinSet<()>,
    core: &Arc<Core>,
    service: &Service,
    background: Background,
) {
    tasks.spawn(registry::supervise(
        core.clone(),
        background.rescan_requests,
    ));
    tasks.spawn(credentials::watch_signed_out(
        service.clone(),
        credentials::TIMING,
    ));
    if let Some(control) = background.logging {
        tasks.spawn(log_level::follow(core.clone(), control));
    }
    tasks.spawn(release_task::run(core.clone()));
    if let Some((updates, requests)) = background.updates {
        tasks.spawn(update::run(core.clone(), updates, requests));
    }
    if let Some(fetch) = background.status_pages {
        tasks.spawn(status::run(core.clone(), fetch));
    }
}

fn diagnostics_context(
    core: &Core,
    socket: bool,
    logging: Option<Arc<dyn LogControl>>,
) -> DiagnosticsContext {
    let mut transports = Vec::new();
    if cfg!(target_os = "linux") {
        transports.push(TransportKind::Dbus);
    }
    if socket {
        transports.push(TransportKind::Socket);
    }
    DiagnosticsContext {
        started_at: core.clock.now(),
        system: system_info::detect(),
        transports,
        logging,
        homes: HomeDisplay::new(dirs::home_dir()),
    }
}

fn serve_socket(
    (listener, file): (UnixListener, SocketFile),
    service: &Service,
    hub: &Arc<Hub>,
    tasks: &mut JoinSet<()>,
) -> SocketFile {
    tasks.spawn(ipc::accept(listener, service.clone(), hub.clone()));
    tracing::info!(path = %file.path().display(), "serving on a Unix socket");
    file
}

fn core_parts(
    config: DaemonConfig,
    storage: Storage,
    notifier: Arc<dyn Notifier>,
) -> (CoreParts, Option<UpdateConfig>, Shutdown) {
    let parts = CoreParts {
        storage,
        providers: config.providers,
        catalog: config.catalog,
        price_book: config.price_book,
        clock: config.clock,
        random: Arc::new(ThreadRandom),
        tz: config.tz,
        homes: HomeDisplay::new(dirs::home_dir()),
        notifier,
        system_locale: config.system_locale,
    };
    (parts, config.updates, config.shutdown)
}

async fn stop(shutdown: Shutdown, mut tasks: JoinSet<()>, socket_file: Option<SocketFile>) {
    shutdown.await;
    tasks.shutdown().await;
    drop(socket_file);
    tracing::info!("headroom daemon stopped");
}

async fn open_storage(path: PathBuf) -> Result<Storage, DaemonError> {
    let opened = tokio::task::spawn_blocking(move || Storage::open(&path))
        .await
        .map_err(|error| StorageError::Task(error.to_string()))?;
    Ok(opened?)
}
