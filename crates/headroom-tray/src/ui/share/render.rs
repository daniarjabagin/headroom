use gtk::prelude::*;
use gtk::{gdk, gsk, pango};

use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::combined::SegmentFill;
use crate::i18n::Lang;
use crate::palette::{Palette, Scheme};
use crate::popup_model::share::{ShareCard, ShareRow};
use crate::ui::share::paint::{Font, Painter, SCALE, rect};
use crate::ui::widgets::svg_texture_at;

const WIDTH: f64 = 600.0;
const HEIGHT: f64 = 315.0;
const PAD_TOP: f64 = 22.0;
const PAD_X: f64 = 28.0;
const PAD_BOTTOM: f64 = 18.0;
const LOGO_HEIGHT: f64 = 34.0;
const LOGO_RATIO: f64 = 636.0 / 240.0;
const CARD_WIDTH: f64 = 262.0;
const CARD_PAD_X: f64 = 14.0;
const CARD_PAD_Y: f64 = 14.0;
const METER_HEIGHT: f64 = 5.0;
const TICK_HEIGHT: f64 = 9.0;
const BAR_WIDTH: f64 = 250.0;
const BAR_HEIGHT: f64 = 7.0;
const LOGO_LIGHT: &str = include_str!("../../../../../assets/brand/headroom-logo-on-light.svg");
const LOGO_DARK: &str = include_str!("../../../../../assets/brand/headroom-logo-on-dark.svg");
const LOGO_SIZE: &str = "width=\"636\" height=\"240\"";

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("the share card could not be drawn")]
    Empty,
    #[error("the renderer is unavailable: {0}")]
    Renderer(#[from] gtk::glib::Error),
}

fn texture(svg: &str) -> Option<gdk::Texture> {
    let bytes = gtk::glib::Bytes::from_owned(svg.to_owned().into_bytes());
    gdk::Texture::from_bytes(&bytes).ok()
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the logo is a small positive pixel size"
)]
fn logo(scheme: Scheme) -> Option<gdk::Texture> {
    let source = match scheme {
        Scheme::Light => LOGO_LIGHT,
        Scheme::Dark => LOGO_DARK,
    };
    let height = (LOGO_HEIGHT * f64::from(SCALE)).round() as u32;
    let width = (f64::from(height) * LOGO_RATIO).round() as u32;
    texture(&source.replacen(
        LOGO_SIZE,
        &format!("width=\"{width}\" height=\"{height}\""),
        1,
    ))
}

fn top_bar(painter: &Painter, card: &ShareCard) -> f64 {
    if let Some(logo) = logo(painter.palette.scheme()) {
        painter.texture(
            &logo,
            rect(PAD_X, PAD_TOP, LOGO_HEIGHT * LOGO_RATIO, LOGO_HEIGHT),
        );
    }
    let stamp = painter.layout(
        &card.stamp,
        Font::new(10.0, pango::Weight::Normal).spaced(0.8),
    );
    let (width, height) = Painter::size(&stamp);
    let y = PAD_TOP + (LOGO_HEIGHT - height) / 2.0;
    painter.text(&stamp, WIDTH - PAD_X - width, y, "share-dim");
    let line = PAD_TOP + LOGO_HEIGHT + 12.0;
    painter.fill(
        rect(PAD_X, line, WIDTH - 2.0 * PAD_X, 1.0),
        0.0,
        "share-line",
    );
    line + 1.0
}

fn footer(painter: &Painter, lang: Lang) -> f64 {
    let font = Font::new(10.0, pango::Weight::Normal);
    let motto = painter.layout(lang.tr("Know what's left."), font);
    let (_, height) = Painter::size(&motto);
    let y = HEIGHT - PAD_BOTTOM - height;
    painter.skewed_plate(PAD_X, y + (height - 8.0) / 2.0, 8.0, "share-accent");
    painter.text(&motto, PAD_X + 16.0, y, "share-dim");
    let brand = painter.layout("headroom", font);
    let (width, _) = Painter::size(&brand);
    painter.text(&brand, WIDTH - PAD_X - width, y, "share-dim");
    let line = y - 10.0 - 1.0;
    painter.fill(
        rect(PAD_X, line, WIDTH - 2.0 * PAD_X, 1.0),
        0.0,
        "share-line",
    );
    line
}

