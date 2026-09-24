use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;
use zbus::Connection;

use crate::events::{Command, Event, Events};

const SYSTEMD_UNIT: &str = "headroom.service";
const BUS_NAME: &str = "io.github.daniarjabagin.Headroom";

#[zbus::proxy(
    interface = "io.github.daniarjabagin.Headroom1",
    default_service = "io.github.daniarjabagin.Headroom",
    default_path = "/io/github/daniarjabagin/Headroom",
    gen_blocking = false
)]
trait Daemon {
    fn get_state(&self) -> zbus::Result<String>;

    fn get_settings(&self) -> zbus::Result<String>;

    fn list_providers(&self) -> zbus::Result<String>;

    fn refresh(&self, account_id: &str) -> zbus::Result<()>;

    fn refresh_now(&self) -> zbus::Result<()>;

    fn update_settings(&self, patch: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn state_changed(&self, state: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn open_requested(&self) -> zbus::Result<()>;
}

fn message(error: &zbus::Error) -> String {
    match error {
        zbus::Error::MethodError(_, Some(text), _) => text.clone(),
        other => other.to_string(),
    }
}

async fn fetch(proxy: &DaemonProxy<'_>, events: &Events) {
    match proxy.get_state().await {
        Ok(json) => events.send(Event::State(json)),
        Err(zbus::Error::MethodError(name, _, _))
            if name.as_str() == "org.freedesktop.DBus.Error.ServiceUnknown" =>
        {
            events.send(Event::Vanished);
            return;
        }
        Err(error) => {
            events.send(Event::CallFailed(message(&error)));
            return;
        }
    }
    fetch_settings(proxy, events).await;
    if let Ok(json) = proxy.list_providers().await {
        events.send(Event::Providers(json));
    }
}

async fn fetch_settings(proxy: &DaemonProxy<'_>, events: &Events) {
    if let Ok(json) = proxy.get_settings().await {
        events.send(Event::Settings(json));
    }
}

async fn start_service(connection: &Connection) -> Result<(), String> {
    connection
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "StartUnit",
            &(SYSTEMD_UNIT, "replace"),
        )
        .await
        .map(|_| ())
        .map_err(|error| message(&error))
}

async fn run_command(
    connection: &Connection,
    proxy: &DaemonProxy<'_>,
    command: Command,
    events: &Events,
) {
    let failed = |error: zbus::Error| events.send(Event::CallFailed(message(&error)));
    match command {
        Command::RefreshNow => {
            let result = proxy.refresh_now().await;
            events.send(Event::RefreshSettled(result.is_ok()));
            if let Err(error) = result {
                failed(error);
            }
        }
        Command::Refresh(id) => {
            if let Err(error) = proxy.refresh(&id).await {
                failed(error);
            }
        }
        Command::UpdateSettings(patch) => {
            if let Err(error) = proxy.update_settings(&patch).await {
                failed(error);
            }
        }
        Command::StartService => {
            events.send(Event::ServiceStarted(start_service(connection).await));
        }
    }
}

pub async fn serve(
    connection: Connection,
    events: Events,
    mut commands: UnboundedReceiver<Command>,
) -> zbus::Result<()> {
    let proxy = DaemonProxy::new(&connection).await?;
    let mut owners = proxy.inner().receive_owner_changed().await?;
    let mut states = proxy.receive_state_changed().await?;
    let mut opens = proxy.receive_open_requested().await?;
    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
    let name = zbus::names::BusName::try_from(BUS_NAME)?;
    if bus.name_has_owner(name).await? {
        fetch(&proxy, &events).await;
    } else {
        events.send(Event::Vanished);
    }
    loop {
        tokio::select! {
            Some(owner) = owners.next() => match owner {
                Some(_) => fetch(&proxy, &events).await,
                None => events.send(Event::Vanished),
            },
            Some(signal) = states.next() => {
                if let Ok(args) = signal.args() {
                    events.send(Event::State(args.state().clone()));
                }
                fetch_settings(&proxy, &events).await;
            },
            Some(_) = opens.next() => events.send(Event::OpenRequested),
            command = commands.recv() => match command {
                Some(command) => run_command(&connection, &proxy, command, &events).await,
                None => return Ok(()),
            },
        }
    }
}
