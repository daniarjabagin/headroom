use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, OfflineReason, ToolTip, TrayMethods};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::events::{Event, Events, MenuAction, MenuLabels, TrayUpdate};
use crate::icon::Pixmap;

const ITEM_ID: &str = "headroom";
const MARK_ICON: &str = "headroom-symbolic";
const TITLE: &str = "Headroom";

struct HeadroomTray {
    events: Events,
    ring: Option<Vec<Pixmap>>,
    mark: Vec<Pixmap>,
    tooltip: String,
    labels: MenuLabels,
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
        TITLE.to_owned()
    }

    fn activate(&mut self, x: i32, y: i32) {
        self.events.send(Event::Activate { x, y });
    }

    fn icon_name(&self) -> String {
        if self.ring.is_some() {
            String::new()
        } else {
            MARK_ICON.to_owned()
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        icons(self.ring.as_deref().unwrap_or(&self.mark))
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: TITLE.to_owned(),
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

pub async fn serve(
    events: Events,
    mark: Vec<Pixmap>,
    first: TrayUpdate,
    mut updates: UnboundedReceiver<TrayUpdate>,
) {
    let tray = HeadroomTray {
        events: events.clone(),
        ring: first.ring,
        mark,
        tooltip: first.tooltip,
        labels: first.labels,
    };
    let handle = match tray.assume_sni_available(true).spawn().await {
        Ok(handle) => handle,
        Err(error) => {
            events.send(Event::TrayFailed(error.to_string()));
            return;
        }
    };
    while let Some(update) = updates.recv().await {
        let applied = handle
            .update(move |tray| {
                tray.ring = update.ring;
                tray.tooltip = update.tooltip;
                tray.labels = update.labels;
            })
            .await;
        if applied.is_none() {
            return;
        }
    }
}