fn provider_line(painter: &Painter, card: &ShareCard, top: f64) -> f64 {
    let (svg, tint) = provider_logo(&card.provider);
    let color = match tint {
        Tint::Brand => CLAUDE_COLOR.to_owned(),
        Tint::Text => painter
            .palette
            .css_value("share-fg")
            .unwrap_or("#151617")
            .to_owned(),
    };
    let name = painter.layout(
        &card.provider_name,
        Font::new(13.0, pango::Weight::Semibold),
    );
    let (name_width, height) = Painter::size(&name);
    if let Some(glyph) = svg_texture_at(svg, &color, 32) {
        painter.texture(&glyph, rect(PAD_X, top + (height - 16.0) / 2.0, 16.0, 16.0));
    }
    painter.text(&name, PAD_X + 23.0, top, "share-fg");
    if let Some(meta) = &card.meta {
        let meta = painter.layout(&format!("· {meta}"), Font::new(13.0, pango::Weight::Normal));
        painter.text(&meta, PAD_X + 23.0 + name_width + 6.0, top, "share-dim");
    }
    top + height
}

fn hero(painter: &Painter, card: &ShareCard, top: f64) -> f64 {
    let big = painter.layout(
        &card.hero,
        Font::new(58.0, pango::Weight::Bold).spaced(-1.7),
    );
    let (width, height) = Painter::size(&big);
    painter.text(&big, PAD_X - 2.0, top, "share-fg");
    let unit = painter.layout(&card.hero_unit, Font::new(22.0, pango::Weight::Semibold));
    let (_, unit_height) = Painter::size(&unit);
    let baseline = f64::from(big.baseline()) / f64::from(pango::SCALE);
    let unit_baseline = f64::from(unit.baseline()) / f64::from(pango::SCALE);
    painter.text(
        &unit,
        PAD_X + width + 4.0,
        top + baseline - unit_baseline,
        "share-fg",
    );
    top + height.max(unit_height)
}

fn bars(painter: &Painter, fractions: &[f64], top: f64) {
    let count = fractions.len().max(1);
    #[allow(clippy::cast_precision_loss, reason = "a handful of account bars")]
    let slot = (BAR_WIDTH - 3.0 * (count as f64 - 1.0)) / count as f64;
    let mut x = PAD_X;
    for fraction in fractions {
        painter.fill(
            rect(x, top, slot, BAR_HEIGHT),
            BAR_HEIGHT / 2.0,
            "share-line",
        );
        let filled =
            (slot * fraction.clamp(0.0, 1.0)).max(if *fraction > 0.0 { BAR_HEIGHT } else { 0.0 });
        if filled > 0.0 {
            painter.fill(
                rect(x, top, filled, BAR_HEIGHT),
                BAR_HEIGHT / 2.0,
                "share-fg",
            );
        }
        x += slot + 3.0;
    }
}

fn left_column(painter: &Painter, card: &ShareCard, top: f64, bottom: f64) {
    let block = 18.0 + 6.0 + 70.0 + 4.0 + 16.0 + 18.0 + BAR_HEIGHT;
    let mut y = top + ((bottom - top) - block).max(0.0) / 2.0;
    y = provider_line(painter, card, y) + 6.0;
    y = hero(painter, card, y) + 4.0;
    let sub = painter.layout(&card.sub, Font::new(12.0, pango::Weight::Normal));
    painter.text(&sub, PAD_X, y, "share-dim");
    y += Painter::size(&sub).1 + 18.0;
    bars(painter, &card.bars, y);
}

