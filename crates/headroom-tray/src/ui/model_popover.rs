use gtk::prelude::*;

use crate::i18n::Lang;
use crate::palette::Rgba;
use crate::popup_model::model_card::{ModelCard, ModelLine};
use crate::ui::context::Ctx;
use crate::ui::motion::{FAST_MS, fade};
use crate::ui::popover::{beside_window, on_dwell, share_bar, transient};
use crate::ui::widgets::{column, label, row, wrapping_label};

const CARD_WIDTH: i32 = 244;
const SHARE_COLUMN: i32 = 118;
const WINDOW_GAP: i32 = 10;

#[derive(Debug, Clone, Copy)]
pub struct CardLook {
    pub lang: Lang,
    pub series: Rgba,
    pub track: Rgba,
}

fn separator() -> gtk::Separator {
    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
    line.add_css_class("headroom-breakdown-separator");
    line
}

fn head(card: &ModelCard) -> gtk::Box {
    let line = row(8, &[]);
    let title = label(&card.title, &["headroom-popover-strong"]);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    line.append(&title);
    line.append(&label(&card.total, &["headroom-popover-strong"]));
    line
}

fn model_line(look: CardLook, line: &ModelLine) -> gtk::Box {
    let body = column(3, &["headroom-popover-line"]);
    let top = row(4, &[]);
    let name = label(&line.name, &["headroom-popover-text"]);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    top.append(&name);
    if let Some(detail) = &line.detail {
        top.append(&label(&format!("· {detail}"), &["headroom-popover-dim"]));
    }
    let value = label(&line.value, &["headroom-popover-strong"]);
    value.set_hexpand(true);
    value.set_xalign(1.0);
    top.append(&value);
    let bottom = row(8, &[]);
    bottom.append(&share_bar(vec![(look.series, line.fraction)], look.track));
    let share = label(
        &format!("{} · {}", line.share, line.tokens),
        &["headroom-popover-dim"],
    );
    share.set_xalign(1.0);
    share.set_size_request(SHARE_COLUMN, -1);
    bottom.append(&share);
    body.append(&top);
    body.append(&bottom);
    body
}

fn footnote(lang: Lang, card: &ModelCard) -> gtk::Label {
    let estimated = lang.tr("Estimated from local logs and public pricing.");
    let text = if card.folded {
        format!(
            "{}\n{estimated}",
            lang.tr("Models after the top 5 are folded into Other.")
        )
    } else {
        estimated.to_owned()
    };
    wrapping_label(&text, &["headroom-popover-note"])
}

pub fn model_card_view(look: CardLook, card: &ModelCard) -> gtk::Box {
    let body = column(7, &["headroom-model-card"]);
    body.set_size_request(CARD_WIDTH, -1);
    body.append(&head(card));
    body.append(&separator());
    let lines = column(7, &[]);
    for line in &card.lines {
        lines.append(&model_line(look, line));
    }
    body.append(&lines);
    body.append(&separator());
    body.append(&footnote(look.lang, card));
    body
}

pub fn attach_model_popover(ctx: &Ctx, entry: &gtk::Box, provider: &str, card: ModelCard) {
    let look = CardLook {
        lang: ctx.locale.lang,
        series: ctx.series_color(provider),
        track: ctx.color("track"),
    };
    let (anchor, motion) = (entry.downgrade(), ctx.motion);
    on_dwell(entry, move || {
        let anchor = anchor.upgrade()?;
        let view = model_card_view(look, &card);
        let popover = transient(&anchor, &view);
        popover.set_autohide(false);
        popover.set_offset(-WINDOW_GAP, 0);
        beside_window(&popover, anchor.upcast_ref());
        popover.popup();
        fade(&view, 0.0, 1.0, FAST_MS, motion);
        Some(popover)
    });
}
