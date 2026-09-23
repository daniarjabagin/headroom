use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use super::ingest::pass;
use crate::core::Core;
use crate::home::UsageHome;

pub const DEBOUNCE: Duration = Duration::from_secs(2);
pub const POLL_EVERY: Duration = Duration::from_secs(60);

pub async fn watch_home(core: Arc<Core>, home: UsageHome) {
    let (sender, mut changes) = mpsc::channel(1);
    let _watcher = start_watcher(&home.home, sender);
    let mut poll = tokio::time::interval(POLL_EVERY);
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut requested = core.ingest_requests();
    let mut summarized_for = None;
    loop {
        tokio::select! {
            _ = poll.tick() => {}
            Ok(()) = requested.changed() => {}
            Some(()) = changes.recv() => {
                tokio::time::sleep(DEBOUNCE).await;
                while changes.try_recv().is_ok() {}
            }
        }
        pass(&core, &home, &mut summarized_for).await;
    }
}

fn start_watcher(path: &Path, sender: mpsc::Sender<()>) -> Option<RecommendedWatcher> {
    let handler = move |event: notify::Result<Event>| {
        if event.is_ok_and(|e| is_relevant(e.kind)) {
            sender.try_send(()).ok();
        }
    };
    let mut watcher = notify::recommended_watcher(handler)
        .inspect_err(|error| tracing::warn!(%error, "file watcher unavailable, polling only"))
        .ok()?;
    watcher
        .watch(path, RecursiveMode::Recursive)
        .inspect_err(|error| {
            tracing::info!(path = %path.display(), %error, "cannot watch usage home, polling only");
        })
        .ok()?;
    Some(watcher)
}

fn is_relevant(kind: EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(modify) => !matches!(modify, ModifyKind::Metadata(_)),
        EventKind::Access(_) | EventKind::Any | EventKind::Other => false,
    }
}

#[cfg(test)]
mod tests {
    use notify::event::{AccessKind, CreateKind, DataChange, MetadataKind};

    use super::*;

    #[test]
    fn only_content_changes_trigger_ingest() {
        assert!(is_relevant(EventKind::Create(CreateKind::File)));
        assert!(is_relevant(EventKind::Modify(ModifyKind::Data(
            DataChange::Content
        ))));
        assert!(!is_relevant(EventKind::Access(AccessKind::Read)));
        assert!(!is_relevant(EventKind::Modify(ModifyKind::Metadata(
            MetadataKind::AccessTime
        ))));
    }
}
