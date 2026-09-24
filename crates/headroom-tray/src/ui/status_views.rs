use gtk::prelude::*;

use crate::assets::MARK;
use crate::ui::context::{Action, Ctx};
use crate::ui::widgets::{column, row, spacer, svg_image, text_button, wrapping_label};

const LOADING_SECTIONS: [usize; 2] = [2, 3];
const MARK_SIZE: i32 = 32;

fn block(width: i32, height: i32) -> gtk::Box {
    let block = row(0, &["headroom-skeleton-block"]);
    block.set_size_request(width, height);
    block.set_valign(gtk::Align::Center);
    block
}

fn line(left: i32, right: i32) -> gtk::Box {
    let line = row(0, &["headroom-row-line"]);
    line.append(&block(left, 12));
    line.append(&spacer());
    line.append(&block(right, 12));
    line
}

fn skeleton_meter() -> gtk::Box {
    let meter = block(-1, 5);
    meter.set_hexpand(true);
    meter.add_css_class("headroom-meter");
    meter
}

pub fn skeleton_rows(count: usize) -> gtk::Box {
    let rows = column(0, &[]);
    for _ in 0..count {
        let quota = column(6, &["headroom-quota-row"]);
        quota.append(&line(64, 48));
        quota.append(&skeleton_meter());
        quota.append(&line(56, 88));
        rows.append(&quota);
    }
    rows
}

fn skeleton_section(rows: usize) -> gtk::Box {
    let section = column(4, &[]);
    let header = row(6, &["headroom-section-header"]);
    header.append(&block(16, 16));
    header.append(&block(96, 14));
    section.append(&header);
    let card = column(0, &["headroom-card"]);
    card.append(&skeleton_rows(rows));
    section.append(&card);
    section
}

pub fn loading_sections() -> Vec<gtk::Box> {
    LOADING_SECTIONS
        .iter()
        .map(|rows| skeleton_section(*rows))
        .collect()
}

fn centered(text: &str, class: &str) -> gtk::Label {
    let label = wrapping_label(text, &[class]);
    label.set_justify(gtk::Justification::Center);
    label.set_xalign(0.5);
    label
}

fn status_card(children: &[gtk::Widget]) -> gtk::Box {
    let card = column(10, &["headroom-card", "headroom-status-card"]);
    for child in children {
        card.append(child);
    }
    card
}

fn primary(ctx: &Ctx, text: &str, action: Action) -> gtk::Button {
    let button = text_button(text, &["headroom-primary-button"], ctx.action(action));
    button.set_halign(gtk::Align::Center);
    button
}

pub fn service_view(ctx: &Ctx) -> gtk::Box {
    let lang = ctx.locale.lang;
    let mark = svg_image(
        MARK,
        &ctx.css("text-secondary"),
        MARK_SIZE,
        &["headroom-status-mark"],
    );
    let starting = ctx.ui.service_starting;
    let start_text = lang.tr(if starting {
        "Starting…"
    } else {
        "Start service"
    });
    let start = primary(ctx, start_text, Action::StartService);
    start.set_sensitive(!starting);
    let mut children: Vec<gtk::Widget> = vec![
        mark.upcast(),
        centered(
            lang.tr("Headroom service isn't running"),
            "headroom-status-title",
        )
        .upcast(),
        centered(
            lang.tr("Start it to see your usage limits here."),
            "headroom-status-detail",
        )
        .upcast(),
        start.upcast(),
    ];
    if let Some(error) = &ctx.ui.service_error {
        children.push(centered(error, "headroom-status-error").upcast());
    }
    status_card(&children)
}

pub fn error_view(ctx: &Ctx, message: &str) -> gtk::Box {
    let lang = ctx.locale.lang;
    status_card(&[
        centered(
            lang.tr("Couldn't read Headroom's state"),
            "headroom-status-title",
        )
        .upcast(),
        centered(message, "headroom-status-detail").upcast(),
        primary(ctx, lang.tr("Try again"), Action::Refresh(String::new())).upcast(),
    ])
}

pub fn empty_view(ctx: &Ctx) -> gtk::Box {
    let lang = ctx.locale.lang;
    let again = text_button(
        lang.tr("Check again"),
        &["headroom-small-button"],
        ctx.action(Action::Refresh(String::new())),
    );
    again.set_halign(gtk::Align::Center);
    status_card(&[
        centered(
            lang.tr("No AI coding tools found."),
            "headroom-status-detail",
        )
        .upcast(),
        again.upcast(),
    ])
}
