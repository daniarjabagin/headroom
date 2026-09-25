use gtk::prelude::*;
use jiff::Timestamp;

use crate::popup_model::status::{StatusView, elapsed_text, started_text};
use crate::ui::breakdown::dot;
use crate::ui::context::{Action, Ctx};
use crate::ui::motion::{SLIDE_MS, fade};
use crate::ui::widgets::{button, column, icon, label, row, spacer, wrapping_label};

const TILE_ICON: i32 = 14;
const LINK_ICON: i32 = 10;

fn link(ctx: &Ctx, url: &str, class: &str) -> gtk::Button {
    let content = row(3, &[]);
    content.append(&label(
        ctx.locale.lang.tr("Status page"),
        &["headroom-status-link-text", class],
    ));
    let arrow = icon(
        "adw-external-link-symbolic",
        LINK_ICON,
        &["headroom-status-link-icon"],
    );
    arrow.set_valign(gtk::Align::Center);
    content.append(&arrow);
    let link = button(
        &content,
        &["headroom-status-link"],
        ctx.action(Action::OpenUrl(url.to_owned())),
    );
    link.set_tooltip_text(Some(url));
    link.set_valign(gtk::Align::Center);
    link
}

fn ticking(
    ctx: &Ctx,
    text: &gtk::Label,
    since: Timestamp,
    show: fn(crate::i18n::Lang, Timestamp, Timestamp) -> String,
) {
    let (lang, text) = (ctx.locale.lang, text.clone());
    ctx.on_tick(move |now| text.set_text(&show(lang, since, now)));
}

fn tile(view: &StatusView) -> gtk::Box {
    let tile = row(0, &["headroom-notice-tile"]);
    tile.set_valign(gtk::Align::Start);
    let name = if view.critical {
        "dialog-error-symbolic"
    } else {
        "dialog-warning-symbolic"
    };
    let image = icon(name, TILE_ICON, &[]);
    image.set_hexpand(true);
    image.set_halign(gtk::Align::Center);
    tile.append(&image);
    tile
}

fn full_notice(ctx: &Ctx, view: &StatusView, now: Timestamp) -> gtk::Box {
    let kind = if view.critical { "error" } else { "warning" };
    let body = row(8, &["headroom-notice", "headroom-status-notice", kind]);
    body.append(&tile(view));
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.append(&wrapping_label(&view.title, &["headroom-notice-title"]));
    if let Some(detail) = &view.detail {
        texts.append(&wrapping_label(detail, &["headroom-notice-detail"]));
    }
    let meta = row(5, &["headroom-status-meta"]);
    if let Some(since) = view.started_at {
        let started = label(
            &started_text(ctx.locale.lang, since, now),
            &["headroom-notice-detail"],
        );
        started.set_ellipsize(gtk::pango::EllipsizeMode::End);
        ticking(ctx, &started, since, started_text);
        meta.append(&started);
    }
    meta.append(&spacer());
    meta.append(&link(ctx, &view.url, "full"));
    texts.append(&meta);
    body.append(&texts);
    body
}

fn compact_notice(ctx: &Ctx, view: &StatusView, now: Timestamp) -> gtk::Box {
    let line = row(7, &["headroom-status-line"]);
    let tone = if view.critical { "crit" } else { "notice" };
    line.append(&dot(ctx.color(tone)));
    line.append(&label(&view.kind, &["headroom-status-line-title"]));
    if let Some(since) = view.started_at {
        let elapsed = label(
            &format!("· {}", elapsed_text(ctx.locale.lang, since, now)),
            &["headroom-status-line-meta"],
        );
        let (lang, shown) = (ctx.locale.lang, elapsed.clone());
        ctx.on_tick(move |now| shown.set_text(&format!("· {}", elapsed_text(lang, since, now))));
        line.append(&elapsed);
    }
    line.append(&spacer());
    line.append(&link(ctx, &view.url, "compact"));
    line.set_tooltip_text(Some(&view.title));
    line
}

pub fn status_notice(ctx: &Ctx, view: &StatusView, now: Timestamp, animate: bool) -> gtk::Box {
    let notice = if ctx.compact() {
        compact_notice(ctx, view, now)
    } else {
        full_notice(ctx, view, now)
    };
    if animate {
        fade(&notice, 0.0, 1.0, SLIDE_MS, ctx.motion);
    }
    notice
}

pub fn status_mark(view: &StatusView) -> gtk::Image {
    let (name, class) = if view.critical {
        ("dialog-error-symbolic", "headroom-header-critical")
    } else {
        ("dialog-warning-symbolic", "headroom-header-warning")
    };
    let mark = icon(name, 12, &[class]);
    mark.set_tooltip_text(Some(&view.title));
    mark.set_valign(gtk::Align::Center);
    mark
}