fn meter(painter: &Painter, segments: &[SegmentFill], x: f64, top: f64, width: f64) {
    let count = segments.len().max(1);
    #[allow(clippy::cast_precision_loss, reason = "a handful of account segments")]
    let slot = (width - 2.0 * (count as f64 - 1.0)) / count as f64;
    let track_top = top + (TICK_HEIGHT - METER_HEIGHT) / 2.0;
    let mut left = x;
    for segment in segments {
        painter.fill(rect(left, track_top, slot, METER_HEIGHT), 2.5, "track");
        let filled = (slot * segment.fraction).max(if segment.fraction > 0.0 {
            METER_HEIGHT
        } else {
            0.0
        });
        if filled > 0.0 {
            let tone = crate::palette::tone_token(segment.tone);
            painter.fill(
                rect(left, track_top, filled.min(slot), METER_HEIGHT),
                2.5,
                tone,
            );
        }
        if let Some(tick) = segment.tick {
            let tick_x = (left + slot * tick - 1.0).clamp(left, left + slot - 2.0);
            painter.fill(rect(tick_x, top, 2.0, TICK_HEIGHT), 1.0, "tick");
        }
        left += slot + 2.0;
    }
}

fn card_row(painter: &Painter, row: &ShareRow, x: f64, top: f64) -> f64 {
    let inner = CARD_WIDTH - 2.0 * CARD_PAD_X;
    let label = painter.layout(&row.label, Font::new(13.0, pango::Weight::Semibold));
    painter.text(&label, x, top, "text");
    let mut y = top + Painter::size(&label).1 + 3.0;
    meter(painter, &row.segments, x, y, inner);
    y += TICK_HEIGHT + 3.0;
    let reading = painter.layout(&row.headline, Font::new(12.0, pango::Weight::Normal));
    painter.text(&reading, x, y, "text");
    let trailing = painter.layout(&row.trailing, Font::new(12.0, pango::Weight::Normal));
    let (width, height) = Painter::size(&trailing);
    painter.text(&trailing, x + inner - width, y, "text-secondary");
    y + height
}

fn row_height(painter: &Painter) -> f64 {
    let label = painter.layout("Session", Font::new(13.0, pango::Weight::Semibold));
    let reading = painter.layout("0", Font::new(12.0, pango::Weight::Normal));
    Painter::size(&label).1 + 3.0 + TICK_HEIGHT + 3.0 + Painter::size(&reading).1
}

fn right_card(painter: &Painter, card: &ShareCard, top: f64, bottom: f64) {
    let rows = &card.rows[..card.rows.len().min(3)];
    #[allow(clippy::cast_precision_loss, reason = "at most three rows")]
    let count = rows.len() as f64;
    let height = 2.0 * CARD_PAD_Y + count * row_height(painter) + (count - 1.0).max(0.0) * 14.0;
    let x = WIDTH - PAD_X - CARD_WIDTH;
    let y = top + ((bottom - top) - height).max(0.0) / 2.0;
    painter.fill(rect(x, y, CARD_WIDTH, height), 12.0, "card");
    let mut row_top = y + CARD_PAD_Y;
    for row in rows {
        row_top = card_row(painter, row, x + CARD_PAD_X, row_top) + 14.0;
    }
}

pub fn render_card(
    widget: &gtk::Widget,
    card: &ShareCard,
    palette: &Palette,
    lang: Lang,
) -> Result<gdk::Texture, ShareError> {
    let painter = Painter {
        snapshot: gtk::Snapshot::new(),
        widget,
        palette,
    };
    painter.snapshot.scale(SCALE, SCALE);
    painter.fill(rect(0.0, 0.0, WIDTH, HEIGHT), 0.0, "share-bg");
    let top = top_bar(&painter, card);
    let bottom = footer(&painter, lang);
    left_column(&painter, card, top, bottom);
    right_card(&painter, card, top, bottom);
    let node = painter.snapshot.to_node().ok_or(ShareError::Empty)?;
    let renderer = gsk::CairoRenderer::new();
    renderer.realize_for_display(&widget.display())?;
    let bounds = rect(
        0.0,
        0.0,
        WIDTH * f64::from(SCALE),
        HEIGHT * f64::from(SCALE),
    );
    let texture = renderer.render_texture(node, Some(&bounds));
    renderer.unrealize();
    Ok(texture)
}
