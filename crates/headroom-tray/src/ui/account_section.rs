use gtk::prelude::*;
use jiff::Timestamp;

use crate::account::{HeaderStatus, account_title, header_mark, header_status, shown_plan};
use crate::format::ago_text;
use crate::i18n::fill;
use crate::payload::{Account, ProviderStatus, Usage};
use crate::popup_model::status::{StatusView, status_view};
use crate::preferences::registry::ProviderLinks;
use crate::ui::context::{Ctx, ShareTarget};
use crate::ui::header::{HeaderInput, links_of, section_header};
use crate::ui::header_menu::MenuInput;
use crate::ui::keyed::{Keyed, LookKey, SharedTick};
use crate::ui::section_body::MountedBody;
use crate::ui::status_notice::status_mark;
use crate::ui::widgets::{column, icon, label};

pub struct SectionInput<'a> {
    pub account: &'a Account,
    pub usage: Option<&'a Usage>,
    pub status: Option<&'a ProviderStatus>,
    pub show_name: bool,
    pub now: Timestamp,
    pub animate: bool,
}

#[derive(PartialEq)]
struct HeaderKey {
    provider: String,
    title: String,
    plan: Option<String>,
    status: Option<HeaderStatus>,
    updated_at: Option<Timestamp>,
    error: Option<String>,
    incident: Option<StatusView>,
    menu: MenuInput,
}

impl HeaderKey {
    fn of(ctx: &Ctx, input: &SectionInput) -> Self {
        let account = input.account;
        let status = header_status(account, ctx.offline);
        Self {
            provider: account.provider.clone(),
            title: account_title(account, input.show_name),
            plan: shown_plan(account).map(str::to_owned),
            status,
            updated_at: account
                .updated_at
                .filter(|_| status == Some(HeaderStatus::Outdated)),
            error: account
                .error
                .as_ref()
                .filter(|_| status == Some(HeaderStatus::Error))
                .map(|error| error.message.clone()),
            incident: input
                .status
                .and_then(|status| status_view(ctx.locale.lang, status)),
            menu: MenuInput {
                provider_name: account.provider_name.clone(),
                account_ids: vec![account.id.clone()],
                starred: ctx.display.is_starred(&account.id),
                target: ShareTarget::Account(account.id.clone()),
                links: links_of(ctx, &account.provider),
            },
        }
    }
}

fn outdated_tag(ctx: &Ctx, updated_at: Option<Timestamp>) -> gtk::Widget {
    let lang = ctx.locale.lang;
    let tag = label(lang.tr("Outdated"), &["headroom-stale-tag"]);
    if let Some(at) = updated_at {
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

fn error_text<'a>(ctx: &Ctx, key: &'a HeaderKey) -> &'a str {
    key.error
        .as_deref()
        .unwrap_or(ctx.locale.lang.tr("Refresh failed"))
}

fn status_widget(ctx: &Ctx, key: &HeaderKey) -> Option<gtk::Widget> {
    match header_mark(key.status, key.incident.is_some())? {
        HeaderStatus::Refreshing => {
            let spinner = gtk::Spinner::new();
            spinner.set_spinning(ctx.motion);
            spinner.set_size_request(12, 12);
            Some(spinner.upcast())
        }
        HeaderStatus::Outdated => Some(outdated_tag(ctx, key.updated_at)),
        HeaderStatus::Error => {
            let warning = icon("dialog-warning-symbolic", 12, &["headroom-header-warning"]);
            warning.set_tooltip_text(Some(error_text(ctx, key)));
            Some(warning.upcast())
        }
    }
}

fn incident_mark(ctx: &Ctx, key: &HeaderKey, view: &StatusView) -> gtk::Widget {
    let mark = status_mark(view);
    if key.status == Some(HeaderStatus::Error) {
        let tooltip = format!("{}\n{}", view.title, error_text(ctx, key));
        mark.set_tooltip_text(Some(&tooltip));
    }
    mark.upcast()
}

fn header(ctx: &Ctx, key: &HeaderKey) -> gtk::Box {
    let status = status_widget(ctx, key)
        .into_iter()
        .chain(
            key.incident
                .as_ref()
                .map(|view| incident_mark(ctx, key, view)),
        )
        .collect();
    let header = HeaderInput {
        title: key.title.clone(),
        plan: key.plan.clone(),
        status,
        menu: key.menu.clone(),
    };
    section_header(ctx, &key.provider, header)
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
    header: Option<Keyed<HeaderKey>>,
    body: MountedBody,
    key: Option<SectionKey>,
}

impl MountedSection {
    pub fn new(ctx: &Ctx, input: &SectionInput) -> Self {
        let widget = column(0, &[]);
        let body = MountedBody::new();
        widget.append(&body.card);
        let mut section = Self {
            widget,
            ticks: Vec::new(),
            header: None,
            body,
            key: None,
        };
        section.update(ctx, input);
        section
    }

    pub fn update(&mut self, ctx: &Ctx, input: &SectionInput) {
        let key = SectionKey::of(ctx, input);
        let Some(old) = self.key.take() else {
            self.mount(ctx, input, false);
            self.key = Some(key);
            return;
        };
        if old == key {
            self.key = Some(old);
            return;
        }
        if old.look != key.look {
            self.reset();
        }
        let appeared = old.status.is_none() && key.status.is_some();
        self.mount(ctx, input, appeared);
        self.key = Some(key);
    }

    fn reset(&mut self) {
        if let Some(header) = self.header.take() {
            self.widget.remove(&header.widget);
        }
        self.widget.remove(&self.body.card);
        self.body = MountedBody::new();
        self.widget.append(&self.body.card);
    }

    fn mount(&mut self, ctx: &Ctx, input: &SectionInput, appeared: bool) {
        self.widget.set_spacing(if ctx.compact() { 2 } else { 4 });
        let key = HeaderKey::of(ctx, input);
        let previous = self.header.take();
        let old_widget = previous.as_ref().map(|kept| kept.widget.clone());
        let (kept, built) = Keyed::reuse(previous, key, ctx, |key| header(ctx, key).upcast());
        if built {
            if let Some(old) = old_widget {
                self.widget.remove(&old);
            }
            self.widget.prepend(&kept.widget);
        }
        self.header = Some(kept);
        self.body.update(ctx, input, appeared);
        let header_ticks = self
            .header
            .iter()
            .flat_map(|kept| kept.ticks.iter().cloned());
        self.ticks = header_ticks.chain(self.body.ticks()).collect();
    }
}
