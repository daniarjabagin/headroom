use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::shows_name;
use crate::combined::CombinedGroup;
use crate::payload::{Account, Spend, State};
use crate::popup_model::collapse::{
    Attention, MoreMember, MoreSummary, account_attention, group_attention, more_summary,
};
use crate::ui::account_section::SectionInput;
use crate::ui::combined_section::GroupInput;
use crate::ui::context::Ctx;
use crate::ui::status_views::{error_view, loading_sections, service_view};
use crate::ui::widgets::column;
use crate::view::View;

const CONTENT_SPACING: i32 = 14;
const COMPACT_CONTENT_SPACING: i32 = 8;

pub struct Frame {
    pub now: Timestamp,
    pub max_height: i32,
    pub animate: bool,
}

pub enum Entry<'a> {
    Empty,
    Leading(Option<&'a Spend>),
    Account(SectionInput<'a>),
    Group(GroupInput<'a>),
    More(MoreSummary),
    Less,
}

struct Placed<'a> {
    entry: Entry<'a>,
    collapsed: bool,
    member: MoreMember<'a>,
}

fn group_of<'a>(state: &'a State, account: &Account) -> Option<&'a CombinedGroup> {
    state
        .combined
        .iter()
        .find(|group| group.account_ids.contains(&account.id))
}

fn member(account: &Account, attention: Option<Attention>) -> MoreMember<'_> {
    MoreMember {
        provider: account.provider.as_str(),
        name: account.provider_name.as_str(),
        attention,
    }
}

fn place<'a>(
    state: &'a State,
    account: &'a Account,
    accounts: &[&Account],
    frame: &Frame,
) -> Option<Placed<'a>> {
    let status = state.provider_status_of(&account.provider);
    if let Some(group) = group_of(state, account) {
        let first = group
            .account_ids
            .iter()
            .find(|id| accounts.iter().any(|a| &a.id == *id));
        return (first == Some(&account.id)).then(|| Placed {
            entry: Entry::Group(GroupInput {
                group,
                members: &state.accounts,
                status,
                now: frame.now,
            }),
            collapsed: group.collapsed,
            member: member(account, group_attention(group, accounts, state.offline)),
        });
    }
    Some(Placed {
        entry: Entry::Account(SectionInput {
            account,
            usage: state.usage_of(account),
            status,
            show_name: shows_name(account, accounts),
            now: frame.now,
            animate: frame.animate,
        }),
        collapsed: account.collapsed,
        member: member(account, account_attention(account, state.offline)),
    })
}

pub fn ready_entries<'a>(ctx: &Ctx, state: &'a State, frame: &Frame) -> Vec<Entry<'a>> {
    let accounts: Vec<&Account> = state.accounts.iter().filter(|a| !a.hidden).collect();
    let spend = (ctx.display.show_spend && !state.usage.is_empty()).then_some(&state.spend);
    if accounts.is_empty() && spend.is_none() {
        return vec![Entry::Empty];
    }
    let (collapsed, pinned): (Vec<Placed>, Vec<Placed>) = accounts
        .iter()
        .filter_map(|account| place(state, account, &accounts, frame))
        .partition(|placed| placed.collapsed);
    let members: Vec<MoreMember> = collapsed.iter().map(|placed| placed.member).collect();
    let mut entries = vec![Entry::Leading(spend)];
    entries.extend(pinned.into_iter().map(|placed| placed.entry));
    let Some(summary) = more_summary(ctx.locale.lang, &members) else {
        return entries;
    };
    if ctx.ui.more_expanded {
        entries.push(Entry::Less);
        entries.extend(collapsed.into_iter().map(|placed| placed.entry));
    } else {
        entries.push(Entry::More(summary));
    }
    entries
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
pub fn content_column(ctx: &Ctx) -> gtk::Box {
    let content = column(CONTENT_SPACING, &["headroom-content"]);
    set_density(ctx, &content);
    content
}

pub fn set_density(ctx: &Ctx, content: &gtk::Box) {
    content.set_spacing(if ctx.compact() {
        COMPACT_CONTENT_SPACING
    } else {
        CONTENT_SPACING
    });
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

pub fn root_column(ctx: &Ctx) -> gtk::Box {
    let root = column(0, &["headroom-popup"]);
    apply_root_classes(ctx, &root);
    root
}

fn toggle_class(widget: &gtk::Box, class: &str, on: bool) {
    if on {
        widget.add_css_class(class);
    } else {
        widget.remove_css_class(class);
    }
}

pub fn apply_root_classes(ctx: &Ctx, root: &gtk::Box) {
    toggle_class(root, "reduced-motion", !ctx.motion);
    toggle_class(root, "compact", ctx.compact());
}
