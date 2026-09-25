use gtk::prelude::*;
use jiff::Timestamp;

use crate::assets::FLAME;
use crate::combined::{
    CombinedGroup, CombinedRow, CombinedWindow, combined_row, group_plans, group_title,
};
use crate::dates::Locale;
use crate::payload::{Account, Display, ProviderStatus};
use crate::popup_model::compact::compact_reset;
use crate::popup_model::status::status_view;
use crate::ui::context::{Ctx, ShareTarget};
use crate::ui::header::{HeaderInput, links_of, section_header};
use crate::ui::header_menu::MenuInput;
use crate::ui::meter::{MeterColors, MeterPart, meter_size, segmented_meter};
use crate::ui::quota_row::{reset_toggle, value_toggle};
use crate::ui::status_notice::{status_mark, status_notice};
use crate::ui::widgets::{column, label, row, spacer, svg_image, wrapping_label};

const FLAME_SIZE: i32 = 11;
const COMPACT_FLAME_SIZE: i32 = 10;

pub struct GroupInput<'a> {
    pub group: &'a CombinedGroup,
    pub members: &'a [Account],
    pub status: Option<&'a ProviderStatus>,
    pub now: Timestamp,
}

fn meter_parts(ctx: &Ctx, row: &CombinedRow) -> Vec<MeterPart> {
    let (track, tick) = (ctx.color("track"), ctx.color("tick"));
    row.segments
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

fn top_line(ctx: &Ctx, row_view: &CombinedRow) -> gtk::Box {
    let top = row(8, &["headroom-row-line"]);
    let title = label(&row_view.label, &["headroom-metric-label"]);
    title.set_hexpand(true);
    top.append(&title);
    if let Some(note) = &row_view.note {
        let notes = row(4, &[]);
        if note.flame {
            notes.append(&svg_image(
                FLAME,
                &ctx.css("crit"),
                FLAME_SIZE,
                &["headroom-flame"],
            ));
        }
        notes.append(&label(&note.text, &["headroom-reading", "dim"]));
        top.append(&notes);
    }
    top
}

fn bottom_line(ctx: &Ctx, row_view: &CombinedRow) -> (gtk::Box, gtk::Label) {
    let bottom = row(8, &[]);
    let headline = label(&row_view.headline, &["headroom-reading"]);
    bottom.append(&value_toggle(ctx, &headline));
    bottom.append(&spacer());
    let trailing = label(&row_view.trailing, &["headroom-reading", "dim"]);
    bottom.append(&reset_toggle(ctx, &trailing));
    (bottom, trailing)
}

fn meter_tip(row_view: &CombinedRow) -> String {
    std::iter::once(row_view.trailing.as_str())
        .chain(row_view.forecast.as_deref())
        .chain(std::iter::once(row_view.breakdown.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

struct CompactParts {
    trailing: gtk::Label,
    meter: gtk::DrawingArea,
}

impl CompactParts {
    fn show(&self, locale: &Locale, window: &CombinedWindow, display: &Display, now: Timestamp) {
        let text = compact_reset(locale, window.resets_at, now, display.reset_format);
        self.trailing.set_text(&text);
        let row_view = combined_row(locale, window, &[], display, now);
        self.meter.set_tooltip_text(Some(&meter_tip(&row_view)));
    }
}

fn compact_window_row(
    ctx: &Ctx,
    window: &CombinedWindow,
    members: &[Account],
    now: Timestamp,
) -> gtk::Box {
    let row_view = combined_row(&ctx.locale, window, members, &ctx.display, now);
    let line = row(6, &["headroom-row-line", "headroom-compact-line"]);
    let title = label(&row_view.label, &["headroom-metric-label"]);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    if row_view.note.as_ref().is_some_and(|note| note.flame) {
        let flame = svg_image(
            FLAME,
            &ctx.css("crit"),
            COMPACT_FLAME_SIZE,
            &["headroom-flame"],
        );
        flame.set_valign(gtk::Align::Center);
        line.append(&flame);
    }
    let headline = label(&row_view.headline, &["headroom-reading"]);
    line.append(&value_toggle(ctx, &headline));
    let parts = CompactParts {
        trailing: label("", &["headroom-reading", "dim"]),
        meter: segmented_meter(meter_parts(ctx, &row_view), meter_size(true)),
    };
    line.append(&reset_toggle(ctx, &parts.trailing));
    let body = column(3, &["headroom-quota-row"]);
    body.append(&line);
    body.append(&parts.meter);
    parts.show(&ctx.locale, window, &ctx.display, now);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| parts.show(&locale, &window, &display, now));
    body
}

fn window_row(ctx: &Ctx, window: &CombinedWindow, members: &[Account], now: Timestamp) -> gtk::Box {
    if ctx.compact() {
        return compact_window_row(ctx, window, members, now);
    }
    let row_view = combined_row(&ctx.locale, window, members, &ctx.display, now);
    let body = column(2, &["headroom-quota-row"]);
    body.append(&top_line(ctx, &row_view));
    body.append(&segmented_meter(
        meter_parts(ctx, &row_view),
        meter_size(false),
    ));
    let (bottom, trailing) = bottom_line(ctx, &row_view);
    body.append(&bottom);
    let (locale, display, window) = (ctx.locale.clone(), ctx.display.clone(), window.clone());
    ctx.on_tick(move |now| {
        trailing.set_text(&combined_row(&locale, &window, &[], &display, now).trailing);
    });
    if let Some(forecast) = &row_view.forecast {
        body.append(&wrapping_label(forecast, &["headroom-forecast"]));
    }
    body.set_tooltip_text(Some(&row_view.breakdown));
    body
}

fn header(ctx: &Ctx, group: &CombinedGroup, status: Option<&ProviderStatus>) -> gtk::Box {
    let incident = status.and_then(|status| status_view(ctx.locale.lang, status));
    let starred = group
        .account_ids
        .iter()
        .any(|id| ctx.display.is_starred(id));
    let menu = MenuInput {
        provider_name: group.provider_name.clone(),
        account_ids: group.account_ids.clone(),
        starred,
        target: ShareTarget::Combined(group.provider.clone()),
        links: links_of(ctx, &group.provider),
    };
    let input = HeaderInput {
        title: group_title(ctx.locale.lang, group),
        plan: group_plans(group),
        status: incident
            .iter()
            .map(|view| status_mark(view).upcast())
            .collect(),
        menu,
    };
    section_header(ctx, &group.provider, input)
}

pub fn combined_section(ctx: &Ctx, input: &GroupInput) -> gtk::Box {
    let group = input.group;
    let section = column(if ctx.compact() { 2 } else { 4 }, &[]);
    section.append(&header(ctx, group, input.status));
    let card = column(0, &["headroom-card"]);
    let incident = input
        .status
        .and_then(|status| status_view(ctx.locale.lang, status));
    if let Some(view) = incident {
        card.append(&status_notice(ctx, &view, input.now, false));
    }
    for window in &group.windows {
        card.append(&window_row(ctx, window, input.members, input.now));
    }
    card.set_visible(card.first_child().is_some());
    section.append(&card);
    section
}
