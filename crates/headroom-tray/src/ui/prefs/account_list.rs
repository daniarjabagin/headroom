use adw::prelude::*;

use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::i18n::Lang;
use crate::payload::Tone;
use crate::preferences::accounts::{Mark, SidebarItem};
use crate::ui::widgets::svg_image;

const LOGO_SIZE: i32 = 24;
const DOT_SIZE: i32 = 8;
const HIDDEN_OPACITY: f64 = 0.6;
const STARRED: &str = "starred-symbolic";
const HIDDEN: &str = "view-conceal-symbolic";
const PROBLEM: &str = "dialog-warning-symbolic";
const TONE_CLASSES: [&str; 4] = ["accent", "warning", "error", "dim-label"];

#[must_use]
pub fn provider_image(provider: &str, text_color: &str, size: i32) -> gtk::Image {
    let (svg, tint) = provider_logo(provider);
    let color = match tint {
        Tint::Brand => CLAUDE_COLOR,
        Tint::Text => text_color,
    };
    svg_image(svg, color, size, &[])
}

fn tone_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Good => "accent",
        Tone::Warning => "warning",
        Tone::Critical => "error",
        Tone::Neutral => "dim-label",
    }
}

fn dot() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::builder()
        .content_width(DOT_SIZE)
        .content_height(DOT_SIZE)
        .valign(gtk::Align::Center)
        .build();
    area.set_draw_func(|area, cr, width, height| {
        let color = area.color();
        let radius = f64::from(width.min(height)) / 2.0;
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()),
        );
        cr.arc(radius, radius, radius, 0.0, std::f64::consts::TAU);
        crate::ui::draw::fill(cr);
    });
    area
}

fn flag(icon: &str) -> gtk::Image {
    let image = gtk::Image::from_icon_name(icon);
    image.add_css_class("dim-label");
    image.set_valign(gtk::Align::Center);
    image
}

pub struct SidebarRow {
    pub row: adw::ActionRow,
    pub account_id: String,
    provider: String,
    star: gtk::Image,
    hidden: gtk::Image,
    problem: gtk::Image,
    dot: gtk::DrawingArea,
}

impl SidebarRow {
    pub fn new(item: &SidebarItem, logo_color: &str) -> Self {
        let row = adw::ActionRow::builder()
            .use_markup(false)
            .activatable(true)
            .build();
        row.add_prefix(&provider_image(&item.provider, logo_color, LOGO_SIZE));
        let problem = flag(PROBLEM);
        problem.remove_css_class("dim-label");
        problem.add_css_class("warning");
        let sidebar = Self {
            star: flag(STARRED),
            hidden: flag(HIDDEN),
            problem,
            dot: dot(),
            row,
            account_id: item.account_id.clone(),
            provider: item.provider.clone(),
        };
        for widget in [
            sidebar.star.upcast_ref::<gtk::Widget>(),
            sidebar.hidden.upcast_ref(),
            sidebar.problem.upcast_ref(),
            sidebar.dot.upcast_ref(),
        ] {
            sidebar.row.add_suffix(widget);
        }
        sidebar
    }

    #[must_use]
    pub fn fits(&self, item: &SidebarItem) -> bool {
        self.account_id == item.account_id && self.provider == item.provider
    }

    pub fn update(&self, lang: Lang, item: &SidebarItem) {
        self.row.set_title(&item.title);
        self.row.set_subtitle(&item.subtitle);
        self.row.set_subtitle_lines(1);
        self.row.set_title_lines(1);
        self.row
            .set_opacity(if item.hidden { HIDDEN_OPACITY } else { 1.0 });
        self.star.set_visible(item.starred);
        self.star
            .set_tooltip_text(Some(lang.tr("Always open in the popup")));
        self.hidden.set_visible(item.hidden);
        self.hidden
            .set_tooltip_text(Some(lang.tr("Hidden from the tray and popup")));
        self.update_mark(item);
    }

    fn update_mark(&self, item: &SidebarItem) {
        self.problem.set_visible(item.mark == Mark::Problem);
        self.problem.set_tooltip_text(item.note);
        self.dot.set_visible(item.mark != Mark::Problem);
        self.dot.set_tooltip_text(item.note);
        if let Mark::Tone(tone) = item.mark {
            for class in TONE_CLASSES {
                self.dot.remove_css_class(class);
            }
            self.dot.add_css_class(tone_class(tone));
            self.dot.queue_draw();
        }
    }
}
