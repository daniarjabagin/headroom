use gtk::prelude::*;

use crate::numbers::{exact_spend_line, exact_tokens_text};
use crate::payload::{PeriodSpend, ProviderSpend, Spend, SpendPeriod};
use crate::popup_model::model_card::model_card;
use crate::popup_model::spend_view::{
    Figures, SpendChoice, SpendOverride, period_heading, period_title, periods, ring_center,
    shows_breakdown, slice_values, unit_value,
};
use crate::spend::{info_text, shows_token_line};
use crate::ui::breakdown::{breakdown_block, dot};
use crate::ui::context::{Action, Ctx};
use crate::ui::donut::donut;
use crate::ui::model_popover::attach_model_popover;
use crate::ui::unit_menu::unit_button;
use crate::ui::widgets::{column, icon, label, row, spacer, text_button};

fn adopt(widget: &gtk::Widget) {
    if widget.parent().is_some() {
        widget.unparent();
    }
}

fn header(ctx: &Ctx, choice: SpendChoice, period: &PeriodSpend, refresh: &gtk::Widget) -> gtk::Box {
    let line = row(6, &["headroom-section-header", "headroom-spend-header"]);
    line.append(&unit_button(ctx, choice.unit));
    let info = icon("help-about-symbolic", 11, &["headroom-info-icon"]);
    info.set_tooltip_text(Some(&info_text(ctx.locale.lang, period)));
    info.set_valign(gtk::Align::Center);
    line.append(&info);
    line.append(&spacer());
    adopt(refresh);
    line.append(refresh);
    line
}

fn segmented(ctx: &Ctx, spend: &Spend, current: SpendPeriod) -> gtk::Box {
    let track = row(2, &["headroom-segmented"]);
    let shown = periods(spend);
    if shown.len() > 3 {
        track.add_css_class("tight");
    }
    for period in shown {
        let segment = text_button(
            period_title(ctx.locale.lang, period),
            &["headroom-segment"],
            ctx.action(Action::SelectPeriod(period)),
        );
        segment.set_hexpand(true);
        if period == current {
            segment.add_css_class("checked");
        }
        track.append(&segment);
    }
    track
}

fn legend_entry(
    ctx: &Ctx,
    choice: SpendChoice,
    spend: &ProviderSpend,
    with_tokens: bool,
) -> gtk::Box {
    let lang = ctx.locale.lang;
    let entry = column(1, &["headroom-legend-entry", "headroom-hover-chip"]);
    let line = row(6, &[]);
    line.append(&dot(ctx.series_color(&spend.provider)));
    let name = label(&spend.provider_name, &["headroom-legend-name"]);
    name.set_hexpand(true);
    line.append(&name);
    let value = unit_value(lang, choice.unit, Figures::from(spend));
    line.append(&label(&value, &["headroom-legend-value"]));
    entry.append(&line);
    if with_tokens {
        let tokens = exact_tokens_text(lang, spend.total_tokens);
        entry.append(&label(&tokens, &["headroom-legend-tokens"]));
    }
    let heading = period_heading(lang, choice.period);
    match model_card(lang, choice.unit, choice.basis(), heading, spend) {
        Some(card) => attach_model_popover(ctx, &entry, &spend.provider, card),
        None => entry.set_tooltip_text(Some(&exact_spend_line(
            lang,
            spend.cost_usd_micros,
            spend.total_tokens,
            spend.partial,
        ))),
    }
    entry
}

fn ring_body(ctx: &Ctx, choice: SpendChoice, period: &PeriodSpend, animate: bool) -> gtk::Box {
    let body = row(8, &[]);
    let colors = period
        .by_provider
        .iter()
        .map(|spend| ctx.series_color(&spend.provider))
        .collect();
    body.append(&donut(
        &slice_values(choice, period),
        colors,
        &ring_center(ctx.locale.lang, choice.unit, period),
        animate,
        ctx.compact(),
    ));
    let legend = column(1, &[]);
    legend.set_hexpand(true);
    legend.set_valign(gtk::Align::Center);
    let with_tokens = shows_token_line(period);
    for spend in &period.by_provider {
        legend.append(&legend_entry(ctx, choice, spend, with_tokens));
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
    let choice = ctx.spend.unwrap_or_else(|| {
        SpendChoice::resolve(&ctx.display, SpendOverride::default(), spend, ctx.recent)
    });
    let period = choice.period_of(spend);
    let compact = ctx.compact();
    let section = column(if compact { 2 } else { 4 }, &[]);
    section.append(&header(ctx, choice, period, refresh));
    let card = column(
        if compact { 8 } else { 12 },
        &["headroom-card", "headroom-spend-card"],
    );
    card.append(&segmented(ctx, spend, choice.period));
    if period.by_provider.is_empty() {
        card.append(&empty_body(ctx));
    } else {
        card.append(&ring_body(ctx, choice, period, animate));
        if shows_breakdown(&ctx.display, spend) {
            card.append(&breakdown_block(ctx, choice, period));
        }
    }
    section.append(&card);
    section
}
