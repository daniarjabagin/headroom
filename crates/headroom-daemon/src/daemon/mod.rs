#[cfg(target_os = "linux")]
mod bus;

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
use crate::notify::Notifier;
use crate::random::ThreadRandom;
use crate::service::Service;
use crate::storage::Storage;
use crate::update::UpdateConfig;
use crate::{credentials, registry, rescan, update};

pub async fn run(config: DaemonConfig) -> Result<(), DaemonError> {
    let socket = config.socket.as_deref().map(ipc::bind).transpose()?;
    let hub = Arc::new(Hub::default());
    let storage = open_storage(config.db_path.clone()).await?;
    #[cfg(target_os = "linux")]
    let bus = bus::BusTransport::connect(&config.bus).await?;
    #[cfg(target_os = "linux")]
    let notifier = bus.notifier();
    #[cfg(not(target_os = "linux"))]
    let notifier: Arc<dyn Notifier> = hub.clone();
    let (parts, updates, shutdown) = core_parts(config, storage, notifier);
    let core = Arc::new(Core::load(parts).await?);
    let (rescans, rescan_requests) = rescan::channel();
    let (update_checks, update_requests) = update::channel();
    let mut service = Service::new(core.clone(), rescans);
    if updates.is_some() {
        service = service.with_update_checks(update_checks);
    }
    let mut sinks = EventSinks::default();
    #[cfg(target_os = "linux")]
    bus.serve(service.clone(), &mut sinks).await?;
    let mut tasks = JoinSet::new();
    let socket_file = socket.map(|socket| {
        sinks.push(hub.clone());
        serve_socket(socket, &service, &hub, &mut tasks)
    });
    let sink: Arc<dyn EventSink> = Arc::new(sinks);
    tasks.spawn(registry::supervise(core.clone(), rescan_requests));
    tasks.spawn(credentials::watch_signed_out(
        service.clone(),
        credentials::TIMING,
    ));
    if let Some(updates) = updates {
        tasks.spawn(update::run(core.clone(), updates, update_requests));
    }
    #[cfg(target_os = "linux")]
    tasks.spawn(bus.forward_actions(sink.clone()));
    tasks.spawn(events::publish_changes(core, sink));
    tracing::info!("headroom daemon running");
    stop(shutdown, tasks, socket_file).await;
    Ok(())
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
