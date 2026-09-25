use std::rc::Rc;

use gtk::prelude::*;

use crate::account::NoticeView;
use crate::payload::{Account, Balance, Usage, Window};
use crate::popup_model::status::{StatusView, status_view};
use crate::ui::account_section::SectionInput;
use crate::ui::context::{Action, Ctx};
use crate::ui::keyed::{Keyed, SharedTick, arrange};
use crate::ui::notice::MountedNotice;
use crate::ui::quota_row::MountedQuota;
use crate::ui::section_card::{Card, RestKey, button_state, card, mount_notice, rest_widget};
use crate::ui::status_notice::status_notice;
use crate::ui::usage_rows::{extra_rows, trend_row};
use crate::ui::widgets::{button, column, icon};

#[derive(PartialEq)]
struct AlertKey {
    notice: NoticeView,
    motion: bool,
}

struct MountedAlert {
    key: AlertKey,
    notice: MountedNotice,
}

#[derive(PartialEq)]
struct ExtrasKey {
    provider_name: String,
    balances: Vec<Balance>,
    usage: Option<Usage>,
    framed: bool,
}

fn caret_name(expanded: bool) -> &'static str {
    if expanded {
        "pan-up-symbolic"
    } else {
        "pan-down-symbolic"
    }
}

fn expander(ctx: &Ctx, account: &Account, content: &gtk::Box) -> gtk::Box {
    let expanded = ctx.ui.expanded.contains(&account.id);
    let revealer = gtk::Revealer::new();
    revealer.set_child(Some(content));
    revealer.set_reveal_child(expanded);
    revealer.set_transition_duration(if ctx.motion { 200 } else { 0 });
    let caret = icon(caret_name(expanded), 10, &["headroom-caret-icon"]);
    let (id, act) = (account.id.clone(), Rc::clone(&ctx.act));
    let (shown, arrow) = (revealer.clone(), caret.clone());
    let toggle = button(&caret, &["headroom-caret"], move || {
        let open = !shown.reveals_child();
        shown.set_reveal_child(open);
        arrow.set_icon_name(Some(caret_name(open)));
        act(Action::SetExpanded(id.clone(), open));
    });
    let body = column(0, &[]);
    body.append(&toggle);
    body.append(&revealer);
    body
}

fn extras_widget(ctx: &Ctx, input: &SectionInput, framed: bool) -> Option<gtk::Widget> {
    let extras = extra_rows(ctx, input.account, input.usage)?;
    Some(if framed {
        expander(ctx, input.account, &extras).upcast()
    } else {
        extras.upcast()
    })
}

pub struct MountedBody {
    pub card: gtk::Box,
    alert: Option<MountedAlert>,
    incident: Option<Keyed<StatusView>>,
    rest: Option<Keyed<RestKey>>,
    rows: Vec<(String, MountedQuota)>,
    trend: Option<Keyed<Usage>>,
    extras: Option<Keyed<ExtrasKey>>,
}

impl MountedBody {
    pub fn new() -> Self {
        Self {
            card: column(0, &["headroom-card"]),
            alert: None,
            incident: None,
            rest: None,
            rows: Vec::new(),
            trend: None,
            extras: None,
        }
    }

    pub fn update(&mut self, ctx: &Ctx, input: &SectionInput, appeared: bool) {
        let account = input.account;
        let Card {
            alert,
            rest,
            windows,
        } = card(ctx, account);
        let mut order = Vec::new();
        order.extend(self.alert(ctx, account, alert));
        order.extend(self.incident(ctx, input, appeared));
        order.extend(self.rest(ctx, account, rest));
        if let Some(windows) = windows {
            self.limits(ctx, input, &windows, &mut order);
        } else {
            self.rows.clear();
            self.trend = None;
            self.extras = None;
        }
        arrange(&self.card, &order, None);
        self.card.set_visible(!order.is_empty());
    }

