use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::dates::Locale;
use crate::payload::{Display, Window};
use crate::popup_model::compact::{compact_reset, meter_tooltip, reset_hint, value_hint};
use crate::quota::{QuotaView, quota_view};
use crate::ui::context::{Action, Ctx};
use crate::ui::keyed::SharedTick;
use crate::ui::meter::{Meter, MeterColors, MeterPart, meter_size};
use crate::ui::widgets::{button, column, label, row, spacer, wrapping_label};

const FLAME_SIZE: i32 = 11;
const COMPACT_FLAME_SIZE: i32 = 10;

struct Texts {
    title: gtk::Label,
    flame: gtk::Image,
    headline: gtk::Label,
    trailing: gtk::Label,
}

struct Normal {
    texts: Texts,
    note: gtk::Label,
    forecast: gtk::Label,
}

struct Compact {
    texts: Texts,
    meter: gtk::DrawingArea,
}

enum Parts {
    Normal(Normal),
    Compact(Compact),
}

struct Look {
    locale: Locale,
    display: Display,
}

impl Parts {
    fn show(&self, look: &Look, window: &Window, now: Timestamp) {
        let view = quota_view(&look.locale, window, &look.display, now);
        let texts = match self {
            Self::Normal(normal) => &normal.texts,
            Self::Compact(compact) => &compact.texts,
        };
        texts.title.set_text(&view.label);
        texts.headline.set_text(&view.headline);
        texts
            .flame
            .set_visible(view.note.as_ref().is_some_and(|note| note.flame));
        match self {
            Self::Normal(normal) => normal.show(&view),
            Self::Compact(compact) => compact.show(look, &view, window, now),
        }
    }
}

impl Normal {
    fn show(&self, view: &QuotaView) {
        let note = view.note.as_ref();
        self.note.set_visible(note.is_some());
        self.note
            .set_text(note.map_or("", |note| note.text.as_str()));
        self.texts.trailing.set_text(&view.trailing);
        self.forecast.set_visible(view.forecast.is_some());
        self.forecast
            .set_text(view.forecast.as_deref().unwrap_or(""));
    }
}

impl Compact {
    fn show(&self, look: &Look, view: &QuotaView, window: &Window, now: Timestamp) {
        let (locale, display) = (&look.locale, &look.display);
        self.texts.trailing.set_text(&compact_reset(
            locale,
            window.resets_at,
            now,
            display.reset_format,
        ));
        let tip = meter_tooltip(locale, view, window.resets_at, now, display);
        self.meter.set_tooltip_text(Some(&tip));
    }
}

fn meter_part(ctx: &Ctx, view: &QuotaView) -> MeterPart {
    MeterPart {
        fraction: view.fill,
        tick: view.tick,
        colors: MeterColors {
            track: ctx.color("track"),
            fill: ctx.tone_color(view.tone),
            tick: ctx.color("tick"),
        },
    }
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

fn texts(ctx: &Ctx, flame_size: i32) -> Texts {
    let title = label("", &["headroom-metric-label"]);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    Texts {
        title,
        flame: ctx.svg_image(FLAME, &ctx.css("crit"), flame_size, &["headroom-flame"]),
        headline: label("", &["headroom-reading"]),
        trailing: label("", &["headroom-reading", "dim"]),
    }
}

fn normal_row(ctx: &Ctx, meter: &Meter) -> (gtk::Box, Parts) {
    let texts = texts(ctx, FLAME_SIZE);
    let note = label("", &["headroom-reading", "dim"]);
    let forecast = wrapping_label("", &["headroom-forecast"]);
    let top = row(8, &["headroom-row-line"]);
    let notes = row(4, &[]);
    notes.append(&texts.flame);
    notes.append(&note);
    top.append(&texts.title);
    top.append(&notes);
    let bottom = row(8, &[]);
    bottom.append(&value_toggle(ctx, &texts.headline));
    bottom.append(&spacer());
    bottom.append(&reset_toggle(ctx, &texts.trailing));
    let body = column(2, &["headroom-quota-row"]);
    body.append(&top);
    body.append(&meter.area);
    body.append(&bottom);
    body.append(&forecast);
    let parts = Normal {
        texts,
        note,
        forecast,
    };
    (body, Parts::Normal(parts))
}

fn compact_row(ctx: &Ctx, meter: &Meter) -> (gtk::Box, Parts) {
    let texts = texts(ctx, COMPACT_FLAME_SIZE);
    let line = row(6, &["headroom-row-line", "headroom-compact-line"]);
    line.append(&texts.title);
    texts.flame.set_valign(gtk::Align::Center);
    line.append(&texts.flame);
    line.append(&value_toggle(ctx, &texts.headline));
    line.append(&reset_toggle(ctx, &texts.trailing));
    let body = column(3, &["headroom-quota-row"]);
    body.append(&line);
    body.append(&meter.area);
    let parts = Compact {
        texts,
        meter: meter.area.clone(),
    };
    (body, Parts::Compact(parts))
}

pub struct MountedQuota {
    pub widget: gtk::Box,
    pub tick: SharedTick,
    window: Rc<RefCell<Window>>,
    meter: Meter,
}

impl MountedQuota {
    pub fn new(ctx: &Ctx, window: &Window, now: Timestamp, animate: bool) -> Self {
        let view = quota_view(&ctx.locale, window, &ctx.display, now);
        let meter = Meter::new(
            meter_part(ctx, &view),
            animate,
            meter_size(ctx.compact()),
            &ctx.sheen,
        );
        let (widget, parts) = if ctx.compact() {
            compact_row(ctx, &meter)
        } else {
            normal_row(ctx, &meter)
        };
        let look = Look {
            locale: ctx.locale.clone(),
            display: ctx.display.clone(),
        };
        let window = Rc::new(RefCell::new(window.clone()));
        let shown = Rc::clone(&window);
        let tick: SharedTick = Rc::new(move |now| parts.show(&look, &shown.borrow(), now));
        tick(now);
        Self {
            widget,
            tick,
            window,
            meter,
        }
    }

    pub fn update(&self, ctx: &Ctx, window: &Window, now: Timestamp) {
        if *self.window.borrow() == *window {
            return;
        }
        window.clone_into(&mut self.window.borrow_mut());
        let view = quota_view(&ctx.locale, window, &ctx.display, now);
        self.meter.set(meter_part(ctx, &view), ctx.motion);
        (self.tick)(now);
    }
}
