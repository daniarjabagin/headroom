use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;

use super::alerts::Release;
use super::quiet::quiet_ends;
use super::text::Locale;
use crate::core::Core;

const LONGEST_WAIT: Duration = Duration::from_secs(60);
const SHORTEST_WAIT: Duration = Duration::from_secs(1);

pub async fn run(core: Arc<Core>) {
    let mut settings = core.settings_changes();
    loop {
        let wait = release_due(&core).await;
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            changed = settings.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

pub async fn release_due(core: &Core) -> Duration {
    let settings = core.model().settings.clone();
    let now = core.clock.now();
    let release = Release {
        settings: &settings.notifications,
        locale: Locale::resolve(settings.display.language, core.system_locale),
        now,
        tz: &core.tz,
    };
    if let Err(error) = core.alerts.release(&release).await {
        tracing::warn!(%error, "could not release notifications held during quiet hours");
    }
    next_wait(
        quiet_ends(now, &core.tz, settings.notifications.quiet_hours),
        now,
    )
}

#[must_use]
pub fn next_wait(quiet_end: Option<Timestamp>, now: Timestamp) -> Duration {
    let Some(end) = quiet_end else {
        return LONGEST_WAIT;
    };
    Duration::try_from(end.duration_since(now))
        .unwrap_or(SHORTEST_WAIT)
        .clamp(SHORTEST_WAIT, LONGEST_WAIT)
}

#[cfg(test)]
#[path = "release_task_tests.rs"]
mod tests;
