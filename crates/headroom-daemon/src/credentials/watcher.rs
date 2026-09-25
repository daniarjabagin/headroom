use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use headroom_core::account::AccountId;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::service::Service;
use crate::usage::is_relevant;

pub async fn watch_file(service: Service, id: AccountId, path: PathBuf, settle: Duration) {
    let (sender, mut changes) = mpsc::channel(1);
    let Some(_watcher) = start_watcher(&path, sender) else {
        return;
    };
    while changes.recv().await.is_some() {
        tokio::time::sleep(settle).await;
        while changes.try_recv().is_ok() {}
        tracing::info!(account = %id, "credentials changed, refreshing");
        if let Err(error) = service.refresh(&id.0) {
            tracing::debug!(account = %id, %error, "refresh after a credential change failed");
        }
    }
}

fn start_watcher(path: &Path, sender: mpsc::Sender<()>) -> Option<RecommendedWatcher> {
    let dir = path.parent()?;
    let name = path.file_name()?.to_owned();
    let handler = move |event: notify::Result<Event>| {
        if event.is_ok_and(|e| touches(&e, &name)) {
            sender.try_send(()).ok();
        }
    };
    let mut watcher = notify::recommended_watcher(handler)
        .inspect_err(|error| tracing::debug!(%error, "credential watcher unavailable"))
        .ok()?;
    watcher
        .watch(dir, RecursiveMode::NonRecursive)
        .inspect_err(|error| {
            tracing::debug!(dir = %dir.display(), %error, "cannot watch credentials");
        })
        .ok()?;
    Some(watcher)
}

fn touches(event: &Event, name: &OsString) -> bool {
    is_relevant(event.kind)
        && event
            .paths
            .iter()
            .any(|path| path.file_name() == Some(name.as_os_str()))
}

#[cfg(test)]
mod tests {
    use notify::EventKind;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};

    use super::*;

    fn event(kind: EventKind, path: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn only_changes_to_the_credential_file_count() {
        let name = OsString::from("auth.json");
        let created = event(EventKind::Create(CreateKind::File), "/h/auth.json");
        let renamed = event(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            "/h/auth.json",
        );
        let other = event(EventKind::Create(CreateKind::File), "/h/history.jsonl");
        let read = event(EventKind::Access(AccessKind::Read), "/h/auth.json");
        assert!(touches(&created, &name));
        assert!(touches(&renamed, &name));
        assert!(!touches(&other, &name));
        assert!(!touches(&read, &name));
    }
}
