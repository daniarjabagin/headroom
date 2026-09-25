use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::shows_name;
use crate::payload::{Account, State};
use crate::ui::account_section::SectionInput;
use crate::ui::combined_section::combined_section;
use crate::ui::context::Ctx;
use crate::ui::footer::{footer, refresh_button, top_bar};
use crate::ui::spend_card::spend_section;
use crate::ui::status_views::{empty_view, error_view, loading_sections, service_view};
use crate::ui::update_row::update_row;
use crate::ui::widgets::column;
use crate::view::View;

const CONTENT_SPACING: i32 = 14;

pub struct Frame {
    pub now: Timestamp,
    pub max_height: i32,
    pub animate: bool,
}

pub enum Entry<'a> {
    Widget(gtk::Widget),
    Account(SectionInput<'a>),
}

pub fn ready_entries<'a>(ctx: &Ctx, state: &'a State, frame: &Frame) -> Vec<Entry<'a>> {
    let accounts: Vec<&Account> = state.accounts.iter().filter(|a| !a.hidden).collect();
    let spend = (ctx.display.show_spend && !state.usage.is_empty()).then_some(&state.spend);
    if accounts.is_empty() && spend.is_none() {
        return vec![Entry::Widget(empty_view(ctx).upcast())];
    }
    let refresh = refresh_button(ctx);
    let leading: gtk::Widget = match spend {
        Some(spend) => spend_section(ctx, spend, &refresh, frame.animate).upcast(),
        None => top_bar(&refresh).upcast(),
    };
    let sections = accounts
        .iter()
        .filter_map(|account| entry(ctx, state, account, &accounts, frame));
    std::iter::once(Entry::Widget(leading))
        .chain(sections)
        .collect()
}

fn entry<'a>(
    ctx: &Ctx,
    state: &'a State,
    account: &'a Account,
    accounts: &[&Account],
    frame: &Frame,
) -> Option<Entry<'a>> {
    let group = state
        .combined
        .iter()
        .find(|group| group.account_ids.contains(&account.id));
    if let Some(group) = group {
        let first = group
            .account_ids
            .iter()
            .find(|id| accounts.iter().any(|a| &a.id == *id));
        return (first == Some(&account.id)).then(|| {
            Entry::Widget(combined_section(ctx, group, &state.accounts, frame.now).upcast())
        });
    }
    Some(Entry::Account(SectionInput {
        account,
        usage: state.usage_of(account),
        show_name: shows_name(account, accounts),
        now: frame.now,
        animate: frame.animate,
    }))
}

pub fn view_widgets(ctx: &Ctx, view: &View) -> Vec<gtk::Widget> {
    match view {
        View::Loading => loading_sections().into_iter().map(Cast::upcast).collect(),
        View::Unavailable { .. } => vec![service_view(ctx).upcast()],
        View::Failed(message) => vec![error_view(ctx, message).upcast()],
        View::Ready(_) => Vec::new(),
    }
}

#[must_use]
pub fn content_column() -> gtk::Box {
    column(CONTENT_SPACING, &["headroom-content"])
}

#[must_use]
pub fn scroller(content: &gtk::Box, frame: &Frame) -> gtk::ScrolledWindow {
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_propagate_natural_height(true);
    scroller.set_max_content_height(frame.max_height);
    scroller.set_overlay_scrolling(true);
    scroller.set_vexpand(true);
    scroller.set_child(Some(content));
    scroller
}

pub fn tail(ctx: &Ctx, view: &View, now: Timestamp) -> Vec<gtk::Widget> {
    let update = view
        .state()
        .and_then(|state| state.update.as_ref())
        .map(|update| update_row(ctx, update).upcast());
    update
        .into_iter()
        .chain(std::iter::once(footer(ctx, view, now).upcast()))
        .collect()
}

pub fn root_column(ctx: &Ctx) -> gtk::Box {
    let root = column(0, &["headroom-popup"]);
    if !ctx.motion {
        root.add_css_class("reduced-motion");
    }
    root
}
