pub mod tracker;

use jiff::Timestamp;

use crate::core::Core;
use crate::home::UsageHome;

pub use tracker::ActivityTracker;

pub fn record_write(core: &Core, home: &UsageHome, latest: Option<Timestamp>) {
    let now = core.clock.now();
    let at = latest.map_or(now, |latest| latest.min(now));
    let became_live = core.model().activity.record(home, at, now);
    if became_live {
        tracing::debug!(provider = %home.provider, home = %home.home.display(), "usage home is live");
        core.activity_started();
        core.mark_changed();
    }
}

#[cfg(test)]
mod tests;
