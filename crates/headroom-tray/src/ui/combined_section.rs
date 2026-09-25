use gtk::prelude::*;
use jiff::Timestamp;

use crate::combined::{CombinedGroup, group_plans, group_title};
use crate::payload::{Account, ProviderStatus};
use crate::popup_model::status::{StatusView, status_view};
use crate::preferences::registry::ProviderLinks;
use crate::ui::combined_row::MountedWindowRow;
use crate::ui::context::{Ctx, ShareTarget};
use crate::ui::header::{HeaderInput, links_of, section_header};
use crate::ui::header_menu::MenuInput;
use crate::ui::keyed::{Keyed, LookKey, SharedTick, arrange};
use crate::ui::status_notice::{status_mark, status_notice};
use crate::ui::widgets::column;

pub struct GroupInput<'a> {
    pub group: &'a CombinedGroup,
    pub members: &'a [Account],
    pub status: Option<&'a ProviderStatus>,
    pub now: Timestamp,
}

#[derive(PartialEq)]
struct HeaderKey {
    provider: String,
    title: String,
    plan: Option<String>,
    incident: Option<StatusView>,
    menu: MenuInput,
}

impl HeaderKey {
    fn of(ctx: &Ctx, input: &GroupInput) -> Self {
        let group = input.group;
        let starred = group
            .account_ids
            .iter()
            .any(|id| ctx.display.is_starred(id));
        Self {
            provider: group.provider.clone(),
            title: group_title(ctx.locale.lang, group),
            plan: group_plans(group),
            incident: incident(ctx, input),
            menu: MenuInput {
                provider_name: group.provider_name.clone(),
                account_ids: group.account_ids.clone(),
                starred,
                target: ShareTarget::Combined(group.provider.clone()),
                links: links_of(ctx, &group.provider),
            },
        }
    }
}

fn incident(ctx: &Ctx, input: &GroupInput) -> Option<StatusView> {
    input
        .status
        .and_then(|status| status_view(ctx.locale.lang, status))
}

fn header(ctx: &Ctx, key: &HeaderKey) -> gtk::Widget {
    let input = HeaderInput {
        title: key.title.clone(),
        plan: key.plan.clone(),
        status: key
            .incident
            .iter()
            .map(|view| status_mark(view).upcast())
            .collect(),
        menu: key.menu.clone(),
    };
    section_header(ctx, &key.provider, input).upcast()
}

fn members(input: &GroupInput) -> Vec<Account> {
    input
        .members
        .iter()
        .filter(|account| input.group.account_ids.contains(&account.id))
        .cloned()
        .collect()
}

#[derive(PartialEq)]
struct GroupKey {
    look: LookKey,
    group: CombinedGroup,
    members: Vec<Account>,
    status: Option<ProviderStatus>,
    links: Option<ProviderLinks>,
}

impl GroupKey {
    fn of(ctx: &Ctx, input: &GroupInput) -> Self {
        Self {
            look: LookKey::of(ctx),
            group: input.group.clone(),
            members: members(input),
            status: input.status.cloned(),
            links: ctx.links.get(&input.group.provider).cloned(),
        }
    }
}

pub struct MountedGroup {
    pub widget: gtk::Box,
    pub ticks: Vec<SharedTick>,
    key: Option<GroupKey>,
    header: Option<Keyed<HeaderKey>>,
    card: gtk::Box,
    incident: Option<Keyed<StatusView>>,
    rows: Vec<(String, MountedWindowRow)>,
}

impl MountedGroup {
    pub fn new(ctx: &Ctx, input: &GroupInput) -> Self {
        let widget = column(0, &[]);
        let card = column(0, &["headroom-card"]);
        widget.append(&card);
        let mut group = Self {
            widget,
            ticks: Vec::new(),
            key: None,
            header: None,
            card,
            incident: None,
            rows: Vec::new(),
        };
        group.update(ctx, input);
        group
    }

    pub fn update(&mut self, ctx: &Ctx, input: &GroupInput) {
        let key = GroupKey::of(ctx, input);
        match self.key.take() {
            Some(old) if old == key => {
                self.key = Some(old);
                return;
            }
            Some(old) if old.look != key.look => self.reset(),
            _ => {}
        }
        self.key = Some(key);
        self.widget.set_spacing(if ctx.compact() { 2 } else { 4 });
        self.header(ctx, input);
        let mut order: Vec<gtk::Widget> = self.incident(ctx, input).into_iter().collect();
        self.rows(ctx, input, &mut order);
        arrange(&self.card, &order, None);
        self.card.set_visible(!order.is_empty());
        let incident = self
            .incident
            .iter()
            .flat_map(|slot| slot.ticks.iter().cloned());
        let rows = self
            .rows
            .iter()
            .map(|(_, row)| std::rc::Rc::clone(&row.tick));
        self.ticks = incident.chain(rows).collect();
    }

    fn reset(&mut self) {
        if let Some(header) = self.header.take() {
            self.widget.remove(&header.widget);
        }
        self.incident = None;
        self.rows.clear();
        self.widget.remove(&self.card);
        self.card = column(0, &["headroom-card"]);
        self.widget.append(&self.card);
    }

    fn header(&mut self, ctx: &Ctx, input: &GroupInput) {
        let previous = self.header.take();
        let old = previous.as_ref().map(|kept| kept.widget.clone());
        let key = HeaderKey::of(ctx, input);
        let (kept, built) = Keyed::reuse(previous, key, ctx, |key| header(ctx, key));
        if built {
            if let Some(old) = old {
                self.widget.remove(&old);
            }
            self.widget.prepend(&kept.widget);
        }
        self.header = Some(kept);
    }

    fn incident(&mut self, ctx: &Ctx, input: &GroupInput) -> Option<gtk::Widget> {
        let Some(view) = incident(ctx, input) else {
            self.incident = None;
            return None;
        };
        let (kept, _) = Keyed::reuse(self.incident.take(), view, ctx, |view| {
            status_notice(ctx, view, input.now, false).upcast()
        });
        let widget = kept.widget.clone();
        self.incident = Some(kept);
        Some(widget)
    }

    fn rows(&mut self, ctx: &Ctx, input: &GroupInput, order: &mut Vec<gtk::Widget>) {
        let members = members(input);
        let mut previous = std::mem::take(&mut self.rows);
        for window in &input.group.windows {
            let row = match previous.iter().position(|(id, _)| id == &window.id) {
                Some(index) => {
                    let (id, row) = previous.swap_remove(index);
                    row.update(ctx, window, &members, input.now);
                    (id, row)
                }
                None => (
                    window.id.clone(),
                    MountedWindowRow::new(ctx, window, &members, input.now),
                ),
            };
            order.push(row.1.widget.clone().upcast());
            self.rows.push(row);
        }
    }
}
