use gtk::prelude::*;
use jiff::Timestamp;

use crate::ui::context::{Action, Ctx, RefreshMode};
use crate::ui::widgets::{button, column, icon, label, row};
use crate::view::{View, footer_status};

const REFRESH_ICON: i32 = 14;
const FOOTER_ICON: i32 = 16;

pub fn refresh_button(ctx: &Ctx) -> gtk::Widget {
    let lang = ctx.locale.lang;
    let mode = ctx.ui.refresh;
    let child: gtk::Widget = if mode == RefreshMode::Busy && ctx.motion {
        let spinner = gtk::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_size_request(REFRESH_ICON, REFRESH_ICON);
        spinner.upcast()
    } else {
        icon("view-refresh-symbolic", REFRESH_ICON, &[]).upcast()
    };
    let refresh = button(
        &child,
        &["headroom-icon-button", "headroom-refresh-button"],
        ctx.action(Action::RefreshNow),
    );
    refresh.set_valign(gtk::Align::Center);
    match mode {
        RefreshMode::Busy => {
            refresh.add_css_class("busy");
            refresh.set_opacity(if ctx.motion { 1.0 } else { 0.55 });
        }
        RefreshMode::Failed => refresh.add_css_class("failed"),
        RefreshMode::Idle => {}
    }
    let tip = if mode == RefreshMode::Failed {
        "Refresh failed"
    } else {
        "Refresh"
    };
    refresh.set_tooltip_text(Some(lang.tr(tip)));
    refresh.upcast()
}

pub fn top_bar(refresh: &gtk::Widget) -> gtk::Box {
    let bar = row(0, &["headroom-top-bar"]);
    refresh.set_hexpand(true);
    refresh.set_halign(gtk::Align::End);
    bar.append(refresh);
    bar
}

fn status_label(ctx: &Ctx, view: &View, now: Timestamp) -> gtk::Button {
    let status = label("", &["headroom-footer-text"]);
    let link = button(
        &status,
        &["headroom-footer-link"],
        ctx.action(Action::RefreshNow),
    );
    link.set_halign(gtk::Align::Start);
    link.set_sensitive(view.state().is_some());
    let show = {
        let (status, link, view, locale) = (
            status.clone(),
            link.clone(),
            view.clone(),
            ctx.locale.clone(),
        );
        move |now: Timestamp| {
            let status_line = footer_status(&view, &locale, now);
            status.set_text(&status_line.text);
            if status_line.notice {
                status.add_css_class("notice");
            } else {
                status.remove_css_class("notice");
            }
            link.set_visible(!status_line.text.is_empty());
        }
    };
    show(now);
    ctx.on_tick(show);
    link
}

pub fn footer(ctx: &Ctx, view: &View, now: Timestamp) -> gtk::Box {
    let bar = row(12, &["headroom-footer"]);
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.set_valign(gtk::Align::Center);
    texts.append(&label(&ctx.version, &["headroom-footer-text"]));
    texts.append(&status_label(ctx, view, now));
    bar.append(&texts);
    let quit = button(
        &icon("application-exit-symbolic", FOOTER_ICON, &[]),
        &["headroom-icon-button"],
        ctx.action(Action::Quit),
    );
    quit.set_valign(gtk::Align::Center);
    quit.set_tooltip_text(Some(ctx.locale.lang.tr("Quit")));
    bar.append(&quit);
    bar
}
