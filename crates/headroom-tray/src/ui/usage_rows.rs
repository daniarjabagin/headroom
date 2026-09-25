use gtk::prelude::*;

use crate::dates::day_title;
use crate::i18n::Lang;
use crate::labels::{balance_title, balance_value};
use crate::numbers::{compact_tokens_text, exact_spend_line, spend_line, usd};
use crate::payload::{Account, Daily, Usage};
use crate::spend::{Period, TREND_HEIGHT, bar_height, breakdown_text, trend_days};
use crate::ui::context::Ctx;
use crate::ui::draw::{fill, rounded_top, set_color};
use crate::ui::widgets::{column, label, row};

const BAR_WIDTH: f64 = 4.0;
const BAR_GAP: f64 = 1.0;
const BAR_SLOTS: i32 = 30;
const COMPACT_TREND_HEIGHT: i32 = 14;

fn day_tooltip(lang: Lang, day: &Daily) -> String {
    let figures = if day.total_tokens == 0 {
        lang.tr("No usage").to_owned()
    } else {
        format!(
            "{} · {}",
            compact_tokens_text(lang, day.total_tokens),
            usd(day.cost_usd_micros)
        )
    };
    let partial = if day.partial {
        lang.tr(" · some models unpriced")
    } else {
        ""
    };
    format!("{}\n{figures}{partial}", day_title(lang, day.date))
}

fn strip_width() -> i32 {
    let width = f64::from(BAR_SLOTS) * BAR_WIDTH + f64::from(BAR_SLOTS - 1) * BAR_GAP;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a small constant pixel width"
    )]
    let pixels = width as i32;
    pixels
}

fn slot_tooltip(heights: &[(u64, Option<Daily>)], x: i32, lang: Lang) -> Option<String> {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive constant slot pitch"
    )]
    let pitch = (BAR_WIDTH + BAR_GAP) as usize;
    let slot = usize::try_from(x).ok()? / pitch;
    heights
        .get(slot)?
        .1
        .as_ref()
        .map(|day| day_tooltip(lang, day))
}

fn day_heights(usage: &Usage) -> Vec<(u64, Option<Daily>)> {
    let days = trend_days(&usage.daily);
    let peak = days
        .iter()
        .flatten()
        .map(|day| day.total_tokens)
        .max()
        .unwrap_or(0);
    days.iter()
        .map(|day| {
            (
                bar_height(day.map_or(0, |d| d.total_tokens), peak),
                day.cloned(),
            )
        })
        .collect()
}

fn trend_strip(ctx: &Ctx, usage: &Usage) -> gtk::DrawingArea {
    let heights = day_heights(usage);
    let area = gtk::DrawingArea::new();
    area.set_content_width(strip_width());
    let full = i32::try_from(TREND_HEIGHT).unwrap_or(18);
    area.set_content_height(if ctx.compact() {
        COMPACT_TREND_HEIGHT
    } else {
        full
    });
    area.set_valign(gtk::Align::Center);
    let color = ctx.tone_color(crate::payload::Tone::Good);
    let bars: Vec<u64> = heights.iter().map(|(height, _)| *height).collect();
    area.set_draw_func(move |_, cr, _, height| {
        set_color(cr, color);
        let scale = f64::from(height) / f64::from(full);
        for (index, bar) in bars.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "30 slots and 18 px bars")]
            let (x, bar) = (index as f64 * (BAR_WIDTH + BAR_GAP), *bar as f64 * scale);
            rounded_top(cr, x, f64::from(height) - bar, BAR_WIDTH, bar, 1.0);
            fill(cr);
        }
    });
    area.set_has_tooltip(true);
    let lang = ctx.locale.lang;
    area.connect_query_tooltip(move |_, x, _, _, tooltip| {
        let text = slot_tooltip(&heights, x, lang);
        tooltip.set_text(text.as_deref());
        text.is_some()
    });
    area
}

pub fn trend_row(ctx: &Ctx, usage: &Usage) -> gtk::Box {
    let line = row(10, &["headroom-text-row"]);
    let title = label(ctx.locale.lang.tr("Usage Trend"), &["headroom-value-label"]);
    title.set_hexpand(true);
    line.append(&title);
    line.append(&trend_strip(ctx, usage));
    line
}

fn value_row(title: &str, value: &str, tooltip: Option<&str>) -> gtk::Box {
    let line = row(10, &["headroom-text-row"]);
    let name = label(title, &["headroom-value-label"]);
    name.set_hexpand(true);
    let figure = label(value, &["headroom-value"]);
    if let Some(text) = tooltip {
        figure.add_css_class("headroom-hover-chip");
        figure.set_tooltip_text(Some(text));
    }
    line.append(&name);
    line.append(&figure);
    line
}

fn spend_rows(ctx: &Ctx, account: &Account, usage: &Usage) -> Vec<gtk::Box> {
    let lang = ctx.locale.lang;
    Period::ALL
        .iter()
        .map(|period| {
            let totals = period.totals(usage);
            let title = period.row_title(lang);
            let heading = format!("{title} · {}", account.provider_name);
            let tooltip = (totals.tokens.total > 0).then(|| {
                let figures = (totals.cost_usd_micros, totals.tokens.total);
                breakdown_text(
                    lang,
                    &heading,
                    &totals.models,
                    totals.models_other.as_ref(),
                    figures,
                )
                .unwrap_or_else(|| exact_spend_line(lang, figures.0, figures.1, totals.partial))
            });
            let value = spend_line(lang, totals.cost_usd_micros, totals.tokens.total);
            value_row(title, &value, tooltip.as_deref())
        })
        .collect()
}

pub fn extra_rows(ctx: &Ctx, account: &Account, usage: Option<&Usage>) -> Option<gtk::Box> {
    let lang = ctx.locale.lang;
    let mut rows = Vec::new();
    if let Some(usage) = usage.filter(|_| ctx.display.show_account_spend) {
        rows.extend(spend_rows(ctx, account, usage));
    }
    rows.extend(account.balances.iter().filter_map(|balance| {
        let value = balance_value(lang, balance)?;
        Some(value_row(&balance_title(lang, balance), &value, None))
    }));
    if rows.is_empty() {
        return None;
    }
    let body = column(0, &[]);
    for line in rows {
        body.append(&line);
    }
    Some(body)
}
