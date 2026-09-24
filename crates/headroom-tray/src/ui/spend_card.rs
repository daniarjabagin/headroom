use gtk::prelude::*;

use crate::numbers::{exact_spend_line, exact_tokens_text, ring_usd, usd};
use crate::payload::{PeriodSpend, ProviderSpend, Spend};
use crate::spend::{Period, breakdown_text, info_text, shows_token_line};
use crate::ui::context::{Action, Ctx};
use crate::ui::donut::donut;
use crate::ui::widgets::{column, icon, label, row, spacer, text_button};

const DOT_SIZE: i32 = 8;

fn header(ctx: &Ctx, period: &PeriodSpend, refresh: &gtk::Widget) -> gtk::Box {
    let line = row(6, &["headroom-section-header"]);
    line.append(&label(
        ctx.locale.lang.tr("Total Spend"),
        &["headroom-title"],
    ));
    let info = icon("help-about-symbolic", 11, &["headroom-info-icon"]);
    info.set_tooltip_text(Some(&info_text(ctx.locale.lang, period)));
    line.append(&info);
    line.append(&spacer());
    line.append(refresh);
    line
}

fn segmented(ctx: &Ctx) -> gtk::Box {
    let track = row(2, &["headroom-segmented"]);
    for period in Period::ALL {
        let segment = text_button(
            period.segment_title(ctx.locale.lang),
            &["headroom-segment"],
            ctx.action(Action::SelectPeriod(period)),
        );
        segment.set_hexpand(true);
        if period == ctx.ui.period {
            segment.add_css_class("checked");
        }
        track.append(&segment);
    }
    track
}

fn dot(ctx: &Ctx, provider: &str) -> gtk::DrawingArea {
    let color = ctx.series_color(provider);
    let area = gtk::DrawingArea::new();
    area.set_content_width(DOT_SIZE);
    area.set_content_height(DOT_SIZE);
    area.set_valign(gtk::Align::Center);
    area.set_draw_func(move |_, cr, width, height| {
        crate::ui::draw::set_color(cr, color);
        let radius = f64::from(width.min(height)) / 2.0;
        cr.arc(radius, radius, radius, 0.0, std::f64::consts::TAU);
        crate::ui::draw::fill(cr);
    });
    area
}

fn legend_entry(ctx: &Ctx, spend: &ProviderSpend, with_tokens: bool) -> gtk::Box {
    let lang = ctx.locale.lang;
    let entry = column(1, &["headroom-legend-entry", "headroom-hover-chip"]);
    let line = row(6, &[]);
    line.append(&dot(ctx, &spend.provider));
    let name = label(&spend.provider_name, &["headroom-legend-name"]);
    name.set_hexpand(true);
    line.append(&name);
    line.append(&label(
        &usd(spend.cost_usd_micros),
        &["headroom-legend-value"],
    ));
    entry.append(&line);
    if with_tokens {
        let tokens = exact_tokens_text(lang, spend.total_tokens);
        entry.append(&label(&tokens, &["headroom-legend-tokens"]));
    }
    let title = format!(
        "{} · {}",
        ctx.ui.period.segment_title(lang),
        spend.provider_name
    );
    let totals = (spend.cost_usd_micros, spend.total_tokens);
    let tip = breakdown_text(
        lang,
        &title,
        &spend.models,
        spend.models_other.as_ref(),
        totals,
    )
    .unwrap_or_else(|| exact_spend_line(lang, totals.0, totals.1, spend.partial));
    entry.set_tooltip_text(Some(&tip));
    entry
}

#[allow(
    clippy::cast_precision_loss,
    reason = "slice proportions only, never shown as money"
)]
fn slice_value(spend: &ProviderSpend) -> f64 {
    spend.cost_usd_micros.max(0) as f64
}

fn ring_body(ctx: &Ctx, period: &PeriodSpend, animate: bool) -> gtk::Box {
    let body = row(8, &[]);
    let values: Vec<f64> = period.by_provider.iter().map(slice_value).collect();
    let colors = period
        .by_provider
        .iter()
        .map(|spend| ctx.series_color(&spend.provider))
        .collect();
    body.append(&donut(
        &values,
        colors,
        &ring_usd(period.cost_usd_micros),
        animate,
    ));
    let legend = column(1, &[]);
    legend.set_hexpand(true);
    legend.set_valign(gtk::Align::Center);
    let with_tokens = shows_token_line(period);
    for spend in &period.by_provider {
        legend.append(&legend_entry(ctx, spend, with_tokens));
    }
    body.append(&legend);
    body
}

fn empty_body(ctx: &Ctx) -> gtk::Label {
    let empty = label(
        ctx.locale.lang.tr("No usage in this period"),
        &["headroom-empty-period"],
    );
    empty.set_xalign(0.5);
    empty
}

pub fn spend_section(ctx: &Ctx, spend: &Spend, refresh: &gtk::Widget, animate: bool) -> gtk::Box {
    let period = ctx.ui.period.of(spend);
    let section = column(4, &[]);
    section.append(&header(ctx, period, refresh));
    let card = column(12, &["headroom-card", "headroom-spend-card"]);
    card.append(&segmented(ctx));
    if period.by_provider.is_empty() {
        card.append(&empty_body(ctx));
    } else {
        card.append(&ring_body(ctx, period, animate));
    }
    section.append(&card);
    section
}
