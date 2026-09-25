use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::dates::Locale;
use crate::payload::{Display, Window};
use crate::popup_model::compact::{compact_reset, meter_tooltip, reset_hint, value_hint};
use crate::quota::{QuotaView, quota_view};
use crate::ui::context::{Action, Ctx};
use crate::ui::meter::{MeterColors, meter, meter_size};
use crate::ui::widgets::{button, column, label, row, spacer, svg_image, wrapping_label};

const FLAME_SIZE: i32 = 11;
const COMPACT_FLAME_SIZE: i32 = 10;

struct Dynamic {
    flame: gtk::Image,
    note: gtk::Label,
    trailing: gtk::Label,
    forecast: gtk::Label,
}

impl Dynamic {
    fn show(&self, view: &QuotaView) {
        let note = view.note.as_ref();
        self.flame.set_visible(note.is_some_and(|note| note.flame));
        self.note.set_visible(note.is_some());
        self.note
            .set_text(note.map_or("", |note| note.text.as_str()));
        self.trailing.set_text(&view.trailing);
        self.forecast.set_visible(view.forecast.is_some());
        self.forecast
            .set_text(view.forecast.as_deref().unwrap_or(""));
    }
}

fn colors(ctx: &Ctx, view: &QuotaView) -> MeterColors {
    MeterColors {
        track: ctx.color("track"),
        fill: ctx.tone_color(view.tone),
        tick: ctx.color("tick"),
    }
}

fn top_line(view: &QuotaView, dynamic: &Dynamic) -> gtk::Box {
    let top = row(8, &["headroom-row-line"]);
    let title = label(&view.label, &["headroom-metric-label"]);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let notes = row(4, &[]);
    notes.append(&dynamic.flame);
    notes.append(&dynamic.note);
    top.append(&title);
    top.append(&notes);
    top
}

pub fn value_toggle(ctx: &Ctx, text: &gtk::Label) -> gtk::Button {
    let toggle = button(
        text,
        &["headroom-toggle", "reading"],
        ctx.action(Action::ToggleValueMode),
    );
    toggle.set_valign(gtk::Align::Center);
    toggle.set_tooltip_text(Some(value_hint(ctx.locale.lang, ctx.display.value_mode)));
    toggle
}

pub fn reset_toggle(ctx: &Ctx, text: &gtk::Label) -> gtk::Button {
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let toggle = button(
        text,
        &["headroom-toggle", "trailing"],
        ctx.action(Action::ToggleResetFormat),
    );
    toggle.set_valign(gtk::Align::Center);
    toggle.set_tooltip_text(Some(reset_hint(ctx.locale.lang, ctx.display.reset_format)));
    toggle
}

fn bottom_line(ctx: &Ctx, view: &QuotaView, dynamic: &Dynamic) -> gtk::Box {
    let bottom = row(8, &[]);
    let headline = label(&view.headline, &["headroom-reading"]);
    bottom.append(&value_toggle(ctx, &headline));
    bottom.append(&spacer());
    bottom.append(&reset_toggle(ctx, &dynamic.trailing));
    bottom
}

fn normal_row(ctx: &Ctx, window: &Window, now: Timestamp, animate: bool) -> gtk::Box {
    let view = quota_view(&ctx.locale, window, &ctx.display, now);
    let dynamic = Dynamic {
        flame: svg_image(FLAME, &ctx.css("crit"), FLAME_SIZE, &["headroom-flame"]),
        note: label("", &["headroom-reading", "dim"]),
        trailing: label("", &["headroom-reading", "dim"]),
        forecast: wrapping_label("", &["headroom-forecast"]),
    };
    dynamic.show(&view);
    let body = column(2, &["headroom-quota-row"]);
    body.append(&top_line(&view, &dynamic));
    body.append(&meter(
        view.fill,
        view.tick,
        colors(ctx, &view),
        animate,
        meter_size(false),
    ));
    body.append(&bottom_line(ctx, &view, &dynamic));
    body.append(&dynamic.forecast);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| dynamic.show(&quota_view(&locale, &window, &display, now)));
    body
}

struct CompactParts {
    flame: gtk::Image,
    trailing: gtk::Label,
    meter: gtk::DrawingArea,
}

impl CompactParts {
    fn show(&self, locale: &Locale, window: &Window, display: &Display, now: Timestamp) {
        let view = quota_view(locale, window, display, now);
        self.flame
            .set_visible(view.note.as_ref().is_some_and(|note| note.flame));
        self.trailing.set_text(&compact_reset(
            locale,
            window.resets_at,
            now,
            display.reset_format,
        ));
        let tip = meter_tooltip(locale, &view, window.resets_at, now, display);
        self.meter.set_tooltip_text(Some(&tip));
    }
}

fn compact_row(ctx: &Ctx, window: &Window, now: Timestamp, animate: bool) -> gtk::Box {
    let view = quota_view(&ctx.locale, window, &ctx.display, now);
    let parts = CompactParts {
        flame: svg_image(
            FLAME,
            &ctx.css("crit"),
            COMPACT_FLAME_SIZE,
            &["headroom-flame"],
        ),
        trailing: label("", &["headroom-reading", "dim"]),
        meter: meter(
            view.fill,
            view.tick,
            colors(ctx, &view),
            animate,
            meter_size(true),
        ),
    };
    let line = row(6, &["headroom-row-line", "headroom-compact-line"]);
    let title = label(&view.label, &["headroom-metric-label"]);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    parts.flame.set_valign(gtk::Align::Center);
    line.append(&parts.flame);
    let headline = label(&view.headline, &["headroom-reading"]);
    line.append(&value_toggle(ctx, &headline));
    line.append(&reset_toggle(ctx, &parts.trailing));
    let body = column(3, &["headroom-quota-row"]);
    body.append(&line);
    body.append(&parts.meter);
    parts.show(&ctx.locale, window, &ctx.display, now);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| parts.show(&locale, &window, &display, now));
    body
}

pub fn quota_row(ctx: &Ctx, window: &Window, now: Timestamp, animate: bool) -> gtk::Box {
    if ctx.compact() {
        compact_row(ctx, window, now, animate)
    } else {
        normal_row(ctx, window, now, animate)
    }
}
