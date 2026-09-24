use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::combined::{
    CombinedGroup, CombinedRow, CombinedWindow, combined_row, group_plans, group_title,
};
use crate::palette::Rgba;
use crate::ui::account_section::provider_icon;
use crate::ui::context::{Action, Ctx};
use crate::ui::draw::{capsule, fill, set_color};
use crate::ui::widgets::{button, column, label, row, spacer, svg_image, wrapping_label};

const METER_HEIGHT: i32 = 9;
const TRACK_HEIGHT: f64 = 5.0;
const SEGMENT_GAP: f64 = 2.0;

fn segmented_meter(ctx: &Ctx, row: &CombinedRow) -> gtk::DrawingArea {
    let track = ctx.color("track");
    let fills: Vec<(f64, Rgba)> = row
        .segments
        .iter()
        .map(|segment| (segment.fraction, ctx.tone_color(segment.tone)))
        .collect();
    let area = gtk::DrawingArea::new();
    area.set_content_height(METER_HEIGHT);
    area.set_hexpand(true);
    area.add_css_class("headroom-meter");
    area.set_draw_func(move |_, cr, width, height| {
        #[allow(clippy::cast_precision_loss, reason = "a handful of segments")]
        let count = fills.len().max(1) as f64;
        let width = f64::from(width);
        let each = ((width - SEGMENT_GAP * (count - 1.0)) / count).max(0.0);
        let top = ((f64::from(height) - TRACK_HEIGHT) / 2.0).round();
        for (index, (fraction, color)) in fills.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a handful of segments")]
            let x = index as f64 * (each + SEGMENT_GAP);
            set_color(cr, track);
            capsule(cr, x, top, each, TRACK_HEIGHT);
            fill(cr);
            if *fraction > 0.0 {
                set_color(cr, *color);
                capsule(
                    cr,
                    x,
                    top,
                    (each * fraction).round().max(TRACK_HEIGHT).min(each),
                    TRACK_HEIGHT,
                );
                fill(cr);
            }
        }
    });
    area
}

fn top_line(ctx: &Ctx, row_view: &CombinedRow) -> gtk::Box {
    let top = row(8, &["headroom-row-line"]);
    let title = label(&row_view.label, &["headroom-metric-label"]);
    title.set_hexpand(true);
    top.append(&title);
    if let Some(note) = &row_view.note {
        let notes = row(4, &[]);
        if note.flame {
            notes.append(&svg_image(FLAME, &ctx.css("crit"), 11, &["headroom-flame"]));
        }
        notes.append(&label(&note.text, &["headroom-reading", "dim"]));
        top.append(&notes);
    }
    top
}

fn bottom_line(ctx: &Ctx, row_view: &CombinedRow) -> (gtk::Box, gtk::Label) {
    let bottom = row(8, &[]);
    let headline = label(&row_view.headline, &["headroom-reading"]);
    bottom.append(&button(
        &headline,
        &["headroom-toggle", "reading"],
        ctx.action(Action::ToggleValueMode),
    ));
    bottom.append(&spacer());
    let trailing = label(&row_view.trailing, &["headroom-reading", "dim"]);
    bottom.append(&button(
        &trailing,
        &["headroom-toggle", "trailing"],
        ctx.action(Action::ToggleResetFormat),
    ));
    (bottom, trailing)
}

fn window_row(ctx: &Ctx, window: &CombinedWindow, now: Timestamp) -> gtk::Box {
    let row_view = combined_row(&ctx.locale, window, &ctx.display, now);
    let body = column(2, &["headroom-quota-row"]);
    body.append(&top_line(ctx, &row_view));
    body.append(&segmented_meter(ctx, &row_view));
    let (bottom, trailing) = bottom_line(ctx, &row_view);
    body.append(&bottom);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| {
        trailing.set_text(&combined_row(&locale, &window, &display, now).trailing);
    });
    if let Some(forecast) = &row_view.forecast {
        body.append(&wrapping_label(forecast, &["headroom-forecast"]));
    }
    body.set_tooltip_text(Some(&row_view.breakdown));
    body
}

fn header(ctx: &Ctx, group: &CombinedGroup) -> gtk::Box {
    let line = row(6, &["headroom-section-header"]);
    line.append(&provider_icon(ctx, &group.provider));
    let title = label(&group_title(ctx.locale.lang, group), &["headroom-title"]);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    if let Some(plans) = group_plans(group) {
        line.append(&label(&plans, &["headroom-plan"]));
    }
    line
}

pub fn combined_section(ctx: &Ctx, group: &CombinedGroup, now: Timestamp) -> gtk::Box {
    let section = column(4, &[]);
    section.append(&header(ctx, group));
    let card = column(0, &["headroom-card"]);
    for window in &group.windows {
        card.append(&window_row(ctx, window, now));
    }
    card.set_visible(!group.windows.is_empty());
    section.append(&card);
    section
}
