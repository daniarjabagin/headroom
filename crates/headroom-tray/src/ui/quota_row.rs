use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::payload::Window;
use crate::quota::{QuotaView, quota_view};
use crate::ui::context::{Action, Ctx};
use crate::ui::meter::{MeterColors, meter};
use crate::ui::widgets::{button, column, label, row, spacer, svg_image, wrapping_label};

const FLAME_SIZE: i32 = 11;

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

fn toggle(text: &gtk::Label, class: &str, on_click: impl Fn() + 'static) -> gtk::Button {
    let toggle = button(text, &["headroom-toggle", class], on_click);
    toggle.set_valign(gtk::Align::Center);
    toggle
}

fn bottom_line(ctx: &Ctx, view: &QuotaView, dynamic: &Dynamic) -> gtk::Box {
    let bottom = row(8, &[]);
    let headline = label(&view.headline, &["headroom-reading"]);
    bottom.append(&toggle(
        &headline,
        "reading",
        ctx.action(Action::ToggleValueMode),
    ));
    bottom.append(&spacer());
    bottom.append(&toggle(
        &dynamic.trailing,
        "trailing",
        ctx.action(Action::ToggleResetFormat),
    ));
    bottom
}

pub fn quota_row(ctx: &Ctx, window: &Window, now: Timestamp, animate: bool) -> gtk::Box {
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
    body.append(&meter(view.fill, view.tick, colors(ctx, &view), animate));
    body.append(&bottom_line(ctx, &view, &dynamic));
    body.append(&dynamic.forecast);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| dynamic.show(&quota_view(&locale, &window, &display, now)));
    body
}
