use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::combined::{CombinedRow, CombinedWindow, combined_row};
use crate::dates::Locale;
use crate::payload::{Account, Display};
use crate::popup_model::compact::compact_reset;
use crate::ui::context::Ctx;
use crate::ui::keyed::SharedTick;
use crate::ui::meter::{MeterColors, MeterPart, SegmentedMeter, meter_size};
use crate::ui::quota_row::{reset_toggle, value_toggle};
use crate::ui::widgets::{column, label, row, spacer, wrapping_label};

const FLAME_SIZE: i32 = 11;
const COMPACT_FLAME_SIZE: i32 = 10;

#[derive(Clone, PartialEq)]
struct Input {
    window: CombinedWindow,
    members: Vec<Account>,
}

struct Look {
    locale: Locale,
    display: Display,
    compact: bool,
}

struct Parts {
    body: gtk::Box,
    title: gtk::Label,
    flame: gtk::Image,
    note: Option<gtk::Label>,
    headline: gtk::Label,
    trailing: gtk::Label,
    forecast: Option<gtk::Label>,
    meter: SegmentedMeter,
}

impl Parts {
    fn show(&self, look: &Look, input: &Input, now: Timestamp) {
        let view = combined_row(
            &look.locale,
            &input.window,
            &input.members,
            &look.display,
            now,
        );
        self.title.set_text(&view.label);
        self.headline.set_text(&view.headline);
        let note = view.note.as_ref();
        self.flame.set_visible(note.is_some_and(|note| note.flame));
        if let Some(label) = &self.note {
            label.set_visible(note.is_some());
            label.set_text(note.map_or("", |note| note.text.as_str()));
        }
        if let Some(label) = &self.forecast {
            label.set_visible(view.forecast.is_some());
            label.set_text(view.forecast.as_deref().unwrap_or(""));
        }
        if look.compact {
            let window = &input.window;
            let reset = compact_reset(
                &look.locale,
                window.resets_at,
                now,
                look.display.reset_format,
            );
            self.trailing.set_text(&reset);
            self.meter.area.set_tooltip_text(Some(&meter_tip(&view)));
        } else {
            self.trailing.set_text(&view.trailing);
            self.body.set_tooltip_text(Some(&view.breakdown));
        }
    }
}

fn meter_tip(view: &CombinedRow) -> String {
    std::iter::once(view.trailing.as_str())
        .chain(view.forecast.as_deref())
        .chain(std::iter::once(view.breakdown.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn meter_parts(ctx: &Ctx, view: &CombinedRow) -> Vec<MeterPart> {
    let (track, tick) = (ctx.color("track"), ctx.color("tick"));
    view.segments
        .iter()
        .map(|segment| MeterPart {
            fraction: segment.fraction,
            tick: segment.tick,
            colors: MeterColors {
                track,
                fill: ctx.tone_color(segment.tone),
                tick,
            },
        })
        .collect()
}

fn texts(ctx: &Ctx, flame_size: i32) -> (gtk::Label, gtk::Image, gtk::Label, gtk::Label) {
    let title = label("", &["headroom-metric-label"]);
    title.set_hexpand(true);
    let flame = ctx.svg_image(FLAME, &ctx.css("crit"), flame_size, &["headroom-flame"]);
    let headline = label("", &["headroom-reading"]);
    let trailing = label("", &["headroom-reading", "dim"]);
    (title, flame, headline, trailing)
}

fn normal_parts(ctx: &Ctx, meter: SegmentedMeter) -> Parts {
    let (title, flame, headline, trailing) = texts(ctx, FLAME_SIZE);
    let (note, forecast) = (
        label("", &["headroom-reading", "dim"]),
        wrapping_label("", &["headroom-forecast"]),
    );
    let top = row(8, &["headroom-row-line"]);
    let notes = row(4, &[]);
    notes.append(&flame);
    notes.append(&note);
    top.append(&title);
    top.append(&notes);
    let bottom = row(8, &[]);
    bottom.append(&value_toggle(ctx, &headline));
    bottom.append(&spacer());
    bottom.append(&reset_toggle(ctx, &trailing));
    let body = column(2, &["headroom-quota-row"]);
    body.append(&top);
    body.append(&meter.area);
    body.append(&bottom);
    body.append(&forecast);
    Parts {
        body,
        title,
        flame,
        note: Some(note),
        headline,
        trailing,
        forecast: Some(forecast),
        meter,
    }
}

fn compact_parts(ctx: &Ctx, meter: SegmentedMeter) -> Parts {
    let (title, flame, headline, trailing) = texts(ctx, COMPACT_FLAME_SIZE);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    flame.set_valign(gtk::Align::Center);
    let line = row(6, &["headroom-row-line", "headroom-compact-line"]);
    line.append(&title);
    line.append(&flame);
    line.append(&value_toggle(ctx, &headline));
    line.append(&reset_toggle(ctx, &trailing));
    let body = column(3, &["headroom-quota-row"]);
    body.append(&line);
    body.append(&meter.area);
    Parts {
        body,
        title,
        flame,
        note: None,
        headline,
        trailing,
        forecast: None,
        meter,
    }
}

pub struct MountedWindowRow {
    pub widget: gtk::Box,
    pub tick: SharedTick,
    input: Rc<RefCell<Input>>,
    parts: Rc<Parts>,
}

impl MountedWindowRow {
    pub fn new(ctx: &Ctx, window: &CombinedWindow, members: &[Account], now: Timestamp) -> Self {
        let input = Input {
            window: window.clone(),
            members: members.to_vec(),
        };
        let view = combined_row(&ctx.locale, window, members, &ctx.display, now);
        let meter = SegmentedMeter::new(
            meter_parts(ctx, &view),
            meter_size(ctx.compact()),
            &ctx.sheen,
        );
        let parts = Rc::new(if ctx.compact() {
            compact_parts(ctx, meter)
        } else {
            normal_parts(ctx, meter)
        });
        let look = Look {
            locale: ctx.locale.clone(),
            display: ctx.display.clone(),
            compact: ctx.compact(),
        };
        let input = Rc::new(RefCell::new(input));
        let (shown, drawn) = (Rc::clone(&input), Rc::clone(&parts));
        let tick: SharedTick = Rc::new(move |now| drawn.show(&look, &shown.borrow(), now));
        tick(now);
        Self {
            widget: parts.body.clone(),
            tick,
            input,
            parts,
        }
    }

    pub fn update(&self, ctx: &Ctx, window: &CombinedWindow, members: &[Account], now: Timestamp) {
        let next = Input {
            window: window.clone(),
            members: members.to_vec(),
        };
        if *self.input.borrow() == next {
            return;
        }
        let view = combined_row(&ctx.locale, window, members, &ctx.display, now);
        self.parts.meter.set(meter_parts(ctx, &view));
        *self.input.borrow_mut() = next;
        (self.tick)(now);
    }
}
