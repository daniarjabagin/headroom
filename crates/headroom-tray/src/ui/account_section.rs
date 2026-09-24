use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::{
    CardBody, HeaderStatus, NoticeView, account_title, card_body, header_status, is_retrying,
    is_signed_out, shown_plan,
};
use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::format::ago_text;
use crate::i18n::fill;
use crate::notices::NoticeKind;
use crate::payload::{Account, Usage, Window};
use crate::ui::context::{Action, Ctx};
use crate::ui::notice::{NoticeActions, notice_line, notice_row};
use crate::ui::quota_row::quota_row;
use crate::ui::status_views::skeleton_rows;
use crate::ui::usage_rows::{extra_rows, trend_row};
use crate::ui::widgets::{button, column, icon, label, row, svg_image};

const PROVIDER_ICON: i32 = 16;

pub struct SectionInput<'a> {
    pub account: &'a Account,
    pub usage: Option<&'a Usage>,
    pub show_name: bool,
    pub now: Timestamp,
    pub animate: bool,
}

pub fn provider_icon(ctx: &Ctx, provider: &str) -> gtk::Image {
    let (svg, tint) = provider_logo(provider);
    let color = match tint {
        Tint::Brand => CLAUDE_COLOR.to_owned(),
        Tint::Text => ctx.css("text-secondary"),
    };
    svg_image(svg, &color, PROVIDER_ICON, &["headroom-provider-icon"])
}

fn status_widget(ctx: &Ctx, account: &Account, now: Timestamp) -> Option<gtk::Widget> {
    let lang = ctx.locale.lang;
    match header_status(account, ctx.offline)? {
        HeaderStatus::Refreshing => {
            let spinner = gtk::Spinner::new();
            spinner.set_spinning(ctx.motion);
            spinner.set_size_request(12, 12);
            Some(spinner.upcast())
        }
        HeaderStatus::Outdated => {
            let tag = label(lang.tr("Outdated"), &["headroom-stale-tag"]);
            let tip = account.updated_at.map(|at| {
                fill(
                    lang.tr("Last updated {ago}"),
                    &[("ago", &ago_text(lang, at, now))],
                )
            });
            tag.set_tooltip_text(tip.as_deref());
            Some(tag.upcast())
        }
        HeaderStatus::Error => {
            let warning = icon("dialog-warning-symbolic", 12, &["headroom-header-warning"]);
            let message = account
                .error
                .as_ref()
                .map_or(lang.tr("Refresh failed"), |e| &e.message);
            warning.set_tooltip_text(Some(message));
            Some(warning.upcast())
        }
    }
}

fn header(ctx: &Ctx, input: &SectionInput) -> gtk::Box {
    let account = input.account;
    let line = row(6, &["headroom-section-header"]);
    line.append(&provider_icon(ctx, &account.provider));
    let title = label(
        &account_title(account, input.show_name),
        &["headroom-title"],
    );
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    if let Some(plan) = shown_plan(account) {
        let plan = label(plan, &["headroom-plan"]);
        plan.set_valign(gtk::Align::Baseline);
        line.append(&plan);
    }
    if let Some(status) = status_widget(ctx, account, input.now) {
        status.set_valign(gtk::Align::Center);
        line.append(&status);
    }
    line
}

fn retry(ctx: &Ctx, account: &Account) -> Box<dyn Fn()> {
    Box::new(ctx.action(Action::Refresh(account.id.clone())))
}

fn blocked(ctx: &Ctx, account: &Account, notice: &NoticeView) -> gtk::Box {
    let sign_in = (is_signed_out(account) && ctx.sign_in.contains(&account.provider))
        .then(|| Box::new(ctx.action(Action::SignIn(account.provider.clone()))) as Box<dyn Fn()>);
    let actions = NoticeActions {
        sign_in,
        retry: Some(retry(ctx, account)),
        retrying: is_retrying(account),
    };
    notice_row(ctx, notice, actions)
}

fn notice_widget(ctx: &Ctx, account: &Account, notice: &NoticeView) -> gtk::Box {
    if notice.kind == NoticeKind::Info {
        return notice_line(&notice.title);
    }
    let actions = NoticeActions {
        sign_in: None,
        retry: notice.retry.then(|| retry(ctx, account)),
        retrying: false,
    };
    notice_row(ctx, notice, actions)
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
    let (id, act) = (account.id.clone(), std::rc::Rc::clone(&ctx.act));
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

fn limits(ctx: &Ctx, input: &SectionInput, card: &gtk::Box, windows: &[&Window]) {
    for window in windows {
        card.append(&quota_row(ctx, window, input.now, input.animate));
    }
    let trend = input.usage.filter(|_| ctx.display.show_trend);
    if let Some(usage) = trend {
        card.append(&trend_row(ctx, usage));
    }
    let Some(extras) = extra_rows(ctx, input.account, input.usage) else {
        return;
    };
    if windows.is_empty() && trend.is_none() {
        card.append(&extras);
    } else {
        card.append(&expander(ctx, input.account, &extras));
    }
}

pub fn account_section(ctx: &Ctx, input: &SectionInput) -> gtk::Box {
    let section = column(4, &[]);
    section.append(&header(ctx, input));
    let card = column(0, &["headroom-card"]);
    match card_body(ctx.locale.lang, input.account, ctx.offline) {
        CardBody::Blocked(notice) => card.append(&blocked(ctx, input.account, &notice)),
        CardBody::Skeleton(rows) => card.append(&skeleton_rows(rows)),
        CardBody::Limits { notices, windows } => {
            for notice in &notices {
                card.append(&notice_widget(ctx, input.account, notice));
            }
            limits(ctx, input, &card, &windows);
        }
    }
    card.set_visible(card.first_child().is_some());
    section.append(&card);
    section
}