    pub fn ticks(&self) -> Vec<SharedTick> {
        let incident = self
            .incident
            .iter()
            .flat_map(|slot| slot.ticks.iter().cloned());
        let rows = self.rows.iter().map(|(_, row)| Rc::clone(&row.tick));
        incident.chain(rows).collect()
    }

    fn alert(
        &mut self,
        ctx: &Ctx,
        account: &Account,
        alert: Option<NoticeView>,
    ) -> Option<gtk::Widget> {
        let Some(notice) = alert else {
            self.alert = None;
            return None;
        };
        let key = AlertKey {
            notice,
            motion: ctx.motion,
        };
        if self.alert.as_ref().is_none_or(|mounted| mounted.key != key) {
            let notice = mount_notice(ctx, account, &key.notice);
            self.alert = Some(MountedAlert { key, notice });
        }
        let mounted = self.alert.as_ref()?;
        mounted
            .notice
            .apply(ctx.locale.lang, |_| button_state(ctx, account));
        Some(mounted.notice.widget.clone().upcast())
    }

    fn incident(&mut self, ctx: &Ctx, input: &SectionInput, appeared: bool) -> Option<gtk::Widget> {
        let Some(view) = input
            .status
            .and_then(|status| status_view(ctx.locale.lang, status))
        else {
            self.incident = None;
            return None;
        };
        let (kept, _) = Keyed::reuse(self.incident.take(), view, ctx, |view| {
            status_notice(ctx, view, input.now, appeared).upcast()
        });
        let widget = kept.widget.clone();
        self.incident = Some(kept);
        Some(widget)
    }

    fn rest(&mut self, ctx: &Ctx, account: &Account, rest: Option<RestKey>) -> Option<gtk::Widget> {
        let Some(rest) = rest else {
            self.rest = None;
            return None;
        };
        let (kept, _) = Keyed::reuse(self.rest.take(), rest, ctx, |rest| {
            rest_widget(ctx, account, rest)
        });
        let widget = kept.widget.clone();
        self.rest = Some(kept);
        Some(widget)
    }

    fn limits(
        &mut self,
        ctx: &Ctx,
        input: &SectionInput,
        windows: &[&Window],
        order: &mut Vec<gtk::Widget>,
    ) {
        let mut previous = std::mem::take(&mut self.rows);
        for window in windows {
            let row = match previous.iter().position(|(id, _)| id == &window.id) {
                Some(index) => {
                    let (id, row) = previous.swap_remove(index);
                    row.update(ctx, window, input.now);
                    (id, row)
                }
                None => (
                    window.id.clone(),
                    MountedQuota::new(ctx, window, input.now, input.animate),
                ),
            };
            order.push(row.1.widget.clone().upcast());
            self.rows.push(row);
        }
        order.extend(self.trend(ctx, input, windows.is_empty()));
    }

    fn trend(&mut self, ctx: &Ctx, input: &SectionInput, no_rows: bool) -> Vec<gtk::Widget> {
        let usage = input.usage.filter(|_| ctx.display.show_trend);
        let mut shown = Vec::new();
        match usage {
            Some(usage) => {
                let (kept, _) = Keyed::reuse(self.trend.take(), usage.clone(), ctx, |_| {
                    trend_row(ctx, usage).upcast()
                });
                shown.push(kept.widget.clone());
                self.trend = Some(kept);
            }
            None => self.trend = None,
        }
        shown.extend(self.extras(ctx, input, !(no_rows && usage.is_none())));
        shown
    }

    fn extras(&mut self, ctx: &Ctx, input: &SectionInput, framed: bool) -> Option<gtk::Widget> {
        let account = input.account;
        let key = ExtrasKey {
            provider_name: account.provider_name.clone(),
            balances: account.balances.clone(),
            usage: input
                .usage
                .filter(|_| ctx.display.show_account_spend)
                .cloned(),
            framed,
        };
        self.extras = Keyed::reuse_optional(self.extras.take(), key, ctx, || {
            extras_widget(ctx, input, framed)
        });
        self.extras.as_ref().map(|kept| kept.widget.clone())
    }
}
