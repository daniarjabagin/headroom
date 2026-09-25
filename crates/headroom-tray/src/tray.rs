use std::sync::Arc;

use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, OfflineReason, ToolTip, TrayMethods};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{Instant, Interval, MissedTickBehavior, interval_at};

use crate::events::{Event, Events, MenuAction, MenuLabels, TrayUpdate};
use crate::icon::{FRAME, FULL_OPACITY, Painter, Pixmap, Pixmaps, TrayText, opacity};

const ITEM_ID: &str = "headroom";
const MARK_ICON: &str = "headroom-symbolic";

struct HeadroomTray {
    events: Events,
    drawn: Option<Pixmaps>,
    mark: Vec<Pixmap>,
    text: TrayText,
    tooltip: String,
    labels: MenuLabels,
}

impl HeadroomTray {
    fn apply(&mut self, update: TrayUpdate, drawn: Option<Pixmaps>) {
        self.drawn = drawn;
        self.text = update.text;
        self.tooltip = update.tooltip;
        self.labels = update.labels;
    }
}

fn icons(pixmaps: &[Pixmap]) -> Vec<Icon> {
    pixmaps
        .iter()
        .filter_map(|pixmap| {
            let side = i32::try_from(pixmap.size).ok()?;
            Some(Icon {
                width: side,
                height: side,
                data: pixmap.argb.clone(),
            })
        })
        .collect()
}

fn menu_item(label: &str, action: MenuAction) -> MenuItem<HeadroomTray> {
    StandardItem {
        label: label.to_owned(),
        activate: Box::new(move |tray: &mut HeadroomTray| {
            tray.events.send(Event::Menu(action.clone()));
        }),
        ..StandardItem::default()
    }
    .into()
}

impl ksni::Tray for HeadroomTray {
    fn id(&self) -> String {
        ITEM_ID.to_owned()
    }

    fn title(&self) -> String {
        self.text.title.clone()
    }

    fn activate(&mut self, x: i32, y: i32) {
        self.events.send(Event::Activate { x, y });
    }

    fn icon_name(&self) -> String {
        if self.drawn.is_some() {
            String::new()
        } else {
            MARK_ICON.to_owned()
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        icons(self.drawn.as_deref().unwrap_or(&self.mark))
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: self.text.tooltip_title.clone(),
            description: self.tooltip.clone(),
            ..ToolTip::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            menu_item(&self.labels.open, MenuAction::Open),
            menu_item(&self.labels.refresh, MenuAction::RefreshNow),
            menu_item(&self.labels.settings, MenuAction::Settings),
            MenuItem::Separator,
            menu_item(&self.labels.quit, MenuAction::Quit),
        ]
    }

    fn watcher_offline(&self, reason: OfflineReason) -> bool {
        tracing::warn!(
            ?reason,
            "no system tray host is running; start one (for i3, polybar or tint2: snixembed --fork) \
             or bind `headroom-tray --toggle` to a key"
        );
        true
    }
}

fn same_pixmaps(previous: Option<&Pixmaps>, next: Option<&Pixmaps>) -> bool {
    match (previous, next) {
        (Some(previous), Some(next)) => Arc::ptr_eq(previous, next),
        (None, None) => true,
        _ => false,
    }
}

fn replace_changed(current: &mut TrayUpdate, update: TrayUpdate) -> bool {
    if *current == update {
        return false;
    }
    *current = update;
    true
}

#[derive(Default)]
struct Pulse {
    ticker: Option<Interval>,
    frame: u32,
}

impl Pulse {
    fn follow(&mut self, pulsing: bool) {
        match (pulsing, self.ticker.is_some()) {
            (true, false) => {
                let mut ticker = interval_at(Instant::now() + FRAME, FRAME);
                ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                self.ticker = Some(ticker);
                self.frame = 0;
            }
            (false, true) => self.ticker = None,
            _ => {}
        }
    }

    async fn next_frame(&mut self) {
        match self.ticker.as_mut() {
            Some(ticker) => {
                ticker.tick().await;
                self.frame = self.frame.wrapping_add(1);
            }
            None => std::future::pending().await,
        }
    }

    fn opacity(&self) -> u8 {
        if self.ticker.is_some() {
            opacity(self.frame)
        } else {
            FULL_OPACITY
        }
    }
}

pub async fn serve(
    events: Events,
    mark: Vec<Pixmap>,
    first: TrayUpdate,
    mut updates: UnboundedReceiver<TrayUpdate>,
) {
    let mut painter = Painter::default();
    let mut pulse = Pulse::default();
    pulse.follow(first.icon.pulses());
    let mut drawn = painter.paint(&first.icon, pulse.opacity());
    let tray = HeadroomTray {
        events: events.clone(),
        drawn: drawn.clone(),
        mark,
        text: first.text.clone(),
        tooltip: first.tooltip.clone(),
        labels: first.labels.clone(),
    };
    let handle = match tray.assume_sni_available(true).spawn().await {
        Ok(handle) => handle,
        Err(error) => {
            events.send(Event::TrayFailed(error.to_string()));
            return;
        }
    };
    let mut current = first;
    loop {
        let changed = tokio::select! {
            update = updates.recv() => match update {
                Some(update) => replace_changed(&mut current, update),
                None => return,
            },
            () = pulse.next_frame() => false,
        };
        pulse.follow(current.icon.pulses());
        let next = painter.paint(&current.icon, pulse.opacity());
        if !changed && same_pixmaps(drawn.as_ref(), next.as_ref()) {
            continue;
        }
        drawn.clone_from(&next);
        let update = current.clone();
        if handle
            .update(move |tray| tray.apply(update, next))
            .await
            .is_none()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_pixmaps_are_recognised_by_identity() {
        let pixmaps: Pixmaps = vec![Pixmap {
            size: 1,
            argb: vec![0; 4],
        }]
        .into();
        let copy: Pixmaps = pixmaps.to_vec().into();
        assert!(same_pixmaps(Some(&pixmaps), Some(&Arc::clone(&pixmaps))));
        assert!(!same_pixmaps(Some(&pixmaps), Some(&copy)));
        assert!(same_pixmaps(None, None));
        assert!(!same_pixmaps(Some(&pixmaps), None));
    }

    #[tokio::test]
    async fn the_pulse_timer_runs_only_while_critical() {
        let mut pulse = Pulse::default();
        assert_eq!(pulse.opacity(), FULL_OPACITY);
        pulse.follow(true);
        assert!(pulse.ticker.is_some());
        for _ in 0..4 {
            pulse.next_frame().await;
        }
        assert_eq!(pulse.opacity(), opacity(4));
        assert!(pulse.opacity() < FULL_OPACITY);
        pulse.follow(false);
        assert!(pulse.ticker.is_none());
        assert_eq!(pulse.opacity(), FULL_OPACITY);
    }
}
