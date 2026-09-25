use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use jiff::Timestamp;

use crate::dates::Locale;
use crate::i18n::fill;
use crate::popup_model::footer::{FooterLines, FooterMood, footer_lines};
use crate::ui::context::{Action, Ctx, RefreshMode};
use crate::ui::draw::{fill as fill_path, set_color};
use crate::ui::keyed::SharedTick;
use crate::ui::widgets::{button, column, icon, label, row};
use crate::view::View;

const REFRESH_ICON: i32 = 14;
const FOOTER_ICON: i32 = 16;
const STALE_ICON: i32 = 10;
const LIVE_DOT: i32 = 12;

pub struct RefreshButton {
    pub widget: gtk::Button,
    icon: gtk::Image,
    spinner: gtk::Spinner,
}

impl RefreshButton {
    pub fn new(ctx: &Ctx) -> Self {
        let content = gtk::Stack::new();
        let icon = icon("view-refresh-symbolic", REFRESH_ICON, &[]);
        let spinner = gtk::Spinner::new();
        spinner.set_size_request(REFRESH_ICON, REFRESH_ICON);
        content.add_child(&icon);
        content.add_child(&spinner);
        let widget = button(
            &content,
            &["headroom-icon-button", "headroom-refresh-button"],
            ctx.action(Action::RefreshNow),
        );
        widget.set_valign(gtk::Align::Center);
        let refresh = Self {
            widget,
            icon,
            spinner,
        };
        refresh.apply(ctx);
        refresh
    }

    pub fn apply(&self, ctx: &Ctx) {
        let mode = ctx.ui.refresh;
        let spinning = mode == RefreshMode::Busy && ctx.motion;
        self.spinner.set_visible(spinning);
        self.spinner.set_spinning(spinning);
        self.icon.set_visible(!spinning);
        for class in ["busy", "failed"] {
            self.widget.remove_css_class(class);
        }
        self.widget.set_opacity(1.0);
        match mode {
            RefreshMode::Busy => {
                self.widget.add_css_class("busy");
                self.widget.set_opacity(if ctx.motion { 1.0 } else { 0.55 });
            }
            RefreshMode::Failed => self.widget.add_css_class("failed"),
            RefreshMode::Idle => {}
        }
        let tip = if mode == RefreshMode::Failed {
            "Refresh failed"
        } else {
            "Refresh"
        };
        self.widget.set_tooltip_text(Some(ctx.locale.lang.tr(tip)));
    }
}

pub fn top_bar(refresh: &gtk::Widget) -> gtk::Box {
    let bar = row(0, &["headroom-top-bar"]);
    if refresh.parent().is_some() {
        refresh.unparent();
    }
    refresh.set_hexpand(true);
    refresh.set_halign(gtk::Align::End);
    bar.append(refresh);
    bar
}

fn live_dot(ctx: &Ctx) -> gtk::DrawingArea {
    let (dot, halo) = (ctx.color("positive"), ctx.color("positive-halo"));
    let area = gtk::DrawingArea::new();
    area.set_content_width(LIVE_DOT);
    area.set_content_height(LIVE_DOT);
    area.set_valign(gtk::Align::Center);
    area.set_draw_func(move |_, cr, width, height| {
        let (x, y) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
        set_color(cr, halo);
        cr.arc(x, y, x.min(y), 0.0, std::f64::consts::TAU);
        fill_path(cr);
        set_color(cr, dot);
        cr.arc(x, y, 3.0, 0.0, std::f64::consts::TAU);
        fill_path(cr);
    });
    area
}

struct Lines {
    first_row: gtk::Box,
    first: gtk::Label,
    stale: gtk::Image,
    link: gtk::Button,
    second: gtk::Label,
    dot: gtk::DrawingArea,
}

fn set_notice(label: &gtk::Label, on: bool) {
    if on {
        label.add_css_class("notice");
    } else {
        label.remove_css_class("notice");
    }
}

impl Lines {
    fn show(&self, lines: &FooterLines) {
        let stale = lines.mood == FooterMood::Stale;
        self.first_row.set_visible(lines.first.is_some());
        self.first.set_text(lines.first.as_deref().unwrap_or(""));
        self.stale.set_visible(stale);
        set_notice(&self.first, stale);
        set_notice(&self.second, stale);
        self.second.set_text(&lines.second);
        self.dot.set_visible(lines.mood == FooterMood::Live);
        self.link.set_visible(!lines.second.is_empty());
        let tip = lines.tooltip.as_deref();
        self.link.set_tooltip_text(tip);
        self.first_row.set_tooltip_text(tip);
    }
}

fn lines(ctx: &Ctx) -> (gtk::Box, Lines) {
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.set_valign(gtk::Align::Center);
    let first_row = row(4, &[]);
    let stale = icon(
        "dialog-warning-symbolic",
        STALE_ICON,
        &["headroom-footer-stale"],
    );
    stale.set_valign(gtk::Align::Center);
    let first = label("", &["headroom-footer-text"]);
    first.set_ellipsize(gtk::pango::EllipsizeMode::End);
    first_row.append(&stale);
    first_row.append(&first);
    let second_row = row(3, &[]);
    let dot = live_dot(ctx);
    let second = label("", &["headroom-footer-text"]);
    second.set_ellipsize(gtk::pango::EllipsizeMode::End);
    second_row.append(&dot);
    second_row.append(&second);
    let link = button(
        &second_row,
        &["headroom-footer-link"],
        ctx.action(Action::RefreshNow),
    );
    link.set_halign(gtk::Align::Start);
    texts.append(&first_row);
    texts.append(&link);
    let parts = Lines {
        first_row,
        first,
        stale,
        link,
        second,
        dot,
    };
    (texts, parts)
}

fn icon_button(ctx: &Ctx, name: &str, tip: &str, action: Action) -> gtk::Button {
    let icon_button = button(
        &icon(name, FOOTER_ICON, &[]),
        &["headroom-icon-button", "headroom-footer-button"],
        ctx.action(action),
    );
    icon_button.set_valign(gtk::Align::Center);
    icon_button.set_tooltip_text(Some(tip));
    icon_button
}

pub struct MountedFooter {
    pub widget: gtk::Box,
    pub ticks: Vec<SharedTick>,
    input: Rc<RefCell<(View, Locale)>>,
}

impl MountedFooter {
    pub fn new(ctx: &Ctx, view: &View, now: Timestamp) -> Self {
        let lang = ctx.locale.lang;
        let bar = row(12, &["headroom-footer"]);
        let (texts, parts) = lines(ctx);
        bar.append(&texts);
        let settings_tip = fill(
            lang.tr("Settings · {version}"),
            &[("version", &ctx.version)],
        );
        bar.append(&icon_button(
            ctx,
            "emblem-system-symbolic",
            &settings_tip,
            Action::OpenSettings,
        ));
        bar.append(&icon_button(
            ctx,
            "application-exit-symbolic",
            lang.tr("Quit"),
            Action::Quit,
        ));
        let input = Rc::new(RefCell::new((view.clone(), ctx.locale.clone())));
        let shown = Rc::clone(&input);
        let show: SharedTick = Rc::new(move |now| {
            let (view, locale) = &*shown.borrow();
            parts.show(&footer_lines(view, locale, now));
            parts.link.set_sensitive(view.state().is_some());
        });
        show(now);
        Self {
            widget: bar,
            ticks: vec![show],
            input,
        }
    }

    pub fn update(&self, ctx: &Ctx, view: &View, now: Timestamp) {
        *self.input.borrow_mut() = (view.clone(), ctx.locale.clone());
        if let Some(show) = self.ticks.first() {
            show(now);
        }
    }
}
