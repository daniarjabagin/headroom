use gtk::prelude::*;

use crate::account::NoticeView;
use crate::notices::NoticeKind;
use crate::ui::context::Ctx;
use crate::ui::widgets::{button, column, icon, label, row, text_button, wrapping_label};

const TILE_ICON: i32 = 14;
const LINE_ICON: i32 = 12;

pub struct NoticeActions {
    pub sign_in: Option<Box<dyn Fn()>>,
    pub retry: Option<Box<dyn Fn()>>,
    pub retrying: bool,
}

fn kind_icon(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Error => "dialog-error-symbolic",
        NoticeKind::SignIn => "system-users-symbolic",
        NoticeKind::Warning | NoticeKind::Info => "dialog-warning-symbolic",
    }
}

fn tile(kind: NoticeKind) -> gtk::Box {
    let tile = row(0, &["headroom-notice-tile"]);
    tile.set_valign(gtk::Align::Start);
    let image = icon(kind_icon(kind), TILE_ICON, &[]);
    image.set_hexpand(true);
    image.set_halign(gtk::Align::Center);
    tile.append(&image);
    tile
}

fn retry_button(ctx: &Ctx, run: Box<dyn Fn()>, retrying: bool) -> gtk::Button {
    let content = row(4, &[]);
    if retrying {
        let spinner = gtk::Spinner::new();
        spinner.set_spinning(ctx.motion);
        spinner.set_size_request(10, 10);
        content.append(&spinner);
    }
    let text = if retrying { "Retrying…" } else { "Retry" };
    content.append(&label(ctx.locale.lang.tr(text), &[]));
    let retry = button(&content, &["headroom-small-button"], run);
    retry.set_sensitive(!retrying);
    retry
}

fn action_row(ctx: &Ctx, actions: NoticeActions) -> Option<gtk::Box> {
    let buttons = row(4, &[]);
    buttons.set_valign(gtk::Align::Center);
    let mut any = false;
    if let Some(sign_in) = actions.sign_in {
        let text = ctx.locale.lang.tr("Sign in…");
        buttons.append(&text_button(
            text,
            &["headroom-small-button", "primary"],
            sign_in,
        ));
        any = true;
    }
    if let Some(retry) = actions.retry {
        buttons.append(&retry_button(ctx, retry, actions.retrying));
        any = true;
    }
    any.then_some(buttons)
}

fn texts(notice: &NoticeView) -> gtk::Box {
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.set_valign(gtk::Align::Center);
    texts.append(&wrapping_label(&notice.title, &["headroom-notice-title"]));
    if let Some(detail) = &notice.detail {
        texts.append(&wrapping_label(detail, &["headroom-notice-detail"]));
    }
    if let Some(note) = &notice.note {
        texts.append(&wrapping_label(
            note,
            &["headroom-notice-detail", "headroom-notice-note"],
        ));
    }
    texts
}

pub fn notice_row(ctx: &Ctx, notice: &NoticeView, actions: NoticeActions) -> gtk::Box {
    let kind_class = match notice.kind {
        NoticeKind::Error => "error",
        NoticeKind::SignIn => "signin",
        NoticeKind::Warning | NoticeKind::Info => "warning",
    };
    let body = row(8, &["headroom-notice", kind_class]);
    let texts = texts(notice);
    let stacked = actions.sign_in.is_some();
    body.append(&tile(notice.kind));
    body.append(&texts);
    if let Some(buttons) = action_row(ctx, actions) {
        if stacked {
            buttons.set_margin_top(6);
            buttons.set_halign(gtk::Align::Start);
            texts.append(&buttons);
        } else {
            body.append(&buttons);
        }
    }
    body
}

pub fn notice_line(text: &str) -> gtk::Box {
    let line = row(6, &["headroom-notice-line"]);
    let image = icon(
        "dialog-information-symbolic",
        LINE_ICON,
        &["headroom-notice-line-icon"],
    );
    image.set_valign(gtk::Align::Start);
    line.append(&image);
    line.append(&wrapping_label(text, &["headroom-notice-line-text"]));
    line
}
