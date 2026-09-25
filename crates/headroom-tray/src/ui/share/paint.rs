use gtk::prelude::*;
use gtk::{gdk, graphene, gsk, pango};

use crate::palette::{Palette, Rgba};

pub const SCALE: f32 = 2.0;

pub struct Painter<'a> {
    pub snapshot: gtk::Snapshot,
    pub widget: &'a gtk::Widget,
    pub palette: &'a Palette,
}

#[derive(Debug, Clone, Copy)]
pub struct Font {
    pub size: f64,
    pub weight: pango::Weight,
    pub spacing: f64,
}

impl Font {
    pub const fn new(size: f64, weight: pango::Weight) -> Self {
        Self {
            size,
            weight,
            spacing: 0.0,
        }
    }

    pub const fn spaced(self, spacing: f64) -> Self {
        Self { spacing, ..self }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "colour channels and pango units are small bounded values"
)]
fn rgba(color: Rgba) -> gdk::RGBA {
    gdk::RGBA::new(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32,
    )
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "font sizes and letter spacing are small pixel values"
)]
fn pango_units(pixels: f64) -> i32 {
    (pixels * f64::from(pango::SCALE)).round() as i32
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "logical share-card coordinates are small"
)]
pub fn point(x: f64, y: f64) -> graphene::Point {
    graphene::Point::new(x as f32, y as f32)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "logical share-card coordinates are small"
)]
pub fn rect(x: f64, y: f64, width: f64, height: f64) -> graphene::Rect {
    graphene::Rect::new(x as f32, y as f32, width as f32, height as f32)
}

impl Painter<'_> {
    pub fn color(&self, token: &str) -> gdk::RGBA {
        self.palette.color(token).map_or(gdk::RGBA::BLACK, rgba)
    }

    pub fn fill(&self, area: graphene::Rect, radius: f64, token: &str) {
        self.fill_rgba(area, radius, &self.color(token));
    }

    pub fn fill_rgba(&self, area: graphene::Rect, radius: f64, color: &gdk::RGBA) {
        #[allow(clippy::cast_possible_truncation, reason = "a small corner radius")]
        let corner = graphene::Size::new(radius as f32, radius as f32);
        let rounded = gsk::RoundedRect::new(area, corner, corner, corner, corner);
        self.snapshot.push_rounded_clip(&rounded);
        self.snapshot.append_color(color, &area);
        self.snapshot.pop();
    }

    pub fn layout(&self, text: &str, font: Font) -> pango::Layout {
        let layout = self.widget.create_pango_layout(Some(text));
        let mut description = self
            .widget
            .pango_context()
            .font_description()
            .unwrap_or_default();
        description.set_absolute_size(f64::from(pango_units(font.size)));
        description.set_weight(font.weight);
        layout.set_font_description(Some(&description));
        let attributes = pango::AttrList::new();
        attributes.insert(pango::AttrFontFeatures::new("tnum"));
        if font.spacing != 0.0 {
            attributes.insert(pango::AttrInt::new_letter_spacing(pango_units(
                font.spacing,
            )));
        }
        layout.set_attributes(Some(&attributes));
        layout
    }

    pub fn size(layout: &pango::Layout) -> (f64, f64) {
        let (width, height) = layout.pixel_size();
        (f64::from(width), f64::from(height))
    }

    pub fn text(&self, layout: &pango::Layout, x: f64, y: f64, token: &str) {
        self.snapshot.save();
        self.snapshot.translate(&point(x, y));
        self.snapshot.append_layout(layout, &self.color(token));
        self.snapshot.restore();
    }

    pub fn texture(&self, texture: &gdk::Texture, area: graphene::Rect) {
        self.snapshot.append_texture(texture, &area);
    }

    pub fn skewed_plate(&self, x: f64, y: f64, size: f64, token: &str) {
        self.snapshot.save();
        self.snapshot
            .translate(&point(x + size / 2.0, y + size / 2.0));
        self.snapshot
            .transform(Some(&gsk::Transform::new().skew(-16.0, 0.0)));
        self.fill(rect(-size / 2.0, -size / 2.0, size, size), 1.0, token);
        self.snapshot.restore();
    }
}
