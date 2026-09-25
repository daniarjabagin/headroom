use gtk::prelude::*;

use crate::palette::Rgba;
use crate::payload::{PeriodSpend, SpendBreakdown};
use crate::popup_model::breakdown::{
    BreakdownList, BreakdownRow, RowMark, model_list, project_list,
};
use crate::popup_model::spend_view::SpendChoice;
use crate::ui::context::{Action, Ctx};
use crate::ui::draw::{fill, set_color};
use crate::ui::popover::share_bar;
use crate::ui::widgets::{column, icon, label, row, text_button};

const DOT_SIZE: i32 = 8;
const FOLDER_ICON: i32 = 12;
const VALUE_WIDTH: i32 = 46;

pub fn dot(color: Rgba) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(DOT_SIZE);
    area.set_content_height(DOT_SIZE);
    area.set_valign(gtk::Align::Center);
    area.set_draw_func(move |_, cr, width, height| {
        set_color(cr, color);
        let radius = f64::from(width.min(height)) / 2.0;
        cr.arc(radius, radius, radius, 0.0, std::f64::consts::TAU);
        fill(cr);
    });
    area
}

fn mark(ctx: &Ctx, mark: &RowMark) -> Option<gtk::Widget> {
    match mark {
        RowMark::Dot(provider) => Some(dot(ctx.series_color(provider)).upcast()),
        RowMark::Folder => Some(
            icon(
                "folder-symbolic",
                FOLDER_ICON,
                &["headroom-breakdown-folder"],
            )
            .upcast(),
        ),
        RowMark::None => None,
    }
}

fn bar(ctx: &Ctx, row_view: &BreakdownRow) -> gtk::DrawingArea {
    let neutral = ctx.color("text-tertiary");
    let parts = row_view
        .bar
        .iter()
        .map(|part| {
            let color = part
                .provider
                .as_deref()
                .map_or(neutral, |provider| ctx.series_color(provider));
            (color, part.fraction)
        })
        .collect();
    share_bar(parts, ctx.color("track"))
}

fn breakdown_row(ctx: &Ctx, row_view: &BreakdownRow) -> gtk::Box {
    let body = column(4, &["headroom-breakdown-row", "headroom-hover-chip"]);
    let line = row(6, &[]);
    if let Some(mark) = mark(ctx, &row_view.mark) {
        line.append(&mark);
    }
    let name = label(&row_view.name, &["headroom-breakdown-name"]);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&name);
    if let Some(detail) = &row_view.detail {
        let detail = label(&format!("· {detail}"), &["headroom-breakdown-share"]);
        detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
        line.append(&detail);
    }
    let share = label(&row_view.share, &["headroom-breakdown-share"]);
    share.set_hexpand(true);
    share.set_xalign(1.0);
    line.append(&share);
    let value = label(&row_view.value, &["headroom-breakdown-value"]);
    value.set_xalign(1.0);
    value.set_size_request(VALUE_WIDTH, -1);
    line.append(&value);
    body.append(&line);
    body.append(&bar(ctx, row_view));
    body.set_tooltip_text(Some(&row_view.tooltip));
    body
}

fn switch(ctx: &Ctx, current: SpendBreakdown) -> gtk::Box {
    let lang = ctx.locale.lang;
    let track = row(0, &["headroom-segmented", "headroom-mini-segmented"]);
    for (choice, title) in [
        (SpendBreakdown::Models, "Models"),
        (SpendBreakdown::Projects, "Projects"),
    ] {
        let segment = text_button(
            lang.tr(title),
            &["headroom-segment"],
            ctx.action(Action::SelectBreakdown(choice)),
        );
        if choice == current {
            segment.add_css_class("checked");
        }
        track.append(&segment);
    }
    track
}

fn list(ctx: &Ctx, choice: SpendChoice, period: &PeriodSpend) -> BreakdownList {
    let (lang, unit, basis) = (ctx.locale.lang, choice.unit, choice.basis());
    match choice.breakdown {
        SpendBreakdown::Projects => project_list(lang, unit, basis, period)
            .unwrap_or_else(|| model_list(lang, unit, basis, period)),
        SpendBreakdown::Models => model_list(lang, unit, basis, period),
    }
}

pub fn breakdown_block(ctx: &Ctx, choice: SpendChoice, period: &PeriodSpend) -> gtk::Box {
    let block = column(4, &["headroom-breakdown"]);
    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("headroom-breakdown-separator");
    block.append(&separator);
    let shown = list(ctx, choice, period);
    let head = row(8, &["headroom-breakdown-head"]);
    head.append(&switch(ctx, choice.breakdown));
    let caption = label(&shown.caption, &["headroom-breakdown-caption"]);
    caption.set_hexpand(true);
    caption.set_xalign(1.0);
    head.append(&caption);
    block.append(&head);
    let rows = column(0, &[]);
    for row_view in &shown.rows {
        rows.append(&breakdown_row(ctx, row_view));
    }
    block.append(&rows);
    block
}
