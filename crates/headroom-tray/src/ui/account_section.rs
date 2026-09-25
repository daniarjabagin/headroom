use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::{HeaderStatus, NoticeView, account_title, header_status, shown_plan};
use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::format::ago_text;
use crate::i18n::fill;
use crate::payload::{Account, Usage, Window};
use crate::ui::context::{Action, Ctx};
use crate::ui::notice::MountedNotice;
use crate::ui::quota_row::quota_row;
use crate::ui::section_card::{Card, button_state, card, clear_except, mount_notice};
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

#[derive(PartialEq)]
struct AlertKey {
    notice: NoticeView,
    motion: bool,
}

struct MountedAlert {
    key: AlertKey,
    notice: MountedNotice,
}

pub struct MountedSection {
    pub widget: gtk::Box,
    header: gtk::Box,
    card: gtk::Box,
    alert: Option<MountedAlert>,
}

impl MountedSection {
    pub fn new(ctx: &Ctx, input: &SectionInput) -> Self {
        let widget = column(4, &[]);
        let header = header(ctx, input);
        let card = column(0, &["headroom-card"]);
        widget.append(&header);
        widget.append(&card);
        let mut section = Self {
            widget,
            header,
            card,
            alert: None,
        };
        section.fill_card(ctx, input);
        section
    }

    pub fn update(&mut self, ctx: &Ctx, input: &SectionInput) {
        let header = header(ctx, input);
        self.widget.prepend(&header);
        self.widget.remove(&self.header);
        self.header = header;
        self.fill_card(ctx, input);
    }

    fn fill_card(&mut self, ctx: &Ctx, input: &SectionInput) {
        let account = input.account;
        let Card {
            alert,
            rest,
            windows,
        } = card(ctx, account);
        self.keep_alert(ctx, account, alert);
        let kept = self.alert.as_ref().map(|alert| &alert.notice.widget);
        clear_except(&self.card, kept);
        if let Some(alert) = &self.alert {
            alert
                .notice
                .apply(ctx.locale.lang, |_| button_state(ctx, account));
            if alert.notice.widget.parent().is_none() {
                self.card.prepend(&alert.notice.widget);
            }
        }
        for widget in &rest {
            self.card.append(widget);
        }
        if let Some(windows) = windows {
            limits(ctx, input, &self.card, &windows);
        }
        self.card.set_visible(self.card.first_child().is_some());
    }

    fn keep_alert(&mut self, ctx: &Ctx, account: &Account, alert: Option<NoticeView>) {
        let Some(notice) = alert else {
            self.alert = None;
            return;
        };
        let key = AlertKey {
            notice,
            motion: ctx.motion,
        };
        if self
            .alert
            .as_ref()
            .is_some_and(|mounted| mounted.key == key)
        {
            return;
        }
        let notice = mount_notice(ctx, account, &key.notice);
        self.alert = Some(MountedAlert { key, notice });
    }
}
