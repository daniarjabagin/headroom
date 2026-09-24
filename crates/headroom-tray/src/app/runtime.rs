use std::thread;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::daemon;
use crate::events::{Command, Event, Events, TrayUpdate};
use crate::icon::Pixmap;
use crate::tray;

pub struct Channels {
    pub commands: UnboundedSender<Command>,
    pub tray: UnboundedSender<TrayUpdate>,
    pub events: async_channel::Receiver<Event>,
}

struct Worker {
    events: Events,
    commands: UnboundedReceiver<Command>,
    tray: UnboundedReceiver<TrayUpdate>,
    mark: Vec<Pixmap>,
    first: TrayUpdate,
}

async fn work(worker: Worker) {
    let Worker {
        events,
        commands,
        tray,
        mark,
        first,
    } = worker;
    let tray_task = tray::serve(events.clone(), mark, first, tray);
    let daemon_task = async {
        match zbus::Connection::session().await {
            Ok(connection) => {
                if let Err(error) = daemon::serve(connection, events.clone(), commands).await {
                    events.send(Event::CallFailed(error.to_string()));
                }
            }
            Err(error) => events.send(Event::CallFailed(error.to_string())),
        }
    };
    tokio::join!(tray_task, daemon_task);
}

pub fn start(mark: Vec<Pixmap>, first: TrayUpdate) -> std::io::Result<Channels> {
    let (event_sender, events) = async_channel::unbounded();
    let (commands, command_receiver) = unbounded_channel();
    let (tray, tray_receiver) = unbounded_channel();
    let worker = Worker {
        events: Events::new(event_sender),
        commands: command_receiver,
        tray: tray_receiver,
        mark,
        first,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    thread::Builder::new()
        .name("headroom-bus".into())
        .spawn(move || runtime.block_on(work(worker)))?;
    Ok(Channels {
        commands,
        tray,
        events,
    })
}
