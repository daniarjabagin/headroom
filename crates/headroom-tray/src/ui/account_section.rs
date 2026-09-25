use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::{HeaderStatus, NoticeView, account_title, header_status, shown_plan};
use crate::format::ago_text;
use crate::i18n::fill;
use crate::payload::{Account, ProviderStatus, Usage, Window};
use crate::popup_model::status::status_view;
use crate::preferences::registry::ProviderLinks;
use crate::ui::context::{Action, Ctx, ShareTarget};
use crate::ui::header::{HeaderInput, links_of, section_header};
use crate::ui::header_menu::MenuInput;
use crate::ui::keyed::{LookKey, SharedTick, capture};
use crate::ui::notice::MountedNotice;
use crate::ui::quota_row::quota_row;
use crate::ui::section_card::{Card, button_state, card, clear_except, mount_notice};
use crate::ui::status_notice::{status_mark, status_notice};
use crate::ui::usage_rows::{extra_rows, trend_row};
use crate::ui::widgets::{button, column, icon, label};

pub struct SectionInput<'a> {
    pub account: &'a Account,
    pub usage: Option<&'a Usage>,
    pub status: Option<&'a ProviderStatus>,
    pub show_name: bool,
    pub now: Timestamp,
    pub animate: bool,
}

fn outdated_tag(ctx: &Ctx, account: &Account) -> gtk::Widget {
    let lang = ctx.locale.lang;
    let tag = label(lang.tr("Outdated"), &["headroom-stale-tag"]);
    if let Some(at) = account.updated_at {
        let shown = tag.clone();
        let tip = move |now: Timestamp| {
            let text = fill(
                lang.tr("Last updated {ago}"),
                &[("ago", &ago_text(lang, at, now))],
            );
            shown.set_tooltip_text(Some(&text));
        };
        tip(Timestamp::now());
        ctx.on_tick(tip);
    }
    tag.upcast()
}

fn status_widget(ctx: &Ctx, account: &Account) -> Option<gtk::Widget> {
    let lang = ctx.locale.lang;
    match header_status(account, ctx.offline)? {
        HeaderStatus::Refreshing => {
            let spinner = gtk::Spinner::new();
            spinner.set_spinning(ctx.motion);
            spinner.set_size_request(12, 12);
            Some(spinner.upcast())
        }
        HeaderStatus::Outdated => Some(outdated_tag(ctx, account)),
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
    let incident = input
        .status
        .and_then(|status| status_view(ctx.locale.lang, status));
    let status = status_widget(ctx, account)
        .into_iter()
        .chain(incident.as_ref().map(|view| status_mark(view).upcast()))
        .collect();
    let menu = MenuInput {
        provider_name: account.provider_name.clone(),
        account_ids: vec![account.id.clone()],
        starred: ctx.display.is_starred(&account.id),
        target: ShareTarget::Account(account.id.clone()),
        links: links_of(ctx, &account.provider),
    };
    let header = HeaderInput {
        title: account_title(account, input.show_name),
        plan: shown_plan(account).map(str::to_owned),
        status,
        menu,
    };
    section_header(ctx, &account.provider, header)
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

#[derive(PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent input that changes how the section looks"
)]
struct SectionKey {
    look: LookKey,
    account: Account,
    usage: Option<Usage>,
    status: Option<ProviderStatus>,
    links: Option<ProviderLinks>,
    show_name: bool,
    sign_in: bool,
    expanded: bool,
    retrying: bool,
    copied: bool,
}

impl SectionKey {
    fn of(ctx: &Ctx, input: &SectionInput) -> Self {
        let account = input.account;
        Self {
            look: LookKey::of(ctx),
            account: account.clone(),
            usage: input.usage.cloned(),
            status: input.status.cloned(),
            links: ctx.links.get(&account.provider).cloned(),
            show_name: input.show_name,
            sign_in: ctx.sign_in.contains(&account.provider),
            expanded: ctx.ui.expanded.contains(&account.id),
            retrying: ctx.ui.retrying.contains(&account.id),
            copied: ctx.ui.copied_command.as_deref() == Some(account.id.as_str()),
        }
    }
}

pub struct MountedSection {
    pub widget: gtk::Box,
    pub ticks: Vec<SharedTick>,
    header: gtk::Box,
    card: gtk::Box,
    alert: Option<MountedAlert>,
    key: Option<SectionKey>,
}

impl MountedSection {
    pub fn new(ctx: &Ctx, input: &SectionInput) -> Self {
        let widget = column(if ctx.compact() { 2 } else { 4 }, &[]);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let card = column(0, &["headroom-card"]);
        widget.append(&header);
        widget.append(&card);
        let mut section = Self {
            widget,
            ticks: Vec::new(),
            header,
            card,
            alert: None,
            key: None,
        };
        section.update(ctx, input);
        section
    }

    pub fn update(&mut self, ctx: &Ctx, input: &SectionInput) {
        let key = SectionKey::of(ctx, input);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let appeared = self.key.is_some()
            && self.key.as_ref().is_some_and(|old| old.status.is_none())
            && key.status.is_some();
        let ((), ticks) = capture(ctx, || {
            let header = header(ctx, input);
            self.widget.prepend(&header);
            self.widget.remove(&self.header);
            self.header = header;
            self.widget.set_spacing(if ctx.compact() { 2 } else { 4 });
            self.fill_card(ctx, input, appeared);
        });
        self.ticks = ticks;
        self.key = Some(key);
    }

    fn fill_card(&mut self, ctx: &Ctx, input: &SectionInput, appeared: bool) {
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
        let incident = input
            .status
            .and_then(|status| status_view(ctx.locale.lang, status));
        if let Some(view) = incident {
            self.card
                .append(&status_notice(ctx, &view, input.now, appeared));
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
